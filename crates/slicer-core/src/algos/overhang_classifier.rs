//! Overhang-quartile classifier.
//!
//! [`classify_layers`] walks a slice of [`LayerCollectionIR`] in order and
//! annotates every wall-family point in layer `i` with an overhang quartile
//! (Q1–Q4) derived from its signed distance to the previous layer's wall
//! geometry.

use crate::aabb_lines_2d::LinesDistancer2D;
use slicer_ir::{ExtrusionRole, LayerCollectionIR};

use slicer_ir::FeedrateConfig;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Classifies overhang quartiles for every wall-family point across `layers`.
///
/// **Short-circuit**: when all four overhang speeds in `feedrate_config` are
/// exactly `0.0`, returns immediately without touching any point.
///
/// Layer 0 is never classified (no previous layer exists).  Non-wall roles
/// (`SparseInfill`, bridging, …) are left with `overhang_quartile = None`.
pub fn classify_layers(layers: &mut [LayerCollectionIR], feedrate_config: &FeedrateConfig) {
    if feedrate_config.overhang_1_4_speed == 0.0
        && feedrate_config.overhang_2_4_speed == 0.0
        && feedrate_config.overhang_3_4_speed == 0.0
        && feedrate_config.overhang_4_4_speed == 0.0
    {
        return;
    }

    for i in 1..layers.len() {
        let (prev_half, cur_half) = layers.split_at_mut(i);
        let prev_layer = &prev_half[i - 1];
        let cur_layer = &mut cur_half[0];

        let (segments, polygons) = build_prev_geometry(prev_layer);

        if segments.is_empty() {
            continue;
        }

        let distancer = LinesDistancer2D::new(segments);

        classify_layer(cur_layer, &distancer, &polygons);
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Returns true when `role` belongs to the wall family.
#[inline]
fn is_wall_role(role: &ExtrusionRole) -> bool {
    matches!(
        role,
        ExtrusionRole::OuterWall | ExtrusionRole::InnerWall | ExtrusionRole::ThinWall
    )
}

/// Builds the segment list and polygon list from a layer's wall-family paths.
fn build_prev_geometry(
    layer: &LayerCollectionIR,
) -> (Vec<([f32; 2], [f32; 2])>, Vec<Vec<[f32; 2]>>) {
    let mut segments: Vec<([f32; 2], [f32; 2])> = Vec::new();
    let mut polygons: Vec<Vec<[f32; 2]>> = Vec::new();

    for entity in &layer.ordered_entities {
        if !is_wall_role(&entity.role) {
            continue;
        }

        let pts = &entity.path.points;
        if pts.len() < 2 {
            continue;
        }

        let poly: Vec<[f32; 2]> = pts.iter().map(|p| [p.x, p.y]).collect();

        for w in pts.windows(2) {
            segments.push(([w[0].x, w[0].y], [w[1].x, w[1].y]));
        }
        let last = pts.last().unwrap();
        let first = &pts[0];
        segments.push(([last.x, last.y], [first.x, first.y]));

        polygons.push(poly);
    }

    (segments, polygons)
}

/// Classifies every wall-family point in `layer` using `distancer` and `polygons`.
fn classify_layer(
    layer: &mut LayerCollectionIR,
    distancer: &LinesDistancer2D,
    polygons: &[Vec<[f32; 2]>],
) {
    for entity in &mut layer.ordered_entities {
        if !is_wall_role(&entity.role) {
            continue;
        }

        for pt in &mut entity.path.points {
            let sd = distancer.signed_distance([pt.x, pt.y], polygons);
            let w = pt.width;

            let q: u8 = if sd > 0.0 {
                4
            } else if sd > -0.25 * w {
                3
            } else if sd > -0.5 * w {
                2
            } else {
                1
            };

            debug_assert!((1..=4).contains(&q), "quartile out of range: {q}");
            pt.overhang_quartile = Some(q);
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ExtrusionPath3D, LayerCollectionIR, PrintEntity, RegionKey};

    fn zero_config() -> FeedrateConfig {
        FeedrateConfig {
            overhang_1_4_speed: 0.0,
            overhang_2_4_speed: 0.0,
            overhang_3_4_speed: 0.0,
            overhang_4_4_speed: 0.0,
            ..FeedrateConfig::default()
        }
    }

    fn active_config() -> FeedrateConfig {
        FeedrateConfig {
            overhang_1_4_speed: 10.0,
            overhang_2_4_speed: 20.0,
            overhang_3_4_speed: 30.0,
            overhang_4_4_speed: 40.0,
            ..FeedrateConfig::default()
        }
    }

    fn make_point(x: f32, y: f32) -> slicer_ir::Point3WithWidth {
        slicer_ir::Point3WithWidth {
            x,
            y,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }
    }

    fn make_entity(role: ExtrusionRole, pts: Vec<slicer_ir::Point3WithWidth>) -> PrintEntity {
        PrintEntity {
            entity_id: 1,
            path: ExtrusionPath3D {
                points: pts,
                role: role.clone(),
                speed_factor: 1.0,
            },
            role,
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "obj0".to_string(),
                region_id: 0,
            },
            topo_order: 0,
        }
    }

    fn empty_layer(global_layer_index: u32) -> LayerCollectionIR {
        LayerCollectionIR {
            global_layer_index,
            z: global_layer_index as f32 * 0.2,
            ..Default::default()
        }
    }

    fn layer_with_entity(
        global_layer_index: u32,
        role: ExtrusionRole,
        pts: Vec<slicer_ir::Point3WithWidth>,
    ) -> LayerCollectionIR {
        let mut l = empty_layer(global_layer_index);
        l.ordered_entities.push(make_entity(role, pts));
        l
    }

    fn square_wall_layer(idx: u32) -> LayerCollectionIR {
        let pts = vec![
            make_point(0.0, 0.0),
            make_point(10.0, 0.0),
            make_point(10.0, 10.0),
            make_point(0.0, 10.0),
        ];
        layer_with_entity(idx, ExtrusionRole::OuterWall, pts)
    }

    #[test]
    fn short_circuit_on_zero_config() {
        let mut layers = vec![
            square_wall_layer(0),
            layer_with_entity(
                1,
                ExtrusionRole::OuterWall,
                vec![make_point(5.0, 5.0), make_point(6.0, 6.0)],
            ),
        ];

        classify_layers(&mut layers, &zero_config());

        for layer in &layers {
            for entity in &layer.ordered_entities {
                for pt in &entity.path.points {
                    assert_eq!(
                        pt.overhang_quartile, None,
                        "short-circuit: expected None, got {:?}",
                        pt.overhang_quartile
                    );
                }
            }
        }
    }

    #[test]
    fn first_layer_all_none() {
        let mut layers = vec![layer_with_entity(
            0,
            ExtrusionRole::OuterWall,
            vec![make_point(5.0, 5.0)],
        )];

        classify_layers(&mut layers, &active_config());

        for pt in &layers[0].ordered_entities[0].path.points {
            assert_eq!(pt.overhang_quartile, None);
        }
    }

    #[test]
    fn role_scope_guard() {
        let mut layers = vec![
            square_wall_layer(0),
            layer_with_entity(
                1,
                ExtrusionRole::SparseInfill,
                vec![make_point(5.0, 5.0), make_point(6.0, 6.0)],
            ),
        ];

        classify_layers(&mut layers, &active_config());

        for pt in &layers[1].ordered_entities[0].path.points {
            assert_eq!(
                pt.overhang_quartile, None,
                "SparseInfill point should remain None"
            );
        }
    }

    #[test]
    fn quartile_boundary_inside() {
        let mut layers = vec![
            square_wall_layer(0),
            layer_with_entity(
                1,
                ExtrusionRole::OuterWall,
                vec![make_point(5.0, 5.0), make_point(5.0, 6.0)],
            ),
        ];

        classify_layers(&mut layers, &active_config());

        for pt in &layers[1].ordered_entities[0].path.points {
            assert_eq!(
                pt.overhang_quartile,
                Some(4),
                "fully-inside point should be Q4, got {:?}",
                pt.overhang_quartile
            );
        }
    }

    #[test]
    fn quartile_boundary_q1() {
        let mut layers = vec![
            square_wall_layer(0),
            layer_with_entity(
                1,
                ExtrusionRole::OuterWall,
                vec![make_point(50.0, 50.0), make_point(51.0, 51.0)],
            ),
        ];

        classify_layers(&mut layers, &active_config());

        for pt in &layers[1].ordered_entities[0].path.points {
            assert_eq!(
                pt.overhang_quartile,
                Some(1),
                "far-outside point should be Q1, got {:?}",
                pt.overhang_quartile
            );
        }
    }
}
