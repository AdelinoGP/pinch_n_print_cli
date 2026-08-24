#![allow(missing_docs)]
//! TDD for `detect_support_contacts`, the support-generation sibling of
//! `annotate_overhangs` (packet 224, RC-0).
//!
//! These tests pin the properties that the previous support implementation got
//! wrong, and that made the decisive SupportTest fixture undetectable:
//!
//! 1. A contact is produced **once, at the overhang's own Z** — not re-derived
//!    at every layer, and not absent because the source facets happen to be
//!    coplanar. Contact detection is 2D over slices, so facet coplanarity is
//!    irrelevant to it.
//! 2. A mesh with no overhang produces **no contacts at all**. Any change that
//!    makes support appear under non-overhanging geometry is a regression.
//!
//! and the canonical-parity rules landed with F-24/F-25/F-26:
//!
//! 3. The threshold angle is bumped by `+1` for inclusivity and clamped to 89
//!    degrees, and an angle of **0** selects canonical's `fw - overlap` branch
//!    rather than a plain difference.
//! 4. The trimmed contact is **expanded back** to the full overhang, so
//!    downstream support columns are wide enough.
//! 5. Contacts that vanish under a `-0.1 * fw` erosion are dropped entirely.
//!
//! Mirrors canonical `detect_overhangs` (`SupportMaterial.cpp`), which grows the
//! lower layer by an angle-derived offset before differencing.

use slicer_core::algos::overhang_annotation::{
    detect_support_contacts, detect_support_contacts_with_annotations, SupportContactParams,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

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

/// Total area of a polygon set, in mm^2, via the shoelace formula on contours.
/// Holes are ignored — every fixture here is hole-free.
fn area_mm2(polys: &[ExPolygon]) -> f32 {
    polys
        .iter()
        .map(|poly| {
            let pts = &poly.contour.points;
            let mut acc = 0.0_f64;
            for i in 0..pts.len() {
                let a = pts[i];
                let b = pts[(i + 1) % pts.len()];
                let (ax, ay) = (
                    slicer_ir::units_to_mm(a.x) as f64,
                    slicer_ir::units_to_mm(a.y) as f64,
                );
                let (bx, by) = (
                    slicer_ir::units_to_mm(b.x) as f64,
                    slicer_ir::units_to_mm(b.y) as f64,
                );
                acc += ax * by - bx * ay;
            }
            (acc / 2.0).abs() as f32
        })
        .sum()
}

/// Params with the canonical defaults (`fw` 0.4mm, overlap 50% of `fw`, no XY
/// expansion) and the two per-call values these tests vary.
fn params(threshold_angle_deg: f32, lower_layer_height_mm: f32) -> SupportContactParams {
    SupportContactParams {
        threshold_angle_deg,
        lower_layer_height_mm,
        ..SupportContactParams::default()
    }
}

/// Params whose `lower_layer_offset` is exactly `0`, i.e. canonical's plain
/// `diff(region, lower)` branch: angle 0 selects the overlap branch, and an
/// overlap equal to `fw` makes `fw - overlap` zero. This is the *only* way to
/// get a plain difference — a zero angle on its own is **not** one (F-26).
fn plain_difference_params() -> SupportContactParams {
    let default = SupportContactParams::default();
    SupportContactParams {
        threshold_angle_deg: 0.0,
        threshold_overlap_mm: default.external_perimeter_width_mm,
        ..default
    }
}

/// A pillar that abruptly widens into a cap, reproducing the decisive
/// SupportTest fixture's shape: narrow column below, wide plate above. The
/// widening is a step, so in the mesh its downward facets are coplanar — which
/// is exactly what defeated facet-based detection.
fn pillar_then_cap() -> Vec<Vec<ExPolygon>> {
    let pillar = vec![rect(0.0, 0.0, 4.0, 4.0)];
    let cap = vec![rect(-8.0, 0.0, 12.0, 4.0)];
    vec![
        pillar.clone(),
        pillar.clone(),
        pillar,
        // The step: layer 3 is much wider than layer 2.
        cap.clone(),
        cap.clone(),
        cap,
    ]
}

/// A 4x4mm pillar capped by a plate overhanging it by `ledge_mm` on the -X and
/// +X sides (two wings, each `ledge_mm` wide and 4mm deep). Two layers: index 0
/// is the pillar, index 1 is the cap.
fn pillar_with_ledge(ledge_mm: f32) -> (Vec<ExPolygon>, Vec<ExPolygon>) {
    (
        vec![rect(0.0, 0.0, 4.0, 4.0)],
        vec![rect(-ledge_mm, 0.0, 4.0 + ledge_mm, 4.0)],
    )
}

/// Runs the per-(layer, region) entry point over a single-region layer stack,
/// returning `(layer_index, contacts)` for every layer that produced one. Layer
/// 0 has no layer below it and is never a contact.
fn sweep(layers: &[Vec<ExPolygon>], params: &SupportContactParams) -> Vec<(usize, Vec<ExPolygon>)> {
    layers
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(index, current)| {
            let contacts = detect_support_contacts(current, &layers[index - 1], &[], params);
            (!contacts.is_empty()).then_some((index, contacts))
        })
        .collect()
}

fn sharp_tail_profile() -> Vec<ExPolygon> {
    let p = |x: f32, y: f32| Point2::from_mm(x, y);
    vec![ExPolygon {
        contour: Polygon {
            points: vec![p(4.0, 2.0), p(4.01, 2.0), p(4.0, 2.01)],
        },
        holes: Vec::new(),
    }]
}

#[test]
fn sharp_tails_add_first_layer_contacts_when_enabled() {
    let profile = sharp_tail_profile();
    let contacts = detect_support_contacts(
        &profile,
        &[rect(0.0, 0.0, 4.0, 4.0)],
        &[],
        &SupportContactParams {
            support_sharp_tails: true,
            layer_id: 0,
            ..params(45.0, 0.2)
        },
    );

    assert!(!contacts.is_empty());
}

#[test]
fn sharp_tails_disabled_by_default_emits_none() {
    let profile = sharp_tail_profile();
    let contacts = detect_support_contacts(
        &profile,
        &[rect(0.0, 0.0, 4.0, 4.0)],
        &[],
        &SupportContactParams {
            layer_id: 0,
            ..SupportContactParams::default()
        },
    );

    assert!(contacts.is_empty());
}

#[test]
fn enforce_support_layers_forces_full_contacts_in_leading_layers() {
    let (lower, upper) = pillar_with_ledge(0.15);
    let contacts = detect_support_contacts(
        &upper,
        &lower,
        &[],
        &SupportContactParams {
            enforce_support_layers: 2,
            layer_id: 1,
            ..params(10.0, 0.2)
        },
    );

    assert!((area_mm2(&contacts) - 2.0 * 0.15 * 4.0).abs() < 0.05);
}

#[test]
fn enforce_support_layers_beyond_model_changes_nothing() {
    let (lower, upper) = pillar_with_ledge(0.5);
    let ordinary = detect_support_contacts(&upper, &lower, &[], &params(45.0, 0.2));
    let enforced = detect_support_contacts(
        &upper,
        &lower,
        &[],
        &SupportContactParams {
            enforce_support_layers: 2,
            layer_id: 2,
            ..params(45.0, 0.2)
        },
    );

    assert!((area_mm2(&enforced) - area_mm2(&ordinary)).abs() < 1e-3);
}

#[test]
fn bridge_areas_are_removed_from_contacts_under_bridge_no_support() {
    let (lower, upper) = pillar_with_ledge(2.0);
    let bridge = vec![rect(-2.0, 0.0, 0.0, 4.0)];
    let contacts = detect_support_contacts(
        &upper,
        &lower,
        &[],
        &SupportContactParams {
            bridge_no_support: true,
            bridge_polygons: bridge,
            ..params(45.0, 0.2)
        },
    );

    assert!(area_mm2(&contacts) < 25.0);
}

#[test]
fn bridge_removal_disabled_keeps_bridge_contacts() {
    let (lower, upper) = pillar_with_ledge(2.0);
    let bridge = vec![rect(-2.0, 0.0, 0.0, 4.0)];
    let contacts = detect_support_contacts(
        &upper,
        &lower,
        &[],
        &SupportContactParams {
            bridge_polygons: bridge,
            ..params(45.0, 0.2)
        },
    );

    assert!(area_mm2(&contacts) > 10.0);
}

#[test]
fn cantilever_pass_records_wide_overhang_annotations() {
    let (lower, upper) = pillar_with_ledge(4.0);
    let result =
        detect_support_contacts_with_annotations(&upper, &lower, &[], &plain_difference_params());

    assert!(!result.contacts.is_empty());
    assert_eq!(result.cantilever_surfaces, result.contacts);
}

#[test]
fn overhang_is_detected_once_at_the_step_layer() {
    let layers = pillar_then_cap();
    let contacts = sweep(&layers, &params(45.0, 0.2));

    assert_eq!(
        contacts.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
        vec![3_usize],
        "contact must be produced exactly once, at the step layer"
    );

    // The cap overhangs the pillar on both sides: 8mm left + 8mm right, 4mm
    // deep. The lower layer is grown by `0.2 / tan(46 deg)` before the
    // difference (45 + 1 inclusivity bump), which trims each wing — and then
    // the canonical expand-back re-grows the trimmed contact by the same offset
    // and re-intersects it with the region, restoring the *full* 8mm wing. That
    // restoration is the point of F-25: contacts feed support columns, which
    // must be as wide as the overhang they carry.
    let expected = 2.0 * 8.0 * 4.0;
    let got = area_mm2(&contacts[0].1);
    assert!(
        (got - expected).abs() < 0.1,
        "contact area {got:.3}mm^2 should be the expanded-back overhang {expected:.3}mm^2"
    );
}

#[test]
fn coplanar_step_does_not_hide_the_contact() {
    // Regression pin for RC-1: facet-based detection filtered facets by
    // `max_z >= slab_bottom && min_z <= layer.z`, so a step whose downward
    // facets are coplanar matched at most one layer slab and typically none.
    // Slice-based detection cannot have that failure mode.
    let layers = pillar_then_cap();
    assert!(
        !sweep(&layers, &params(45.0, 0.2)).is_empty(),
        "a coplanar step must still register a support contact"
    );
}

#[test]
fn straight_column_produces_no_contacts() {
    // The invariant the previous session's fallback destroyed: no overhang
    // must mean no support, regardless of the mesh being non-empty.
    let column = vec![rect(0.0, 0.0, 4.0, 4.0)];
    let layers: Vec<_> = (0..8).map(|_| column.clone()).collect();

    let contacts = sweep(&layers, &params(45.0, 0.2));

    assert!(
        contacts.is_empty(),
        "a straight column has no overhang and must produce no contacts, got {} layers",
        contacts.len()
    );
}

#[test]
fn a_region_sitting_on_a_different_region_below_is_not_a_contact() {
    // F-24. Canonical diffs each region against the union of *all* regions of
    // the layer below (`object.layers()[layer_id - 1]->lslices`), not against
    // the same region's own history. A region that first appears at this layer
    // but sits squarely on a *different* region below is fully supported.
    // Keying the lower layer per-region emitted its entire cross-section as a
    // contact — spurious full-area support on every multi-material object.
    let region_a_below = rect(0.0, 0.0, 4.0, 4.0);
    let region_b_above = rect(1.0, 1.0, 3.0, 3.0);
    let lower_layer_union = vec![region_a_below];

    let contacts = detect_support_contacts(
        &[region_b_above],
        &lower_layer_union,
        &[],
        &params(45.0, 0.2),
    );

    assert!(
        contacts.is_empty(),
        "a region resting on a different region of the layer below is supported, got {contacts:?}"
    );
}

#[test]
fn shallower_threshold_yields_no_more_contact_area_than_a_plain_difference() {
    // Growing the lower layer before differencing can only shrink the result,
    // so an angle threshold is always a subset of the unsupported area — the
    // expand-back is re-intersected with the region and re-differenced against
    // the lower layer, so it can restore the contact but never exceed the plain
    // difference.
    let (lower, upper) = pillar_with_ledge(0.5);
    let plain = detect_support_contacts(&upper, &lower, &[], &plain_difference_params());
    let steep = detect_support_contacts(&upper, &lower, &[], &params(45.0, 0.2));
    // 0.2 / tan(11 deg) = 1.03mm of required overlap — more than the 0.5mm
    // ledge, so a 10-degree threshold treats this ledge as self-supporting.
    let shallow = detect_support_contacts(&upper, &lower, &[], &params(10.0, 0.2));

    let plain_area = area_mm2(&plain);
    let steep_area = area_mm2(&steep);
    let shallow_area = area_mm2(&shallow);

    assert!(plain_area > 0.0, "the plain difference must find the ledge");
    assert!(
        steep_area <= plain_area + 1e-3,
        "thresholded contact ({steep_area:.3}) must not exceed the plain difference ({plain_area:.3})"
    );
    assert!(
        shallow_area < steep_area,
        "a shallower threshold must classify more of the ledge as self-supporting; \
         got {shallow_area:.3} at 10 deg vs {steep_area:.3} at 45 deg"
    );
}

#[test]
fn zero_angle_uses_the_overlap_offset_not_a_plain_difference() {
    // F-26. Canonical:
    //   lower_layer_offset = threshold_rad > 0 ? height / tan(threshold_rad)
    //                                          : fw - support_threshold_overlap;
    // `support_threshold_overlap` is `ConfigOptionFloatOrPercent(50., true)`,
    // so with the default `fw` of 0.4mm a zero angle offsets by 0.2mm — it does
    // NOT mean "support everything", and it is NOT a plain difference. This
    // test previously asserted the opposite.
    let (lower, upper) = pillar_with_ledge(0.15);

    let zero_angle = detect_support_contacts(&upper, &lower, &[], &params(0.0, 0.2));
    assert!(
        zero_angle.is_empty(),
        "a 0.15mm ledge is inside the 0.2mm (fw - overlap) offset, so a zero angle must \
         find no contact; got {zero_angle:?}"
    );

    // The same geometry under a genuinely zero offset does yield the ledge,
    // proving the difference above comes from the offset and not from the
    // geometry being undetectable.
    let plain = detect_support_contacts(&upper, &lower, &[], &plain_difference_params());
    let expected = 2.0 * 0.15 * 4.0;
    let got = area_mm2(&plain);
    assert!(
        (got - expected).abs() < 0.05,
        "a zero `lower_layer_offset` is the plain difference: got {got:.3}mm^2, \
         expected {expected:.3}mm^2"
    );
}

#[test]
fn threshold_angle_bump_catches_an_exactly_at_threshold_overhang() {
    // F-26's `+1`: canonical uses `support_threshold_angle + 1` so an overhang
    // sitting exactly at the configured angle is still caught. A 0.2mm ledge
    // under a 0.2mm layer is exactly 45 degrees; the un-bumped offset
    // (0.2 / tan(45) = 0.2mm) would consume it entirely and find nothing.
    let (lower, upper) = pillar_with_ledge(0.2);
    let contacts = detect_support_contacts(&upper, &lower, &[], &params(45.0, 0.2));

    assert!(
        !contacts.is_empty(),
        "an overhang exactly at the threshold angle must still be caught (the +1 bump)"
    );
    // The expand-back restores the full ledge.
    let expected = 2.0 * 0.2 * 4.0;
    let got = area_mm2(&contacts);
    assert!(
        (got - expected).abs() < 0.05,
        "expand-back must restore the full ledge: got {got:.3}mm^2, expected {expected:.3}mm^2"
    );
}

#[test]
fn threshold_angle_is_clamped_to_eighty_nine_degrees() {
    // F-26's `min(thresh_angle, 89.)`. Without the clamp, `tan` of an
    // out-of-range angle goes negative (tan(1001 deg) < 0), which flips the
    // offset's sign and *shrinks* the lower layer instead of growing it.
    let (lower, upper) = pillar_with_ledge(0.5);
    let clamped = detect_support_contacts(&upper, &lower, &[], &params(88.0, 0.2));
    let absurd = detect_support_contacts(&upper, &lower, &[], &params(1000.0, 0.2));

    let clamped_area = area_mm2(&clamped);
    let absurd_area = area_mm2(&absurd);
    assert!(
        clamped_area > 0.0,
        "an 88-degree threshold must find the ledge"
    );
    assert!(
        (clamped_area - absurd_area).abs() < 1e-3,
        "88 (+1) and 1000 degrees both clamp to 89, so they must agree: \
         {clamped_area:.4} vs {absurd_area:.4}"
    );
}

#[test]
fn tiny_spots_are_filtered_out() {
    // F-25 step 4: `if (diff_polygons.empty() || offset(diff_polygons, -0.1 * fw).empty()) continue;`
    // A 0.02mm-wide ledge erodes away under `-0.1 * 0.4 = -0.04mm` and must be
    // dropped wholesale, while a 1mm ledge survives.
    let (lower, tiny) = pillar_with_ledge(0.02);
    let (_, printable) = pillar_with_ledge(1.0);

    assert!(
        detect_support_contacts(&tiny, &lower, &[], &plain_difference_params()).is_empty(),
        "a sub-line-width contact must be filtered out entirely"
    );
    assert!(
        !detect_support_contacts(&printable, &lower, &[], &plain_difference_params()).is_empty(),
        "a 1mm ledge is well above the tiny-spot threshold and must survive"
    );
}

#[test]
fn xy_expansion_grows_the_finished_contact() {
    // F-25 step 5: `support_expansion` (`coFloat`, default 0), applied to the
    // finished contact region.
    let (lower, upper) = pillar_with_ledge(0.5);
    let unexpanded = detect_support_contacts(&upper, &lower, &[], &params(45.0, 0.2));
    let expanded = detect_support_contacts(
        &upper,
        &lower,
        &[],
        &SupportContactParams {
            xy_expansion_mm: 1.0,
            ..params(45.0, 0.2)
        },
    );

    assert!(
        area_mm2(&expanded) > area_mm2(&unexpanded) + 1.0,
        "a 1mm `support_expansion` must visibly grow the contact: {:.3} vs {:.3}",
        area_mm2(&expanded),
        area_mm2(&unexpanded)
    );
}

#[test]
fn blockers_are_subtracted_from_the_contact() {
    // F-25 step 3: `diff_polygons = diff(diff_polygons, blocker)`. The host
    // stage has no blocker source yet, but the entry point takes them so a
    // blocker-aware caller needs no signature change.
    let (lower, upper) = pillar_with_ledge(2.0);
    let unblocked = detect_support_contacts(&upper, &lower, &[], &params(45.0, 0.2));
    // Covers the whole -X wing.
    let blocker = vec![rect(-3.0, -1.0, 0.0, 5.0)];
    let blocked = detect_support_contacts(&upper, &lower, &blocker, &params(45.0, 0.2));

    let unblocked_area = area_mm2(&unblocked);
    let blocked_area = area_mm2(&blocked);
    assert!(
        unblocked_area > 0.0,
        "the ledge must be a contact to begin with"
    );
    assert!(
        (blocked_area - unblocked_area / 2.0).abs() < 0.1,
        "a blocker over one of the two wings must remove half the contact: \
         {blocked_area:.3} vs {unblocked_area:.3}"
    );
}

#[test]
fn degenerate_inputs_do_not_panic() {
    let square = vec![rect(0.0, 0.0, 4.0, 4.0)];
    assert!(detect_support_contacts(&[], &square, &[], &params(45.0, 0.2)).is_empty());
    // An unsupported region with nothing below it is wholly a contact.
    assert!(!detect_support_contacts(&square, &[], &[], &params(45.0, 0.2)).is_empty());
    assert!(detect_support_contacts(&[], &[], &[], &params(0.0, 0.0)).is_empty());
}
