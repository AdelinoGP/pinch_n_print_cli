#![allow(missing_docs, dead_code)]

//! TDD-RED test file for packet 51 (`paint-semantic-region-overrides`).
//! Tests assert the paint-aware overlay behaviour of `execute_region_mapping`
//! once it gains the extended signature (Step 5). All three tests are RED:
//! they reach `panic!("RED: â€¦")` placeholders at runtime until Steps 4 & 5
//! implement the required symbols.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_core::algos::region_mapping::RegionMappingPlanProjection;
use slicer_ir::{
    ActiveRegion, BoundingBox3, FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR,
    MeshIR, ModuleId, ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point3,
    RegionKey, ResolvedConfig, SemVer, Transform3d,
};
use slicer_runtime::{
    build_execution_plan, execute_region_mapping, ExecutionPlan, ExecutionPlanRequest,
    LoadDiagnostic, SortedStageModules,
};
use slicer_scheduler::region_split::AggregatedRegionSplitEntry;
use slicer_scheduler::RegionSplitValueType;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn minimal_mesh(object_id: &str) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![
            // exhaustive: ObjectMesh fixture intentionally specifies every field.
            ObjectMesh {
                id: object_id.to_string(),
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
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: vec![],
                paint_data: None,
                world_z_extent: None,
            },
        ],
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
    // exhaustive: ActiveRegion fixture intentionally specifies every field.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// AC-3 (packet 51): When a `PaintRegionIR` has a semantic region overlapping
/// `RegionKey(layer=5, object="obj-a", region_id=0)` and a paint-semantic-config
/// map sets `perimeter_count=5` for `Custom("fuzzy_skin")` (global is 2), the
/// produced `RegionPlan` carries that override in `paint_overrides` AND the
/// effective `config.perimeter_count` is 5.
#[test]
fn region_overlap_applies_override() {
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![
            // exhaustive: GlobalLayer fixture intentionally specifies every field.
            GlobalLayer {
                index: 5,
                z: 1.0,
                active_regions: vec![active_region("obj-a", 0)],
                has_nonplanar: false,
                is_sync_layer: false,
                is_raft: false,
            },
        ],
        object_participation: HashMap::new(),
    });

    let mut paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    paint_semantic_configs.insert(
        PaintSemantic::Custom("fuzzy_skin".to_string()),
        ResolvedConfig {
            wall_count: 5,
            ..ResolvedConfig::default()
        },
    );

    let plan = empty_execution_plan();
    let si = plan_stage_invocations(&plan);
    let projection = RegionMappingPlanProjection {
        stage_invocations: &si,
    };

    // v2 contract (packet 95): execute_region_mapping populates paint_overrides
    // by enumerating chains from aggregated_region_split × per-object paint_data.
    // Both inputs MUST be present for a paint semantic to surface in a chain.
    let mut aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
    aggregated_region_split.insert(
        "fuzzy_skin".to_string(),
        AggregatedRegionSplitEntry {
            priority: 0,
            value_type: RegionSplitValueType::Flag,
            declaring_modules: vec![ModuleId::from("com.test.fuzzy_skin")],
        },
    );

    let object = object_with_paint(
        "obj-a",
        PaintSemantic::Custom("fuzzy_skin".to_string()),
        PaintValue::Flag(true),
    );
    let objects = vec![object];

    let rm = execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &aggregated_region_split,
        &objects,
    )
    .expect("execute_region_mapping must succeed");

    // The cross-product enumerates two variant chains: BASE (empty) and the
    // fuzzy_skin chain.  paint_overrides is populated on the fuzzy_skin chain.
    let fuzzy_key = RegionKey {
        global_layer_index: 5,
        object_id: "obj-a".to_string(),
        region_id: 0,
        variant_chain: vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
    };
    let rp = rm
        .entries
        .get(&fuzzy_key)
        .expect("entry for obj-a layer 5 with fuzzy_skin variant chain must exist");
    assert!(
        rp.paint_overrides
            .contains_key(&PaintSemantic::Custom("fuzzy_skin".to_string())),
        "paint_overrides must contain fuzzy_skin"
    );
    assert_eq!(
        rm.config_for(&fuzzy_key).wall_count,
        5,
        "effective config.wall_count must be 5 from paint override"
    );
}

// Helper: build an `ObjectMesh` whose `paint_data` carries one PaintLayer with
// the given semantic + facet_value.  Used by v2 region-mapping tests to
// declare which paint semantics are present on which objects.
fn object_with_paint(object_id: &str, semantic: PaintSemantic, value: PaintValue) -> ObjectMesh {
    let mesh = minimal_mesh(object_id);
    let mut obj = mesh.objects.into_iter().next().unwrap();
    obj.paint_data = Some(FacetPaintData {
        layers: vec![PaintLayer {
            semantic,
            facet_values: vec![Some(value)],
            strokes: vec![],
        }],
    });
    obj
}

/// AC-4 (packet 51): When no paint region overlaps a `RegionKey`, the
/// resulting `RegionPlan.paint_overrides` is empty and `config` equals the
/// per-object config unchanged.
#[test]
fn no_overlap_keeps_object_config() {
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![
            // exhaustive: GlobalLayer fixture intentionally specifies every field.
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: vec![active_region("obj-b", 0)],
                has_nonplanar: false,
                is_sync_layer: false,
                is_raft: false,
            },
        ],
        object_participation: HashMap::new(),
    });

    let paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();

    let plan = empty_execution_plan();
    let si = plan_stage_invocations(&plan);
    let projection = RegionMappingPlanProjection {
        stage_invocations: &si,
    };

    let rm = execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &BTreeMap::new(),
        &[],
    )
    .expect("execute_region_mapping must succeed");
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-b".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let rp = rm
        .entries
        .get(&key)
        .expect("entry for obj-b layer 0 must exist");
    assert!(
        rp.paint_overrides.is_empty(),
        "paint_overrides must be empty when no overlap"
    );
    assert_eq!(
        *rm.config_for(&key),
        ResolvedConfig::default(),
        "config must equal per-object config when no paint overlay"
    );
}

/// AC-5 (packet 51): When two paint semantics (`Custom("aaa_first")` and
/// `Custom("zzz_last")`) BOTH overlap the same `RegionKey` and each sets a
/// different `perimeter_count`, the lexicographically-last semantic wins
/// because semantics are applied in ascending sort order. Both semantics
/// appear in `paint_overrides`. Running twice produces bit-identical output.
#[test]
fn overlap_precedence_is_deterministic() {
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![
            // exhaustive: GlobalLayer fixture intentionally specifies every field.
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: vec![active_region("obj-c", 0)],
                has_nonplanar: false,
                is_sync_layer: false,
                is_raft: false,
            },
        ],
        object_participation: HashMap::new(),
    });

    // Both semantics overlap layer 0 / obj-c.
    let sem_aaa = PaintSemantic::Custom("aaa_first".to_string());
    let sem_zzz = PaintSemantic::Custom("zzz_last".to_string());

    let mut paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    paint_semantic_configs.insert(
        sem_aaa.clone(),
        ResolvedConfig {
            wall_count: 3,
            ..ResolvedConfig::default()
        },
    );
    paint_semantic_configs.insert(
        sem_zzz.clone(),
        ResolvedConfig {
            wall_count: 9,
            ..ResolvedConfig::default()
        },
    );

    let plan = empty_execution_plan();
    let si = plan_stage_invocations(&plan);
    let projection = RegionMappingPlanProjection {
        stage_invocations: &si,
    };

    // Provide aggregated_region_split + per-object paint_data for both semantics.
    let mut aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
    aggregated_region_split.insert(
        "aaa_first".to_string(),
        AggregatedRegionSplitEntry {
            priority: 0,
            value_type: RegionSplitValueType::Flag,
            declaring_modules: vec![ModuleId::from("com.test.aaa_first")],
        },
    );
    aggregated_region_split.insert(
        "zzz_last".to_string(),
        AggregatedRegionSplitEntry {
            priority: 1,
            value_type: RegionSplitValueType::Flag,
            declaring_modules: vec![ModuleId::from("com.test.zzz_last")],
        },
    );

    // obj-c has BOTH paint semantics so the cross-product yields a chain
    // containing both entries.
    let mesh = minimal_mesh("obj-c");
    let mut obj = mesh.objects.into_iter().next().unwrap();
    obj.paint_data = Some(FacetPaintData {
        layers: vec![
            PaintLayer {
                semantic: sem_aaa.clone(),
                facet_values: vec![Some(PaintValue::Flag(true))],
                strokes: vec![],
            },
            PaintLayer {
                semantic: sem_zzz.clone(),
                facet_values: vec![Some(PaintValue::Flag(true))],
                strokes: vec![],
            },
        ],
    });
    let objects = vec![obj];

    let assert_result = |rm: &slicer_ir::RegionMapIR| {
        // The cross-product chain containing BOTH semantics — that's the entry
        // where paint_overrides accumulates both AND zzz_last wins on the
        // effective config (overlay applies in canonical sort order).
        let both_key = RegionKey {
            global_layer_index: 0,
            object_id: "obj-c".to_string(),
            region_id: 0,
            variant_chain: vec![
                ("aaa_first".to_string(), PaintValue::Flag(true)),
                ("zzz_last".to_string(), PaintValue::Flag(true)),
            ],
        };
        let rp = rm
            .entries
            .get(&both_key)
            .expect("entry for obj-c layer 0 with both-semantic chain must exist");
        assert!(
            rp.paint_overrides
                .contains_key(&PaintSemantic::Custom("aaa_first".to_string())),
            "paint_overrides must contain aaa_first"
        );
        assert!(
            rp.paint_overrides
                .contains_key(&PaintSemantic::Custom("zzz_last".to_string())),
            "paint_overrides must contain zzz_last"
        );
        assert_eq!(
            rm.config_for(&both_key).wall_count,
            9,
            "zzz_last (lexicographically last) must win -> wall_count=9"
        );
    };

    let rm1 = execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &aggregated_region_split,
        &objects,
    )
    .expect("first execute_region_mapping must succeed");
    assert_result(&rm1);

    let rm2 = execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &aggregated_region_split,
        &objects,
    )
    .expect("second execute_region_mapping must succeed");
    assert_result(&rm2);

    assert_eq!(rm1, rm2, "output must be bit-identical across two runs");
}

fn plan_stage_invocations(
    plan: &ExecutionPlan,
) -> Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> {
    plan.per_layer_stages
        .iter()
        .chain(plan.postpass_stages.iter())
        .map(|stage| {
            let invocations = stage
                .modules
                .iter()
                .map(|m| slicer_ir::ModuleInvocation {
                    module_id: m.module_id().to_owned(),
                    config_view: m.config_view().as_ref().clone(),
                })
                .collect::<Vec<_>>();
            (stage.stage_id.clone(), invocations)
        })
        .collect()
}

/// AC-13: scalar paint is ignored unless its semantic opts into discrete splitting.
#[test]
fn scalar_paint_production_prepass_characterization() {
    use slicer_runtime::{
        execute_prepass_with_builtins, Blackboard, CompiledModuleLive, ExecutionModuleBinding,
        PrepassExecutionError, PrepassRunnerError, PrepassStageInput, PrepassStageOutput,
        PrepassStageRunner, RegionMappingBuiltinError, RegionMappingError,
    };
    use slicer_scheduler::{load_modules_from_roots_with_integrated, IntegratedModuleRegistration};

    // Only per-layer modules are scheduled; all prepass work must be host built-ins.
    struct NoUserPrepass;
    impl PrepassStageRunner for NoUserPrepass {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            _module: &CompiledModuleLive<'_>,
            _input: PrepassStageInput<'_>,
        ) -> Result<PrepassStageOutput, PrepassRunnerError> {
            panic!("unexpected user prepass stage: {stage_id}")
        }
    }

    const BASE_MANIFEST: &str = r#"
[module]
id = "com.test.scalar-characterization"
version = "1.0.0"
[stage]
id = "Layer::Perimeters"
[ir-access]
reads = ["SliceIR"]
writes = ["PerimeterIR"]
[claims]
holds = []
requires = []
[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "5.0.0"
[hints]
layer-parallel-safe = true
"#;
    // Registration borrows static manifest text, just like embedded production modules.
    const SPLIT_MANIFEST: &str = r#"
[[region_split]]
semantic = "material"
priority = 100
value_type = "tool_index"
[module]
id = "com.test.scalar-characterization"
version = "1.0.0"
[stage]
id = "Layer::Perimeters"
[ir-access]
reads = ["SliceIR"]
writes = ["PerimeterIR"]
[claims]
holds = []
requires = []
[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "5.0.0"
[hints]
layer-parallel-safe = true
"#;

    for (declared, manifest_toml) in [(false, BASE_MANIFEST), (true, SPLIT_MANIFEST)] {
        let report = load_modules_from_roots_with_integrated(
            &[],
            &[IntegratedModuleRegistration {
                manifest_toml,
                origin_label: "scalar-characterization",
            }],
        )
        .expect("in-test manifest must load");
        assert_eq!(report.modules.len(), 1, "fixture module must be admitted");
        let module = report.modules.into_iter().next().unwrap();
        let request = ExecutionPlanRequest {
            sorted_stages: vec![SortedStageModules {
                stage_id: module.stage().to_owned(),
                module_ids: vec![module.id().to_owned()],
            }],
            module_bindings: vec![ExecutionModuleBinding {
                module,
                config_view: Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())),
            }],
            global_layers: Arc::new(vec![]),
            region_plans: Arc::new(HashMap::new()),
        };
        let plan = build_execution_plan(&request, &mut Vec::new()).expect("build scalar plan");

        for (case, values) in [("uniform", [0.25_f32, 0.25]), ("varying", [0.25, 0.75])] {
            let mut mesh = minimal_mesh("scalar-object");
            // Two adjacent facets, each with an explicit scalar paint value.
            mesh.objects[0].mesh.vertices.push(Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            });
            mesh.objects[0].mesh.indices = vec![0, 1, 2, 1, 3, 2];
            mesh.objects[0].paint_data = Some(FacetPaintData {
                layers: vec![PaintLayer {
                    semantic: PaintSemantic::Material,
                    facet_values: values
                        .into_iter()
                        .map(|v| Some(PaintValue::Scalar(v)))
                        .collect(),
                    strokes: vec![],
                }],
            });
            let mut blackboard = Blackboard::new(Arc::new(mesh), 0);
            blackboard
                .commit_layer_plan(Arc::new(LayerPlanIR {
                    schema_version: sv(1, 0, 0),
                    global_layers: vec![GlobalLayer {
                        z: 0.2,
                        active_regions: vec![active_region("scalar-object", 0)],
                        ..Default::default()
                    }],
                    object_participation: HashMap::new(),
                }))
                .expect("seed layer plan");

            let result = execute_prepass_with_builtins(
                &plan,
                &mut blackboard,
                &NoUserPrepass,
                &HashMap::new(),
            );
            if declared {
                let error = result.expect_err("declared scalar paint must reject, even if uniform");
                match error {
                    PrepassExecutionError::RegionMapping {
                        source:
                            RegionMappingBuiltinError::Mapping(
                                RegionMappingError::ScalarInRegionSplitFacetValue {
                                    object_id,
                                    semantic,
                                    scalar_bits,
                                },
                            ),
                    } => {
                        assert_eq!(object_id, "scalar-object");
                        assert_eq!(semantic, "material");
                        assert_eq!(scalar_bits, 0.25_f32.to_bits());
                    }
                    other => panic!("{case}: expected typed scalar rejection, got {other:?}"),
                }
                assert!(
                    blackboard.region_map().is_none(),
                    "rejection must not publish a map"
                );
            } else {
                result.unwrap_or_else(|error| {
                    panic!("{case}: non-split scalar must succeed: {error:?}")
                });
                let map = blackboard
                    .region_map()
                    .expect("prepass must commit the region map");
                let base_key = RegionKey {
                    global_layer_index: 0,
                    object_id: "scalar-object".to_owned(),
                    region_id: 0,
                    variant_chain: vec![],
                };
                assert_eq!(
                    map.entries.len(),
                    1,
                    "{case}: no scalar variant key may be fabricated"
                );
                let region = map
                    .entries
                    .get(&base_key)
                    .expect("only the base key must remain");
                assert!(
                    region.paint_overrides.is_empty(),
                    "scalar must not select a paint override"
                );
                let layers = blackboard
                    .layer_plan()
                    .expect("layer plan must remain available");
                assert_eq!(layers.global_layers.len(), 1);
                let active = &layers.global_layers[0].active_regions;
                assert_eq!(active.len(), 1, "scalar must not mint a selected region");
                assert_eq!(active[0].object_id, "scalar-object");
                assert_eq!(active[0].region_id, 0);
                assert!(
                    blackboard.slice_ir().is_some(),
                    "production prepass must reach slicing"
                );
            }
        }
    }
}
