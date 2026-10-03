#![allow(missing_docs)]
//! Wayfinder perf-vs-orca ticket 47 — native/WASM view-seam identity for
//! `Layer::InfillPostProcess` perimeter views.
//!
//! Ticket 33's oracle gate FAILED because the native leg built its
//! perimeter regions from `PerimeterIR` alone (partitioned fill polygons
//! defaulting empty), while the WASM leg enriches them from the arena's
//! partitioned `SliceIR` (`push_infill_postprocess_regions`), so the infill
//! linker silently fell back to the union boundary. The native request must
//! now carry the same postprocess view content the WASM leg pushes,
//! field-for-field:
//!
//! - identity: object_id / region_id = the slice region's (never the donor's);
//! - fill: sparse/top/bottom/bridge/raft mirrored verbatim from `SliceIR`;
//! - walls: from the region's own `PerimeterIR` entry, else the
//!   `wall_source_region_id` base region's entry (shared walls), else empty;
//! - tool index: the pinned `resolve_region_tool_index` precedence;
//! - wall-source id: `None` for regions with their own entry, else the base;
//! - config: the ticket-23 per-region resolver on both legs.
//!
//! The two legs operate on different transports (SDK views vs WIT resources in
//! a wasmtime store), so this is a mirrored-content contract, not shared code.
//! The WASM-leg expectation is reconstructed here from the same shared host
//! predicates the dispatch arm uses, so the contract fails if either leg's
//! observable view content changes independently.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, PaintValue, PerimeterIR, PerimeterRegion, Point2, Polygon,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_sdk::views::PerimeterRegionView;
use slicer_wasm_host::binding::LayerStageInput;
use slicer_wasm_host::dispatch::{
    perimeter_region_index, resolve_region_tool_index, wall_source_region_id,
};
use slicer_wasm_host::marshal::native::build_native_layer_request;
use slicer_wasm_host::{CompiledModuleLive, WasmInstancePool};

fn expoly(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

/// The donor/enrichment shape the WASM leg produces, reconstructed per region
/// from the same shared host predicates (`push_infill_postprocess_regions`'s
/// loop). Where the WASM seam is lossy in ways a module observes, the loss is
/// mirrored here (documented per field).
fn wasm_leg_expected_view(
    perimeter: &PerimeterIR,
    region: &SlicedRegion,
    region_map: Option<&RegionMapIR>,
    layer_index: u32,
) -> PerimeterRegionView {
    let perim_index = perimeter_region_index(perimeter);
    let own_entry = perim_index
        .get(&(&region.object_id, region.region_id))
        .copied();
    let wall_source = wall_source_region_id(own_entry.is_some(), region);
    let donor = own_entry.or_else(|| {
        wall_source.and_then(|base| perim_index.get(&(&region.object_id, base)).copied())
    });
    let mut view = match donor {
        Some(p) => {
            // WASM: PerimeterRegion → `perimeter_region_to_data` (WIT) → the
            // guest glue's reverse adapter → a fresh SDK view. Its walls
            // reconstruct `width_profile` at 0.4/vertex (WIT has no profile)
            // and its seam candidates lose `reason`; the postprocess consumer
            // reads neither, so this mirror keeps the donor as-is and the
            // wall-equality assertions below compare like-for-like.
            PerimeterRegionView::from_ir(p)
        }
        None => PerimeterRegionView::default(),
    };
    // The view's identity is the slice region's, not the wall donor's.
    view.set_object_id(region.object_id.clone());
    view.set_region_id(region.region_id);
    // WIT carries no `Custom` paint value: a WASM module observes ToolIndex(0).
    view.set_variant_chain(
        region
            .variant_chain
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    match value {
                        PaintValue::Flag(v) => PaintValue::Flag(*v),
                        PaintValue::Scalar(v) => PaintValue::Scalar(*v),
                        PaintValue::ToolIndex(v) => PaintValue::ToolIndex(*v),
                        PaintValue::Custom(_) => PaintValue::ToolIndex(0),
                    },
                )
            })
            .collect(),
    );
    view.set_sparse_infill_area(region.sparse_infill_area.clone());
    view.set_top_solid_fill(region.top_solid_fill.clone());
    view.set_bottom_solid_fill(region.bottom_solid_fill.clone());
    view.set_bridge_areas(region.bridge_areas.clone());
    view.set_raft_fill(region.raft_fill.clone());
    view.set_tool_index(resolve_region_tool_index(
        &region.variant_chain,
        region_map,
        layer_index,
        &region.object_id,
        region.region_id,
    ));
    view.set_wall_source_region_id(wall_source);
    // The wasm leg's per-region `config` accessor falls back to the
    // object-level config (module.config_view) when no RegionMap pool entry
    // exists; the native leg's `set_config` fallback mirrors it.
    view.set_config(ConfigView::default());
    view
}

struct Fixture {
    slice: SliceIR,
    perimeter: PerimeterIR,
    region_map: Option<RegionMapIR>,
    config: Arc<ConfigView>,
}

#[allow(clippy::needless_pass_by_value)]
fn native_views(fx: &Fixture, stage: &'static str) -> Vec<PerimeterRegionView> {
    let module_id = "view-identity".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        fx.config.clone(),
    );
    // exhaustive: LayerStageInput has no Default; the fixture supplies every field
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: fx.region_map.as_ref().map(|rm| Arc::new(rm.clone())),
        slice: Some(&fx.slice),
        perimeter: Some(&fx.perimeter),
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let native = build_native_layer_request(stage, 0, &input, &module, &HashMap::new());
    native
        .perimeter_regions
        .expect("postprocess requests always carry Some(perimeter_regions)")
}

#[test]
fn native_postprocess_views_match_the_wasm_enrichment() {
    let fixture = Fixture {
        slice: SliceIR {
            global_layer_index: 0,
            z: 0.2,
            regions: vec![
                SlicedRegion {
                    object_id: "obj-0".to_owned(),
                    region_id: 3, // own PerimeterIR entry → owns walls
                    polygons: vec![expoly(0, 0, 100, 100)],
                    sparse_infill_area: vec![expoly(10, 10, 90, 90)],
                    top_solid_fill: vec![expoly(20, 20, 80, 80)],
                    bridge_areas: vec![expoly(30, 30, 70, 70)],
                    raft_fill: vec![expoly(40, 40, 60, 60)],
                    ..Default::default()
                },
                SlicedRegion {
                    object_id: "obj-0".to_owned(),
                    region_id: 2_000_001, // paint variant of base 2; no own entry
                    variant_chain: vec![("material".to_owned(), PaintValue::ToolIndex(2))],
                    polygons: vec![expoly(0, 0, 50, 50)],
                    sparse_infill_area: vec![expoly(5, 5, 45, 45)],
                    ..Default::default()
                },
                SlicedRegion {
                    object_id: "obj-1".to_owned(),
                    region_id: 9, // no entry anywhere → empty donor
                    polygons: vec![expoly(0, 0, 10, 10)],
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        perimeter: PerimeterIR {
            global_layer_index: 0,
            regions: vec![
                PerimeterRegion {
                    object_id: "obj-0".to_owned(),
                    region_id: 3,
                    infill_areas: vec![expoly(0, 0, 100, 100)],
                    ..Default::default()
                },
                PerimeterRegion {
                    object_id: "obj-0".to_owned(),
                    region_id: 2, // borrowed-walls donor for 2_000_001
                    infill_areas: vec![expoly(0, 0, 50, 50)],
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        region_map: None,
        config: Arc::new(ConfigView::from_map(HashMap::new())),
    };

    let native = native_views(&fixture, "Layer::InfillPostProcess");
    let expected: Vec<PerimeterRegionView> = fixture
        .slice
        .regions
        .iter()
        .map(|region| {
            wasm_leg_expected_view(&fixture.perimeter, region, fixture.region_map.as_ref(), 0)
        })
        .collect();
    assert_eq!(native.len(), expected.len(), "one view per slice region");
    for (i, (got, want)) in native.iter().zip(&expected).enumerate() {
        assert_eq!(got.object_id(), want.object_id(), "view {i} object_id");
        assert_eq!(got.region_id(), want.region_id(), "view {i} region_id");
        assert_eq!(
            got.variant_chain(),
            want.variant_chain(),
            "view {i} variant_chain"
        );
        assert_eq!(
            got.sparse_infill_area(),
            want.sparse_infill_area(),
            "view {i} sparse_infill_area"
        );
        assert_eq!(
            got.top_solid_fill(),
            want.top_solid_fill(),
            "view {i} top_solid_fill"
        );
        assert_eq!(
            got.bottom_solid_fill(),
            want.bottom_solid_fill(),
            "view {i} bottom_solid_fill"
        );
        assert_eq!(
            got.bridge_areas(),
            want.bridge_areas(),
            "view {i} bridge_areas"
        );
        assert_eq!(got.raft_fill(), want.raft_fill(), "view {i} raft_fill");
        assert_eq!(got.tool_index(), want.tool_index(), "view {i} tool_index");
        assert_eq!(
            got.wall_source_region_id(),
            want.wall_source_region_id(),
            "view {i} wall_source_region_id"
        );
        assert_eq!(
            got.infill_areas(),
            want.infill_areas(),
            "view {i} infill_areas"
        );
        assert_eq!(got.wall_loops(), want.wall_loops(), "view {i} wall_loops");
        assert_eq!(got.config(), want.config(), "view {i} config");
    }
}

/// The one region the linker must NOT see through the union fallback: a
/// partition-only area. `RoleBoundaries::is_partitioned` is reachable
/// natively — the pre-fix builder left all four partitions empty and the
/// linker handed every role the union.
#[test]
fn native_postprocess_partition_makes_the_linker_verdict_reachable() {
    let mut slice = SliceIR {
        global_layer_index: 0,
        z: 0.2,
        ..Default::default()
    };
    slice.regions.push(SlicedRegion {
        object_id: "obj-0".to_owned(),
        region_id: 0,
        polygons: vec![expoly(0, 0, 100, 100)],
        sparse_infill_area: vec![expoly(10, 10, 90, 90)],
        ..Default::default()
    });
    let perimeter = PerimeterIR {
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "obj-0".to_owned(),
            region_id: 0,
            infill_areas: vec![expoly(0, 0, 100, 100)],
            ..Default::default()
        }],
        ..Default::default()
    };
    let fixture = Fixture {
        slice,
        perimeter,
        region_map: None,
        config: Arc::new(ConfigView::from_map(HashMap::new())),
    };
    let native = native_views(&fixture, "Layer::InfillPostProcess");
    assert_eq!(native.len(), 1);
    assert!(
        !native[0].sparse_infill_area().is_empty(),
        "the sparse partition must reach the native view (pre-fix it was empty)"
    );
    assert_eq!(
        native[0].infill_areas().len(),
        1,
        "the union boundary still arrives from the donor"
    );
}

/// Virtual paint-variant tool resolution and `Custom` degrade on the native
/// leg mirror the WASM seam.
#[test]
fn native_postprocess_tool_index_and_paint_degrade() {
    let mut slice = SliceIR {
        global_layer_index: 0,
        z: 0.2,
        ..Default::default()
    };
    slice.regions.push(SlicedRegion {
        object_id: "obj-0".to_owned(),
        // Virtual paint variant of base region 5: `paint_variant_region_id`
        // payload form `5 * 1_000_000 + 5`.
        region_id: 5_000_005,
        variant_chain: vec![
            ("material".to_owned(), PaintValue::ToolIndex(3)),
            (
                CUSTOM_MODULE_SEMANTIC.to_owned(),
                PaintValue::Custom("x".into()),
            ),
        ],
        polygons: vec![expoly(0, 0, 10, 10)],
        sparse_infill_area: vec![expoly(1, 1, 9, 9)],
        ..Default::default()
    });
    let perimeter = PerimeterIR {
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "obj-0".to_owned(),
            region_id: 5,
            ..Default::default()
        }],
        ..Default::default()
    };
    // RegionMap extensions["extruder"] would resolve tool 1 via the exact
    // variant key, but the painted material variant (3) must win.
    let mut rm = RegionMapIR::default();
    let mut rc = ResolvedConfig::default();
    rc.extensions
        .insert("extruder".to_string(), ConfigValue::Int(1));
    let cfg_id = rm.intern_config(rc);
    rm.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 5_000_005,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: cfg_id,
            ..Default::default()
        },
    );
    let fixture = Fixture {
        slice,
        perimeter,
        region_map: Some(rm),
        config: Arc::new(ConfigView::from_map(HashMap::new())),
    };
    let native = native_views(&fixture, "Layer::InfillPostProcess");
    assert_eq!(native.len(), 1);
    assert_eq!(
        native[0].tool_index(),
        3,
        "material ToolIndex(3) wins over extruder=1"
    );
    assert_eq!(
        native[0].wall_source_region_id(),
        Some(&5),
        "virtual variant of base 5 borrows the base region's walls"
    );
    // The `Custom` entry is observable to a native module exactly as WIT shows
    // it to a WASM module: degraded to ToolIndex(0).
    assert_eq!(
        native[0]
            .variant_chain()
            .iter()
            .find(|(name, _)| name == CUSTOM_MODULE_SEMANTIC),
        Some(&(CUSTOM_MODULE_SEMANTIC.to_string(), PaintValue::ToolIndex(0))),
        "Custom must degrade to ToolIndex(0) on the native seam too"
    );
}

const CUSTOM_MODULE_SEMANTIC: &str = "SomeCustomModule";

/// Missing `SliceIR` falls back to the legacy `PerimeterIR`-driven views
/// (both WASM arms keep this defensive fallback).
#[test]
fn native_postprocess_without_slice_falls_back_to_perimeter_views() {
    let module_id = "view-identity-fallback".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    let perimeter = PerimeterIR {
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "obj-0".to_owned(),
            region_id: 7,
            infill_areas: vec![expoly(0, 0, 100, 100)],
            ..Default::default()
        }],
        ..Default::default()
    };
    // exhaustive: LayerStageInput has no Default; the fixture supplies every field
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: None,
        slice: None,
        perimeter: Some(&perimeter),
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let native = build_native_layer_request(
        "Layer::InfillPostProcess",
        0,
        &input,
        &module,
        &HashMap::new(),
    );
    let views = native
        .perimeter_regions
        .expect("postprocess requests always carry Some(perimeter_regions)");
    assert_eq!(
        views.len(),
        1,
        "the fallback projects the PerimeterIR regions"
    );
    assert_eq!(views[0].region_id(), &7);
    assert!(
        views[0].sparse_infill_area().is_empty(),
        "the fallback keeps the legacy ADR-0028-empty defaults"
    );
}

/// The ordered-entities snapshot (ticket 47, second projection): the two
/// builder-consuming stages must see the staged `LayerCollectionIR` content
/// through the native request exactly as `get_ordered_entities` returns it on
/// the WASM leg; every other stage carries an empty snapshot.
#[test]
fn native_ordered_entities_snapshot_matches_the_wasm_seam() {
    let entity = |idx: u32, role: slicer_ir::ExtrusionRole, lock: Option<u64>| {
        // exhaustive: the seam under test carries every PrintEntity field
        slicer_ir::PrintEntity {
            entity_id: u64::from(idx) + 1,
            // exhaustive: fixture entity pins only the fields the seam carries
            path: slicer_ir::ExtrusionPath3D {
                points: vec![
                    slicer_ir::Point3WithWidth {
                        x: idx as f32,
                        y: 2.0,
                        ..slicer_ir::Point3WithWidth::default()
                    },
                    slicer_ir::Point3WithWidth {
                        x: idx as f32 + 1.0,
                        y: 3.0,
                        ..slicer_ir::Point3WithWidth::default()
                    },
                ],
                role: role.clone(),
                speed_factor: 1.0,
                tool_index: None,
                order_lock: lock,
            },
            role,
            region_key: slicer_ir::RegionKey {
                global_layer_index: 4,
                object_id: "obj-0".to_owned(),
                region_id: 9,
                variant_chain: Vec::new(),
            },
            topo_order: idx,
            tool_index: 2,
        }
    };
    let lc = slicer_ir::LayerCollectionIR {
        ordered_entities: vec![
            entity(0, slicer_ir::ExtrusionRole::SparseInfill, None),
            entity(1, slicer_ir::ExtrusionRole::OuterWall, None),
            entity(2, slicer_ir::ExtrusionRole::SparseInfill, Some(77)),
            entity(3, slicer_ir::ExtrusionRole::SparseInfill, Some(77)),
        ],
        // exhaustive: fixture LayerCollectionIR carries no support attribution
        ..slicer_ir::LayerCollectionIR::default()
    };

    let module_id = "ordered-entities-identity".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    let slice = SliceIR {
        global_layer_index: 4,
        z: 0.9,
        ..Default::default()
    };
    // exhaustive: LayerStageInput has no Default; the fixture supplies every field
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: None,
        slice: Some(&slice),
        perimeter: None,
        layer_collection: Some(&lc),
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };

    for (stage, expect_snapshot) in [
        ("Layer::PathOptimization", true),
        ("Layer::AnchoredEvents", true),
        ("Layer::Infill", false),
    ] {
        let native = build_native_layer_request(stage, 4, &input, &module, &HashMap::new());
        if !expect_snapshot {
            assert!(
                native.ordered_entities.is_empty(),
                "{stage} must not carry the snapshot"
            );
            continue;
        }
        assert_eq!(native.ordered_entities.len(), 4, "{stage} snapshot size");
        // Mirror the WASM seam's per-entry content: region-key variant chain
        // flattens empty, start/end points and counts carry field-for-field.
        for (view, entity) in native.ordered_entities.iter().zip(&lc.ordered_entities) {
            assert_eq!(view.original_index, entity.topo_order, "original_index");
            assert_eq!(view.tool_index, entity.tool_index, "tool_index");
            assert_eq!(view.region_key.global_layer_index, 4, "region_key.layer");
            assert_eq!(view.region_key.object_id, "obj-0", "region_key.object");
            assert_eq!(view.region_key.region_id, 9, "region_key.region");
            assert_eq!(view.role, entity.role, "role");
            assert_eq!(view.start_point.x, entity.path.points[0].x, "start.x");
            assert_eq!(view.end_point.x, entity.path.points[1].x, "end.x");
            assert_eq!(view.point_count, 2, "point_count");
            assert_eq!(view.order_lock, entity.path.order_lock, "order_lock");
        }
    }
}

/// `PerimeterIR` missing entirely → empty region list, never `None`
/// (the wasm leg pushes zero regions; cf. 9685cd03).
#[test]
fn native_postprocess_without_perimeter_yields_empty_list() {
    let module_id = "view-identity-empty".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    let slice = SliceIR {
        global_layer_index: 0,
        z: 0.2,
        ..Default::default()
    };
    // exhaustive: LayerStageInput has no Default; the fixture supplies every field
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: None,
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let native = build_native_layer_request(
        "Layer::InfillPostProcess",
        0,
        &input,
        &module,
        &HashMap::new(),
    );
    assert!(native
        .perimeter_regions
        .expect("Some(perimeter_regions)")
        .is_empty());
}
