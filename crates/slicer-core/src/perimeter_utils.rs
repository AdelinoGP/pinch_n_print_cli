// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PerimeterGenerator.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Shared perimeter-generation helpers used by both classic and Arachne perimeter
//! modules.

use std::collections::HashMap;

use slicer_ir::{
    MaterialBoundarySegment, PaintSemantic, PaintValue, Point3, Point3WithWidth, WallBoundaryType,
    WallFeatureFlags,
};

/// Default base speed used for normalizing speed factors (mm/s).
pub const BASE_SPEED: f32 = 50.0;

/// Build feature flags for outer wall points by propagating segment_annotations.
///
/// Reads Material and FuzzySkin semantics from `segment_annotations` for the given
/// polygon index. Sets `tool_index` from Material ToolIndex values, `fuzzy_skin`
/// from FuzzySkin Flag values. Detects adjacent material changes and returns
/// `WallBoundaryType::MaterialBoundary` with a segment for each transition.
pub fn build_outer_wall_flags(
    num_points: usize,
    poly_idx: usize,
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
) -> (Vec<WallFeatureFlags>, WallBoundaryType) {
    let mut flags = vec![default_feature_flags(); num_points];

    let material_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::Material)
        .and_then(|per_poly| per_poly.get(poly_idx));

    let fuzzy_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::FuzzySkin)
        .and_then(|per_poly| per_poly.get(poly_idx));

    if let Some(mat_vals) = material_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::ToolIndex(tool))) = mat_vals.get(i) {
                flag.tool_index = Some(*tool);
            }
        }
    }

    if let Some(fuzzy_vals) = fuzzy_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::Flag(true))) = fuzzy_vals.get(i) {
                flag.fuzzy_skin = true;
            }
        }
    }

    let boundary_type = match material_values {
        Some(mat_vals) => {
            let transitions = find_all_transitions(mat_vals);
            if transitions.is_empty() {
                WallBoundaryType::ExteriorSurface
            } else {
                WallBoundaryType::MaterialBoundary {
                    segments: transitions,
                }
            }
        }
        None => WallBoundaryType::ExteriorSurface,
    };

    (flags, boundary_type)
}

/// Find all material boundary transitions on a polygon contour.
///
/// Walks the circular material paint list and emits a `MaterialBoundarySegment`
/// for each edge where two adjacent points have different tool indices.
/// Each segment records the point range (half-open `[i, i+1)`) and the
/// near/far tool indices.
pub fn find_all_transitions(mat_vals: &[Option<PaintValue>]) -> Vec<MaterialBoundarySegment> {
    let n = mat_vals.len();
    if n < 2 {
        return Vec::new();
    }

    let mut segments = Vec::new();

    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);

        if tool_a != tool_b {
            segments.push(MaterialBoundarySegment {
                point_range: i as u32..(i as u32 + 1),
                near_tool: tool_a,
                far_tool: tool_b,
            });
        }
    }

    segments
}

/// Extract tool index from a PaintValue, if it is a ToolIndex variant.
pub fn extract_tool_index(val: &Option<PaintValue>) -> Option<u32> {
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
pub fn expolygon_to_path3d(
    contour: &slicer_ir::Polygon,
    z: f32,
    width: f32,
) -> Vec<Point3WithWidth> {
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
    close_loop(&mut pts);
    pts
}

/// Create default WallFeatureFlags (no paint, no bridge, no thin wall).
pub fn default_feature_flags() -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

/// A seam candidate: a 3D position and a score (higher = better).
pub struct SeamCandidate {
    /// Position in mm.
    pub position: Point3,
    /// Score (higher is preferred).
    pub score: f32,
}

/// Generate seam candidates at sharp corners of the outer wall path.
///
/// All corners with a non-trivial turn angle are candidates. Concave corners
/// receive a higher score (seam is less visible there), convex corners get a
/// lower but positive score.
pub fn generate_seam_candidates(contour: &slicer_ir::Polygon, z: f32) -> Vec<SeamCandidate> {
    let pts = &contour.points;
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }

    let mut signed_area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += (pts[i].x as i128) * (pts[j].y as i128);
        signed_area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    let is_ccw = signed_area > 0;

    let mut candidates = Vec::new();

    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;

        let dx1 = pts[i].x - pts[prev].x;
        let dy1 = pts[i].y - pts[prev].y;
        let dx2 = pts[next].x - pts[i].x;
        let dy2 = pts[next].y - pts[i].y;

        let cross = dx1 * dy2 - dy1 * dx2;
        if cross == 0 {
            continue;
        }

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0
        } else {
            sin_angle * 0.5
        };

        let position = Point3 {
            x: slicer_ir::units_to_mm(pts[i].x),
            y: slicer_ir::units_to_mm(pts[i].y),
            z,
        };
        candidates.push(SeamCandidate { position, score });
    }

    candidates
}

fn close_loop<T: Clone>(items: &mut Vec<T>) {
    if let Some(first) = items.first().cloned() {
        items.push(first);
    }
}
