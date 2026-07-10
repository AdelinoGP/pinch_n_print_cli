//! `PrePass::OverhangAnnotation` host built-in wiring and stage-order
//! enforcement, updated for the overhang-after-Slice inversion.
//!
//! Overhang classification now derives each object's per-layer footprints from
//! the committed `SliceIR` and diffs consecutive layers — OrcaSlicer's
//! `detect_overhangs_for_lift` (`PrintObject.cpp:880-908`) shape — instead of
//! re-slicing the mesh in a dedicated pass. The stage therefore runs strictly
//! AFTER `PrePass::Slice`. These tests drive the real host builtins
//! (`commit_slice_builtin` then `commit_overhang_annotation_builtin`) over a
//! seeded blackboard and assert:
//!
//! - the stage refuses to run without a committed `SliceIR` (its dependency on,
//!   and ordering after, `PrePass::Slice`);
//! - `STAGE_ORDER` places it after `PrePass::Slice`;
//! - it populates `overhang_quartile_polygons` from the slices, including for a
//!   non-identity object transform (which `PrePass::Slice` bakes into its
//!   world-space cross-sections) and for a multi-object mesh (bands merged by
//!   quartile).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigKey, ConfigValue, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, Point3, ResolvedConfig, Transform3d,
};
use slicer_runtime::{
    commit_overhang_annotation_builtin, commit_slice_builtin, execute_mesh_analysis, Blackboard,
    OverhangAnnotationBuiltinError, STAGE_ORDER,
};

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Column-major 4x4 identity translated by `(0, 0, z_mm)`.
fn z_translation_transform(z_mm: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, z_mm, 1.0,
        ],
    }
}

/// Axis-aligned box triangle soup (12 triangles), same winding convention as
/// `slicer_core::algos::overhang_annotation`'s own `flat_cube_mesh` fixture:
/// bottom CW-from-above, top CCW-from-above.
fn box_triangles(
    base_index: u32,
    (x0, y0, z0): (f32, f32, f32),
    (x1, y1, z1): (f32, f32, f32),
) -> (Vec<Point3>, Vec<u32>) {
    let vertices = vec![
        Point3 {
            x: x0,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z1,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z1,
        },
    ];
    let b = base_index;
    #[rustfmt::skip]
    let indices = vec![
        b, b + 1, b + 2,   b, b + 2, b + 3,
        b + 4, b + 5, b + 6,   b + 4, b + 6, b + 7,
        b, b + 1, b + 5,   b, b + 5, b + 4,
        b + 1, b + 2, b + 6,   b + 1, b + 6, b + 5,
        b + 2, b + 3, b + 7,   b + 2, b + 7, b + 6,
        b + 3, b, b + 4,   b + 3, b + 4, b + 7,
    ];
    (vertices, indices)
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: Point3::default(),
        max: Point3 {
            x: 200.0,
            y: 200.0,
            z: 200.0,
        },
    }
}

/// Two 10x10x1mm boxes stacked in Z: the lower box spans x:[0,10], the upper
/// box spans x:[5,15] — laterally offset by 5mm so the upper layer's footprint
/// (at z=1.5) is NOT fully supported by the lower layer's footprint (at z=0.5),
/// producing a real overhang strip x:[10,15]. `transform` is applied at the
/// `ObjectMesh` level.
fn overhang_ramp_mesh_with_transform(transform: Transform3d) -> MeshIR {
    let (mut vertices, mut indices) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(vertices.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    vertices.extend(v2);
    indices.extend(i2);

    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("ramp"),
            mesh: IndexedTriangleSet { vertices, indices },
            transform,
            ..Default::default()
        }],
        build_volume: build_volume(),
        ..Default::default()
    }
}

fn overhang_ramp_mesh() -> MeshIR {
    overhang_ramp_mesh_with_transform(identity_transform())
}

/// Same ramp geometry as [`overhang_ramp_mesh`], duplicated as TWO separate
/// `ObjectMesh` entries: object "ramp-a" at y:[0,10] and object "ramp-b" at
/// y:[20,30] (identity transforms). Both objects produce overhang at global
/// layer index 1.
fn two_object_overhang_mesh() -> MeshIR {
    let (mut va, mut ia) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(va.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    va.extend(v2);
    ia.extend(i2);

    let (mut vb, mut ib) = box_triangles(0, (0.0, 20.0, 0.0), (10.0, 30.0, 1.0));
    let (v4, i4) = box_triangles(vb.len() as u32, (5.0, 20.0, 1.0), (15.0, 30.0, 2.0));
    vb.extend(v4);
    ib.extend(i4);

    MeshIR {
        objects: vec![
            ObjectMesh {
                id: String::from("ramp-a"),
                mesh: IndexedTriangleSet {
                    vertices: va,
                    indices: ia,
                },
                transform: identity_transform(),
                ..Default::default()
            },
            ObjectMesh {
                id: String::from("ramp-b"),
                mesh: IndexedTriangleSet {
                    vertices: vb,
                    indices: ib,
                },
                transform: identity_transform(),
                ..Default::default()
            },
        ],
        build_volume: build_volume(),
        ..Default::default()
    }
}

fn active_region(object_id: &str) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 1.0,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn global_layer(index: u32, z: f32, object_ids: &[&str]) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: object_ids.iter().map(|id| active_region(id)).collect(),
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn empty_raw_config() -> HashMap<ConfigKey, ConfigValue> {
    HashMap::new()
}

/// Commit mesh analysis + a layer plan, run `PrePass::Slice`, and return the
/// blackboard with `SliceIR` committed — the prerequisite state
/// `PrePass::OverhangAnnotation` now consumes. `commit_slice_builtin` needs no
/// `RegionMapIR` (it falls back to per-region defaults when none is present).
fn seed_and_slice(mesh: MeshIR, global_layers: Vec<GlobalLayer>) -> Blackboard {
    let sc = execute_mesh_analysis(&mesh).expect("mesh analysis must succeed");
    let n_layers = global_layers.len();
    let mut bb = Blackboard::new(Arc::new(mesh), n_layers);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers,
        ..Default::default()
    }))
    .expect("commit layer plan");
    bb.commit_surface_classification(Arc::new(sc))
        .expect("commit surface classification");
    commit_slice_builtin(&mut bb).expect("PrePass::Slice must succeed");
    bb
}

/// Ordering guard: `commit_overhang_annotation_builtin` must refuse to run
/// before `PrePass::Slice` has committed `SliceIR`, since it now derives
/// overhang from the slices.
#[test]
fn overhang_annotation_requires_committed_slice_ir() {
    let mesh = overhang_ramp_mesh();
    let sc = execute_mesh_analysis(&mesh).expect("mesh analysis must succeed");
    let mut bb = Blackboard::new(Arc::new(mesh), 2);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers: vec![
            global_layer(0, 0.5, &["ramp"]),
            global_layer(1, 1.5, &["ramp"]),
        ],
        ..Default::default()
    }))
    .expect("commit layer plan");
    bb.commit_surface_classification(Arc::new(sc))
        .expect("commit surface classification");

    let err = commit_overhang_annotation_builtin(&mut bb, &empty_raw_config())
        .expect_err("overhang must fail without committed SliceIR");
    assert!(
        matches!(err, OverhangAnnotationBuiltinError::MissingSliceIr),
        "expected MissingSliceIr (proving the stage runs after PrePass::Slice), got {err:?}"
    );
}

/// `STAGE_ORDER` — the canonical live-host stage list — must place
/// `PrePass::OverhangAnnotation` after `PrePass::Slice`.
#[test]
fn stage_order_places_overhang_annotation_after_slice() {
    let pos = |s: &str| {
        STAGE_ORDER
            .iter()
            .position(|x| *x == s)
            .unwrap_or_else(|| panic!("{s} must be present in STAGE_ORDER"))
    };
    assert!(
        pos("PrePass::OverhangAnnotation") > pos("PrePass::Slice"),
        "PrePass::OverhangAnnotation must come after PrePass::Slice in STAGE_ORDER"
    );
}

/// Positive: overhang bands are populated from the committed slices for the
/// layer that overhangs its predecessor.
#[test]
fn overhang_annotation_populates_bands_from_committed_slices() {
    let mut bb = seed_and_slice(
        overhang_ramp_mesh(),
        vec![
            global_layer(0, 0.5, &["ramp"]),
            global_layer(1, 1.5, &["ramp"]),
        ],
    );
    commit_overhang_annotation_builtin(&mut bb, &empty_raw_config())
        .expect("overhang annotation must succeed");

    let sc = bb
        .surface_classification()
        .expect("SurfaceClassificationIR committed");
    // Layer 0 has no predecessor and is never overhanging; global layer 1 is.
    assert!(
        !sc.overhang_quartile_polygons.contains_key(&0u32),
        "layer 0 has no previous layer and must carry no overhang bands"
    );
    assert!(
        sc.overhang_quartile_polygons.contains_key(&1u32),
        "expected overhang bands at global layer index 1, got keys {:?}",
        sc.overhang_quartile_polygons.keys().collect::<Vec<_>>()
    );
}

/// The object transform must reach the overhang bands *through* `PrePass::Slice`:
/// the mesh is authored in LOCAL space z:[0,2] but carries a `+5mm` Z transform
/// into GLOBAL space z:[5,7], and the layers are declared at global Z (5.5,
/// 6.5). `PrePass::Slice` bakes the transform into its world-space cross-sections
/// (`object_world_mesh`), so overhang — derived from those slices — is non-empty
/// at layer 1. If Slice sliced the untransformed local mesh at global Zs, the
/// cross-sections would miss the mesh entirely and this would be empty.
#[test]
fn overhang_annotation_reflects_slice_object_transform() {
    let z_offset_mm = 5.0_f64;
    let mut bb = seed_and_slice(
        overhang_ramp_mesh_with_transform(z_translation_transform(z_offset_mm)),
        vec![
            global_layer(0, 0.5 + z_offset_mm as f32, &["ramp"]),
            global_layer(1, 1.5 + z_offset_mm as f32, &["ramp"]),
        ],
    );
    commit_overhang_annotation_builtin(&mut bb, &empty_raw_config())
        .expect("overhang annotation must succeed");

    let sc = bb
        .surface_classification()
        .expect("SurfaceClassificationIR committed");
    assert!(
        sc.overhang_quartile_polygons.contains_key(&1u32),
        "expected overhang bands at global layer index 1 once PrePass::Slice applies the \
         +{z_offset_mm}mm object transform (empty iff Slice wrongly sliced the untransformed \
         local mesh against global-space Zs); got keys {:?}",
        sc.overhang_quartile_polygons.keys().collect::<Vec<_>>()
    );
}

/// The multi-object merge must aggregate per-object results **by quartile** — at
/// most one `QuartileBand` per quartile per layer, all objects' polygons
/// concatenated into that band — preserving design.md's locked assumption
/// ("inner Vec carries one `QuartileBand` per quartile").
#[test]
fn overhang_annotation_merges_multi_object_bands_by_quartile() {
    let mut bb = seed_and_slice(
        two_object_overhang_mesh(),
        vec![
            global_layer(0, 0.5, &["ramp-a", "ramp-b"]),
            global_layer(1, 1.5, &["ramp-a", "ramp-b"]),
        ],
    );
    commit_overhang_annotation_builtin(&mut bb, &empty_raw_config())
        .expect("overhang annotation must succeed");

    let sc = bb
        .surface_classification()
        .expect("SurfaceClassificationIR committed");
    let bands = sc
        .overhang_quartile_polygons
        .get(&1u32)
        .expect("both ramp objects overhang at global layer index 1");

    assert!(
        bands.len() <= 4,
        "at most one QuartileBand per quartile per layer regardless of object count; \
         got quartiles {:?}",
        bands.iter().map(|b| b.quartile).collect::<Vec<_>>()
    );
    let quartiles: Vec<u8> = bands.iter().map(|b| b.quartile).collect();
    assert!(
        quartiles.windows(2).all(|w| w[0] < w[1]),
        "bands must be unique and sorted by quartile, got {quartiles:?}"
    );
    // Both objects contribute identical (Y-translated) overhang geometry, so
    // every present band must carry a polygon from BOTH objects.
    for band in bands {
        assert!(
            band.polygons.len() >= 2,
            "band {} must merge polygons from both objects, got {} polygon(s)",
            band.quartile,
            band.polygons.len()
        );
    }
}
