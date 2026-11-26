//! Opt-in property tests for the skirt/brim kernel.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_16_pbt-io-gcode/packet.spec.md`
//! § H-SKIRT-BRIM, registry record `H-SKIRT-BRIM` in `test-properties.toml`):
//!
//! * `H-SKIRT-BRIM` — the real private `SkirtBrim::generate` against an
//!   independent count / enclosure / nesting model over a single closed
//!   rectangle `OuterWall` footprint on the exact `1/16` mm grid.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! # Real seam (packet deviation 1)
//!
//! The authored charter names `generate_skirt_entities` and
//! `generate_brim_entities`; neither exists. The owner is one private
//! `SkirtBrim::generate(&self, &[LayerSnapshot<'_>]) -> Vec<GeneratedEntity>`,
//! and `LayerSnapshot` / `GeneratedEntity` are crate-private structs. This file
//! is an in-crate unit-test child, so it constructs `LayerSnapshot`s over
//! `LayerCollectionIR`s and calls the real `generate` without widening any
//! production visibility. `generate` is the live path for both public entry
//! points (`process` and `run_finalization`), so this is not a
//! helper-nobody-calls witness.
//!
//! # Independent expected model
//!
//! Nothing below asks the module for an expected value. The model is:
//!
//! ```text
//! spacing   = line_width - first_layer_height * (1 - pi/4)        [canonical
//!             Flow::rounded_rectangle_extrusion_spacing, evaluated in f64]
//! bands     = floor(brim_width / spacing / 2)
//! quantised = 2 * bands * spacing
//!
//! skirt ring k (0 = outermost) is the convex hull of the occupied outline --
//! for this footprint a box, `rect +/- (annex + d_k)` with
//! `annex = 0` when no brim area exists and `0.5 * wall width + gap +
//! quantised` when it does, and
//! `d_k = skirt_distance + spacing/2 + (loops - 1 - k) * spacing`.
//!
//! brim band b emits two closed rings, outermost band first:
//!   outer ring at `0.5 * wall width + gap + quantised - (2b+1) * spacing/2`
//!   hole  ring at `0.5 * wall width + gap             + (2b+1) * spacing/2`
//! ```
//!
//! **Shared formula dependency.** `spacing` is the documented canonical
//! `Flow::rounded_rectangle_extrusion_spacing` value; the model re-derives it in
//! f64 from the real operands instead of calling `SkirtBrim::flow_spacing`
//! (f32). The two differ by ~1e-8 relative, which is why
//! `QUANTISATION_GUARD` exists: a case whose `brim_width / spacing / 2` sits
//! within 1e-3 of an integer is *classified* and its count is accepted only
//! among the two legal floors. Every other case is exact.
//!
//! # The classified thin tail (`thin_tail_classified`)
//!
//! The `2 * bands` ring count is exact only while every band is wide enough for
//! clipper's two band boundaries to stay disjoint. Band `b` is
//! `(2 * (bands - b) - 1) * spacing` wide, so the **last** band is exactly one
//! `spacing` — and its two boundaries are not parallel offsets of one another:
//! the annulus's outer boundary is `offset(contour, quantised)`, whose
//! `OFFSET_SHORTEST_EDGE_FACTOR * quantised` vertex threshold converts the
//! reconstruction contour's 90° corner arc into a single chord, while the inner
//! boundary is the contour itself (`brim_object_gap == 0` short-circuits
//! `offset(contour, 0)` to a clone). One chord across a corner sits up to
//! `wall_width / 2 * (1 - cos 45°)` inside a true parallel offset, and each
//! boundary's own round joins add one `BRIM_ARC_TOLERANCE_MM`, so the boundaries
//! can cross once
//! `spacing <= wall_width / 2 * (1 - cos 45°) + 2 * BRIM_ARC_TOLERANCE_MM`
//! (0.083579 mm at the fixture's 0.4 mm wall width). On the line-width ladder
//! that holds only at `line_width = 1/8` (spacing 0.082080 mm). Every earlier
//! band is at least three spacings wide, so it cannot split.
//!
//! Measured on the exact witness `line_width = 0.125, brim_width = 56.6875,
//! rect = 1/16 x 3/16 mm at (-64, -64)`: 345 bands = 690 band boundaries,
//! **693** emitted rings. The last band splits into two ExPolygons, emitting
//! five rings: contours of 185 and 60 points (areas 7.463228 and 2.122190 mm²)
//! and three sliver holes of 4 points each (areas 0.001455, 0.001567 and
//! 0.000351 mm²). Every one is closed, has three distinct vertices and
//! non-zero area.
//!
//! **Candidate observation — append policy verified, emission not claimed.**
//! Canonical's *append step* is confirmed against
//! `Brim.cpp::tryExPolygonOffset` (upstream `main`, read 2026-11-26): after a
//! first `-0.5 * spacing` offset it appends every contour **and every hole** of
//! each Clipper2 result (`polygons_append(loops, to_polygons(islands_ex))`)
//! with no area, vertex or `has_inner_brim` filter, and
//! `Brim.cpp::outer_inner_brim_area` uses the same
//! `floor(width/flow/2)*flow*2` brim-width quantisation. That is **not** the whole
//! emission pipeline: canonical calls `poly_ex.douglas_peucker(resolution)`
//! with `resolution = 0.0125 / SCALING_FACTOR` immediately before each append,
//! and `makeBrimInfillImpl` then unions the loops with
//! `union_pt_chained_outside_in`. PnP does neither, and a 3-vertex sliver less
//! than 0.0125 mm high is exactly what Douglas-Peucker collapses, so **whether
//! canonical would emit these four-point slivers is unmeasured and not
//! claimed**. The exact tail ring multiset is likewise not claimed equivalent:
//! canonical erodes `-1.3 * spacing` and dilates `+0.3 * spacing` per iteration
//! (net one spacing) with `douglas_peucker` between passes, whereas
//! `SkirtBrim::generate` performs a single `-depth` offset per band with
//! `depth += spacing`. The disposition is: classified and recorded
//! (`tail_fragments`), each fragment validated, production left unchanged, and
//! the sliver emission logged here as a candidate observation for a differential
//! check — not claimed defect-free.
//!
//! The model therefore asserts the exact two-rings-per-band structure for the
//! first `2 * (bands - 1)` rings (bands `0..=bands-2`, all at least three
//! spacings wide), and for the tail: at least its contour and its hole (the
//! excess fragment count is clipper's boolean result, so it is *recorded* as
//! `tail_fragments`, never asserted), each fragment closed with three distinct
//! vertices and non-zero area, each inside the last band's clearance window,
//! with the end-to-end ordering still checked. `skirt_brim_controls` pins the
//! measured 693/3/5/2 numbers (rings / excess / tail rings / tail contours) on
//! this exact input, and `skirt_brim_rounding_boundary_control` asserts the
//! relaxation does *not* fire on its own case.
//!
//! # Why the containment oracle is test-local f64, not `oracle::point_in_region`
//!
//! The packet's shared oracles take `Point2`; converting the *query* points
//! would quantise them to 1e-4 mm, and the skirt/brim rings themselves are
//! clipper f32 output rounded to 1e-4 mm with round-join arcs. The enclosure
//! witness is the rectangle's four edges sampled at a step below the smallest
//! loop pitch, so a handful of sampled points land within one unit of a ring
//! chord: an integer conversion would decide those points by its own rounding,
//! not by the geometry. A documented f64 ray-cast with an explicit
//! on-boundary tolerance (`ON_BOUNDARY_MM`) keeps the decision where it
//! belongs, and the rectangle corners themselves remain exact 1/16 mm values.
//!
//! # Derived envelopes (finding M2)
//!
//! Every window below is derived from one of three mechanism terms, never from
//! "slack that happens to pass":
//!
//! * `BBOX_ENV_MM = 2e-3` — clipper's integer vertex rounding (a few 1e-4 mm
//!   units) plus f32 narrowing.
//! * `corner_chord_sagitta_mm() = wall_width / 2 * (1 - cos 45°)`
//!   (0.058579 mm). `offset_paths` decimates an offset's input ring against
//!   `OFFSET_SHORTEST_EDGE_FACTOR * |delta|`; once that threshold reaches a
//!   chord of the reconstruction contour's 90° corner arc, the arc is replaced
//!   by a chord whose sagitta is this value. A chord can only cut *inside* a
//!   true outward offset.
//! * `SKIRT_ENV_MM = SKIRT_ARC_TOLERANCE_MM + 2 units + 1e-3` — the skirt's own
//!   round joins (`Print::_make_skirt` tessellates against 0.1 mm) plus
//!   rounding.
//!
//! `contour_anchored_env_mm()` = `2 * BRIM_ARC_TOLERANCE_MM (two round-join
//! passes) + corner_chord_sagitta + BBOX + slack` = 0.086579 mm. That is the
//! two-sided bound for the brim's **hole** rings and for every *extent*
//! (`max`/`min` of x/y, which is attained on a flat edge). It is well below one
//! anchor half-width, so a ring sitting half a wall width off its documented
//! reference fails — proved by the `anchor_delta_mm` control in
//! `skirt_brim_corner_join_witness`, which rejects a uniform 0.2 mm reference
//! error with `min_x ... deviation 0.201207; window [-73.298008 -0.086579,
//! +0.086579]`.
//!
//! `outer_derived_env_mm()` widens to `contour_anchored_env_mm() + wall_width/2`
//! only when the contour's **flats** themselves decimate
//! (`flats_decimated`, i.e. the footprint's short side is below
//! `0.005 * quantised`), because only then is the extreme *vertex* displaced.
//! That regime is counted as `decimation_classified` and its band ordering is
//! asserted end-to-end rather than band-by-band.
//!
//! The **nearest-vertex** clearance is the one metric with a genuinely
//! half-anchor-wide window, and it is derived exactly: an outer ring's nearest
//! vertex can be a round-join vertex around a collapsed contour corner, which
//! sits at `quantised - depth` from the footprint *corner* instead of
//! `anchor + quantised - depth` — the anchor term is absent there, not slack.
//! `skirt_brim_corner_join_witness` pins that vertex
//! (`(-11.658100, -95.147697)` at 61.649048 mm against `quantised - depth =
//! 61.648008`, with the ring's extent still exact at 61.847801 against
//! 61.848008) and is the reason the counter `max_clearance_dev_mm` now reports
//! *excess outside the derived window* (0.0) rather than the raw residual.
//!
//! # Literal review (manual; the scanners do not cover this file)
//!
//! `cargo xtask check-literals` is **not** evidence here:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and an out-of-line `mod property_tests;` is
//! not followed. Watched-type literal sites (a watched type is a `pub` struct
//! with at least five named fields under `crates/*/src`):
//!
//! * `PrintEntity` — `outer_wall_loop` sets `entity_id`, `path` and
//!   `region_key` and takes `role`, `topo_order` and `tool_index` from
//!   `print_entity_base(role)` (`..` FRU).
//! * `ExtrusionPath3D` — `outer_wall_loop` sets `points` and takes `role`,
//!   `speed_factor`, `tool_index` and `order_lock` from
//!   `extrusion_path3d_base(role)` (`..` FRU).
//! * `LayerCollectionIR` — `layers_for` sets `global_layer_index`, `z` and
//!   `ordered_entities` and takes the rest from `..Default::default()` (FRU).
//! * `Point3WithWidth` — `outer_wall_loop`'s `point` sets `x`, `y`, `z`,
//!   `width` and `flow_factor` and takes `overhang_quartile`,
//!   `dist_to_top_mm` and `overhang_distance_mm` from `..Default::default()`
//!   (FRU).
//! * `RegionKey` has four named fields, so it is not watched; its literal is
//!   the fixture value and is written exhaustively on purpose. `Rect`,
//!   `SkirtBrimCase`, `Limits`, `RingMeasure` and the mode enums are private
//!   test types.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! patterns of `docs/22_test_quality.md` were reviewed one by one: R1/R8 are
//! absent (no constant asserts, no sleeps); R2/R4 do not apply (every test body
//! asserts values); R3 is absent (no fixture-probe early return); R5 is absent
//! (no hand-maintained roster is compared to a count — the expected counts are
//! derived from the bounds, the case and the spacing; the one pinned count, the
//! 693/5/2/3 thin-tail witness in `skirt_brim_controls`, is a
//! *characterisation regression* for a measured clipper fragmentation, and the
//! pre-fix model rejected it with "expected 690..=690 Brim rings, got 693"); R6
//! is absent (unique names); R7 does not apply (expected values come from the
//! model, never from a second call of `generate`). Non-vacuity is enforced by
//! requiring positive populations for every mode after the run, so a generator
//! regression cannot pass silently. `skirt_brim_controls` mutates one expected
//! decision at a time and requires the *same* checker to reject it, and the
//! shrink probe walks a witness tree to exhaustion and back.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    mm_to_units, ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR,
    Point2, Point3WithWidth, PrintEntity, RegionKey,
};
use slicer_sdk::test_support::fixtures::{extrusion_path3d_base, print_entity_base};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::geometry::{BRIM_ARC_TOLERANCE_MM, SKIRT_ARC_TOLERANCE_MM};
use super::{GeneratedEntity, LayerSnapshot, SkirtBrim};

const PROPERTY_ID: &str = "H-SKIRT-BRIM";
const PROPERTY_TEST: &str = "property_tests::proptest_skirt_brim_enclosure";
/// Exact `PropertyRun::bound` keys of the registry record.
const BOUND_KEYS: [&str; 4] = [
    "max_abs_mm",
    "max_skirt_distance_mm",
    "max_loops",
    "max_line_width_mm",
];

/// Deterministic controls and the shrink probe have no `PropertyRun` context;
/// these are the exact registry bounds (routine and extended agree).
const CONTROL_MAX_ABS_MM: u32 = 64;
const CONTROL_MAX_SKIRT_DISTANCE_MM: u32 = 8;
const CONTROL_MAX_LOOPS: u32 = 3;
const CONTROL_MAX_LINE_WIDTH_MM: u32 = 2;

/// Exact grid of the generated domain: `1/16` mm is 625 scaled units.
const SIXTEENTHS_PER_MM: f64 = 16.0;
const UNITS_PER_SIXTEENTH: i64 = 625;
/// Fixed first-layer height, so `spacing` is always derived, never assumed
/// equal to `line_width`.
const FIRST_LAYER_HEIGHT_MM: f32 = 0.2;
/// Outer-wall point width stamped on every fixture loop. `SkirtBrim::generate`
/// reconstructs the slice contour as the wall centerline grown by half the
/// modal outer-wall point width.
const ANCHOR_POINT_WIDTH_MM: f32 = 0.4;
/// `skirt_height` for every generated case: the skirt lands on layer 0 only.
const SKIRT_HEIGHT: u32 = 1;
/// `brim_object_gap` and `min_skirt_length` are pinned to zero so the brim
/// annulus starts exactly on the reconstructed contour.
const BRIM_OBJECT_GAP_MM: f32 = 0.0;
const MIN_SKIRT_LENGTH_MM: f32 = 0.0;
/// Nonzero global-layer-index base so the region keys are not trivially zero.
const MAX_LANE_BASE: u32 = 8;

/// One scaled unit (1e-4 mm) is clipper's vertex rounding.
const UNIT_ROUNDING_MM: f64 = 1.0e-4;
/// Slack for f32 -> f64 narrowing and clipper's f64 intermediate arithmetic.
const F64_SLACK_MM: f64 = 1.0e-3;
/// A `brim_width / spacing / 2` value closer than this to an integer is a
/// quantisation boundary: the f32 and f64 spacings disagree there, so the
/// count is classified instead of asserted.
const QUANTISATION_GUARD: f64 = 1.0e-3;
/// Edge samples per rectangle side for the enclosure witness.
const MAX_SAMPLES_PER_EDGE: usize = 256;
/// A sampled point this close to a ring edge is inside-or-on.
const ON_BOUNDARY_MM: f64 = 1.0e-6;

/// `[gap - env, gap + quantised + env]` vertex range, derived in the module
/// header.
const BRIM_RANGE_ENV_MM: f64 = (ANCHOR_POINT_WIDTH_MM as f64) * 0.5
    + 2.0 * (BRIM_ARC_TOLERANCE_MM as f64)
    + 2.0 * UNIT_ROUNDING_MM
    + F64_SLACK_MM;
/// Per-ring extreme-coordinate envelope. The extremes of an axis-aligned box
/// offset are attained on the flat edges, so only clipper's integer vertex
/// rounding applies; the value is measured and reported by the property run.
const BBOX_ENV_MM: f64 = 2.0e-3;

/// Envelope for a ring whose boundary descends from the **contour** (the
/// brim's hole rings). Two round-join passes contribute one
/// `BRIM_ARC_TOLERANCE_MM` each; `offset_paths`' vertex decimation can also
/// replace part of the contour's own 90° corner arc with a chord, whose sagitta
/// is bounded by [`corner_chord_sagitta_mm`]; clipper rounds every vertex to the
/// 1e-4 mm grid. This stays well below one anchor half-width, so a ring sitting
/// half a wall width off its documented reference fails it.
fn contour_anchored_env_mm() -> f64 {
    BBOX_ENV_MM + 2.0 * f64::from(BRIM_ARC_TOLERANCE_MM) + corner_chord_sagitta_mm() + F64_SLACK_MM
}

/// Sagitta of one chord across the reconstruction contour's 90° corner arc.
///
/// `SkirtBrim::generate` builds the contour as `offset(filled, 0.5 * modal
/// wall width)`, so the contour has a corner arc of radius
/// `0.5 * wall_width`. When `offset_paths`' `OFFSET_SHORTEST_EDGE_FACTOR *
/// |delta|` vertex threshold reaches that arc's end-to-end chord
/// (`2 * r * sin 45°`), the whole 90° corner becomes one chord, which sits
/// `r * (1 - cos 45°)` inside the arc — 0.058579 mm at the fixture's 0.4 mm
/// wall width.
fn corner_chord_sagitta_mm() -> f64 {
    let anchor = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
    anchor * (1.0 - std::f64::consts::FRAC_PI_4.cos())
}

/// True once the offset's vertex threshold reaches the contour's **flat**
/// edges (whose length is the footprint's short side). Only then can an axis
/// extreme move by a full anchor half-width, because the extreme vertex itself
/// is displaced.
fn flats_decimated(rect: &Rect, quantised: f64) -> bool {
    let short_side = f64::from(rect.x_max - rect.x_min).min(f64::from(rect.y_max - rect.y_min));
    OFFSET_SHORTEST_EDGE_FACTOR * quantised >= short_side
}

/// `(lower, upper)` envelope for a ring whose boundary descends from the
/// **annulus outer** boundary. That boundary comes from
/// `offset(contour, quantised)` and inherits the decimated contour:
///
/// * an intact contour gives the tight two-sided `contour_anchored_env_mm`;
/// * a collapsed corner arc replaces the arc with a chord, which can only cut
///   *inside* the true offset, so only the lower side widens by
///   [`corner_chord_sagitta_mm`];
/// * once the contour's *flat* edges also decimate (the footprint is thinner
///   than `0.005 * quantised`) the extreme vertex itself moves, in either
///   direction, by up to one anchor half-width.
fn outer_derived_env_mm(rect: &Rect, quantised: f64) -> (f64, f64) {
    let base = contour_anchored_env_mm();
    if flats_decimated(rect, quantised) {
        let wide = base + f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
        (wide, wide)
    } else {
        (base, base)
    }
}

/// Envelope for a difference of two ring extents: one brim arc tolerance plus
/// two grid units. A ring whose nominal extreme is a flat edge can place that
/// extreme on a join-arc vertex instead once the offset is large relative to
/// the footprint (the flat becomes shorter than one arc step), and the vertex
/// then sits up to one chord sagitta inside the flat's line. Both extents are
/// also stamped on the 1e-4 mm grid.
const REL_ENV_MM: f64 = (BRIM_ARC_TOLERANCE_MM as f64) + 2.0 * UNIT_ROUNDING_MM;

/// `slicer-core::polygon_ops::OFFSET_SHORTEST_EDGE_FACTOR`: every offset
/// decimates its input ring against `factor * |delta|` before offsetting.
const OFFSET_SHORTEST_EDGE_FACTOR: f64 = 0.005;

/// Classifies the regime where clipper's own vertex decimation moves a
/// brim-derived ring's **axis extreme**: the contour's flat edges are shorter
/// than the decimation threshold. The annex-free relative band/loop structure
/// stays exact there only in the weaker end-to-end form (the two ends of the
/// sequence), so this regime is counted and never asserted as exact.
fn decimation_classified(rect: &Rect, quantised: f64) -> bool {
    flats_decimated(rect, quantised)
}

/// Classifies the thin tail of the brim band sequence.
///
/// The concentric bands step one `spacing` inward from `depth = spacing / 2`, so
/// band `b` spans `(2b+1) * spacing / 2` .. `quantised - (2b+1) * spacing / 2`
/// and its width is `(2 * (bands - b) - 1) * spacing`. The **last** band is
/// therefore exactly one `spacing` wide, and its two boundaries are not
/// parallel offsets of one another: the annulus's outer boundary comes from
/// `offset(contour, quantised)`, whose `OFFSET_SHORTEST_EDGE_FACTOR *
/// quantised` vertex threshold can replace the reconstruction contour's 90°
/// corner arc with a single chord, while the inner boundary is the contour
/// itself (`brim_object_gap == 0` makes `offset(contour, 0)` a clone). At a
/// corner the single-chord approximation sits up to `0.5 * wall_width *
/// (1 - cos 45°)` inside a true parallel offset, and each boundary's own round
/// joins add one `BRIM_ARC_TOLERANCE_MM`, so the two boundaries can cross once
/// `spacing <= 0.5 * wall_width * (1 - cos 45°) + 2 * BRIM_ARC_TOLERANCE_MM`
/// (0.083579 mm at the fixture's 0.4 mm wall width). On the line-width ladder
/// that holds only at `line_width = 1/8` (spacing 0.082080 mm). Every earlier
/// band is at least three spacings wide, so it cannot split.
///
/// Measured on the witness `line_width = 0.125, brim_width = 56.6875, rect =
/// 1/16 x 3/16 mm at (-64, -64)`: 345 bands = 690 band boundaries, **693**
/// emitted rings. The last band splits into two ExPolygons, emitting five
/// rings: contours of 185 and 60 points (areas 7.463228 and 2.122190 mm²) and
/// three 4-point holes (areas 0.001455, 0.001567 and 0.000351 mm²). Every one is
/// closed, has three distinct vertices and non-zero area. Whether canonical
/// would emit the same slivers is **not verified** — see the open question in
/// the module header — so this is classified, never claimed defect-free.
fn thin_tail_classified(spacing: f64) -> bool {
    let anchor = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
    let corner_sagitta = anchor * (1.0 - std::f64::consts::FRAC_PI_4.cos());
    spacing <= corner_sagitta + 2.0 * f64::from(BRIM_ARC_TOLERANCE_MM)
}

/// Per-vertex clearance envelope for the skirt, derived in the module header.
const SKIRT_ENV_MM: f64 = (SKIRT_ARC_TOLERANCE_MM as f64) + 2.0 * UNIT_ROUNDING_MM + F64_SLACK_MM;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct Limits {
    max_abs_mm: f32,
    max_skirt_distance_mm: f32,
    max_loops: u32,
    max_line_width_mm: f32,
}

fn limits_from_bounds(bounds: &BTreeMap<&'static str, u64>) -> Limits {
    Limits {
        max_abs_mm: bounds["max_abs_mm"] as f32,
        max_skirt_distance_mm: bounds["max_skirt_distance_mm"] as f32,
        max_loops: u32::try_from(bounds["max_loops"]).expect("max_loops fits in u32"),
        max_line_width_mm: bounds["max_line_width_mm"] as f32,
    }
}

fn live_limits(run: &PropertyRun) -> Limits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("H-SKIRT-BRIM bound `{key}`: {error}"));
        bounds.insert(key, value);
    }
    let limits = limits_from_bounds(&bounds);
    validate_limits(&limits);
    limits
}

fn control_limits() -> Limits {
    let limits = Limits {
        max_abs_mm: CONTROL_MAX_ABS_MM as f32,
        max_skirt_distance_mm: CONTROL_MAX_SKIRT_DISTANCE_MM as f32,
        max_loops: CONTROL_MAX_LOOPS,
        max_line_width_mm: CONTROL_MAX_LINE_WIDTH_MM as f32,
    };
    validate_limits(&limits);
    limits
}

/// Rejects a profile that cannot form the declared witnesses, per the charter.
/// A profile that fails here never runs as an empty pass.
fn validate_limits(limits: &Limits) {
    assert!(
        limits.max_abs_mm > 0.0 && limits.max_abs_mm.is_finite(),
        "profile max_abs_mm must be positive and finite, got {}",
        limits.max_abs_mm
    );
    assert!(
        (f64::from(limits.max_abs_mm) * SIXTEENTHS_PER_MM).fract() == 0.0,
        "profile max_abs_mm must be a 1/16 mm multiple, got {}",
        limits.max_abs_mm
    );
    assert!(
        (f64::from(limits.max_skirt_distance_mm) * SIXTEENTHS_PER_MM).fract() == 0.0,
        "profile max_skirt_distance_mm must be a 1/16 mm multiple, got {}",
        limits.max_skirt_distance_mm
    );
    assert!(
        limits.max_line_width_mm >= 1.0 / 8.0,
        "profile max_line_width_mm must be at least 1/8 mm, got {}",
        limits.max_line_width_mm
    );
    assert!(
        line_width_ladder(limits).len() >= 2,
        "profile max_line_width_mm must admit at least two power-of-two line widths"
    );
    // Charter bound: the furthest vertex is `max_abs + max_offset`, where the
    // brim's outermost offset is `max_abs` and the last skirt loop is
    // `max_skirt_distance + (max_loops - 1) * max_line_width`.
    let skirt_max = if limits.max_loops == 0 {
        0.0
    } else {
        f64::from(limits.max_skirt_distance_mm)
            + f64::from(limits.max_loops - 1) * f64::from(limits.max_line_width_mm)
    };
    let max_offset = f64::from(limits.max_abs_mm).max(skirt_max);
    let max_vertex_mm = f64::from(limits.max_abs_mm) + max_offset;
    assert!(
        max_vertex_mm.is_finite() && mm_to_units(max_vertex_mm as f32) < (1 << 24),
        "profile vertex bound {max_vertex_mm} mm must stay exactly representable in scaled units"
    );
}

/// The `line_width` ladder: powers of two in `[1/8, max_line_width_mm]`.
fn line_width_ladder(limits: &Limits) -> Vec<f32> {
    let mut ladder = Vec::new();
    for exponent in -3i32..=1 {
        let width = 2f32.powi(exponent);
        if width <= limits.max_line_width_mm {
            ladder.push(width);
        }
    }
    ladder
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RectMode {
    /// Nonzero width and height.
    Regular,
    /// `x_min == x_max`: no polygon, so no brim and no hull.
    DegenerateX,
    /// `y_min == y_max`.
    DegenerateY,
}

/// `Regular` appears three times so the degenerate witnesses stay a minority of
/// the population while remaining guaranteed; `select` draws uniformly over the
/// slice, so this is a static weighting, not a rejection.
const RECT_MODES: [RectMode; 5] = [
    RectMode::Regular,
    RectMode::Regular,
    RectMode::Regular,
    RectMode::DegenerateX,
    RectMode::DegenerateY,
];

/// `brim_width` magnitude classes (every member is a 1/16 mm multiple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrimClass {
    Zero,
    /// Up to 1 mm.
    Small,
    /// Up to 8 mm.
    Mid,
    /// Up to the profile's `max_abs_mm`.
    Wide,
}

const BRIM_CLASSES: [BrimClass; 4] = [
    BrimClass::Zero,
    BrimClass::Small,
    BrimClass::Mid,
    BrimClass::Wide,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopClass {
    Zero,
    One,
    Max,
    Random,
}

const LOOP_CLASSES: [LoopClass; 4] = [
    LoopClass::Zero,
    LoopClass::One,
    LoopClass::Max,
    LoopClass::Random,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DistanceClass {
    Zero,
    Max,
    Random,
}

const DISTANCE_CLASSES: [DistanceClass; 3] = [
    DistanceClass::Zero,
    DistanceClass::Max,
    DistanceClass::Random,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerClass {
    One,
    Two,
}

const LAYER_CLASSES: [LayerClass; 2] = [LayerClass::One, LayerClass::Two];

/// An axis-aligned rectangle whose corners are exact 1/16 mm values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

impl Rect {
    fn is_degenerate(&self) -> bool {
        self.x_min == self.x_max || self.y_min == self.y_max
    }

    fn scaled(&self, scale: f32) -> Rect {
        if scale == 1.0 {
            return *self;
        }
        let cx = 0.5 * (self.x_min + self.x_max);
        let cy = 0.5 * (self.y_min + self.y_max);
        let half_w = 0.5 * (self.x_max - self.x_min) * scale;
        let half_h = 0.5 * (self.y_max - self.y_min) * scale;
        Rect {
            x_min: cx - half_w,
            x_max: cx + half_w,
            y_min: cy - half_h,
            y_max: cy + half_h,
        }
    }
}

#[derive(Debug, Clone)]
struct SkirtBrimCase {
    line_width: f32,
    skirt_distance: f32,
    skirt_loops: u32,
    brim_width: f32,
    rect: Rect,
    base_layer_index: u32,
    layer_count: u32,
    enabled: bool,
    rect_mode: RectMode,
    brim_class: BrimClass,
}

/// Converts a `1/16` mm multiple to f32 and proves the conversion is exact in
/// both the f32 value and the 1e-4 mm scaled-unit grid.
fn sixteenth_mm(sixteenths: i64) -> f32 {
    let mm = (sixteenths as f32) / SIXTEENTHS_PER_MM as f32;
    let expected_units = sixteenths * UNITS_PER_SIXTEENTH;
    assert_eq!(
        mm_to_units(mm),
        expected_units,
        "1/16 mm grid value {sixteenths}/16 must convert exactly"
    );
    assert_eq!(
        Point2::from_mm(mm, mm).x,
        expected_units,
        "1/16 mm grid value {sixteenths}/16 must be a Point2 round trip"
    );
    mm
}

fn sixteenth_value(seed: u32, max: i64) -> i64 {
    let span = (2 * max + 1) as u32;
    i64::from(seed % span) - max
}

/// Construction-first case builder: every numeric input is derived from the
/// integer seed by bounded arithmetic, so no case is ever rejected.
fn build_case(
    line_width: f32,
    rect_mode: RectMode,
    brim_class: BrimClass,
    loop_class: LoopClass,
    distance_class: DistanceClass,
    layer_class: LayerClass,
    seed: [u32; 6],
    limits: &Limits,
) -> SkirtBrimCase {
    let max_sixteenths = (f64::from(limits.max_abs_mm) * SIXTEENTHS_PER_MM).round() as i64;
    let ax = sixteenth_value(seed[0], max_sixteenths);
    let bx = sixteenth_value(seed[1], max_sixteenths);
    let ay = sixteenth_value(seed[2], max_sixteenths);
    let by = sixteenth_value(seed[3], max_sixteenths);
    let (mut x_min, mut x_max) = (ax.min(bx), ax.max(bx));
    let (mut y_min, mut y_max) = (ay.min(by), ay.max(by));
    match rect_mode {
        RectMode::Regular => {
            if x_min == x_max {
                if x_max < max_sixteenths {
                    x_max += 1;
                } else {
                    x_min -= 1;
                }
            }
            if y_min == y_max {
                if y_max < max_sixteenths {
                    y_max += 1;
                } else {
                    y_min -= 1;
                }
            }
        }
        RectMode::DegenerateX => x_max = x_min,
        RectMode::DegenerateY => y_max = y_min,
    }
    let rect = Rect {
        x_min: sixteenth_mm(x_min),
        x_max: sixteenth_mm(x_max),
        y_min: sixteenth_mm(y_min),
        y_max: sixteenth_mm(y_max),
    };

    let brim_cap = match brim_class {
        BrimClass::Zero => 0,
        BrimClass::Small => 15.min(max_sixteenths),
        BrimClass::Mid => 128.min(max_sixteenths),
        BrimClass::Wide => max_sixteenths,
    };
    let brim_sixteenths = i64::from(seed[4] % (brim_cap as u32 + 1)).min(max_sixteenths);
    let brim_width = sixteenth_mm(brim_sixteenths);

    let max_distance_sixteenths =
        (f64::from(limits.max_skirt_distance_mm) * SIXTEENTHS_PER_MM).round() as i64;
    let distance_sixteenths = match distance_class {
        DistanceClass::Zero => 0,
        DistanceClass::Max => max_distance_sixteenths,
        DistanceClass::Random => i64::from(seed[5] % (max_distance_sixteenths as u32 + 1)),
    };
    let skirt_distance = sixteenth_mm(distance_sixteenths);

    let skirt_loops = match loop_class {
        LoopClass::Zero => 0,
        LoopClass::One => 1,
        LoopClass::Max => limits.max_loops,
        LoopClass::Random => seed[5] % (limits.max_loops + 1),
    };

    SkirtBrimCase {
        line_width,
        skirt_distance,
        skirt_loops,
        brim_width,
        rect,
        base_layer_index: seed[0] % MAX_LANE_BASE,
        layer_count: match layer_class {
            LayerClass::One => 1,
            LayerClass::Two => 2,
        },
        enabled: true,
        rect_mode,
        brim_class,
    }
}

fn layers_for(case: &SkirtBrimCase) -> Vec<LayerCollectionIR> {
    (0..case.layer_count)
        .map(|offset| {
            let index = case.base_layer_index + offset;
            let z = FIRST_LAYER_HEIGHT_MM * (offset + 1) as f32;
            // Layer 1 carries a distinct footprint so a regression that folded
            // higher layers into the `skirt_height = 1` hull would move the
            // measured extremes by 1/16 mm, far outside `BBOX_ENV_MM`.
            let rect = if offset == 0 {
                case.rect
            } else {
                case.rect.translated_one_sixteenth()
            };
            LayerCollectionIR {
                global_layer_index: index,
                z,
                ordered_entities: vec![outer_wall_loop(index, z, &rect)],
                ..Default::default()
            }
        })
        .collect()
}

impl Rect {
    fn translated_one_sixteenth(&self) -> Rect {
        Rect {
            x_min: self.x_min + sixteenth_mm(1),
            x_max: self.x_max + sixteenth_mm(1),
            y_min: self.y_min + sixteenth_mm(1),
            y_max: self.y_max + sixteenth_mm(1),
        }
    }
}

fn outer_wall_loop(layer_index: u32, z: f32, rect: &Rect) -> PrintEntity {
    let point = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: ANCHOR_POINT_WIDTH_MM,
        flow_factor: 1.0,
        ..Default::default()
    };
    let points = vec![
        point(rect.x_min, rect.y_min),
        point(rect.x_max, rect.y_min),
        point(rect.x_max, rect.y_max),
        point(rect.x_min, rect.y_max),
        point(rect.x_min, rect.y_min),
    ];
    PrintEntity {
        entity_id: u64::from(layer_index) + 1,
        path: ExtrusionPath3D {
            points,
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        },
        region_key: RegionKey {
            global_layer_index: layer_index,
            object_id: "obj1".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        ..print_entity_base(ExtrusionRole::OuterWall)
    }
}

fn snapshots_for(layers: &[LayerCollectionIR]) -> Vec<LayerSnapshot<'_>> {
    layers
        .iter()
        .map(|layer| LayerSnapshot {
            layer_index: layer.global_layer_index,
            z: layer.z,
            entities: layer.ordered_entities.as_slice(),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures through the real constructors
// ─────────────────────────────────────────────────────────────────────────────

fn config_for(case: &SkirtBrimCase) -> ConfigView {
    ConfigView::from_map(HashMap::from([
        (
            "skirt_brim_enabled".to_string(),
            ConfigValue::Bool(case.enabled),
        ),
        (
            "skirt_loops".to_string(),
            ConfigValue::Int(i64::from(case.skirt_loops)),
        ),
        (
            "skirt_distance".to_string(),
            ConfigValue::Float(f64::from(case.skirt_distance)),
        ),
        (
            "skirt_height".to_string(),
            ConfigValue::Int(i64::from(SKIRT_HEIGHT)),
        ),
        (
            "brim_width".to_string(),
            ConfigValue::Float(f64::from(case.brim_width)),
        ),
        (
            "brim_object_gap".to_string(),
            ConfigValue::Float(f64::from(BRIM_OBJECT_GAP_MM)),
        ),
        (
            "min_skirt_length".to_string(),
            ConfigValue::Float(f64::from(MIN_SKIRT_LENGTH_MM)),
        ),
        (
            "line_width".to_string(),
            ConfigValue::Float(f64::from(case.line_width)),
        ),
        (
            "first_layer_height".to_string(),
            ConfigValue::Float(f64::from(FIRST_LAYER_HEIGHT_MM)),
        ),
    ]))
}

fn module_for(case: &SkirtBrimCase) -> Result<SkirtBrim, String> {
    let config = config_for(case);
    SkirtBrim::from_config(&config).map_err(|error| format!("from_config: {}", error.message))
}

/// Drives the real `generate` over snapshots built from owned layer IRs.
fn observe(case: &SkirtBrimCase) -> Result<Vec<GeneratedEntity>, String> {
    let module = module_for(case)?;
    let layers = layers_for(case);
    let snapshots = snapshots_for(&layers);
    Ok(module.generate(&snapshots))
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent model
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical `Flow::rounded_rectangle_extrusion_spacing`, in f64 mm.
fn model_spacing(line_width: f32) -> f64 {
    let width = f64::from(line_width);
    let height = f64::from(FIRST_LAYER_HEIGHT_MM);
    let spacing = width - height * (1.0 - std::f64::consts::FRAC_PI_4);
    // `SkirtBrim::flow_spacing` never takes its `line_width` fallback on this
    // ladder; assert that so the model cannot silently model the wrong branch.
    assert!(
        spacing > 0.5 * width,
        "line width {line_width} must keep the derived spacing above half the width"
    );
    spacing
}

/// Mutation knobs for the deterministic controls. The property always passes
/// `SkirtBrimModel::default()`.
#[derive(Debug, Clone, Copy)]
struct SkirtBrimModel {
    /// Off-by-one skirt-loop count expectation.
    loop_count_delta: i32,
    /// Dropped (negative) or fabricated (positive) brim ring count.
    brim_count_delta: i32,
    /// Expect the skirt loops innermost-first instead of outermost-first.
    ascending_skirt_order: bool,
    /// Scale the enclosure witness rectangle about its centre.
    enclosure_scale: f32,
    /// Demand the fabricated "a skirt loop is a four-corner rectangle" shape.
    assume_rectangle_shape: bool,
    /// Offset every expected anchor term by this much (a uniform half-wall
    /// reference error). Zero in the property; the control proves such an error
    /// cannot pass the derived windows.
    anchor_delta_mm: f64,
}

impl Default for SkirtBrimModel {
    fn default() -> Self {
        Self {
            loop_count_delta: 0,
            brim_count_delta: 0,
            ascending_skirt_order: false,
            enclosure_scale: 1.0,
            assume_rectangle_shape: false,
            anchor_delta_mm: 0.0,
        }
    }
}

/// Mutations of the *observed* output, used by the controls to prove the same
/// checker rejects a corrupted result rather than only a corrupted expectation.
#[derive(Debug, Clone, Copy, Default)]
struct ObservedMutation {
    open_first_skirt_ring: bool,
    shrink_first_skirt_ring: bool,
    drop_first_brim_ring: bool,
}

#[derive(Debug, Clone, Default)]
struct Summary {
    skirt_rings: u32,
    brim_rings: u32,
    bands: u32,
    boundary_class: bool,
    decimation_classified: bool,
    tail_classified: bool,
    tail_fragments: u32,
    degenerate: bool,
    enclosure_loops: u32,
    coarse_sampled: bool,
    /// Raw maximum |extent deviation| over every checked ring (a measurement:
    /// the decimated-corner chord contributes up to its sagitta).
    max_bbox_dev_mm: f32,
    /// Maximum *excess outside the derived window* over every extent check. Zero
    /// means every extent sat inside its derivation.
    max_bbox_excess_mm: f32,
    /// Maximum *excess outside the derived nearest-vertex window*. Zero means
    /// every nearest-vertex clearance was explained by the model.
    max_clearance_dev_mm: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Geometry oracle (test-local f64; see the module header)
// ─────────────────────────────────────────────────────────────────────────────

fn point_segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::MIN_POSITIVE {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
}

/// Distance from an outside point to the filled rectangle, which equals the
/// distance to its boundary for every point outside it.
fn box_distance(p: (f64, f64), rect: &Rect) -> f64 {
    let dx = (f64::from(rect.x_min) - p.0)
        .max(p.0 - f64::from(rect.x_max))
        .max(0.0);
    let dy = (f64::from(rect.y_min) - p.1)
        .max(p.1 - f64::from(rect.y_max))
        .max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn point_in_ring(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    if ring.len() < 3 {
        return false;
    }
    // On-boundary tolerance first: a witness point exactly on a chord is
    // inside-or-on, and a ray cast alone would decide it arbitrarily.
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + 1) % ring.len()];
        if point_segment_distance(p, a, b) <= ON_BOUNDARY_MM {
            return true;
        }
    }
    let mut inside = false;
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + 1) % ring.len()];
        if (a.1 > p.1) != (b.1 > p.1) {
            let t = (p.1 - a.1) / (b.1 - a.1);
            let x = a.0 + t * (b.0 - a.0);
            if x > p.0 {
                inside = !inside;
            }
        }
    }
    inside
}

/// Samples all four edges of the witness rectangle at a step below the
/// smallest loop pitch (`min(spacing / 2, 0.25 mm)`), capped so a long edge
/// cannot dominate the run. The cap is reported, never silent.
fn witness_samples(rect: &Rect, step: f64) -> (Vec<(f64, f64)>, bool) {
    let corners = [
        (f64::from(rect.x_min), f64::from(rect.y_min)),
        (f64::from(rect.x_max), f64::from(rect.y_min)),
        (f64::from(rect.x_max), f64::from(rect.y_max)),
        (f64::from(rect.x_min), f64::from(rect.y_max)),
    ];
    let mut out = Vec::new();
    let mut capped = false;
    for index in 0..4 {
        let a = corners[index];
        let b = corners[(index + 1) % 4];
        let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let mut count = (length / step.max(f64::MIN_POSITIVE)).ceil() as usize;
        count = count.max(1);
        if count > MAX_SAMPLES_PER_EDGE {
            count = MAX_SAMPLES_PER_EDGE;
            capped = true;
        }
        for index in 0..=count {
            let t = index as f64 / count as f64;
            out.push((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)));
        }
    }
    (out, capped)
}

#[derive(Debug, Clone)]
struct RingMeasure {
    points: usize,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    /// Largest of the four extreme-coordinate offsets from the footprint.
    /// Every side of a correct ring reports the same value; the extent is
    /// attained on a flat offset edge, so it is exact where the per-vertex
    /// clearance carries clipper's round-join tessellation error.
    extent_offset: f64,
    min_clearance: f64,
    max_clearance: f64,
}

/// Validates the shared ring invariants and measures the ring's extremes and
/// clearance to the footprint.
fn measure_ring(
    label: &str,
    path: &ExtrusionPath3D,
    rect: &Rect,
    model: &SkirtBrimModel,
) -> Result<RingMeasure, String> {
    if path.points.len() < 4 {
        return Err(format!(
            "{label}: a closed ring needs at least 4 stamped points, got {}",
            path.points.len()
        ));
    }
    let first = path.points[0];
    let last = *path.points.last().expect("non-empty");
    if first.x != last.x || first.y != last.y || first.z != last.z {
        return Err(format!(
            "{label}: ring is open (first {:?} != last {:?})",
            (first.x, first.y, first.z),
            (last.x, last.y, last.z)
        ));
    }
    let ring: Vec<(f64, f64)> = path
        .points
        .iter()
        .map(|p| (f64::from(p.x), f64::from(p.y)))
        .collect();
    for index in 1..ring.len() {
        if ring[index] == ring[index - 1] {
            return Err(format!(
                "{label}: duplicate consecutive vertex at index {index}: {:?}",
                ring[index]
            ));
        }
    }
    let mut twice_area = 0.0f64;
    for index in 0..ring.len() - 1 {
        let (x0, y0) = ring[index];
        let (x1, y1) = ring[index + 1];
        twice_area += x0 * y1 - x1 * y0;
    }
    if twice_area == 0.0 {
        return Err(format!("{label}: zero-area ring"));
    }
    if model.assume_rectangle_shape && path.points.len() != 5 {
        return Err(format!(
            "{label}: a fabricated four-corner rectangle shape would need 5 stamped \
             points, the real loop has {}",
            path.points.len()
        ));
    }
    let mut measure = RingMeasure {
        points: path.points.len(),
        min_x: f64::MAX,
        max_x: f64::MIN,
        min_y: f64::MAX,
        max_y: f64::MIN,
        extent_offset: f64::MIN,
        min_clearance: f64::MAX,
        max_clearance: f64::MIN,
    };
    for &p in ring.iter().take(ring.len() - 1) {
        measure.min_x = measure.min_x.min(p.0);
        measure.max_x = measure.max_x.max(p.0);
        measure.min_y = measure.min_y.min(p.1);
        measure.max_y = measure.max_y.max(p.1);
        let clearance = box_distance(p, rect);
        measure.min_clearance = measure.min_clearance.min(clearance);
        measure.max_clearance = measure.max_clearance.max(clearance);
    }
    measure.extent_offset = (measure.max_x - f64::from(rect.x_max))
        .max(f64::from(rect.x_min) - measure.min_x)
        .max(measure.max_y - f64::from(rect.y_max))
        .max(f64::from(rect.y_min) - measure.min_y);
    Ok(measure)
}

/// Compares the ring's four extreme coordinates against `rect +/- offset`.
fn check_ring_extremes(
    label: &str,
    measure: &RingMeasure,
    rect: &Rect,
    offset: f64,
    lower_env: f64,
    upper_env: f64,
    summary: &mut Summary,
) -> Result<(), String> {
    let expected = [
        (f64::from(rect.x_min) - offset, measure.min_x, "min_x"),
        (f64::from(rect.x_max) + offset, measure.max_x, "max_x"),
        (f64::from(rect.y_min) - offset, measure.min_y, "min_y"),
        (f64::from(rect.y_max) + offset, measure.max_y, "max_y"),
    ];
    for (expected, observed, axis) in expected {
        let deviation = observed - expected;
        summary.max_bbox_dev_mm = summary.max_bbox_dev_mm.max(deviation.abs() as f32);
        // One-sided: a decimated chord can only cut *inside* the true offset,
        // so the lower envelope is the wide one and the upper stays tight.
        let excess = (-deviation - lower_env).max(deviation - upper_env).max(0.0);
        summary.max_bbox_excess_mm = summary.max_bbox_excess_mm.max(excess as f32);
        if excess > 0.0 {
            return Err(format!(
                "{label}: {axis} is {observed:.6} mm, expected {expected:.6} mm \
                 (deviation {deviation:.6}; window [{} -{lower_env}, +{upper_env}])",
                expected
            ));
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Checker
// ─────────────────────────────────────────────────────────────────────────────

fn check_case(
    case: &SkirtBrimCase,
    model: &SkirtBrimModel,
    mutation: ObservedMutation,
) -> Result<Summary, String> {
    let mut observed = observe(case)?;
    apply_mutation(&mut observed, mutation)?;
    check_observed(case, &observed, model)
}

fn apply_mutation(
    observed: &mut Vec<GeneratedEntity>,
    mutation: ObservedMutation,
) -> Result<(), String> {
    if mutation.drop_first_brim_ring {
        let position = observed
            .iter()
            .position(|entity| entity.path.role == ExtrusionRole::Brim)
            .ok_or_else(|| "mutation witness has no brim ring".to_string())?;
        observed.remove(position);
    }
    if !mutation.open_first_skirt_ring && !mutation.shrink_first_skirt_ring {
        return Ok(());
    }
    let ring = observed
        .iter_mut()
        .find(|entity| entity.path.role == ExtrusionRole::Skirt)
        .ok_or_else(|| "mutation witness has no skirt ring".to_string())?;
    if mutation.open_first_skirt_ring {
        ring.path
            .points
            .pop()
            .ok_or_else(|| "mutation witness skirt ring is empty".to_string())?;
    }
    if mutation.shrink_first_skirt_ring {
        let distinct = ring.path.points.len() - 1;
        if distinct < 3 {
            return Err("mutation witness skirt ring has fewer than 3 vertices".to_string());
        }
        let point_count = ring.path.points.len() as f32;
        let cx = ring.path.points.iter().map(|p| p.x).sum::<f32>() / point_count;
        let cy = ring.path.points.iter().map(|p| p.y).sum::<f32>() / point_count;
        for point in &mut ring.path.points {
            point.x = cx + (point.x - cx) * 0.5;
            point.y = cy + (point.y - cy) * 0.5;
        }
    }
    Ok(())
}

fn check_observed(
    case: &SkirtBrimCase,
    observed: &[GeneratedEntity],
    model: &SkirtBrimModel,
) -> Result<Summary, String> {
    let spacing = model_spacing(case.line_width);
    let degenerate = case.rect.is_degenerate();
    // A disabled module, an empty footprint and a zero brim all suppress every
    // brim: `generate` never builds the annulus there.
    let brim_suppressed = !case.enabled || degenerate || case.brim_width <= 0.0;
    let half_bands = f64::from(case.brim_width) / spacing / 2.0;
    let bands_floor = if !brim_suppressed && half_bands > 0.0 {
        half_bands.floor() as u32
    } else {
        0
    };
    let boundary_class =
        !brim_suppressed && (half_bands - half_bands.round()).abs() < QUANTISATION_GUARD;

    let skirt_entities: Vec<&GeneratedEntity> = observed
        .iter()
        .filter(|entity| entity.path.role == ExtrusionRole::Skirt)
        .collect();
    let brim_entities: Vec<&GeneratedEntity> = observed
        .iter()
        .filter(|entity| entity.path.role == ExtrusionRole::Brim)
        .collect();
    if let Some(other) = observed
        .iter()
        .find(|entity| !matches!(entity.path.role, ExtrusionRole::Skirt | ExtrusionRole::Brim))
    {
        return Err(format!(
            "generate emitted role {:?}, only Skirt and Brim are legal",
            other.path.role
        ));
    }
    // Canonical order: the priming skirt precedes the adhesion brim.
    let last_skirt = observed
        .iter()
        .rposition(|entity| entity.path.role == ExtrusionRole::Skirt);
    let first_brim = observed
        .iter()
        .position(|entity| entity.path.role == ExtrusionRole::Brim);
    if let (Some(last_skirt), Some(first_brim)) = (last_skirt, first_brim) {
        if last_skirt > first_brim {
            return Err(format!(
                "generate emitted a skirt ring at index {last_skirt} after a brim ring at \
                 {first_brim}; the skirt must come first"
            ));
        }
    }

    let mut summary = Summary {
        skirt_rings: u32::try_from(skirt_entities.len()).unwrap_or(u32::MAX),
        brim_rings: u32::try_from(brim_entities.len()).unwrap_or(u32::MAX),
        degenerate,
        boundary_class,
        ..Default::default()
    };

    // ---- Counts -----------------------------------------------------------
    let skirt_layers = (SKIRT_HEIGHT as usize).min(case.layer_count as usize);
    let expected_skirt = if !case.enabled || degenerate {
        0i64
    } else {
        i64::from(case.skirt_loops) * skirt_layers as i64
    };
    let expected_skirt = (expected_skirt + i64::from(model.loop_count_delta)).max(0) as usize;
    if skirt_entities.len() != expected_skirt {
        return Err(format!(
            "expected {expected_skirt} Skirt rings for {} loops x {skirt_layers} layers, got {}",
            case.skirt_loops,
            skirt_entities.len()
        ));
    }

    let brim_floor = if bands_floor == 0 {
        0i64
    } else {
        2 * i64::from(bands_floor - 1)
    };
    let brim_exact = 2 * i64::from(bands_floor);
    // Three count regimes: `boundary_class` accepts the two legal floors, the
    // classified thin tail adds fragments on top of the exact count, and
    // everything else is exact.
    let tail_classified = !brim_suppressed && !boundary_class && thin_tail_classified(spacing);
    let observed_brim = brim_entities.len() as i64;
    let delta = i64::from(model.brim_count_delta);
    if brim_suppressed {
        let expected = delta.max(0);
        if observed_brim != expected {
            return Err(format!(
                "expected {expected} Brim rings for a suppressed brim, got {observed_brim}"
            ));
        }
    } else if boundary_class {
        let low = (brim_floor + delta).max(0);
        let high = (brim_exact + delta).max(0);
        if observed_brim < low || observed_brim > high {
            return Err(format!(
                "expected {low}..={high} Brim rings (quantisation boundary) for brim_width \
                 {} mm at spacing {spacing:.6} mm, got {observed_brim}",
                case.brim_width
            ));
        }
    } else if tail_classified {
        // The last band is exactly one spacing wide and may split; the fragment
        // count is clipper's boolean result, so only the lower bound (one ring
        // per band boundary) is asserted here and the excess is recorded as
        // `tail_fragments` (see `thin_tail_classified`). Every fragment is
        // validated individually in the brim block below.
        let low = (brim_exact + delta).max(0);
        if observed_brim < low {
            return Err(format!(
                "expected at least {low} Brim rings (classified thin tail: one ring per \
                 band boundary) for brim_width {} mm at spacing {spacing:.6} mm, got \
                 {observed_brim}",
                case.brim_width
            ));
        }
    } else {
        let expected = (brim_exact + delta).max(0);
        if observed_brim != expected {
            return Err(format!(
                "expected {expected} Brim rings for brim_width {} mm at spacing \
                 {spacing:.6} mm, got {observed_brim}",
                case.brim_width
            ));
        }
    }
    let bands_effective = if boundary_class {
        // Only in the classified boundary case does the geometry follow the
        // observed floor: both neighbours are legal quantisations there.
        u32::try_from(observed_brim / 2).unwrap_or(0)
    } else {
        debug_assert!(
            observed_brim == brim_exact || (tail_classified && observed_brim > brim_exact),
            "the non-classified count path pins the exact count"
        );
        bands_floor
    };
    summary.bands = bands_effective;
    summary.tail_classified = tail_classified;
    summary.tail_fragments = if tail_classified {
        u32::try_from(observed_brim - brim_exact).unwrap_or(0)
    } else {
        0
    };

    // `annex` is the hull inflation over the footprint: the brim area's outer
    // boundary when a brim area exists, zero otherwise.
    let annex = if observed_brim > 0 {
        f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5
            + f64::from(BRIM_OBJECT_GAP_MM)
            + 2.0 * f64::from(bands_effective) * spacing
    } else {
        0.0
    };
    let anchor = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5 + model.anchor_delta_mm;
    let gap = f64::from(BRIM_OBJECT_GAP_MM);
    let quantised = 2.0 * f64::from(bands_effective) * spacing;
    let decimation_classified = observed_brim > 0 && decimation_classified(&case.rect, quantised);
    summary.decimation_classified = decimation_classified;
    // Brim rings derive from one of two boundaries: the annulus outer (which
    // inherits the decimated contour corner chord, so its extent can only move
    // *inward*) and the intact contour (the hole rings, exact both ways).
    let contour_env = contour_anchored_env_mm();
    let (outer_lower_env, outer_upper_env) = outer_derived_env_mm(&case.rect, quantised);

    if skirt_entities.is_empty() && brim_entities.is_empty() {
        return Ok(summary);
    }

    // ---- Skirt ------------------------------------------------------------
    let witness = case.rect.scaled(model.enclosure_scale);
    let sample_step = 0.5 * spacing.min(0.5);
    let (samples, capped) = witness_samples(&witness, sample_step);
    summary.coarse_sampled = capped;

    let mut index = 0usize;
    for layer in 0..skirt_layers {
        let layer_index = case.base_layer_index + layer as u32;
        let layer_z = FIRST_LAYER_HEIGHT_MM * (layer + 1) as f32;
        let rings: Vec<&&GeneratedEntity> = skirt_entities
            .iter()
            .skip(index)
            .take(case.skirt_loops as usize)
            .collect();
        index += case.skirt_loops as usize;
        let mut layer_extents: Vec<f64> = Vec::with_capacity(rings.len());
        if rings.len() != case.skirt_loops as usize {
            return Err(format!(
                "layer {layer} carried {} skirt rings, expected {}",
                rings.len(),
                case.skirt_loops
            ));
        }
        for (order, entity) in rings.iter().enumerate() {
            // Outermost first: order 0 is loop `skirt_loops - 1`.
            let loop_index = case.skirt_loops as i64 - 1 - order as i64;
            let model_order = if model.ascending_skirt_order {
                order as i64
            } else {
                loop_index
            };
            let distance =
                f64::from(case.skirt_distance) + spacing * 0.5 + model_order as f64 * spacing;
            let label = format!("skirt ring {order} on layer {layer}");
            if entity.layer_pos != layer {
                return Err(format!(
                    "{label}: layer_pos {} does not match its layer",
                    entity.layer_pos
                ));
            }
            if entity.region_key.object_id != "__skirt__"
                || entity.region_key.region_id != 0
                || !entity.region_key.variant_chain.is_empty()
                || entity.region_key.global_layer_index != layer_index
            {
                return Err(format!(
                    "{label}: region key {:?} must be __skirt__/0/empty at layer {layer_index}",
                    entity.region_key
                ));
            }
            if entity.path.speed_factor != 1.0
                || entity.path.tool_index.is_some()
                || entity.path.order_lock.is_some()
            {
                return Err(format!(
                    "{label}: skirt ring must be unlocked and unassigned"
                ));
            }
            for point in &entity.path.points {
                // `flow_spacing` evaluates the same formula in f32, so the
                // stamped width can differ from this f64 derivation by a few
                // ulps; the assertion is still far tighter than any spacing
                // step on the ladder.
                if (point.width - f64::from(case_spacing_f32(spacing)) as f32).abs() > 1.0e-6 {
                    return Err(format!(
                        "{label}: point width {} must be the derived spacing {spacing:.6}",
                        point.width
                    ));
                }
                if point.flow_factor != 1.0 {
                    return Err(format!("{label}: point flow factor must be 1.0"));
                }
                if point.z != layer_z {
                    return Err(format!(
                        "{label}: point z {} must match the layer z {layer_z}",
                        point.z
                    ));
                }
            }
            let measure = measure_ring(&label, &entity.path, &case.rect, model)?;
            // `annex == 0` means the hull is the exact rectangle, whose offset
            // has exact flat extremes. With a brim folded into the hull the
            // skirt ring is an offset of the brim-area outer boundary, so its
            // extent inherits that boundary's decimated corner chord (inward
            // only) and its nearest vertex inherits the corner join's
            // tangential reach (one anchor half-width). The annex-free loop
            // step below stays the exact structural check.
            let (extent_lower_env, extent_upper_env) = if annex == 0.0 {
                (BBOX_ENV_MM, BBOX_ENV_MM)
            } else {
                (outer_lower_env, outer_upper_env)
            };
            check_ring_extremes(
                &label,
                &measure,
                &case.rect,
                annex + distance,
                extent_lower_env,
                extent_upper_env,
                &mut summary,
            )?;
            layer_extents.push(measure.extent_offset);
            // The nearest vertex: a flat vertex sits exactly at `annex +
            // distance`; a round-join vertex around a collapsed contour corner
            // can reach `quantised + distance` (the anchor term is absent there
            // because the join is centred on a contour vertex one anchor
            // half-width off the footprint corner, and its tangential reach is
            // at that radius). With no brim the hull is the exact rectangle and
            // only the skirt's own join tolerance applies.
            let expected_clearance = annex + distance;
            let (lower, upper) = if annex > 0.0 {
                (
                    expected_clearance - anchor - SKIRT_ENV_MM,
                    expected_clearance + SKIRT_ENV_MM,
                )
            } else {
                (
                    expected_clearance - SKIRT_ENV_MM,
                    expected_clearance + SKIRT_ENV_MM,
                )
            };
            let excess = (lower - measure.min_clearance)
                .max(measure.min_clearance - upper)
                .max(0.0);
            summary.max_clearance_dev_mm = summary.max_clearance_dev_mm.max(excess as f32);
            if excess > 0.0 {
                return Err(format!(
                    "{label}: nearest vertex at {:.6} mm (farthest {:.6}) left the derived \
                     window [{lower:.6}, {upper:.6}] mm",
                    measure.min_clearance, measure.max_clearance
                ));
            }
            let ring_points: Vec<(f64, f64)> = entity
                .path
                .points
                .iter()
                .map(|p| (f64::from(p.x), f64::from(p.y)))
                .collect();
            for sample in &samples {
                if !point_in_ring(&ring_points, *sample) {
                    return Err(format!(
                        "{label}: footprint edge sample {sample:?} is outside the ring, so the \
                         skirt does not enclose the whole footprint"
                    ));
                }
            }
            summary.enclosure_loops += 1;
        }
        // Outermost first: the extreme-coordinate offset shrinks by exactly one
        // spacing per loop. This difference is annex-independent, so it stays
        // exact even when the absolute annex carries a classified deviation.
        for pair in layer_extents.windows(2) {
            let step = pair[0] - pair[1];
            if (step - spacing).abs() > REL_ENV_MM {
                return Err(format!(
                    "skirt rings on layer {layer} step by {step:.6} mm between loops, \
                     expected {spacing:.6}; loops must be emitted outermost first"
                ));
            }
        }
    }
    if index != skirt_entities.len() {
        return Err(format!(
            "{} skirt rings were not placed on any layer",
            skirt_entities.len() - index
        ));
    }

    // ---- Brim -------------------------------------------------------------
    if !brim_entities.is_empty() {
        if case.line_width <= 0.0 {
            return Err("a brim cannot exist at a non-positive line width".to_string());
        }
        let layer_zero = case.base_layer_index;
        // The last band is exactly one `spacing` wide; in the classified thin
        // tail clipper may split it (see `thin_tail_classified`). Every earlier
        // band is at least three spacings wide and keeps the exact
        // two-rings-per-band structure.
        let clean_rings = 2 * bands_effective.saturating_sub(1) as usize;
        let mut clearances = Vec::with_capacity(brim_entities.len());
        let mut min_clearances = Vec::with_capacity(brim_entities.len());
        for (index, entity) in brim_entities.iter().enumerate() {
            let label = format!("brim ring {index}");
            if entity.layer_pos != 0 {
                return Err(format!(
                    "{label}: the brim belongs on layer 0 only, got layer_pos {}",
                    entity.layer_pos
                ));
            }
            if entity.region_key.object_id != "brim"
                || entity.region_key.region_id != 0
                || !entity.region_key.variant_chain.is_empty()
                || entity.region_key.global_layer_index != layer_zero
            {
                return Err(format!(
                    "{label}: region key {:?} must be brim/0/empty at layer {layer_zero}",
                    entity.region_key
                ));
            }
            if entity.path.speed_factor != 1.0
                || entity.path.tool_index.is_some()
                || entity.path.order_lock.is_some()
            {
                return Err(format!(
                    "{label}: brim ring must be unlocked and unassigned"
                ));
            }
            for point in &entity.path.points {
                if point.width != case.line_width {
                    return Err(format!(
                        "{label}: point width {} must be the line width {}",
                        point.width, case.line_width
                    ));
                }
                if point.flow_factor != 1.0 {
                    return Err(format!("{label}: point flow factor must be 1.0"));
                }
                if point.z != FIRST_LAYER_HEIGHT_MM {
                    return Err(format!("{label}: point z {} must match layer 0", point.z));
                }
            }
            let measure = measure_ring(&label, &entity.path, &case.rect, model)?;
            let in_tail = summary.tail_classified && index >= clean_rings;
            if !in_tail {
                let band = index / 2;
                let is_outer = index % 2 == 0;
                let depth = (2 * band + 1) as f64 * spacing * 0.5;
                let expected = if is_outer {
                    anchor + gap + quantised - depth
                } else {
                    anchor + gap + depth
                };
                // The hole ring is an offset of the *intact* contour, so it is
                // exact both ways. An outer ring descends from the annulus
                // outer boundary, whose decimated corner chord can only pull it
                // inward.
                let (lower_env, upper_env) = if is_outer {
                    (outer_lower_env, outer_upper_env)
                } else {
                    (contour_env, contour_env)
                };
                check_ring_extremes(
                    &label,
                    &measure,
                    &case.rect,
                    expected,
                    lower_env,
                    upper_env,
                    &mut summary,
                )?;
                // Nearest vertex. A hole ring's flats and corner arcs are all
                // at `anchor + depth`. An outer ring's nearest vertex can also
                // be a round-join vertex around a collapsed contour corner,
                // which sits at `quantised - depth` from the footprint corner
                // (the anchor term is absent there — measured, and pinned by
                // `skirt_brim_corner_join_witness`), so its window is one
                // anchor wide by derivation, not by slack.
                let (lower, upper) = if is_outer {
                    ((quantised - depth) - contour_env, expected + contour_env)
                } else {
                    (expected - contour_env, expected + contour_env)
                };
                let excess = (lower - measure.min_clearance)
                    .max(measure.min_clearance - upper)
                    .max(0.0);
                summary.max_clearance_dev_mm = summary.max_clearance_dev_mm.max(excess as f32);
                if excess > 0.0 {
                    return Err(format!(
                        "{label}: nearest vertex at {:.6} mm left the derived window \
                         [{lower:.6}, {upper:.6}] mm",
                        measure.min_clearance
                    ));
                }
            }
            // Loose authored range over every brim vertex, fragments included.
            if measure.min_clearance < gap - BRIM_RANGE_ENV_MM
                || measure.max_clearance > gap + quantised + BRIM_RANGE_ENV_MM
            {
                return Err(format!(
                    "{label}: vertex clearance {:.6}..={:.6} mm left the authored range \
                     [{:.6}, {:.6}]",
                    measure.min_clearance,
                    measure.max_clearance,
                    gap - BRIM_RANGE_ENV_MM,
                    gap + quantised + BRIM_RANGE_ENV_MM
                ));
            }
            clearances.push(measure.extent_offset);
            min_clearances.push(measure.min_clearance);
        }

        let assignment_env = contour_anchored_env_mm();
        if summary.tail_classified {
            // Only the last band can split (`thin_tail_classified`): every
            // earlier band is at least three spacings wide. So the first
            // `clean_rings` entities keep the exact band structure, and the
            // remainder are fragments of the one-spacing-wide last band.
            let last_band = bands_effective.saturating_sub(1) as usize;
            let last_depth = (2 * last_band + 1) as f64 * spacing * 0.5;
            let last_outer = anchor + gap + quantised - last_depth;
            let last_hole = anchor + gap + last_depth;
            let tail = &min_clearances[clean_rings.min(min_clearances.len())..];
            // The band exists with positive width, so it contributes at least
            // one contour and one hole ring. The fragment count above that is
            // clipper's boolean result and depends on its tessellation, so it is
            // recorded (`tail_fragments`) rather than asserted; every fragment
            // is validated individually below and in the clean-prefix checks.
            if tail.len() < 2 {
                return Err(format!(
                    "the classified thin tail emitted only {} ring(s); the last band needs \
                     its contour and its hole",
                    tail.len()
                ));
            }
            for (offset, clearance) in tail.iter().enumerate() {
                if *clearance < last_hole - assignment_env
                    || *clearance > last_outer + assignment_env
                {
                    return Err(format!(
                        "brim tail fragment {} sits at {clearance:.6} mm, outside the last \
                         band's window [{:.6}, {:.6}]",
                        clean_rings + offset,
                        last_hole - assignment_env,
                        last_outer + assignment_env
                    ));
                }
            }
            // Ordering, end to end: the first emitted ring is the outermost of
            // the sequence and ring 1 is band 0's hole, the innermost ring.
            let farthest = min_clearances.iter().cloned().fold(f64::MIN, f64::max);
            let nearest = min_clearances.iter().cloned().fold(f64::MAX, f64::min);
            if min_clearances[0] < farthest - assignment_env {
                return Err(format!(
                    "brim ring 0 is not the outermost ring: {:.6} mm vs the maximum {farthest:.6}",
                    min_clearances[0]
                ));
            }
            if min_clearances[1] > nearest + assignment_env {
                return Err(format!(
                    "brim ring 1 is not the innermost ring: {:.6} mm vs the minimum {nearest:.6}",
                    min_clearances[1]
                ));
            }
            // The clean prefix still carries the exact band steps; a step
            // touches rings 2b..2b+3, so the loop stops two bands short of the
            // tail.
            for band in 0..bands_effective.saturating_sub(2) as usize {
                let outer_step = clearances[2 * band] - clearances[2 * (band + 1)];
                if (outer_step - spacing).abs() > REL_ENV_MM {
                    return Err(format!(
                        "brim bands {band} and {}: outer rings step by {outer_step:.6} mm, \
                         expected {spacing:.6} (envelope {REL_ENV_MM}); bands must be emitted \
                         outermost first",
                        band + 1
                    ));
                }
                let hole_step = clearances[2 * (band + 1) + 1] - clearances[2 * band + 1];
                if (hole_step - spacing).abs() > REL_ENV_MM {
                    return Err(format!(
                        "brim bands {band} and {}: hole rings step by {hole_step:.6} mm, \
                         expected {spacing:.6} (envelope {REL_ENV_MM})",
                        band + 1
                    ));
                }
            }
        } else if decimation_classified {
            // A ring extreme can move by up to the anchor half-width here, which
            // is wider than one band, so the ordering claim is reduced to the
            // two ends of the sequence.
            let farthest = min_clearances.iter().cloned().fold(f64::MIN, f64::max);
            let nearest = min_clearances.iter().cloned().fold(f64::MAX, f64::min);
            if min_clearances[0] < farthest - assignment_env {
                return Err(format!(
                    "brim ring 0 is not the outermost ring: {:.6} mm vs the maximum {farthest:.6}",
                    min_clearances[0]
                ));
            }
            if min_clearances[1] > nearest + assignment_env {
                return Err(format!(
                    "brim ring 1 is not the innermost ring: {:.6} mm vs the minimum {nearest:.6}",
                    min_clearances[1]
                ));
            }
        } else {
            for band in 0..bands_effective as usize {
                let expected_within = (2 * (bands_effective as usize - band) - 1) as f64 * spacing;
                let within = clearances[2 * band] - clearances[2 * band + 1];
                if (within - expected_within).abs() > REL_ENV_MM {
                    return Err(format!(
                        "brim band {band}: outer ring is {within:.6} mm outside its hole ring, \
                         expected {expected_within:.6} (envelope {REL_ENV_MM})"
                    ));
                }
                if band + 1 < bands_effective as usize {
                    let outer_step = clearances[2 * band] - clearances[2 * (band + 1)];
                    if (outer_step - spacing).abs() > REL_ENV_MM {
                        return Err(format!(
                            "brim bands {band} and {}: outer rings step by {outer_step:.6} mm, \
                             expected {spacing:.6} (envelope {REL_ENV_MM}); bands must be \
                             emitted outermost first",
                            band + 1
                        ));
                    }
                    let hole_step = clearances[2 * (band + 1) + 1] - clearances[2 * band + 1];
                    if (hole_step - spacing).abs() > REL_ENV_MM {
                        return Err(format!(
                            "brim bands {band} and {}: hole rings step by {hole_step:.6} mm, \
                             expected {spacing:.6} (envelope {REL_ENV_MM})",
                            band + 1
                        ));
                    }
                }
            }
        }

        // Nesting: the sawtooth is the documented `tryExPolygonOffset` shape
        // (outer contour then hole boundary per band). It needs the exact
        // two-rings-per-band pairing, which the classified thin tail breaks.
        if !summary.tail_classified {
            let mut sawtooth = true;
            for band in 0..bands_effective as usize {
                let outer = clearances[2 * band];
                let hole = clearances[2 * band + 1];
                if !(outer > hole) {
                    sawtooth = false;
                }
                if band + 1 < bands_effective as usize
                    && !(clearances[2 * band] >= clearances[2 * (band + 1)] - BBOX_ENV_MM)
                {
                    sawtooth = false;
                }
            }
            if sawtooth {
                for band in 0..bands_effective as usize {
                    let outer = clearances[2 * band];
                    let hole = clearances[2 * band + 1];
                    if band + 1 < bands_effective as usize {
                        let next_outer = clearances[2 * (band + 1)];
                        let next_hole = clearances[2 * (band + 1) + 1];
                        if next_outer > outer + BBOX_ENV_MM {
                            return Err(format!(
                                "brim band {}'s outer clearance {outer:.6} is inside band {}'s \
                                 {next_outer:.6}; bands must be emitted outermost first",
                                band,
                                band + 1
                            ));
                        }
                        if hole > next_hole + BBOX_ENV_MM {
                            return Err(format!(
                                "brim band {}'s hole clearance {hole:.6} is outside band {}'s \
                                 {next_hole:.6}",
                                band,
                                band + 1
                            ));
                        }
                    }
                }
            }
        }
        if min_clearances.iter().cloned().fold(f64::MAX, f64::min) <= 0.0 {
            return Err(
                "every brim ring must sit at a positive clearance on the safe \
                        sub-domain"
                    .to_string(),
            );
        }
    }
    Ok(summary)
}

/// The f32 value `SkirtBrim::flow_spacing` stamps on skirt points.
fn case_spacing_f32(spacing: f64) -> f32 {
    spacing as f32
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

type CoreSeed = [u32; 6];

#[test]
fn proptest_skirt_brim_enclosure() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("H-SKIRT-BRIM context");
    let limits = live_limits(&run);
    let ladder = line_width_ladder(&limits);
    let indices: Vec<usize> = (0..ladder.len()).collect();

    let strategy = (
        prop::sample::select(indices).no_shrink(),
        prop::sample::select(RECT_MODES.to_vec()).no_shrink(),
        prop::sample::select(BRIM_CLASSES.to_vec()).no_shrink(),
        prop::sample::select(LOOP_CLASSES.to_vec()).no_shrink(),
        prop::sample::select(DISTANCE_CLASSES.to_vec()).no_shrink(),
        prop::sample::select(LAYER_CLASSES.to_vec()).no_shrink(),
        any::<CoreSeed>(),
    )
        .prop_map(
            move |(
                line_width,
                rect_mode,
                brim_class,
                loop_class,
                distance_class,
                layer_class,
                seed,
            )| {
                build_case(
                    ladder[line_width],
                    rect_mode,
                    brim_class,
                    loop_class,
                    distance_class,
                    layer_class,
                    seed,
                    &limits,
                )
            },
        );

    let cases = Cell::new(0u64);
    let skirt_and_brim = Cell::new(0u64);
    let zero_brim = Cell::new(0u64);
    let zero_loops = Cell::new(0u64);
    let multi_loop = Cell::new(0u64);
    let degenerate = Cell::new(0u64);
    let boundary_cases = Cell::new(0u64);
    let decimation_cases = Cell::new(0u64);
    let thin_tail_cases = Cell::new(0u64);
    let tail_fragments = Cell::new(0u64);
    let fine_sampled = Cell::new(0u64);
    let coarse_sampled = Cell::new(0u64);
    let enclosed_loops = Cell::new(0u64);
    let max_bbox_dev = Cell::new(0.0f32);
    let max_bbox_excess = Cell::new(0.0f32);
    let max_clearance_dev = Cell::new(0.0f32);

    run.check(&strategy, |case, _guard| {
        cases.set(cases.get() + 1);
        let summary = match check_case(
            &case,
            &SkirtBrimModel::default(),
            ObservedMutation::default(),
        ) {
            Ok(summary) => summary,
            Err(message) => return fail(message),
        };
        if summary.skirt_rings > 0 && summary.brim_rings > 0 {
            skirt_and_brim.set(skirt_and_brim.get() + 1);
        }
        if case.brim_class == BrimClass::Zero {
            zero_brim.set(zero_brim.get() + 1);
        }
        if case.skirt_loops == 0 {
            zero_loops.set(zero_loops.get() + 1);
        }
        if summary.skirt_rings >= 2 {
            multi_loop.set(multi_loop.get() + 1);
        }
        if summary.degenerate {
            degenerate.set(degenerate.get() + 1);
        }
        if summary.boundary_class {
            boundary_cases.set(boundary_cases.get() + 1);
        }
        if summary.decimation_classified {
            decimation_cases.set(decimation_cases.get() + 1);
        }
        if summary.tail_classified {
            thin_tail_cases.set(thin_tail_cases.get() + 1);
            tail_fragments.set(tail_fragments.get() + u64::from(summary.tail_fragments));
        }
        if summary.enclosure_loops > 0 {
            if summary.coarse_sampled {
                coarse_sampled.set(coarse_sampled.get() + 1);
            } else {
                fine_sampled.set(fine_sampled.get() + 1);
            }
        }
        enclosed_loops.set(enclosed_loops.get() + u64::from(summary.enclosure_loops));
        max_bbox_dev.set(max_bbox_dev.get().max(summary.max_bbox_dev_mm));
        max_bbox_excess.set(max_bbox_excess.get().max(summary.max_bbox_excess_mm));
        max_clearance_dev.set(max_clearance_dev.get().max(summary.max_clearance_dev_mm));
        Ok(())
    })
    .expect("H-SKIRT-BRIM skirt/brim counts, enclosure and nesting");

    assert!(cases.get() > 0, "no skirt/brim case was generated");
    assert!(
        skirt_and_brim.get() > 0,
        "no case produced a skirt and a brim together"
    );
    assert!(zero_brim.get() > 0, "no case used brim_width = 0");
    assert!(zero_loops.get() > 0, "no case used skirt_loops = 0");
    assert!(
        multi_loop.get() > 0,
        "no case produced at least two skirt loops"
    );
    assert!(degenerate.get() > 0, "no case used a degenerate footprint");
    assert!(
        enclosed_loops.get() > 0,
        "no skirt loop was checked for whole-footprint enclosure"
    );
    assert!(
        fine_sampled.get() > 0,
        "every enclosure witness was coarse-sampled; the fine witness never ran"
    );
    println!(
        "H-SKIRT-BRIM cases={} skirt+brim={} zero_brim={} zero_loops={} multi_loop={} \
         degenerate={} quantisation_boundary={} decimation_classified={} \
         thin_tail={} tail_fragments={} enclosed_loops={} fine={} coarse={} \
         max_bbox_dev={:.6} max_bbox_excess={:.6} max_clearance_dev={:.6}",
        cases.get(),
        skirt_and_brim.get(),
        zero_brim.get(),
        zero_loops.get(),
        multi_loop.get(),
        degenerate.get(),
        boundary_cases.get(),
        decimation_cases.get(),
        thin_tail_cases.get(),
        tail_fragments.get(),
        enclosed_loops.get(),
        fine_sampled.get(),
        coarse_sampled.get(),
        max_bbox_dev.get(),
        max_bbox_excess.get(),
        max_clearance_dev.get(),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls, boundary control and shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn control_case(
    line_width: f32,
    skirt_loops: u32,
    skirt_distance: f32,
    brim_width: f32,
    rect: Rect,
) -> SkirtBrimCase {
    SkirtBrimCase {
        line_width,
        skirt_distance,
        skirt_loops,
        brim_width,
        rect,
        base_layer_index: 0,
        layer_count: 1,
        enabled: true,
        rect_mode: RectMode::Regular,
        brim_class: BrimClass::Mid,
    }
}

fn witness_rect() -> Rect {
    Rect {
        x_min: sixteenth_mm(-160),
        x_max: sixteenth_mm(160),
        y_min: sixteenth_mm(-160),
        y_max: sixteenth_mm(160),
    }
}

/// Deterministic witness for the derived nearest-vertex window (finding M2).
///
/// The brim's **outer** ring descends from `offset(contour, quantised)`. When
/// clipper's `OFFSET_SHORTEST_EDGE_FACTOR * quantised` vertex threshold replaces
/// the reconstruction contour's 90° corner arc with a chord, the contour's
/// corner becomes a vertex one anchor half-width (`0.2 mm`) away from the
/// footprint corner along an edge. The round join around that vertex, radius
/// `quantised`, can therefore place a ring vertex at `quantised - depth` from
/// the footprint *corner* instead of `anchor + quantised - depth` — the anchor
/// term is genuinely absent there, which is why the outer ring's window is one
/// anchor wide **by derivation**.
///
/// This test pins the exact vertex and both ends of the derivation, and pins
/// that the same ring's *axis extreme* is still exact — so the extreme, not the
/// nearest vertex, is the half-wall falsifier. Measured on
/// `line_width = 2.0, brim_width = 63.6875, rect = 43.8125 x 32.625 mm`:
/// ring 0 (band 0 outer) nearest vertex `(-11.658100, -95.147697)` at
/// `61.649048 mm`, i.e. `quantised - depth = 61.648008` plus 0.001; its
/// `min_y` extent is `61.847801` against `anchor + quantised - depth =
/// 61.848008` (2e-4). Ring 1 (band 0 hole) sits at `1.168165` against
/// `anchor + depth = 1.178540` (1.0e-2 < the contour envelope).
#[test]
fn skirt_brim_corner_join_witness() {
    let case = SkirtBrimCase {
        line_width: 2.0,
        skirt_distance: 0.0,
        skirt_loops: 0,
        brim_width: 63.6875,
        rect: Rect {
            x_min: -11.25,
            x_max: 32.5625,
            y_min: -33.5,
            y_max: -0.875,
        },
        base_layer_index: 0,
        layer_count: 1,
        enabled: true,
        rect_mode: RectMode::Regular,
        brim_class: BrimClass::Wide,
    };
    let spacing = model_spacing(case.line_width);
    let anchor = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
    let bands = (f64::from(case.brim_width) / spacing / 2.0).floor() as u64;
    assert_eq!(bands, 16, "the witness carries sixteen bands");
    let quantised = 2.0 * bands as f64 * spacing;
    assert!(
        !flats_decimated(&case.rect, quantised),
        "the witness' contour flats (32 mm) exceed 0.005 * quantised, so its \
         extremes stay exact"
    );
    let contour_env = contour_anchored_env_mm();
    let (lower_env, upper_env) = outer_derived_env_mm(&case.rect, quantised);

    let observed = observe(&case).expect("the witness builds");
    let brim: Vec<&GeneratedEntity> = observed
        .iter()
        .filter(|entity| entity.path.role == ExtrusionRole::Brim)
        .collect();
    assert_eq!(brim.len(), 2 * bands as usize, "two rings per band");

    let depth = spacing * 0.5;
    let outer_expected = anchor + quantised - depth;
    let hole_expected = anchor + depth;

    // ---- ring 0: the outer ring's nearest vertex and exact extreme --------
    let outer = measure_ring(
        "witness outer",
        &brim[0].path,
        &case.rect,
        &SkirtBrimModel::default(),
    )
    .expect("the outer ring is well formed");
    let mut nearest = (f64::MAX, 0.0f64, 0.0f64);
    for point in &brim[0].path.points {
        let (x, y) = (f64::from(point.x), f64::from(point.y));
        let distance = box_distance((x, y), &case.rect);
        if distance < nearest.0 {
            nearest = (distance, x, y);
        }
    }
    let (nearest_clearance, nearest_x, nearest_y) = nearest;
    println!(
        "corner-join witness: outer nearest=({nearest_x:.6},{nearest_y:.6}) \
         clearance={nearest_clearance:.6} expected={outer_expected:.6} \
         quantised-depth={:.6} extent={:.6} anchor={anchor:.6} contour_env={contour_env:.6}",
        quantised - depth,
        outer.extent_offset
    );
    // The nearest vertex is a corner-join vertex at `quantised - depth`.
    assert!(
        (nearest_clearance - (quantised - depth)).abs() <= contour_env,
        "the witness' nearest vertex must sit at `quantised - depth` \
         ({:.6}), got {nearest_clearance:.6}",
        quantised - depth
    );
    // The anchor term is genuinely absent there: the gap to the nominal flat
    // line is one anchor half-width, not an arc tolerance.
    assert!(
        outer_expected - nearest_clearance > anchor - 2.0 * contour_env,
        "the anchor term must be absent at the corner join: expected \
         {outer_expected:.6}, nearest {nearest_clearance:.6}"
    );
    // ... and yet the axis extreme is exact, which is the falsifier.
    assert!(
        (outer.extent_offset - outer_expected).abs() <= contour_env,
        "the outer ring's extent must stay at `anchor + quantised - depth` \
         ({outer_expected:.6}), got {:.6}",
        outer.extent_offset
    );
    assert!(nearest_clearance >= (quantised - depth) - lower_env);
    assert!(outer_expected <= outer_expected + upper_env);

    // ---- ring 1: the hole ring is exact both ways -------------------------
    let hole = measure_ring(
        "witness hole",
        &brim[1].path,
        &case.rect,
        &SkirtBrimModel::default(),
    )
    .expect("the hole ring is well formed");
    println!(
        "corner-join witness: hole nearest={:.6} extent={:.6} expected={hole_expected:.6}",
        hole.min_clearance, hole.extent_offset
    );
    assert!(
        (hole.min_clearance - hole_expected).abs() <= contour_env,
        "the hole ring's nearest vertex must be `anchor + depth` ({hole_expected:.6}), \
         got {:.6}",
        hole.min_clearance
    );
    assert!(
        (hole.extent_offset - hole_expected).abs() <= contour_env,
        "the hole ring's extent must be `anchor + depth` ({hole_expected:.6}), got {:.6}",
        hole.extent_offset
    );
    // Both ends of the derived outer window are exercised by this ring pair.
    let model = SkirtBrimModel::default();
    let summary =
        check_case(&case, &model, ObservedMutation::default()).expect("the witness checks");
    assert_eq!(
        summary.max_clearance_dev_mm, 0.0,
        "the window explains every vertex"
    );
    assert_eq!(
        summary.max_bbox_excess_mm, 0.0,
        "the extent window explains every side"
    );

    // ---- half-anchor falsification ---------------------------------------
    // A uniform half-wall reference error (the `anchor` term off by exactly one
    // anchor half-width) must be rejected: the hole rings and every extent sit
    // inside a bound far tighter than 0.2 mm.
    let error = check_case(
        &case,
        &SkirtBrimModel {
            anchor_delta_mm: anchor,
            ..model
        },
        ObservedMutation::default(),
    )
    .expect_err("a uniform half-anchor reference error must be rejected");
    println!("half-anchor control rejected: {error}");
    assert!(
        error.contains("nearest vertex") || error.contains("min_"),
        "the rejection must come from a derived window, got: {error}"
    );
}

/// The authored rounding-boundary case: `brim_width = 1.19`, `line_width =
/// 0.17`, `first_layer_height = 0.2`. Both operands are deliberately outside
/// the generated domain (not a power of two, not a 1/16 mm multiple).
fn boundary_case() -> SkirtBrimCase {
    control_case(0.17, 0, 0.0, 1.19, witness_rect())
}

#[derive(Debug, Clone)]
struct BoundaryMeasurement {
    entity_count: usize,
    /// `(outer clearance, hole clearance)` per band, in mm.
    bands: Vec<(f64, f64)>,
    /// Number of degenerate (open / duplicate-vertex / zero-area) rings.
    degenerate_rings: usize,
    max_ring_points: usize,
    min_ring_points: usize,
}

fn run_boundary_control() -> (SkirtBrimCase, Vec<GeneratedEntity>, BoundaryMeasurement) {
    let case = boundary_case();
    let observed = observe(&case).expect("the boundary witness builds");
    let mut measurement = BoundaryMeasurement {
        entity_count: observed.len(),
        bands: Vec::new(),
        degenerate_rings: 0,
        max_ring_points: 0,
        min_ring_points: usize::MAX,
    };
    let mut clearances = Vec::new();
    for (index, entity) in observed.iter().enumerate() {
        let label = format!("boundary ring {index}");
        match measure_ring(&label, &entity.path, &case.rect, &SkirtBrimModel::default()) {
            Ok(measure) => {
                measurement.max_ring_points = measurement.max_ring_points.max(measure.points);
                measurement.min_ring_points = measurement.min_ring_points.min(measure.points);
                clearances.push(measure.extent_offset);
            }
            Err(_) => {
                measurement.degenerate_rings += 1;
                clearances.push(f64::NAN);
            }
        }
    }
    for pair in clearances.chunks(2) {
        if pair.len() == 2 {
            measurement.bands.push((pair[0], pair[1]));
        }
    }
    (case, observed, measurement)
}

/// Analytic band offsets of the boundary witness, in mm.
fn boundary_expectation() -> (f64, Vec<(f64, f64)>) {
    let spacing = model_spacing(0.17);
    let half_bands = 1.19 / spacing / 2.0;
    let bands = half_bands.floor() as u32;
    let anchor = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
    let quantised = 2.0 * f64::from(bands) * spacing;
    let offsets = (0..bands)
        .map(|band| {
            let depth = (2 * band + 1) as f64 * spacing * 0.5;
            (anchor + quantised - depth, anchor + depth)
        })
        .collect();
    (quantised, offsets)
}

fn check_boundary_measurement(
    measurement: &BoundaryMeasurement,
    offsets: &[(f64, f64)],
) -> Result<(), String> {
    if measurement.degenerate_rings != 0 {
        return Err(format!(
            "{} boundary ring(s) are degenerate (open, duplicate consecutive vertex, or \
             zero area)",
            measurement.degenerate_rings
        ));
    }
    if measurement.bands.len() != offsets.len() {
        return Err(format!(
            "boundary witness has {} bands, the model expects {}",
            measurement.bands.len(),
            offsets.len()
        ));
    }
    for (band, ((outer, hole), (expected_outer, expected_hole))) in
        measurement.bands.iter().zip(offsets).enumerate()
    {
        if (outer - expected_outer).abs() > BBOX_ENV_MM {
            return Err(format!(
                "boundary band {band} outer clearance {outer:.6} != {expected_outer:.6}"
            ));
        }
        if (hole - expected_hole).abs() > BBOX_ENV_MM {
            return Err(format!(
                "boundary band {band} hole clearance {hole:.6} != {expected_hole:.6}"
            ));
        }
    }
    Ok(())
}

#[test]
fn skirt_brim_rounding_boundary_control() {
    let (case, observed, measurement) = run_boundary_control();
    let (quantised, offsets) = boundary_expectation();
    let spacing = model_spacing(0.17);

    println!(
        "skirt_brim_rounding_boundary_control: brim_width={} line_width={} \
         first_layer_height={} spacing={spacing:.6} quantised={quantised:.6} \
         bands={} entities={} ring_points={}..={}",
        case.brim_width,
        case.line_width,
        FIRST_LAYER_HEIGHT_MM,
        offsets.len(),
        measurement.entity_count,
        measurement.min_ring_points,
        measurement.max_ring_points,
    );
    for (band, (outer, hole)) in measurement.bands.iter().enumerate() {
        println!(
            "  band {band}: outer_offset={outer:.6} hole_offset={hole:.6} \
             depth={:.6}",
            (2 * band + 1) as f64 * spacing * 0.5
        );
    }

    // The exact observed values. `spacing` here is 0.127080 mm, so the
    // authored `ceil(brim_width / line_width)` model would predict 7 loops;
    // the real canonical quantisation yields floor(1.19 / 0.127080 / 2) = 4
    // bands, i.e. 8 ring entities (four outer contours, four hole boundaries).
    assert_eq!(
        measurement.entity_count,
        2 * offsets.len(),
        "the boundary witness emits one outer and one hole ring per band"
    );
    assert_eq!(
        measurement.entity_count, 8,
        "the reviewed 1.19/0.17 case yields eight ring entities"
    );
    assert_eq!(offsets.len(), 4, "the reviewed case yields four bands");
    // The authored review also predicted a final zero offset. The real
    // innermost ring is the last band's hole boundary at `anchor + 7 *
    // spacing / 2`, strictly positive; no zero-offset ring exists. Classified
    // as a mis-prediction, not a defect.
    let innermost = measurement
        .bands
        .last()
        .map(|(_, hole)| *hole)
        .expect("at least one band");
    assert!(
        innermost > 0.2,
        "the innermost hole boundary sits at {innermost:.6} mm, so no zero-offset ring \
         exists; the authored `final zero offset` note is a mis-prediction"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|entity| entity.path.role == ExtrusionRole::Brim)
            .count(),
        8,
        "every boundary ring is a Brim entity"
    );

    // The relaxation must not fire on this control: it asserts the exact count.
    let half_bands = 1.19 / spacing / 2.0;
    assert!(
        (half_bands - half_bands.round()).abs() >= QUANTISATION_GUARD,
        "the boundary control must not be classified as a quantisation boundary"
    );
    check_boundary_measurement(&measurement, &offsets).expect("the boundary offsets hold");
    let summary = check_case(
        &case,
        &SkirtBrimModel::default(),
        ObservedMutation::default(),
    )
    .expect("the boundary witness passes the same checker the property uses");
    assert!(
        !summary.decimation_classified,
        "the boundary control must not take the classified decimation relaxation"
    );
}

#[test]
fn skirt_brim_controls() {
    let honest = SkirtBrimModel::default();
    let case = control_case(0.5, 2, 1.0, 2.0, witness_rect());
    let baseline = check_case(&case, &honest, ObservedMutation::default())
        .expect("the control witness is accepted by the honest checker");
    assert_eq!(baseline.skirt_rings, 2, "the witness has two skirt rings");
    assert_eq!(baseline.brim_rings, 4, "the witness has two brim bands");

    // 1. Off-by-one skirt loop count.
    let error = check_case(
        &case,
        &SkirtBrimModel {
            loop_count_delta: 1,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("an off-by-one skirt loop count must be rejected");
    assert!(error.contains("Skirt rings"), "{error}");

    // 2. Inward (ascending) skirt order.
    let error = check_case(
        &case,
        &SkirtBrimModel {
            ascending_skirt_order: true,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("an innermost-first skirt order must be rejected");
    assert!(error.contains("skirt ring"), "{error}");

    // 3. An open ring: the closing stamp is dropped from a real result.
    let error = check_case(
        &case,
        &honest,
        ObservedMutation {
            open_first_skirt_ring: true,
            ..ObservedMutation::default()
        },
    )
    .expect_err("an open skirt ring must be rejected");
    assert!(error.contains("open"), "{error}");

    // 4a. A lost whole-footprint enclosure witness: the checker is told the
    //     footprint is 25% larger, which no real skirt ring encloses.
    let error = check_case(
        &case,
        &SkirtBrimModel {
            enclosure_scale: 1.25,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("a lost whole-footprint enclosure witness must be rejected");
    assert!(error.contains("enclose the whole footprint"), "{error}");

    // 4b. The same witness fails when the observed skirt ring is shrunk until
    //     the footprint pokes out of it.
    let error = check_case(
        &case,
        &honest,
        ObservedMutation {
            shrink_first_skirt_ring: true,
            ..ObservedMutation::default()
        },
    )
    .expect_err("an inward-shrunk skirt ring must fail enclosure");
    assert!(error.contains("skirt ring"), "{error}");

    // 5. A dropped brim ring.
    let error = check_case(
        &case,
        &SkirtBrimModel {
            brim_count_delta: -1,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("a dropped brim ring must be rejected");
    assert!(error.contains("Brim rings"), "{error}");

    // 6. A fabricated rectangle-shape equality (every loop is four corners).
    let error = check_case(
        &case,
        &SkirtBrimModel {
            assume_rectangle_shape: true,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("a fabricated rectangle-shape equality must be rejected");
    assert!(error.contains("four-corner rectangle"), "{error}");

    // 7. Degenerate and zero modes are closure/count witnesses only.
    let mut degenerate = case.clone();
    degenerate.rect = Rect {
        x_min: sixteenth_mm(0),
        x_max: sixteenth_mm(0),
        y_min: sixteenth_mm(-32),
        y_max: sixteenth_mm(32),
    };
    degenerate.rect_mode = RectMode::DegenerateX;
    let summary = check_case(&degenerate, &honest, ObservedMutation::default())
        .expect("a degenerate footprint produces nothing");
    assert_eq!(summary.skirt_rings, 0);
    assert_eq!(summary.brim_rings, 0);

    let mut disabled = case.clone();
    disabled.enabled = false;
    let summary = check_case(&disabled, &honest, ObservedMutation::default())
        .expect("a disabled module produces nothing");
    assert_eq!(summary.skirt_rings + summary.brim_rings, 0);

    let module = module_for(&case).expect("the control witness builds");
    let empty: Vec<LayerSnapshot<'_>> = Vec::new();
    assert!(
        module.generate(&empty).is_empty(),
        "an empty layer list must produce nothing"
    );

    let mut brim_only = case.clone();
    brim_only.skirt_loops = 0;
    let summary = check_case(&brim_only, &honest, ObservedMutation::default())
        .expect("brim-only is a legal mode");
    assert_eq!(summary.skirt_rings, 0);
    assert!(summary.brim_rings > 0);

    let mut skirt_only = case.clone();
    skirt_only.brim_width = 0.0;
    let summary = check_case(&skirt_only, &honest, ObservedMutation::default())
        .expect("skirt-only is a legal mode");
    assert_eq!(summary.brim_rings, 0);
    assert_eq!(summary.skirt_rings, 2);

    // The clipper-decimation regime is classified rather than asserted: a
    // 1/16 mm-wide footprint under a ~29 mm brim lets the offset's
    // `0.005 * |delta|` vertex threshold eat the contour's own features.
    let skinny = control_case(
        0.5,
        1,
        0.0,
        30.0,
        Rect {
            x_min: sixteenth_mm(-1024),
            x_max: sixteenth_mm(-1023),
            y_min: sixteenth_mm(-480),
            y_max: sixteenth_mm(0),
        },
    );
    let summary = check_case(&skinny, &honest, ObservedMutation::default())
        .expect("the classified decimation witness passes");
    assert!(
        summary.decimation_classified,
        "a 1/16 mm footprint under a 30 mm brim must be classified as a decimation case"
    );
    assert!(
        summary.brim_rings > 0,
        "the decimation witness keeps its brim"
    );
    assert!(
        summary.enclosure_loops > 0,
        "the decimation witness still checks enclosure"
    );

    // The classified thin tail: at `line_width = 1/8` the last brim band is
    // exactly one `spacing` (0.082080 mm) wide, below the derived corner
    // sagitta plus two arc tolerances (0.083579 mm), so clipper may split it.
    // Measured on this exact input: 345 bands, 690 band boundaries, 693 rings
    // (the last band split into 2 contours and 3 sliver holes).
    let thin = SkirtBrimCase {
        line_width: 0.125,
        skirt_distance: 0.0,
        skirt_loops: 1,
        brim_width: 56.6875,
        rect: Rect {
            x_min: sixteenth_mm(-1024),
            x_max: sixteenth_mm(-1023),
            y_min: sixteenth_mm(-1024),
            y_max: sixteenth_mm(-1021),
        },
        base_layer_index: 0,
        layer_count: 1,
        enabled: true,
        rect_mode: RectMode::Regular,
        brim_class: BrimClass::Wide,
    };
    let summary = check_case(&thin, &honest, ObservedMutation::default())
        .expect("the classified thin tail passes the coverage model");
    assert!(
        summary.tail_classified,
        "a 0.082080 mm spacing must be classified as the thin tail"
    );
    assert_eq!(summary.bands, 345, "the witness carries 345 bands");
    assert_eq!(summary.brim_rings, 693, "the witness emits 693 rings");
    assert_eq!(
        summary.tail_fragments, 3,
        "the last band adds three fragment rings"
    );
    // Every fragment is a closed, well-formed ring with non-zero area, and all
    // of them sit on the last band's boundaries.
    let thin_observed = observe(&thin).expect("the thin witness builds");
    let thin_brim: Vec<&GeneratedEntity> = thin_observed
        .iter()
        .filter(|entity| entity.path.role == ExtrusionRole::Brim)
        .collect();
    let spacing_thin = model_spacing(thin.line_width);
    let anchor_thin = f64::from(ANCHOR_POINT_WIDTH_MM) * 0.5;
    let quantised_thin = 2.0 * 345.0 * spacing_thin;
    let last_outer = anchor_thin + quantised_thin - 689.0 * spacing_thin * 0.5;
    let last_hole = anchor_thin + 689.0 * spacing_thin * 0.5;
    let mut fragments = 0usize;
    let mut slivers = 0usize;
    let mut contours = 0usize;
    // `clean_rings` = bands 0..=n-2; band 344 is the classified tail.
    let tail_start = 2 * (345 - 1);
    for (index, entity) in thin_brim.iter().enumerate().skip(tail_start) {
        fragments += 1;
        let points = &entity.path.points;
        let first = points[0];
        let last = *points.last().expect("non-empty");
        assert_eq!(
            (first.x, first.y),
            (last.x, last.y),
            "brim tail fragment {index} must be closed"
        );
        let mut twice_area = 0.0f64;
        for window in points.windows(2) {
            twice_area += f64::from(window[0].x) * f64::from(window[1].y)
                - f64::from(window[1].x) * f64::from(window[0].y);
        }
        let area = (twice_area * 0.5).abs();
        // One fragment is the second contour of the split band (2.12 mm^2); the
        // rest are slivers of 0.0004..0.0016 mm^2. Every one is a real piece of
        // the one-spacing-wide last band, so the asserted facts are "closed,
        // non-zero area, no larger than the band itself".
        assert!(area > 0.0, "brim tail fragment {index} has zero area");
        assert!(
            area < 20.0,
            "brim tail fragment {index} exceeds the last band's own area: {area} mm^2"
        );
        if area < 0.01 {
            slivers += 1;
        } else {
            contours += 1;
        }
        println!(
            "thin-tail fragment {index}: points={} area={area:.9} mm^2 clearance={:.6}",
            points.len(),
            measure_ring(
                &format!("tail fragment {index}"),
                &entity.path,
                &thin.rect,
                &honest,
            )
            .expect("well-formed")
            .min_clearance
        );
        let measure = measure_ring(
            &format!("tail fragment {index}"),
            &entity.path,
            &thin.rect,
            &honest,
        )
        .expect("a tail fragment is a well-formed ring");
        let nearest = (measure.min_clearance - last_outer)
            .abs()
            .min((measure.min_clearance - last_hole).abs());
        assert!(
            nearest <= contour_anchored_env_mm(),
            "tail fragment {index} at {:.6} mm is not on the last band's boundaries",
            measure.min_clearance
        );
    }
    assert_eq!(fragments, 5, "the last band emits five rings");
    assert_eq!(contours, 2, "the split last band emits two contours");
    assert_eq!(slivers, 3, "and three sub-print sliver holes");
    // Dropping one brim ring shifts the whole band sequence, which the exact
    // clean-prefix structure must reject even though 692 rings is still inside
    // the classified count window.
    let error = check_case(
        &thin,
        &honest,
        ObservedMutation {
            drop_first_brim_ring: true,
            ..ObservedMutation::default()
        },
    )
    .expect_err("a dropped brim ring must shift the band structure");
    assert!(error.contains("brim ring 0"), "{error}");

    // The boundary helper is the same one the boundary control test uses, and
    // the same checker rejects a dropped brim ring on it.
    let (boundary, _observed, measurement) = run_boundary_control();
    let (_quantised, offsets) = boundary_expectation();
    check_boundary_measurement(&measurement, &offsets)
        .expect("the boundary helper agrees with the analytic offsets");
    assert!(measurement.bands.len() >= 2, "the boundary witness nests");
    let error = check_case(
        &boundary,
        &SkirtBrimModel {
            brim_count_delta: -1,
            ..honest
        },
        ObservedMutation::default(),
    )
    .expect_err("a dropped brim ring on the boundary witness must be rejected");
    assert!(error.contains("Brim rings"), "{error}");

    skirt_brim_witness_probe();
}

/// One probe witness: the structural modes that must survive shrinking.
#[derive(Debug, Clone, Copy)]
struct WitnessModes {
    rect_mode: RectMode,
    skirt_loops: u32,
    brim_width_sixteenths: i64,
    layer_count: u32,
}

fn probe_case(modes: WitnessModes, seed: [u32; 3], limits: &Limits) -> SkirtBrimCase {
    let max_sixteenths = (f64::from(limits.max_abs_mm) * SIXTEENTHS_PER_MM).round() as i64;
    let ax = sixteenth_value(seed[0], max_sixteenths);
    let bx = sixteenth_value(seed[1], max_sixteenths);
    let ay = sixteenth_value(seed[2], max_sixteenths);
    let by = sixteenth_value((seed[0] >> 8) ^ seed[2], max_sixteenths);
    let (mut x_min, mut x_max) = (ax.min(bx), ax.max(bx));
    let (mut y_min, mut y_max) = (ay.min(by), ay.max(by));
    match modes.rect_mode {
        RectMode::Regular => {
            if x_min == x_max {
                if x_max < max_sixteenths {
                    x_max += 1;
                } else {
                    x_min -= 1;
                }
            }
            if y_min == y_max {
                if y_max < max_sixteenths {
                    y_max += 1;
                } else {
                    y_min -= 1;
                }
            }
        }
        RectMode::DegenerateX => x_max = x_min,
        RectMode::DegenerateY => y_max = y_min,
    }
    SkirtBrimCase {
        line_width: 0.5,
        skirt_distance: 1.0,
        skirt_loops: modes.skirt_loops,
        brim_width: sixteenth_mm(modes.brim_width_sixteenths),
        rect: Rect {
            x_min: sixteenth_mm(x_min),
            x_max: sixteenth_mm(x_max),
            y_min: sixteenth_mm(y_min),
            y_max: sixteenth_mm(y_max),
        },
        base_layer_index: 0,
        layer_count: modes.layer_count,
        enabled: true,
        rect_mode: modes.rect_mode,
        brim_class: BrimClass::Mid,
    }
}

fn check_witness(case: &SkirtBrimCase, modes: WitnessModes) -> Summary {
    assert_eq!(case.rect_mode, modes.rect_mode, "the rect mode changed");
    assert_eq!(case.skirt_loops, modes.skirt_loops, "the loop mode changed");
    assert_eq!(
        case.layer_count, modes.layer_count,
        "the layer mode changed"
    );
    assert_eq!(
        case.brim_width,
        sixteenth_mm(modes.brim_width_sixteenths),
        "the brim width mode changed"
    );
    let summary = check_case(
        case,
        &SkirtBrimModel::default(),
        ObservedMutation::default(),
    )
    .expect("the witness case checks");
    let degenerate = modes.rect_mode != RectMode::Regular;
    if degenerate {
        assert_eq!(
            summary.skirt_rings + summary.brim_rings,
            0,
            "a degenerate witness gained an entity"
        );
    } else {
        assert_eq!(
            summary.skirt_rings, modes.skirt_loops,
            "the skirt loop count did not survive"
        );
        // spacing(0.5) = 0.457080 mm; a 2.0 mm brim quantises to two bands.
        assert_eq!(
            summary.brim_rings, 4,
            "the four-ring brim mode did not survive"
        );
    }
    summary
}

fn skirt_brim_witness_probe() {
    let limits = control_limits();
    let witnesses = [
        WitnessModes {
            rect_mode: RectMode::Regular,
            skirt_loops: 2,
            brim_width_sixteenths: 32,
            layer_count: 2,
        },
        WitnessModes {
            rect_mode: RectMode::DegenerateX,
            skirt_loops: 2,
            brim_width_sixteenths: 32,
            layer_count: 1,
        },
        WitnessModes {
            rect_mode: RectMode::Regular,
            skirt_loops: 0,
            brim_width_sixteenths: 32,
            layer_count: 1,
        },
    ];

    for modes in witnesses {
        let strategy = any::<[u32; 3]>().prop_map(move |seed| probe_case(modes, seed, &limits));
        let mut runner = TestRunner::deterministic();
        let mut tree = strategy.new_tree(&mut runner).expect("witness tree");
        check_witness(&tree.current(), modes);
        let initial = format!("{:?}", tree.current());
        let mut simplified = 0usize;
        let mut changed = false;
        while tree.simplify() {
            simplified += 1;
            let current = tree.current();
            check_witness(&current, modes);
            changed |= format!("{current:?}") != initial;
            assert!(
                tree.complicate(),
                "complicate must restore the last simplification"
            );
            check_witness(&tree.current(), modes);
        }
        assert!(
            simplified > 0,
            "false-only shrink probe for the {modes:?} witness"
        );
        assert!(
            changed,
            "no allowed reduction changed the {modes:?} witness from its nonminimal start"
        );
        println!("witness {modes:?}: simplify={simplified} changed={changed}");
    }

    // A shrinking mode tree could switch a witness's class mid-search.
    assert_mode_selector_does_not_shrink(RECT_MODES.to_vec());
    assert_mode_selector_does_not_shrink(BRIM_CLASSES.to_vec());
    assert_mode_selector_does_not_shrink(LOOP_CLASSES.to_vec());
    assert_mode_selector_does_not_shrink(DISTANCE_CLASSES.to_vec());
    assert_mode_selector_does_not_shrink(LAYER_CLASSES.to_vec());
}

fn assert_mode_selector_does_not_shrink<T>(modes: Vec<T>)
where
    T: Clone + std::fmt::Debug + 'static,
{
    let mut runner = TestRunner::deterministic();
    let strategy = prop::sample::select(modes).no_shrink();
    let mut tree = strategy.new_tree(&mut runner).expect("mode-selector tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "the mode selector simplified");
    assert!(!tree.complicate(), "the mode selector complicated");
    assert_eq!(format!("{:?}", tree.current()), before);
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit checks on the oracle itself
// ─────────────────────────────────────────────────────────────────────────────

/// The witness sampler and the ray caster are the oracle's own load-bearing
/// parts, so they get their own falsifiable witness (a negative control for the
/// shared geometric predicate, per `docs/22_test_quality.md` §3).
#[test]
fn skirt_brim_oracle_self_check() {
    let square: [(f64, f64); 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    assert!(point_in_ring(&square, (5.0, 5.0)), "the centre is inside");
    assert!(point_in_ring(&square, (0.0, 5.0)), "an edge point is on");
    assert!(point_in_ring(&square, (10.0, 10.0)), "a corner is on");
    assert!(!point_in_ring(&square, (10.5, 5.0)), "outside is outside");
    assert!(
        !point_in_ring(&square, (11.0, 11.0)),
        "a diagonal outside point is outside"
    );
    assert!(!point_in_ring(&[(0.0, 0.0), (1.0, 1.0)], (0.5, 0.5)));

    let rect = Rect {
        x_min: -1.0,
        x_max: 1.0,
        y_min: -1.0,
        y_max: 1.0,
    };
    assert_eq!(box_distance((2.0, 0.0), &rect), 1.0);
    assert_eq!(box_distance((2.0, 2.0), &rect), 2f64.sqrt());
    let (samples, capped) = witness_samples(&rect, 0.25);
    assert!(!capped, "a small rectangle is sampled at the fine step");
    assert!(samples.len() >= 33, "every edge is sampled");
    for sample in &samples {
        assert!(
            box_distance(*sample, &rect) <= 1e-9,
            "every sample lies on the edge set"
        );
    }
}
