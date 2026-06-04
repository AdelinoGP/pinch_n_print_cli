#![allow(missing_docs)]

//! TDD tests for packet 60 Step 6: `format_xyz` sibling function with configurable decimal
//! precision. AC-5 acceptance criteria.

use slicer_gcode::{format_coord, format_xyz};

#[test]
fn format_coord_decimals() {
    // format_xyz: configurable precision with trailing-zero stripping
    assert_eq!(format_xyz(1.23456, 3), "1.235");
    assert_eq!(format_xyz(1.0, 3), "1");
    assert_eq!(format_xyz(1.10000, 3), "1.1");

    // format_coord: legacy 4-decimal behavior is byte-identical (unchanged)
    assert_eq!(format_coord(1.23456), "1.2346");
}
