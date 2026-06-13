//! Classic perimeter generator module.
//!
//! Implements `LayerModule::run_perimeters` for the `Layer::Perimeters` stage.
//! Generates wall loops from slice contour polygons via iterative Clipper2
//! polygon insets (negative offsets).
//!
//! Per OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp process_classic().

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::{BTreeMap, HashMap};

use slicer_core::polygon_ops::{offset, OffsetJoinType};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, PaintSemantic,
    PaintValue, Point3, Point3WithWidth, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Classic perimeter generator.
///
/// Produces wall loops via iterative constant-width polygon insets.
/// Outer wall first, then inner walls, with remaining area as infill.
pub struct ClassicPerimeters {
    /// Number of wall loops to generate.
    wall_count: u32,
    /// Extrusion line width in millimeters.
    line_width: f32,
    /// Speed factor for outer walls (outer_wall_speed / BASE_SPEED).
    outer_speed_factor: f32,
    /// Speed factor for inner walls (inner_wall_speed / BASE_SPEED).
    inner_speed_factor: f32,
    /// Arc tolerance for polygon offset operations (mm).
    perimeter_arc_tolerance: f32,
}

#[slicer_module]
impl LayerModule for ClassicPerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 2, // default
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4, // default
        };

        let outer_wall_speed = match config.get("outer_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let inner_wall_speed = match config.get("inner_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let perimeter_arc_tolerance = match config.get("perimeter_arc_tolerance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0125,
        };

        Ok(Self {
            wall_count,
            line_width,
            outer_speed_factor: outer_wall_speed / BASE_SPEED,
            inner_speed_factor: inner_wall_speed / BASE_SPEED,
            perimeter_arc_tolerance,
        })
    }

    fn run_perimeters(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Group regions by object so each painted object's model perimeter is
        // traced exactly once (AC-22b bisector-edge dedup).
        let mut by_object: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, region) in regions.iter().enumerate() {
            if region.polygons().is_empty() {
                continue;
            }
            by_object
                .entry(region.object_id().clone())
                .or_default()
                .push(i);
        }

        let empty_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
            HashMap::new();

        for indices in by_object.values() {
            // A painted object exposes a shared external contour on its cells.
            let shared_boundary = indices.iter().find_map(|&i| regions[i].external_contour());

            if let Some(boundary) = shared_boundary {
                // Trace the model perimeter ONCE as the outer wall (single loop).
                if self.wall_count > 0 {
                    let z = regions[indices[0]].z();
                    self.emit_walls(boundary, z, &empty_annotations, true, false, output);
                }
                // Each cell adds only inner walls + infill from its own polygon.
                for &i in indices {
                    let region = &regions[i];
                    let polygons = region.polygons();
                    let z = region.z();
                    if self.wall_count == 0 {
                        let _ = output.set_infill_areas(polygons.to_vec());
                        continue;
                    }
                    self.emit_walls(
                        polygons,
                        z,
                        region.segment_annotations(),
                        false,
                        true,
                        output,
                    );
                }
            } else {
                // Unpainted object: full per-region emission (unchanged).
                for &i in indices {
                    let region = &regions[i];
                    let polygons = region.polygons();
                    let z = region.z();
                    if self.wall_count == 0 {
                        let _ = output.set_infill_areas(polygons.to_vec());
                        continue;
                    }
                    self.emit_walls(
                        polygons,
                        z,
                        region.segment_annotations(),
                        true,
                        true,
                        output,
                    );
                }
            }
        }

        Ok(())
    }
}

impl ClassicPerimeters {
    /// Emit wall loops (plus seam candidates and infill) for `polygons`.
    ///
    /// `emit_outer` / `emit_inner` gate which bands and the infill are produced
    /// (AC-22b): a painted object's perimeter is traced ONCE from the shared
    /// external contour (`true, false`) so the outer-wall count matches the
    /// unpainted baseline, and each colour cell adds only its inner walls + infill
    /// (`false, true`). Unpainted regions pass `true, true`.
    #[allow(clippy::too_many_arguments)]
    fn emit_walls(
        &self,
        polygons: &[ExPolygon],
        z: f32,
        segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
        emit_outer: bool,
        emit_inner: bool,
        output: &mut PerimeterOutputBuilder,
    ) {
        // Generate wall loops via iterative insets.
        let mut current_polygons = polygons.to_vec();
        let mut all_wall_polygons: Vec<(u32, Vec<ExPolygon>)> = Vec::new();

        for i in 0..self.wall_count {
            let inset_delta = if i == 0 {
                -(self.line_width / 2.0)
            } else {
                -self.line_width
            };
            let inset_result = offset(
                &current_polygons,
                inset_delta,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if inset_result.is_empty() {
                break;
            }
            all_wall_polygons.push((i, inset_result.clone()));
            current_polygons = inset_result;
        }

        for (perimeter_index, wall_polys) in &all_wall_polygons {
            let is_outer = *perimeter_index == 0;
            // AC-22b: emit only the requested bands (outer-once / inner-per-cell).
            if (is_outer && !emit_outer) || (!is_outer && !emit_inner) {
                continue;
            }
            let loop_type = if is_outer {
                LoopType::Outer
            } else {
                LoopType::Inner
            };
            let role = if is_outer {
                ExtrusionRole::OuterWall
            } else {
                ExtrusionRole::InnerWall
            };
            let speed_factor = if is_outer {
                self.outer_speed_factor
            } else {
                self.inner_speed_factor
            };

            for (poly_idx, poly) in wall_polys.iter().enumerate() {
                let points = expolygon_to_path3d(&poly.contour, z, self.line_width);
                if points.is_empty() {
                    continue;
                }
                let num_points = points.len();

                let (mut feature_flags, boundary_type) = if is_outer {
                    build_outer_wall_flags(num_points, poly_idx, segment_annotations)
                } else {
                    (
                        vec![default_feature_flags(); num_points],
                        WallBoundaryType::Interior,
                    )
                };
                slicer_sdk::mirror_first_to_last(&mut feature_flags);

                let wall = WallLoop {
                    perimeter_index: *perimeter_index,
                    loop_type,
                    path: ExtrusionPath3D {
                        points,
                        role: role.clone(),
                        speed_factor,
                    },
                    width_profile: WidthProfile {
                        widths: vec![self.line_width; num_points],
                    },
                    feature_flags,
                    boundary_type,
                };
                let _ = output.push_wall_loop(wall);
            }
        }

        // Seam candidates belong to the outer wall (the shared-perimeter pass).
        if emit_outer {
            if let Some((_, outer_polys)) = all_wall_polygons.first() {
                for poly in outer_polys {
                    generate_seam_candidates(&poly.contour, z, output);
                }
            }
        }

        // Only the inner/infill pass owns the infill region.
        if emit_inner && !current_polygons.is_empty() {
            let infill = offset(
                &current_polygons,
                -(self.line_width / 2.0),
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if !infill.is_empty() {
                let _ = output.set_infill_areas(infill);
            }
        }
    }
}

/// Build feature flags for outer wall points by propagating segment_annotations.
///
/// Reads Material and FuzzySkin semantics from `segment_annotations` for the given
/// polygon index. Sets `tool_index` from Material ToolIndex values, `fuzzy_skin`
/// from FuzzySkin Flag values. Detects adjacent material changes and returns
/// `WallBoundaryType::MaterialBoundary` when different tool indices are adjacent.
fn build_outer_wall_flags(
    num_points: usize,
    poly_idx: usize,
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
) -> (Vec<WallFeatureFlags>, WallBoundaryType) {
    let mut flags = vec![default_feature_flags(); num_points];

    // Extract per-point Material paint values for this polygon
    let material_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::Material)
        .and_then(|per_poly| per_poly.get(poly_idx));

    // Extract per-point FuzzySkin paint values for this polygon
    let fuzzy_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::FuzzySkin)
        .and_then(|per_poly| per_poly.get(poly_idx));

    // Propagate Material -> tool_index
    if let Some(mat_vals) = material_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::ToolIndex(tool))) = mat_vals.get(i) {
                flag.tool_index = Some(*tool);
            }
        }
    }

    // Propagate FuzzySkin -> fuzzy_skin
    if let Some(fuzzy_vals) = fuzzy_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::Flag(true))) = fuzzy_vals.get(i) {
                flag.fuzzy_skin = true;
            }
        }
    }

    // Detect material boundary: adjacent points with different tool_index
    let has_material_boundary = if let Some(mat_vals) = material_values {
        has_adjacent_material_change(mat_vals)
    } else {
        false
    };

    let boundary_type = if has_material_boundary {
        // Find the first adjacent tool index that differs for the boundary metadata
        let adjacent_tool = find_adjacent_tool(material_values.unwrap());
        WallBoundaryType::MaterialBoundary { adjacent_tool }
    } else {
        WallBoundaryType::ExteriorSurface
    };

    (flags, boundary_type)
}

/// Check if adjacent points in a material paint list have different tool indices.
fn has_adjacent_material_change(mat_vals: &[Option<PaintValue>]) -> bool {
    let n = mat_vals.len();
    if n < 2 {
        return false;
    }
    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);
        if tool_a != tool_b {
            return true;
        }
    }
    false
}

/// Find the adjacent tool index from the first material boundary transition.
fn find_adjacent_tool(mat_vals: &[Option<PaintValue>]) -> u32 {
    let n = mat_vals.len();
    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);
        if tool_a != tool_b {
            // Return the "other" tool — prefer the non-None one
            return tool_b.or(tool_a).unwrap_or(0);
        }
    }
    0
}

/// Extract tool index from a PaintValue, if it is a ToolIndex variant.
fn extract_tool_index(val: &Option<PaintValue>) -> Option<u32> {
    match val {
        Some(PaintValue::ToolIndex(t)) => Some(*t),
        _ => None,
    }
}

/// Convert an ExPolygon contour to a Vec<Point3WithWidth> at the given Z and width.
///
/// Converts from scaled i64 coordinates to f32 mm. The returned Vec has N+1
/// entries for an N-vertex polygon: the first point is repeated at the end so
/// the path is a closed loop in OrcaSlicer convention
/// (`ExtrusionPath::is_closed()` at `ExtrusionEntity.hpp:269`). Downstream
/// consumers (seam-placer, fuzzy-skin, G-code emitter) rely on this so the
/// final closing edge is processed exactly like every other wall segment.
fn expolygon_to_path3d(contour: &slicer_ir::Polygon, z: f32, width: f32) -> Vec<Point3WithWidth> {
    let mut pts: Vec<Point3WithWidth> = contour
        .points
        .iter()
        .map(|p| Point3WithWidth {
            x: slicer_ir::units_to_mm(p.x),
            y: slicer_ir::units_to_mm(p.y),
            z,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        })
        .collect();
    slicer_sdk::close_loop(&mut pts);
    pts
}

/// Create default WallFeatureFlags (no paint, no bridge, no thin wall).
fn default_feature_flags() -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

/// Generate seam candidates at sharp corners of the outer wall path.
///
/// All corners with a non-trivial turn angle are candidates. Concave corners
/// receive a higher score (seam is less visible there), convex corners get a
/// lower but positive score.
fn generate_seam_candidates(
    contour: &slicer_ir::Polygon,
    z: f32,
    output: &mut PerimeterOutputBuilder,
) {
    let pts = &contour.points;
    let n = pts.len();
    if n < 3 {
        return;
    }

    // Determine winding via signed area
    let mut signed_area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += (pts[i].x as i128) * (pts[j].y as i128);
        signed_area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    let is_ccw = signed_area > 0;

    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;

        let dx1 = pts[i].x - pts[prev].x;
        let dy1 = pts[i].y - pts[prev].y;
        let dx2 = pts[next].x - pts[i].x;
        let dy2 = pts[next].y - pts[i].y;

        let cross = dx1 * dy2 - dy1 * dx2;
        if cross == 0 {
            // Collinear — not a real corner
            continue;
        }

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        // Concave corners score higher (seam hides better)
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0 // concave bias
        } else {
            sin_angle * 0.5 // convex, lower score
        };

        let pos = Point3 {
            x: slicer_ir::units_to_mm(pts[i].x),
            y: slicer_ir::units_to_mm(pts[i].y),
            z,
        };
        let _ = output.push_seam_candidate(pos, score);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let module = ClassicPerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 2);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
