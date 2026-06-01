//! Convenience re-exports for module unit tests.
//!
//! `use slicer_test::prelude::*;` pulls in the most commonly used fixtures,
//! capture sinks, assertion helpers, and the `MockHost` adapter in one
//! import.

pub use crate::assert_paths::*;
pub use crate::capture::{InfillOutputCapture, PerimeterOutputCapture, SupportOutputCapture};
pub use crate::fixtures::{
    rect_path, square_polygon, ConfigViewBuilder, PerimeterRegionViewBuilder,
    SliceRegionViewBuilder,
};
pub use crate::mock_host::MockHost;
