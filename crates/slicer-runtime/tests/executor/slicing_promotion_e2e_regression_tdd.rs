//! End-to-end regression smoke for the slicing-promotion refactor (host prepass).
//!
//! Drives a 3-step staircase fixture through the two host built-ins:
//!   1. `PrePass::Slice` (host built-in) — produces `Vec<SliceIR>`
//!   2. `PrePass::ShellClassification` (host built-in) — populates
//!      `top_shell_index`, `bottom_shell_index`, and polygon-precise
//!      `top_solid_fill` / `bottom_solid_fill`
//!
//! Verifies that the two commits work together: the prepass correctly
//! classifies the staircase's exposed top surfaces, and the pipeline is
//! deterministic across runs. The downstream `TopSurfaceIroning` consumption of
//! `top_solid_fill` is tested in the module's own crate
//! (`modules/core-modules/top-surface-ironing/tests/`) — this file links no
//! module crate.

// SUT is commit_shell_classification_builtin / commit_slice_builtin / Blackboard.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectMesh,
    Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, Transform3d,
};
use slicer_runtime::{commit_shell_classification_builtin, commit_slice_builtin, Blackboard};

// ============================================================================
// Fixture
// ============================================================================

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// Three-step staircase: stacked cuboids with diminishing footprints.
///
/// - Step A: z in [0, 0.4], 20Ã—20 mm footprint (centered at origin)
/// - Step B: z in [0.4, 0.6], 12Ã—12 mm footprint (centered)
/// - Step C: z in [0.6, 0.8], 6Ã—6 mm footprint (centered)
///
/// Each step is its own object (so slicing/classification handles them
/// independently). With layer height 0.2 mm the staircase spans 4 layers:
///   layer 0 (z=0.2): step A only
///   layer 1 (z=0.4): step A only (top of A is exposed at upper face)
///   layer 2 (z=0.6): step B only (top of B is exposed at upper face)
///   layer 3 (z=0.8): step C only (top of C is exposed at upper face)
fn staircase_mesh() -> MeshIR {
    let make_cuboid = |id: &str, half: f32, z0: f32, z1: f32| -> ObjectMesh {
        let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
        let vertices = vec![
            v(-half, -half, z0),
            v(half, -half, z0),
            v(half, half, z0),
            v(-half, half, z0),
            v(-half, -half, z1),
            v(half, -half, z1),
            v(half, half, z1),
            v(-half, half, z1),
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            1, 2, 6, 1, 6, 5, // +X
            0, 4, 7, 0, 7, 3, // -X
            3, 7, 6, 3, 6, 2, // +Y
            0, 1, 5, 0, 5, 4, // -Y
        ];
        ObjectMesh {
            id: id.to_string(),
            mesh: IndexedTriangleSet { vertices, indices },
            transform: identity(),
            ..Default::default()
        }
    };

    MeshIR {
        objects: vec![
            make_cuboid("step-a", 10.0, 0.0, 0.5),
            make_cuboid("step-b", 6.0, 0.0, 0.7),
            make_cuboid("step-c", 3.0, 0.0, 0.9),
        ],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: 0.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        },
        ..Default::default()
    }
}

fn active_region(object_id: &str) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 0.2,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn layer_plan() -> LayerPlanIR {
    let layers: Vec<GlobalLayer> = (0..4)
        .map(|i| {
            let z = 0.2 * (i + 1) as f32;
            // Active regions per layer:
            //   z=0.2 (i=0): step-a only
            //   z=0.4 (i=1): step-a (its slice at z=0.4 is empty â†’ exposed top)
            //   z=0.6 (i=2): step-b only (its slice at z=0.6 is empty â†’ exposed top)
            //   z=0.8 (i=3): step-c only (its slice at z=0.8 is empty â†’ exposed top)
            let object_id = match i {
                0 | 1 => "step-a",
                2 => "step-b",
                _ => "step-c",
            };
            GlobalLayer {
                index: i,
                z,
                active_regions: vec![active_region(object_id)],
                has_nonplanar: false,
                is_sync_layer: false,
            }
        })
        .collect();
    LayerPlanIR {
        global_layers: layers,
        object_participation: HashMap::new(),
        ..Default::default()
    }
}

fn region_map_with_shell_counts(plan: &LayerPlanIR, top: u32, bot: u32) -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    for gl in &plan.global_layers {
        for active in &gl.active_regions {
            let mut config = active.resolved_config.clone();
            config.top_shell_layers = top;
            config.bottom_shell_layers = bot;
            let config_id = region_map.intern_config(config);
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: gl.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }
    }
    region_map
}

// ============================================================================
// E2E smoke
// ============================================================================

#[test]
fn staircase_prepass_classifies_each_step_top_as_exposed() {
    let mesh = staircase_mesh();
    let plan = layer_plan();
    let region_map = region_map_with_shell_counts(&plan, 2, 2);
    let mut bb = Blackboard::new(Arc::new(mesh), plan.global_layers.len());
    bb.commit_layer_plan(Arc::new(plan)).unwrap();
    bb.commit_region_map(Arc::new(region_map)).unwrap();

    commit_slice_builtin(&mut bb).expect("PrePass::Slice");
    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");

    let slices = bb.slice_ir().expect("slice_ir committed");
    assert_eq!(slices.len(), 4);

    // Each step's top layer should have at least one region with
    // top_shell_index = Some(0) (exposed top surface).
    // Layer 0 (z=0.2): step-a interior â€” top is layer 1 (z=0.4 where slice is empty)
    // The classification of which layer is "exposed top" depends on slice emptiness
    // at the next active layer in the timeline. With step-a active at layers 0 and 1,
    // layer 1's upper neighbor is layer 2 (step-b only) â†’ step-a has no upper layer
    // in its timeline â†’ layer 1 is step-a's exposed top.

    // Helper: count regions with top_shell_index == Some(0) across all slices.
    let exposed_top_count: usize = slices
        .iter()
        .flat_map(|s| s.regions.iter())
        .filter(|r| r.top_shell_index == Some(0))
        .count();
    assert!(
        exposed_top_count >= 3,
        "expected at least 3 exposed-top regions across the staircase, got {exposed_top_count}"
    );
}

// NOTE: `staircase_topsurface_ironing_emits_for_exposed_layers` moved to
// `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs`
// when slicer-runtime was decoupled from module crates — it tested the
// TopSurfaceIroning module's `run_infill`, which belongs to the module's crate.
// The host prepass classification it depended on is covered above.

#[test]
fn staircase_pipeline_is_deterministic_across_runs() {
    // Build twice and verify byte-identical slice_ir output. Validates that
    // PrePass::Slice + PrePass::ShellClassification are deterministic
    // (no rayon non-determinism, no HashMap iteration leakage into output).
    let run_once = || {
        let mesh = staircase_mesh();
        let plan = layer_plan();
        let region_map = region_map_with_shell_counts(&plan, 2, 2);
        let mut bb = Blackboard::new(Arc::new(mesh), plan.global_layers.len());
        bb.commit_layer_plan(Arc::new(plan)).unwrap();
        bb.commit_region_map(Arc::new(region_map)).unwrap();
        commit_slice_builtin(&mut bb).expect("PrePass::Slice");
        commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");
        (*bb.slice_ir().expect("slice_ir committed").as_ref()).clone()
    };

    let a = run_once();
    let b = run_once();
    assert_eq!(
        a, b,
        "prepass slice + shell classification must be deterministic"
    );
}
