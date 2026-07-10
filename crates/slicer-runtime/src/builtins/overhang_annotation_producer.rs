//! BuiltinProducer wrapper for the host-side `PrePass::OverhangAnnotation` step.
//!
//! The pure kernel lives in `slicer_core::algos::overhang_annotation`. This
//! thin wrapper bridges from `Blackboard` (committed `SliceIR`) to the IR-only
//! kernel and merges the per-object results back into the already-committed
//! `SurfaceClassificationIR.overhang_quartile_polygons`.
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
//! # Multi-object merge
//!
//! `SurfaceClassificationIR.overhang_quartile_polygons` is keyed by *global*
//! layer index, not per-object, so when a mesh has more than one object each
//! object's per-layer `QuartileBand` list is computed independently from that
//! object's `SliceIR` footprints, then merged **by quartile**: all objects'
//! band-`k` polygons are concatenated (in `mesh.objects` iteration order) into
//! a single `QuartileBand` with `quartile == k`. Each layer therefore carries
//! at most 4 bands, one per quartile, sorted by quartile — preserving the
//! design.md locked assumption ("inner Vec carries one `QuartileBand` per
//! quartile") regardless of object count, so consumers may safely index/`find`
//! by `quartile`.
//!
//! # Line width resolution
//!
//! `outer_wall_line_width` is read from the raw config source, falling back
//! to `line_width`, and finally to `0.4` mm if neither is present — matching
//! the per-invocation config-read convention documented on
//! `annotate_overhangs`'s module doc-comment and mirrored by
//! `classic-perimeters`/`arachne-perimeters`'s guest-side config reads.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::slice_ir::QuartileBand;
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
    // layer index -> quartile -> polygons from every object, in `mesh.objects`
    // iteration order (deterministic).
    let mut per_quartile: HashMap<u32, HashMap<u8, Vec<ExPolygon>>> = HashMap::new();
    for object in &mesh.objects {
        // This object's per-layer footprint, in plan order: the concatenated
        // region polygons it owns in each committed `SliceIR` (empty where the
        // object is not active). `difference_ex` unions the subject internally,
        // so overlapping sibling regions need no explicit pre-union. Keyed by
        // each slice's `global_layer_index` — the same key the WIT marshal uses
        // to hand overhang polygons to layer-tier modules.
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

        let per_object = slicer_core::algos::overhang_annotation::annotate_overhangs(
            &layer_footprints,
            line_width_mm,
        );
        for (layer_index, bands) in per_object {
            let layer_entry = per_quartile.entry(layer_index).or_default();
            for band in bands {
                layer_entry
                    .entry(band.quartile)
                    .or_default()
                    .extend(band.polygons);
            }
        }
    }
    let merged: HashMap<u32, Vec<QuartileBand>> = per_quartile
        .into_iter()
        .map(|(layer_index, by_quartile)| {
            let mut bands: Vec<QuartileBand> = by_quartile
                .into_iter()
                .map(|(quartile, polygons)| QuartileBand { quartile, polygons })
                .collect();
            bands.sort_by_key(|b| b.quartile);
            (layer_index, bands)
        })
        .collect();

    let updated = SurfaceClassificationIR {
        overhang_quartile_polygons: merged,
        ..surface_classification.as_ref().clone()
    };

    blackboard
        .replace_surface_classification(Arc::new(updated))
        .map_err(|source| OverhangAnnotationBuiltinError::Blackboard { source })
}
