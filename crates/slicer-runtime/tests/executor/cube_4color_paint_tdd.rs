//! Paint pipeline tests for `cube_4color.3mf` — migrated to v2 driver.
//!
//! The cube (25mm side) has 4 Material ToolIndex values:
//!   0=orange, 1=green, 2=blue, 3=red
//!
//! Per-face paint layout (world-space after build transform translation):
//!   Front (-Y, Y≈92.5) — 4 colors banded by height (subdivided hex)
//!   Back  (+Y, Y≈117.5) — fully blue (ToolIndex 2)
//!   Left  (-X, X≈112.5) — orange with 4 circles of each color (subdivided hex)
//!   Right (+X, X≈137.5) — green (ToolIndex 1)
//!   Top   (+Z, Z≈24.9) — half red (ToolIndex 3) / half orange (ToolIndex 0)
//!   Bottom(-Z, Z≈0.1)  — one blue (ToolIndex 2) / one unpainted
//!
//! GREEN tests verify whole-facet paint survives the pipeline.
//! RED tests document gaps where sub-facet paint detail (circles, bands, half-faces
//! from hex subdivision) is lost because the v2 kernel requires `host-algos` feature
//! and the Voronoi pipeline is not yet exercised by these integration tests.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, GlobalLayer, LayerPlanIR, MeshIR, ObjectLayerRef, PaintValue, Point2, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
};
use slicer_model_io::load_model;

const LAYER_COUNT: u32 = 50;
const LAYER_HEIGHT_MM: f32 = 0.5;
const EPSILON: i64 = 100; // 0.01 mm in internal units

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

/// World-space face bounds for cube_4color after build transform (125,105,12.5).
/// Units: internal (1 unit = 100 nm, 1 mm = 10_000 units).
struct FaceBounds {
    x_min: i64,
    x_max: i64,
    y_min: i64,
    y_max: i64,
}

fn face_bounds() -> FaceBounds {
    FaceBounds {
        x_min: 1_125_000, // 112.5 mm
        x_max: 1_375_000, // 137.5 mm
        y_min: 925_000,   //  92.5 mm
        y_max: 1_175_000, // 117.5 mm
    }
}

fn is_on_left_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.x - fb.x_min).abs() < EPSILON
}
fn is_on_right_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.x - fb.x_max).abs() < EPSILON
}
fn is_on_front_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.y - fb.y_min).abs() < EPSILON
}
fn is_on_back_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.y - fb.y_max).abs() < EPSILON
}

/// Collect all unique Material ToolIndex values from variant_chain entries in a SliceIR.
///
/// v2 contract: Material paint lives in `variant_chain` entries with semantic name
/// "material", NOT in `segment_annotations[PaintSemantic::Material]`.
fn unique_material_tool_indices(slice_ir: &SliceIR) -> std::collections::HashSet<u32> {
    slice_ir
        .regions
        .iter()
        .flat_map(|r| r.variant_chain.iter())
        .filter_map(|(sem, pv)| {
            if sem == "material" {
                if let PaintValue::ToolIndex(t) = pv {
                    Some(*t)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Return all SlicedRegions in `slice_ir` whose polygons contain at least one point
/// satisfying `predicate`. Used for positional assertions against a specific face.
fn regions_covering<'a>(
    slice_ir: &'a SliceIR,
    predicate: impl Fn(Point2) -> bool,
) -> Vec<&'a SlicedRegion> {
    slice_ir
        .regions
        .iter()
        .filter(|r| {
            r.polygons
                .iter()
                .any(|exp| exp.contour.points.iter().any(|pt| predicate(*pt)))
        })
        .collect()
}

/// Return all SlicedRegions that have a polygon EDGE lying along a face — i.e. two
/// CONSECUTIVE contour vertices both satisfying `on_band` (the face plane ± tolerance).
///
/// This is the correct "covers the face" test. A single polygon vertex merely
/// touching the band at a shared CORNER is NOT coverage: adjacent painted faces
/// legitimately share their corner vertex (OrcaSlicer
/// `MultiMaterialSegmentation.cpp:547-548` — the bisector arc emanates from the
/// corner and is consumed by both cells; along the face edge itself only the exact
/// corner point belongs to the neighbour). The earlier "any vertex near the plane"
/// predicate counted that shared corner as bleed, which the segmentation papered
/// over by displacing the corner ~0.5mm inward — opening a visible cross-colour gap.
/// Requiring an edge (≥2 consecutive on-band vertices) still flags any real bleed (a
/// foreign region spanning the face interior has a wall running along it) while
/// permitting the geometrically-correct shared corner.
fn regions_with_edge_on_face<'a>(
    slice_ir: &'a SliceIR,
    on_band: impl Fn(Point2) -> bool,
) -> Vec<&'a SlicedRegion> {
    slice_ir
        .regions
        .iter()
        .filter(|r| {
            r.polygons.iter().any(|exp| {
                let pts = &exp.contour.points;
                let n = pts.len();
                n >= 2 && (0..n).any(|i| on_band(pts[i]) && on_band(pts[(i + 1) % n]))
            })
        })
        .collect()
}

/// Collect Material ToolIndex values from the variant_chains of a slice of SlicedRegions.
fn material_tool_indices_from_regions(regions: &[&SlicedRegion]) -> std::collections::HashSet<u32> {
    regions
        .iter()
        .flat_map(|r| r.variant_chain.iter())
        .filter_map(|(sem, pv)| {
            if sem == "material" {
                if let PaintValue::ToolIndex(t) = pv {
                    Some(*t)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Build initial `Vec<SliceIR>` by slicing `object_mesh` at each layer Z.
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

/// Build a minimal `RegionMapIR` with one BASE entry per layer.
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

/// Run the v2 pipeline on cube_4color with 50 layers.
fn run_v2(mesh: Arc<MeshIR>, layer_plan: &LayerPlanIR) -> Arc<Vec<SliceIR>> {
    let object_id = &mesh.objects[0].id;
    let object_mesh = mesh.objects[0].mesh.clone();
    let initial = build_initial_slice_ir(object_id, &object_mesh, layer_plan);
    let region_map = build_region_map(object_id, LAYER_COUNT);
    execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed")
}

// ---------------------------------------------------------------------------
// GREEN: whole-facet paint survives the pipeline
// ---------------------------------------------------------------------------

/// Smoke: full v2 pipeline does not panic.
#[test]
fn cube_4color_full_pipeline_no_panic() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );
    // Pipeline ran without panic; new_slice_ir must have 50 layers.
    assert_eq!(
        new_slice_ir.len() as u32,
        LAYER_COUNT,
        "v2 output must have {LAYER_COUNT} layers"
    );
}

/// Paint segmentation v2 produces at least 4 distinct Material ToolIndex regions
/// across the 50 layers via variant_chain entries (v2 contract).
///
/// NOTE: with the host-algos kernel disabled (default integration-test build),
/// v2 short-circuits through the no-paint or empty-regions path and variant_chains
/// remain empty. This test is a RED gate until host-algos is enabled.
/// TODO(closure-log P95): enable host-algos in integration-test build.
#[test]
fn cube_4color_paint_segmentation_4_tool_indices_across_layers() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let lp = build_50_layer_plan(object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    // Collect distinct ToolIndex values from variant_chain entries across all layers/regions.
    // v2 contract: Material paint lives in variant_chain (semantic name "material"),
    // NOT in segment_annotations[PaintSemantic::Material].
    let mut tool_indices = std::collections::HashSet::new();
    for layer in new_slice_ir.iter() {
        for region in &layer.regions {
            for (sem_name, pv) in &region.variant_chain {
                if sem_name == "material" {
                    if let PaintValue::ToolIndex(t) = pv {
                        tool_indices.insert(*t);
                    }
                }
            }
        }
    }

    assert!(
        tool_indices.len() >= 4,
        "expected >=4 distinct Material ToolIndex values across all layers via variant_chain, \
         got {}: {:?}\n\
         Gap: host-algos kernel not enabled — v2 kernel produces no variant_chain paint data \
         without it. RED gate until host-algos is wired into the integration-test build.",
        tool_indices.len(),
        tool_indices
    );
    for expected in [0, 1, 2, 3] {
        assert!(
            tool_indices.contains(&expected),
            "expected ToolIndex({expected}) in variant_chain entries across all layers"
        );
    }
}

/// All 50 layers are present in the v2 output.
#[test]
fn cube_4color_all_50_layers_have_layer_map_entries() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let lp = build_50_layer_plan(object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    assert_eq!(
        new_slice_ir.len() as u32,
        LAYER_COUNT,
        "expected v2 output length == LAYER_COUNT ({LAYER_COUNT}), got {}",
        new_slice_ir.len()
    );
    for i in 0..LAYER_COUNT {
        assert!(
            new_slice_ir.iter().any(|s| s.global_layer_index == i),
            "layer index {i} must be present in v2 output"
        );
    }
}

/// At Z=12.5mm, at least one SlicedRegion must carry a Material ToolIndex in its
/// variant_chain (v2 contract: Material paint is in variant_chain, not segment_annotations).
///
/// NOTE: requires host-algos kernel. Without it variant_chains are empty.
/// RED gate for AC-17. TODO(closure-log P95): enable host-algos.
#[test]
fn cube_4color_mid_layer_has_material_paint() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    // Find the layer closest to Z=12.5mm.
    let mid = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have at least one layer");

    // v2 contract: Material paint lives in variant_chain entries with sem_name "material".
    let material_region_count = mid
        .regions
        .iter()
        .filter(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "material" && matches!(pv, PaintValue::ToolIndex(_)))
        })
        .count();

    let tool_indices = unique_material_tool_indices(mid);
    eprintln!(
        "DIAGNOSTIC: mid-layer at Z≈12.5mm tool indices from variant_chain = {:?}",
        tool_indices
    );

    assert!(
        material_region_count > 0,
        "mid-layer at Z≈12.5mm must have at least one SlicedRegion with Material \
         ToolIndex in variant_chain (v2 contract).\n\
         Gap: host-algos kernel not enabled — variant_chains are empty without it. \
         RED gate until host-algos is wired in."
    );
}

// ---------------------------------------------------------------------------
// RED: whole-facet projection coverage gaps
// ---------------------------------------------------------------------------

/// Top face (Z≈24.9mm): two triangles — ToolIndex 0 (orange) and ToolIndex 3 (red).
/// v2 contract: tool indices appear in variant_chain entries.
/// Requires host-algos kernel + vertical-face projection fix.
#[test]
fn cube_4color_top_face_two_tool_indices_requires_projection_coverage() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let top_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 24.9f32)
                .abs()
                .partial_cmp(&(b.z - 24.9f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=24.9mm");

    // v2 contract: ToolIndex values appear in variant_chain (sem_name "material").
    let tool_indices = unique_material_tool_indices(top_layer);
    assert!(
        tool_indices.len() >= 2 && tool_indices.contains(&0) && tool_indices.contains(&3),
        "RED: top face at Z≈24.9mm should have BOTH ToolIndex 0 (orange) and \
         ToolIndex 3 (red) as SlicedRegion variant_chain entries. Got {} distinct ToolIndex \
         values in variant_chain: {tool_indices:?}.\n\
         Gap: host-algos kernel not enabled or vertical-face projection gap — \
         variant_chains are empty without it.",
        tool_indices.len()
    );
}

/// Top surface / ironing layer (Z≈24.75mm): should NOT contain green (ToolIndex 1)
/// or blue (ToolIndex 2) slivers leaked from side-face painted regions. Only orange
/// (0) and red (3) are expected — the two colours painted on the top face itself.
///
/// Regression test for bug where Phase-6/7 top/bottom solid-fill harvest left
/// sub-extrusion (~0.06 mm²) side-face regions on the top surface layer, causing
/// ironing to run a stray pass in the wrong colour.
#[test]
fn cube_4color_top_contact_layer_has_no_green_blue_sliver() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let top_surface = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 24.75f32)
                .abs()
                .partial_cmp(&(b.z - 24.75f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=24.75mm (top surface / ironing layer)");

    let tool_indices = unique_material_tool_indices(top_surface);
    assert!(
        !tool_indices.contains(&1) && !tool_indices.contains(&2),
        "BUG: top surface layer at Z≈24.75mm should NOT contain green (ToolIndex 1) \
         or blue (ToolIndex 2) slivers leaked from side-face painted regions. \
         Got tool indices: {tool_indices:?}. \
         Expected: subset of {{0 (orange), 3 (red)}}.\n\
         Root cause: Phase-6/7 top/bottom solid-fill harvest left sub-extrusion \
         side-face regions that survived into the final output."
    );
}

/// Bottom face (Z≈0.1mm): half ToolIndex 2 (blue) / half unpainted.
/// v2 contract: painted area has variant_chain [("material", ToolIndex(2))];
/// unpainted area has variant_chain [] (BASE).
/// Requires host-algos kernel + vertical-face projection fix.
#[test]
fn cube_4color_bottom_face_painted_and_unpainted_requires_projection_coverage() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let bot_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 0.1f32)
                .abs()
                .partial_cmp(&(b.z - 0.1f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=0.1mm");

    // v2 contract: painted = variant_chain contains ("material", ToolIndex(2));
    // unpainted = variant_chain is empty (BASE region).
    let has_painted_blue = bot_layer.regions.iter().any(|r| {
        r.variant_chain
            .iter()
            .any(|(sem, pv)| sem == "material" && *pv == PaintValue::ToolIndex(2))
    });
    let has_base_region = bot_layer.regions.iter().any(|r| r.variant_chain.is_empty());

    assert!(
        has_painted_blue && has_base_region,
        "RED: bottom face at Z≈0.1mm should have BOTH a ToolIndex(2)=blue variant_chain \
         region (painted triangle) AND a BASE variant_chain region (unpainted triangle). \
         has_painted_blue={has_painted_blue}, has_base_region={has_base_region}.\n\
         Gap: host-algos kernel not enabled or facet projection gap — variant_chains are \
         empty (only BASE) without it."
    );
}

/// Top face per-point variation: adjacent SlicedRegions from each triangle
/// should carry different ToolIndex values (0 and 3) in their variant_chains.
/// v2 contract: per-region variant_chain rather than per-contour-point annotations.
#[test]
fn cube_4color_top_face_per_point_variation() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let lp = build_50_layer_plan(object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let top_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 24.9f32)
                .abs()
                .partial_cmp(&(b.z - 24.9f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=24.9mm");

    // v2 contract: per-region variant_chain carries the tool index.
    // Expect at least two distinct Material ToolIndex regions at the top face.
    let top_face_tool_indices: std::collections::HashSet<u32> = top_layer
        .regions
        .iter()
        .flat_map(|r| r.variant_chain.iter())
        .filter_map(|(sem, pv)| {
            if sem == "material" {
                if let PaintValue::ToolIndex(t) = pv {
                    if *t == 0 || *t == 3 {
                        return Some(*t);
                    }
                }
            }
            None
        })
        .collect();

    assert!(
        top_face_tool_indices.len() >= 2,
        "RED: top face (Z≈24.9mm) should have BOTH ToolIndex 0 (orange) and ToolIndex 3 (red) \
         as distinct variant_chain entries. Got {} distinct ToolIndex 0/3 values: {:?}.\n\
         Gap: host-algos kernel required for per-region variant-chain production.",
        top_face_tool_indices.len(),
        top_face_tool_indices
    );
}

// ---------------------------------------------------------------------------
// RED: sub-facet detail lost — executable gap specifications
// ---------------------------------------------------------------------------

/// Front face (-Y, Y≈92.5): 4 colors banded by height.
/// v2 contract: each band is a distinct SlicedRegion with a different variant_chain.
/// Requires host-algos kernel + sub-facet stroke support.
#[test]
fn cube_4color_front_face_banded_by_z_requires_subfacet_strokes() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let lp = build_50_layer_plan(object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let test_zs = [2.0f32, 6.0, 10.0, 14.0, 18.0, 22.0];

    // Collect per-Z distinct ToolIndex sets from variant_chain entries.
    let mut front_tool_indices_by_z: HashMap<u32, std::collections::HashSet<u32>> = HashMap::new();

    for &test_z in &test_zs {
        let layer = new_slice_ir.iter().min_by(|a, b| {
            (a.z - test_z)
                .abs()
                .partial_cmp(&(b.z - test_z).abs())
                .unwrap()
        });
        let Some(layer) = layer else { continue };

        // v2 contract: Material paint is in variant_chain, not segment_annotations.
        let z_tool_indices: std::collections::HashSet<u32> = layer
            .regions
            .iter()
            .flat_map(|r| r.variant_chain.iter())
            .filter_map(|(sem, pv)| {
                if sem == "material" {
                    if let PaintValue::ToolIndex(t) = pv {
                        Some(*t)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if !z_tool_indices.is_empty() {
            front_tool_indices_by_z.insert(test_z as u32, z_tool_indices);
        }
    }

    let unique_sets: std::collections::HashSet<Vec<u32>> = front_tool_indices_by_z
        .values()
        .map(|s| {
            let mut v: Vec<u32> = s.iter().copied().collect();
            v.sort();
            v
        })
        .collect();

    assert!(
        unique_sets.len() >= 2,
        "RED: front face (banded by height) should produce different ToolIndex values in \
         variant_chains at different Z heights. Got {} unique ToolIndex sets across {} Z levels: {:?}\n\
         Gap: v2 kernel requires host-algos + sub-facet stroke support for banding.",
        unique_sets.len(),
        front_tool_indices_by_z.len(),
        front_tool_indices_by_z
    );
}

/// Left face (-X, X≈112.5): orange (ToolIndex 0) with 4 circles of each color.
/// v2 contract: each circle region is a distinct SlicedRegion with different variant_chain.
/// Requires host-algos kernel + sub-facet stroke support.
#[test]
fn cube_4color_left_face_circles_produce_per_point_variation() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let lp = build_50_layer_plan(object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // v2 contract: distinct ToolIndex values for the left face come from distinct
    // variant_chain entries across SlicedRegions, not per-contour-point annotations.
    let left_tool_indices: std::collections::HashSet<u32> = mid_layer
        .regions
        .iter()
        .flat_map(|r| r.variant_chain.iter())
        .filter_map(|(sem, pv)| {
            if sem == "material" {
                if let PaintValue::ToolIndex(t) = pv {
                    Some(*t)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    assert!(
        left_tool_indices.len() >= 2,
        "RED: left face (Z≈12.5mm) should have >=2 distinct ToolIndex values in variant_chain \
         entries (circle colors + background orange=0). Got {} unique values: {:?}\n\
         Gap: v2 kernel requires host-algos + sub-facet stroke support for circles.",
        left_tool_indices.len(),
        left_tool_indices
    );
}

// ---------------------------------------------------------------------------
// RED: per-face paint correctness — vertical-face projection gap
// ---------------------------------------------------------------------------

/// Right face (+X, X≈137.5): green (ToolIndex 1).
/// v2 contract: SlicedRegion(s) whose polygons cover the right face position (x≈x_max)
/// must carry variant_chain [("material", ToolIndex(1))].
/// Positional assertion: does NOT require the whole layer to be ToolIndex(1) only.
/// Requires host-algos kernel + vertical-face projection.
#[test]
fn cube_4color_right_face_uniform_requires_vertical_face_projection() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // Positional query: find regions with a wall EDGE along the right face (x ≈ x_max).
    // A region "covers" the face only if it has an edge there, not a single shared
    // corner vertex (see regions_with_edge_on_face).
    let fb = face_bounds();
    // Use a generous tolerance: 1% of cube width (25mm) = 0.25mm = 2500 units.
    let tol = (fb.x_max - fb.x_min) / 100;
    let right_face_regions = regions_with_edge_on_face(mid_layer, |pt| pt.x >= fb.x_max - tol);

    // v2 contract: the right-face regions must carry ToolIndex(1)=green.
    let right_face_tools = material_tool_indices_from_regions(&right_face_regions);

    assert!(
        right_face_tools.contains(&1),
        "RED: right face (x≈{}) must have variant_chain ToolIndex(1)=green in covering \
         SlicedRegions. Got: {right_face_tools:?} from {} covering region(s).\n\
         Gap: host-algos kernel not enabled or vertical-face projection not implemented.",
        fb.x_max,
        right_face_regions.len()
    );
    // Stronger: the right-face should be uniformly ToolIndex(1) (no bleed from other faces).
    assert!(
        right_face_tools.len() == 1 && right_face_tools.contains(&1),
        "RED: right face should be uniformly ToolIndex(1)=green (no bleed). \
         Got right-face covering region tools: {right_face_tools:?}.\n\
         Gap: vertical face projection must confine ToolIndex(1) to the right face.",
    );
}

/// AC-2 — painted entity resolves a real (small) tool index through the variant-chain
/// resolver, never a synthesised `region_id` identity value.
///
/// Contract (Step 2 invariant / Step 3c reframe): for a material-painted model
/// (cube_4color: tools 0-3), every painted SlicedRegion must carry a
/// `variant_chain` entry `("material", ToolIndex(t))` where `t < 16`.  A
/// `region_id`-identity leak would produce t ≥ PAINT_VARIANT_REGION_ID_STRIDE
/// (1_000_000), which this test explicitly rejects.
///
/// This is the honest AC-2 assertion: the fuzzy-painted cube has NO tool (single
/// tool, tool=0 everywhere), so we assert against cube_4color which genuinely
/// has 4 Material ToolIndex values (0-3).
#[test]
fn painted_entity_resolves_real_tool() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    // Collect all (object_id, region_id, tool_index) triples from variant_chains.
    let mut found_any_material = false;
    for layer in new_slice_ir.iter() {
        for region in &layer.regions {
            for (sem_name, value) in &region.variant_chain {
                if sem_name == "material" {
                    found_any_material = true;
                    if let PaintValue::ToolIndex(t) = value {
                        // AC-2 core: tool index must be a real small value (< 16),
                        // never a region_id identity leak (≥ 1_000_000).
                        assert!(
                            *t < 16,
                            "AC-2: material paint resolved tool_index={t} which looks like a \
                             region_id identity leak (must be < 16 for cube_4color's 4 tools). \
                             region_id={}, layer={}, object={}",
                            region.region_id,
                            layer.global_layer_index,
                            region.object_id
                        );
                    }
                }
            }
        }
    }

    assert!(
        found_any_material,
        "AC-2: expected at least one SlicedRegion with variant_chain material entry \
         in cube_4color output. Pipeline produced no material-painted regions — \
         check host-algos feature gate or paint segmentation path."
    );
}

/// AC-G3 — first-layer / bottom-shell perimeter colour.
///
/// On the first layer (Z≈0.2mm), OrcaSlicer's bottom-face-dominance rule means that
/// the bottom-face projection wins over any vertical-face (side-wall) paint.
///
/// In cube_4color the bottom face has TWO triangles:
///   • one painted blue (ToolIndex 2)   → produces a ToolIndex(2) bottom projection
///   • one UNPAINTED                    → defaults to the base extruder (ToolIndex 0)
///                                         and produces a ToolIndex(0) bottom projection
///
/// Together they cover the full cross-section at layer 0.  After the Phase-7 merge
/// the first-layer region set MUST contain ToolIndex(0) (from the unpainted-bottom half
/// defaulting to the base extruder) and MUST NOT be `{ToolIndex(1)}` (green right-face
/// only), which would indicate the bottom-face projection failed.
#[test]
fn cube_4color_first_layer_perimeter_colour_matches_bottom_face() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    // Select the first layer (minimum z in the output).
    let first_layer = new_slice_ir
        .iter()
        .min_by(|a, b| a.z.partial_cmp(&b.z).unwrap())
        .expect("must have at least one layer");

    let tools = unique_material_tool_indices(first_layer);
    eprintln!(
        "DIAGNOSTIC AC-G3: first layer Z={} tool indices = {:?}",
        first_layer.z, tools
    );

    // The base-extruder colour (ToolIndex 0 = orange) must appear: the unpainted
    // half of the bottom face defaults to the base extruder and its bottom
    // projection is full-area at the contact layer (layer 0).
    assert!(
        tools.contains(&0),
        "AC-G3: first layer (Z={}) material-tool set must include ToolIndex(0) (orange / base \
         extruder from unpainted bottom-face projection). Got: {:?}.\n\
         Check Phase-6 bottom projection for ToolIndex(0) at layer 0 and Phase-7 merge.",
        first_layer.z,
        tools
    );

    // The set must NOT be exactly {{ToolIndex(1)}} alone — that would mean the
    // green right-face side-wall colour dominated the entire first layer, which
    // violates the bottom-face-dominance rule.
    let only_green = tools.len() == 1 && tools.contains(&1);
    assert!(
        !only_green,
        "AC-G3: first layer (Z={}) material-tool set is {{ToolIndex(1)}} (green only), \
         indicating bottom-face projection did not reach layer 0. Got: {:?}.",
        first_layer.z, tools
    );
}

/// Back face (+Y, Y≈117.5): fully blue (ToolIndex 2).
/// v2 contract: SlicedRegion(s) whose polygons cover the back face position (y≈y_max)
/// must carry variant_chain [("material", ToolIndex(2))].
/// Positional assertion: does NOT require the whole layer to be ToolIndex(2) only.
/// Requires host-algos kernel + vertical-face projection.
#[test]
fn cube_4color_back_face_uniform_requires_vertical_face_projection() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // Positional query: find regions with a wall EDGE along the back face (y ≈ y_max).
    // A region "covers" the face only if it has an edge there, not a single shared
    // corner vertex (see regions_with_edge_on_face).
    let fb = face_bounds();
    // Use a generous tolerance: 1% of cube depth (25mm) = 0.25mm = 2500 units.
    let tol = (fb.y_max - fb.y_min) / 100;
    let back_face_regions = regions_with_edge_on_face(mid_layer, |pt| pt.y >= fb.y_max - tol);

    // v2 contract: the back-face regions must carry ToolIndex(2)=blue.
    let back_face_tools = material_tool_indices_from_regions(&back_face_regions);

    assert!(
        back_face_tools.contains(&2),
        "RED: back face (y≈{}) must have variant_chain ToolIndex(2)=blue in covering \
         SlicedRegions. Got: {back_face_tools:?} from {} covering region(s).\n\
         Gap: host-algos kernel not enabled or vertical-face projection not implemented.",
        fb.y_max,
        back_face_regions.len()
    );
    // Stronger: the back-face should be uniformly ToolIndex(2) (no bleed from other faces).
    assert!(
        back_face_tools.len() == 1 && back_face_tools.contains(&2),
        "RED: back face should be uniformly ToolIndex(2)=blue (no bleed). \
         Got back-face covering region tools: {back_face_tools:?}.\n\
         Gap: vertical face projection must confine ToolIndex(2) to the back face.",
    );
}

/// Regression for commit e1fb1781: the bottom-shell SOLID INFILL must be coloured by
/// the BOTTOM-FACE tools (blue=2 painted, orange=0 = the unpainted half's base
/// extruder), NOT by the vertical SIDE-FACE tools (green=1, red=3).
///
/// The Phase-6/7 merge previously harvested top/bottom solid fill only from the BASE
/// region, but the pre-Phase-6 step distributes that fill to the per-colour side-face
/// regions. So the top/bottom-face projection took over only the WALLS while the
/// INFILL stayed side-coloured (bottom-layer infill green/red, top-surface ironing
/// following the wrong colour). The fix harvests solid fill from EVERY overlapping
/// region. This test seeds a full-area `bottom_solid_fill` (the minimal harness does
/// not run ShellClassification) and asserts the face colours own it after Phase 6.
#[test]
fn cube_4color_bottom_shell_infill_uses_bottom_face_colour_regression() {
    let mesh = load_cube_4color();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let lp = build_50_layer_plan(&object_id);

    let mut initial = build_initial_slice_ir(&object_id, &object_mesh, &lp);
    // Bottom (contact) layer = lowest z; seed its solid bottom shell.
    let bottom_idx = initial
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.z.partial_cmp(&b.z).unwrap())
        .map(|(i, _)| i)
        .expect("must have layers");
    for r in &mut initial[bottom_idx].regions {
        r.bottom_solid_fill = r.polygons.clone();
    }

    let region_map = build_region_map(&object_id, LAYER_COUNT);
    let out = execute_paint_segmentation(Arc::new(mesh), Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed");

    fn fill_area(polys: &[slicer_ir::ExPolygon]) -> f64 {
        let mut a = 0.0_f64;
        for ep in polys {
            let p = &ep.contour.points;
            if p.len() >= 3 {
                let mut acc = 0i128;
                for i in 0..p.len() {
                    let j = (i + 1) % p.len();
                    acc +=
                        (p[i].x as i128) * (p[j].y as i128) - (p[j].x as i128) * (p[i].y as i128);
                }
                a += (acc as f64).abs() * 0.5;
            }
        }
        a
    }

    let mut per_tool: std::collections::BTreeMap<u32, f64> = std::collections::BTreeMap::new();
    for r in &out[bottom_idx].regions {
        let tool = r
            .variant_chain
            .iter()
            .find_map(|(n, v)| match (n.as_str(), v) {
                ("material", PaintValue::ToolIndex(t)) => Some(*t),
                _ => None,
            });
        if let Some(t) = tool {
            *per_tool.entry(t).or_default() += fill_area(&r.bottom_solid_fill);
        }
    }

    let face = per_tool.get(&0).copied().unwrap_or(0.0) + per_tool.get(&2).copied().unwrap_or(0.0);
    let side = per_tool.get(&1).copied().unwrap_or(0.0) + per_tool.get(&3).copied().unwrap_or(0.0);
    assert!(
        face > 1.0e9,
        "bottom-face colours (orange=0 base / blue=2) must own the bottom-shell solid \
         infill; got per_tool={per_tool:?}"
    );
    assert!(
        side < face * 0.01,
        "REGRESSION e1fb1781: side-face colours (green=1, red=3) must NOT colour the \
         bottom-shell solid infill. side={side:.0} face={face:.0} per_tool={per_tool:?}"
    );
}
