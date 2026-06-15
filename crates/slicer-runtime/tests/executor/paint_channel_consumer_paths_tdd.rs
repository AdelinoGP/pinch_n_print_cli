//! AC-7 consumer-path tests: paint strokes loaded from disk (or constructed in-memory)
//! reach `execute_paint_segmentation` and produce observable SlicedRegion effects,
//! per channel.
//!
//! # Routing confirmed (Step A):
//!
//! All paint semantics flow through the same `painted_subsets` BTreeMap path in
//! `execute_paint_segmentation` (mod.rs ~L860-1030). Both `facet_values` and
//! `strokes` contribute triangles to that map. The resulting per-layer polygons are
//! merged into `SlicedRegion.variant_chain` as `(sem_name, value)` pairs — for
//! every semantic including Material, FuzzySkin, SupportEnforcer, SupportBlocker,
//! and Custom("seam_enforcer" / "seam_blocker").
//!
//! Modifier-volume SupportEnforcer/Blocker land in `segment_annotations` (D14 path,
//! separate from the PaintLayer/hex-stroke path).
//!
//! # D-98-SEAM-NO-CONSUMER
//!
//! Seam paint (Custom("seam_enforcer") / Custom("seam_blocker")) flows into
//! SlicedRegion.variant_chain — the data reaches SliceIR. However, NO live downstream
//! module reads variant_chain("seam_enforcer"/"seam_blocker"). The seam-placer module
//! uses geometric SeamCandidate scores only. This is recorded as deviation D-98-SEAM-NO-CONSUMER.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, BoundingBox3, FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR,
    MeshIR, ObjectConfig, ObjectLayerRef, ObjectMesh, PaintLayer, PaintSemantic, PaintValue,
    Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    Transform3d, CURRENT_MESH_IR_SCHEMA_VERSION, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_model_io::load_model;

const LAYER_COUNT: u32 = 50;
const LAYER_HEIGHT_MM: f32 = 0.5;

// ---------------------------------------------------------------------------
// Helpers shared across all tests in this file
// ---------------------------------------------------------------------------

fn cube_4color_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_4color.3mf"
    ))
}

fn cube_fuzzy_painted_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_fuzzyPainted.3mf"
    ))
}

fn bridge_support_enforcers_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/bridge_support_enforcers.3mf"
    ))
}

fn cube_cilindrical_modifier_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_cilindrical_modifier.3mf"
    ))
}

fn build_layer_plan(object_id: &str, layer_count: u32) -> Arc<LayerPlanIR> {
    let global_layer_indices: Vec<u32> = (0..layer_count).collect();
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
    object_mesh: &IndexedTriangleSet,
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

/// Build a 1mm unit cube MeshIR (vertices in mm, 12 triangles, 8 vertices).
fn unit_cube_its() -> IndexedTriangleSet {
    let pt3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            pt3(0.0, 0.0, 0.0),
            pt3(1.0, 0.0, 0.0),
            pt3(1.0, 1.0, 0.0),
            pt3(0.0, 1.0, 0.0),
            pt3(0.0, 0.0, 1.0),
            pt3(1.0, 0.0, 1.0),
            pt3(1.0, 1.0, 1.0),
            pt3(0.0, 1.0, 1.0),
        ],
        #[rustfmt::skip]
        indices: vec![
            0, 2, 1, 0, 3, 2,   // bottom
            4, 5, 6, 4, 6, 7,   // top
            0, 1, 5, 0, 5, 4,   // front
            2, 3, 7, 2, 7, 6,   // back
            0, 4, 7, 0, 7, 3,   // left
            1, 2, 6, 1, 6, 5,   // right
        ],
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn default_build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: Point3 {
            x: 0.0_f32,
            y: 0.0_f32,
            z: 0.0_f32,
        },
        max: Point3 {
            x: 250.0_f32,
            y: 210.0_f32,
            z: 220.0_f32,
        },
    }
}

/// Build a MeshIR with one object using the unit cube ITS and the given PaintLayers.
fn make_single_object_mesh_ir(object_id: &str, paint_layers: Vec<PaintLayer>) -> Arc<MeshIR> {
    let mesh = unit_cube_its();
    Arc::new(MeshIR {
        schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh,
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: Some(FacetPaintData {
                layers: paint_layers,
            }),
            world_z_extent: None,
        }],
        build_volume: default_build_volume(),
    })
}

/// Build a SliceIR for a unit-cube (1mm x 1mm x 1mm) using `slice_mesh_ex`.
///
/// Uses 8 layers at 0.125mm spacing (z = 0.0625, 0.1875, ..., 0.9375mm) so that
/// the top face (z=1.0mm) falls within the top slab, enabling `top_bottom` propagation
/// to reach the layers below via the default `top_shell_layers=3` window.
fn make_unit_cube_slice_ir(object_id: &str, its: &IndexedTriangleSet) -> Arc<Vec<SliceIR>> {
    const LAYER_HEIGHT: f32 = 0.125;
    const N: usize = 8;
    let zs: Vec<f32> = (0..N).map(|i| LAYER_HEIGHT * (i as f32 + 0.5)).collect();
    let slabs = slice_mesh_ex(its, &zs);
    let layers: Vec<SliceIR> = zs
        .iter()
        .enumerate()
        .map(|(idx, &z)| {
            let polys = slabs.get(idx).cloned().unwrap_or_default();
            SliceIR {
                schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: polys.clone(),
                    infill_areas: polys,
                    effective_layer_height: LAYER_HEIGHT,
                    segment_annotations: HashMap::new(),
                    ..Default::default()
                }],
            }
        })
        .collect();
    Arc::new(layers)
}

/// Build a RegionMapIR with BASE entries for 8 layers (matching make_unit_cube_slice_ir).
fn make_unit_cube_region_map(object_id: &str) -> Arc<RegionMapIR> {
    const N: u32 = 8;
    let mut entries = HashMap::new();
    for i in 0..N {
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

// ---------------------------------------------------------------------------
// Test 1: Material / color channel (cube_4color.3mf fixture)
// ---------------------------------------------------------------------------

/// Regression guard: material paint strokes loaded from cube_4color.3mf reach
/// `execute_paint_segmentation` and populate SlicedRegion.variant_chain with
/// ("material", PaintValue::ToolIndex(n)) entries.
///
/// Routing confirmed: Material -> variant_chain via `painted_subsets` path (not segment_annotations).
///
/// NOTE: This test requires the host-algos kernel to produce non-empty variant_chains.
/// Without it the pipeline short-circuits and variant_chains remain empty.
/// Marked as a RED gate (same as cube_4color_paint_tdd::cube_4color_paint_segmentation_4_tool_indices_across_layers,
/// which already covers this assertion). Added here as a per-AC-7 regression guard with
/// explicit stroke-path documentation.
///
/// DUPLICATE NOTE: cube_4color_paint_tdd.rs::cube_4color_paint_segmentation_4_tool_indices_across_layers
/// already asserts this from the fixture. This test is a lighter, focused regression guard
/// that explicitly documents the stroke routing path for AC-7.
#[test]
fn paint_channel_color_strokes_reach_material_variant_chain() {
    let path = cube_4color_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh = Arc::new(load_model(&path).expect("load cube_4color.3mf should succeed"));

    // Verify the fixture actually has paint data with strokes (or facet_values).
    let has_paint_data = mesh.objects.iter().any(|obj| {
        obj.paint_data
            .as_ref()
            .map(|pd| !pd.layers.is_empty())
            .unwrap_or(false)
    });
    assert!(
        has_paint_data,
        "cube_4color.3mf must have paint_data on at least one object"
    );

    let object_id = &mesh.objects[0].id;
    let lp = build_layer_plan(object_id, LAYER_COUNT);
    let initial = build_initial_slice_ir(object_id, &mesh.objects[0].mesh, &lp);
    let region_map = build_region_map(object_id, LAYER_COUNT);

    let result = execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed");

    // Routing assertion: Material paint -> variant_chain ("material", ToolIndex(n)).
    // NOT in segment_annotations.
    let has_material_variant_chain = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "material" && matches!(pv, PaintValue::ToolIndex(_)))
        })
    });

    assert!(
        has_material_variant_chain,
        "AC-7: Material paint from cube_4color.3mf must reach at least one \
         SlicedRegion.variant_chain as (\"material\", ToolIndex(n)).\n\
         Routing: facet_values + strokes both flow through painted_subsets -> variant_chain.\n\
         Gap: host-algos kernel not enabled — variant_chains are empty without it. \
         RED gate until host-algos is wired in."
    );
}

// ---------------------------------------------------------------------------
// Test 2: FuzzySkin channel (cube_fuzzyPainted.3mf fixture) — PRIMARY P98 WIN
// ---------------------------------------------------------------------------

/// PRIMARY P98 WIN: FuzzySkin paint strokes loaded from cube_fuzzyPainted.3mf reach
/// `execute_paint_segmentation` and populate SlicedRegion.variant_chain with
/// ("fuzzy_skin", PaintValue::Flag(true)) entries.
///
/// Routing confirmed: FuzzySkin -> variant_chain ("fuzzy_skin", Flag(true))
/// via `painted_subsets` path. Both facet_values AND strokes triangles contribute.
///
/// This test explicitly verifies the stroke path: the 3MF fixture contains hex strokes
/// (sub-facet paint detail) in addition to whole-facet values. The stroke triangles are
/// appended to the painted_subset vertex pool (mod.rs ~L908-929) and sliced independently.
///
/// NOTE: cube_fuzzy_painted_tdd.rs::cube_fuzzy_painted_paint_segmentation_emits_fuzzy_regions
/// already covers the same assertion for this fixture. This test is a focused regression guard
/// for AC-7 that also verifies strokes are present in the loaded fixture.
#[test]
fn paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain() {
    let path = cube_fuzzy_painted_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh = Arc::new(load_model(&path).expect("load cube_fuzzyPainted.3mf should succeed"));

    // Verify the fixture has FuzzySkin paint layers.
    let fuzzy_layers: Vec<_> = mesh
        .objects
        .iter()
        .flat_map(|obj| {
            obj.paint_data
                .iter()
                .flat_map(|pd| pd.layers.iter())
                .filter(|l| l.semantic == PaintSemantic::FuzzySkin)
        })
        .collect();
    assert!(
        !fuzzy_layers.is_empty(),
        "cube_fuzzyPainted.3mf must have at least one FuzzySkin PaintLayer"
    );

    // Diagnostic: log whether strokes are present (the P98 primary concern).
    let stroke_count: usize = fuzzy_layers.iter().map(|l| l.strokes.len()).sum();
    let facet_value_count: usize = fuzzy_layers
        .iter()
        .map(|l| l.facet_values.iter().filter(|v| v.is_some()).count())
        .sum();
    eprintln!(
        "DIAGNOSTIC [P98]: cube_fuzzyPainted.3mf FuzzySkin — \
         facet_values={facet_value_count}, strokes={stroke_count}"
    );

    let object_id = &mesh.objects[0].id;
    let lp = build_layer_plan(object_id, LAYER_COUNT);
    let initial = build_initial_slice_ir(object_id, &mesh.objects[0].mesh, &lp);
    let region_map = build_region_map(object_id, LAYER_COUNT);

    let result = execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed");

    // Routing assertion: FuzzySkin -> variant_chain ("fuzzy_skin", Flag(true)).
    // NOT in segment_annotations.
    let has_fuzzy_variant_chain = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "fuzzy_skin" && matches!(pv, PaintValue::Flag(true)))
        })
    });

    // Also confirm no FuzzySkin in segment_annotations (wrong routing would put it there).
    let has_fuzzy_in_annotations = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.segment_annotations
                .contains_key(&PaintSemantic::FuzzySkin)
        })
    });

    assert!(
        !has_fuzzy_in_annotations,
        "AC-7: FuzzySkin must NOT appear in segment_annotations — \
         that path is for modifier volumes only (D14). \
         Found FuzzySkin in segment_annotations — routing regression."
    );

    assert!(
        has_fuzzy_variant_chain,
        "AC-7 / PRIMARY P98 WIN: FuzzySkin paint from cube_fuzzyPainted.3mf must reach \
         at least one SlicedRegion.variant_chain as (\"fuzzy_skin\", Flag(true)).\n\
         Routing: both facet_values and strokes flow through painted_subsets -> variant_chain.\n\
         facet_value_count={facet_value_count}, stroke_count={stroke_count}.\n\
         Gap: host-algos kernel not enabled — variant_chains are empty without it. \
         RED gate until host-algos is wired in."
    );
}

// ---------------------------------------------------------------------------
// Test 3: SupportEnforcer channel — end-to-end disk fixture (bridge_support_enforcers.3mf)
// ---------------------------------------------------------------------------

/// SupportEnforcer paint strokes loaded from `bridge_support_enforcers.3mf` reach
/// `execute_paint_segmentation` and populate SlicedRegion.variant_chain.
///
/// # Fixture layout (post P98 edit):
///
/// bridge_support_enforcers.3mf now has 3 objects:
///   obj[0], obj[1] — modifier-volume objects (paint_data=None)
///   obj[2] — NEW bridge body with PaintLayer SupportEnforcer: 2 facet_values + 8899 sub-facet strokes
///
/// # Routing confirmed:
///
/// PaintLayer SupportEnforcer (paint_supports hex strokes) flows through the same
/// `painted_subsets` path as Material and FuzzySkin. The resulting polygons are merged
/// into variant_chain as ("support_enforcer", Flag(true)) — NOT into segment_annotations.
/// segment_annotations is populated ONLY by the D14 modifier-volume path.
///
/// NOTE: painted-support-hex has NO live downstream consumer reading variant_chain
/// ("support_enforcer"). The support-planner module reads `paint_layers` (WIT side)
/// from the mesh, not from SlicedRegion.variant_chain. This is a variant_chain dead-end
/// analogous to seam (pre-existing, not P98's change). See D-98-SEAM-NO-CONSUMER pattern.
#[test]
fn paint_channel_supports_strokes_reach_consumer() {
    let path = bridge_support_enforcers_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh =
        Arc::new(load_model(&path).expect("load bridge_support_enforcers.3mf should succeed"));

    // Find the object with a SupportEnforcer PaintLayer (obj[2] per P98 fixture edit).
    let painted_obj_idx = mesh.objects.iter().position(|obj| {
        obj.paint_data
            .as_ref()
            .map(|pd| {
                pd.layers
                    .iter()
                    .any(|l| l.semantic == PaintSemantic::SupportEnforcer)
            })
            .unwrap_or(false)
    });
    let painted_obj_idx = painted_obj_idx
        .expect("bridge_support_enforcers.3mf must have at least one object with a SupportEnforcer PaintLayer");

    // Diagnostic: log stroke count (P98 primary concern).
    let painted_obj = &mesh.objects[painted_obj_idx];
    let enforcer_layer = painted_obj
        .paint_data
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::SupportEnforcer)
        .unwrap();
    let stroke_count = enforcer_layer.strokes.len();
    let facet_value_count = enforcer_layer
        .facet_values
        .iter()
        .filter(|v| v.is_some())
        .count();
    eprintln!(
        "DIAGNOSTIC [P98-supports disk]: obj[{painted_obj_idx}] SupportEnforcer — \
         facet_values={facet_value_count}, strokes={stroke_count}"
    );
    assert!(
        facet_value_count > 0 || stroke_count > 0,
        "fixture obj[{painted_obj_idx}] must have SupportEnforcer paint data"
    );

    // Build layers spanning the object's actual Z extent so painted triangles
    // fall within the slice planes. Use the mesh's vertex Z range as the guide.
    let obj_z_min = painted_obj
        .mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::INFINITY, f32::min);
    let obj_z_max = painted_obj
        .mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::NEG_INFINITY, f32::max);
    let obj_z_min = if obj_z_min.is_infinite() {
        0.0
    } else {
        obj_z_min
    };
    let obj_z_max = if obj_z_max.is_infinite() || obj_z_max <= obj_z_min {
        obj_z_min + 25.0
    } else {
        obj_z_max
    };
    let layer_count = LAYER_COUNT;
    let layer_height = ((obj_z_max - obj_z_min) / layer_count as f32).max(0.1);

    let object_id = &painted_obj.id;

    // Build the layer plan spanning the actual object Z range.
    let global_layer_indices: Vec<u32> = (0..layer_count).collect();
    let layers: Vec<slicer_ir::GlobalLayer> = global_layer_indices
        .iter()
        .map(|idx| slicer_ir::GlobalLayer {
            index: *idx,
            z: obj_z_min + layer_height * (*idx as f32 + 0.5),
            active_regions: vec![slicer_ir::ActiveRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                resolved_config: slicer_ir::ResolvedConfig::default(),
                effective_layer_height: layer_height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    let lp = Arc::new(slicer_ir::LayerPlanIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: layers,
        object_participation: std::collections::HashMap::from([(
            object_id.to_string(),
            global_layer_indices
                .iter()
                .copied()
                .enumerate()
                .map(|(local_idx, global_idx)| slicer_ir::ObjectLayerRef {
                    local_layer_index: local_idx as u32,
                    global_layer_index: global_idx,
                    effective_layer_height: layer_height,
                })
                .collect(),
        )]),
    });

    let initial = build_initial_slice_ir(object_id, &painted_obj.mesh, &lp);
    let region_map = build_region_map(object_id, layer_count);

    let result = execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed");

    assert!(!result.is_empty(), "result must have at least one layer");

    // Routing assertion: SupportEnforcer PaintLayer (facet_values + strokes) ->
    // variant_chain ("support_enforcer", Flag(true)).
    // This is the PAINT-LAYER path, NOT the modifier-volume (D14) path.
    let has_support_enforcer_variant_chain = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "support_enforcer" && matches!(pv, PaintValue::Flag(true)))
        })
    });

    // NOTE: bridge_support_enforcers.3mf has modifier volumes (support_enforcer / support_blocker)
    // on obj[0] and obj[1]. When the full MeshIR is passed to execute_paint_segmentation, those
    // modifier volumes are sliced via the D14 path and populate segment_annotations on BASE chains
    // for ALL objects (including obj[2]). This is correct D14 behavior, not a routing regression.
    // We do NOT assert that segment_annotations is empty here.
    let has_support_enforcer_in_annotations = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.segment_annotations
                .contains_key(&PaintSemantic::SupportEnforcer)
        })
    });

    eprintln!(
        "DIAGNOSTIC [P98-supports disk]: SupportEnforcer via PaintLayer strokes — \
         variant_chain={has_support_enforcer_variant_chain}, \
         segment_annotations={has_support_enforcer_in_annotations}\n\
         (segment_annotations=true is expected: obj[0]/obj[1] have modifier volumes → D14 path)"
    );

    // Primary assertion: painted hex strokes from obj[2] reach variant_chain.
    assert!(
        has_support_enforcer_variant_chain,
        "AC-7: SupportEnforcer PaintLayer (facet_values + hex strokes) from \
         bridge_support_enforcers.3mf obj[{painted_obj_idx}] must reach at least one \
         SlicedRegion.variant_chain as (\"support_enforcer\", Flag(true)).\n\
         facet_value_count={facet_value_count}, stroke_count={stroke_count}.\n\
         Routing: PaintLayer strokes -> painted_subsets/top_bottom -> variant_chain."
    );
}

// ---------------------------------------------------------------------------
// Test 4: Seam channel — disk fixture (cube_cilindrical_modifier.3mf),
//         data reaches SliceIR, no live consumer (D-98-SEAM-NO-CONSUMER)
// ---------------------------------------------------------------------------

/// Seam paint (Custom("seam_enforcer")) loaded from `cube_cilindrical_modifier.3mf`
/// reaches `execute_paint_segmentation` and populates SlicedRegion.variant_chain.
///
/// # Fixture layout (post P98 edit):
///
/// cube_cilindrical_modifier.3mf has 1 object (12 tris) with
/// PaintLayer Custom("seam_enforcer"): 3 facet_values + 2706 sub-facet strokes.
///
/// # D-98-SEAM-NO-CONSUMER
///
/// Data reaches SlicedRegion.variant_chain as ("seam_enforcer", _) — the routing works.
/// HOWEVER, NO live downstream module reads variant_chain("seam_enforcer").
/// The seam-placer uses geometric SeamCandidate scores only (not paint annotations).
/// This gap is registered as deviation D-98-SEAM-NO-CONSUMER.
#[test]
fn paint_channel_seam_strokes_have_no_live_consumer() {
    let path = cube_cilindrical_modifier_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh =
        Arc::new(load_model(&path).expect("load cube_cilindrical_modifier.3mf should succeed"));

    // Verify the fixture has a Custom("seam_enforcer") PaintLayer.
    let seam_layers: Vec<_> = mesh
        .objects
        .iter()
        .flat_map(|obj| {
            obj.paint_data
                .iter()
                .flat_map(|pd| pd.layers.iter())
                .filter(|l| matches!(&l.semantic, PaintSemantic::Custom(s) if s == "seam_enforcer"))
        })
        .collect();
    assert!(
        !seam_layers.is_empty(),
        "cube_cilindrical_modifier.3mf must have at least one Custom(\"seam_enforcer\") PaintLayer"
    );

    let stroke_count: usize = seam_layers.iter().map(|l| l.strokes.len()).sum();
    let facet_value_count: usize = seam_layers
        .iter()
        .map(|l| l.facet_values.iter().filter(|v| v.is_some()).count())
        .sum();
    eprintln!(
        "DIAGNOSTIC [P98-seam disk / D-98-SEAM-NO-CONSUMER]: \
         cube_cilindrical_modifier.3mf seam_enforcer — \
         facet_values={facet_value_count}, strokes={stroke_count}"
    );

    let object_id = &mesh.objects[0].id;
    let lp = build_layer_plan(object_id, LAYER_COUNT);
    let initial = build_initial_slice_ir(object_id, &mesh.objects[0].mesh, &lp);
    let region_map = build_region_map(object_id, LAYER_COUNT);

    let result = execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed for seam disk test");

    assert!(!result.is_empty(), "result must have at least one layer");

    // Routing assertion: Custom("seam_enforcer") -> variant_chain ("seam_enforcer", _).
    let has_seam_variant_chain = result.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, _pv)| sem == "seam_enforcer")
        })
    });

    eprintln!(
        "DIAGNOSTIC [P98-seam disk / D-98-SEAM-NO-CONSUMER]: \
         variant_chain populated={has_seam_variant_chain}\n\
         DATA REACHES SliceIR via variant_chain. \
         NO live module reads variant_chain(\"seam_enforcer\"). \
         Seam-placer uses geometric SeamCandidate scores only."
    );

    // D-98-SEAM-NO-CONSUMER: Assert data-reaches-SliceIR fact.
    // The seam paint enters SlicedRegion.variant_chain as ("seam_enforcer", _).
    assert!(
        has_seam_variant_chain,
        "AC-7 / D-98-SEAM-NO-CONSUMER: Custom(\"seam_enforcer\") PaintLayer from \
         cube_cilindrical_modifier.3mf must reach at least one SlicedRegion.variant_chain \
         as (\"seam_enforcer\", _).\n\
         facet_value_count={facet_value_count}, stroke_count={stroke_count}.\n\
         ROUTING: painted_subsets path -> variant_chain.\n\
         CONSUMER GAP: No live downstream module reads variant_chain(\"seam_enforcer\"). \
         The seam-placer uses geometric SeamCandidate scores, not paint. \
         Registered as deviation D-98-SEAM-NO-CONSUMER."
    );
}
