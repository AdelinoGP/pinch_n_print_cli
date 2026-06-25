#![allow(missing_docs)]

//! TDD test for packet 35a AC-4: `commit_region_mapping_builtin` stamps each
//! `RegionPlan.config` from the per-object `resolved_configs` map.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectConfig, ObjectMesh, Point3, RegionKey, ResolvedConfig, SemVer, Transform3d,
};
use slicer_runtime::{
    build_execution_plan, commit_region_mapping_builtin, Blackboard, ExecutionPlanRequest,
    LoadDiagnostic, SortedStageModules,
};

// --- helpers ----------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn minimal_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "obj-A".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn active_region(object_id: &str, region_id: u64) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 0.2,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn empty_execution_plan() -> slicer_runtime::ExecutionPlan {
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::<SortedStageModules>::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    build_execution_plan(&request, &mut diagnostics).expect("empty execution plan should build")
}

// --- AC-4 test --------------------------------------------------------------

#[test]
fn commit_stamps_per_object_resolved_config() {
    // Build a LayerPlanIR with one layer containing two active regions:
    // one on "obj-A" and one on "obj-B".
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("obj-A", 1), active_region("obj-B", 1)],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    });

    // Build per-object resolved configs:
    // obj-A.top_shell_layers = 5, obj-B.top_shell_layers = 3.
    let config_a = ResolvedConfig {
        top_shell_layers: 5,
        ..ResolvedConfig::default()
    };

    let config_b = ResolvedConfig {
        top_shell_layers: 3,
        ..ResolvedConfig::default()
    };

    let mut resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    resolved_configs.insert("obj-A".to_string(), config_a);
    resolved_configs.insert("obj-B".to_string(), config_b);

    let default_resolved_config = ResolvedConfig::default();

    // Build a blackboard and commit the layer plan.
    let mesh = Arc::new(minimal_mesh());
    let mut blackboard = Blackboard::new(mesh, 0);
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("commit layer plan");

    let plan = empty_execution_plan();

    // Invoke commit_region_mapping_builtin directly (not via execute_prepass_with_builtins).
    commit_region_mapping_builtin(
        &plan,
        &mut blackboard,
        &resolved_configs,
        &default_resolved_config,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
    )
    .expect("commit_region_mapping_builtin must succeed");

    let rm = blackboard
        .region_map()
        .expect("RegionMapIR must be committed after builtin runs");

    // Exactly two entries.
    assert_eq!(rm.entries.len(), 2, "expected exactly 2 region entries");

    // obj-A entry has top_shell_layers == 5.
    let key_a = RegionKey {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    assert!(
        rm.entries.contains_key(&key_a),
        "entry for obj-A must be present"
    );
    let resolved_a = rm.config_for(&key_a);
    assert_eq!(
        resolved_a.top_shell_layers, 5,
        "obj-A region plan must have top_shell_layers=5 from per-object config"
    );

    // obj-B entry has top_shell_layers == 3.
    let key_b = RegionKey {
        global_layer_index: 0,
        object_id: "obj-B".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    assert!(
        rm.entries.contains_key(&key_b),
        "entry for obj-B must be present"
    );
    let resolved_b = rm.config_for(&key_b);
    assert_eq!(
        resolved_b.top_shell_layers, 3,
        "obj-B region plan must have top_shell_layers=3 from per-object config"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// P95 W-P1: loader → ResolvedConfig.extensions["extruder"] chain
//
// The 3MF loader (slicer-model-io::loader) rebases OrcaSlicer's 1-indexed
// `extruder` metadata to the runtime's 0-indexed convention before stamping
// `ObjectMesh.config.data["extruder"] = ConfigValue::Int(0)` (for raw
// `extruder=1` in the 3MF). `run.rs` then lifts each `ObjectMesh.config.data`
// entry into a `config_source` key of the form `object_config:<obj>:<key>`,
// which `resolve_per_object_configs` (slicer-scheduler) overlays onto each
// per-object `ResolvedConfig`. Because `extruder` is not a declared
// `ResolvedConfig` field, it must fall through to the `extensions` overflow
// bucket as `ConfigValue::Int(0)`.
//
// This test pins that handoff so a regression in any of the three hops
// (loader rebase / `run.rs` lift / scheduler overlay) is caught here rather
// than only at the gcode-output gate.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn loader_extruder_int_zero_lands_in_extensions() {
    use slicer_scheduler::{resolve_global_config, resolve_per_object_configs, ConfigBoundsIndex};

    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    // Simulate the `run.rs` lift of `ObjectMesh.config.data["extruder"] = Int(0)`
    // (which is what the loader stamps for OrcaSlicer-1-indexed `extruder=1`).
    source.insert(
        "object_config:obj-A:extruder".to_string(),
        ConfigValue::Int(0),
    );

    let bounds = ConfigBoundsIndex::default();
    let global = resolve_global_config(&source, &bounds).expect("global resolution must succeed");
    let per_object = resolve_per_object_configs(&global, &source, &["obj-A"], &bounds)
        .expect("per-object resolution must succeed");

    let cfg = per_object
        .get("obj-A")
        .expect("obj-A must have a per-object resolved config");

    assert_eq!(
        cfg.extensions.get("extruder"),
        Some(&ConfigValue::Int(0)),
        "loader-stamped `Int(0)` for `extruder` must land in `ResolvedConfig.extensions` \
         (Int(0) is meaningful — tool 0 — and must not be dropped or coerced)"
    );
}
