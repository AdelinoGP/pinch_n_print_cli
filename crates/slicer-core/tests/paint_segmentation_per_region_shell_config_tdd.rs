//! AC-2 / AC-6 / AC-9 / AC-N2: per-`variant_chain` shell-config resolution
//! end-to-end tests on `execute_paint_segmentation`.
//!
//! Step 1 of packet 207. Drives the public `execute_paint_segmentation` with a
//! programmatically-built painted cube and asserts that the interned config's
//! `top_shell_layers` controls how many layers the painted top-face projection
//! covers. In the RED step every config variant produces identical output
//! (the driver still reads `configs[0]`), so all four tests fail.
//!
//! Host-only: `paint_segmentation` is gated behind the `host-algos` feature.

#![cfg(feature = "host-algos")]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    BoundingBox3, FacetPaintData, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, PaintLayer,
    PaintSemantic, PaintValue, Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR,
    SlicedRegion, Transform3d,
};

const LAYER_COUNT: u32 = 20;
const LAYER_HEIGHT_MM: f32 = 0.5;
const CUBE_SIZE_MM: f32 = 10.0;

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
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Point3 {
            x: 250.0,
            y: 210.0,
            z: 220.0,
        },
    }
}

fn cube_vertices(size: f32) -> Vec<Point3> {
    vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: size,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: size,
            y: size,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: size,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: size,
        },
        Point3 {
            x: size,
            y: 0.0,
            z: size,
        },
        Point3 {
            x: size,
            y: size,
            z: size,
        },
        Point3 {
            x: 0.0,
            y: size,
            z: size,
        },
    ]
}

fn cube_indices() -> Vec<u32> {
    vec![
        0, 2, 1, 0, 3, 2, // bottom
        4, 5, 6, 4, 6, 7, // top
        0, 1, 5, 0, 5, 4, // front
        2, 3, 7, 2, 7, 6, // back
        0, 4, 7, 0, 7, 3, // left
        1, 2, 6, 1, 6, 5, // right
    ]
}

struct ObjectSpec {
    id: String,
    /// `(top-face triangle index, tool index)` pairs to paint.
    top_paint: Vec<(usize, u32)>,
}

fn build_mesh(objects: Vec<ObjectSpec>) -> MeshIR {
    let object_meshes = objects
        .into_iter()
        .map(|spec| {
            let mut facet_values: Vec<Option<PaintValue>> = (0..12).map(|_| None).collect();
            for (tri, tool) in spec.top_paint {
                facet_values[tri] = Some(PaintValue::ToolIndex(tool));
            }
            ObjectMesh {
                id: spec.id,
                mesh: IndexedTriangleSet {
                    vertices: cube_vertices(CUBE_SIZE_MM),
                    indices: cube_indices(),
                },
                transform: identity_transform(),
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
                ..Default::default()
            }
        })
        .collect();
    MeshIR {
        objects: object_meshes,
        build_volume: default_build_volume(),
        ..Default::default()
    }
}

fn slice_mesh(mesh: &MeshIR) -> Vec<SliceIR> {
    let zs: Vec<f32> = (0..LAYER_COUNT)
        .map(|i| LAYER_HEIGHT_MM * (i as f32 + 0.5))
        .collect();
    let mut layers: Vec<SliceIR> = zs
        .iter()
        .enumerate()
        .map(|(idx, &z)| SliceIR {
            global_layer_index: idx as u32,
            z,
            regions: Vec::new(),
            ..Default::default()
        })
        .collect();
    for obj in &mesh.objects {
        let slabs = slice_mesh_ex(&obj.mesh, &zs);
        for (idx, polys) in slabs.iter().enumerate() {
            layers[idx].regions.push(SlicedRegion {
                object_id: obj.id.clone(),
                region_id: 0,
                polygons: polys.clone(),
                infill_areas: polys.clone(),
                effective_layer_height: LAYER_HEIGHT_MM,
                segment_annotations: HashMap::new(),
                ..Default::default()
            });
        }
    }
    layers
}

fn material_chain(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".to_string(), PaintValue::ToolIndex(tool))]
}

/// Build a region map with BASE entries per object and painted-chain entries
/// per `(object_id, tool, top_shell_layers, bottom_shell_layers)` spec.
fn build_region_map(specs: &[(&str, u32, u32, u32)]) -> Arc<RegionMapIR> {
    let mut region_map = RegionMapIR::default();
    let mut object_ids: Vec<&str> = specs.iter().map(|s| s.0).collect();
    object_ids.sort_unstable();
    object_ids.dedup();
    for object_id in object_ids {
        for i in 0..LAYER_COUNT {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: object_id.to_string(),
                    region_id: 0,
                    variant_chain: vec![],
                },
                RegionPlan::default(),
            );
        }
    }
    for (object_id, tool, top, bottom) in specs {
        let cfg_id = region_map.intern_config(ResolvedConfig {
            top_shell_layers: *top,
            bottom_shell_layers: *bottom,
            ..ResolvedConfig::default()
        });
        for i in 0..LAYER_COUNT {
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: object_id.to_string(),
                    region_id: 0,
                    variant_chain: material_chain(*tool),
                },
                RegionPlan {
                    config: cfg_id,
                    ..RegionPlan::default()
                },
            );
        }
    }
    Arc::new(region_map)
}

fn count_painted_layers(result: &[SliceIR], chain: &[(String, PaintValue)]) -> usize {
    result
        .iter()
        .filter(|s| {
            s.regions
                .iter()
                .any(|r| r.variant_chain.as_slice() == chain && !r.polygons.is_empty())
        })
        .count()
}

fn count_painted_layers_for_object(
    result: &[SliceIR],
    object_id: &str,
    chain: &[(String, PaintValue)],
) -> usize {
    result
        .iter()
        .filter(|s| {
            s.regions.iter().any(|r| {
                r.object_id == object_id
                    && r.variant_chain.as_slice() == chain
                    && !r.polygons.is_empty()
            })
        })
        .count()
}

/// AC-2: one painted object sliced twice, differing only in the interned
/// config's `top_shell_layers` (3 vs 7) → the 7-run's painted top-face chain
/// appears on strictly more layers.
#[test]
fn top_shell_layers_changes_projection_depth() {
    let mesh = Arc::new(build_mesh(vec![ObjectSpec {
        id: "obj-a".to_string(),
        top_paint: vec![(2, 1), (3, 1)],
    }]));
    let slice = Arc::new(slice_mesh(&mesh));
    let chain = material_chain(1);

    let rmap_3 = build_region_map(&[("obj-a", 1, 3, 3)]);
    let rmap_7 = build_region_map(&[("obj-a", 1, 7, 3)]);

    let result_3 = execute_paint_segmentation(mesh.clone(), slice.clone(), rmap_3).unwrap();
    let result_7 = execute_paint_segmentation(mesh.clone(), slice.clone(), rmap_7).unwrap();

    let count_3 = count_painted_layers(&result_3, &chain);
    let count_7 = count_painted_layers(&result_7, &chain);

    assert!(
        count_7 > count_3,
        "top=7 ({count_7}) must cover strictly more layers than top=3 ({count_3})"
    );
}

/// AC-6: two objects, same painted chain, obj-a top=2, obj-b top=8 → obj-b's
/// painted projection covers strictly more layers than obj-a's.
#[test]
fn multi_object_shell_counts_are_independent() {
    let mesh = Arc::new(build_mesh(vec![
        ObjectSpec {
            id: "obj-a".to_string(),
            top_paint: vec![(2, 1), (3, 1)],
        },
        ObjectSpec {
            id: "obj-b".to_string(),
            top_paint: vec![(2, 1), (3, 1)],
        },
    ]));
    let slice = Arc::new(slice_mesh(&mesh));
    let chain = material_chain(1);

    let rmap = build_region_map(&[("obj-a", 1, 2, 3), ("obj-b", 1, 8, 3)]);

    let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();

    let count_a = count_painted_layers_for_object(&result, "obj-a", &chain);
    let count_b = count_painted_layers_for_object(&result, "obj-b", &chain);

    assert!(
        count_b > count_a,
        "obj-b top=8 ({count_b}) must cover strictly more layers than obj-a top=2 ({count_a})"
    );
}

/// AC-9: single object, two painted chains ToolIndex(1) top=2 and
/// ToolIndex(2) top=8 → ToolIndex(2)'s projection covers strictly more layers.
#[test]
fn per_variant_chain_shell_counts_are_independent() {
    let mesh = Arc::new(build_mesh(vec![ObjectSpec {
        id: "obj-a".to_string(),
        top_paint: vec![(2, 1), (3, 2)],
    }]));
    let slice = Arc::new(slice_mesh(&mesh));
    let chain_1 = material_chain(1);
    let chain_2 = material_chain(2);

    let rmap = build_region_map(&[("obj-a", 1, 2, 3), ("obj-a", 2, 8, 3)]);

    let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();

    let count_1 = count_painted_layers(&result, &chain_1);
    let count_2 = count_painted_layers(&result, &chain_2);

    assert!(
        count_2 > count_1,
        "ToolIndex(2) top=8 ({count_2}) must cover strictly more layers than ToolIndex(1) top=2 ({count_1})"
    );
}

/// AC-N2: config with top_shell_layers=0 and bottom_shell_layers=0 → the
/// painted projection still appears on the contact layer (`.max(1)` floor) and
/// the run does not error.
#[test]
fn zero_shell_counts_keep_contact_layer() {
    let mesh = Arc::new(build_mesh(vec![ObjectSpec {
        id: "obj-a".to_string(),
        top_paint: vec![(2, 1), (3, 1)],
    }]));
    let slice = Arc::new(slice_mesh(&mesh));
    let chain = material_chain(1);

    let rmap = build_region_map(&[("obj-a", 1, 0, 0)]);

    let result =
        execute_paint_segmentation(mesh, slice, rmap).expect("zero shell counts must not error");

    let count = count_painted_layers(&result, &chain);
    assert_eq!(
        count, 1,
        "top=0/bottom=0 must floor to exactly the contact layer (.max(1)), got {count}"
    );
}
