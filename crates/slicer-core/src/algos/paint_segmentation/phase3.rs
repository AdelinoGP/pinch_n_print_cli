// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
/// Phase 3 driver — intersect painted triangles with layer Z plane.
use crate::algos::paint_segmentation::colorize::Contour;
use crate::algos::paint_segmentation::painted_line::{PaintedLine, PaintedLineVisitor};
use crate::algos::paint_segmentation::preprocess::{extract_paint_layer_data, extract_stroke_data};
use crate::algos::paint_segmentation::triangle_intersect::{triangle_z_intersection, Line};
use slicer_ir::{MeshIR, PaintLayer, Point2, Point3, SliceIR};

/// First-layer detection threshold (world Z, mm).  At slice z within this distance of the
/// object's world-space z_min, vertical-face paint is suppressed for any PaintLayer whose
/// bottom-face triangles carry no paint — this implements the OrcaSlicer Phase 6
/// bottom-face-dominance rule for the unpainted-bottom case without requiring the full
/// top/bottom propagation pipeline.
const FIRST_LAYER_Z_THRESHOLD_MM: f32 = 0.5;

/// Z-tolerance for classifying a vertex as "on" a horizontal face (mm).
const BOTTOM_FACE_Z_EPS_MM: f32 = 0.01;

/// Minimum |n_xy| / |n_3d| ratio for using the face-normal projection hint.  Triangles
/// flatter than ~17.5° from horizontal fall back to the geometric-closest (first-match)
/// behavior — they cannot reliably disambiguate between contour edges.
const FACE_NORMAL_XY_RATIO_MIN: f64 = 0.3;

// ---------------------------------------------------------------------------
// Coordinate-space helpers
// ---------------------------------------------------------------------------

/// Transform a 2D contour point from local object space to world space using
/// the 4×4 column-major transform matrix (same layout as `transform_point3`).
///
/// The contour lives at the layer Z plane; the third coordinate (z) is supplied
/// as `local_z` so that the correct column (w = matrix[15]) denominator is used.
/// For affine transforms without perspective (matrix[3]==matrix[7]==matrix[11]==0,
/// matrix[15]==1) the w component is always 1 and the formula simplifies to a
/// straightforward affine map.
#[inline]
fn transform_point2_to_world(pt: Point2, local_z: f64, matrix: &[f64; 16]) -> Point2 {
    // Convert internal units → mm for the matrix multiply (matrix is in mm).
    let x_mm = pt.x as f64 / 10_000.0;
    let y_mm = pt.y as f64 / 10_000.0;
    // Column-major: element at column c, row r → matrix[c*4 + r].
    // World_x = m[0]*x + m[4]*y + m[8]*z + m[12]
    // World_y = m[1]*x + m[5]*y + m[9]*z + m[13]
    let wx_mm = matrix[0] * x_mm + matrix[4] * y_mm + matrix[8] * local_z + matrix[12];
    let wy_mm = matrix[1] * x_mm + matrix[5] * y_mm + matrix[9] * local_z + matrix[13];
    let ww = matrix[3] * x_mm + matrix[7] * y_mm + matrix[11] * local_z + matrix[15];
    let w = if ww.abs() < 1e-9 { 1.0 } else { ww };
    Point2 {
        x: ((wx_mm / w) * 10_000.0).round() as i64,
        y: ((wy_mm / w) * 10_000.0).round() as i64,
    }
}

/// Compute the local-space Z corresponding to `world_z` for a given transform.
///
/// For a column-major 4×4 transform `M`, the Z row of the forward map is:
///   `world_z = M[2]*local_x + M[6]*local_y + M[10]*local_z + M[14]`
///
/// For typical slicer objects (no shear on Z, orthogonal Z axis) `M[2]≈M[6]≈0`
/// and `M[10]≈scale_z ≥ 1`, so:
///   `local_z ≈ (world_z - M[14]) / M[10]`
///
/// Returns `world_z` unchanged when `M` is the identity or when the Z scale
/// factor is effectively zero (degenerate transform).
#[inline]
fn world_z_to_local(world_z: f32, matrix: &[f64; 16]) -> f32 {
    let scale_z = matrix[10]; // column-major: element at col 2, row 2 = index 2*4+2 = 10
    let tz = matrix[14]; // column-major: element at col 3, row 2 = index 3*4+2 = 14
    if scale_z.abs() < 1e-9 {
        world_z // degenerate — return unchanged
    } else {
        ((world_z as f64 - tz) / scale_z) as f32
    }
}

/// Build world-space contours for a single object by transforming each
/// contour-edge endpoint from local space to world space.
///
/// `local_z` is the local-space Z at which the contour was computed;
/// it is needed to correctly handle rotated objects where the XY projection
/// of the Z-axis is non-zero (M[2], M[6] terms).
fn world_contours_for_object(
    contours: &[Contour],
    local_z: f64,
    matrix: &[f64; 16],
) -> Vec<Contour> {
    // Fast path: identity transform → no conversion needed.
    let is_identity = matrix[0] == 1.0
        && matrix[1] == 0.0
        && matrix[2] == 0.0
        && matrix[3] == 0.0
        && matrix[4] == 0.0
        && matrix[5] == 1.0
        && matrix[6] == 0.0
        && matrix[7] == 0.0
        && matrix[8] == 0.0
        && matrix[9] == 0.0
        && matrix[10] == 1.0
        && matrix[11] == 0.0
        && matrix[12] == 0.0
        && matrix[13] == 0.0
        && matrix[14] == 0.0
        && matrix[15] == 1.0;
    if is_identity {
        return contours.to_vec();
    }

    contours
        .iter()
        .map(|c| Contour {
            edges: c
                .edges
                .iter()
                .map(|e| Line {
                    start: transform_point2_to_world(e.start, local_z, matrix),
                    end: transform_point2_to_world(e.end, local_z, matrix),
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Contour-projection helpers
// ---------------------------------------------------------------------------

/// Return true when `pt` lies within 1 unit of the edge's bounding box.
#[inline]
fn point_near_edge_bbox(pt: Point2, edge: &Line) -> bool {
    let min_x = edge.start.x.min(edge.end.x) - 1;
    let max_x = edge.start.x.max(edge.end.x) + 1;
    let min_y = edge.start.y.min(edge.end.y) - 1;
    let max_y = edge.start.y.max(edge.end.y) + 1;
    pt.x >= min_x && pt.x <= max_x && pt.y >= min_y && pt.y <= max_y
}

/// Compute the XY-projected, normalized face normal of a 3D triangle.
///
/// Returns `None` when the triangle is nearly horizontal (the XY share of the 3D
/// normal is below `FACE_NORMAL_XY_RATIO_MIN`), since such triangles cannot
/// reliably disambiguate between adjacent contour edges.
fn triangle_face_normal_xy(verts: &[Point3; 3]) -> Option<(f64, f64)> {
    let e1x = (verts[1].x - verts[0].x) as f64;
    let e1y = (verts[1].y - verts[0].y) as f64;
    let e1z = (verts[1].z - verts[0].z) as f64;
    let e2x = (verts[2].x - verts[0].x) as f64;
    let e2y = (verts[2].y - verts[0].y) as f64;
    let e2z = (verts[2].z - verts[0].z) as f64;
    let nx = e1y * e2z - e1z * e2y;
    let ny = e1z * e2x - e1x * e2z;
    let nz = e1x * e2y - e1y * e2x;
    let len_xy = (nx * nx + ny * ny).sqrt();
    let len_3d = (nx * nx + ny * ny + nz * nz).sqrt();
    if len_3d < 1e-9 || len_xy / len_3d < FACE_NORMAL_XY_RATIO_MIN {
        return None;
    }
    Some((nx / len_xy, ny / len_xy))
}

/// Compute the outward unit normal of a contour edge, assuming CCW polygon
/// orientation (interior on the left of the walk; outward is rotate-right of
/// the edge direction → `(dy, -dx) / |edge|`).
fn edge_outward_normal(edge: &Line) -> Option<(f64, f64)> {
    let dx = (edge.end.x - edge.start.x) as f64;
    let dy = (edge.end.y - edge.start.y) as f64;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 {
        return None;
    }
    Some((dy / len, -dx / len))
}

/// Try to match `line` to a contour edge whose bounding box contains both endpoints.
///
/// When `face_normal_xy` is provided AND multiple candidate edges match, the edge
/// whose outward normal has the highest absolute alignment with the face normal
/// wins.  This resolves the corner-ambiguity where a vertical-face stroke triangle
/// near a polygon corner has its Z-intersection projecting onto BOTH adjacent
/// edges by bbox-containment — first-match would pick the wrong face.
///
/// Returns `None` when no candidate edge is found within tolerance.
fn project_onto_contour(
    line: &Line,
    contours: &[Contour],
    face_normal_xy: Option<(f64, f64)>,
) -> Option<(usize, usize, Line)> {
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (ci, contour) in contours.iter().enumerate() {
        for (li, edge) in contour.edges.iter().enumerate() {
            if point_near_edge_bbox(line.start, edge) && point_near_edge_bbox(line.end, edge) {
                candidates.push((ci, li));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }

    let (ci, li) = match (face_normal_xy, candidates.len()) {
        (Some((fnx, fny)), n) if n > 1 => candidates
            .iter()
            .copied()
            .max_by(|a, b| {
                let score = |(c, l): (usize, usize)| -> f64 {
                    edge_outward_normal(&contours[c].edges[l])
                        .map(|(nx, ny)| (nx * fnx + ny * fny).abs())
                        .unwrap_or(f64::NEG_INFINITY)
                };
                score(*a)
                    .partial_cmp(&score(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(candidates[0]),
        _ => candidates[0],
    };

    let edge = &contours[ci].edges[li];
    let clamp = |v: i64, lo: i64, hi: i64| v.max(lo).min(hi);
    let min_x = edge.start.x.min(edge.end.x);
    let max_x = edge.start.x.max(edge.end.x);
    let min_y = edge.start.y.min(edge.end.y);
    let max_y = edge.start.y.max(edge.end.y);
    let projected = Line {
        start: Point2 {
            x: clamp(line.start.x, min_x, max_x),
            y: clamp(line.start.y, min_y, max_y),
        },
        end: Point2 {
            x: clamp(line.end.x, min_x, max_x),
            y: clamp(line.end.y, min_y, max_y),
        },
    };
    Some((ci, li, projected))
}

/// Compute the object's world-space z_min from its local-space vertices and transform.
/// Assumes the transform's Z row has no XY shear (M[2]≈M[6]≈0), which holds for the
/// typical slicer transforms (rotation about Z + translation + scale).
fn object_world_z_min(mesh: &slicer_ir::IndexedTriangleSet, matrix: &[f64; 16]) -> f32 {
    if mesh.vertices.is_empty() {
        return 0.0;
    }
    let local_min = mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::INFINITY, f32::min);
    (matrix[10] * local_min as f64 + matrix[14]) as f32
}

/// Return `true` iff `paint_layer` carries any paint on a triangle whose all three
/// vertices lie on the object's local-space bottom plane (z within
/// `BOTTOM_FACE_Z_EPS_MM` of `local_z_min`).
///
/// Used by the first-layer suppression check: when the bottom face is unpainted for
/// a given semantic, vertical-face paint of the same semantic does not bleed into
/// the first-layer slab (matches OrcaSlicer Phase 6 bottom-face dominance).
fn paint_layer_bottom_face_has_paint(
    paint_layer: &PaintLayer,
    mesh: &slicer_ir::IndexedTriangleSet,
    local_z_min: f32,
) -> bool {
    let facet_count = mesh.indices.len() / 3;
    let eps = BOTTOM_FACE_Z_EPS_MM;
    for (facet_idx, fv) in paint_layer.facet_values.iter().enumerate() {
        if facet_idx >= facet_count {
            break;
        }
        if fv.is_none() {
            continue;
        }
        let base = facet_idx * 3;
        let v0 = mesh.vertices[mesh.indices[base] as usize];
        let v1 = mesh.vertices[mesh.indices[base + 1] as usize];
        let v2 = mesh.vertices[mesh.indices[base + 2] as usize];
        if (v0.z - local_z_min).abs() < eps
            && (v1.z - local_z_min).abs() < eps
            && (v2.z - local_z_min).abs() < eps
        {
            return true;
        }
    }
    for stroke in &paint_layer.strokes {
        for tri in &stroke.triangles {
            if (tri[0].z - local_z_min).abs() < eps
                && (tri[1].z - local_z_min).abs() < eps
                && (tri[2].z - local_z_min).abs() < eps
            {
                return true;
            }
        }
    }
    false
}

/// World-space test: are all three triangle vertices within `BOTTOM_FACE_Z_EPS_MM`
/// of `world_z_min`?  Identifies horizontal bottom-face triangles after transform.
#[inline]
fn triangle_is_bottom_face_world(verts: &[Point3; 3], world_z_min: f32) -> bool {
    let eps = BOTTOM_FACE_Z_EPS_MM;
    (verts[0].z - world_z_min).abs() < eps
        && (verts[1].z - world_z_min).abs() < eps
        && (verts[2].z - world_z_min).abs() < eps
}

/// Collect painted lines for one layer by intersecting painted triangles with the Z plane.
///
/// `contours` — the ordered polygon boundaries for this layer, expressed in **local object
/// space** (as produced by `slice_mesh_ex` on the untransformed mesh).  Each object's
/// transform is used to lift the contour edges into world space before matching against
/// the world-space triangle-intersection lines.  When a painted line falls geometrically
/// on a contour edge, its `contour_idx`, `line_idx`, and `projected_line` are set
/// accordingly.  Lines that do not match any contour edge are **dropped** — emitting
/// an unmatched line onto contour edge 0 pollutes that edge with paint from geometrically
/// unrelated faces (cross-face bleed).
pub fn collect_painted_lines(
    slice: &SliceIR,
    mesh_ir: &MeshIR,
    contours: &[Contour],
) -> Vec<PaintedLine> {
    let mut visitor = PaintedLineVisitor::new();
    let world_z = slice.z;

    for object in &mesh_ir.objects {
        let Some(paint_data) = &object.paint_data else {
            continue;
        };
        let transform = &object.transform.matrix;

        // Compute local-space Z for this object.  The contours were built from
        // the untransformed mesh sliced at `world_z`, so the contour XY coordinates
        // are in local space.  We transform the contour edges to world space to match
        // the world-space triangle intersections produced by `extract_paint_layer_data`.
        let local_z = world_z_to_local(world_z, transform);
        let obj_contours = world_contours_for_object(contours, local_z as f64, transform);

        // First-layer suppression context: derive world-space z_min and local-space
        // z_min once per object (cheap; reuse across paint layers).
        let local_z_min = if object.mesh.vertices.is_empty() {
            0.0_f32
        } else {
            object
                .mesh
                .vertices
                .iter()
                .map(|v| v.z)
                .fold(f32::INFINITY, f32::min)
        };
        let world_z_min = object_world_z_min(&object.mesh, transform);
        let is_first_layer = (world_z - world_z_min) < FIRST_LAYER_Z_THRESHOLD_MM;

        for paint_layer in &paint_data.layers {
            // Bottom-face-dominance check: only suppress this PaintLayer's vertical-face
            // paint at the first layer if the bottom face is unpainted for THIS semantic.
            let suppress_first_layer = is_first_layer
                && !paint_layer_bottom_face_has_paint(paint_layer, &object.mesh, local_z_min);

            // From facet_values (world-space vertices after transform)
            let facet_paints = extract_paint_layer_data(paint_layer, &object.mesh, transform);
            for tp in facet_paints {
                if suppress_first_layer && !triangle_is_bottom_face_world(&tp.vertices, world_z_min)
                {
                    continue;
                }
                if let Some(line) =
                    triangle_z_intersection(tp.vertices[0], tp.vertices[1], tp.vertices[2], world_z)
                {
                    let face_n = triangle_face_normal_xy(&tp.vertices);
                    let Some((contour_idx, line_idx, projected_line)) =
                        project_onto_contour(&line, &obj_contours, face_n)
                    else {
                        continue;
                    };
                    visitor.push(PaintedLine {
                        line,
                        semantic: tp.semantic.clone(),
                        value: tp.value.clone(),
                        cell_indices: Vec::new(),
                        contour_idx,
                        line_idx,
                        projected_line,
                    });
                }
            }

            // From strokes (world-space vertices after transform)
            let stroke_paints = extract_stroke_data(&paint_layer.strokes, transform);
            for tp in stroke_paints {
                if suppress_first_layer && !triangle_is_bottom_face_world(&tp.vertices, world_z_min)
                {
                    continue;
                }
                if let Some(line) =
                    triangle_z_intersection(tp.vertices[0], tp.vertices[1], tp.vertices[2], world_z)
                {
                    let face_n = triangle_face_normal_xy(&tp.vertices);
                    let Some((contour_idx, line_idx, projected_line)) =
                        project_onto_contour(&line, &obj_contours, face_n)
                    else {
                        continue;
                    };
                    visitor.push(PaintedLine {
                        line,
                        semantic: tp.semantic.clone(),
                        value: tp.value.clone(),
                        cell_indices: Vec::new(),
                        contour_idx,
                        line_idx,
                        projected_line,
                    });
                }
            }
        }
    }

    visitor.lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::paint_segmentation::colorize::Contour;
    use crate::algos::paint_segmentation::triangle_intersect::Line;
    use slicer_ir::{
        BoundingBox3, FacetPaintData, ObjectConfig, ObjectMesh, PaintSemantic, PaintValue, Point2,
        Point3, Transform3d, CURRENT_MESH_IR_SCHEMA_VERSION, CURRENT_SLICE_IR_SCHEMA_VERSION,
    };

    fn identity_transform() -> Transform3d {
        Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn make_mesh_ir(paint_data: Option<FacetPaintData>) -> MeshIR {
        MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 5.0,
                            y: 10.0,
                            z: 10.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data,
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 100.0,
                    y: 100.0,
                    z: 100.0,
                },
            },
        }
    }

    #[test]
    fn collect_painted_lines_simple() {
        let mesh_ir = make_mesh_ir(Some(FacetPaintData {
            layers: vec![slicer_ir::PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(1))],
                strokes: Vec::new(),
            }],
        }));
        let slice = SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 5,
            z: 5.0,
            regions: Vec::new(),
        };
        // The mesh triangle (vertices at (0,0,0), (10,0,10), (5,10,10)) intersects z=5.0
        // from approx (5mm,0mm) to (2.5mm,5mm) in world space, i.e. units (50000,0)→(25000,50000).
        // Provide a contour edge large enough to cover both endpoints so the match succeeds.
        let contours = vec![Contour {
            edges: vec![Line {
                start: Point2 { x: 0, y: 0 },
                end: Point2 {
                    x: 100000,
                    y: 100000,
                },
            }],
        }];
        let lines = collect_painted_lines(&slice, &mesh_ir, &contours);
        assert!(!lines.is_empty());
        assert_eq!(lines[0].semantic, PaintSemantic::Material);
    }

    #[test]
    fn collect_painted_lines_no_paint() {
        let mesh_ir = make_mesh_ir(None);
        let slice = SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.2,
            regions: Vec::new(),
        };
        let lines = collect_painted_lines(&slice, &mesh_ir, &[]);
        assert!(lines.is_empty());
    }

    /// Synthetic test: a painted facet whose Z intersection falls exactly on a known
    /// contour edge; asserts the emitted PaintedLine carries the correct contour_idx,
    /// line_idx, and a projected_line within the edge's span.
    #[test]
    fn collect_painted_lines_tags_real_contour_indices() {
        // Contour edge: (0,0) → (10000,0) in 2D (units = 100 nm).
        // The mesh triangle below intersects z=5.0 along the line y≈0, x in [0..10000].
        // We mirror those coordinates in Point3 (mm), knowing to_point2 scales by ×10000.
        // to_point2: x_units = (x_mm * 10000).round(); so x_mm=0.0 → 0, x_mm=1.0 → 10000.
        let contour_edge = Line {
            start: Point2 { x: 0, y: 0 },
            end: Point2 { x: 10000, y: 0 },
        };
        let contours = vec![Contour {
            edges: vec![contour_edge],
        }];

        // Triangle in 3D (mm) that straddles z=5.0 and whose intersection
        // projects onto the contour edge above.
        // Vertices:
        //   p0 = (0.0, 0.0,  0.0) — below z=5
        //   p1 = (1.0, 0.0, 10.0) — above z=5
        //   p2 = (0.5, 0.0, 10.0) — above z=5
        // Z=5 intersection is along y=0, x in [0..~0.5] (mm) = [0..~5000] units.
        use slicer_ir::{IndexedTriangleSet, ObjectMesh};
        let vertices = vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 10.0,
            },
            Point3 {
                x: 0.5,
                y: 0.0,
                z: 10.0,
            },
        ];
        let mesh_ir = MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj_test".to_string(),
                mesh: IndexedTriangleSet {
                    vertices: vertices.clone(),
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: Some(FacetPaintData {
                    layers: vec![slicer_ir::PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(3))],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 100.0,
                    y: 100.0,
                    z: 100.0,
                },
            },
        };

        let slice = SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 1,
            z: 5.0,
            regions: Vec::new(),
        };

        let lines = collect_painted_lines(&slice, &mesh_ir, &contours);
        assert!(
            !lines.is_empty(),
            "expected at least one painted line from the triangle"
        );

        // The intersection at y=0 falls on contour edge (ci=0, li=0).
        let matched = lines
            .iter()
            .find(|pl| pl.contour_idx == 0 && pl.line_idx == 0);
        assert!(
            matched.is_some(),
            "expected PaintedLine tagged with contour_idx=0 line_idx=0; got: {:?}",
            lines
        );
        let pl = matched.unwrap();
        // projected_line endpoints must be within the edge's bounding box (x in [0..10000], y=0).
        assert!(
            pl.projected_line.start.x >= 0 && pl.projected_line.start.x <= 10000,
            "projected_line.start.x out of edge range: {}",
            pl.projected_line.start.x
        );
        assert!(
            pl.projected_line.end.x >= 0 && pl.projected_line.end.x <= 10000,
            "projected_line.end.x out of edge range: {}",
            pl.projected_line.end.x
        );
    }

    // ---------------------------------------------------------------------------
    // Vertical-face projection tests (B-1 regression guard)
    // ---------------------------------------------------------------------------

    /// A single vertical triangle from (1.0mm, 0, 0mm) → (1.0mm, 2.5mm, 0mm) →
    /// (1.0mm, 2.5mm, 2.5mm) at z=1.25mm should produce ≥ 1 PaintedLine with a
    /// non-degenerate `projected_line` on the contour edge at x=1.0mm.
    ///
    /// In internal units: x=10000, y∈[0..25000], z∈[0..25000]; contour edge at
    /// x=10000 running from (10000, 0) to (10000, 25000); slice z=1.25mm=12500u.
    #[test]
    fn vertical_face_triangle_produces_painted_line_with_contour_match() {
        use slicer_ir::{
            BoundingBox3, FacetPaintData, IndexedTriangleSet, ObjectConfig, ObjectMesh,
        };

        // Contour edge: the vertical edge at x=10000 (1mm) from y=0 to y=25000 (2.5mm).
        let contour_edge = Line {
            start: Point2 { x: 10000, y: 0 },
            end: Point2 { x: 10000, y: 25000 },
        };
        let contours = vec![Contour {
            edges: vec![contour_edge],
        }];

        // Vertical triangle (all vertices at x=1.0mm in 3D):
        //   p0 = (1.0mm, 0,    0)    → local units: x=10000, y=0,     z=0
        //   p1 = (1.0mm, 2.5mm, 0)   → local units: x=10000, y=25000, z=0
        //   p2 = (1.0mm, 2.5mm, 2.5mm) → x=10000, y=25000, z=25000
        // At z=1.25mm the intersection should be a non-degenerate line along x=10000.
        let mesh_ir = MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "vf_obj".to_string(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 2.5,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 2.5,
                            z: 2.5,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: Some(FacetPaintData {
                    layers: vec![slicer_ir::PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(1))],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 10.0,
                    y: 10.0,
                    z: 10.0,
                },
            },
        };

        let slice = slicer_ir::SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 1.25, // 1.25mm = midpoint of vertical face
            regions: Vec::new(),
        };

        let lines = collect_painted_lines(&slice, &mesh_ir, &contours);

        assert!(
            !lines.is_empty(),
            "vertical face triangle must produce ≥ 1 PaintedLine at z=1.25mm"
        );

        // At least one PaintedLine must be non-degenerate (start ≠ end).
        let non_degenerate = lines
            .iter()
            .any(|pl| pl.projected_line.start != pl.projected_line.end);
        assert!(
            non_degenerate,
            "at least one PaintedLine from vertical face must have non-degenerate \
             projected_line (start ≠ end); got: {:?}",
            lines.iter().map(|pl| pl.projected_line).collect::<Vec<_>>()
        );

        // The matching contour edge (ci=0, li=0) should be found.
        let matched = lines
            .iter()
            .find(|pl| pl.contour_idx == 0 && pl.line_idx == 0);
        assert!(
            matched.is_some(),
            "expected PaintedLine matched to contour_idx=0 line_idx=0; got: {:?}",
            lines
        );
    }

    /// Same vertical face triangle BUT with a non-identity translation transform.
    /// Verifies that `world_contours_for_object` correctly lifts local contour edges
    /// to world space so that the world-space intersection matches.
    ///
    /// Transform: translate by (10.0mm, 5.0mm, 0.0mm) in XY.
    /// Local triangle: x=1.0mm → world x=11.0mm = 110000 units.
    /// Local contour edge: x=10000 → world x=110000 after transform.
    #[test]
    fn vertical_face_with_translation_transform_matches_world_contour() {
        use slicer_ir::{
            BoundingBox3, FacetPaintData, IndexedTriangleSet, ObjectConfig, ObjectMesh,
        };

        // Local-space contour edge (as produced by slice_mesh_ex on local mesh):
        // x=10000 (1mm), y from 0 to 25000.
        let contour_edge = Line {
            start: Point2 { x: 10000, y: 0 },
            end: Point2 { x: 10000, y: 25000 },
        };
        let contours = vec![Contour {
            edges: vec![contour_edge],
        }];

        // Translation transform: +10mm in X, +5mm in Y, +0mm in Z.
        // Column-major 4×4:
        //   [1, 0, 0, 0,   0, 1, 0, 0,   0, 0, 1, 0,   10, 5, 0, 1]
        // i.e. matrix[12]=10.0, matrix[13]=5.0, matrix[14]=0.0.
        let translate_transform = Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, // col 0
                0.0, 1.0, 0.0, 0.0, // col 1
                0.0, 0.0, 1.0, 0.0, // col 2
                10.0, 5.0, 0.0, 1.0, // col 3
            ],
        };

        let mesh_ir = MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "vf_translated".to_string(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        }, // local
                        Point3 {
                            x: 1.0,
                            y: 2.5,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 2.5,
                            z: 2.5,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: translate_transform,
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: Some(FacetPaintData {
                    layers: vec![slicer_ir::PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(2))],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 30.0,
                    y: 30.0,
                    z: 10.0,
                },
            },
        };

        // World z = 1.25mm (local z = 1.25mm since no Z translation).
        let slice = slicer_ir::SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 1.25,
            regions: Vec::new(),
        };

        let lines = collect_painted_lines(&slice, &mesh_ir, &contours);

        assert!(
            !lines.is_empty(),
            "vertical face with translation must produce ≥ 1 PaintedLine"
        );

        // Verify the painted line has the correct ToolIndex value (2, not some default).
        let correct_value = lines.iter().any(|pl| pl.value == PaintValue::ToolIndex(2));
        assert!(
            correct_value,
            "PaintedLine must carry ToolIndex(2); got values: {:?}",
            lines.iter().map(|pl| &pl.value).collect::<Vec<_>>()
        );

        // The contour match should succeed: after transforming local edge x=10000
        // by +10mm → world x=110000. The world-space intersection is also at x=110000.
        let matched_contour = lines
            .iter()
            .find(|pl| pl.contour_idx == 0 && pl.line_idx == 0);
        assert!(
            matched_contour.is_some(),
            "vertical face with translation must match contour edge (ci=0, li=0) \
             after world-space lifting; got: {:?}",
            lines
        );
    }
}
