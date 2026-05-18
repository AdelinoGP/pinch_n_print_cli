//! TDD tests for TASK-201 / packet 60 Step 1: 7 new precision keys on `ResolvedConfig`.

use slicer_ir::resolved_config::ResolvedConfig;

#[test]
fn new_precision_keys_have_orca_defaults() {
    let cfg = ResolvedConfig::default();
    assert_eq!(cfg.gcode_resolution, 0.0125_f32);
    assert_eq!(cfg.infill_resolution, 0.04_f32);
    assert_eq!(cfg.support_resolution, 0.0375_f32);
    assert_eq!(cfg.min_segment_length, 0.05_f32);
    assert_eq!(cfg.gcode_xy_decimals, 3_u32);
    assert_eq!(cfg.perimeter_arc_tolerance, 0.0125_f32);
    assert_eq!(cfg.slice_closing_radius, 0.049_f32);
}
