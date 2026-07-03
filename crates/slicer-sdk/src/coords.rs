//! Coordinate conversion helpers for module authors.

/// Scaled integer coordinate factor used for 2D geometry.
///
/// One unit is 100nm (10^-4 mm), so one millimeter equals 10_000 units.
/// Delegates to `slicer_ir::UNITS_PER_MM` — the single authoritative source
/// for the mm→unit factor; change it there, never here.
pub const SCALING_FACTOR: i64 = slicer_ir::UNITS_PER_MM as i64;

/// Converts millimeters to scaled integer units.
#[inline(always)]
#[must_use]
pub fn mm_to_units(mm: f32) -> i64 {
    (mm * SCALING_FACTOR as f32).round() as i64
}

/// Converts scaled integer units back to millimeters.
#[inline(always)]
#[must_use]
pub fn units_to_mm(units: i64) -> f32 {
    units as f32 / SCALING_FACTOR as f32
}
