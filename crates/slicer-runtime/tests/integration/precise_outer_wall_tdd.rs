//! AC-7 + AC-N4: precise_outer_wall contract (T-053, packet 105).
//!
//! When precise_outer_wall=true AND wall_sequence=InnerOuter:
//!   - Inner walls are emitted BEFORE the outer wall in PerimeterRegion.walls.
//!   - The outer wall's inset uses ext_perimeter_spacing2 =
//!     (outer_wall_line_width + inner_wall_line_width)/2 rather than
//!     outer_wall_line_width/2.
//!
//! (OrcaSlicer PerimeterGenerator.cpp:1501-1506,1644)
//!
//! AC-N4: precise_outer_wall=true + wall_sequence=OuterInner → emission
//! identical to precise_outer_wall=false (gate-off: precise only applies when
//! wall_sequence=InnerOuter).

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ExtrusionRole, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// Run perimeters with the given config and return emitted wall loops.
fn run_with_config(config: slicer_ir::ConfigView, _wall_count: i64) -> Vec<slicer_ir::WallLoop> {
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();
    // Only return non-ThinWall, non-GapFill walls for clarity.
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
        .cloned()
        .collect()
}

/// AC-7 positive case: precise_outer_wall=true + wall_sequence=InnerOuter.
/// Inner walls must appear before the outer wall; the outer wall's X position
/// must reflect ext_perimeter_spacing2 inset rather than outer_wall_line_width/2.
#[test]
fn precise_mode_inner_first_and_spacing2() {
    let outer_w = 0.5_f32;
    let inner_w = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .string("wall_sequence", "InnerOuter")
        .bool("precise_outer_wall", true)
        .build();

    let walls = run_with_config(config, 3);
    assert_eq!(
        walls.len(),
        3,
        "Expected 3 wall loops (outer + 2 inner); got {}",
        walls.len()
    );

    // Ordering: inner walls must appear before the outer wall.
    // walls[0] and walls[1] should be inner; walls[2] should be outer.
    assert_eq!(
        walls[0].loop_type,
        LoopType::Inner,
        "First emitted wall should be Inner in precise mode"
    );
    assert_eq!(
        walls[1].loop_type,
        LoopType::Inner,
        "Second emitted wall should be Inner in precise mode"
    );
    assert_eq!(
        walls[2].loop_type,
        LoopType::Outer,
        "Third emitted wall should be Outer in precise mode (outer last)"
    );

    // Width check: outer wall vertices should have outer_w.
    let outer_wall = &walls[2];
    for pt in &outer_wall.path.points {
        assert!(
            (pt.width - outer_w).abs() < 0.005,
            "Outer wall vertex width {} != outer_w {}",
            pt.width,
            outer_w
        );
    }

    // Spacing check: in precise mode, the outer wall uses ext_perimeter_spacing2
    // = (outer_w + inner_w) / 2 for its inset (NOT outer_w / 2).
    // The outer wall's right-edge X should be:
    //   half_side - ext_perimeter_spacing2 = 5.0 - (0.5 + 0.4)/2 = 5.0 - 0.45 = 4.55 mm
    let half_side = 5.0_f32;
    let ext_perimeter_spacing2 = (outer_w + inner_w) / 2.0;
    let expected_outer_right = half_side - ext_perimeter_spacing2;
    let outer_x = outer_wall
        .path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MIN, f32::max);
    assert!(
        (outer_x - expected_outer_right).abs() < 0.005,
        "Precise-mode outer wall right edge X {} != expected {} (ext_perimeter_spacing2={})",
        outer_x,
        expected_outer_right,
        ext_perimeter_spacing2
    );
}

/// AC-N1 for precise: precise_outer_wall=false → standard InnerOuter ordering
/// (outer wall emitted FIRST).
#[test]
fn precise_mode_off_standard_spacing() {
    let outer_w = 0.5_f32;
    let inner_w = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .string("wall_sequence", "InnerOuter")
        .bool("precise_outer_wall", false)
        .build();

    let walls = run_with_config(config, 3);
    assert_eq!(walls.len(), 3, "Expected 3 wall loops");

    // Standard InnerOuter: outer first.
    assert_eq!(
        walls[0].loop_type,
        LoopType::Outer,
        "First emitted wall should be Outer in standard (precise=false) mode"
    );
    assert_eq!(
        walls[1].loop_type,
        LoopType::Inner,
        "Second emitted wall should be Inner in standard mode"
    );
    assert_eq!(
        walls[2].loop_type,
        LoopType::Inner,
        "Third emitted wall should be Inner in standard mode"
    );

    // Standard spacing: outer wall uses outer_w/2 for first inset.
    // Right-edge X = 5.0 - outer_w/2 = 5.0 - 0.25 = 4.75 mm
    let half_side = 5.0_f32;
    let expected_outer_right = half_side - outer_w / 2.0;
    let outer_wall = &walls[0];
    let outer_x = outer_wall
        .path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MIN, f32::max);
    assert!(
        (outer_x - expected_outer_right).abs() < 0.005,
        "Standard-mode outer wall right edge X {} != expected {}",
        outer_x,
        expected_outer_right
    );
}

/// AC-N4: precise_outer_wall=true + wall_sequence=OuterInner → gate-off.
/// The precise flag is SILENTLY IGNORED when wall_sequence != InnerOuter.
/// Emission must be identical to precise_outer_wall=false with OuterInner.
/// (OuterInner reverses all walls: innermost first, outer last.)
#[test]
fn gate_off_case_precise_true_outer_inner_sequence() {
    let outer_w = 0.5_f32;
    let inner_w = 0.4_f32;

    // precise=true + OuterInner → should behave like precise=false + OuterInner
    let config_precise = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .string("wall_sequence", "OuterInner")
        .bool("precise_outer_wall", true)
        .build();

    // precise=false + OuterInner → reference
    let config_standard = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .string("wall_sequence", "OuterInner")
        .bool("precise_outer_wall", false)
        .build();

    let walls_precise = run_with_config(config_precise, 3);
    let walls_standard = run_with_config(config_standard, 3);

    assert_eq!(
        walls_precise.len(),
        walls_standard.len(),
        "Gate-off: wall count must match between precise=true and precise=false with OuterInner"
    );

    // Compare loop types in order — should be identical.
    for (i, (pw, sw)) in walls_precise.iter().zip(walls_standard.iter()).enumerate() {
        assert_eq!(
            pw.loop_type, sw.loop_type,
            "Gate-off: wall[{}] loop_type differs: precise={:?} vs standard={:?}",
            i, pw.loop_type, sw.loop_type
        );
    }

    // Compare outer wall position: must match (no spacing2 shift when gated off).
    let find_max_x = |walls: &[slicer_ir::WallLoop]| {
        walls
            .iter()
            .filter(|w| w.path.role == ExtrusionRole::OuterWall)
            .flat_map(|w| w.path.points.iter())
            .map(|p| p.x)
            .fold(f32::MIN, f32::max)
    };
    let outer_x_precise = find_max_x(&walls_precise);
    let outer_x_standard = find_max_x(&walls_standard);
    assert!(
        (outer_x_precise - outer_x_standard).abs() < 0.005,
        "Gate-off: outer wall X {} != standard {} (precise flag must have no effect when OuterInner)",
        outer_x_precise,
        outer_x_standard
    );
}
