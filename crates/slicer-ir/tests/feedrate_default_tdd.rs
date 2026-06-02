#![allow(missing_docs)]

use slicer_ir::FeedrateConfig;

#[test]
fn feedrate_default_returns_documented_values() {
    let cfg = FeedrateConfig::default();

    assert_eq!(cfg.outer_wall_speed, 60.0);
    assert_eq!(cfg.inner_wall_speed, 60.0);
    assert_eq!(cfg.thin_wall_speed, 30.0);
    assert_eq!(cfg.top_surface_speed, 100.0);
    assert_eq!(cfg.bottom_surface_speed, 100.0);
    assert_eq!(cfg.sparse_infill_speed, 100.0);
    assert_eq!(cfg.bridge_speed, 25.0);
    assert_eq!(cfg.internal_bridge_speed, 37.5);
    assert_eq!(cfg.support_speed, 80.0);
    assert_eq!(cfg.support_interface_speed, 80.0);
    assert_eq!(cfg.gap_infill_speed, 30.0);
    assert_eq!(cfg.ironing_speed, 20.0);
    assert_eq!(cfg.skirt_speed, 50.0);
    assert_eq!(cfg.wipe_tower_speed, 90.0);
    assert_eq!(cfg.prime_tower_speed, 90.0);
    assert_eq!(cfg.travel_speed, 120.0);
    assert_eq!(cfg.travel_speed_z, 0.0);
    assert_eq!(cfg.initial_layer_speed, 30.0);
    assert_eq!(cfg.initial_layer_infill_speed, 60.0);
    assert_eq!(cfg.initial_layer_travel_speed, 120.0);
    assert_eq!(cfg.wipe_speed, 96.0);
    assert_eq!(cfg.overhang_1_4_speed, 0.0);
    assert_eq!(cfg.overhang_2_4_speed, 0.0);
    assert_eq!(cfg.overhang_3_4_speed, 0.0);
    assert_eq!(cfg.overhang_4_4_speed, 0.0);
    assert_eq!(cfg.filament_ironing_speed, 0.0);
}

#[test]
fn feedrate_field_count_is_26() {
    let cfg = FeedrateConfig::default();
    let mut count = 0usize;
    count += cfg.outer_wall_speed.is_finite() as usize;
    count += cfg.inner_wall_speed.is_finite() as usize;
    count += cfg.thin_wall_speed.is_finite() as usize;
    count += cfg.top_surface_speed.is_finite() as usize;
    count += cfg.bottom_surface_speed.is_finite() as usize;
    count += cfg.sparse_infill_speed.is_finite() as usize;
    count += cfg.bridge_speed.is_finite() as usize;
    count += cfg.internal_bridge_speed.is_finite() as usize;
    count += cfg.support_speed.is_finite() as usize;
    count += cfg.support_interface_speed.is_finite() as usize;
    count += cfg.gap_infill_speed.is_finite() as usize;
    count += cfg.ironing_speed.is_finite() as usize;
    count += cfg.skirt_speed.is_finite() as usize;
    count += cfg.wipe_tower_speed.is_finite() as usize;
    count += cfg.prime_tower_speed.is_finite() as usize;
    count += cfg.travel_speed.is_finite() as usize;
    count += cfg.travel_speed_z.is_finite() as usize;
    count += cfg.initial_layer_speed.is_finite() as usize;
    count += cfg.initial_layer_infill_speed.is_finite() as usize;
    count += cfg.initial_layer_travel_speed.is_finite() as usize;
    count += cfg.wipe_speed.is_finite() as usize;
    count += cfg.overhang_1_4_speed.is_finite() as usize;
    count += cfg.overhang_2_4_speed.is_finite() as usize;
    count += cfg.overhang_3_4_speed.is_finite() as usize;
    count += cfg.overhang_4_4_speed.is_finite() as usize;
    count += cfg.filament_ironing_speed.is_finite() as usize;
    assert_eq!(count, 26, "FeedrateConfig must have exactly 26 fields");
}
