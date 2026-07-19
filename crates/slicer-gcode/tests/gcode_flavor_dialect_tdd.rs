//! RED tests for the five supported G-code flavor dialects.
//!
//! The expected command forms follow canonical `GCodeWriter.cpp` functions:
//! `set_temperature`, `set_acceleration_internal`, `set_jerk_xy`,
//! `set_junction_deviation`, `set_pressure_advance`, and
//! `supports_separate_travel_acceleration`.

use slicer_gcode::GcodeFlavor;
use std::sync::{Mutex, OnceLock};

struct CapturingLogger(Mutex<Vec<(log::Level, String)>>);

static CAPTURED: OnceLock<CapturingLogger> = OnceLock::new();

impl log::Log for CapturingLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        self.0
            .lock()
            .unwrap()
            .push((record.level(), format!("{}", record.args())));
    }

    fn flush(&self) {}
}

fn install_capture() -> &'static CapturingLogger {
    let logger = CAPTURED.get_or_init(|| CapturingLogger(Mutex::new(Vec::new())));
    let _ = log::set_logger(logger);
    log::set_max_level(log::LevelFilter::Trace);
    logger
}

#[test]
fn flavor_parses_five_config_strings() {
    assert_eq!(GcodeFlavor::from_config_str("marlin"), GcodeFlavor::Marlin);
    assert_eq!(
        GcodeFlavor::from_config_str("marlin2"),
        GcodeFlavor::Marlin2
    );
    assert_eq!(
        GcodeFlavor::from_config_str("klipper"),
        GcodeFlavor::Klipper
    );
    assert_eq!(
        GcodeFlavor::from_config_str("reprapfirmware"),
        GcodeFlavor::RepRapFirmware
    );
    assert_eq!(
        GcodeFlavor::from_config_str("repetier"),
        GcodeFlavor::Repetier
    );
    assert_eq!(GcodeFlavor::default(), GcodeFlavor::Marlin);
}

/// Mirrors canonical `GCodeWriter.cpp::set_temperature`, including RRF's
/// separate wait-for-all command.
#[test]
fn rrf_temperature_uses_g10_and_m116() {
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_temperature(0, 210.5, false),
        "G10 P0 S210.5\n"
    );
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_temperature(0, 210.5, true),
        "G10 P0 S210.5\nM116\n"
    );

    for flavor in [
        GcodeFlavor::Marlin,
        GcodeFlavor::Marlin2,
        GcodeFlavor::Klipper,
        GcodeFlavor::Repetier,
    ] {
        assert_eq!(flavor.set_temperature(0, 210.5, false), "M104 T0 S210.5\n");
        assert_eq!(flavor.set_temperature(0, 210.5, true), "M109 T0 S210.5\n");
    }
}

/// Mirrors canonical `GCodeWriter.cpp::set_acceleration_internal`.
#[test]
fn acceleration_dialect_per_flavor() {
    assert_eq!(GcodeFlavor::Marlin.set_acceleration(1000), "M204 S1000\n");
    assert_eq!(GcodeFlavor::Marlin2.set_acceleration(1000), "M204 P1000\n");
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_acceleration(1000),
        "M204 P1000\n"
    );
    assert_eq!(
        GcodeFlavor::Repetier.set_acceleration(1000),
        "M201 X1000 Y1000\n"
    );
    assert_eq!(
        GcodeFlavor::Klipper.set_acceleration(1000),
        "SET_VELOCITY_LIMIT ACCEL=1000\n"
    );
}

/// Mirrors canonical `GCodeWriter.cpp::supports_separate_travel_acceleration`
/// and the travel branch of `set_acceleration_internal`.
#[test]
fn travel_acceleration_capability_matrix() {
    assert!(GcodeFlavor::Repetier.supports_separate_travel_acceleration());
    assert!(GcodeFlavor::Marlin2.supports_separate_travel_acceleration());
    assert!(GcodeFlavor::RepRapFirmware.supports_separate_travel_acceleration());
    assert!(!GcodeFlavor::Marlin.supports_separate_travel_acceleration());
    assert!(!GcodeFlavor::Klipper.supports_separate_travel_acceleration());

    assert_eq!(
        GcodeFlavor::Repetier.set_travel_acceleration(2000),
        Some("M202 X2000 Y2000\n".to_string())
    );
    assert_eq!(
        GcodeFlavor::Marlin2.set_travel_acceleration(2000),
        Some("M204 T2000\n".to_string())
    );
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_travel_acceleration(2000),
        Some("M204 T2000\n".to_string())
    );
    assert_eq!(GcodeFlavor::Marlin.set_travel_acceleration(2000), None);
    assert_eq!(GcodeFlavor::Klipper.set_travel_acceleration(2000), None);
}

/// Mirrors canonical `GCodeWriter.cpp::set_jerk_xy`. The dialect contract
/// pins jerk output at zero fractional digits for all five flavors.
#[test]
fn jerk_xy_dialect_per_flavor() {
    assert_eq!(
        GcodeFlavor::Klipper.set_jerk_xy(10.0),
        "SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=10\n"
    );
    assert_eq!(GcodeFlavor::Repetier.set_jerk_xy(10.0), "M207 X10\n");
    for flavor in [
        GcodeFlavor::Marlin,
        GcodeFlavor::Marlin2,
        GcodeFlavor::RepRapFirmware,
    ] {
        assert_eq!(flavor.set_jerk_xy(10.0), "M205 X10 Y10\n");
    }
}

/// Mirrors canonical `GCodeWriter.cpp::set_junction_deviation`; the Rust
/// dialect uses `None` rather than an empty string for unsupported flavors.
#[test]
fn junction_deviation_marlin2_only() {
    assert_eq!(
        GcodeFlavor::Marlin2.set_junction_deviation(0.05),
        Some("M205 J0.05\n".to_string())
    );
    assert_eq!(GcodeFlavor::Marlin.set_junction_deviation(0.05), None);
    assert_eq!(GcodeFlavor::Klipper.set_junction_deviation(0.05), None);
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_junction_deviation(0.05),
        None
    );
    assert_eq!(GcodeFlavor::Repetier.set_junction_deviation(0.05), None);
}

/// Mirrors canonical `GCodeWriter.cpp::set_pressure_advance`. The dialect
/// contract pins four fractional digits for every flavor.
#[test]
fn pressure_advance_dialect_per_flavor() {
    assert_eq!(
        GcodeFlavor::Marlin.set_pressure_advance(0.05),
        "M900 K0.0500\n"
    );
    assert_eq!(
        GcodeFlavor::Marlin2.set_pressure_advance(0.05),
        "M900 K0.0500\n"
    );
    assert_eq!(
        GcodeFlavor::Klipper.set_pressure_advance(0.05),
        "SET_PRESSURE_ADVANCE ADVANCE=0.0500\n"
    );
    assert_eq!(
        GcodeFlavor::RepRapFirmware.set_pressure_advance(0.05),
        "M572 D0 S0.0500\n"
    );
    assert_eq!(
        GcodeFlavor::Repetier.set_pressure_advance(0.05),
        "M233 X0.0500 Y0.0500\n"
    );
}

#[test]
fn bed_temperature_uniform() {
    for flavor in [
        GcodeFlavor::Marlin,
        GcodeFlavor::Marlin2,
        GcodeFlavor::Klipper,
        GcodeFlavor::RepRapFirmware,
        GcodeFlavor::Repetier,
    ] {
        assert_eq!(flavor.set_bed_temperature(60.5, false), "M140 S60.5\n");
        assert_eq!(flavor.set_bed_temperature(60.5, true), "M190 S60.5\n");
    }
}

#[test]
fn unknown_flavor_falls_back_to_marlin() {
    let logger = install_capture();
    logger.0.lock().unwrap().clear();

    let result = GcodeFlavor::from_config_str("smoothie");
    let messages = logger.0.lock().unwrap().clone();

    assert_eq!(result, GcodeFlavor::Marlin);
    assert!(
        messages
            .iter()
            .any(|(level, message)| *level == log::Level::Warn && message.contains("smoothie")),
        "expected log::warn! to mention 'smoothie'; got: {messages:?}"
    );
}

#[test]
fn default_is_marlin() {
    assert_eq!(GcodeFlavor::default(), GcodeFlavor::Marlin);
}

#[test]
fn config_str_roundtrip() {
    assert_eq!(GcodeFlavor::Marlin.config_str(), "marlin");
    assert_eq!(GcodeFlavor::Marlin2.config_str(), "marlin2");
    assert_eq!(GcodeFlavor::Klipper.config_str(), "klipper");
    assert_eq!(GcodeFlavor::RepRapFirmware.config_str(), "reprapfirmware");
    assert_eq!(GcodeFlavor::Repetier.config_str(), "repetier");
}
