#![allow(missing_docs, dead_code)]

//! TDD-RED test file for packet 51 (`paint-semantic-region-overrides`).
//! Tests assert the paint-aware overlay behaviour of `execute_region_mapping`
//! once it gains the extended signature (Step 5). All three tests are RED:
//! they reach `panic!("RED: …")` placeholders at runtime until Steps 4 & 5
//! implement the required symbols.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_host::{
    build_execution_plan, execute_region_mapping, ExecutionPlanRequest, SortedStageModules,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, GlobalLayer, IndexedTriangleSet, LayerPaintMap, LayerPlanIR,
    MeshIR, ObjectConfig, ObjectMesh, PaintRegionIR, PaintSemantic, PaintValue, Point3, RegionKey,
    ResolvedConfig, SemVer, SemanticRegion, Transform3d,
};

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
        objects: vec![ObjectMesh {
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

fn empty_execution_plan() -> slicer_host::ExecutionPlan {
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::<SortedStageModules>::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    build_execution_plan(&request).expect("empty execution plan should build")
}

/// Build a `PaintRegionIR` with a single semantic covering object "obj-a" at
/// layer `layer_index`.
fn paint_region_ir_single(
    layer_index: u32,
    object_id: &str,
    semantic: PaintSemantic,
) -> PaintRegionIR {
    let region = SemanticRegion {
        object_id: object_id.to_string(),
        polygons: vec![],
        value: PaintValue::Flag(true),
        paint_order: 0,
    };
    let mut semantic_regions = HashMap::new();
    semantic_regions.insert(semantic, vec![region]);
    let layer_map = LayerPaintMap {
        global_layer_index: layer_index,
        semantic_regions,
    };
    let mut per_layer = HashMap::new();
    per_layer.insert(layer_index, layer_map);
    PaintRegionIR {
        schema_version: sv(1, 0, 0),
        per_layer,
    }
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
        global_layers: vec![GlobalLayer {
            index: 5,
            z: 1.0,
            active_regions: vec![active_region("obj-a", 0)],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    });

    let paint_regions =
        paint_region_ir_single(5, "obj-a", PaintSemantic::Custom("fuzzy_skin".to_string()));

    let mut paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    paint_semantic_configs.insert(
        PaintSemantic::Custom("fuzzy_skin".to_string()),
        ResolvedConfig {
            wall_count: 5,
            ..ResolvedConfig::default()
        },
    );

    let plan = empty_execution_plan();

    let rm = execute_region_mapping(
        &layer_plan,
        &plan,
        Some(&paint_regions),
        &paint_semantic_configs,
    )
    .expect("execute_region_mapping must succeed");
    let key = RegionKey {
        global_layer_index: 5,
        object_id: "obj-a".to_string(),
        region_id: 0,
    };
    let rp = rm
        .entries
        .get(&key)
        .expect("entry for obj-a layer 5 must exist");
    assert!(
        rp.paint_overrides
            .contains_key(&PaintSemantic::Custom("fuzzy_skin".to_string())),
        "paint_overrides must contain fuzzy_skin"
    );
    assert_eq!(
        rp.config.wall_count, 5,
        "effective config.wall_count must be 5 from paint override"
    );
}

/// AC-4 (packet 51): When no paint region overlaps a `RegionKey`, the
/// resulting `RegionPlan.paint_overrides` is empty and `config` equals the
/// per-object config unchanged.
#[test]
fn no_overlap_keeps_object_config() {
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("obj-b", 0)],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    });

    // Paint region IR covers a *different* layer, so obj-b layer-0 has no overlap.
    let paint_regions =
        paint_region_ir_single(99, "obj-b", PaintSemantic::Custom("fuzzy_skin".to_string()));

    let paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();

    let plan = empty_execution_plan();

    let rm = execute_region_mapping(
        &layer_plan,
        &plan,
        Some(&paint_regions),
        &paint_semantic_configs,
    )
    .expect("execute_region_mapping must succeed");
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-b".to_string(),
        region_id: 0,
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
        rp.config,
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
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("obj-c", 0)],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    });

    // Both semantics overlap layer 0 / obj-c.
    let sem_aaa = PaintSemantic::Custom("aaa_first".to_string());
    let sem_zzz = PaintSemantic::Custom("zzz_last".to_string());

    let region_aaa = SemanticRegion {
        object_id: "obj-c".to_string(),
        polygons: vec![],
        value: PaintValue::Flag(true),
        paint_order: 0,
    };
    let region_zzz = SemanticRegion {
        object_id: "obj-c".to_string(),
        polygons: vec![],
        value: PaintValue::Flag(true),
        paint_order: 1,
    };
    let mut semantic_regions = HashMap::new();
    semantic_regions.insert(sem_aaa.clone(), vec![region_aaa]);
    semantic_regions.insert(sem_zzz.clone(), vec![region_zzz]);
    let layer_map = LayerPaintMap {
        global_layer_index: 0,
        semantic_regions,
    };
    let mut per_layer = HashMap::new();
    per_layer.insert(0u32, layer_map);
    let paint_regions = PaintRegionIR {
        schema_version: sv(1, 0, 0),
        per_layer,
    };

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

    let assert_result = |rm: &slicer_ir::RegionMapIR| {
        let key = RegionKey {
            global_layer_index: 0,
            object_id: "obj-c".to_string(),
            region_id: 0,
        };
        let rp = rm
            .entries
            .get(&key)
            .expect("entry for obj-c layer 0 must exist");
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
            rp.config.wall_count, 9,
            "zzz_last (lexicographically last) must win → wall_count=9"
        );
    };

    let rm1 = execute_region_mapping(
        &layer_plan,
        &plan,
        Some(&paint_regions),
        &paint_semantic_configs,
    )
    .expect("first execute_region_mapping must succeed");
    assert_result(&rm1);

    let rm2 = execute_region_mapping(
        &layer_plan,
        &plan,
        Some(&paint_regions),
        &paint_semantic_configs,
    )
    .expect("second execute_region_mapping must succeed");
    assert_result(&rm2);

    assert_eq!(rm1, rm2, "output must be bit-identical across two runs");
}
