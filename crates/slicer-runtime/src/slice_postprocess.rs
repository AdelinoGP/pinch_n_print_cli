//! Slice-postprocess paint annotation execution contract.
//! The active annotation path is `PrePass::PaintSegmentation` which writes
//! directly into `SliceIR` segment annotations; this module retains the error
//! types and warning plumbing for the event chain, but the host built-in
//! `execute_slice_postprocess_paint_annotation` body is a no-op (AC-16).

use slicer_ir::{PaintSemantic, PaintValue, SliceIR};

use crate::progress_events::{EventReason, ProgressError, ProgressEvent, ProgressPhase};

/// One per-layer paint annotation invocation (retained for API continuity; body is no-op).
#[derive(Debug, Clone)]
pub struct SlicePostProcessPaintAnnotationRequest {
    /// Slice geometry (passed through unchanged by the no-op body).
    pub slice_ir: SliceIR,
    /// Semantics that were required (unused by the no-op body).
    pub required_semantics: Vec<PaintSemantic>,
}

/// Output of the built-in paint annotation finalization step.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicePostProcessPaintAnnotationResult {
    /// Slice IR with `segment_annotations` rewritten for all regions.
    pub slice_ir: SliceIR,
    /// True when non-fatal fallback behavior was required.
    pub degraded: bool,
    /// Structured non-fatal warnings suitable for progress events.
    pub warnings: Vec<SlicePostProcessPaintAnnotationWarning>,
}

/// Structured non-fatal fallback record.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicePostProcessPaintAnnotationWarning {
    /// Stable warning code for frontend/event routing.
    pub code: u16,
    /// Layer where the fallback occurred.
    pub global_layer_index: u32,
    /// Region object identifier.
    pub object_id: String,
    /// Region identifier.
    pub region_id: u64,
    /// Semantic that was defaulted.
    pub semantic: PaintSemantic,
    /// Polygon index within `SlicedRegion.polygons`.
    pub polygon_index: usize,
    /// Contour-point index within the polygon contour.
    pub contour_point_index: usize,
    /// Deterministic fallback value that was written.
    pub fallback_value: PaintValue,
    /// Stable machine-readable warning kind.
    pub reason: SlicePostProcessPaintAnnotationWarningReason,
}

/// Warning kinds emitted by the host paint annotator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicePostProcessPaintAnnotationWarningReason {
    /// Point classification remained numerically unresolved after polygon edits.
    NumericalEdgeAmbiguity,
}

/// Fatal paint annotation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlicePostProcessPaintAnnotationError {
    /// Required paint data is missing for the layer entirely.
    MissingPaintRegionLayer {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Missing semantic family.
        semantic: PaintSemantic,
    },
    /// Required paint data is missing for one semantic on this layer.
    MissingPaintRegionSemantic {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Missing semantic family.
        semantic: PaintSemantic,
    },
    /// Existing boundary paint no longer matches final contour cardinality.
    SegmentAnnotationsCardinalityMismatch {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Region object identifier.
        object_id: String,
        /// Region identifier.
        region_id: u64,
        /// Semantic family carrying stale cardinality.
        semantic: PaintSemantic,
        /// Polygon index within `SlicedRegion.polygons`.
        polygon_index: usize,
        /// Final contour point count.
        expected_points: usize,
        /// Existing boundary-paint point count.
        actual_points: usize,
    },
    /// Equal-precedence conflicting custom values were encountered deterministically.
    DeterministicConflict {
        /// Stable fatal code.
        code: u16,
        /// Layer being annotated.
        global_layer_index: u32,
        /// Region object identifier.
        object_id: String,
        /// Region identifier.
        region_id: u64,
        /// Conflicting semantic family.
        semantic: PaintSemantic,
        /// Polygon index within `SlicedRegion.polygons`.
        polygon_index: usize,
        /// Contour-point index that triggered the conflict.
        contour_point_index: usize,
    },
}

/// Convert a paint-annotation fallback warning into a non-fatal
/// `ProgressEvent::module_error` so it propagates through the documented
/// progress-events chain (docs/04 §Recoverability and docs/11 §73-75).
///
/// The emitted event carries `fatal=false`, `phase=PerLayer`,
/// `stage="Layer::SlicePostProcess"`, and the warning's stable code so
/// that `SliceEventCollector` flips `degraded=true` on ingestion. The
/// `module_id` defaults to the host built-in id but callers may override
/// it when a guest module triggered the fallback.
pub fn paint_annotation_warning_to_progress_event(
    warning: &SlicePostProcessPaintAnnotationWarning,
    slice_id: String,
    module_id: String,
    timestamp_ms: u64,
) -> ProgressEvent {
    let reason_label = match warning.reason {
        SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity => {
            "numerical-edge-ambiguity"
        }
    };
    let message = format!(
        "paint annotation fell back to deterministic default ({reason}) on layer {layer} \
         object='{obj}' region={rid} semantic={sem:?} polygon={pi} contour_point={cpi}",
        reason = reason_label,
        layer = warning.global_layer_index,
        obj = warning.object_id,
        rid = warning.region_id,
        sem = warning.semantic,
        pi = warning.polygon_index,
        cpi = warning.contour_point_index,
    );
    ProgressEvent::module_error(
        slice_id,
        ProgressPhase::PerLayer,
        String::from("Layer::SlicePostProcess"),
        warning.global_layer_index,
        module_id,
        timestamp_ms,
        ProgressError {
            code: warning.code as u32,
            message,
            fatal: false,
            suggestion: Some(String::from(
                "regenerate paint regions with denser sampling, or accept the deterministic default",
            )),
            reason: Some(match warning.reason {
                SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity => {
                    EventReason::NumericalEdgeAmbiguity
                }
            }),
        },
    )
}

/// Coalesce a sequence of paint-annotation warnings into one progress event
/// per `(global_layer_index, object_id, region_id, semantic, polygon_index)`
/// group, preserving the input ordering of first occurrence per group.
///
/// Each emitted event carries a message that includes the number of affected
/// contour points and the first/last contour-point index in the group, so a
/// single structurally-noisy paint region produces one log line per polygon
/// per semantic instead of one line per contour point.
///
/// `timestamp_ms_base` is the timestamp of the first emitted event; subsequent
/// events receive `timestamp_ms_base + i` where `i` is the group index. The
/// individual `SlicePostProcessPaintAnnotationWarning` values are still
/// available on the annotation result for callers that need per-point detail.
pub fn paint_annotation_warnings_to_progress_events(
    warnings: &[SlicePostProcessPaintAnnotationWarning],
    slice_id: String,
    module_id: String,
    timestamp_ms_base: u64,
) -> Vec<ProgressEvent> {
    // Group by (layer, object_id, region_id, semantic, polygon_index) while
    // preserving the order in which each group was first encountered.
    type GroupKey<'a> = (u32, &'a str, u64, &'a PaintSemantic, usize);
    let mut order: Vec<GroupKey> = Vec::new();
    let mut groups: std::collections::HashMap<
        GroupKey,
        Vec<&SlicePostProcessPaintAnnotationWarning>,
    > = std::collections::HashMap::new();

    for w in warnings {
        let key: GroupKey = (
            w.global_layer_index,
            w.object_id.as_str(),
            w.region_id,
            &w.semantic,
            w.polygon_index,
        );
        if !groups.contains_key(&key) {
            order.push(key);
        }
        groups.entry(key).or_default().push(w);
    }

    order
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let members = &groups[&key];
            let first = members
                .first()
                .expect("group must contain at least one warning");
            let reason_label = match first.reason {
                SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity => {
                    "numerical-edge-ambiguity"
                }
            };
            let count = members.len();
            let first_cpi = members
                .iter()
                .map(|w| w.contour_point_index)
                .min()
                .expect("non-empty group");
            let last_cpi = members
                .iter()
                .map(|w| w.contour_point_index)
                .max()
                .expect("non-empty group");
            let message = if count == 1 {
                format!(
                    "paint annotation fell back to deterministic default ({reason}) on layer {layer} \
                     object='{obj}' region={rid} semantic={sem:?} polygon={pi} contour_point={cpi}",
                    reason = reason_label,
                    layer = first.global_layer_index,
                    obj = first.object_id,
                    rid = first.region_id,
                    sem = first.semantic,
                    pi = first.polygon_index,
                    cpi = first_cpi,
                )
            } else {
                format!(
                    "paint annotation fell back to deterministic default ({reason}) on layer {layer} \
                     object='{obj}' region={rid} semantic={sem:?} polygon={pi} on {count} contour points \
                     (first={first_cpi}, last={last_cpi})",
                    reason = reason_label,
                    layer = first.global_layer_index,
                    obj = first.object_id,
                    rid = first.region_id,
                    sem = first.semantic,
                    pi = first.polygon_index,
                )
            };
            ProgressEvent::module_error(
                slice_id.clone(),
                ProgressPhase::PerLayer,
                String::from("Layer::SlicePostProcess"),
                first.global_layer_index,
                module_id.clone(),
                timestamp_ms_base + i as u64,
                ProgressError {
                    code: first.code as u32,
                    message,
                    fatal: false,
                    suggestion: Some(String::from(
                        "regenerate paint regions with denser sampling, or accept the deterministic default",
                    )),
                    reason: Some(match first.reason {
                        SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity => {
                            EventReason::NumericalEdgeAmbiguity
                        }
                    }),
                },
            )
        })
        .collect()
}

/// No-op stub: paint annotation is handled by `PrePass::PaintSegmentation`
/// which writes colour data directly into `SliceIR` segment annotations (AC-16).
/// The request's `slice_ir` is passed through unchanged.
pub fn execute_slice_postprocess_paint_annotation(
    request: SlicePostProcessPaintAnnotationRequest,
) -> Result<SlicePostProcessPaintAnnotationResult, SlicePostProcessPaintAnnotationError> {
    Ok(SlicePostProcessPaintAnnotationResult {
        slice_ir: request.slice_ir,
        degraded: false,
        warnings: Vec::new(),
    })
}
