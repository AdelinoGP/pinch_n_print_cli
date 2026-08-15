//! Seam-paint annotation writer.

use std::collections::BTreeMap;

/// XY tolerance for [`point_near_degenerate_projection`], in integer
/// coordinate units (1 unit = 100 nm, `docs/08_coordinate_system.md`), i.e.
/// 0.01 mm.
///
/// A painted triangle lying in a vertical plane projects onto XY as a sliver
/// with no interior, so `any_expolygon_contains_point` can never report
/// containment against it and every vertex on that wall would go unstamped.
/// The fallback measures distance to the sliver's edges instead. The tolerance
/// must stay far below the spacing between adjacent walls, or the fallback
/// would capture a vertex belonging to a neighbouring contour; 0.01 mm is two
/// orders of magnitude below the extrusion widths this pipeline configures.
/// Cost impact is unmeasured — the fallback only runs for degenerate
/// projections.
const SEAM_PAINT_POINT_EPS_UNITS: i64 = 100;

/// Degeneracy threshold for a projected triangle, as |2·area| in squared
/// coordinate units.
///
/// Testing `area2 == 0` exactly is a discontinuity rather than a tolerance: a
/// sliver of area 1 unit² (10⁻⁸ mm²) is degenerate for every practical
/// purpose, yet an exact test denies it both the containment path (no
/// interior to contain anything) and the fallback. Anything whose |2·area|
/// fits inside a `SEAM_PAINT_POINT_EPS_UNITS`-sided square is treated as a
/// sliver.
const SEAM_PAINT_DEGENERATE_AREA2_UNITS: i128 =
    (SEAM_PAINT_POINT_EPS_UNITS as i128) * (SEAM_PAINT_POINT_EPS_UNITS as i128);

fn seam_name(semantic: &slicer_ir::PaintSemantic) -> Option<&str> {
    if super::is_seam_paint_semantic(semantic) {
        match semantic {
            slicer_ir::PaintSemantic::Custom(name) => Some(name.as_str()),
            _ => None,
        }
    } else {
        None
    }
}

pub(crate) fn mesh_has_seam_paint(mesh: &slicer_ir::MeshIR) -> bool {
    mesh.objects.iter().any(|object| {
        let Some(paint_data) = &object.paint_data else {
            return false;
        };
        paint_data.layers.iter().any(|layer| {
            (seam_name(&layer.semantic).is_some()
                && (layer.facet_values.iter().any(Option::is_some) || !layer.strokes.is_empty()))
                || layer
                    .strokes
                    .iter()
                    .any(|stroke| seam_name(&stroke.semantic).is_some())
        })
    })
}

pub(crate) fn stamp_seam_paint_annotations(
    mesh: &slicer_ir::MeshIR,
    layers: &mut [slicer_ir::SliceIR],
) {
    let mut subsets: BTreeMap<String, slicer_ir::IndexedTriangleSet> = BTreeMap::new();

    for object in &mesh.objects {
        let Some(paint_data) = &object.paint_data else {
            continue;
        };
        for layer in &paint_data.layers {
            if let Some(name) = seam_name(&layer.semantic) {
                let subset = subsets.entry(name.to_owned()).or_insert_with(|| {
                    slicer_ir::IndexedTriangleSet {
                        vertices: Vec::new(),
                        indices: Vec::new(),
                    }
                });
                for (facet_idx, value) in layer.facet_values.iter().enumerate() {
                    if value.is_none() {
                        continue;
                    }
                    let base = facet_idx * 3;
                    if base + 2 >= object.mesh.indices.len() {
                        continue;
                    }
                    let indices = [
                        object.mesh.indices[base] as usize,
                        object.mesh.indices[base + 1] as usize,
                        object.mesh.indices[base + 2] as usize,
                    ];
                    if indices
                        .iter()
                        .any(|&index| index >= object.mesh.vertices.len())
                    {
                        continue;
                    }
                    let start = subset.vertices.len() as u32;
                    subset
                        .vertices
                        .extend(indices.iter().map(|&index| object.mesh.vertices[index]));
                    subset.indices.extend([start, start + 1, start + 2]);
                }
            }

            for stroke in &layer.strokes {
                let name = seam_name(&stroke.semantic).or_else(|| seam_name(&layer.semantic));
                let Some(name) = name else {
                    continue;
                };
                let subset = subsets.entry(name.to_owned()).or_insert_with(|| {
                    slicer_ir::IndexedTriangleSet {
                        vertices: Vec::new(),
                        indices: Vec::new(),
                    }
                });
                for triangle in &stroke.triangles {
                    let start = subset.vertices.len() as u32;
                    subset.vertices.extend(triangle);
                    subset.indices.extend([start, start + 1, start + 2]);
                }
            }
        }
    }

    for (name, subset) in subsets {
        if subset.indices.is_empty() {
            continue;
        }
        let projected_triangles: Vec<(slicer_ir::ExPolygon, f32, f32)> = subset
            .indices
            .chunks_exact(3)
            .filter_map(|indices| {
                let a = *subset.vertices.get(indices[0] as usize)?;
                let b = *subset.vertices.get(indices[1] as usize)?;
                let c = *subset.vertices.get(indices[2] as usize)?;
                let projection = slicer_ir::ExPolygon {
                    contour: slicer_ir::Polygon {
                        points: vec![
                            slicer_ir::Point2 {
                                x: (a.x * 10_000.0) as i64,
                                y: (a.y * 10_000.0) as i64,
                            },
                            slicer_ir::Point2 {
                                x: (b.x * 10_000.0) as i64,
                                y: (b.y * 10_000.0) as i64,
                            },
                            slicer_ir::Point2 {
                                x: (c.x * 10_000.0) as i64,
                                y: (c.y * 10_000.0) as i64,
                            },
                        ],
                    },
                    holes: Vec::new(),
                };
                Some((projection, a.z.min(b.z).min(c.z), a.z.max(b.z).max(c.z)))
            })
            .collect();
        let semantic = slicer_ir::PaintSemantic::Custom(name);
        for layer in layers.iter_mut() {
            for region in &mut layer.regions {
                let band_min = layer.z - region.effective_layer_height / 2.0;
                let band_max = layer.z + region.effective_layer_height / 2.0;
                let values = region
                    .polygons
                    .iter()
                    .map(|polygon| {
                        let points = &polygon.contour.points;
                        (0..points.len())
                            .map(|k| {
                                let a = points[k];
                                projected_triangles
                                    .iter()
                                    .any(|(projection, tri_min, tri_max)| {
                                        *tri_min <= band_max
                                        && *tri_max >= band_min
                                        && (super::modifier_volumes::any_expolygon_contains_point(
                                            std::slice::from_ref(projection), a,
                                        ) || point_near_degenerate_projection(projection, a))
                                    })
                                    .then_some(slicer_ir::PaintValue::Flag(true))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                region
                    .segment_annotations
                    .entry(semantic.clone())
                    .or_insert(values);
            }
        }
    }
}

fn point_near_degenerate_projection(
    projection: &slicer_ir::ExPolygon,
    point: slicer_ir::Point2,
) -> bool {
    let points = &projection.contour.points;
    if points.len() < 2 {
        return false;
    }
    let area2: i128 = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x as i128 * next.y as i128 - next.x as i128 * point.y as i128
        })
        .sum();
    if area2.abs() > SEAM_PAINT_DEGENERATE_AREA2_UNITS {
        return false;
    }
    points.iter().enumerate().any(|(index, start)| {
        let end = points[(index + 1) % points.len()];
        let dx = (end.x - start.x) as f64;
        let dy = (end.y - start.y) as f64;
        let length_squared = dx * dx + dy * dy;
        let t = if length_squared == 0.0 {
            0.0
        } else {
            (((point.x - start.x) as f64 * dx + (point.y - start.y) as f64 * dy) / length_squared)
                .clamp(0.0, 1.0)
        };
        let nearest_x = start.x as f64 + t * dx;
        let nearest_y = start.y as f64 + t * dy;
        (point.x as f64 - nearest_x).hypot(point.y as f64 - nearest_y)
            <= SEAM_PAINT_POINT_EPS_UNITS as f64
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{
        BoundingBox3, ConfigDelta, ConfigValue, IndexedTriangleSet, ModifierScope, ModifierVolume,
        ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point2, Point3, Polygon,
        Transform3d,
    };
    use std::collections::HashMap;

    fn cube_mesh(size: f32) -> IndexedTriangleSet {
        let v = |x, y, z| Point3 { x, y, z };
        let vertices = vec![
            v(0.0, 0.0, 0.0),
            v(size, 0.0, 0.0),
            v(size, size, 0.0),
            v(0.0, size, 0.0),
            v(0.0, 0.0, size),
            v(size, 0.0, size),
            v(size, size, size),
            v(0.0, size, size),
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    fn make_modifier_volume(subtype: &str, mesh: IndexedTriangleSet) -> ModifierVolume {
        let mut fields = HashMap::new();
        fields.insert(
            "subtype".to_string(),
            ConfigValue::String(subtype.to_string()),
        );
        // exhaustive: `ModifierVolume` has no `Default` impl, and this helper pins `priority`/`applies_to` deliberately — the writer must ignore modifier volumes regardless of scope or ordering
        ModifierVolume {
            id: "mv1".to_string(),
            mesh,
            config_delta: ConfigDelta { fields },
            priority: 0,
            applies_to: ModifierScope::AllFeatures,
        }
    }

    fn mesh_with_modifier(subtype: &str, mv_mesh: IndexedTriangleSet) -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: slicer_ir::CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: cube_mesh(10.0),
                transform: Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: vec![make_modifier_volume(subtype, mv_mesh)],
                paint_data: None,
                ..Default::default()
            }],
            build_volume: BoundingBox3 {
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
            },
        }
    }

    fn mesh_with_paint(name: &str) -> slicer_ir::MeshIR {
        let mut mesh = mesh_with_modifier("unused", cube_mesh(1.0));
        mesh.objects[0].paint_data = Some(Default::default());
        mesh.objects[0].paint_data.as_mut().unwrap().layers = vec![PaintLayer {
            semantic: PaintSemantic::Custom(name.to_string()),
            facet_values: vec![Some(PaintValue::Flag(true))],
            strokes: Vec::new(),
        }];
        mesh
    }

    fn mesh_with_vertex_only_paint(name: &str) -> slicer_ir::MeshIR {
        let mut mesh = mesh_with_paint(name);
        mesh.objects[0].mesh = IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                Point3 {
                    x: 0.2,
                    y: 0.0,
                    z: 1.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
            ],
            indices: vec![0, 1, 2],
        };
        mesh
    }

    fn region() -> slicer_ir::SlicedRegion {
        slicer_ir::SlicedRegion {
            polygons: vec![slicer_ir::ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 { x: 0, y: 10_000 },
                    ],
                },
                holes: Vec::new(),
            }],
            effective_layer_height: 1.0,
            variant_chain: Vec::new(),
            segment_annotations: Default::default(),
            ..Default::default()
        }
    }

    #[test]
    fn seam_blocker_only_is_stamped_without_enforcer() {
        let mesh = mesh_with_paint("seam_blocker");
        let mut layers = vec![slicer_ir::SliceIR {
            z: 0.5,
            regions: vec![region()],
            ..Default::default()
        }];
        stamp_seam_paint_annotations(&mesh, &mut layers);
        let annotations = &layers[0].regions[0].segment_annotations;
        assert!(
            annotations
                .get(&PaintSemantic::Custom("seam_blocker".into()))
                .unwrap()
                .iter()
                .flatten()
                .count()
                > 0
        );
        assert!(!annotations.contains_key(&PaintSemantic::Custom("seam_enforcer".into())));
    }

    #[test]
    fn non_empty_variant_chain_is_stamped() {
        let mesh = mesh_with_paint("seam_enforcer");
        let mut r = region();
        r.variant_chain
            .push(("variant".to_string(), PaintValue::Flag(true)));
        let mut layers = vec![slicer_ir::SliceIR {
            z: 0.5,
            regions: vec![r],
            ..Default::default()
        }];
        stamp_seam_paint_annotations(&mesh, &mut layers);
        assert!(layers[0].regions[0]
            .segment_annotations
            .contains_key(&PaintSemantic::Custom("seam_enforcer".into())));
    }

    #[test]
    fn holes_do_not_add_annotation_slots() {
        let mesh = mesh_with_paint("seam_blocker");
        let mut r = region();
        r.polygons[0].holes = vec![Polygon {
            points: vec![
                Point2 { x: 1, y: 1 },
                Point2 { x: 2, y: 1 },
                Point2 { x: 1, y: 2 },
            ],
        }];
        let mut layers = vec![slicer_ir::SliceIR {
            z: 0.5,
            regions: vec![r],
            ..Default::default()
        }];
        stamp_seam_paint_annotations(&mesh, &mut layers);
        let values = layers[0].regions[0]
            .segment_annotations
            .values()
            .next()
            .unwrap();
        assert_eq!(
            values[0].len(),
            layers[0].regions[0].polygons[0].contour.points.len()
        );
    }

    #[test]
    fn vertex_inside_paint_is_stamped_even_when_edge_midpoint_is_outside() {
        let mesh = mesh_with_vertex_only_paint("seam_enforcer");
        let mut layers = vec![slicer_ir::SliceIR {
            z: 0.5,
            regions: vec![region()],
            ..Default::default()
        }];

        stamp_seam_paint_annotations(&mesh, &mut layers);

        let values = &layers[0].regions[0].segment_annotations
            [&PaintSemantic::Custom("seam_enforcer".into())];
        assert_eq!(values[0][0], Some(PaintValue::Flag(true)));
        assert_eq!(values[0][1], None);
        assert_eq!(values[0][2], None);
    }
}
