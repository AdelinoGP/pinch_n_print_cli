//! Packet 56b â€” Modifier-part IR routing E2E tests against cube 3MF fixtures.
//!
//! Fixtures: cube_cilindrical_modifier.3mf (modifier-part tests) and
//! cube_4color.3mf (no-modifier negative-invariant test).

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use slicer_ir::{ActiveRegion, ConfigValue, GlobalLayer, LayerPlanIR, ResolvedConfig, SemVer};
use slicer_runtime::{execute_region_mapping, ExecutionPlan};

use crate::common::model_cache::cached_load_model;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn cube_cilindrical_modifier_3mf() -> PathBuf {
    repo_root().join("resources/cube_cilindrical_modifier.3mf")
}

fn cube_4color_3mf() -> PathBuf {
    repo_root().join("resources/cube_4color.3mf")
}

/// Build a minimal single-region `LayerPlanIR` at the given Z height (mm).
///
/// This is used by Step 4 RED tests to drive `execute_region_mapping`
/// with a synthetic layer plan so we can inspect whether modifier-volume
/// config deltas are stamped into `RegionPlan.config.extensions`.
fn single_region_layer_plan(layer_index: u32, z_mm: f32) -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: layer_index,
            z: z_mm,
            active_regions: vec![ActiveRegion {
                object_id: "obj_0".to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: Default::default(),
        ..Default::default()
    }
}

/// Build a minimal `PaintRegionIR` with a tiny FuzzySkin semantic region for the
/// given layer index. The region polygon is too small to cover any contour point,
/// so all points get the default `Flag(false)`. Modifier projections then overlay
// fuzzy_paint_regions removed: PaintRegionIR/SemanticRegion deleted in packet 95

// ---------------------------------------------------------------------------
// REQ-MODIFIER-001 / REQ-MODIFIER-002
// Modifier geometry must be excluded from the solid mesh and appear in
// modifier_volumes instead.
// ---------------------------------------------------------------------------

#[test]
fn modifier_part_excluded_from_solid_mesh() {
    let path = cube_cilindrical_modifier_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir = cached_load_model(&path);

    // Packet 89: cube_cilindrical_modifier.3mf is a 12-triangle axis-aligned cube
    // (the "Cube" part) plus a cylindrical modifier_part volume. The solid mesh
    // must contain only the cube's 12 triangles — the modifier-part cylinder is
    // routed to `modifier_volumes` and excluded from the solid mesh.
    let solid_obj = &mesh_ir.objects[0];
    let tri_count = solid_obj.mesh.indices.len() / 3;
    assert_eq!(
        tri_count, 12,
        "solid mesh has {tri_count} triangles, expected 12 (cube hull)"
    );

    assert_eq!(
        mesh_ir.schema_version,
        SemVer {
            major: 1,
            minor: 1,
            patch: 0
        },
        "expected schema_version 1.1.0, got {:?}",
        mesh_ir.schema_version
    );
}

// ---------------------------------------------------------------------------
// REQ-MODIFIER-003
// ModifierVolume must carry typed metadata from the sidecar.
// ---------------------------------------------------------------------------

#[test]
fn modifier_volume_carries_typed_metadata() {
    let path = cube_cilindrical_modifier_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir = cached_load_model(&path);

    let solid_obj = &mesh_ir.objects[0];

    assert_eq!(
        solid_obj.modifier_volumes.len(),
        1,
        "expected 1 modifier volume"
    );

    let mv = &solid_obj.modifier_volumes[0];

    // Packet 89: cube_cilindrical_modifier.3mf authors the modifier cylinder with
    // four typed overrides (inner_wall_line_width, outer_wall_line_width,
    // sparse_infill_density, sparse_infill_line_width — verified via
    // `unzip -p ... Metadata/model_settings.config`). However, the current 3MF
    // loader (`crates/slicer-model-io/src/loader.rs`) only extracts an
    // allowlisted set of part-metadata keys into `config_delta.fields`:
    // `subtype`, `fuzzy_skin`, `extruder`, and `matrix`. Extending the loader's
    // allowlist to include the four wall/infill keys is out of scope for this
    // packet (no source edits permitted). The strengthened typed-metadata
    // assertion will be added once the loader is extended; until then, this
    // test verifies the keys the loader DOES preserve.
    let subtype = mv.config_delta.fields.get("subtype");
    assert_eq!(
        subtype,
        Some(&ConfigValue::String("modifier_part".to_string())),
        "config_delta[subtype] = {:?}",
        subtype
    );

    // The fixture stores the local offset for the cylinder in the `matrix` metadata
    // key; the loader preserves it verbatim. P98 fixture edit repositioned the cylinder
    // (offset: ~17.98, ~16.49, 0). Assert the actual translation components are present
    // so we verify the loader preserved the full transform matrix, not just a non-empty string.
    let matrix = mv.config_delta.fields.get("matrix");
    let matrix_str = match matrix {
        Some(ConfigValue::String(s)) => s.as_str(),
        _ => panic!(
            "config_delta[matrix] = {:?} — expected a String value",
            matrix
        ),
    };
    assert!(
        matrix_str.contains("17.98"),
        "matrix must contain X-translation '17.98', got: {:?}",
        matrix_str
    );
    assert!(
        matrix_str.contains("16.49"),
        "matrix must contain Y-translation '16.49', got: {:?}",
        matrix_str
    );
}

// ---------------------------------------------------------------------------
// REQ-MODIFIER-004
// Modifier world-space AABB centroid must be in the positive octant.
// ---------------------------------------------------------------------------

#[test]
fn modifier_world_aabb_matches_composition() {
    let path = cube_cilindrical_modifier_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir = cached_load_model(&path);

    let solid_obj = &mesh_ir.objects[0];

    assert!(
        !solid_obj.modifier_volumes.is_empty(),
        "modifier_volumes is empty"
    );

    let mv = &solid_obj.modifier_volumes[0];
    let verts = &mv.mesh.vertices;
    assert!(!verts.is_empty(), "modifier mesh has no vertices");

    let n = verts.len() as f32;
    let cx: f32 = verts.iter().map(|v| v.x).sum::<f32>() / n;
    let cy: f32 = verts.iter().map(|v| v.y).sum::<f32>() / n;
    let cz: f32 = verts.iter().map(|v| v.z).sum::<f32>() / n;

    // cube_cilindrical_modifier.3mf: the loaded cylinder modifier MESH centroid
    // sits at world ~(133.99, 113.25, 12.5). Values measured empirically from the
    // loaded mesh vertices (cargo run), not derived from the matrix metadata.
    // NOTE (P98): after the fixture's mid-packet edit, the modifier `matrix`
    // metadata string reports a ~(17.98, 16.49) offset, but the actual mesh
    // geometry asserted here is unchanged (offset ~8.99) — i.e. metadata and
    // geometry disagree. Flagged for fixture review; out of P98's scope (loader
    // paint symmetry). Both this geometry check and the matrix-metadata check pass.
    const EXPECTED_CX: f32 = 133.99;
    const EXPECTED_CY: f32 = 113.25;
    const EXPECTED_CZ: f32 = 12.5;
    // Loose tolerance: the cylinder mesh vertex centroid can deviate from the
    // analytical axis center because vertices are not uniformly distributed.
    const TOL: f32 = 2.0;

    assert!(
        (cx - EXPECTED_CX).abs() < TOL,
        "centroid X mismatch: got {cx}, expected {EXPECTED_CX}"
    );
    assert!(
        (cy - EXPECTED_CY).abs() < TOL,
        "centroid Y mismatch: got {cy}, expected {EXPECTED_CY}"
    );
    assert!(
        (cz - EXPECTED_CZ).abs() < TOL,
        "centroid Z mismatch: got {cz}, expected {EXPECTED_CZ}"
    );
}

/// D14 contract (replaces the v1 `execute_slice_postprocess_paint_annotation`
/// surface deleted in packet 95): a modifier volume with FuzzySkin (or any
/// supported semantic) flows into `SlicedRegion.segment_annotations[<semantic>]`
/// on the BASE variant chain only.  This mirrors the cube_fuzzy modifier-overlay
/// test in `cube_fuzzy_painted_tdd` but exercises the FuzzySkin semantic path
/// against a synthetic mesh + a SupportEnforcer modifier volume.
#[test]
fn modifier_projections_annotate_contour_points() {
    use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
    use slicer_ir::{
        ConfigDelta, ConfigValue, MeshIR, ModifierScope, ModifierVolume, ObjectConfig, ObjectMesh,
        PaintSemantic, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer, SliceIR, SlicedRegion,
        CURRENT_SLICE_IR_SCHEMA_VERSION,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    let object_id = "obj1";
    let mv_mesh = box_mesh_xyz(4.0, 4.0, 4.0); // modifier wider than the test polygon
    let mut mv_fields = HashMap::new();
    mv_fields.insert(
        "subtype".to_string(),
        ConfigValue::String("support_enforcer".to_string()),
    );
    let mv = ModifierVolume {
        id: "mv-1".to_string(),
        mesh: mv_mesh,
        config_delta: ConfigDelta { fields: mv_fields },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    };
    let mesh = Arc::new(MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: box_mesh_xyz(5.0, 5.0, 5.0),
            transform: identity_transform_3d(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![mv],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 {
                x: -10.0,
                y: -10.0,
                z: -10.0,
            },
            max: Point3 {
                x: 10.0,
                y: 10.0,
                z: 10.0,
            },
        },
    });

    let parent_polygon = test_square_polygon(4.0);
    let zs: Vec<f32> = (0..5).map(|i| 0.5 + 0.5 * i as f32).collect();
    let slice_ir = Arc::new(
        zs.iter()
            .enumerate()
            .map(|(idx, &z)| SliceIR {
                schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: vec![parent_polygon.clone()],
                    ..Default::default()
                }],
            })
            .collect(),
    );

    let mut entries = HashMap::new();
    for (idx, _z) in zs.iter().enumerate() {
        entries.insert(
            RegionKey {
                global_layer_index: idx as u32,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    let region_map = Arc::new(RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries,
        configs: vec![slicer_ir::ResolvedConfig::default()],
    });

    let result = execute_paint_segmentation(mesh, slice_ir, region_map).expect("v2 driver ok");

    let mut layers_annotated = 0;
    for slice in result.iter() {
        for region in &slice.regions {
            if !region.variant_chain.is_empty() {
                continue; // D14: enforcer rides BASE chain only.
            }
            if region
                .segment_annotations
                .get(&PaintSemantic::SupportEnforcer)
                .is_some_and(|perim| perim.iter().any(|p| p.iter().any(|v| v.is_some())))
            {
                layers_annotated += 1;
            }
        }
    }
    assert!(
        layers_annotated > 0,
        "D14: modifier projection must populate segment_annotations[SupportEnforcer] \
         on BASE-chain SlicedRegion(s); got 0 annotated layers"
    );
}

/// D14 Z-band restriction: a modifier volume with finite Z extent must only
/// annotate layers whose Z falls inside that extent.  Layers above/below the
/// modifier produce no annotations for that semantic.
#[test]
fn modifier_projection_z_band_restriction() {
    use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
    use slicer_ir::{
        ConfigDelta, ConfigValue, MeshIR, ModifierScope, ModifierVolume, ObjectConfig, ObjectMesh,
        PaintSemantic, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer, SliceIR, SlicedRegion,
        CURRENT_SLICE_IR_SCHEMA_VERSION,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    let object_id = "obj1";
    // Modifier confined to z ∈ [-1, 1].
    let mv_mesh = box_mesh_z_band(4.0, 4.0, -1.0, 1.0);
    let mut mv_fields = HashMap::new();
    mv_fields.insert(
        "subtype".to_string(),
        ConfigValue::String("support_blocker".to_string()),
    );
    let mv = ModifierVolume {
        id: "mv-band".to_string(),
        mesh: mv_mesh,
        config_delta: ConfigDelta { fields: mv_fields },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    };
    let mesh = Arc::new(MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: box_mesh_xyz(5.0, 5.0, 5.0),
            transform: identity_transform_3d(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![mv],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 {
                x: -10.0,
                y: -10.0,
                z: -10.0,
            },
            max: Point3 {
                x: 10.0,
                y: 10.0,
                z: 10.0,
            },
        },
    });

    let parent_polygon = test_square_polygon(4.0);
    // Sample z's that span ABOVE and BELOW the modifier's z ∈ [-1, 1] band.
    let zs: Vec<f32> = vec![-3.0, -2.0, -0.5, 0.0, 0.5, 2.0, 3.0];
    let slice_ir = Arc::new(
        zs.iter()
            .enumerate()
            .map(|(idx, &z)| SliceIR {
                schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: vec![parent_polygon.clone()],
                    ..Default::default()
                }],
            })
            .collect(),
    );

    let mut entries = HashMap::new();
    for (idx, _z) in zs.iter().enumerate() {
        entries.insert(
            RegionKey {
                global_layer_index: idx as u32,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    let region_map = Arc::new(RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries,
        configs: vec![slicer_ir::ResolvedConfig::default()],
    });

    let result = execute_paint_segmentation(mesh, slice_ir, region_map).expect("v2 driver ok");

    for slice in result.iter() {
        let z = slice.z;
        let in_band = (-1.0..=1.0).contains(&z);
        let has_blocker = slice.regions.iter().any(|r| {
            r.variant_chain.is_empty()
                && r.segment_annotations
                    .get(&PaintSemantic::SupportBlocker)
                    .is_some_and(|perim| perim.iter().any(|p| p.iter().any(|v| v.is_some())))
        });
        if !in_band {
            assert!(
                !has_blocker,
                "Z-band restriction violation: layer at z={z} (outside modifier's [-1, 1] band) \
                 must NOT carry segment_annotations[SupportBlocker]"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers for the modifier-projection rewrites above
// ---------------------------------------------------------------------------

fn identity_transform_3d() -> slicer_ir::Transform3d {
    slicer_ir::Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn box_mesh_xyz(hx: f32, hy: f32, hz: f32) -> slicer_ir::IndexedTriangleSet {
    use slicer_ir::{IndexedTriangleSet, Point3};
    let v = vec![
        Point3 {
            x: -hx,
            y: -hy,
            z: -hz,
        },
        Point3 {
            x: hx,
            y: -hy,
            z: -hz,
        },
        Point3 {
            x: hx,
            y: hy,
            z: -hz,
        },
        Point3 {
            x: -hx,
            y: hy,
            z: -hz,
        },
        Point3 {
            x: -hx,
            y: -hy,
            z: hz,
        },
        Point3 {
            x: hx,
            y: -hy,
            z: hz,
        },
        Point3 {
            x: hx,
            y: hy,
            z: hz,
        },
        Point3 {
            x: -hx,
            y: hy,
            z: hz,
        },
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7, 3,
        1, 2, 6, 1, 6, 5,
    ];
    IndexedTriangleSet {
        vertices: v,
        indices,
    }
}

fn box_mesh_z_band(hx: f32, hy: f32, z_min: f32, z_max: f32) -> slicer_ir::IndexedTriangleSet {
    use slicer_ir::{IndexedTriangleSet, Point3};
    let v = vec![
        Point3 {
            x: -hx,
            y: -hy,
            z: z_min,
        },
        Point3 {
            x: hx,
            y: -hy,
            z: z_min,
        },
        Point3 {
            x: hx,
            y: hy,
            z: z_min,
        },
        Point3 {
            x: -hx,
            y: hy,
            z: z_min,
        },
        Point3 {
            x: -hx,
            y: -hy,
            z: z_max,
        },
        Point3 {
            x: hx,
            y: -hy,
            z: z_max,
        },
        Point3 {
            x: hx,
            y: hy,
            z: z_max,
        },
        Point3 {
            x: -hx,
            y: hy,
            z: z_max,
        },
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7, 3,
        1, 2, 6, 1, 6, 5,
    ];
    IndexedTriangleSet {
        vertices: v,
        indices,
    }
}

fn test_square_polygon(side_mm: f32) -> slicer_ir::ExPolygon {
    let h = side_mm / 2.0;
    slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2::from_mm(-h, -h),
                slicer_ir::Point2::from_mm(h, -h),
                slicer_ir::Point2::from_mm(h, h),
                slicer_ir::Point2::from_mm(-h, h),
            ],
        },
        holes: vec![],
    }
}

/// Negative-invariant: when a model has no modifier volumes (`cube_4color.3mf`
/// is paint-only — no modifier_part subtype is declared), `execute_region_mapping`
/// must not stamp any modifier-derived keys into `RegionPlan.config.extensions`.
#[test]
fn empty_modifier_volume_stamps_no_regions() {
    // Packet 89: cube_4color.3mf is the no-modifier comparator. It carries
    // 4-color paint strokes only — no `subtype="modifier_part"` parts → no
    // modifier volumes are parsed → modifier_volumes is empty.
    let path = cube_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());

    let mesh_ir = cached_load_model(&path);

    // Confirm: no modifier volumes on any object.
    let total_modifier_volumes: usize = mesh_ir
        .objects
        .iter()
        .map(|o| o.modifier_volumes.len())
        .sum();
    assert_eq!(
        total_modifier_volumes, 0,
        "cube_4color.3mf must have 0 modifier volumes (paint-only), \
         got {total_modifier_volumes}"
    );

    // Run region mapping on a minimal plan covering Z = 10.0 mm.
    let layer_plan = single_region_layer_plan(0, 10.0);
    let plan = ExecutionPlan::default();
    let si: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = plan
        .per_layer_stages
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
        .collect();
    let projection = slicer_core::algos::region_mapping::RegionMappingPlanProjection {
        stage_invocations: &si,
    };
    let paint_semantic_configs = BTreeMap::new();

    let region_map = execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &BTreeMap::new(),
        &[],
    )
    .expect("execute_region_mapping must succeed with empty modifier volumes");

    // No modifier volumes â†’ no modifier-derived keys in any region.
    let stamped_count = region_map
        .entries
        .keys()
        .filter(|key| {
            region_map
                .config_for(key)
                .extensions
                .contains_key("fuzzy_skin.apply_to_all")
        })
        .count();

    assert_eq!(
        stamped_count, 0,
        "empty modifier_volumes must produce 0 stamped regions, \
         got stamped_count={stamped_count}"
    );
}

// ---------------------------------------------------------------------------
// Full pipeline paint diagnostic: prepass extraction â†’ per-layer annotation
// ---------------------------------------------------------------------------

/// Full pipeline paint-region diagnostic for `cube_4color.3mf`.
///
/// Runs the host-side pipeline (paint extraction + paint annotation) end-to-end
/// on the 4-color painted cube and reports exactly at which stage Material
/// ToolIndex values are lost.
///
/// Failure mode 1: `STAGE=prepass_paint_extraction` â€” `execute_paint_segmentation`
///     produced fewer than 4 distinct Material ToolIndex values. The 3MF paint
///     strokes exist, but the paint-extraction IR pipeline dropped them.
/// Failure mode 2: `STAGE=prepass_material_semantic` â€” the extracted
///     `PaintRegionIR` has no `PaintSemantic::Material` key in any layer. The
///     paint data reached the IR but with a wrong semantic label.
/// Failure mode 3: `STAGE=per_layer_paint_annotation` â€” `execute_slice_postprocess_
///     paint_annotation` produced zero contour points with `Material/ToolIndex`
///     segment_annotations. The PaintRegionIR has correct semantics, but the annotation
///     step did not project them onto SlicedRegion contour points. This could be
///     a bounding-box mismatch, polygon emptiness, or semantic routing bug.
/// Full pipeline paint-region diagnostic for `cube_4color.3mf`.
///
/// Runs the v2 driver on the loaded cube_4color mesh and asserts that the
/// final SliceIR carries ≥ 4 distinct Material ToolIndex values across all
/// variant_chain entries.  Pinpoints the failure stage when this expectation
/// breaks:
/// - STAGE=v2_driver — `execute_paint_segmentation` errored or returned the
///   short-circuit path on a painted mesh.
/// - STAGE=variant_chain_propagation — Material strokes were detected but
///   the variant_chain composition dropped distinct TI values.
#[test]
fn cube_4color_full_pipeline_paint_diagnostic() {
    use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
    use slicer_ir::{
        PaintValue, RegionKey, RegionMapIR, RegionPlan, SemVer, SliceIR, SlicedRegion,
        CURRENT_SLICE_IR_SCHEMA_VERSION,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    let path = cube_4color_3mf();
    if !path.exists() {
        eprintln!("SKIP: cube_4color.3mf fixture missing");
        return;
    }

    let mesh_ir = cached_load_model(&path);
    let object_id = mesh_ir
        .objects
        .first()
        .expect("cube must have object")
        .id
        .clone();

    // Slice the cube at multiple mid-Z levels.
    let zs: Vec<f32> = (1..20).map(|i| i as f32 * 1.0).collect();
    let parent_obj = mesh_ir.objects.first().unwrap();
    let layer_polys = slicer_core::slice_mesh_ex(&parent_obj.mesh, &zs);
    let slice_ir = Arc::new(
        zs.iter()
            .enumerate()
            .map(|(idx, &z)| SliceIR {
                schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.clone(),
                    region_id: 0,
                    polygons: layer_polys.get(idx).cloned().unwrap_or_default(),
                    ..Default::default()
                }],
            })
            .collect(),
    );

    let mut entries = HashMap::new();
    for (idx, _z) in zs.iter().enumerate() {
        entries.insert(
            RegionKey {
                global_layer_index: idx as u32,
                object_id: object_id.clone(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    let region_map = Arc::new(RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries,
        configs: vec![slicer_ir::ResolvedConfig::default()],
    });

    let result = execute_paint_segmentation(mesh_ir, slice_ir, region_map)
        .expect("STAGE=v2_driver — execute_paint_segmentation must succeed on cube_4color");

    let mut distinct_tool_indices = std::collections::HashSet::new();
    for slice in result.iter() {
        for region in &slice.regions {
            for (sem, value) in &region.variant_chain {
                if sem == "material" {
                    if let PaintValue::ToolIndex(t) = value {
                        distinct_tool_indices.insert(*t);
                    }
                }
            }
        }
    }

    assert!(
        distinct_tool_indices.len() >= 4,
        "STAGE=variant_chain_propagation — cube_4color must produce ≥4 distinct Material \
         ToolIndex values across all variant_chain entries; got {} ({:?})",
        distinct_tool_indices.len(),
        distinct_tool_indices
    );
}
