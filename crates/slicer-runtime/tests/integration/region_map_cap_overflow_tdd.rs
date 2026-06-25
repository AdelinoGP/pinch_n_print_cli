#![allow(missing_docs, dead_code)]

//! P93 AC-N2 — `RegionMappingError::CapExceeded::top_contributors[0]` must
//! name the worst-contributing `ObjectId` (highest cross-product entry count)
//! when the per-insert cap guard in `execute_region_mapping_inner` trips.
//!
//! Uses a small synthetic cap (5) — we never approach the real
//! `DEFAULT_REGION_MAP_CAP` (750_000) in tests because that would explode
//! memory. The diagnostic shape is identical regardless of cap magnitude, so
//! the test exercises the same code path that fires in production.

use std::collections::{BTreeMap, HashMap};

use slicer_core::algos::region_mapping::{
    execute_region_mapping_inner, RegionMappingError, RegionMappingPlanProjection,
};
use slicer_ir::{
    ActiveRegion, FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR, ModuleInvocation,
    ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, ResolvedConfig, SemVer,
    StageId, Transform3d,
};
use slicer_scheduler::region_split::AggregatedRegionSplitEntry;
use slicer_scheduler::RegionSplitValueType;

#[test]
fn region_map_cap_exceeded_named_contributor() {
    // Two objects: `obj_alpha` paints 4 distinct Material values across 3
    // active regions over 2 layers; `obj_beta` paints 2 distinct values
    // over 1 region on 1 layer.
    //
    //   Cross-product per (layer, region):
    //     - obj_alpha: 1 (empty chain) + 4 (single-element chains) = 5
    //     - obj_beta : 1 (empty chain) + 2 (single-element chains) = 3
    //   Entries inserted per active region.
    //   With cap=5:
    //     - layer 0:
    //         (obj_alpha, region 1): inserts 5 (saturates the cap at the 5th)
    //     - The 6th insert (next region) trips the per-insert guard.
    //
    //   By the time the guard fires, obj_alpha has contributed strictly more
    //   entries than obj_beta, so `top_contributors[0].object_id == "obj_alpha"`.
    let layer_plan = layer_plan_alpha_beta();
    let aggregated = aggregated_region_split_material();
    let objects = vec![
        object_with_paint("obj_alpha", &[0, 1, 2, 3]),
        object_with_paint("obj_beta", &[10, 11]),
    ];
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };

    let err = execute_region_mapping_inner(
        &layer_plan,
        &projection,
        &BTreeMap::new(),
        &aggregated,
        &objects,
        None,
        &BTreeMap::new(),
        /* cap = */ 5,
    )
    .expect_err("cross-product expansion must exceed cap of 5");

    match err {
        RegionMappingError::CapExceeded {
            top_contributors,
            cap,
            ..
        } => {
            assert_eq!(cap, 5);
            assert!(
                !top_contributors.is_empty(),
                "top_contributors must be populated"
            );
            assert_eq!(
                top_contributors[0].object_id, "obj_alpha",
                "worst-contributing ObjectId must be FIRST element (AC-N2)"
            );
        }
        other => panic!("expected CapExceeded, got {other:?}"),
    }
}

// ---- fixtures ----------------------------------------------------------

fn sv() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn layer_plan_alpha_beta() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: sv(),
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: vec![
                    active_region("obj_alpha", 1),
                    active_region("obj_alpha", 2),
                    active_region("obj_beta", 1),
                ],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: vec![active_region("obj_alpha", 1)],
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: HashMap::new(),
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

fn aggregated_region_split_material() -> BTreeMap<String, AggregatedRegionSplitEntry> {
    let mut m = BTreeMap::new();
    m.insert(
        "material".to_string(),
        AggregatedRegionSplitEntry {
            priority: 0,
            value_type: RegionSplitValueType::ToolIndex,
            declaring_modules: Vec::new(),
        },
    );
    m
}

fn object_with_paint(id: &str, tool_indices: &[u32]) -> ObjectMesh {
    let facet_values: Vec<Option<PaintValue>> = tool_indices
        .iter()
        .map(|&t| Some(PaintValue::ToolIndex(t)))
        .collect();
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet::default(),
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values,
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: None,
    }
}
