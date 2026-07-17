//! # Arachne parity audit — red tests against `parity/arachne`.
//!
//! This file is the **deliverable** of a read-only audit of the Pinch 'n Print
//! Arachne implementation on `parity/arachne` against the canonical OrcaSlicer
//! reference at `F:\slicerProject\pinch_n_print\OrcaSlicerDocumented`.
//!
//! **Framing:** PnP is a modular pipeline (`Layer::Perimeters` is filled by a
//! claim holder; today either `classic-perimeters` or `arachne-perimeters`).
//! Gaps fall into three categories:
//!
//! - **GAP_ARACHNE_PATH** — feature implemented in `classic-perimeters` /
//!   downstream, but NOT in `arachne-perimeters`. The pipeline reaches Orca
//!   parity via Classic; the Arachne path diverges. Tracked as
//!   `D-104-OVERHANG-QUARTILE-NONE` and this test suite.
//! - **GAP_PIPELINE** — feature absent from the PnP pipeline as a whole.
//!   Proposed new deviations: D-104b/c/d/e/f (see
//!   `tmp/arachne_parity_audit_20260709.md`).
//! - **STALE_DOC** — behavior implemented but a manifest description string
//!   or module doc-comment still claims otherwise (doc-hygiene only).
//!
//! Intentional design choices (detect_thin_wall default, fill_outline_gaps
//! config-key existence, wall_generator default classic) are documented in the
//! audit doc but NOT locked by tests in this file — they are deliberate
//! divergences, not defects.
//!
//! Every test below **fails on purpose** on the current `parity/arachne` tree,
//! panicking with a `PARITY GAP: <category>: <feature> | expected: <orcaslicer
//! behavior> | got: <current behavior> | ref: <OrcaSlicer path:line>` message.
//! The failure message *is* the deliverable.
//!
//! Coordinate convention (`docs/08_coordinate_system.md`): 1 unit = 100 nm =
//! 10⁻⁴ mm. OrcaSlicer uses 1 µm units; mm constants are ported via
//! `Point2::from_mm` / `mm_to_units`. All config keys are snake_case.
//!
//! Test naming: `arachne_parity_<category>_<feature>_<expectation>`.
//! See `tmp/arachne_parity_audit_20260709.md` for the full gap inventory.

#![allow(dead_code)]

/// Reads the Arachne module manifest TOML (relative to this test file).
const ARACHNE_MANIFEST_TEXT: &str =
    include_str!("../../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml");

/// The Arachne module source body (relative to this test file), so the
/// arachne-path-specific gaps (classify_line, WallLoop construction) can be
/// asserted without spawning a WASM guest.
const ARACHNE_MODULE_SRC: &str =
    include_str!("../../../modules/core-modules/arachne-perimeters/src/lib.rs");

/// The Classic perimeter module source body (relative to this test file), so
/// we can confirm the pipeline as a whole reaches Orca parity via Classic.
const CLASSIC_MODULE_SRC: &str =
    include_str!("../../../modules/core-modules/classic-perimeters/src/lib.rs");

#[path = "fixtures/arachne_parity/mod.rs"]
mod fixtures;

use arachne_perimeters::ArachnePerimeters;
use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_core::perimeter_utils::point_in_any_polygon;
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    mm_to_units, point_in_polygon_winding, units_to_mm, ConfigView, ExPolygon, ExtrusionLine,
    LoopType, Point2, WallBoundaryType,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Parse the module manifest TOML into a `toml::Value` for key-presence tests.
fn manifest() -> toml::Value {
    toml::from_str(ARACHNE_MANIFEST_TEXT).expect("arachne-perimeters.toml must parse as valid TOML")
}

/// Run the Arachne pipeline against `polygons` with default params and return
/// the produced `ExtrusionLine`s (toolpaths) plus the inner-contour markers.
fn arachne_lines(polygons: &[ExPolygon]) -> (Vec<ExtrusionLine>, Vec<ExtrusionLine>) {
    run_arachne_pipeline(polygons, &ArachneParams::default(), false)
        .expect("Arachne pipeline should succeed on a well-formed fixture polygon")
}

/// True when the manifest's `[config.schema.<key>]` section is present.
fn manifest_has_config_key(key: &str) -> bool {
    manifest()
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.as_table())
        .map(|t| t.contains_key(key))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// AC-9 harness: drives `ArachnePerimeters::run_perimeters` natively via the
// `LayerModule` trait, mirroring the canonical tdd fixtures under
// `modules/core-modules/arachne-perimeters/tests/`.
// ---------------------------------------------------------------------------

/// Config with a nominal wall_count + bead width, no thin-wall detection.
/// Mirrors `arachne_parity_outer_wall_boundary_type_tdd.rs::make_config` /
/// `arachne_parity_is_bridge_flag_tdd.rs::make_config` /
/// `arachne_parity_overhang_quartile_tdd.rs::make_config` /
/// `arachne_parity_seam_candidate_tdd.rs::make_config`.
fn native_wall_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        .build()
}

/// Config with a nominal 0.4mm bead width and thin-wall detection toggled.
/// Mirrors `arachne_parity_thin_wall_loop_type_tdd.rs::make_config` /
/// `arachne_parity_is_thin_wall_flag_tdd.rs::make_config`.
fn native_thin_wall_config(detect_thin_wall_on: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .bool("detect_thin_wall", detect_thin_wall_on)
        .build()
}

/// A plain square region, centered at origin. Mirrors the tdd files' own
/// `make_region(side_mm, z)`.
fn native_square_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// A 0.25mm x 5mm thin strip, narrower than one full 0.4mm bead. Mirrors
/// `arachne_parity_thin_wall_loop_type_tdd.rs::make_thin_strip_region`.
fn native_thin_strip_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(rect_polygon(0.0, 0.0, 0.25, 5.0))
        .build()
}

/// A square region with a bridge area that overlaps the emitted walls. Mirrors
/// `arachne_parity_is_bridge_flag_tdd.rs::make_region`.
///
/// The `bridge_side_mm` parameter is **intentionally ignored** — its `_` prefix
/// signals this. The strict point-in-polygon bridge detector
/// (`crates/slicer-core/src/perimeter_utils.rs:608`) requires wall vertices to
/// lie STRICTLY inside the bridge area. Under the packet-151 `wall_count →
/// max_bead_count = 2 × wall_count` wiring, the emitted walls on a centered
/// 10×10 region are at half-widths ~5 (Outer) and ~4.6 (Inner), so a 4×4
/// centered bridge area (the old default) never intersects any wall and no
/// vertex ever gets `is_bridge=true`. We hardcode a 12×12 bridge area (larger
/// than the region) so every wall vertex is strictly inside it, exercising both
/// the `is_bridge=true → flow_factor=bridge_flow` branch and the
/// `is_bridge=false → flow_factor=1.0` branch (the latter via vertices outside
/// the region, which Arachne clips). Callers that compare against the bridge
/// area (e.g. Caller B's local `bridge_areas`) MUST use the same 12.0 value.
fn native_bridge_region(side_mm: f32, _bridge_side_mm: f32, z: f32) -> SliceRegionView {
    let bridge_side_mm = 12.0_f32;
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .bridge_areas(vec![square_polygon(0.0, 0.0, bridge_side_mm)])
        .build()
}

/// A square region with a smaller centered overhang-quartile band. Mirrors
/// `arachne_parity_overhang_quartile_tdd.rs::make_region`.
fn native_overhang_region(
    side_mm: f32,
    band_side_mm: f32,
    quartile: u8,
    z: f32,
) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .overhang_quartile_polygons(vec![QuartileBand {
            quartile,
            polygons: vec![square_polygon(0.0, 0.0, band_side_mm)],
        }])
        .build()
}

// ===========================================================================
// STALE_DOC: wall_transition_filter_deviation manifest description
// ====================================================================================

/// STALE_DOC: `wall_transition_filter_deviation` IS consumed by compute
/// (D-143 closed — `ArachneParams.transition_filter_dist` →
/// `BeadingFactoryParams` → `DistributedBeadingStrategy::get_transition_filter_dist`,
/// used by `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs:162`
/// and `propagation.rs:670,719`). The manifest description at
/// `arachne-perimeters.toml:64` still says "reserved, not yet read by compute".
///
/// OrcaSlicer ref: `PrintConfig.cpp:7180-7193`; `SkeletalTrapezoidation.cpp:820-956`.
#[test]
fn arachne_parity_stale_doc_wall_transition_filter_deviation_description() {
    let m = manifest();
    let desc = m
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get("wall_transition_filter_deviation"))
        .and_then(|k| k.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("");
    assert!(
        !desc
            .to_lowercase()
            .contains("reserved, not yet read by compute"),
        "PARITY GAP: stale_doc: wall_transition_filter_deviation manifest doc accurate | \
         expected: the manifest description reflects that the key IS consumed \
         by the compute path (D-143 closed; transition_filter_dist wired \
         through BeadingFactoryParams -> DistributedBeadingStrategy::\
         get_transition_filter_dist, used by centrality.rs:162 and \
         propagation.rs:670,719) | got: arachne-perimeters.toml:64's \
         [config.schema.wall_transition_filter_deviation].description still \
         reads \"reserved, not yet read by compute\" — stale since D-143 | ref: \
         PrintConfig.cpp:7180-7193"
    );
}

// ===========================================================================
// STALE_DOC: detect_thin_wall manifest citation (false PrintConfig.cpp coBool ref)
// ====================================================================================

/// STALE_DOC: the `detect_thin_wall` description falsely cites
/// `PrintConfig.cpp:6299-6305` as the OrcaSlicer coBool provenance. That
/// PrintConfig.cpp coBool does not exist in the OrcaSlicer reference;
/// OrcaSlicer's `fill_outline_gaps` is a hardcoded `constexpr bool` in
/// `WallToolPaths.hpp:18`, not a `coBool` in `PrintConfig.cpp`.
///
/// OrcaSlicer ref: `WallToolPaths.hpp:18`; `WallToolPaths.cpp:76`.
#[test]
fn arachne_parity_stale_doc_fill_outline_gaps_manifest_citation() {
    let m = manifest();
    let desc = m
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get("detect_thin_wall"))
        .and_then(|k| k.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("");
    assert!(
        !desc.to_lowercase().contains("printconfig.cpp"),
        "PARITY GAP: stale_doc: fill_outline_gaps manifest citation accurate | \
         expected: OrcaSlicer's fill_outline_gaps is a hardcoded constexpr \
         bool = true (WallToolPaths.hpp:18), NOT a coBool in PrintConfig.cpp; \
         the manifest description should not cite a PrintConfig.cpp coBool | \
         got: arachne-perimeters.toml:182's \
         [config.schema.detect_thin_wall].description falsely cites \
         PrintConfig.cpp:6299-6305 as the OrcaSlicer provenance — that \
         coBool does not exist in the OrcaSlicer reference | ref: \
         WallToolPaths.hpp:18"
    );
}

// ===========================================================================
// STALE_DOC: manifest description + display-name
// ====================================================================================

/// STALE_DOC: `display-name = "...(skeleton)"` and
/// `description = "...walls not yet produced, see P112"` are stale. P112/P141–P147
/// ship real wall generation; the description should reflect that.
#[test]
fn arachne_parity_stale_doc_manifest_description_not_stale() {
    let m = manifest();
    let desc = m
        .get("module")
        .and_then(|m| m.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("");
    let display_name = m
        .get("module")
        .and_then(|m| m.get("display-name"))
        .and_then(|d| d.as_str())
        .unwrap_or("");
    assert!(
        !desc.to_lowercase().contains("walls not yet produced")
            && !display_name.to_lowercase().contains("skeleton"),
        "PARITY GAP: stale_doc: manifest description + display-name not stale | \
         expected: the manifest description and display-name reflect that \
         Arachne walls ARE produced (P112/P141–P147 shipped wall generation) | \
         got: arachne-perimeters.toml [module].description = \"{desc}\", \
         [module].display-name = \"{display_name}\" | ref: n/a (hygiene)"
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-5): overhang_quartile populated
// per vertex
// ====================================================================================

/// `overhang_quartile` is now populated per-vertex in the arachne path via
/// `region.overhang_quartile_polygons()` band lookup, mirroring
/// `perimeter_utils::expolygon_to_path3d`
/// (`crates/slicer-core/src/perimeter_utils.rs:316-331`). Rewritten (packet
/// 148 AC-9) to drive `ArachnePerimeters::run_perimeters` natively instead of
/// substring-matching source text — mirrors
/// `arachne_parity_overhang_quartile_tdd.rs`.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:2113-2119`, `:370-460`, `:1117-1453`.
#[test]
fn arachne_parity_arachne_path_overhang_quartile_set_per_vertex() {
    let config = native_wall_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_overhang_region(10.0, 4.0, 3, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let band_polygon = square_polygon(0.0, 0.0, 4.0);
    let mut checked_any_point = false;

    for wall in output.wall_loops() {
        for pt in &wall.path.points {
            checked_any_point = true;
            let inside = point_in_polygon_winding(&band_polygon, pt.x as f64, pt.y as f64, 0.0);
            let expected = if inside { Some(3u8) } else { None };
            assert_eq!(
                pt.overhang_quartile, expected,
                "vertex at ({}, {}) mm: expected overhang_quartile == {:?} \
                 (point-in-band == {}), got {:?}",
                pt.x, pt.y, expected, inside, pt.overhang_quartile
            );
        }
    }

    assert!(
        checked_any_point,
        "expected at least one path point to verify"
    );
}

// ===========================================================================
// GAP_PIPELINE (CLOSED, packet 149 D4): bridge flow_factor on overhang
// ====================================================================================

/// `flow_factor` is now reduced for bridge vertices via
/// `slicer_core::flow::bridging_flow(bridge_flow, thick_bridges)`, applied
/// per-vertex in arachne wherever `feature_flags[i].is_bridge == true`
/// (mirrors `arachne_parity_arachne_path_is_bridge_flag_set_per_vertex`'s own
/// `region.bridge_areas()` fixture). Rewritten (packet 149) to drive
/// `ArachnePerimeters::run_perimeters` natively with a `bridge_areas`
/// fixture — the original drove the HOST `arachne_lines` pipeline directly
/// on a bridgeless square and asserted on `junctions[].p.flow_factor`, which
/// can never observe this guest-side (module) fix.
///
/// OrcaSlicer ref: `LayerRegion.cpp:135` (`bridging_flow(frPerimeter,
/// thick_bridges)`).
#[test]
fn arachne_parity_pipeline_bridge_flow_factor_on_overhang() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .float("bridge_flow", 0.7)
        .bool("thick_bridges", false)
        .build();
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_bridge_region(10.0, 4.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let mut found_bridge_vertex = false;
    for wall in output.wall_loops() {
        // is_bridge is only ever set on Outer/Inner walls (packet 148 AC-4).
        if !matches!(wall.loop_type, LoopType::Outer | LoopType::Inner) {
            continue;
        }
        for (j, flag) in wall.feature_flags.iter().enumerate() {
            let pt = &wall.path.points[j];
            if flag.is_bridge {
                found_bridge_vertex = true;
                assert!(
                    (pt.flow_factor - 0.7).abs() < f32::EPSILON,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=true, expected flow_factor == 0.7 (bridge_flow via \
                     bridging_flow), got {}",
                    wall.loop_type,
                    wall.perimeter_index,
                    j,
                    pt.x,
                    pt.y,
                    pt.flow_factor
                );
            } else {
                assert!(
                    (pt.flow_factor - 1.0).abs() < f32::EPSILON,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=false, expected flow_factor == 1.0, got {}",
                    wall.loop_type,
                    wall.perimeter_index,
                    j,
                    pt.x,
                    pt.y,
                    pt.flow_factor
                );
            }
        }
    }

    assert!(
        found_bridge_vertex,
        "expected at least one is_bridge==true vertex to verify flow_factor against \
         (fixture must produce bridge vertices, or this test can never fail)"
    );
}

// ===========================================================================
// GAP_PIPELINE: 3 of 4 overhang config keys (propose D-104c)
// ====================================================================================

/// GAP_PIPELINE: 3 of 4 overhang config keys are not registered; the 4th
/// (`extra_perimeters_on_overhangs`) IS wired in classic (T-077).
///
/// OrcaSlicer ref: `PrintConfig.cpp:5003-5066`, `:1519-1534`.
/// Proposed deviation: `D-104c-OVERHANG-REVERSE-NONE`.
#[test]
fn arachne_parity_pipeline_overhang_reverse_config_keys() {
    let missing: Vec<&str> = [
        "detect_overhang_wall",
        "overhang_reverse",
        "overhang_reverse_internal_only",
    ]
    .iter()
    .copied()
    .filter(|k| !manifest_has_config_key(k))
    .collect();
    // The implemented one (extra_perimeters_on_overhangs) is wired in classic,
    // not in the arachne manifest. Assert it is present in the manifest as a
    // pipeline-level claim (fails today — classic consumes it but does not
    // re-publish the key in arachne's manifest).
    let extra_present = manifest_has_config_key("extra_perimeters_on_overhangs");
    assert!(
        missing.is_empty() && extra_present,
        "PARITY GAP: pipeline: overhang config keys | expected: OrcaSlicer \
         exposes detect_overhang_wall, extra_perimeters_on_overhangs, \
         overhang_reverse, overhang_reverse_internal_only (PrintConfig.cpp:\
         5003-5066, :1519-1534) | got: missing keys in arachne-perimeters.toml: \
         {:?}; extra_perimeters_on_overhangs present in arachne manifest: {} \
         (classic consumes it via resolved config) — propose \
         D-104c-OVERHANG-REVERSE-NONE for the three missing keys | ref: \
         PrintConfig.cpp:5003-5066",
        missing,
        extra_present
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-2): LoopType::ThinWall emitted
// ====================================================================================

/// `LoopType::ThinWall` is now emitted by the arachne module's
/// `classify_line` when `print_thin_walls` (`detect_thin_wall`) is on and a
/// widened `is_odd`/`inset_idx == 0` center-line bead is thinner than one
/// full bead, mirroring classic (`classic-perimeters/src/lib.rs:783-790`).
/// Rewritten (packet 148 AC-9) to drive `ArachnePerimeters::run_perimeters`
/// natively instead of substring-matching source text — mirrors
/// `arachne_parity_thin_wall_loop_type_tdd.rs`.
///
/// OrcaSlicer ref: `WallToolPaths.hpp:18`; `WideningBeadingStrategy.cpp:27-77`.
#[test]
fn arachne_parity_arachne_path_thin_wall_loop_type_emitted() {
    let config = native_thin_wall_config(true);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_thin_strip_region(0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        !walls.is_empty(),
        "the thin-strip fixture must emit at least one wall loop"
    );
    assert!(
        walls.iter().any(|w| w.loop_type == LoopType::ThinWall),
        "a 0.25mm-wide strip with detect_thin_wall=true must emit at least one \
         WallLoop with loop_type == LoopType::ThinWall; got loop_types: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-7/AC-8): precise_outer_wall
// registered and honored
// ====================================================================================

/// `precise_outer_wall` is now registered in `arachne-perimeters.toml` and
/// gated on `precise_outer_wall && wall_sequence == "InnerOuter"`, offsetting
/// the outer wall's toolpath by `-(preferred_bead_width_outer/2 -
/// optimal_width/2)`, mirroring classic
/// (`classic-perimeters/src/lib.rs:176-178, 545, 712`, T-053). Rewritten
/// (packet 148 AC-9) to drive `ArachnePerimeters::run_perimeters` natively
/// instead of substring-matching source text — mirrors
/// `precise_outer_wall_tdd.rs`.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:2146-2158`;
/// `OuterWallInsetBeadingStrategy.cpp:44-60`; `PrintConfig.cpp:1484-1489`.
#[test]
fn arachne_parity_arachne_path_precise_outer_wall_registered() {
    assert!(
        manifest_has_config_key("precise_outer_wall"),
        "expected arachne-perimeters.toml to declare [config.schema.precise_outer_wall]"
    );

    const OUTER_WIDTH_MM: f32 = 0.5;
    const SPACING_WIDTH_MM: f32 = 0.4;
    const LAYER_HEIGHT_MM: f64 = 0.2; // matches lib.rs's `unwrap_or(0.2)` default (no "layer_height" key set below)
                                      // Orca-parity precise-outer-wall inset: wall_0_inset =
                                      // -(ext_perimeter_width/2 - ext_perimeter_spacing/2). Because spacing =
                                      // width - layer_height*(1 - PI/4) (line_width_to_spacing), this reduces
                                      // to the WIDTH-INDEPENDENT expression below: -layer_height*(1-PI/4)/2 ==
                                      // -0.0214602 for LAYER_HEIGHT_MM = 0.2. Was `-0.05` (a raw-width-era
                                      // value, stale after the AC-3 spacing conversion); corrected in packet
                                      // 150 Step 4 alongside the lib.rs fix that pairs the outer wall's own
                                      // spacing with its own raw width in `outer_wall_offset`.
    const EXPECTED_OFFSET_MM: f64 = -(LAYER_HEIGHT_MM * (1.0 - std::f64::consts::PI / 4.0)) / 2.0;
    const TOLERANCE_MM: f32 = 1e-3;

    let make_config = |precise_outer_wall: bool| -> ConfigView {
        ConfigViewBuilder::new()
            .int("wall_count", 2)
            .float("inner_wall_line_width", SPACING_WIDTH_MM as f64)
            .float("outer_wall_line_width", OUTER_WIDTH_MM as f64)
            .bool("precise_outer_wall", precise_outer_wall)
            .string("wall_sequence", "InnerOuter")
            .build()
    };
    let run_and_get_outer_wall = |config: &ConfigView| -> slicer_ir::WallLoop {
        let module = ArachnePerimeters::on_print_start(config).unwrap();
        let regions = vec![native_square_region(10.0, 0.2)];
        let paint = PaintRegionLayerView::new(0);
        let mut output = PerimeterOutputBuilder::new();
        module
            .run_perimeters(0, &regions, &paint, &mut output, config)
            .unwrap();
        output
            .wall_loops()
            .iter()
            .find(|w| w.perimeter_index == 0)
            .expect("a wall loop with perimeter_index == 0 must be emitted")
            .clone()
    };
    let min_x = |wall: &slicer_ir::WallLoop| -> f32 {
        wall.path
            .points
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min)
    };

    let outer_off = run_and_get_outer_wall(&make_config(false));
    let outer_on = run_and_get_outer_wall(&make_config(true));
    let observed_delta = (min_x(&outer_on) - min_x(&outer_off)) as f64;

    assert!(
        (observed_delta - EXPECTED_OFFSET_MM).abs() < TOLERANCE_MM as f64,
        "expected outer wall min-x to shift by {EXPECTED_OFFSET_MM} mm when \
         precise_outer_wall is gated on, observed shift {observed_delta} mm \
         (off min-x={}, on min-x={})",
        min_x(&outer_off),
        min_x(&outer_on)
    );
}

// ===========================================================================
// GAP_PIPELINE: only_one_wall_top (classic) vs min_width_top_surface (missing)
// ====================================================================================

/// GAP_PIPELINE: `only_one_wall_top` IS implemented in classic
/// (`classic-perimeters/src/lib.rs:222, 268`); `min_width_top_surface` is the
/// threshold for the OrcaSlicer single-wall cutoff and has zero readers
/// anywhere in the pipeline.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:2160-2245`; `PrintConfig.cpp:1491-1511`.
/// Proposed deviation: `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`.
#[test]
fn arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface() {
    let classic_reads_top = CLASSIC_MODULE_SRC.contains("only_one_wall_top");
    let classic_reads_min_width = CLASSIC_MODULE_SRC.contains("min_width_top_surface");
    let arachne_reads_top = ARACHNE_MODULE_SRC.contains("only_one_wall_top");
    assert!(
        classic_reads_top && classic_reads_min_width && arachne_reads_top,
        "PARITY GAP: pipeline: min_width_top_surface registered | expected: \
         OrcaSlicer exposes only_one_wall_top + min_width_top_surface which \
         re-run Arachne for remaining inner loops to keep a single wall on \
         top surfaces (PerimeterGenerator.cpp:2160-2245; PrintConfig.cpp:1491-\
         1511) | classic path DOES read only_one_wall_top (lib.rs:222, 268) | \
         got: min_width_top_surface has ZERO readers anywhere in the pipeline \
         (classic reads min_width: {}, arachne reads only_one_wall_top: {}) — \
         propose D-104d-MIN-WIDTH-TOP-SURFACE-NONE | ref: \
         PerimeterGenerator.cpp:2160-2245",
        classic_reads_min_width,
        arachne_reads_top
    );
}

// ===========================================================================
// GAP_PIPELINE: alternate_extra_wall (propose D-104e)
// ====================================================================================

/// GAP_PIPELINE: `alternate_extra_wall` is not registered and not
/// implemented anywhere in the pipeline.
///
/// OrcaSlicer ref: `PrintConfig.cpp:5059-5066`.
/// Proposed deviation: `D-104e-ALTERNATE-EXTRA-WALL-NONE`.
#[test]
fn arachne_parity_pipeline_alternate_extra_wall_not_registered() {
    let present_in_either = manifest_has_config_key("alternate_extra_wall")
        || CLASSIC_MODULE_SRC.contains("alternate_extra_wall");
    assert!(
        present_in_either,
        "PARITY GAP: pipeline: alternate_extra_wall config | expected: \
         OrcaSlicer exposes alternate_extra_wall to add an extra wall on \
         alternating layers (PrintConfig.cpp:5059-5066) | got: no \
         alternate_extra_wall key in arachne-perimeters.toml and no \
         implementation in classic-perimeters either — propose \
         D-104e-ALTERNATE-EXTRA-WALL-NONE | ref: PrintConfig.cpp:5059-5066"
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-1): WallBoundaryType::ExteriorSurface
// for outer wall
// ====================================================================================

/// `WallBoundaryType::ExteriorSurface` is now set for the outermost wall
/// (`perimeter_index == 0`) in arachne, mirroring classic
/// (`crates/slicer-core/src/perimeter_utils.rs:194-195`). Rewritten (packet
/// 148 AC-9) to drive `ArachnePerimeters::run_perimeters` natively instead of
/// constructing a local `WallBoundaryType::Interior` and asserting it isn't
/// `Interior` (structurally un-passable) — mirrors
/// `arachne_parity_outer_wall_boundary_type_tdd.rs`.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:383`; `docs/02_ir_schemas.md:1418-1428`.
#[test]
fn arachne_parity_arachne_path_outer_wall_boundary_type_exterior_surface() {
    let config = native_wall_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_square_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let outer_wall = output
        .wall_loops()
        .iter()
        .find(|w| w.perimeter_index == 0)
        .expect("a wall loop with perimeter_index == 0 must be emitted");

    assert_eq!(
        outer_wall.boundary_type,
        WallBoundaryType::ExteriorSurface,
        "the perimeter_index == 0 wall loop (outermost bead, facing air) must have \
         boundary_type == ExteriorSurface, got {:?}",
        outer_wall.boundary_type
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-4): is_bridge flag set per vertex
// ====================================================================================

/// `WallFeatureFlags.is_bridge` is now set per-vertex in arachne via
/// `region.bridge_areas()` point-in-polygon lookup on Outer/Inner walls,
/// mirroring classic (`classic-perimeters/src/lib.rs:675-678`).
/// ThinWall/GapFill walls never set `is_bridge`. Rewritten (packet 148 AC-9)
/// to drive `ArachnePerimeters::run_perimeters` natively instead of grepping
/// source text for `is_bridge` + `true` on one line (structurally
/// un-passable) — mirrors `arachne_parity_is_bridge_flag_tdd.rs`.
///
/// OrcaSlicer ref: `docs/02_ir_schemas.md:1520-1533`;
/// `PerimeterGenerator.cpp:2113-2119`.
#[test]
fn arachne_parity_arachne_path_is_bridge_flag_set_per_vertex() {
    let config = native_wall_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_bridge_region(10.0, 4.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    // Matches the bridge area used by native_bridge_region (see helper's doc comment).
    let bridge_areas = vec![square_polygon(0.0, 0.0, 12.0)];
    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let mut checked_any_outer_inner = false;
    for wall in output.wall_loops() {
        for (j, flag) in wall.feature_flags.iter().enumerate() {
            let pt = &wall.path.points[j];
            let units_pt = Point2 {
                x: mm_to_units(pt.x),
                y: mm_to_units(pt.y),
            };
            let inside = point_in_any_polygon(&units_pt, &bridge_areas);

            match wall.loop_type {
                LoopType::Outer | LoopType::Inner => {
                    checked_any_outer_inner = true;
                    assert_eq!(
                        flag.is_bridge,
                        inside,
                        "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                         expected is_bridge == {} (point-in-bridge-area == {}), got {}",
                        wall.loop_type,
                        wall.perimeter_index,
                        j,
                        pt.x,
                        pt.y,
                        inside,
                        inside,
                        flag.is_bridge
                    );
                }
                _ => {
                    assert!(
                        !flag.is_bridge,
                        "ThinWall/GapFill/NonPlanarShell walls must never set is_bridge \
                         (loop_type={:?}, vertex {})",
                        wall.loop_type, j
                    );
                }
            }
        }
    }

    assert!(
        checked_any_outer_inner,
        "expected at least one Outer or Inner wall loop to verify is_bridge against"
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-3): is_thin_wall flag set on thin
// region
// ====================================================================================

/// `WallFeatureFlags.is_thin_wall` is now set on every vertex of a
/// `LoopType::ThinWall` wall, and never on `Outer`/`Inner` walls, mirroring
/// classic (`classic-perimeters/src/lib.rs:772` in ThinWall emission).
/// Rewritten (packet 148 AC-9) to drive `ArachnePerimeters::run_perimeters`
/// natively instead of substring-matching source text — mirrors
/// `arachne_parity_is_thin_wall_flag_tdd.rs`.
///
/// OrcaSlicer ref: `docs/02_ir_schemas.md:1528`; `WideningBeadingStrategy.cpp:27-77`.
#[test]
fn arachne_parity_arachne_path_is_thin_wall_flag_set_on_thin_wall_loops() {
    let config = native_thin_wall_config(true);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_thin_strip_region(0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(
        !walls.is_empty(),
        "the thin-strip fixture must emit at least one wall loop"
    );

    let thin_wall_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .collect();
    assert!(
        !thin_wall_loops.is_empty(),
        "the thin-strip fixture must emit at least one LoopType::ThinWall wall; \
         got loop_types: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );
    for wall in &thin_wall_loops {
        assert!(
            wall.feature_flags.iter().all(|f| f.is_thin_wall),
            "every vertex's WallFeatureFlags on a LoopType::ThinWall wall must have \
             is_thin_wall == true; got {:?}",
            wall.feature_flags
                .iter()
                .map(|f| f.is_thin_wall)
                .collect::<Vec<_>>()
        );
    }

    // Negative shape: is_thin_wall must never be set on Outer/Inner walls,
    // even if geometrically narrow.
    for wall in walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
    {
        assert!(
            wall.feature_flags.iter().all(|f| !f.is_thin_wall),
            "Outer/Inner walls must never have is_thin_wall == true (loop_type {:?}); \
             got {:?}",
            wall.loop_type,
            wall.feature_flags
                .iter()
                .map(|f| f.is_thin_wall)
                .collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// GAP_ARACHNE_PATH (CLOSED, packet 148 AC-6): seam_candidate producer present
// ====================================================================================

/// Arachne now emits seam candidates for each region's outer contour via
/// `generate_sharp_corner_seam_candidates`, mirroring classic
/// (`classic-perimeters/src/lib.rs:889-900`). Rewritten (packet 148 AC-9) to
/// drive `ArachnePerimeters::run_perimeters` natively instead of
/// substring-matching source text — mirrors
/// `arachne_parity_seam_candidate_tdd.rs`.
///
/// OrcaSlicer ref: `docs/05_module_sdk.md:601-657`;
/// `PerimeterGenerator.cpp:2093-2535`.
/// Note: `seam-placer` is a CONSUMER (`select_seam_candidate` reads
/// `region.seam_candidates()`); it does not generate them — the perimeter
/// module is the producer.
#[test]
fn arachne_parity_arachne_path_seam_candidate_producer_present() {
    let config = native_wall_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![native_square_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let candidates = output.seam_candidates();
    assert!(
        !candidates.is_empty(),
        "expected at least one seam candidate for a square's sharp corners"
    );

    // square_polygon(0.0, 0.0, 10.0) corners, in mm.
    let corners_mm: Vec<(f32, f32)> = square_polygon(0.0, 0.0, 10.0)
        .contour
        .points
        .iter()
        .map(|p| (units_to_mm(p.x), units_to_mm(p.y)))
        .collect();

    for (pos, _score) in candidates {
        let is_at_corner = corners_mm
            .iter()
            .any(|(cx, cy)| (pos.x - cx).abs() < 1e-3 && (pos.y - cy).abs() < 1e-3);
        assert!(
            is_at_corner,
            "seam candidate at ({}, {}) mm does not match any input polygon corner {:?}",
            pos.x, pos.y, corners_mm
        );
    }
}

// ===========================================================================
// GAP_PIPELINE: concentric infill uses Arachne (propose D-104f)
// ====================================================================================

/// GAP_PIPELINE: Arachne wall generation is not wired into the concentric
/// infill path. OrcaSlicer routes both concentric and concentric-internal
/// infill walls through `WallToolPaths` when `use_arachne` is set.
///
/// OrcaSlicer ref: `FillConcentric.cpp:80-118`; `FillConcentricInternal.cpp:29-55`.
/// Proposed deviation: `D-104f-CONCENTRIC-INFILL-NO-ARACHNE`.
#[test]
fn arachne_parity_pipeline_concentric_infill_uses_arachne() {
    let runtime_src = include_str!("../src/run.rs");
    let uses_arachne_for_infill =
        runtime_src.contains("run_arachne_pipeline") && runtime_src.contains("concentric");
    let rectilinear = include_str!("../../../modules/core-modules/rectilinear-infill/src/lib.rs");
    let gyroid = include_str!("../../../modules/core-modules/gyroid-infill/src/lib.rs");
    let lightning = include_str!("../../../modules/core-modules/lightning-infill/src/lib.rs");
    let any_infill_uses_arachne = rectilinear.contains("run_arachne_pipeline")
        || rectilinear.contains("generate_arachne_walls")
        || gyroid.contains("run_arachne_pipeline")
        || gyroid.contains("generate_arachne_walls")
        || lightning.contains("run_arachne_pipeline")
        || lightning.contains("generate_arachne_walls");
    assert!(
        uses_arachne_for_infill || any_infill_uses_arachne,
        "PARITY GAP: pipeline: concentric infill uses Arachne | expected: \
         OrcaSlicer routes concentric infill wall generation through Arachne's \
         WallToolPaths when use_arachne is set (FillConcentric.cpp:80-118; \
         FillConcentricInternal.cpp:29-55) | got: slicer-runtime's run.rs never \
         dispatches infill through run_arachne_pipeline, and none of the three \
         infill modules (rectilinear/gyroid/lightning) reference \
         run_arachne_pipeline or generate_arachne_walls — propose \
         D-104f-CONCENTRIC-INFILL-NO-ARACHNE | ref: FillConcentric.cpp:80-118"
    );
}
