//! Packet 240a AC-5: the three "object bottom geometry" predicates ruled
//! **Convert** by `design.md` section "First-Model-Layer Audit" resolve their
//! boundary from `support_raft_layers`, not from the literal layer `0`.
//!
//! Under the positive-offset raft band, raft layers occupy global indices
//! `0..support_raft_layers-1` and the object's own first layer is
//! `support_raft_layers`. A predicate that means "the object's bottom"
//! therefore compares against that boundary; a predicate that means "the
//! physical first layer on the plate" keeps comparing against `0`, because the
//! raft IS the physical first layer (the leave-alone list in the same audit).
//!
//! Three sites, one test:
//!
//! 1. `detect_support_contacts`' sharp-tail gate
//!    (`crates/slicer-core/src/algos/overhang_annotation.rs`).
//! 2. `detect_support_contacts`' enforce window - shifted to
//!    `raft_layers .. raft_layers + enforce_support_layers`.
//! 3. `run_perimeters`' overlap-key selection
//!    (`modules/core-modules/classic-perimeters/src/lib.rs`).
//!
//! The `ResolvedConfig` bridge feeding (1) and (2) - `resolve_contact_params`
//! in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - is
//! private, so its own unit test lives in that file's `#[cfg(test)] mod tests`
//! and runs under `--lib`.

use classic_perimeters::ClassicPerimeters;
use slicer_core::algos::overhang_annotation::{detect_support_contacts, SupportContactParams};
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Global index of the object's bottom layer in every fixture below.
const RAFT_LAYERS: u32 = 2;

/// Axis-aligned rectangle in mm.
fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    let p = |x: f32, y: f32| Point2::from_mm(x, y);
    ExPolygon {
        contour: Polygon {
            points: vec![
                p(min_x, min_y),
                p(max_x, min_y),
                p(max_x, max_y),
                p(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

/// Triangular contour small enough that the ordinary angle-thresholded offset
/// consumes it entirely: only the sharp-tail exception can produce a contact.
fn sharp_tail_profile() -> Vec<ExPolygon> {
    let p = |x: f32, y: f32| Point2::from_mm(x, y);
    vec![ExPolygon {
        contour: Polygon {
            points: vec![p(4.0, 2.0), p(4.01, 2.0), p(4.0, 2.01)],
        },
        holes: Vec::new(),
    }]
}

/// A 4x4 mm pillar overhung on both sides by `ledge_mm`.
fn pillar_with_ledge(ledge_mm: f32) -> (Vec<ExPolygon>, Vec<ExPolygon>) {
    (
        vec![rect(0.0, 0.0, 4.0, 4.0)],
        vec![rect(-ledge_mm, 0.0, 4.0 + ledge_mm, 4.0)],
    )
}

/// Base params on a 0.2 mm layer with the raft boundary set. `raft_layers` is
/// the field this packet adds to `SupportContactParams`.
fn raft_aware_params(threshold_angle_deg: f32, layer_id: u32) -> SupportContactParams {
    SupportContactParams {
        threshold_angle_deg,
        lower_layer_height_mm: 0.2,
        raft_layers: RAFT_LAYERS,
        layer_id,
        ..SupportContactParams::default()
    }
}

/// Infill areas emitted by `classic-perimeters` at `layer_index`, with the two
/// overlap keys deliberately set to different values so the key choice is
/// observable in the emitted geometry.
fn classic_infill_areas_at(layer_index: u32) -> Vec<Vec<ExPolygon>> {
    let config: ConfigView = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("line_width", 0.4)
        .int("support_raft_layers", RAFT_LAYERS as i64)
        // Distinct on purpose: the selected key changes the infill inset.
        .float("infill_wall_overlap", 0.0)
        .float("top_bottom_infill_wall_overlap", 0.3)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(layer_index);
    let mut output = PerimeterOutputBuilder::new();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![rect(0.0, 0.0, 10.0, 10.0)]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_bridge_areas(vec![]);

    module
        .run_perimeters(layer_index, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");
    output.infill_areas().to_vec()
}

#[test]
fn object_bottom_predicates_are_raft_aware() {
    // Site 1: the sharp-tail gate fires at the object bottom.
    let profile = sharp_tail_profile();
    let below = vec![rect(0.0, 0.0, 4.0, 4.0)];

    let at_object_bottom = detect_support_contacts(
        &profile,
        &below,
        &[],
        &SupportContactParams {
            support_sharp_tails: true,
            ..raft_aware_params(45.0, RAFT_LAYERS)
        },
    );
    assert!(
        !at_object_bottom.is_empty(),
        "sharp tails must be detected at the object's bottom layer \
         (global index == support_raft_layers == {RAFT_LAYERS})"
    );

    // Site 1 negative: it must NOT fire on the raft itself.
    for raft_index in 0..RAFT_LAYERS {
        let on_raft = detect_support_contacts(
            &profile,
            &below,
            &[],
            &SupportContactParams {
                support_sharp_tails: true,
                ..raft_aware_params(45.0, raft_index)
            },
        );
        assert!(
            on_raft.is_empty(),
            "sharp tails must not fire on raft layer {raft_index}; \
             layer 0 is the raft, not the object bottom"
        );
    }

    // Site 2: the enforce window is shifted by the raft boundary. The window
    // is `raft_layers .. raft_layers + enforce_support_layers`, i.e. 2..4 here.
    // Inside it `force_support` selects the plain difference and the whole
    // ledge is a contact; outside it the angle-thresholded offset consumes the
    // ledge entirely.
    let (lower, upper) = pillar_with_ledge(0.15);
    let enforced = |layer_id: u32| {
        detect_support_contacts(
            &upper,
            &lower,
            &[],
            &SupportContactParams {
                enforce_support_layers: 2,
                ..raft_aware_params(10.0, layer_id)
            },
        )
    };
    for inside in RAFT_LAYERS..RAFT_LAYERS + 2 {
        assert!(
            !enforced(inside).is_empty(),
            "layer {inside} lies inside the shifted enforce window \
             {RAFT_LAYERS}..{} and must force plain-difference contacts",
            RAFT_LAYERS + 2
        );
    }
    for outside in [0_u32, 1, RAFT_LAYERS + 2, RAFT_LAYERS + 3] {
        assert!(
            enforced(outside).is_empty(),
            "layer {outside} lies outside the shifted enforce window \
             {RAFT_LAYERS}..{} and must not be forced",
            RAFT_LAYERS + 2
        );
    }

    // Site 3: the overlap key follows the object bottom.
    let on_raft = classic_infill_areas_at(0);
    let at_bottom = classic_infill_areas_at(RAFT_LAYERS);
    let mid_object = classic_infill_areas_at(RAFT_LAYERS + 5);

    assert_eq!(
        on_raft, mid_object,
        "a raft layer is not bottom-surface context: it must select \
         `infill_wall_overlap`, exactly as an ordinary mid-object layer does"
    );
    assert_ne!(
        at_bottom, mid_object,
        "the object's bottom layer (global index == support_raft_layers) must \
         select `top_bottom_infill_wall_overlap`"
    );
}
