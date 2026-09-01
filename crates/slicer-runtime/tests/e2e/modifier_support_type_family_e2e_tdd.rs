//! Ticket 18 e2e (test 2): per-region support-family binding reaches the
//! support analysis through the PRODUCTION call sites.
//!
//! Shape follows the `mixed_density_internal_bridge_rejection` template (packet
//! 234a AC-N1): a synthetic model — one object carrying a parameter modifier
//! whose `config_delta` sets `support_type=tree(auto)` — drives the host
//! prepass builtins that production actually runs:
//!
//! 1. `commit_region_mapping_builtin` (the production RegionMapIR builder) —
//!    NOT a hand-built map, which is what every pre-ticket-18 test had to do.
//! 2. `commit_support_analysis_builtin` — proves the per-region family
//!    assignment reaches `SupportAnalysisIR.family_assignments`.
//!
//! Frozen bar: the base region's assignment stays "traditional" (the global
//! family — a modifier must never change the whole object's family), and the
//! minted sub-region — keyed by the same deterministic id the Tier-2 geometry
//! split will mint — is assigned "tree".

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ModifierScope, ModifierVolume, ObjectMesh, PaintSemantic,
    Point2, Point3, Polygon, RegionKey, ResolvedConfig, SemVer, SliceIR, SlicedRegion, Transform3d,
};
use slicer_runtime::{
    builtins::support_analysis_producer::commit_support_analysis_builtin,
    commit_region_mapping_builtin, Blackboard, ExecutionPlanRequest, LoadDiagnostic,
    SortedStageModules,
};

const OBJECT_ID: &str = "obj-0";
const LAYER_Z: f32 = 0.5;

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

/// A 1 mm-tall cube spanning `x0..x1` in X, `0..10` in Y, `0..1` in Z — its
/// mid-plane slice at z = 0.5 is a non-empty square.
fn modifier_cube_mesh(x0: f32, x1: f32) -> IndexedTriangleSet {
    let y1 = 10.0f32;
    let z1 = 1.0f32;
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(x0, 0.0, 0.0),
            v(x1, 0.0, 0.0),
            v(x1, y1, 0.0),
            v(x0, y1, 0.0),
            v(x0, 0.0, z1),
            v(x1, 0.0, z1),
            v(x1, y1, z1),
            v(x0, y1, z1),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0,
            7, 3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

/// Parameter modifier carrying `support_type=tree(auto)` on the right half of
/// the object (x 10..20) on every layer.
fn tree_support_modifier() -> ModifierVolume {
    // exhaustive: ModifierVolume has no Default impl; every field is a fixture input
    ModifierVolume {
        id: "mod-tree".to_string(),
        mesh: modifier_cube_mesh(10.0, 20.0),
        config_delta: ConfigDelta {
            fields: HashMap::from([(
                "support_type".to_string(),
                ConfigValue::String("tree(auto)".to_string()),
            )]),
        },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
        // exhaustive: ModifierVolume fixture preserves every field explicitly
    }
}

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

fn mesh_with_modifier() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: IndexedTriangleSet::default(),
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            modifier_volumes: vec![tree_support_modifier()],
            ..Default::default()
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

fn layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: LAYER_Z,
            active_regions: vec![ActiveRegion {
                object_id: OBJECT_ID.to_string(),
                region_id: 0,
                effective_layer_height: 0.2,
                ..Default::default()
            }],
            ..Default::default()
        }],
        object_participation: Default::default(),
    }
}

fn slice_ir() -> SliceIR {
    SliceIR {
        schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: LAYER_Z,
        regions: vec![SlicedRegion {
            object_id: OBJECT_ID.to_string(),
            region_id: 0,
            polygons: vec![square(0.0, 0.0, 20.0, 10.0)],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
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
    slicer_runtime::build_execution_plan(&request, &mut diagnostics)
        .expect("empty execution plan should build")
}

#[test]
fn modifier_support_type_binds_to_minted_sub_region_in_production() {
    // The minted sub-region id is a pure function of (base id, object id,
    // footprint polygons) — the Tier-2 split (`split_modifier_footprints`)
    // and the region-map kernel re-derive it from the SAME inputs, so the test
    // pins the expected id by computing it the same way.
    let expected_sub_id = {
        let mv = tree_support_modifier();
        let polygons = slicer_core::slice_mesh_ex(&mv.mesh, &[LAYER_Z])
            .into_iter()
            .next()
            .expect("modifier cube must slice at z=0.5");
        slicer_ir::modifier_sub_region_id(0, OBJECT_ID, &polygons)
    };

    // Phase 1 — production region mapping. The base config is the global
    // family (no `support_type` anywhere); the modifier's delta must land on
    // the minted sub-region only.
    let mesh = Arc::new(mesh_with_modifier());
    let mut blackboard = Blackboard::new(mesh, 1);
    blackboard
        .commit_layer_plan(Arc::new(layer_plan()))
        .expect("commit layer plan");

    let resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::from([(
        OBJECT_ID.to_string(),
        ResolvedConfig::default(),
    )]);
    let default_resolved_config = ResolvedConfig::default();
    let plan = empty_execution_plan();
    let paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    let tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();

    commit_region_mapping_builtin(
        &plan,
        &mut blackboard,
        &resolved_configs,
        &default_resolved_config,
        &paint_semantic_configs,
        &tool_configs,
    )
    .expect("commit_region_mapping_builtin must succeed");

    let region_map = blackboard
        .region_map()
        .expect("RegionMapIR must be committed");

    let base_key = RegionKey {
        global_layer_index: 0,
        object_id: OBJECT_ID.to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let sub_key = RegionKey {
        global_layer_index: 0,
        object_id: OBJECT_ID.to_string(),
        region_id: expected_sub_id,
        variant_chain: Vec::new(),
    };

    // Base region: pure base config — the modifier's support_type must NOT
    // leak object-wide (this is the pre-ticket-18 bug this ticket fixes).
    let base_cfg = region_map.config_for(&base_key);
    assert!(
        !base_cfg.extensions.contains_key("support_type"),
        "base region must keep the global family; got extensions={:?}",
        base_cfg.extensions
    );

    // Minted sub-region: carries the modifier's delta, under the exact id the
    // Tier-2 geometry split will mint.
    let sub_cfg = region_map.config_for(&sub_key);
    assert_eq!(
        sub_cfg.extensions.get("support_type"),
        Some(&ConfigValue::String("tree(auto)".to_string())),
        "minted sub-region must carry the owning modifier's support_type; got extensions={:?}",
        sub_cfg.extensions
    );

    // Phase 2 — support analysis: per-region family assignment must reach
    // `family_assignments` through the production builtin.
    blackboard
        .commit_slice_ir(Arc::new(vec![slice_ir()]))
        .expect("commit slice IR");
    let analysis_config = ResolvedConfig {
        support_enabled: true,
        ..ResolvedConfig::default()
    };
    commit_support_analysis_builtin(&mut blackboard, &analysis_config)
        .expect("commit_support_analysis_builtin must succeed");

    let analysis = blackboard
        .support_analysis()
        .expect("SupportAnalysisIR must be committed");

    assert_eq!(
        analysis.family_assignments.get(&(OBJECT_ID.to_string(), 0)),
        Some(&"traditional".to_string()),
        "base region must keep the global (traditional) family"
    );
    assert_eq!(
        analysis
            .family_assignments
            .get(&(OBJECT_ID.to_string(), expected_sub_id)),
        Some(&"tree".to_string()),
        "minted sub-region must be assigned the tree family from the modifier's support_type"
    );
}
