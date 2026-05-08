//! Paint segmentation execution contract.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, LayerPaintMap, LayerPlanIR, MeshIR, PaintRegionIR, PaintSemantic, PaintValue,
    Point2, Point3, Polygon, SemanticRegion, SurfaceClassificationIR,
};

/// Structured paint-segmentation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintSegmentationError {
    /// Surface classification data is missing for a mesh object.
    MissingSurfaceObject {
        /// Object that could not be matched in `SurfaceClassificationIR`.
        object_id: String,
    },
    /// Layer planning data is missing for a mesh object.
    MissingLayerParticipation {
        /// Object that could not be matched in `LayerPlanIR.object_participation`.
        object_id: String,
    },
    /// One paint layer does not have one facet value per mesh triangle.
    MalformedFacetValues {
        /// Object carrying the malformed paint layer.
        object_id: String,
        /// Paint layer index within `FacetPaintData.layers`.
        layer_index: usize,
        /// Expected triangle count from `mesh.indices.len() / 3`.
        expected_facets: usize,
        /// Actual number of facet values present in the paint layer.
        actual_facet_values: usize,
    },
    /// Overlapping custom paint produced a deterministic equal-precedence conflict.
    DeterministicConflict {
        /// Global layer where the conflict occurs.
        global_layer_index: u32,
        /// Object owning the conflicting regions.
        object_id: String,
        /// Semantic family carrying the conflict.
        semantic: PaintSemantic,
        /// Equal precedence that caused the fatal ambiguity.
        paint_order: u64,
    },
}

/// Convert segmented whole-triangle paint assignments into immutable per-layer paint regions.
pub fn execute_paint_segmentation(
    mesh_ir: Arc<MeshIR>,
    surface_classification_ir: Arc<SurfaceClassificationIR>,
    layer_plan_ir: Arc<LayerPlanIR>,
) -> Result<Arc<PaintRegionIR>, PaintSegmentationError> {
    let mut per_layer = layer_plan_ir
        .global_layers
        .iter()
        .map(|layer| {
            (
                layer.index,
                LayerPaintMap {
                    global_layer_index: layer.index,
                    semantic_regions: HashMap::new(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    for object in &mesh_ir.objects {
        let Some(paint_data) = &object.paint_data else {
            continue;
        };

        surface_classification_ir
            .per_object
            .get(&object.id)
            .ok_or_else(|| PaintSegmentationError::MissingSurfaceObject {
                object_id: object.id.clone(),
            })?;

        let participating_layers = layer_plan_ir
            .object_participation
            .get(&object.id)
            .ok_or_else(|| PaintSegmentationError::MissingLayerParticipation {
                object_id: object.id.clone(),
            })?;

        let facet_count = object.mesh.indices.len() / 3;
        let projected_facets = (0..facet_count)
            .map(|facet_index| project_facet(&object.mesh, &object.transform.matrix, facet_index))
            .collect::<Vec<_>>();

        for (paint_order, layer) in paint_data.layers.iter().enumerate() {
            if layer.facet_values.len() != facet_count {
                return Err(PaintSegmentationError::MalformedFacetValues {
                    object_id: object.id.clone(),
                    layer_index: paint_order,
                    expected_facets: facet_count,
                    actual_facet_values: layer.facet_values.len(),
                });
            }

            for (facet_index, facet_value) in layer.facet_values.iter().enumerate() {
                let Some(value) = facet_value else {
                    continue;
                };

                let polygon = projected_facets[facet_index].clone();

                for object_layer in participating_layers {
                    let layer_map = per_layer
                        .get_mut(&object_layer.global_layer_index)
                        .ok_or_else(|| PaintSegmentationError::MissingLayerParticipation {
                            object_id: object.id.clone(),
                        })?;

                    let semantic_regions = layer_map
                        .semantic_regions
                        .entry(layer.semantic.clone())
                        .or_insert_with(Vec::new);

                    if matches!(layer.semantic, PaintSemantic::Custom(_)) {
                        detect_custom_conflict(
                            &object.id,
                            object_layer.global_layer_index,
                            &layer.semantic,
                            value.clone(),
                            paint_order as u64,
                            &polygon,
                            semantic_regions,
                        )?;
                    }

                    push_polygon_region(
                        semantic_regions,
                        &object.id,
                        value.clone(),
                        paint_order as u64,
                        polygon.clone(),
                    );
                }
            }
        }
    }

    for layer_map in per_layer.values_mut() {
        for regions in layer_map.semantic_regions.values_mut() {
            regions.sort_by(compare_semantic_regions);
        }
    }

    Ok(Arc::new(PaintRegionIR {
        schema_version: mesh_ir.schema_version,
        per_layer,
    }))
}

fn push_polygon_region(
    semantic_regions: &mut Vec<SemanticRegion>,
    object_id: &str,
    value: PaintValue,
    paint_order: u64,
    polygon: ExPolygon,
) {
    if let Some(existing_region) = semantic_regions.iter_mut().find(|region| {
        region.object_id == object_id && region.value == value && region.paint_order == paint_order
    }) {
        existing_region.polygons.push(polygon);
        return;
    }

    semantic_regions.push(SemanticRegion {
        object_id: object_id.to_owned(),
        polygons: vec![polygon],
        value,
        paint_order,
    });
}

fn detect_custom_conflict(
    object_id: &str,
    global_layer_index: u32,
    semantic: &PaintSemantic,
    value: PaintValue,
    paint_order: u64,
    polygon: &ExPolygon,
    semantic_regions: &[SemanticRegion],
) -> Result<(), PaintSegmentationError> {
    for region in semantic_regions {
        if region.object_id != object_id
            || region.paint_order != paint_order
            || region.value == value
        {
            continue;
        }

        if region
            .polygons
            .iter()
            .any(|existing| polygons_overlap(existing, polygon))
        {
            return Err(PaintSegmentationError::DeterministicConflict {
                global_layer_index,
                object_id: object_id.to_owned(),
                semantic: semantic.clone(),
                paint_order,
            });
        }
    }

    Ok(())
}

fn project_facet(
    mesh: &slicer_ir::IndexedTriangleSet,
    matrix: &[f64; 16],
    facet_index: usize,
) -> ExPolygon {
    let index_base = facet_index * 3;
    let contour = (0..3)
        .map(|offset| {
            let vertex_index = mesh.indices[index_base + offset] as usize;
            let vertex = mesh.vertices[vertex_index];
            let transformed = transform_point(vertex, matrix);
            Point2::from_mm(transformed.x, transformed.y)
        })
        .collect::<Vec<_>>();

    ExPolygon {
        contour: Polygon { points: contour },
        holes: Vec::new(),
    }
}

fn transform_point(point: Point3, matrix: &[f64; 16]) -> Point3 {
    let x = f64::from(point.x);
    let y = f64::from(point.y);
    let z = f64::from(point.z);
    let transformed_x = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12];
    let transformed_y = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13];
    let transformed_z = matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14];
    let transformed_w = matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15];

    if transformed_w != 0.0 && transformed_w != 1.0 {
        return Point3 {
            x: (transformed_x / transformed_w) as f32,
            y: (transformed_y / transformed_w) as f32,
            z: (transformed_z / transformed_w) as f32,
        };
    }

    Point3 {
        x: transformed_x as f32,
        y: transformed_y as f32,
        z: transformed_z as f32,
    }
}

fn compare_semantic_regions(left: &SemanticRegion, right: &SemanticRegion) -> Ordering {
    left.object_id
        .cmp(&right.object_id)
        .then_with(|| left.paint_order.cmp(&right.paint_order))
        .then_with(|| compare_paint_values(&left.value, &right.value))
        .then_with(|| compare_polygon_sets(&left.polygons, &right.polygons))
}

fn compare_paint_values(left: &PaintValue, right: &PaintValue) -> Ordering {
    match (left, right) {
        (PaintValue::Flag(l), PaintValue::Flag(r)) => l.cmp(r),
        (PaintValue::Flag(_), _) => Ordering::Less,
        (_, PaintValue::Flag(_)) => Ordering::Greater,
        (PaintValue::Scalar(l), PaintValue::Scalar(r)) => l.total_cmp(r),
        (PaintValue::Scalar(_), PaintValue::ToolIndex(_)) => Ordering::Less,
        (PaintValue::ToolIndex(_), PaintValue::Scalar(_)) => Ordering::Greater,
        (PaintValue::ToolIndex(l), PaintValue::ToolIndex(r)) => l.cmp(r),
        (PaintValue::Custom(l), PaintValue::Custom(r)) => l.cmp(r),
        (PaintValue::Custom(_), _) => Ordering::Greater,
        (_, PaintValue::Custom(_)) => Ordering::Less,
    }
}

fn compare_polygon_sets(left: &[ExPolygon], right: &[ExPolygon]) -> Ordering {
    let left_signature = polygon_signature(left);
    let right_signature = polygon_signature(right);
    left_signature.cmp(&right_signature)
}

fn polygon_signature(polygons: &[ExPolygon]) -> Vec<Vec<(i64, i64)>> {
    polygons
        .iter()
        .map(|polygon| {
            polygon
                .contour
                .points
                .iter()
                .map(|point| (point.x, point.y))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn polygons_overlap(left: &ExPolygon, right: &ExPolygon) -> bool {
    contours_overlap(&left.contour.points, &right.contour.points)
}

fn contours_overlap(left: &[Point2], right: &[Point2]) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }

    for left_index in 0..left.len() {
        let left_start = left[left_index];
        let left_end = left[(left_index + 1) % left.len()];

        for right_index in 0..right.len() {
            let right_start = right[right_index];
            let right_end = right[(right_index + 1) % right.len()];

            if segments_intersect(left_start, left_end, right_start, right_end) {
                return true;
            }
        }
    }

    point_in_contour(left[0], right) || point_in_contour(right[0], left)
}

fn point_in_contour(point: Point2, contour: &[Point2]) -> bool {
    let mut inside = false;

    for index in 0..contour.len() {
        let start = contour[index];
        let end = contour[(index + 1) % contour.len()];

        if point_on_segment(point, start, end) {
            return true;
        }

        let start_above = start.y > point.y;
        let end_above = end.y > point.y;
        if start_above == end_above {
            continue;
        }

        let orientation = orientation(start, end, point);
        if orientation == 0 {
            return true;
        }

        if (end.y > start.y && orientation > 0) || (end.y < start.y && orientation < 0) {
            inside = !inside;
        }
    }

    inside
}

fn segments_intersect(a_start: Point2, a_end: Point2, b_start: Point2, b_end: Point2) -> bool {
    let ab_start = orientation(a_start, a_end, b_start);
    let ab_end = orientation(a_start, a_end, b_end);
    let cd_start = orientation(b_start, b_end, a_start);
    let cd_end = orientation(b_start, b_end, a_end);

    if ab_start == 0 && point_on_segment(b_start, a_start, a_end) {
        return true;
    }
    if ab_end == 0 && point_on_segment(b_end, a_start, a_end) {
        return true;
    }
    if cd_start == 0 && point_on_segment(a_start, b_start, b_end) {
        return true;
    }
    if cd_end == 0 && point_on_segment(a_end, b_start, b_end) {
        return true;
    }

    (ab_start > 0) != (ab_end > 0) && (cd_start > 0) != (cd_end > 0)
}

fn point_on_segment(point: Point2, start: Point2, end: Point2) -> bool {
    if orientation(start, end, point) != 0 {
        return false;
    }

    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);

    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
}

fn orientation(start: Point2, end: Point2, point: Point2) -> i128 {
    let edge_x = i128::from(end.x - start.x);
    let edge_y = i128::from(end.y - start.y);
    let point_x = i128::from(point.x - start.x);
    let point_y = i128::from(point.y - start.y);
    edge_x * point_y - edge_y * point_x
}
