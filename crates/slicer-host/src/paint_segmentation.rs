//! Paint segmentation execution contract.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use rayon::prelude::*;
use slicer_core::{slice_mesh_ex, union};
use slicer_ir::slice_ir::BoundingBox2;
use slicer_ir::{
    ConfigValue, ExPolygon, LayerPaintMap, LayerPlanIR, MeshIR, PaintRegionIR, PaintSemantic,
    PaintValue, Point2, Point3, Polygon, SemanticRegion, SurfaceClassificationIR,
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

impl std::fmt::Display for PaintSegmentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSurfaceObject { object_id } => {
                write!(f, "surface classification data missing for object '{object_id}'")
            }
            Self::MissingLayerParticipation { object_id } => {
                write!(f, "layer participation data missing for object '{object_id}'")
            }
            Self::MalformedFacetValues {
                object_id,
                layer_index,
                expected_facets,
                actual_facet_values,
            } => write!(
                f,
                "object '{object_id}' paint layer {layer_index}: expected {expected_facets} facet values, got {actual_facet_values}"
            ),
            Self::DeterministicConflict {
                global_layer_index,
                object_id,
                semantic,
                paint_order,
            } => write!(
                f,
                "deterministic conflict at layer {global_layer_index}, object '{object_id}', semantic {semantic:?}, paint order {paint_order}"
            ),
        }
    }
}

impl std::error::Error for PaintSegmentationError {}

/// A single facet-paint entry collected during execution or WIT harvesting.
/// These are the raw input to [`group_and_union_paint_regions`].
#[derive(Debug, Clone)]
pub struct PaintFacetEntry {
    /// Global layer index this entry belongs to.
    pub layer_index: u32,
    /// Object identifier.
    pub object_id: String,
    /// Paint semantic family.
    pub semantic: PaintSemantic,
    /// Paint value payload.
    pub value: PaintValue,
    /// Render order (lower = earlier).
    pub paint_order: u64,
    /// Polygons contributed by this entry (typically 1 per facet).
    pub polygons: Vec<ExPolygon>,
}

/// Hashable wrapper for `PaintValue` so it can be used as a HashMap key.
#[derive(Clone, Hash, Eq, PartialEq)]
enum HashablePaintValue {
    Flag(bool),
    Scalar(u32),
    ToolIndex(u32),
    Custom(String),
}

impl From<&PaintValue> for HashablePaintValue {
    fn from(v: &PaintValue) -> Self {
        match v {
            PaintValue::Flag(b) => HashablePaintValue::Flag(*b),
            PaintValue::Scalar(f) => HashablePaintValue::Scalar(f.to_bits()),
            PaintValue::ToolIndex(n) => HashablePaintValue::ToolIndex(*n),
            PaintValue::Custom(s) => HashablePaintValue::Custom(s.clone()),
        }
    }
}

/// Group facet entries by `(layer_index, object_id, PaintSemantic, PaintValue)`,
/// union polygons per group, compute AABB, and build a sorted `PaintRegionIR`.
pub fn group_and_union_paint_regions(
    entries: Vec<PaintFacetEntry>,
    union_paint_regions_at_harvest: bool,
) -> PaintRegionIR {
    let mut groups: HashMap<
        (u32, String, PaintSemantic, HashablePaintValue),
        Vec<(u64, Vec<ExPolygon>)>,
    > = HashMap::new();

    for entry in entries {
        let key = (
            entry.layer_index,
            entry.object_id.clone(),
            entry.semantic.clone(),
            HashablePaintValue::from(&entry.value),
        );
        groups
            .entry(key)
            .or_default()
            .push((entry.paint_order, entry.polygons));
    }

    let mut per_layer: HashMap<u32, LayerPaintMap> = HashMap::new();

    // Process groups in parallel — each group's union can be expensive.
    let group_vec: Vec<_> = groups.into_iter().collect();
    let results: Vec<(u32, PaintSemantic, SemanticRegion)> = group_vec
        .into_par_iter()
        .map(
            |((layer_index, object_id, semantic, hashable_value), group_entries)| {
                let value = match hashable_value {
                    HashablePaintValue::Flag(b) => PaintValue::Flag(b),
                    HashablePaintValue::Scalar(bits) => PaintValue::Scalar(f32::from_bits(bits)),
                    HashablePaintValue::ToolIndex(n) => PaintValue::ToolIndex(n),
                    HashablePaintValue::Custom(s) => PaintValue::Custom(s),
                };

                let all_polygons: Vec<ExPolygon> = group_entries
                    .iter()
                    .flat_map(|(_, polys)| polys.clone())
                    .collect();
                let paint_order = group_entries.iter().map(|(po, _)| *po).min().unwrap_or(0);

                let unioned = if union_paint_regions_at_harvest && all_polygons.len() > 1 {
                    slicer_core::union(&all_polygons, &[])
                } else {
                    all_polygons
                };

                let (min_x, min_y, max_x, max_y) =
                    unioned.iter().flat_map(|ep| &ep.contour.points).fold(
                        (i64::MAX, i64::MAX, i64::MIN, i64::MIN),
                        |(min_x, min_y, max_x, max_y), pt| {
                            (
                                min_x.min(pt.x),
                                min_y.min(pt.y),
                                max_x.max(pt.x),
                                max_y.max(pt.y),
                            )
                        },
                    );
                let aabb = if min_x <= max_x && min_y <= max_y {
                    Some(BoundingBox2 {
                        min: Point2 { x: min_x, y: min_y },
                        max: Point2 { x: max_x, y: max_y },
                    })
                } else {
                    None
                };

                (
                    layer_index,
                    semantic,
                    SemanticRegion {
                        object_id,
                        polygons: unioned,
                        value,
                        paint_order,
                        aabb,
                    },
                )
            },
        )
        .collect();

    for (layer_index, semantic, region) in results {
        let layer = per_layer
            .entry(layer_index)
            .or_insert_with(|| LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::new(),
            });
        layer
            .semantic_regions
            .entry(semantic)
            .or_default()
            .push(region);
    }

    for layer_map in per_layer.values_mut() {
        for regions in layer_map.semantic_regions.values_mut() {
            regions.sort_by(|a, b| {
                b.paint_order
                    .cmp(&a.paint_order)
                    .then_with(|| a.object_id.cmp(&b.object_id))
                    .then_with(|| match (&a.value, &b.value) {
                        (PaintValue::Flag(la), PaintValue::Flag(rb)) => la.cmp(rb),
                        (PaintValue::Flag(_), _) => Ordering::Less,
                        (_, PaintValue::Flag(_)) => Ordering::Greater,
                        (PaintValue::Scalar(la), PaintValue::Scalar(rb)) => la.total_cmp(rb),
                        (PaintValue::Scalar(_), PaintValue::ToolIndex(_)) => Ordering::Less,
                        (PaintValue::ToolIndex(_), PaintValue::Scalar(_)) => Ordering::Greater,
                        (PaintValue::ToolIndex(la), PaintValue::ToolIndex(rb)) => la.cmp(rb),
                        (PaintValue::Custom(la), PaintValue::Custom(rb)) => la.cmp(rb),
                        (PaintValue::Custom(_), _) => Ordering::Greater,
                        (_, PaintValue::Custom(_)) => Ordering::Less,
                    })
            });
        }
    }

    PaintRegionIR {
        per_layer,
        ..Default::default()
    }
}

/// Convert segmented whole-triangle paint assignments into immutable per-layer paint regions.
pub fn execute_paint_segmentation(
    mesh_ir: Arc<MeshIR>,
    surface_classification_ir: Arc<SurfaceClassificationIR>,
    layer_plan_ir: Arc<LayerPlanIR>,
    union_paint_regions_at_harvest: bool,
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

    // Aggregate facet entries by (layer_index, object_id, semantic, value, paint_order)
    // so we don't create O(facets × layers) individual entries.
    let mut entry_accumulator: HashMap<
        (u32, String, PaintSemantic, HashablePaintValue, u64),
        PaintFacetEntry,
    > = HashMap::new();

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
                    // Conflict detection: check accumulator for same
                    // (layer, object_id, semantic, paint_order) with a
                    // different value whose polygons overlap this facet.
                    if matches!(layer.semantic, PaintSemantic::Custom(_)) {
                        let conflict_value = HashablePaintValue::from(value);
                        for ((l, oid, sem, hv, po), existing) in entry_accumulator.iter() {
                            if *l == object_layer.global_layer_index
                                && *oid == object.id
                                && *sem == layer.semantic
                                && *po == paint_order as u64
                                && *hv != conflict_value
                                && existing
                                    .polygons
                                    .iter()
                                    .any(|ep| polygons_overlap(ep, &polygon))
                            {
                                return Err(PaintSegmentationError::DeterministicConflict {
                                    global_layer_index: object_layer.global_layer_index,
                                    object_id: object.id.clone(),
                                    semantic: layer.semantic.clone(),
                                    paint_order: paint_order as u64,
                                });
                            }
                        }
                    }

                    let acc_key = (
                        object_layer.global_layer_index,
                        object.id.clone(),
                        layer.semantic.clone(),
                        HashablePaintValue::from(value),
                        paint_order as u64,
                    );
                    entry_accumulator
                        .entry(acc_key)
                        .or_insert_with(|| PaintFacetEntry {
                            layer_index: object_layer.global_layer_index,
                            object_id: object.id.clone(),
                            semantic: layer.semantic.clone(),
                            value: value.clone(),
                            paint_order: paint_order as u64,
                            polygons: Vec::new(),
                        })
                        .polygons
                        .push(polygon.clone());
                }
            }
        }
    }

    // Group, union, and compute AABBs via shared function
    let entries: Vec<PaintFacetEntry> = entry_accumulator.into_values().collect();
    let facet_ir = group_and_union_paint_regions(entries, union_paint_regions_at_harvest);
    for (layer_index, layer_map) in facet_ir.per_layer {
        per_layer.insert(layer_index, layer_map);
    }

    // Emit synthetic PaintRegionIR entries for support_enforcer and support_blocker
    // modifier volumes (Packet 56c). Projects each qualifying modifier mesh at every
    // global layer Z and inserts SemanticRegion entries via polygon union.
    let layer_zs: Vec<f32> = layer_plan_ir.global_layers.iter().map(|l| l.z).collect();
    for object in &mesh_ir.objects {
        for mv in &object.modifier_volumes {
            let subtype = match mv.config_delta.fields.get("subtype") {
                Some(ConfigValue::String(s)) => s.as_str(),
                _ => continue,
            };
            let semantic = match subtype {
                "support_enforcer" => PaintSemantic::SupportEnforcer,
                "support_blocker" => PaintSemantic::SupportBlocker,
                _ => continue,
            };
            if mv.mesh.vertices.is_empty() {
                continue; // degenerate mesh — emit nothing
            }
            let projections = slice_mesh_ex(&mv.mesh, &layer_zs);
            for (layer, polys) in layer_plan_ir.global_layers.iter().zip(projections) {
                if polys.is_empty() {
                    continue;
                }
                let layer_map = per_layer
                    .entry(layer.index)
                    .or_insert_with(|| LayerPaintMap {
                        global_layer_index: layer.index,
                        semantic_regions: HashMap::new(),
                    });
                let entry = layer_map
                    .semantic_regions
                    .entry(semantic.clone())
                    .or_insert_with(Vec::new);
                if entry.is_empty() {
                    entry.push(SemanticRegion {
                        object_id: object.id.clone(),
                        polygons: polys,
                        value: PaintValue::Flag(true),
                        paint_order: 0,
                        aabb: None,
                    });
                } else {
                    // Union new polygons into the existing region for this object.
                    let existing = entry.first_mut().unwrap();
                    existing.polygons = union(&existing.polygons, &polys);
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

    let w = if transformed_w == 0.0 {
        1.0
    } else {
        transformed_w
    };
    Point3 {
        x: (transformed_x / w) as f32,
        y: (transformed_y / w) as f32,
        z: (transformed_z / w) as f32,
    }
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
