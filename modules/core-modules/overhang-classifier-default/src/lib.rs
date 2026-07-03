// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/SupportSpotsGenerator.cpp
// (curl-height estimation: `get_flow_width`, `estimate_curled_up_height`,
// `estimate_malformations` — the only LIVE code in that file; the
// support-point-placement code the file's name implies is dead/commented-out
// upstream and was not ported).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Consumer of the per-vertex `overhang_quartile` annotation written by the
//! upstream PrePass::OverhangAnnotation pipeline (ADR-0031, packet 106), plus
//! self-contained curled-edge slowdown (DEV-009): applies speed-factor
//! mutations to wall entities on overhangs and near previously-curled wall
//! geometry.
//!
//! Curl estimation and the cross-layer lookup that consumes it are both
//! computed transiently inside [`run_finalization`] — `curled_height` is not
//! a persisted IR/WIT field (unlike `overhang_quartile`), since nothing else
//! in this codebase needs to read it back out. See `CONTEXT.md` for the
//! **overhang quartile** / **curled height** / **artificial curl distance**
//! vocabulary.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{
    EntityMutation, FinalizationModule, FinalizationOutputBuilder, LayerCollectionView,
};

/// Core overhang classifier that applies speed-factor mutations to wall entities on overhangs.
pub struct OverhangClassifierDefault;

/// Config float for `key`, defaulting to 0.0.
fn speed(config: &ConfigView, key: &str) -> f32 {
    config.get_float(key).unwrap_or(0.0) as f32
}

/// Base wall speed for `role` (0.0 for non-wall roles).
fn base_speed(role: &ExtrusionRole, config: &ConfigView) -> f32 {
    match role {
        ExtrusionRole::OuterWall => speed(config, "outer_wall_speed"),
        ExtrusionRole::InnerWall => speed(config, "inner_wall_speed"),
        ExtrusionRole::ThinWall => speed(config, "thin_wall_speed"),
        _ => 0.0,
    }
}

/// Overhang speed for `quartile` (1..=4), 0.0 otherwise.
fn overhang_speed(quartile: u8, config: &ConfigView) -> f32 {
    match quartile {
        1 => speed(config, "overhang_1_4_speed"),
        2 => speed(config, "overhang_2_4_speed"),
        3 => speed(config, "overhang_3_4_speed"),
        4 => speed(config, "overhang_4_4_speed"),
        _ => 0.0,
    }
}

/// Line width (mm) used for both overhang-quartile bucketing and curl
/// distance synthesis. Reads `outer_wall_line_width`, falling back to
/// `line_width` (matches the resolution convention documented in
/// `crates/slicer-core/src/algos/overhang_annotation.rs`'s "Config wiring
/// note").
fn line_width(config: &ConfigView) -> f32 {
    config
        .get_float("outer_wall_line_width")
        .or_else(|| config.get_float("line_width"))
        .unwrap_or(0.0) as f32
}

/// Same 3 interior band-boundary multipliers as
/// `crates/slicer-core/src/algos/overhang_annotation.rs::BAND_BOUNDARY_MULTIPLIERS`
/// (line-width multiples `{0.5, 1.0, 1.5}` bounding quartiles 1-4). Duplicated
/// rather than imported: that constant is private to `slicer-core`, and this
/// module (a WASM guest) intentionally does not depend on `slicer-core` (it
/// carries native-only dependencies unsuitable for a wasm32 target — see
/// `slicer-ir/src/polygon_predicate.rs`'s module doc-comment for the same
/// reasoning applied elsewhere). Keep these two lists numerically identical.
const BAND_BOUNDARY_MULTIPLIERS: [f32; 3] = [0.5, 1.0, 1.5];

/// Buckets a distance (mm) into the same 1-4 quartile scale overhang uses.
/// `None` for non-positive distance or an unconfigured (zero) line width.
fn quartile_for_distance(distance: f32, line_width: f32) -> Option<u8> {
    if distance <= 0.0 || line_width <= 0.0 {
        return None;
    }
    let q = BAND_BOUNDARY_MULTIPLIERS
        .iter()
        .position(|&m| distance <= m * line_width)
        .map_or(4, |i| (i + 1) as u8);
    Some(q)
}

/// Curl-height estimation, ported from OrcaSlicer's `estimate_curled_up_height`
/// (`SupportSpotsGenerator.cpp:199-236`). `distance` is the unsigned distance
/// (mm) from this point to the nearest reference point on the layer below;
/// `curvature` is signed discrete curvature (1/mm, see [`discrete_curvature`]);
/// `prev_line_curled_height` seeds decay from the nearest reference point's
/// own curled height. Upstream's `malformation_distance_factors` (0.2, 1.1)
/// and `max_curled_height_factor` (10.0) are inlined as named locals — this
/// codebase has no equivalent tunable `Params` struct for this feature yet.
fn estimate_curled_up_height(
    distance: f32,
    curvature: f32,
    layer_height: f32,
    flow_width: f32,
    prev_line_curled_height: f32,
) -> f32 {
    const MALFORMATION_DISTANCE_FACTORS: (f32, f32) = (0.2, 1.1);
    const MAX_CURLED_HEIGHT_FACTOR: f32 = 10.0;

    let mut curled_up_height = 0.0f32;
    if distance.abs() < 3.0 * flow_width {
        curled_up_height = (prev_line_curled_height - layer_height * 0.75).max(0.0);
    }
    if distance > MALFORMATION_DISTANCE_FACTORS.0 * flow_width
        && distance < MALFORMATION_DISTANCE_FACTORS.1 * flow_width
    {
        let curling_section = distance;
        let swelling_radius = (layer_height + curling_section) / 2.0;
        curled_up_height += ((swelling_radius - layer_height) / 2.0).max(0.0);
        if curvature > 0.01 {
            let radius = 1.0 / curvature;
            let curling_t = (radius / 100.0).sqrt();
            let b = curling_t * flow_width;
            let a = curling_section;
            let c = (a * a - b * b).max(0.0).sqrt();
            curled_up_height += c;
        }
        curled_up_height = curled_up_height.min(MAX_CURLED_HEIGHT_FACTOR * layer_height);
    }
    curled_up_height
}

/// Signed discrete curvature (1/mm) at `curr`, given its polyline neighbors.
/// Not a verbatim port of OrcaSlicer's `estimate_points_properties` (that
/// function lives outside the ~150 live lines this port scoped from
/// `SupportSpotsGenerator.cpp` and has its own AABB-tree-based distance
/// annotation infrastructure) — this is a standard angle-over-arc-length
/// discrete curvature estimate, functionally equivalent for the purpose of
/// [`estimate_curled_up_height`]'s convex-turn bonus term.
fn discrete_curvature(prev: (f32, f32), curr: (f32, f32), next: (f32, f32)) -> f32 {
    let d1 = (curr.0 - prev.0, curr.1 - prev.1);
    let d2 = (next.0 - curr.0, next.1 - curr.1);
    let len1 = (d1.0 * d1.0 + d1.1 * d1.1).sqrt();
    let len2 = (d2.0 * d2.0 + d2.1 * d2.1).sqrt();
    if len1 < 1e-6 || len2 < 1e-6 {
        return 0.0;
    }
    let cross = d1.0 * d2.1 - d1.1 * d2.0;
    let dot = d1.0 * d2.0 + d1.1 * d2.1;
    let angle = cross.atan2(dot);
    angle.abs() / ((len1 + len2) / 2.0)
}

/// Nearest reference point to `(x, y)` in `points` (each `(x, y, curled_height)`),
/// returning `(distance_mm, that_point's_curled_height)`. `None` if `points`
/// is empty.
fn nearest_reference_point(points: &[(f32, f32, f32)], x: f32, y: f32) -> Option<(f32, f32)> {
    points
        .iter()
        .map(|&(px, py, ch)| (((x - px).powi(2) + (y - py).powi(2)).sqrt(), ch))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

#[slicer_module]
impl FinalizationModule for OverhangClassifierDefault {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(OverhangClassifierDefault)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Curl reuses the overhang speed table (no separate curl-specific
        // config keys — see the module doc-comment), so "all overhang bands
        // are zero" already means the whole feature family is off; skipping
        // here also avoids the wasted cross-layer point scan below.
        if (1..=4).all(|q| overhang_speed(q, config) == 0.0) {
            return Ok(());
        }
        let flow_width = line_width(config);
        let dist_limit = 10.0 * flow_width;

        // Reference geometry for curl: the previous layer's own OuterWall
        // points, each carrying its own curled_height. Empty for layer 0
        // (no layer below to reference) and stays that way until the first
        // layer with OuterWall geometry has been walked.
        let mut prev_wall_points: Vec<(f32, f32, f32)> = Vec::new();

        for (idx, layer) in layers.iter().enumerate() {
            let layer_height = if idx == 0 {
                None
            } else {
                Some((layer.z() - layers[idx - 1].z()).max(0.001))
            };

            // (1) Consumption: merge overhang_quartile with a curl-derived
            // quartile synthesized from nearby curled points on the layer
            // below, then emit one SetSpeedFactor mutation per entity.
            for entity in layer.ordered_entities() {
                let overhang_q = entity
                    .path
                    .points
                    .iter()
                    .filter_map(|p| p.overhang_quartile)
                    .max();

                let curl_q = if flow_width > 0.0 && !prev_wall_points.is_empty() {
                    // Only reachable once `prev_wall_points` has been seeded by a
                    // prior iteration (idx >= 1), where `layer_height` is always
                    // `Some`; the `unwrap_or` is a defensive fallback, not the
                    // expected path.
                    let lh = layer_height.unwrap_or(flow_width);
                    let mut max_artificial_distance = 0.0f32;
                    for p in &entity.path.points {
                        let Some((distance, curled_height)) =
                            nearest_reference_point(&prev_wall_points, p.x, p.y)
                        else {
                            continue;
                        };
                        if distance < dist_limit && curled_height > 0.0 {
                            // Ported shape from ExtrusionProcessor.hpp's
                            // artificial_distance_to_curled_lines formula.
                            let artificial = flow_width
                                * (1.0 - distance / dist_limit).powi(2)
                                * (curled_height / (lh * 10.0));
                            max_artificial_distance = max_artificial_distance.max(artificial);
                        }
                    }
                    quartile_for_distance(max_artificial_distance, flow_width)
                } else {
                    None
                };

                let Some(q) = overhang_q.max(curl_q) else {
                    continue;
                };
                let base = base_speed(&entity.role, config);
                if base <= 0.0 {
                    continue;
                }
                let mutation = EntityMutation::SetSpeedFactor(overhang_speed(q, config) / base);
                output
                    .modify_entity(layer.layer_index(), entity.entity_id, mutation)
                    .map_err(ModuleError::from_str)?;
            }

            // (2) Production: record this layer's own OuterWall points as
            // reference geometry for the NEXT layer's curl lookup. Always
            // collect positions (layer 0 included, so layer 1 has something
            // to reference); only estimate a nonzero height when a lower
            // layer exists to measure distance/decay against — matches this
            // codebase's "no previous layer ⇒ no signal" precedent already
            // used for `overhang_quartile` at layer 0.
            let mut this_layer_points: Vec<(f32, f32, f32)> = Vec::new();
            if flow_width > 0.0 {
                for entity in layer.ordered_entities() {
                    if entity.role != ExtrusionRole::OuterWall {
                        continue;
                    }
                    let pts = &entity.path.points;
                    let n = pts.len();
                    for i in 0..n {
                        let curr = (pts[i].x, pts[i].y);
                        let curled_height = match layer_height {
                            Some(lh) => {
                                let prev_pt = pts[if i == 0 { n - 1 } else { i - 1 }];
                                let next_pt = pts[(i + 1) % n];
                                let curvature = discrete_curvature(
                                    (prev_pt.x, prev_pt.y),
                                    curr,
                                    (next_pt.x, next_pt.y),
                                );
                                let (distance, prev_h) =
                                    nearest_reference_point(&prev_wall_points, curr.0, curr.1)
                                        .unwrap_or((f32::MAX, 0.0));
                                estimate_curled_up_height(
                                    distance, curvature, lh, flow_width, prev_h,
                                )
                            }
                            None => 0.0,
                        };
                        this_layer_points.push((curr.0, curr.1, curled_height));
                    }
                }
            }
            prev_wall_points = this_layer_points;
        }
        Ok(())
    }
}
