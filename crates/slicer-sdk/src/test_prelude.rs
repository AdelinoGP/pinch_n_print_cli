#![cfg(any(test, feature = "test"))]
//! Convenience re-exports for module unit tests.
//!
//! `use slicer_sdk::test_prelude::*;` pulls in the most commonly used fixtures,
//! capture sinks, assertion helpers, and the `MockHost` adapter in one import.
//!
//! Whole-module feature gate (`#![cfg(any(test, feature = "test"))]`) means
//! the prelude is either fully present or fully absent — never partial. This
//! preserves IDE jump-to-definition and `cargo doc` output stability (per
//! ADR-0004's grilling decision; production `slicer_sdk::prelude` stays
//! test-free).

pub use crate::test_support::assert_paths::assert_extrusion_width_range;
pub use crate::test_support::assert_paths::assert_max_segment_length;
pub use crate::test_support::assert_paths::assert_no_path_intersections;
pub use crate::test_support::assert_paths::assert_paths_inside_polygon;
pub use crate::test_support::assert_paths::assert_paths_planar;
pub use crate::test_support::capture::InfillOutputCapture;
pub use crate::test_support::capture::PerimeterOutputCapture;
pub use crate::test_support::capture::SupportOutputCapture;
pub use crate::test_support::fixtures::print_entity;
pub use crate::test_support::fixtures::rect_path;
pub use crate::test_support::fixtures::rect_polygon;
pub use crate::test_support::fixtures::seam_candidate;
pub use crate::test_support::fixtures::square_polygon;
pub use crate::test_support::fixtures::tool_change;
pub use crate::test_support::fixtures::ConfigViewBuilder;
pub use crate::test_support::fixtures::LayerCollectionFixtureBuilder;
pub use crate::test_support::fixtures::PerimeterRegionViewBuilder;
pub use crate::test_support::fixtures::SliceRegionViewBuilder;
pub use crate::test_support::mock_host::MockHost;
