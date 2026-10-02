//! Packet 184 / D-164: canonical wall-width resolution in `classic-perimeters`.
//!
//! OrcaSlicer declares `outer_wall_line_width` / `inner_wall_line_width` as
//! `coFloatOrPercent` with `ratio_over = "nozzle_diameter"` and an upstream
//! default of `0` (see canonical `PrintConfigDef::init_fff_params` in
//! `PrintConfig.cpp`). A non-percent value `<= 0` is the *auto* sentinel:
//! canonical `Flow::new_from_config_width` routes it to
//! `Flow::auto_extrusion_width`, which returns `1.125 * nozzle_diameter` for
//! both `frExternalPerimeter` and `frPerimeter`.
//!
//! Test 1 locks the auto sentinel. Test 2 locks the packet-185 default move:
//! with the keys absent entirely, the canonical auto-0 default now applies
//! (`1.125 * nozzle_diameter`) — packet 185's AC-5 superseded packet 184's
//! "keep the legacy 0.4 mm fallback for the absent-key case" scope decision,
//! so absent keys behave identically to the explicit auto-0 sentinel in Test 1.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::ConfigValue;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use std::collections::HashMap;
use std::path::PathBuf;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

fn find_max_x(points: &[slicer_ir::Point3WithWidth]) -> f32 {
    points.iter().map(|p| p.x).fold(f32::MIN, f32::max)
}

#[test]
fn zero_width_resolves_to_canonical_auto_extrusion_width() {
    let nozzle_diameter = 0.6_f32;
    let expected = 1.125_f32 * nozzle_diameter; // 0.675 mm

    // Bound-view baseline (packet 06 5c-prime): contract-required `require_*`
    // reads need the full classic surface. `line_width` holds the host-expanded
    // auto width (packet 04: the host expands the auto-0 sentinel to
    // `1.125 * nozzle_diameter` before the guest sees the view), which is the
    // base `resolve_role_width` falls back to when the role width is the 0
    // sentinel — the value this test asserts.
    let config = crate::common::classic_perimeters_baseline()
        .float("line_width", expected as f64)
        .int("wall_count", 3)
        .float("nozzle_diameter", nozzle_diameter as f64)
        .float("outer_wall_line_width", 0.0)
        .float("inner_wall_line_width", 0.0)
        .float("layer_height", 0.2)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(
        !outer.is_empty(),
        "Expected at least one outer (perimeter_index == 0) wall loop, got {} loops total",
        walls.len()
    );

    for wall in &outer {
        assert!(
            !wall.path.points.is_empty(),
            "Outer wall loop has no vertices"
        );
        for pt in &wall.path.points {
            assert!(
                (pt.width - expected).abs() < 0.005,
                "Outer wall vertex width {} != canonical auto_extrusion_width {} \
                 (1.125 * nozzle_diameter {})",
                pt.width,
                expected,
                nozzle_diameter
            );
        }
    }

    // Sanity: the outer centerline must be inset by half the resolved width.
    let outer_x = find_max_x(&outer[0].path.points);
    assert!(
        outer_x < 5.0,
        "Outer wall right edge X {outer_x} should be inset from the 5 mm contour"
    );
}

#[test]
fn absent_width_keys_resolve_to_canonical_auto_width() {
    // No outer_wall_line_width, no inner_wall_line_width, no line_width in the
    // AUTHORED source: an empty flat config ingested against the live registry,
    // resolved with the production scope stack (registry-default seeding +
    // `expand_automatic_values`), and bound through `bind_module_config_view`
    // exactly as `run_slice` does. The width keys reach the view only via
    // registry-default seeding; `line_width` arrives as the host-expanded auto
    // width (packet 04: `expand_automatic_values` turns the auto-0 sentinel
    // into `1.125 * nozzle_diameter`). The expected width is derived from the
    // nozzle alone — nothing authored can carry the expected width into the
    // view, so a resolution defect (a literal 0.4 fallback, an unexpanded
    // sentinel) fails here rather than passing as a passthrough.
    let modules: Vec<slicer_runtime::LoadedModule> =
        slicer_runtime::load_modules_from_roots(std::slice::from_ref(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("modules")
                .join("core-modules"),
        ))
        .unwrap_or_else(|error| panic!("load core module schemas failed: {error:?}"))
        .modules;
    let classic = modules
        .iter()
        .find(|module| module.id() == "com.core.classic-perimeters")
        .expect("classic-perimeters must be among the live core modules");
    let declarations: Vec<slicer_config::ModuleDeclaration> = modules
        .iter()
        .map(|module| slicer_config::ModuleDeclaration {
            module_id: module.id().to_owned(),
            schema: module.config_schema().clone(),
            claim_exclusive_group: None,
        })
        .collect();
    let registry =
        slicer_config::assemble_registry(&declarations, &slicer_config::HostChannels::from_live())
            .unwrap_or_else(|error| panic!("assemble live registry failed: {error}"))
            .registry;

    // Genuinely absent WIDTH keys: only the nozzle is authored, so
    // `line_width`/`outer_wall_line_width`/`inner_wall_line_width` are absent
    // from the source and reach the view purely through registry-default
    // seeding. Authoring the nozzle keeps the oracle independent of the
    // manifest's 0.4 default (which `required_base` prefers over the expansion
    // context) while still proving the absent widths resolve — a passthrough
    // or literal-0.4 defect cannot produce 1.125 * 0.6.
    let mut authored = HashMap::new();
    authored.insert("nozzle_diameter".to_string(), ConfigValue::Float(0.6));
    let mut ingestor = slicer_config::ConfigIngestor::tolerant(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("authored nozzle must decode");
    let ingested = ingestor.finish();
    assert!(
        ingested.warnings.is_empty(),
        "the authored nozzle must ingest clean: {:?}",
        ingested.warnings
    );
    let resolved = slicer_config::resolve_scope_stack(
        &registry,
        &ingested.scoped,
        &slicer_config::ResolutionTarget::default(),
        &slicer_config::ExpansionContext {
            nozzle_diameter_mm: 0.6,
            ..slicer_config::ExpansionContext::default()
        },
    )
    .expect("resolving the authored nozzle against the live registry");

    // Fixture premise: the width keys are absent from the authored source, so
    // their values can only come from seeding + expansion.
    for key in [
        "line_width",
        "outer_wall_line_width",
        "inner_wall_line_width",
    ] {
        assert!(
            !authored.contains_key(key),
            "premise: {key} must not be authored"
        );
    }

    let nozzle_diameter = 0.6_f32;
    let expected = 1.125_f32 * nozzle_diameter; // 0.675 mm
    let config = slicer_runtime::bind_module_config_view(classic, &resolved);

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "Expected at least one wall loop");

    let mut saw_outer = false;
    let mut saw_inner = false;
    for wall in walls {
        if wall.perimeter_index == 0 {
            saw_outer = true;
        } else {
            saw_inner = true;
        }
        for pt in &wall.path.points {
            assert!(
                (pt.width - expected).abs() < 0.005,
                "Wall (perimeter_index {}) vertex width {} != canonical auto \
                 width {} (1.125 * nozzle_diameter {})",
                wall.perimeter_index,
                pt.width,
                expected,
                nozzle_diameter
            );
        }
    }
    assert!(saw_outer, "Expected an outer (perimeter_index == 0) wall");
    assert!(saw_inner, "Expected at least one inner wall");
}
