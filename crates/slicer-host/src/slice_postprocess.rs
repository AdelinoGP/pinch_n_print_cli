//! Slice-postprocess paint annotation execution contract.

use std::sync::Arc;

use rayon::prelude::*;
use rstar::AABB;

use slicer_core::paint_region::{
    ex_polygon_contains_point, point_in_paint_region, BoundaryInclusion, PaintRegionQueryError,
    PaintRegionRTreeIndex,
};
use slicer_ir::{ExPolygon, PaintRegionIR, PaintSemantic, PaintValue, SemanticRegion, SliceIR};

use crate::progress_events::{ProgressError, ProgressEvent, ProgressPhase};

/// One per-layer paint annotation invocation.
#[derive(Debug, Clone)]
pub struct SlicePostProcessPaintAnnotationRequest {
    /// Slice geometry after all `Layer::SlicePostProcess` polygon edits.
    pub slice_ir: SliceIR,
    /// Immutable per-layer paint regions produced by `PrePass::PaintSegmentation`.
    pub paint_regions: Arc<PaintRegionIR>,
    /// Companion spatial index for O(log N) region lookup (optional; falls back to linear scan).
    pub paint_region_rtree: Option<Arc<PaintRegionRTreeIndex>>,
    /// Semantics that must be annotatable for this layer.
    pub required_semantics: Vec<PaintSemantic>,
    /// Per-layer modifier volume projections for fuzzy-skin annotation.
    pub modifier_projections: Vec<ExPolygon>,
}

/// Output of the built-in paint annotation finalization step.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicePostProcessPaintAnnotationResult {
    /// Slice IR with `boundary_paint` rewritten for all regions.
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
    BoundaryPaintCardinalityMismatch {
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
                },
            )
        })
        .collect()
}

/// Annotate one final `SliceIR` layer with contour-parallel `boundary_paint`.
pub fn execute_slice_postprocess_paint_annotation(
    request: SlicePostProcessPaintAnnotationRequest,
) -> Result<SlicePostProcessPaintAnnotationResult, SlicePostProcessPaintAnnotationError> {
    let SlicePostProcessPaintAnnotationRequest {
        mut slice_ir,
        paint_regions,
        required_semantics,
        modifier_projections,
        paint_region_rtree,
    } = request;

    let layer_index = slice_ir.global_layer_index;

    if required_semantics.is_empty() {
        return Ok(SlicePostProcessPaintAnnotationResult {
            slice_ir,
            degraded: false,
            warnings: Vec::new(),
        });
    }

    // Pre-load region slices for all required semantics (one get() per semantic per layer)
    let mut semantic_regions_cache: Vec<&[SemanticRegion]> =
        Vec::with_capacity(required_semantics.len());
    for semantic in &required_semantics {
        let regions = paint_regions.get(layer_index, semantic);
        if regions.is_empty() {
            return Err(
                SlicePostProcessPaintAnnotationError::MissingPaintRegionSemantic {
                    code: 501,
                    global_layer_index: layer_index,
                    semantic: semantic.clone(),
                },
            );
        }
        semantic_regions_cache.push(regions);
    }

    let mut degraded = false;
    let mut warnings = Vec::new();

    // Process each region
    for region in &mut slice_ir.regions {
        for (semantic_index, semantic) in required_semantics.iter().enumerate() {
            let semantic_regions = semantic_regions_cache[semantic_index];

            // Check for stale cardinality if boundary_paint already exists for this semantic
            if let Some(existing) = region.boundary_paint.get(semantic) {
                for (polygon_index, polygon_paint) in existing.iter().enumerate() {
                    if polygon_index >= region.polygons.len() {
                        continue;
                    }
                    let expected_points = region.polygons[polygon_index].contour.points.len();
                    let actual_points = polygon_paint.len();
                    if actual_points != expected_points {
                        return Err(
                            SlicePostProcessPaintAnnotationError::BoundaryPaintCardinalityMismatch {
                                code: 502,
                                global_layer_index: layer_index,
                                object_id: region.object_id.clone(),
                                region_id: region.region_id,
                                semantic: semantic.clone(),
                                polygon_index,
                                expected_points,
                                actual_points,
                            },
                        );
                    }
                }
            }

            // Build boundary_paint for this semantic via parallel polygon iteration
            let object_id = region.object_id.clone();
            let region_id = region.region_id;
            let polygon_results: Result<
                Vec<(
                    Vec<Option<PaintValue>>,
                    Vec<SlicePostProcessPaintAnnotationWarning>,
                    bool,
                )>,
                SlicePostProcessPaintAnnotationError,
            > = region
                .polygons
                .par_iter()
                .enumerate()
                .map(|(polygon_index, polygon)| {
                    let mut point_paint: Vec<Option<PaintValue>> =
                        Vec::with_capacity(polygon.contour.points.len());
                    let mut local_warnings = Vec::new();
                    let mut local_degraded = false;

                    for (contour_point_index, point) in
                        polygon.contour.points.iter().enumerate()
                    {
                        match point_in_paint_region(
                            &paint_regions,
                            layer_index,
                            semantic,
                            *point,
                            BoundaryInclusion::Include,
                            paint_region_rtree.as_deref(),
                        ) {
                            Ok(Some(value)) => {
                                point_paint.push(Some(value));
                            }
                            Ok(None) => {
                                // Point is not in any paint region for this semantic.
                                // Two cases:
                                //   (a) point is within integer-grid quantization noise
                                //       (≤ 1 unit ≈ 100 nm) of a paint-region edge —
                                //       genuine numerical edge ambiguity; emit a 504
                                //       and use the deterministic fallback.
                                //   (b) point is unambiguously outside all painted
                                //       regions — silently use a region-level default
                                //       so the semantic still carries a value.
                                if is_point_numerically_ambiguous(
                                    semantic_regions,
                                    *point,
                                    layer_index,
                                    semantic,
                                    paint_region_rtree.as_deref(),
                                ) {
                                    let fallback_value =
                                        default_fallback_value(semantic);
                                    point_paint.push(Some(fallback_value.clone()));
                                    local_warnings.push(
                                        SlicePostProcessPaintAnnotationWarning {
                                            code: 504,
                                            global_layer_index: layer_index,
                                            object_id: object_id.clone(),
                                            region_id,
                                            semantic: semantic.clone(),
                                            polygon_index,
                                            contour_point_index,
                                            fallback_value,
                                            reason:
                                                SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
                                        },
                                    );
                                    local_degraded = true;
                                } else {
                                    if !semantic_regions.is_empty() {
                                        point_paint.push(Some(
                                            semantic_regions[0].value.clone(),
                                        ));
                                    } else {
                                        point_paint.push(None);
                                    }
                                }
                            }
                            Err(PaintRegionQueryError::DeterministicConflict) => {
                                return Err(
                                    SlicePostProcessPaintAnnotationError::DeterministicConflict {
                                        code: 503,
                                        global_layer_index: layer_index,
                                        object_id: object_id.clone(),
                                        region_id,
                                        semantic: semantic.clone(),
                                        polygon_index,
                                        contour_point_index,
                                    },
                                );
                            }
                        }
                    }

                    Ok((point_paint, local_warnings, local_degraded))
                })
                .collect();

            let polygon_results = polygon_results?;

            let mut semantic_paint: Vec<Vec<Option<PaintValue>>> =
                Vec::with_capacity(polygon_results.len());
            for (point_paint, w, d) in polygon_results {
                semantic_paint.push(point_paint);
                warnings.extend(w);
                if d {
                    degraded = true;
                }
            }

            region
                .boundary_paint
                .insert(semantic.clone(), semantic_paint);
        }

        // Modifier-volume fuzzy-skin annotation (packet 56b)
        // Runs AFTER paint annotation: overrides default Flag(false) with
        // Flag(true) for contour points that fall inside modifier projections.
        if required_semantics.contains(&PaintSemantic::FuzzySkin)
            && !modifier_projections.is_empty()
        {
            if let Some(existing) = region.boundary_paint.get_mut(&PaintSemantic::FuzzySkin) {
                for (polygon_index, polygon_paint) in existing.iter_mut().enumerate() {
                    if polygon_index >= region.polygons.len() {
                        continue;
                    }
                    let polygon = &region.polygons[polygon_index];
                    for (point_index, paint_slot) in polygon_paint.iter_mut().enumerate() {
                        if point_index >= polygon.contour.points.len() {
                            continue;
                        }
                        let point = polygon.contour.points[point_index];
                        let in_modifier = modifier_projections.iter().any(|proj| {
                            ex_polygon_contains_point(proj, point, BoundaryInclusion::Include)
                        });
                        if in_modifier {
                            *paint_slot = Some(PaintValue::Flag(true));
                        }
                    }
                }
            } else if !region.polygons.is_empty() {
                // No existing FuzzySkin annotation at all — create one from scratch
                // using modifier projections only
                let mut semantic_paint: Vec<Vec<Option<PaintValue>>> =
                    Vec::with_capacity(region.polygons.len());
                for polygon in &region.polygons {
                    let mut point_paint: Vec<Option<PaintValue>> =
                        vec![None; polygon.contour.points.len()];
                    for (point_index, point) in polygon.contour.points.iter().enumerate() {
                        let in_modifier = modifier_projections.iter().any(|proj| {
                            ex_polygon_contains_point(proj, *point, BoundaryInclusion::Include)
                        });
                        if in_modifier {
                            point_paint[point_index] = Some(PaintValue::Flag(true));
                        }
                    }
                    semantic_paint.push(point_paint);
                }
                region
                    .boundary_paint
                    .insert(PaintSemantic::FuzzySkin, semantic_paint);
            }
        }
    }

    Ok(SlicePostProcessPaintAnnotationResult {
        slice_ir,
        degraded,
        warnings,
    })
}

/// Returns the deterministic fallback value for a paint semantic when a point
/// cannot be unambiguously resolved.
fn default_fallback_value(semantic: &PaintSemantic) -> PaintValue {
    match semantic {
        PaintSemantic::Material => PaintValue::ToolIndex(0),
        PaintSemantic::FuzzySkin => PaintValue::Flag(false),
        PaintSemantic::SupportEnforcer => PaintValue::Flag(false),
        PaintSemantic::SupportBlocker => PaintValue::Flag(false),
        PaintSemantic::Custom(_) => PaintValue::Scalar(0.0),
    }
}

/// Checks if a point is numerically ambiguous (very close to but not inside
/// any paint region polygon). This detects edge cases where floating point
/// precision or slight polygon modifications may have moved a point just
/// outside its original containing region.
fn is_point_numerically_ambiguous(
    regions: &[SemanticRegion],
    point: slicer_ir::Point2,
    layer_index: u32,
    semantic: &PaintSemantic,
    rtree_index: Option<&PaintRegionRTreeIndex>,
) -> bool {
    if regions.is_empty() {
        return false;
    }

    // A point only counts as numerically ambiguous when it lies within the
    // integer-grid quantization noise of a paint-region edge. The coordinate
    // unit is 100 nm, so 1 unit is the smallest representable non-zero
    // displacement — anything strictly above 1 unit is real separation, not
    // ambiguity. (A previous value of 10 units / 1 µm was inherited from a
    // hand-crafted test fixture and produced thousands of false positives on
    // paint regions composed of un-unioned per-facet projected triangles.)
    const EPSILON_UNITS: i64 = 1;

    if let Some(rtree_index) = rtree_index {
        let tree = match rtree_index.trees.get(&layer_index) {
            Some(layer_trees) => match layer_trees.get(semantic) {
                Some(tree) => tree,
                None => return false,
            },
            None => return false,
        };

        if tree.size() == 0 {
            return false;
        }

        let eps = EPSILON_UNITS as f64;
        let lo = [point.x as f64 - eps, point.y as f64 - eps];
        let hi = [point.x as f64 + eps, point.y as f64 + eps];
        let query_aabb = AABB::from_corners(lo, hi);

        let candidate_indices: Vec<usize> = tree
            .locate_in_envelope_intersecting(&query_aabb)
            .map(|entry| entry.region_index)
            .collect();

        for &candidate_index in &candidate_indices {
            let region = &regions[candidate_index];
            for polygon in &region.polygons {
                if is_point_near_polygon_edge(&polygon.contour.points, point, EPSILON_UNITS) {
                    return true;
                }
            }
        }

        false
    } else {
        for region in regions {
            for polygon in &region.polygons {
                if is_point_near_polygon_edge(&polygon.contour.points, point, EPSILON_UNITS) {
                    return true;
                }
            }
        }

        false
    }
}

/// Checks if a point is within epsilon distance of any edge of a polygon.
fn is_point_near_polygon_edge(
    points: &[slicer_ir::Point2],
    point: slicer_ir::Point2,
    epsilon: i64,
) -> bool {
    if points.len() < 2 {
        return false;
    }

    for i in 0..points.len() {
        let start = points[i];
        let end = points[(i + 1) % points.len()];

        if distance_point_to_segment_squared(point, start, end) <= (epsilon * epsilon) as i128 {
            return true;
        }
    }

    false
}

/// Computes the squared distance from a point to a line segment.
fn distance_point_to_segment_squared(
    point: slicer_ir::Point2,
    start: slicer_ir::Point2,
    end: slicer_ir::Point2,
) -> i128 {
    let dx = i128::from(end.x - start.x);
    let dy = i128::from(end.y - start.y);
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0 {
        // Degenerate segment (start == end)
        let px = i128::from(point.x - start.x);
        let py = i128::from(point.y - start.y);
        return px * px + py * py;
    }

    // Compute projection parameter t
    let px = i128::from(point.x - start.x);
    let py = i128::from(point.y - start.y);
    let dot = px * dx + py * dy;

    // Clamp t to [0, 1]
    let t_num = dot.max(0).min(len_sq);

    // Closest point on segment is start + t * (end - start)
    // Distance squared = |point - closest|^2
    // = |point - start - t*(end-start)|^2
    // = (px - t*dx)^2 + (py - t*dy)^2

    // Using scaled arithmetic to avoid floating point:
    // closest = start + (t_num/len_sq) * (end - start)
    // distance^2 = (point - closest)^2

    // We compute: (px*len_sq - t_num*dx)^2 + (py*len_sq - t_num*dy)^2
    // Then divide by len_sq^2

    let scaled_dx = px * len_sq - t_num * dx;
    let scaled_dy = py * len_sq - t_num * dy;

    (scaled_dx * scaled_dx + scaled_dy * scaled_dy) / (len_sq * len_sq)
}
