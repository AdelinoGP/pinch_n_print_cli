//! BuiltinProducer wrapper for the host-side `PrePass::OverhangAnnotation` step.
//!
//! The pure kernel lives in `slicer_core::algos::overhang_annotation`. This
//! thin wrapper bridges from `Blackboard` (mesh + committed `LayerPlanIR`) to
//! the IR-only kernel and merges the per-object results back into the
//! already-committed `SurfaceClassificationIR.overhang_quartile_polygons`.
//!
//! # Multi-object merge
//!
//! `SurfaceClassificationIR.overhang_quartile_polygons` is keyed by *global*
//! layer index, not per-object, so when a mesh has more than one object each
//! object's per-layer `QuartileBand` list is computed independently (via
//! `annotate_overhangs(&object.mesh, ...)`, mirroring how
//! `mesh_analysis::execute_mesh_analysis` and
//! `prepass_slice::execute_prepass_slice` iterate `mesh.objects`) and then
//! merged **by quartile**: all objects' band-`k` polygons are concatenated
//! (in `mesh.objects` iteration order) into a single `QuartileBand` with
//! `quartile == k`. Each layer therefore carries at most 4 bands, one per
//! quartile, sorted by quartile — preserving the design.md locked assumption
//! ("inner Vec carries one `QuartileBand` per quartile") regardless of
//! object count, so consumers may safely index/`find` by `quartile`.
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
use slicer_ir::{ConfigKey, ConfigValue, IndexedTriangleSet, SurfaceClassificationIR};

use crate::{Blackboard, BlackboardError};

/// Returns a copy of `mesh` with `transform` applied to every vertex.
///
/// `annotate_overhangs` (via `cross_section_at_z`) requires its input mesh
/// in world/global space — see `cross_section_at_z`'s doc-comment
/// (`crates/slicer-core/src/algos/mesh_cross_section.rs`): "Callers slicing a
/// `MeshIR` object should pass `object_mesh.mesh` (applying any needed
/// transform beforehand)". `ObjectMesh::mesh` is local-space, so this helper
/// must run before cross-sectioning; mirrors the transform-application
/// pattern in `slicer_core::algos::prepass_slice`'s per-triangle
/// `transform_point` calls, but pre-applies to the whole mesh once since
/// `annotate_overhangs` takes a bare `IndexedTriangleSet`.
fn world_space_mesh(
    mesh: &IndexedTriangleSet,
    transform: &slicer_ir::Transform3d,
) -> IndexedTriangleSet {
    let vertices = mesh
        .vertices
        .iter()
        .map(|p| slicer_core::transform_point3(&transform.matrix, *p))
        .collect();
    IndexedTriangleSet {
        vertices,
        indices: mesh.indices.clone(),
    }
}

/// Wrapper error used when the built-in runs on the real prepass path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverhangAnnotationBuiltinError {
    /// No `LayerPlanIR` committed to the blackboard yet.
    MissingLayerPlan,
    /// No `SurfaceClassificationIR` committed to the blackboard yet.
    MissingSurfaceClassification,
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
/// Requires `LayerPlanIR` and `SurfaceClassificationIR` to already be
/// committed (the latter by `PrePass::MeshAnalysis`). Idempotent only in the
/// sense that re-calling after a successful run recomputes and re-replaces
/// the same deterministic result; callers should guard on
/// `blackboard.layer_plan().is_some()` (see `run_builtin_stage`'s
/// `should_run` closure in `prepass.rs`) to avoid redundant recomputation.
pub fn commit_overhang_annotation_builtin(
    blackboard: &mut Blackboard,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
) -> Result<(), OverhangAnnotationBuiltinError> {
    let Some(layer_plan) = blackboard.layer_plan().cloned() else {
        return Err(OverhangAnnotationBuiltinError::MissingLayerPlan);
    };
    let Some(surface_classification) = blackboard.surface_classification().cloned() else {
        return Err(OverhangAnnotationBuiltinError::MissingSurfaceClassification);
    };

    let layer_zs: Vec<f32> = layer_plan.global_layers.iter().map(|gl| gl.z).collect();
    let line_width_mm = resolve_line_width_mm(raw_config_source);

    let mesh = blackboard.mesh();
    // layer index -> quartile -> polygons from every object, in `mesh.objects`
    // iteration order (deterministic).
    let mut per_quartile: HashMap<u32, HashMap<u8, Vec<slicer_ir::ExPolygon>>> = HashMap::new();
    for object in &mesh.objects {
        let world_mesh = world_space_mesh(&object.mesh, &object.transform);
        let per_object = slicer_core::algos::overhang_annotation::annotate_overhangs(
            &world_mesh,
            &layer_zs,
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
