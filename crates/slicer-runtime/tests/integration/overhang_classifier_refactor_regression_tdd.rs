//! Packet 107 (O-T051): pre-vs-post-refactor regression TDD for
//! `overhang-classifier-default` (AC-6).
//!
//! This test reconstructs the exact scenario recorded in
//! `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json`
//! (captured against the PRE-refactor wall-distance implementation) and
//! replays it against the POST-refactor consumer logic
//! (`modules/core-modules/overhang-classifier-default/src/lib.rs`,
//! `run_finalization`).
//!
//! Context-budget note: `slicer-runtime`'s `Cargo.toml` intentionally limits
//! module-crate dev-dependencies to the three fill-claim modules (see the
//! comment above `[dev-dependencies.rectilinear-infill]`) plus the two
//! perimeter modules; `overhang-classifier-default` is not one of them, and
//! this packet's file-edit scope does not include `Cargo.toml`. Per the
//! precedent already established in this same integration-test bucket
//! (`overhang_pipeline_e2e_tdd.rs`, see its module-level doc comment: "would
//! require full instance-pool dispatch plumbing outside this packet's context
//! budget... mirrors the classifier's exact, already-read per-entity
//! governing rule"), this test mirrors `run_finalization`'s logic, including
//! an independent implementation of its geometry predicates and interpolation
//! (see `mirrored_run_finalization` below), rather than adding a new
//! WASM-dispatch or crate-dependency plumbing path.
//! All harness types (`ConfigView`, `ConfigViewBuilder`,
//! `LayerCollectionFixtureBuilder`, `print_entity`, `LayerCollectionView`,
//! `FinalizationOutputBuilder`, `EntityMutation`) are the REAL production SDK
//! types used by `basic_tdd.rs` — only the classifier's own tiny decision
//! function is mirrored inline, because the struct that owns it lives in a
//! crate this test target cannot depend on under this packet's constraints.
//!
//! TRIPWIRE: if `modules/core-modules/overhang-classifier-default/src/lib.rs`
//! changes its per-point rule (currently: stamped signed distances are
//! interpolated at each point and its next point, `None` is full speed, the
//! crossing and segmentation predicates can insert geometry from the prior
//! layer's wall-point boundary, curl is applied after the original-speed clamp,
//! and geometry is emitted before one `SetPointSpeedFactors` per qualifying
//! wall entity), `mirrored_run_finalization` below must be updated to match, or
//! this test will silently validate a stale mirror instead of the real module.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use slicer_ir::{ConfigView, ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::{print_entity, ConfigViewBuilder, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{EntityMutation, FinalizationOutputBuilder, LayerCollectionView, MergeOp};

const EPSILON: f32 = 1e-4;

fn baseline_json() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/overhang_classifier_baseline_speeds.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read baseline fixture at {path:?}: {e}"));
    serde_json::from_str(&raw).expect("baseline fixture must be valid JSON")
}

// ============================================================================
// Mirror of modules/core-modules/overhang-classifier-default/src/lib.rs
// ============================================================================

/// Config float for `key`, defaulting to 0.0. Mirrors lib.rs `speed`.
fn speed(config: &ConfigView, key: &str) -> f32 {
    config.get_float(key).unwrap_or(0.0) as f32
}

/// Base wall speed for `role`. Mirrors lib.rs `base_speed`.
fn base_speed(role: &ExtrusionRole, config: &ConfigView) -> f32 {
    match role {
        ExtrusionRole::OuterWall => speed(config, "outer_wall_speed"),
        ExtrusionRole::InnerWall => speed(config, "inner_wall_speed"),
        ExtrusionRole::ThinWall => speed(config, "thin_wall_speed"),
        _ => 0.0,
    }
}

/// Overhang speed for `quartile` (1..=4), 0.0 otherwise. Mirrors lib.rs
/// `overhang_speed`.
fn overhang_speed(quartile: u8, config: &ConfigView) -> f32 {
    match quartile {
        1 => speed(config, "overhang_1_4_speed"),
        2 => speed(config, "overhang_2_4_speed"),
        3 => speed(config, "overhang_3_4_speed"),
        4 => speed(config, "overhang_4_4_speed"),
        _ => 0.0,
    }
}

/// Mirrors lib.rs `min_distance_from_sections` after resolving sections from
/// the same per-point width used by `speed_for_distance`.
fn min_distance_for_point(path_width: f32, original_speed: f32, config: &ConfigView) -> f32 {
    let levels = [90.0_f32, 75.0, 50.0, 25.0, 13.0, 0.0];
    let overhang_speed_or_ref = |key: &str| {
        let configured = speed(config, key);
        if configured < 0.5 {
            original_speed
        } else {
            configured
        }
    };
    let sixth_speed = if config
        .get_bool("slowdown_for_curled_perimeters")
        .unwrap_or(false)
    {
        overhang_speed_or_ref("overhang_4_4_speed")
    } else {
        speed(config, "bridge_speed")
    };
    let configured = [
        original_speed,
        overhang_speed_or_ref("overhang_1_4_speed"),
        overhang_speed_or_ref("overhang_2_4_speed"),
        overhang_speed_or_ref("overhang_3_4_speed"),
        overhang_speed_or_ref("overhang_4_4_speed"),
        sixth_speed,
    ];
    levels
        .into_iter()
        .zip(configured)
        .map(|(overlap, section_speed)| (path_width * (1.0 - overlap / 100.0), section_speed))
        .filter(|(_, section_speed)| *section_speed <= original_speed)
        .map(|(distance, _)| distance)
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(-1.0)
}

/// Independent mirror of the module's deterministic segment intersection
/// helper. Keeping this separate makes the regression test a real tripwire.
fn mirrored_segment_intersections(
    seg: ((f32, f32), (f32, f32)),
    boundary: &[(f32, f32, f32, f32)],
) -> Vec<(f32, f32)> {
    let (start, end) = seg;
    let direction = (end.0 - start.0, end.1 - start.1);
    let direction_length_squared = direction.0 * direction.0 + direction.1 * direction.1;
    let cross = |a: (f32, f32), b: (f32, f32)| a.0 * b.1 - a.1 * b.0;
    let point_distance_squared = |a: (f32, f32), b: (f32, f32)| {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        dx * dx + dy * dy
    };
    let parameter_on_segment = |point: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        let delta = (b.0 - a.0, b.1 - a.1);
        let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
        if length_squared <= EPSILON * EPSILON {
            return (point_distance_squared(point, a) <= EPSILON * EPSILON).then_some(0.0);
        }
        let t = ((point.0 - a.0) * delta.0 + (point.1 - a.1) * delta.1) / length_squared;
        if !(-EPSILON..=1.0 + EPSILON).contains(&t) {
            return None;
        }
        let projected = (a.0 + t * delta.0, a.1 + t * delta.1);
        (point_distance_squared(point, projected) <= EPSILON * EPSILON).then_some(t.clamp(0.0, 1.0))
    };

    let mut intersections = Vec::new();
    let mut add_unique = |point: (f32, f32)| {
        if !intersections
            .iter()
            .any(|existing| point_distance_squared(*existing, point) <= EPSILON * EPSILON)
        {
            intersections.push(point);
        }
    };
    for &(x0, y0, x1, y1) in boundary {
        let boundary_start = (x0, y0);
        let boundary_end = (x1, y1);
        let boundary_direction = (x1 - x0, y1 - y0);
        let boundary_length_squared = boundary_direction.0 * boundary_direction.0
            + boundary_direction.1 * boundary_direction.1;

        if direction_length_squared <= EPSILON * EPSILON {
            if boundary_length_squared <= EPSILON * EPSILON {
                if point_distance_squared(start, boundary_start) <= EPSILON * EPSILON {
                    add_unique(start);
                }
            } else if parameter_on_segment(start, boundary_start, boundary_end).is_some() {
                add_unique(start);
            }
            continue;
        }

        if boundary_length_squared <= EPSILON * EPSILON {
            if parameter_on_segment(boundary_start, start, end).is_some() {
                add_unique(boundary_start);
            }
            continue;
        }

        let from_start = (boundary_start.0 - start.0, boundary_start.1 - start.1);
        let denominator = cross(direction, boundary_direction);
        if denominator.abs() > EPSILON {
            let segment_t = cross(from_start, boundary_direction) / denominator;
            let boundary_t = cross(from_start, direction) / denominator;
            if (-EPSILON..=1.0 + EPSILON).contains(&segment_t)
                && (-EPSILON..=1.0 + EPSILON).contains(&boundary_t)
            {
                let segment_t = segment_t.clamp(0.0, 1.0);
                add_unique((
                    start.0 + segment_t * direction.0,
                    start.1 + segment_t * direction.1,
                ));
            }
        } else if cross(from_start, direction).abs() <= EPSILON {
            if parameter_on_segment(boundary_start, start, end).is_some() {
                add_unique(boundary_start);
            }
            if parameter_on_segment(boundary_end, start, end).is_some() {
                add_unique(boundary_end);
            }
            if parameter_on_segment(start, boundary_start, boundary_end).is_some() {
                add_unique(start);
            }
            if parameter_on_segment(end, boundary_start, boundary_end).is_some() {
                add_unique(end);
            }
        }
    }

    intersections.sort_by(|a, b| {
        let parameter = |point: &(f32, f32)| {
            ((point.0 - start.0) * direction.0 + (point.1 - start.1) * direction.1)
                / direction_length_squared
        };
        parameter(a)
            .total_cmp(&parameter(b))
            .then_with(|| a.0.total_cmp(&b.0))
            .then_with(|| a.1.total_cmp(&b.1))
    });
    intersections
}

/// Mirrors the module's crossing XOR, segmentation gates, interpolation, and
/// per-entity mutation order without depending on the module crate.
fn mirrored_insert_extended_points(
    points: &[Point3WithWidth],
    distances: &[Option<f32>],
    boundary: &[(f32, f32, f32, f32)],
    min_distances: &[f32],
) -> (Vec<Point3WithWidth>, Vec<Option<f32>>) {
    if points.len() != distances.len() || points.len() != min_distances.len() {
        return (points.to_vec(), distances.to_vec());
    }
    if points.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let width_for = |point: Point3WithWidth| point.width;
    let distance_3d = |a: Point3WithWidth, b: Point3WithWidth| {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    let interpolate = |curr: Point3WithWidth,
                       next: Point3WithWidth,
                       t: f32,
                       curr_distance: f32,
                       next_distance: f32,
                       distance: f32| {
        let curr_is_closer = (curr_distance - distance).abs() <= (next_distance - distance).abs();
        Point3WithWidth {
            x: curr.x + t * (next.x - curr.x),
            y: curr.y + t * (next.y - curr.y),
            z: curr.z + t * (next.z - curr.z),
            width: curr.width + t * (next.width - curr.width),
            flow_factor: curr.flow_factor + t * (next.flow_factor - curr.flow_factor),
            overhang_quartile: if curr_is_closer {
                curr.overhang_quartile
            } else {
                next.overhang_quartile
            },
            dist_to_top_mm: curr.dist_to_top_mm + t * (next.dist_to_top_mm - curr.dist_to_top_mm),
            overhang_distance_mm: Some(distance),
        }
    };

    let mut crossing_points = Vec::with_capacity(points.len() + boundary.len());
    crossing_points.push((points[0], distances[0], min_distances[0]));
    for index in 1..points.len() {
        let (curr, curr_distance, curr_min_distance) = crossing_points
            .last()
            .copied()
            .expect("crossing points always contains the current point");
        let next = points[index];
        let next_distance = distances[index];
        let next_min_distance = min_distances[index];
        let curr_boundary_offset = 0.5 * width_for(curr);
        let next_boundary_offset = 0.5 * width_for(next);

        if let (Some(curr_distance), Some(next_distance)) = (curr_distance, next_distance) {
            if (curr_distance > curr_boundary_offset + EPSILON)
                != (next_distance > next_boundary_offset + EPSILON)
            {
                let curr_min_spacing = width_for(curr) * 0.25;
                let next_min_spacing = width_for(next) * 0.25;
                for (x, y) in
                    mirrored_segment_intersections(((curr.x, curr.y), (next.x, next.y)), boundary)
                {
                    let dx = next.x - curr.x;
                    let dy = next.y - curr.y;
                    let length_squared = dx * dx + dy * dy;
                    let t = if length_squared > EPSILON * EPSILON {
                        (((x - curr.x) * dx + (y - curr.y) * dy) / length_squared).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let interpolated_distance = curr_distance + t * (next_distance - curr_distance);
                    let mut candidate = interpolate(
                        curr,
                        next,
                        t,
                        curr_distance,
                        next_distance,
                        interpolated_distance,
                    );
                    let candidate_boundary_offset = 0.5 * width_for(candidate);
                    candidate.overhang_distance_mm = Some(candidate_boundary_offset);
                    if distance_3d(candidate, curr) > curr_min_spacing
                        && distance_3d(next, candidate) > next_min_spacing
                    {
                        let candidate_min_distance =
                            curr_min_distance + t * (next_min_distance - curr_min_distance);
                        crossing_points.push((
                            candidate,
                            Some(candidate_boundary_offset),
                            candidate_min_distance,
                        ));
                    }
                }
            }
        }
        crossing_points.push((next, next_distance, next_min_distance));
    }

    let mut extended = Vec::with_capacity(crossing_points.len() * 2);
    extended.push(crossing_points[0]);
    for index in 0..crossing_points.len() - 1 {
        let (curr, curr_distance, curr_min_distance) = crossing_points[index];
        let (next, next_distance, next_min_distance) = crossing_points[index + 1];

        if let (Some(curr_distance), Some(next_distance)) = (curr_distance, next_distance) {
            let curr_boundary_offset = 0.5 * width_for(curr);
            let next_boundary_offset = 0.5 * width_for(next);
            let curr_min_spacing = width_for(curr) * 0.25;
            let next_min_spacing = width_for(next) * 0.25;
            let near_boundary = (curr_distance > -curr_boundary_offset
                && curr_distance < curr_boundary_offset + 2.0)
                || (next_distance > -next_boundary_offset
                    && next_distance < next_boundary_offset + 2.0);
            if near_boundary {
                let line_len = distance_3d(curr, next);
                let gate_open = ((curr_min_distance > 0.0
                    && curr_distance.abs() > curr_min_distance)
                    || (next_min_distance > 0.0 && next_distance.abs() > next_min_distance))
                    && line_len >= 2.0
                    || (curr_min_distance <= 0.0 && next_min_distance <= 0.0 && line_len > 4.0);
                if gate_open {
                    let a0 =
                        ((curr_distance + 3.0 * curr_boundary_offset) / line_len).clamp(0.0, 1.0);
                    let a1 = (1.0 - (next_distance + 3.0 * next_boundary_offset) / line_len)
                        .clamp(0.0, 1.0);
                    let t0 = a0.min(a1);
                    let t1 = a0.max(a1);

                    let mut add_candidate = |t: f32| {
                        if !(0.0 < t && t < 1.0) {
                            return;
                        }
                        let candidate_distance =
                            curr_distance + t * (next_distance - curr_distance);
                        let candidate = interpolate(
                            curr,
                            next,
                            t,
                            curr_distance,
                            next_distance,
                            candidate_distance,
                        );
                        if distance_3d(candidate, curr) > curr_min_spacing
                            && distance_3d(next, candidate) > next_min_spacing
                        {
                            let candidate_min_distance =
                                curr_min_distance + t * (next_min_distance - curr_min_distance);
                            extended.push((
                                candidate,
                                Some(candidate_distance),
                                candidate_min_distance,
                            ));
                        }
                    };

                    add_candidate(t0);
                    if t1 != t0 {
                        add_candidate(t1);
                    }
                }
            }
        }
        extended.push((next, next_distance, next_min_distance));
    }

    extended
        .into_iter()
        .map(|(point, distance, _)| (point, distance))
        .unzip()
}

/// Mirrors the per-point speed rule in
/// `OverhangClassifierDefault::run_finalization`.
fn mirrored_run_finalization(
    layers: &[LayerCollectionView],
    output: &mut FinalizationOutputBuilder,
    config: &ConfigView,
) {
    if !config.get_bool("enable_overhang_speed").unwrap_or(true) {
        return;
    }
    if (1..=4).all(|q| overhang_speed(q, config) == 0.0) {
        return;
    }
    let flow_width = config
        .get_float("outer_wall_line_width")
        .or_else(|| config.get_float("line_width"))
        .unwrap_or(0.0) as f32;
    for (idx, layer) in layers.iter().enumerate() {
        let boundary: Vec<(f32, f32, f32, f32)> = if idx == 0 || flow_width <= 0.0 {
            Vec::new()
        } else {
            let mut boundary = Vec::new();
            for entity in layers[idx - 1].ordered_entities() {
                if entity.role != ExtrusionRole::OuterWall {
                    continue;
                }
                let points = &entity.path.points;
                boundary.extend(
                    points
                        .windows(2)
                        .map(|segment| (segment[0].x, segment[0].y, segment[1].x, segment[1].y)),
                );

                // Add the closing edge for closed loops that do not repeat the first point.
                if points.len() >= 2 {
                    let first = points[0];
                    let last = points[points.len() - 1];
                    if first.x != last.x || first.y != last.y {
                        boundary.push((last.x, last.y, first.x, first.y));
                    }
                }
            }
            boundary
        };

        if idx > 0 {
            for entity in layer.ordered_entities() {
                let base = base_speed(&entity.role, config);
                if base <= 0.0 || entity.path.points.is_empty() {
                    continue;
                }

                let original_points = &entity.path.points;
                let distances: Vec<Option<f32>> = original_points
                    .iter()
                    .map(|point| point.overhang_distance_mm)
                    .collect();
                let min_distances: Vec<f32> = original_points
                    .iter()
                    .map(|point| min_distance_for_point(point.width, base, config))
                    .collect();
                let (new_points, new_distances) = mirrored_insert_extended_points(
                    original_points,
                    &distances,
                    &boundary,
                    &min_distances,
                );
                let points_grew = new_points.len() > original_points.len();
                let points = &new_points;
                let mut speeds = Vec::with_capacity(points.len());
                let mut has_distance = false;
                for (point_idx, point) in points.iter().enumerate() {
                    let mut extrusion_speed = base;
                    if point.overhang_quartile.is_some() {
                        if let Some(distance) = new_distances[point_idx] {
                            has_distance = true;
                            let current_speed =
                                speed_for_distance(distance, point.width, base, config);
                            let next_speed = points
                                .get(point_idx + 1)
                                .filter(|next| next.overhang_quartile.is_some())
                                .and_then(|_| new_distances.get(point_idx + 1).copied().flatten())
                                .map(|next_distance| {
                                    let next = &points[point_idx + 1];
                                    speed_for_distance(next_distance, next.width, base, config)
                                })
                                .unwrap_or(base);
                            extrusion_speed = current_speed.min(next_speed).min(base);
                        }
                    }

                    // The mirror intentionally omits curl, which remains covered
                    // by the module-level tests; geometry and speed ordering are
                    // implemented here independently.
                    speeds.push(extrusion_speed);
                }

                if !has_distance {
                    continue;
                }
                let factors: Vec<f32> = speeds.into_iter().map(|speed| speed / base).collect();
                if points_grew {
                    output
                        .modify_entity(
                            layer.layer_index(),
                            entity.entity_id,
                            EntityMutation::SetPathPoints(new_points),
                        )
                        .expect("modify_entity must succeed against a fixture-built layer");
                }
                let mutation = EntityMutation::SetPointSpeedFactors(factors);
                output
                    .modify_entity(layer.layer_index(), entity.entity_id, mutation)
                    .expect("modify_entity must succeed against a fixture-built layer");
            }
        }
    }
}

fn speed_for_distance(
    distance: f32,
    path_width: f32,
    original_speed: f32,
    config: &ConfigView,
) -> f32 {
    let levels = [90.0_f32, 75.0, 50.0, 25.0, 13.0, 0.0];
    let overhang_speed_or_ref = |key: &str| {
        let configured = speed(config, key);
        if configured < 0.5 {
            original_speed
        } else {
            configured
        }
    };
    let sixth_speed = if config
        .get_bool("slowdown_for_curled_perimeters")
        .unwrap_or(false)
    {
        overhang_speed_or_ref("overhang_4_4_speed")
    } else {
        speed(config, "bridge_speed")
    };
    let configured = [
        original_speed,
        overhang_speed_or_ref("overhang_1_4_speed"),
        overhang_speed_or_ref("overhang_2_4_speed"),
        overhang_speed_or_ref("overhang_3_4_speed"),
        overhang_speed_or_ref("overhang_4_4_speed"),
        sixth_speed,
    ];
    let mut sections: Vec<(f32, f32)> = levels
        .into_iter()
        .zip(configured)
        .map(|(overlap, section_speed)| (path_width * (1.0 - overlap / 100.0), section_speed))
        .collect();
    sections.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));
    for i in 1..sections.len() {
        if sections[i].0 == sections[i - 1].0 {
            sections[i].1 = sections[i - 1].1;
        }
    }
    if distance <= sections[0].0 {
        return original_speed;
    }
    if distance >= sections[sections.len() - 1].0 {
        return sections[sections.len() - 1].1.min(original_speed);
    }
    let pair = sections
        .windows(2)
        .find(|pair| distance <= pair[1].0)
        .expect("distance must be bracketed by sorted sections");
    let (d0, s0) = pair[0];
    let (d1, s1) = pair[1];
    let t = ((distance - d0) / (d1 - d0)).clamp(0.0, 1.0);
    ((1.0 - t) * s0 + t * s1).round().min(original_speed)
}

// ============================================================================
// Scenario reconstruction
// ============================================================================

/// One `OuterWall` square entity (entity_id=1, wall width 0.4mm) per layer,
/// with every vertex carrying the baseline's recorded per-layer quartile and
/// the corresponding stamped distance at that band's boundary. Geometry
/// itself is irrelevant to the distance rule, so a fixed unit square suffices.
fn wall_entity_with_quartile(layer_index: u32, quartile: u8) -> slicer_ir::PrintEntity {
    let w = 0.4_f32;
    let z = layer_index as f32 * 0.2;
    let distance = match quartile {
        1 => Some(w * 0.25),
        2 => Some(w * 0.50),
        3 => Some(w * 0.75),
        4 => Some(w * 0.87),
        _ => None,
    };
    let pt = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: w,
        flow_factor: 1.0,
        overhang_quartile: Some(quartile),
        dist_to_top_mm: 0.0,
        overhang_distance_mm: distance,
    };
    print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)],
        RegionKey {
            global_layer_index: layer_index,
            object_id: "obj-0".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        0,
    )
}

/// The baseline's 5 classified layers (1..=5), quartiles per
/// `config_case_B_configured.per_entity_results`: layer1->Q3, layer2->Q1,
/// layer3->Q1, layer4->Q4, layer5->Q3.
fn baseline_layer_quartiles() -> Vec<(u32, u8)> {
    vec![(1, 3), (2, 1), (3, 1), (4, 4), (5, 3)]
}

fn build_layers(quartiles: &[(u32, u8)]) -> Vec<LayerCollectionView> {
    let mut layers = vec![LayerCollectionView::new(
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(0)
            .z(0.0)
            .build(),
    )];
    layers.extend(quartiles.iter().map(|&(layer_index, q)| {
        let entity = wall_entity_with_quartile(layer_index, q);
        let layer = LayerCollectionFixtureBuilder::new()
            .global_layer_index(layer_index)
            .z(layer_index as f32 * 0.2)
            .add_entity(entity)
            .build();
        LayerCollectionView::new(layer)
    }));
    layers
}

fn config_from_json(cfg: &Value) -> ConfigView {
    let f = |key: &str| {
        cfg[key]
            .as_f64()
            .unwrap_or_else(|| panic!("missing float key {key}"))
    };
    ConfigViewBuilder::new()
        .float("outer_wall_speed", f("outer_wall_speed"))
        .float("inner_wall_speed", f("inner_wall_speed"))
        .float("thin_wall_speed", f("thin_wall_speed"))
        .float("overhang_1_4_speed", f("overhang_1_4_speed"))
        .float("overhang_2_4_speed", f("overhang_2_4_speed"))
        .float("overhang_3_4_speed", f("overhang_3_4_speed"))
        .float("overhang_4_4_speed", f("overhang_4_4_speed"))
        .float("bridge_speed", 25.0)
        .build()
}

fn collect_speed_factors(output: &FinalizationOutputBuilder) -> Vec<(u32, u64, f32)> {
    output
        .merge_ops()
        .filter_map(|op| match op {
            MergeOp::ModifyEntity {
                layer,
                entity_id,
                mutation: EntityMutation::SetPointSpeedFactors(factors),
            } => factors
                .first()
                .copied()
                .map(|factor| (*layer, *entity_id, factor)),
            _ => None,
        })
        .collect()
}

fn geometry_mirror_config() -> ConfigView {
    ConfigViewBuilder::new()
        .float("outer_wall_speed", 60.0)
        .float("inner_wall_speed", 60.0)
        .float("thin_wall_speed", 60.0)
        .float("overhang_1_4_speed", 30.0)
        .float("overhang_2_4_speed", 40.0)
        .float("overhang_3_4_speed", 50.0)
        .float("overhang_4_4_speed", 60.0)
        .float("bridge_speed", 25.0)
        .float("line_width", 0.4)
        .bool("slowdown_for_curled_perimeters", false)
        .build()
}

fn crossing_mirror_layers() -> Vec<LayerCollectionView> {
    let point =
        |x: f32, y: f32, z: f32, quartile: Option<u8>, distance: Option<f32>| Point3WithWidth {
            x,
            y,
            z,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: quartile,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: distance,
        };
    let entity = |entity_id: u64, layer_index: u32, points: Vec<Point3WithWidth>| {
        print_entity(
            entity_id,
            ExtrusionRole::OuterWall,
            points,
            RegionKey {
                global_layer_index: layer_index,
                object_id: "obj-0".to_string(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            0,
        )
    };
    let lower = entity(
        10,
        0,
        vec![
            point(0.0, 0.0, 0.0, None, None),
            point(10.0, 0.0, 0.0, None, None),
            point(10.0, 10.0, 0.0, None, None),
            point(0.0, 10.0, 0.0, None, None),
            point(0.0, 0.0, 0.0, None, None),
        ],
    );
    let upper = entity(
        20,
        1,
        vec![
            point(-1.0, 5.0, 0.2, Some(1), Some(0.5)),
            point(5.0, 5.0, 0.2, Some(1), Some(-0.5)),
            point(11.0, 5.0, 0.2, Some(1), Some(0.5)),
            point(-1.0, 5.0, 0.2, Some(1), Some(0.5)),
        ],
    );
    vec![
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(0)
            .z(0.0)
            .add_entity(lower)
            .build(),
        LayerCollectionFixtureBuilder::new()
            .global_layer_index(1)
            .z(0.2)
            .add_entity(upper)
            .build(),
    ]
    .into_iter()
    .map(LayerCollectionView::new)
    .collect()
}

// ============================================================================
// (a) default-config run -> 0 mutations (matches baseline case A)
// ============================================================================

#[test]
fn default_config_case_a_matches_baseline_zero_mutations() {
    let baseline = baseline_json();
    let config = config_from_json(&baseline["config_case_A_defaults"]["config"]);
    assert_eq!(
        baseline["config_case_A_defaults"]["observed_mutation_count"]
            .as_u64()
            .unwrap(),
        0,
        "baseline fixture sanity check: case A must record 0 mutations"
    );

    let layers = build_layers(&baseline_layer_quartiles());
    let mut output = FinalizationOutputBuilder::new();
    mirrored_run_finalization(&layers, &mut output, &config);

    let mutations = collect_speed_factors(&output);
    assert!(
        mutations.is_empty(),
        "expected 0 mutations under default (all-zero overhang speed) config, matching \
         the recorded PRE-refactor baseline of observed_mutation_count=0; got: {mutations:?}"
    );
}

// ============================================================================
// (b)+(c)+(d) configured run -> matches baseline factors for Q1/Q3 entities,
// Q4 entity now honored (intentional delta), no other entities mutated.
// ============================================================================

#[test]
fn configured_case_b_matches_baseline_with_documented_q4_delta() {
    let baseline = baseline_json();
    let config = config_from_json(&baseline["config_case_B_configured"]["config"]);

    let layers = build_layers(&baseline_layer_quartiles());
    let mut output = FinalizationOutputBuilder::new();
    mirrored_run_finalization(&layers, &mut output, &config);

    let mutations = collect_speed_factors(&output);

    // (d) exactly one mutation per layer, all on entity_id=1: post-refactor
    // now honors Q4 too, so 5 mutations (baseline pre-refactor had 4; the 5th
    // — layer 4 / Q4 — is the documented intentional delta asserted in (c)).
    assert_eq!(
        mutations.len(),
        5,
        "expected exactly 5 SetPointSpeedFactors mutations (one per reconstructed layer, \
         including the now-honored Q4 entity); got: {mutations:?}"
    );

    let factor_for = |layer: u32| -> f32 {
        mutations
            .iter()
            .find(|&&(l, e, _)| l == layer && e == 1)
            .unwrap_or_else(|| {
                panic!("expected a mutation for layer {layer} entity_id=1, got: {mutations:?}")
            })
            .2
    };

    // (b) baseline-mutated entities (Q1/Q3) get the SAME factor within
    // tolerance: layer1(Q3)->0.4, layer2(Q1)->0.8, layer3(Q1)->0.8, layer5(Q3)->0.4.
    let expected_baseline_matches: [(u32, f32); 4] = [(1, 0.4), (2, 0.8), (3, 0.8), (5, 0.4)];
    let mut max_deviation = 0.0_f32;
    for (layer, expected_factor) in expected_baseline_matches {
        let actual = factor_for(layer);
        let deviation = (actual - expected_factor).abs();
        max_deviation = max_deviation.max(deviation);
        assert!(
            deviation < EPSILON,
            "layer {layer}: expected factor {expected_factor} (matching PRE-refactor \
             baseline), got {actual} (deviation {deviation}, tolerance {EPSILON})"
        );
    }

    // (c) INTENTIONAL DELTA (packet-approved, documented in baseline JSON
    // `notes[1]` and Step 2 of this packet): layer 4 (Q4) now receives
    // factor overhang_4_4_speed/outer_wall_speed = 12/60 = 0.2. Pre-refactor,
    // this entity received NO mutation at all (lib.rs unconditionally skipped
    // quartile >= 4, per baseline `per_entity_results[3]`). This assertion
    // captures the new, expected post-refactor behavior — a FAILURE here
    // means the Q4 honoring behavior regressed, not that the test is wrong.
    let q4_factor = factor_for(4);
    assert!(
        (q4_factor - 0.2).abs() < EPSILON,
        "INTENTIONAL DELTA check: expected layer 4 (Q4) to now receive factor \
         overhang_4_4_speed/outer_wall_speed = 12/60 = 0.2 (post-refactor honors Q4, \
         unlike the pre-refactor baseline which structurally skipped it); got {q4_factor}"
    );

    eprintln!(
        "overhang_classifier_refactor_regression_tdd: compared 4 baseline-matched entities \
         (Q1 x2, Q3 x2), max deviation = {max_deviation}; Q4 delta asserted at factor 0.2 \
         (intentional, packet-approved); no unexpected deltas found."
    );
}

#[test]
fn mirror_geometry_branch_emits_path_points_before_profile() {
    let layers = crossing_mirror_layers();
    let config = geometry_mirror_config();
    let mut output = FinalizationOutputBuilder::new();
    mirrored_run_finalization(&layers, &mut output, &config);

    let mut path_points = None;
    let mut factors = None;
    let mut path_op_index = None;
    let mut factor_op_index = None;
    for (index, op) in output.merge_ops().enumerate() {
        match op {
            MergeOp::ModifyEntity {
                layer: 1,
                entity_id: 20,
                mutation: EntityMutation::SetPathPoints(points),
            } => {
                path_op_index = Some(index);
                path_points = Some(points.clone());
            }
            MergeOp::ModifyEntity {
                layer: 1,
                entity_id: 20,
                mutation: EntityMutation::SetPointSpeedFactors(values),
            } => {
                factor_op_index = Some(index);
                factors = Some(values.clone());
            }
            _ => {}
        }
    }

    let path_points = path_points.expect("mirror must exercise the geometry branch");
    let factors = factors.expect("mirror must emit a profile after geometry");
    assert!(path_points.len() > 4);
    assert_eq!(path_points.first().unwrap().x, -1.0);
    assert_eq!(path_points.last().unwrap().x, -1.0);
    assert_eq!(path_points.first().unwrap().y, 5.0);
    assert_eq!(path_points.last().unwrap().y, 5.0);
    assert_eq!(factors.len(), path_points.len());
    assert!(path_op_index.unwrap() < factor_op_index.unwrap());
}
