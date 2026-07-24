//! AC-4 / AC-N3: MMU paint segmentation partition invariants on `cube_4color.3mf`.
//!
//! Step 4 of packet 113a-arachne-parity-closures. Feeds the painted facets of
//! `resources/cube_4color.3mf` to `slicer_core::algos::paint_segmentation` and
//! asserts the per-color `ExPolygon` cells form a non-overlapping Voronoi partition
//! (intersection empty within tolerance), every non-BASE cell is contained in
//! the model XY bounding box, every non-BASE cell has non-zero area, and the full
//! set of cells (including BASE residual) is pairwise disjoint within
//! `SCALED_EPSILON`.
//!
//! Host-only: `paint_segmentation` is gated behind the `host-algos` feature.

#![cfg(feature = "host-algos")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::slice_mesh_ex;
use slicer_core::transform_point3;
use slicer_ir::slice_ir::BoundingBox2;
use slicer_ir::{
    ActiveRegion, ExPolygon, GlobalLayer, LayerPlanIR, MeshIR, ObjectLayerRef, PaintValue, Point2,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
};
use slicer_model_io::load_model;

const LAYER_COUNT: u32 = 50;
const LAYER_HEIGHT_MM: f32 = 0.5;
/// 1 unit = 100 nm; used as the area tolerance for pairwise cell intersection.
const SCALED_EPSILON: f64 = 1.0;

fn cube_4color_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_4color.3mf"
    ))
}

fn load_cube_4color() -> MeshIR {
    let path = cube_4color_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    load_model(&path).expect("load cube_4color.3mf should succeed")
}

fn build_50_layer_plan(object_id: &str) -> Arc<LayerPlanIR> {
    let global_layer_indices: Vec<u32> = (0..LAYER_COUNT).collect();
    let layers: Vec<GlobalLayer> = global_layer_indices
        .iter()
        .map(|idx| GlobalLayer {
            index: *idx,
            z: LAYER_HEIGHT_MM * (*idx as f32 + 0.5),
            active_regions: vec![ActiveRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: LAYER_HEIGHT_MM,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    Arc::new(LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: layers,
        object_participation: HashMap::from([(
            object_id.to_string(),
            global_layer_indices
                .iter()
                .copied()
                .enumerate()
                .map(|(local_idx, global_idx)| ObjectLayerRef {
                    local_layer_index: local_idx as u32,
                    global_layer_index: global_idx,
                    effective_layer_height: LAYER_HEIGHT_MM,
                })
                .collect(),
        )]),
    })
}

fn build_initial_slice_ir(
    object_id: &str,
    object_mesh: &slicer_ir::IndexedTriangleSet,
    layer_plan: &LayerPlanIR,
) -> Vec<SliceIR> {
    let zs: Vec<f32> = layer_plan.global_layers.iter().map(|l| l.z).collect();
    let slabs = slice_mesh_ex(object_mesh, &zs);
    zs.iter()
        .enumerate()
        .map(|(idx, &z)| {
            let polys = slabs.get(idx).cloned().unwrap_or_default();
            SliceIR {
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: polys.clone(),
                    infill_areas: polys,
                    effective_layer_height: LAYER_HEIGHT_MM,
                    segment_annotations: HashMap::new(),
                    ..Default::default()
                }],
                ..Default::default()
            }
        })
        .collect()
}

fn build_region_map(object_id: &str, layer_count: u32) -> Arc<RegionMapIR> {
    let mut entries = HashMap::new();
    for i in 0..layer_count {
        entries.insert(
            RegionKey {
                global_layer_index: i,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    Arc::new(RegionMapIR {
        schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        entries,
        configs: vec![ResolvedConfig::default()],
    })
}

fn run_paint_segmentation(mesh: Arc<MeshIR>) -> Arc<Vec<SliceIR>> {
    let object_id = &mesh.objects[0].id;
    let object_mesh = mesh.objects[0].mesh.clone();
    let layer_plan = build_50_layer_plan(object_id);
    let initial = build_initial_slice_ir(object_id, &object_mesh, &layer_plan);
    let region_map = build_region_map(object_id, LAYER_COUNT);
    execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed")
}

/// World-space XY bounding box of the first object, in scaled units.
fn model_xy_bounding_box(mesh: &MeshIR) -> BoundingBox2 {
    let obj = &mesh.objects[0];
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for v in &obj.mesh.vertices {
        let p = transform_point3(&obj.transform.matrix, *v);
        let x = slicer_ir::mm_to_units(p.x);
        let y = slicer_ir::mm_to_units(p.y);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    BoundingBox2 {
        min: Point2 { x: min_x, y: min_y },
        max: Point2 { x: max_x, y: max_y },
    }
}

/// Signed area of an `ExPolygon` contour minus its holes, in unit^2.
fn expolygon_area(ep: &ExPolygon) -> f64 {
    let contour_area = |pts: &[Point2]| -> f64 {
        if pts.len() < 3 {
            return 0.0;
        }
        let mut acc: i128 = 0;
        for i in 0..pts.len() {
            let j = (i + 1) % pts.len();
            acc +=
                (pts[i].x as i128) * (pts[j].y as i128) - (pts[j].x as i128) * (pts[i].y as i128);
        }
        (acc as f64) * 0.5
    };
    let mut a = contour_area(&ep.contour.points).abs();
    for hole in &ep.holes {
        a -= contour_area(&hole.points).abs();
    }
    a.max(0.0)
}

fn polys_area(polys: &[ExPolygon]) -> f64 {
    polys.iter().map(expolygon_area).sum()
}

fn polys_contained_in_bbox(polys: &[ExPolygon], bbox: &BoundingBox2) -> bool {
    for ep in polys {
        for p in &ep.contour.points {
            if !bbox.contains_point(*p) {
                return false;
            }
        }
        for hole in &ep.holes {
            for p in &hole.points {
                if !bbox.contains_point(*p) {
                    return false;
                }
            }
        }
    }
    true
}

/// Group per-layer per-color cells. The BASE (empty variant_chain) cell is
/// keyed by `None`; painted Material/ToolIndex cells are keyed by `Some(t)`.
fn cells_by_color(slice_ir: &SliceIR) -> BTreeMap<Option<u32>, Vec<ExPolygon>> {
    let mut out: BTreeMap<Option<u32>, Vec<ExPolygon>> = BTreeMap::new();
    for region in &slice_ir.regions {
        let color = region
            .variant_chain
            .iter()
            .find(|(sem, _)| sem == "material")
            .and_then(|(_, pv)| {
                if let PaintValue::ToolIndex(t) = pv {
                    Some(*t)
                } else {
                    None
                }
            });
        let entry = out.entry(color).or_default();
        for ep in &region.polygons {
            entry.push(ep.clone());
        }
    }
    out
}

/// AC-4: on every layer that carries painted Material ToolIndex cells, the
/// per-color `ExPolygon` cells (a) have pairwise intersection area not greater
/// than `SCALED_EPSILON`, (b) are fully contained in the model XY bounding box,
/// and (c) each have non-zero area.
#[test]
fn cube_4color_mmu_partition_is_non_overlapping() {
    let mesh = Arc::new(load_cube_4color());
    let result = run_paint_segmentation(mesh.clone());
    let bbox = model_xy_bounding_box(&mesh);

    for (layer_idx, slice_ir) in result.iter().enumerate() {
        let cells = cells_by_color(slice_ir);

        // Only layers with at least one painted cell are meaningful for this AC.
        let painted_cells: Vec<(u32, Vec<ExPolygon>)> = cells
            .iter()
            .filter_map(|(color_opt, polys)| color_opt.map(|c| (c, polys.clone())))
            .collect();
        if painted_cells.is_empty() {
            continue;
        }

        // (a) non-overlapping: pairwise painted-vs-painted intersections are tiny.
        for i in 0..painted_cells.len() {
            for j in (i + 1)..painted_cells.len() {
                let overlap = slicer_core::polygon_ops::intersection_ex(
                    &painted_cells[i].1,
                    &painted_cells[j].1,
                );
                let overlap_area = polys_area(&overlap);
                assert!(
                    overlap_area <= SCALED_EPSILON,
                    "layer {layer_idx}: color {} vs color {} overlap area {overlap_area} > {SCALED_EPSILON}",
                    painted_cells[i].0,
                    painted_cells[j].0
                );
            }
        }

        // (b) containment + (c) non-zero area per painted cell.
        for (color, polys) in &painted_cells {
            let area = polys_area(polys);
            assert!(
                area > SCALED_EPSILON,
                "layer {layer_idx}: color {color} has zero or near-zero area ({area})"
            );
            assert!(
                polys_contained_in_bbox(polys, &bbox),
                "layer {layer_idx}: color {color} cell escapes model XY bbox {bbox:?}"
            );
        }
    }
}

/// AC-N3: every pair of cells (including the BASE residual cell) on every layer
/// is disjoint within `SCALED_EPSILON` tolerance. Adjacent Voronoi cells meet at
/// shared bisector boundaries, so their geometric intersection is the shared edge
/// and has negligible area.
#[test]
fn cube_4color_mmu_cells_are_disjoint() {
    let mesh = Arc::new(load_cube_4color());
    let result = run_paint_segmentation(mesh);

    for (layer_idx, slice_ir) in result.iter().enumerate() {
        let cells = cells_by_color(slice_ir);
        let cell_list: Vec<Vec<ExPolygon>> = cells.into_values().collect();
        if cell_list.len() < 2 {
            continue;
        }

        for i in 0..cell_list.len() {
            for j in (i + 1)..cell_list.len() {
                let overlap =
                    slicer_core::polygon_ops::intersection_ex(&cell_list[i], &cell_list[j]);
                let overlap_area = polys_area(&overlap);
                assert!(
                    overlap_area <= SCALED_EPSILON,
                    "layer {layer_idx}: cell pair ({i},{j}) overlap area {overlap_area} > {SCALED_EPSILON}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Regression: segmentation partitions area, it never removes it
// ---------------------------------------------------------------------------

fn cube_cilindrical_modifier_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_cilindrical_modifier.3mf"
    ))
}

/// Paint segmentation must not empty a layer that had geometry.
///
/// `cube_cilindrical_modifier.3mf`'s cube carries `paint_seam` facet data
/// (it was derived from `cube_4color.3mf`), so `mesh_has_any_paint` admits it
/// and the kernel runs on every layer. On layers below the lowest painted
/// facet the cell decomposition yields no cells at all, and BASE was being
/// emitted with the `None`-keyed residual — which on an empty map is `[]`.
/// The resulting geometry-less region set then replaced the real
/// cross-section, because the commit guard only checked that the replacement
/// was non-empty, and a set of geometry-less regions satisfies that.
///
/// The effect was silent and severe: every layer beneath the lowest painted
/// facet was lost. Sliced end to end, that fixture printed nothing at z=0.2 or
/// z=0.4 — no walls, no infill, only skirt. The onset is geometric rather than
/// a fixed layer index (at 0.1mm layers the first surviving layer is index 4,
/// at 0.2mm it is index 2), which is why it read as a fixture quirk rather
/// than a defect.
///
/// The invariant asserted here is the general one, not the fixture's specific
/// layer count: segmentation redistributes a layer's area among regions, so a
/// layer that had area before must still have area after.
#[test]
fn paint_segmentation_never_empties_a_layer_that_had_geometry() {
    let path = cube_cilindrical_modifier_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh = Arc::new(load_model(&path).expect("load cube_cilindrical_modifier.3mf"));

    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let layer_plan = build_50_layer_plan(&object_id);
    let before = build_initial_slice_ir(&object_id, &object_mesh, &layer_plan);
    let region_map = build_region_map(&object_id, LAYER_COUNT);

    let had_geometry: Vec<bool> = before
        .iter()
        .map(|layer| layer.regions.iter().any(|r| !r.polygons.is_empty()))
        .collect();
    assert!(
        had_geometry.iter().filter(|had| **had).count() >= 2,
        "the fixture must slice to at least two non-empty layers for this test \
         to mean anything; got {had_geometry:?}"
    );

    let after = execute_paint_segmentation(mesh, Arc::new(before), region_map)
        .expect("execute_paint_segmentation must succeed");

    let emptied: Vec<usize> = after
        .iter()
        .enumerate()
        .filter(|(i, layer)| {
            had_geometry[*i] && !layer.regions.iter().any(|r| !r.polygons.is_empty())
        })
        .map(|(i, _)| i)
        .collect();

    assert!(
        emptied.is_empty(),
        "paint segmentation emptied layer(s) {emptied:?} that had geometry before \
         it ran. Segmentation partitions a layer's area among regions; it never \
         removes it."
    );
}
