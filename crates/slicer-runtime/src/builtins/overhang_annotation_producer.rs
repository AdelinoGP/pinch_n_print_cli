//! BuiltinProducer wrapper for the host-side `PrePass::OverhangAnnotation` step.
//!
//! The pure kernel lives in `slicer_core::algos::overhang_annotation`. This
//! thin wrapper bridges from `Blackboard` (committed `SliceIR`) to the IR-only
//! kernel and merges the per-object results back into the already-committed
//! `SurfaceClassificationIR`.
//!
//! # Overhang from slices (not a second mesh pass)
//!
//! This stage runs **after** `PrePass::Slice` and derives each object's
//! per-layer footprints from the committed `SliceIR` region polygons, then
//! diffs consecutive layers — matching OrcaSlicer's `detect_overhangs_for_lift`
//! (`PrintObject.cpp:880-908`), which diffs consecutive `lslices`. There is no
//! second mesh-slicing pass here; the object meshes are sliced exactly once,
//! in `PrePass::Slice`.
//!
//! # What it commits (ticket 142)
//!
//! No quartile-band polygons: the per-vertex quartile is derived downstream
//! from the signed distance to `prev_layer_boundaries`, so the stage commits
//! the unsupported area (`overhang_areas`), the previous-layer footprint, and
//! the band width the thresholds are multiples of (`overhang_band_width_mm`).
//! Building the four band polygon sets was 3 round offsets plus 6 whole-layer
//! booleans per overhang layer, and its value had no production reader.
//!
//! # Multi-object merge
//!
//! `SurfaceClassificationIR.overhang_areas` and `prev_layer_boundaries` are
//! keyed by object id first, then global layer index. Each object's per-layer
//! areas are computed independently from that object's `SliceIR` footprints,
//! preserving object isolation for consumers.
//!
//! # Line width resolution
//!
//! `outer_wall_line_width` is read from the raw config source, falling back
//! to `line_width`, and finally to `0.4` mm if neither is present — matching
//! the per-invocation config-read convention documented on
//! `annotate_overhangs`'s module doc-comment and mirrored by
//! `classic-perimeters`/`arachne-perimeters`'s guest-side config reads. The
//! resolved value is committed as `overhang_band_width_mm`.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, ExPolygon, SurfaceClassificationIR};

use crate::{Blackboard, BlackboardError};

/// Wrapper error used when the built-in runs on the real prepass path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverhangAnnotationBuiltinError {
    /// No `LayerPlanIR` committed to the blackboard yet.
    MissingLayerPlan,
    /// No `SurfaceClassificationIR` committed to the blackboard yet.
    MissingSurfaceClassification,
    /// No `SliceIR` committed to the blackboard yet. `PrePass::OverhangAnnotation`
    /// now runs after `PrePass::Slice` and derives overhang from the slices.
    MissingSliceIr,
    /// Blackboard commit failed (e.g. duplicate commit).
    Blackboard {
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
}

impl std::fmt::Display for OverhangAnnotationBuiltinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLayerPlan => write!(
                f,
                "built-in PrePass::OverhangAnnotation requires a committed LayerPlanIR"
            ),
            Self::MissingSurfaceClassification => write!(
                f,
                "built-in PrePass::OverhangAnnotation requires a committed SurfaceClassificationIR"
            ),
            Self::MissingSliceIr => write!(
                f,
                "built-in PrePass::OverhangAnnotation requires a committed SliceIR \
                 (it now runs after PrePass::Slice)"
            ),
            Self::Blackboard { source } => {
                write!(
                    f,
                    "built-in PrePass::OverhangAnnotation commit failed: {source}"
                )
            }
        }
    }
}

impl std::error::Error for OverhangAnnotationBuiltinError {}

/// Resolves the line width (mm) used for quartile-band thresholds:
/// `outer_wall_line_width`, falling back to `line_width`, falling back to
/// `0.4` mm (matches the guest-side default used by
/// `classic-perimeters`/`arachne-perimeters`).
fn resolve_line_width_mm(raw_config_source: &HashMap<ConfigKey, ConfigValue>) -> f32 {
    let legacy = match raw_config_source.get("line_width") {
        Some(ConfigValue::Float(w)) => *w as f32,
        _ => 0.4,
    };
    match raw_config_source.get("outer_wall_line_width") {
        Some(ConfigValue::Float(w)) => *w as f32,
        _ => legacy,
    }
}

/// Run the overhang-annotation kernel over every object in the blackboard's
/// mesh and merge the results into a replacement `SurfaceClassificationIR`.
///
/// Requires `LayerPlanIR`, `SurfaceClassificationIR`, and `SliceIR` to already
/// be committed (the last by `PrePass::Slice`, which this stage now runs
/// after). Idempotent only in the sense that re-calling after a successful run
/// recomputes and re-replaces the same deterministic result; callers should
/// guard on `blackboard.slice_ir().is_some()` (see `run_builtin_stage`'s
/// `should_run` closure in `prepass.rs`) to avoid redundant recomputation.
pub fn commit_overhang_annotation_builtin(
    blackboard: &mut Blackboard,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
) -> Result<(), OverhangAnnotationBuiltinError> {
    if blackboard.layer_plan().is_none() {
        return Err(OverhangAnnotationBuiltinError::MissingLayerPlan);
    }
    let Some(surface_classification) = blackboard.surface_classification().cloned() else {
        return Err(OverhangAnnotationBuiltinError::MissingSurfaceClassification);
    };
    let Some(slice_ir) = blackboard.slice_ir().cloned() else {
        return Err(OverhangAnnotationBuiltinError::MissingSliceIr);
    };

    let line_width_mm = resolve_line_width_mm(raw_config_source);

    let mesh = blackboard.mesh();
    // object id -> layer index -> unsupported area / previous-layer footprint.
    let mut overhang_areas: HashMap<String, HashMap<u32, Vec<ExPolygon>>> = HashMap::new();
    let mut prev_layer_boundaries: HashMap<String, HashMap<u32, Vec<ExPolygon>>> = HashMap::new();
    for object in &mesh.objects {
        // This object's per-layer footprint, in plan order: the concatenated
        // region polygons it owns in each committed `SliceIR` (empty where the
        // object is not active). `difference_ex` unions the subject internally,
        // so overlapping sibling regions need no explicit pre-union. Keyed by
        // each slice's `global_layer_index` — the same key the WIT marshal uses
        // to hand overhang anchors to layer-tier modules.
        let layer_footprints: Vec<(u32, Vec<ExPolygon>)> = slice_ir
            .iter()
            .map(|slice| {
                let footprint: Vec<ExPolygon> = slice
                    .regions
                    .iter()
                    .filter(|region| region.object_id == object.id)
                    .flat_map(|region| region.polygons.iter().cloned())
                    .collect();
                (slice.global_layer_index, footprint)
            })
            .collect();

        let annotation =
            slicer_core::algos::overhang_annotation::annotate_overhangs(&layer_footprints);
        overhang_areas.insert(object.id.clone(), annotation.overhang_areas);
        prev_layer_boundaries.insert(object.id.clone(), annotation.prev_layer_boundaries);
    }

    let updated = SurfaceClassificationIR {
        // Production no longer carries band polygons; the diagnostic renderer
        // reconstructs them from the areas + boundary + band width.
        overhang_quartile_polygons: HashMap::new(),
        overhang_areas,
        prev_layer_boundaries,
        overhang_band_width_mm: line_width_mm,
        ..surface_classification.as_ref().clone()
    };

    blackboard
        .replace_surface_classification(Arc::new(updated))
        .map_err(|source| OverhangAnnotationBuiltinError::Blackboard { source })
}
