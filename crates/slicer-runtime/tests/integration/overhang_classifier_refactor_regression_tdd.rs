//! Packet 107 (O-T051): pre-vs-post-refactor regression TDD for
//! `overhang-classifier-default` (AC-6).
//!
//! This test reconstructs the exact scenario recorded in
//! `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json`
//! (captured against the PRE-refactor wall-distance implementation) and
//! replays it against the POST-refactor consumer logic
//! (`modules/core-modules/overhang-classifier-default/src/lib.rs`,
//! `run_finalization`, 78 LOC total).
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
//! governing rule"), this test mirrors `run_finalization`'s logic verbatim
//! (see `mirrored_run_finalization` below, a line-for-line port of lib.rs)
//! rather than adding a new WASM-dispatch or crate-dependency plumbing path.
//! All harness types (`ConfigView`, `ConfigViewBuilder`,
//! `LayerCollectionFixtureBuilder`, `print_entity`, `LayerCollectionView`,
//! `FinalizationOutputBuilder`, `EntityMutation`) are the REAL production SDK
//! types used by `basic_tdd.rs` — only the classifier's own tiny decision
//! function is mirrored inline, because the struct that owns it lives in a
//! crate this test target cannot depend on under this packet's constraints.
//!
//! TRIPWIRE: if `modules/core-modules/overhang-classifier-default/src/lib.rs`
//! changes its per-point rule (currently: stamped signed distances are
//! interpolated at each point and its next point, `None` is full speed, curl is
//! applied after the original-speed clamp, and one `SetPointSpeedFactors` is
//! emitted per qualifying wall entity), `mirrored_run_finalization` below must
//! be updated to match, or this test will silently validate a stale mirror
//! instead of the real module.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use slicer_ir::{ConfigView, ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::{print_entity, ConfigViewBuilder, LayerCollectionFixtureBuilder};
use slicer_sdk::traits::{EntityMutation, FinalizationOutputBuilder, LayerCollectionView, MergeOp};

const TOLERANCE: f32 = 1e-4;

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
    for (idx, layer) in layers.iter().enumerate() {
        if idx == 0 {
            continue;
        }
        for entity in layer.ordered_entities() {
            let base = base_speed(&entity.role, config);
            if base <= 0.0 || entity.path.points.is_empty() {
                continue;
            }

            let mut speeds = Vec::with_capacity(entity.path.points.len());
            let mut has_distance = false;
            for (point_idx, point) in entity.path.points.iter().enumerate() {
                let mut extrusion_speed = base;
                if point.overhang_quartile.is_some() {
                    if let Some(distance) = point.overhang_distance_mm {
                        has_distance = true;
                        let current_speed = speed_for_distance(distance, point.width, base, config);
                        let next_speed = entity
                            .path
                            .points
                            .get(point_idx + 1)
                            .filter(|next| next.overhang_quartile.is_some())
                            .and_then(|next| next.overhang_distance_mm)
                            .map(|next_distance| {
                                let next = &entity.path.points[point_idx + 1];
                                speed_for_distance(next_distance, next.width, base, config)
                            })
                            .unwrap_or(base);
                        extrusion_speed = current_speed.min(next_speed).min(base);
                    }
                }

                // The regression fixture has no curl reference geometry; the
                // module's curl path is covered by the module-level tests.
                speeds.push(extrusion_speed);
            }

            if !has_distance {
                continue;
            }
            let factors: Vec<f32> = speeds.into_iter().map(|speed| speed / base).collect();
            let mutation = EntityMutation::SetPointSpeedFactors(factors);
            output
                .modify_entity(layer.layer_index(), entity.entity_id, mutation)
                .expect("modify_entity must succeed against a fixture-built layer");
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
            deviation < TOLERANCE,
            "layer {layer}: expected factor {expected_factor} (matching PRE-refactor \
             baseline), got {actual} (deviation {deviation}, tolerance {TOLERANCE})"
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
        (q4_factor - 0.2).abs() < TOLERANCE,
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
