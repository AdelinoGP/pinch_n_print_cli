// only_one_wall_first_layer_tdd.rs — AC-5 TDD tests.
//
// AC-5: When layer_index == 0 and only_one_wall_first_layer = true, run_perimeters
// must clamp wall count to 1 regardless of the configured wall_loops.
// At layer_index > 0 the configured wall_loops (4) must be respected.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a ConfigView with wall_loops=4, line_width=0.4, only_one_wall_first_layer=<flag>.
fn config_4_walls(only_one_wall_first_layer: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_loops", 4)
        .float("line_width", 0.4)
        .bool("only_one_wall_first_layer", only_one_wall_first_layer)
        .build()
}

/// Build a 10×10 mm square polygon (100_000 units per side, 1 unit = 100 nm).
fn outer_square() -> ExPolygon {
    let size = 100_000_i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: size, y: 0 },
                Point2 { x: size, y: size },
                Point2 { x: 0, y: size },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a plain region (no top/bottom shell overrides needed for first-layer tests).
fn make_region() -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![outer_square()]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_bridge_areas(vec![]);
    region
}

/// AC-5: layer_index == 0, only_one_wall_first_layer = true → exactly 1 wall.
#[test]
fn first_layer_clamped_to_one_wall() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region();

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        1,
        "AC-5: layer_index=0 with only_one_wall_first_layer=true must emit 1 wall; got {}",
        walls.len()
    );
}

/// AC-5 negative: layer_index == 5, only_one_wall_first_layer = true → 4 walls.
#[test]
fn non_first_layer_respects_wall_count() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        4,
        "AC-5 negative: layer_index=5 must not be clamped; expected 4 walls, got {}",
        walls.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// DEV-124 — the clamp follows the raft, not layer zero
// ═══════════════════════════════════════════════════════════════════════════

/// Same as `config_4_walls` but with a raft configured.
///
/// PnP's `support_raft_layers` is its name for canonical `raft_layers` (same
/// semantics, same default 0). Canonical gates the single-wall clamp on
/// `layer_id == object_config->raft_layers` in `process_classic` and, via
/// `is_bottom_layer`, in `process_arachne` — i.e. the first *printed* layer.
fn config_4_walls_with_raft(raft_layers: i64) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_loops", 4)
        .float("line_width", 0.4)
        .bool("only_one_wall_first_layer", true)
        .int("support_raft_layers", raft_layers)
        .build()
}

fn classic_wall_count_at(layer_index: u32, config: &ConfigView) -> usize {
    let module = ClassicPerimeters::from_config(config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer_index, &[make_region()], &paint, &mut output, config)
        .expect("run_perimeters must not panic");
    output.wall_loops().len()
}

/// DEV-124: with `support_raft_layers = 3`, layer 0 is raft — it must keep the
/// full wall count — and layer 3 is the first printed object layer, which is
/// where the clamp belongs. Before the fix this was exactly inverted.
#[test]
fn classic_clamp_follows_raft_layers_not_layer_zero() {
    let config = config_4_walls_with_raft(3);

    assert_eq!(
        classic_wall_count_at(0, &config),
        4,
        "DEV-124: layer 0 under a 3-layer raft is not the first printed layer \
         and must keep the configured wall count"
    );
    assert_eq!(
        classic_wall_count_at(3, &config),
        1,
        "DEV-124: layer 3 == support_raft_layers is the first printed layer and \
         must be clamped to one wall"
    );
}

/// DEV-124 regression guard: with no raft (the default), the clamp must still
/// fire on layer 0 exactly as before. This pins that the fix is a no-op for
/// every existing no-raft profile.
#[test]
fn classic_clamp_unchanged_when_no_raft_configured() {
    let config = config_4_walls_with_raft(0);
    assert_eq!(
        classic_wall_count_at(0, &config),
        1,
        "DEV-124: with raft_layers = 0 the clamp must still fire on layer 0"
    );
    assert_eq!(
        classic_wall_count_at(1, &config),
        4,
        "DEV-124: with raft_layers = 0 layer 1 must keep the configured count"
    );
}
