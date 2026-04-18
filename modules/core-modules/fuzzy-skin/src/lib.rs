//! Fuzzy skin module.
//!
//! Implements `LayerModule::run_wall_postprocess` for the `Layer::PerimetersPostProcess` stage.
//! Selectively displaces outer wall perimeter points to create a rough textured surface.
//! Inner walls are never perturbed. When `apply-to-all` is true, all outer wall segments
//! are perturbed regardless of per-vertex `fuzzy_skin` feature flags. Unflagged segments on
//! mixed loops keep their original XY geometry.
//!
//! Core algorithm adapted from OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp
//! (fuzzy_polyline at line 85).

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, LoopType, Point3WithWidth, WallFeatureFlags,
    WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

/// Fuzzy skin module.
///
/// Selectively perturbs outer wall perimeter points to create a rough textured
/// surface finish. Uses a seeded RNG for deterministic output.
pub struct FuzzySkinModule {
    /// Displacement magnitude in mm.
    thickness: f32,
    /// Target distance between perturbation points in mm.
    point_distance: f32,
    /// If true, perturb all outer walls regardless of feature flags.
    apply_to_all: bool,
}

#[slicer_module]
impl LayerModule for FuzzySkinModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let thickness = match config.get("thickness") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.3,
        };
        let point_distance = match config.get("point-distance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.8,
        };
        let apply_to_all = match config.get("apply-to-all") {
            Some(ConfigValue::Bool(v)) => *v,
            _ => false,
        };

        Ok(Self {
            thickness,
            point_distance,
            apply_to_all,
        })
    }

    fn run_wall_postprocess(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for region in regions {
            for (wall_index, wall) in region.wall_loops().iter().enumerate() {
                // Only perturb outer walls
                if wall.loop_type != LoopType::Outer {
                    output
                        .push_wall_loop(wall.clone())
                        .map_err(|e| ModuleError::non_fatal(1, e))?;
                    continue;
                }

                let should_apply =
                    self.apply_to_all || wall.feature_flags.iter().any(|f| f.fuzzy_skin);

                if !should_apply {
                    output
                        .push_wall_loop(wall.clone())
                        .map_err(|e| ModuleError::non_fatal(2, e))?;
                    continue;
                }

                let (new_points, new_flags, new_widths) = apply_fuzzy_skin(
                    &wall.path,
                    &wall.feature_flags,
                    &wall.width_profile,
                    self.apply_to_all,
                    self.thickness,
                    self.point_distance,
                    layer_index,
                    wall_index as u32,
                );

                let fuzzed = WallLoop {
                    perimeter_index: wall.perimeter_index,
                    loop_type: wall.loop_type,
                    path: ExtrusionPath3D {
                        points: new_points,
                        role: wall.path.role.clone(),
                        speed_factor: wall.path.speed_factor,
                    },
                    width_profile: WidthProfile { widths: new_widths },
                    feature_flags: new_flags,
                    boundary_type: wall.boundary_type,
                };

                output
                    .push_wall_loop(fuzzed)
                    .map_err(|e| ModuleError::non_fatal(3, e))?;
            }
        }
        Ok(())
    }
}

/// Simple deterministic PRNG (xorshift32) for reproducible perturbation.
struct Rng {
    state: u32,
}

impl Rng {
    /// Create a new seeded RNG.
    fn new(seed: u32) -> Self {
        // Ensure non-zero state
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Generate a random f32 in [-1.0, 1.0].
    fn next_f32(&mut self) -> f32 {
        // xorshift32
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        // Map to [-1.0, 1.0]
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Apply fuzzy skin perturbation to a wall path.
///
/// Walks each segment of the path and, for flagged spans only, inserts
/// perpendicular XY-displaced subdivision points. Unflagged spans are copied
/// through unchanged so mixed loops retain flat regions.
///
/// Adapted from OrcaSlicer fuzzy_polyline (FuzzySkin.cpp line 85):
/// - min_dist = point_distance * 3/4
/// - range = point_distance / 2
/// - For determinism we use a fixed factor (1.0) for the distance jitter
///
/// Returns (new_points, new_flags, new_widths) all with matching lengths.
#[allow(clippy::too_many_arguments)]
fn apply_fuzzy_skin(
    path: &ExtrusionPath3D,
    feature_flags: &[WallFeatureFlags],
    width_profile: &WidthProfile,
    apply_to_all: bool,
    thickness: f32,
    point_distance: f32,
    layer_index: u32,
    wall_index: u32,
) -> (Vec<Point3WithWidth>, Vec<WallFeatureFlags>, Vec<f32>) {
    let points = &path.points;
    if points.len() < 2
        || !thickness.is_finite()
        || !point_distance.is_finite()
        || thickness <= 0.0
        || point_distance <= 0.0
    {
        return (
            points.to_vec(),
            feature_flags.to_vec(),
            width_profile.widths.clone(),
        );
    }

    // Seed from layer_index + wall_index for determinism
    let seed = layer_index
        .wrapping_mul(65537)
        .wrapping_add(wall_index.wrapping_mul(257))
        .wrapping_add(1);
    let mut rng = Rng::new(seed);

    let min_dist = point_distance * 0.75;
    let range = point_distance * 0.5;

    let mut out_points: Vec<Point3WithWidth> = vec![points[0]];
    let mut out_flags: Vec<WallFeatureFlags> = vec![flag_for_index(feature_flags, 0)];
    let mut out_widths: Vec<f32> = vec![width_for_index(&width_profile.widths, 0)];

    for seg_idx in 0..points.len() - 1 {
        let p0 = &points[seg_idx];
        let p1 = &points[seg_idx + 1];
        let seg_flag = flag_for_index(feature_flags, seg_idx);
        let seg_width = width_for_index(&width_profile.widths, seg_idx);
        let next_flag = flag_for_index(feature_flags, seg_idx + 1);
        let next_width = width_for_index(&width_profile.widths, seg_idx + 1);

        // Should this segment be fuzzed?
        let should_fuzz = apply_to_all || seg_flag.fuzzy_skin;

        if should_fuzz {
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let seg_len = (dx * dx + dy * dy).sqrt();

            if seg_len >= 1e-8 {
                let perp_x = -dy / seg_len;
                let perp_y = dx / seg_len;
                let mut emitted_sample = false;
                let mut dist = if min_dist < seg_len {
                    min_dist
                } else {
                    seg_len * 0.5
                };

                while dist < seg_len {
                    let t = dist / seg_len;
                    let base_x = p0.x + dx * t;
                    let base_y = p0.y + dy * t;
                    let base_z = p0.z + (p1.z - p0.z) * t;
                    let displacement = rng.next_f32() * thickness;

                    out_points.push(Point3WithWidth {
                        x: base_x + perp_x * displacement,
                        y: base_y + perp_y * displacement,
                        z: base_z,
                        width: p0.width + (p1.width - p0.width) * t,
                        flow_factor: p0.flow_factor + (p1.flow_factor - p0.flow_factor) * t,
                    });
                    out_flags.push(seg_flag.clone());
                    out_widths.push(seg_width);
                    emitted_sample = true;

                    dist += min_dist + rng.next_f32().abs() * range;
                }

                if !emitted_sample {
                    let t = 0.5;
                    let base_x = p0.x + dx * t;
                    let base_y = p0.y + dy * t;
                    let base_z = p0.z + (p1.z - p0.z) * t;
                    let displacement = rng.next_f32() * thickness;

                    out_points.push(Point3WithWidth {
                        x: base_x + perp_x * displacement,
                        y: base_y + perp_y * displacement,
                        z: base_z,
                        width: p0.width + (p1.width - p0.width) * t,
                        flow_factor: p0.flow_factor + (p1.flow_factor - p0.flow_factor) * t,
                    });
                    out_flags.push(seg_flag.clone());
                    out_widths.push(seg_width);
                }
            }
        }

        out_points.push(*p1);
        out_flags.push(next_flag);
        out_widths.push(next_width);
    }

    (out_points, out_flags, out_widths)
}

fn flag_for_index(feature_flags: &[WallFeatureFlags], idx: usize) -> WallFeatureFlags {
    feature_flags
        .get(idx)
        .cloned()
        .or_else(|| feature_flags.last().cloned())
        .unwrap_or_else(|| WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        })
}

fn width_for_index(widths: &[f32], idx: usize) -> f32 {
    widths
        .get(idx)
        .copied()
        .or_else(|| widths.last().copied())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_deterministic() {
        let mut r1 = Rng::new(42);
        let mut r2 = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(r1.next_f32().to_bits(), r2.next_f32().to_bits());
        }
    }

    #[test]
    fn rng_range() {
        let mut r = Rng::new(123);
        for _ in 0..1000 {
            let v = r.next_f32();
            assert!(v >= -1.0 && v <= 1.0, "RNG out of range: {v}");
        }
    }
}
