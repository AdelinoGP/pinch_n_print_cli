#![allow(missing_docs)]

//! DEV-045 / DEV-061 regression — host paint-segmentation fallback MUST
//! commit `PaintRegionIR` BEFORE the region-mapping built-in so that
//! `paint_config:<semantic>:<key>` overrides reach
//! `RegionPlan.paint_overrides` (and, when the region overlaps the
//! painted polygon, `RegionPlan.config`).
//!
//! Pre-fix the fallback at `crates/slicer-host/src/prepass.rs:633-666`
//! ran AFTER region-mapping, silently dropping every user-supplied paint
//! override on the live phase-2 path. This test drives the full prepass
//! (including both built-in fallbacks) and asserts BOTH the overlay-stored
//! and overlay-applied contracts. A unit test at
//! `region_mapping_paint_semantic_tdd::region_overlap_applies_override`
//! covers the overlay-applied contract with a hand-built `PaintRegionIR`
//! and `execute_region_mapping`; only this test drives the full prepass
//! dispatcher and catches the ordering bug.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_host::{
    build_execution_plan, execute_prepass_with_builtins_configured, Blackboard, CompiledModule,
    ConfigBoundsIndex, ExecutionPlanRequest, PrepassExecutionError, PrepassStageOutput,
    PrepassStageRunner, SortedStageModules,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigKey, ConfigValue, FacetClass, FacetPaintData, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectConfig, ObjectLayerRef, ObjectMesh,
    ObjectSurfaceData, PaintLayer, PaintSemantic, PaintValue, Point3, RegionKey, ResolvedConfig,
    SemVer, StageId, SurfaceClassificationIR, Transform3d,
};

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn two_triangle_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 0.2,
            },
            Point3 {
                x: 10.0,
                y: 10.0,
                z: 0.2,
            },
        ],
        indices: vec![0, 1, 2, 1, 3, 2],
    }
}

fn painted_custom_object(object_id: &str, semantic: PaintSemantic) -> ObjectMesh {
    ObjectMesh {
        id: object_id.to_string(),
        mesh: two_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic,
                facet_values: vec![Some(PaintValue::Flag(true)), Some(PaintValue::Flag(true))],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: None,
    }
}

fn mesh_fixture(objects: Vec<ObjectMesh>) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects,
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

fn surface_fixture(object_id: &str, facet_count: usize) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: sv(1, 0, 0),
        per_object: [(
            object_id.to_string(),
            ObjectSurfaceData {
                facet_classes: vec![FacetClass::Normal; facet_count],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        )]
        .into(),
    }
}

fn layer_plan_fixture(object_id: &str) -> LayerPlanIR {
    let mut object_participation = HashMap::new();
    object_participation.insert(
        object_id.to_string(),
        vec![ObjectLayerRef {
            local_layer_index: 0,
            global_layer_index: 0,
            effective_layer_height: 0.2,
        }],
    );
    LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.1,
            active_regions: vec![ActiveRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig {
                    wall_count: 2,
                    ..ResolvedConfig::default()
                },
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation,
    }
}

/// Trivial PrepassStageRunner: the test's ExecutionPlan carries zero
/// `prepass_stages`, so `run_stage` is never invoked. The impl exists only
/// to satisfy the `&dyn PrepassStageRunner` argument.
struct NoopRunner;
impl PrepassStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        Ok((PrepassStageOutput::None, Vec::new()))
    }
}

fn empty_execution_plan() -> slicer_host::ExecutionPlan {
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::<SortedStageModules>::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    build_execution_plan(&request).expect("empty execution plan should build")
}

#[test]
fn paint_semantic_override_propagates_through_full_prepass() {
    let object_id = "custom-object";
    let semantic = PaintSemantic::Custom("fuzzy_skin".to_string());

    let mesh = Arc::new(mesh_fixture(vec![painted_custom_object(
        object_id,
        semantic.clone(),
    )]));
    let surface = Arc::new(surface_fixture(object_id, 2));
    let layer_plan = Arc::new(layer_plan_fixture(object_id));

    let mut blackboard = Blackboard::new(mesh, 1);
    blackboard
        .commit_surface_classification(surface)
        .expect("commit surface_classification");
    blackboard
        .commit_layer_plan(layer_plan)
        .expect("commit layer_plan");

    // paint_config:fuzzy_skin:wall_count=5 — the override that must reach
    // RegionPlan.paint_overrides via the host paint-seg fallback → region-mapping
    // ordering. Global baseline is wall_count=2 (active_regions' resolved_config);
    // any region with overlapping fuzzy_skin semantic must resolve to 5.
    let mut raw_config: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw_config.insert(
        ConfigKey::from("paint_config:fuzzy_skin:wall_count"),
        ConfigValue::Int(5),
    );

    let resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig {
        wall_count: 2,
        ..ResolvedConfig::default()
    };
    let bounds = ConfigBoundsIndex::empty();

    let plan = empty_execution_plan();
    let runner = NoopRunner;

    execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        &runner,
        &resolved_configs,
        &default_resolved,
        &raw_config,
        &bounds,
    )
    .expect("execute_prepass_with_builtins_configured must succeed");

    // Step 0 — host fallback fired in the canonical position (before region-mapping).
    assert!(
        blackboard.paint_regions().is_some(),
        "paint-seg fallback must have committed PaintRegionIR before region-mapping"
    );
    let rm = blackboard
        .region_map()
        .cloned()
        .expect("region-mapping built-in must have committed RegionMapIR");

    // Step A — overlay STORED in RegionPlan.paint_overrides for the fuzzy_skin
    // semantic. Pre-fix this map was empty because PaintRegionIR didn't exist
    // when region-mapping computed `paint_semantic_configs`.
    let stored = rm.entries.values().any(|rp| {
        rp.paint_overrides
            .get(&semantic)
            .map(|cfg| cfg.wall_count == 5)
            .unwrap_or(false)
    });
    assert!(
        stored,
        "at least one RegionPlan must carry \
         paint_overrides[Custom(\"fuzzy_skin\")].wall_count=5; pre-fix the \
         fallback ran AFTER region-mapping so the overlay never reached RegionPlan"
    );

    // Step B — overlay APPLIED to RegionPlan.config for the overlapping region.
    // overlapping_semantics_for_region treats any semantic present on the layer
    // as overlapping (see region_mapping.rs:264, :286-290), so the painted face's
    // semantic propagates into config for region_id=0 on global_layer 0.
    let key = RegionKey {
        global_layer_index: 0,
        object_id: object_id.to_string(),
        region_id: 0,
    };
    let rp = rm
        .entries
        .get(&key)
        .expect("RegionPlan for the painted region must exist");
    assert_eq!(
        rp.config.wall_count, 5,
        "RegionPlan.config.wall_count must resolve to 5 via paint-semantic overlay; \
         pre-fix it stayed at the baseline 2 because PaintRegionIR wasn't yet committed \
         when region-mapping ran"
    );
}
