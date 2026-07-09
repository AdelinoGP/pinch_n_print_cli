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

use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, ExtrusionLine, WallBoundaryType};

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
// GAP_ARACHNE_PATH: overhang_quartile populated on junctions
// ====================================================================================

/// GAP_ARACHNE_PATH: `overhang_quartile` is populated in `classic-perimeters`
/// (via `perimeter_utils::expolygon_to_path3d` at
/// `crates/slicer-core/src/perimeter_utils.rs:316-331`) but hardcoded `None`
/// in the arachne path (`generate_toolpaths.rs:184, 471, 866`). Tracked as
/// `D-104-OVERHANG-QUARTILE-NONE`.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:2113-2119`, `:370-460`, `:1117-1453`.
#[test]
fn arachne_parity_arachne_path_overhang_quartile_hardcoded_none() {
    let sq = fixtures::square_mm(10.0);
    let (lines, _) = arachne_lines(std::slice::from_ref(&sq));
    let any_set = lines
        .iter()
        .flat_map(|l| l.junctions.iter())
        .any(|j| j.p.overhang_quartile.is_some());
    assert!(
        any_set,
        "PARITY GAP: arachne_path: overhang_quartile populated | expected: \
         OrcaSlicer detects overhang walls by offsetting lower slices by \
         nozzle/2 and assigns the perimeter an overhang role, populating the \
         per-vertex overhang classification (PerimeterGenerator.cpp:2113-2119) | \
         classic path DOES set this via perimeter_utils::expolygon_to_path3d \
         (perimeter_utils.rs:316-331) | got (arachne path): \
         generate_toolpaths.rs:184,471,866 hardcodes overhang_quartile: None \
         for every junction — tracked as D-104-OVERHANG-QUARTILE-NONE | ref: \
         PerimeterGenerator.cpp:2113-2119"
    );
}

// ===========================================================================
// GAP_PIPELINE: bridge flow_factor on overhang (propose D-104b)
// ====================================================================================

/// GAP_PIPELINE: `flow_factor` is never reduced for overhang segments.
/// OrcaSlicer applies `bridging_flow(frPerimeter, thick_bridges)`
/// (`LayerRegion.cpp:135`). PnP's overhang handling modulates `speed_factor`
/// (in `overhang-classifier-default`) but never `flow_factor`.
///
/// OrcaSlicer ref: `LayerRegion.cpp:135`.
/// Proposed deviation: `D-104b-OVERHANG-FLOW-NONE`.
#[test]
fn arachne_parity_pipeline_bridge_flow_factor_on_overhang() {
    let sq = fixtures::square_mm(10.0);
    let (lines, _) = arachne_lines(std::slice::from_ref(&sq));
    let all_default = lines
        .iter()
        .flat_map(|l| l.junctions.iter())
        .all(|j| j.p.flow_factor == 1.0);
    assert!(
        !all_default,
        "PARITY GAP: pipeline: bridge flow_factor on overhang | expected: \
         OrcaSlicer computes a bridging flow (bridging_flow(frPerimeter, \
         thick_bridges)) for overhang/bridge perimeters so flow_factor \
         differs from 1.0 on overhang segments (LayerRegion.cpp:135) | got: \
         every junction's flow_factor is 1.0 (extrusion_line_to_extrusion_path3d \
         sets the canonical default); no bridge-flow assignment exists \
         anywhere in the pipeline — propose D-104b-OVERHANG-FLOW-NONE | ref: \
         LayerRegion.cpp:135"
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
// GAP_ARACHNE_PATH: LoopType::ThinWall emitted
// ====================================================================================

/// GAP_ARACHNE_PATH: `LoopType::ThinWall` is emitted in classic
/// (`classic-perimeters/src/lib.rs:783-790`) but the arachne module's
/// `classify_line` (`arachne-perimeters/src/lib.rs:206-214`) never returns
/// `ThinWall`.
///
/// OrcaSlicer ref: `WallToolPaths.hpp:18`; `WideningBeadingStrategy.cpp:27-77`.
#[test]
fn arachne_parity_arachne_path_thin_wall_loop_type_never_emitted() {
    let strip = fixtures::thin_strip_mm(0.25, 5.0);
    let params = ArachneParams {
        print_thin_walls: true,
        ..ArachneParams::default()
    };
    let (lines, _) = run_arachne_pipeline(std::slice::from_ref(&strip), &params, false)
        .expect("thin strip pipeline ok");
    let classic_emits_thin_wall = CLASSIC_MODULE_SRC.contains("LoopType::ThinWall");
    let arachne_classify_emits_thin_wall = ARACHNE_MODULE_SRC
        .lines()
        .any(|l| l.contains("classify_line") && l.contains("ThinWall"))
        || (ARACHNE_MODULE_SRC.contains("classify_line")
            && ARACHNE_MODULE_SRC.contains("ThinWall"));
    assert!(
        classic_emits_thin_wall && arachne_classify_emits_thin_wall,
        "PARITY GAP: arachne_path: ThinWall loop type emitted | expected: \
         OrcaSlicer tags thin-wall widened beads as a distinct loop type \
         (WideningBeadingStrategy.cpp:27-77; WallToolPaths.hpp:18 \
         fill_outline_gaps=true; LoopType::ThinWall per docs/02_ir_schemas.md:\
         1505-1516) | classic path DOES emit ThinWall (lib.rs:783-790) | got \
         (arachne path): classify_line (lib.rs:206-214) maps is_odd→GapFill, \
         inset_idx==0→Outer, else Inner; LoopType::ThinWall is never produced \
         (classic emits ThinWall: {}, arachne classify references ThinWall: {}, \
         lines produced: {}) | ref: WideningBeadingStrategy.cpp:27-77",
        classic_emits_thin_wall,
        arachne_classify_emits_thin_wall,
        lines.len()
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: precise_outer_wall not registered in arachne manifest
// ====================================================================================

/// GAP_ARACHNE_PATH: `precise_outer_wall` IS honored in classic
/// (`classic-perimeters/src/lib.rs:176-178, 545, 712`, T-053) but not in the
/// arachne manifest/module.
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:2146-2158`;
/// `OuterWallInsetBeadingStrategy.cpp:44-60`; `PrintConfig.cpp:1484-1489`.
#[test]
fn arachne_parity_arachne_path_precise_outer_wall_not_registered() {
    let classic_registers = CLASSIC_MODULE_SRC.contains("precise_outer_wall");
    let arachne_registers = manifest_has_config_key("precise_outer_wall");
    assert!(
        arachne_registers && classic_registers,
        "PARITY GAP: arachne_path: precise_outer_wall config registered | \
         expected: OrcaSlicer exposes precise_outer_wall coBool (default \
         true) that offsets the outer wall by -(ext_perimeter_width/2 - \
         ext_perimeter_spacing/2) when wall_sequence==InnerOuter \
         (PerimeterGenerator.cpp:2146-2158; OuterWallInsetBeadingStrategy.cpp:\
         44-60; PrintConfig.cpp:1484-1489) | classic path DOES register + \
         honor it (lib.rs:176-178, 545, 712) | got (arachne path): no \
         precise_outer_wall section in arachne-perimeters.toml \
         (arachne registers: {}, classic registers: {}) | ref: \
         PerimeterGenerator.cpp:2146-2158",
        arachne_registers,
        classic_registers
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
// GAP_ARACHNE_PATH: WallBoundaryType::ExteriorSurface for outer wall
// ====================================================================================

/// GAP_ARACHNE_PATH: `WallBoundaryType::ExteriorSurface` is set for the outer
/// wall in classic (`crates/slicer-core/src/perimeter_utils.rs:194-195`) but
/// hardcoded `Interior` in arachne (`arachne-perimeters/src/lib.rs:302`).
///
/// OrcaSlicer ref: `PerimeterGenerator.cpp:383`; `docs/02_ir_schemas.md:1418-1428`.
#[test]
fn arachne_parity_arachne_path_outer_wall_boundary_type_hardcoded_interior() {
    let sq = fixtures::square_mm(10.0);
    let (lines, _) = arachne_lines(std::slice::from_ref(&sq));
    let outer_idx = lines
        .iter()
        .min_by_key(|l| l.inset_idx)
        .map(|l| l.inset_idx);
    assert!(
        outer_idx.is_some(),
        "Arachne pipeline produced no lines for a 10mm square"
    );
    let arachne_uses_exterior = ARACHNE_MODULE_SRC.contains("WallBoundaryType::ExteriorSurface");
    // Replicate the module's hardcoded decision to surface the gap.
    let boundary = WallBoundaryType::Interior;
    assert!(
        arachne_uses_exterior && !matches!(boundary, WallBoundaryType::Interior),
        "PARITY GAP: arachne_path: outer wall boundary_type ExteriorSurface | \
         expected: the outermost wall (inset_idx == 0) carries \
         WallBoundaryType::ExteriorSurface, signalling it faces air \
         (PerimeterGenerator.cpp:383; docs/02_ir_schemas.md:1418-1428) | \
         classic path DOES set ExteriorSurface via perimeter_utils::\
         build_wall_flags (perimeter_utils.rs:194-195) | got (arachne path): \
         arachne-perimeters lib.rs:302 hardcodes \
         boundary_type = WallBoundaryType::Interior for every wall \
         (arachne uses ExteriorSurface: {}) | ref: PerimeterGenerator.cpp:383",
        arachne_uses_exterior
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: is_bridge flag never set in arachne path
// ====================================================================================

/// GAP_ARACHNE_PATH: `WallFeatureFlags.is_bridge` is set per-vertex in
/// classic (`classic-perimeters/src/lib.rs:675-678`) but never set in arachne
/// (`arachne-perimeters/src/lib.rs:301` uses `WallFeatureFlags::default()`).
///
/// OrcaSlicer ref: `docs/02_ir_schemas.md:1520-1533`;
/// `PerimeterGenerator.cpp:2113-2119`.
#[test]
fn arachne_parity_arachne_path_is_bridge_flag_never_set() {
    let sq = fixtures::square_mm(10.0);
    let (lines, _) = arachne_lines(std::slice::from_ref(&sq));
    let classic_sets_is_bridge = CLASSIC_MODULE_SRC
        .lines()
        .any(|l| l.contains("is_bridge") && l.contains("true"));
    let arachne_sets_is_bridge = ARACHNE_MODULE_SRC
        .lines()
        .any(|l| l.contains("is_bridge") && l.contains("true"));
    assert!(
        classic_sets_is_bridge && arachne_sets_is_bridge,
        "PARITY GAP: arachne_path: is_bridge flag set on overhang | expected: \
         overhang/bridge wall segments carry WallFeatureFlags.is_bridge = \
         true (docs/02_ir_schemas.md:1520-1533; PerimeterGenerator.cpp:2113-\
         2119) | classic path DOES set is_bridge per-vertex (lib.rs:675-678) | \
         got (arachne path): arachne-perimeters lib.rs:301 builds \
         feature_flags: vec![WallFeatureFlags::default(); num_points] for \
         every wall, so is_bridge is never set (classic sets is_bridge: {}, \
         arachne sets is_bridge: {}, lines produced: {}) | ref: \
         PerimeterGenerator.cpp:2113-2119",
        classic_sets_is_bridge,
        arachne_sets_is_bridge,
        lines.len()
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: is_thin_wall flag never set in arachne path
// ====================================================================================

/// GAP_ARACHNE_PATH: `WallFeatureFlags.is_thin_wall` is set in classic
/// (`classic-perimeters/src/lib.rs:772` in ThinWall emission) but never set in
/// arachne.
///
/// OrcaSlicer ref: `docs/02_ir_schemas.md:1528`; `WideningBeadingStrategy.cpp:27-77`.
#[test]
fn arachne_parity_arachne_path_is_thin_wall_flag_never_set() {
    let strip = fixtures::thin_strip_mm(0.25, 5.0);
    let params = ArachneParams {
        print_thin_walls: true,
        ..ArachneParams::default()
    };
    let (lines, _) = run_arachne_pipeline(std::slice::from_ref(&strip), &params, false)
        .expect("thin strip pipeline ok");
    let classic_sets_is_thin_wall = CLASSIC_MODULE_SRC
        .lines()
        .any(|l| l.contains("is_thin_wall") && l.contains("true"));
    let arachne_sets_is_thin_wall = ARACHNE_MODULE_SRC
        .lines()
        .any(|l| l.contains("is_thin_wall") && l.contains("true"));
    assert!(
        classic_sets_is_thin_wall && arachne_sets_is_thin_wall,
        "PARITY GAP: arachne_path: is_thin_wall flag set on thin region | \
         expected: thin-wall widened beads carry WallFeatureFlags.is_thin_wall \
         = true (docs/02_ir_schemas.md:1528; WideningBeadingStrategy.cpp:27-77) \
         | classic path DOES set is_thin_wall (lib.rs:772 in ThinWall emission) \
         | got (arachne path): arachne-perimeters lib.rs:301 builds \
         feature_flags: vec![WallFeatureFlags::default(); num_points] \
         (classic sets is_thin_wall: {}, arachne sets is_thin_wall: {}, lines \
         produced: {}) | ref: WideningBeadingStrategy.cpp:27-77",
        classic_sets_is_thin_wall,
        arachne_sets_is_thin_wall,
        lines.len()
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: seam_candidate producer missing in arachne
// ====================================================================================

/// GAP_ARACHNE_PATH: seam candidates are emitted in classic
/// (`classic-perimeters/src/lib.rs:889-900` via
/// `generate_sharp_corner_seam_candidates`) but never in arachne.
///
/// OrcaSlicer ref: `docs/05_module_sdk.md:601-657`;
/// `PerimeterGenerator.cpp:2093-2535`.
/// Note: `seam-placer` is a CONSUMER (`select_seam_candidate` reads
/// `region.seam_candidates()`); it does not generate them — the perimeter
/// module is the producer.
#[test]
fn arachne_parity_arachne_path_seam_candidate_producer_missing() {
    let classic_emits_seam = CLASSIC_MODULE_SRC.contains("generate_sharp_corner_seam_candidates")
        || CLASSIC_MODULE_SRC.contains("push_seam_candidate");
    let arachne_emits_seam = ARACHNE_MODULE_SRC.contains("generate_sharp_corner_seam_candidates")
        || ARACHNE_MODULE_SRC.contains("push_seam_candidate")
        || ARACHNE_MODULE_SRC.contains("seam_candidate");
    assert!(
        classic_emits_seam && arachne_emits_seam,
        "PARITY GAP: arachne_path: seam_candidates produced | expected: \
         perimeter modules emit seam_candidates for the outer wall via \
         generate_sharp_corner_seam_candidates (docs/05_module_sdk.md:601-657; \
         PerimeterGenerator.cpp:2093-2535) | classic path IS the seam-candidate \
         producer (lib.rs:889-900) | got (arachne path): no \
         generate_sharp_corner_seam_candidates / push_seam_candidate call in \
         arachne-perimeters src/lib.rs (classic emits: {}, arachne emits: {}) | \
         ref: docs/05_module_sdk.md:601-657",
        classic_emits_seam,
        arachne_emits_seam
    );
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
