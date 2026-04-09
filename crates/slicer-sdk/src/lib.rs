//! ModularSlicer SDK foundations.
//!
//! This crate currently provides:
//! - stable re-exports for shared IR types,
//! - host service wrapper functions,
//! - coordinate conversion helpers,
//! - module traits (LayerModule),
//! - error types (ModuleError),
//! - view types (SliceRegionView, PerimeterRegionView),
//! - output builders (InfillOutputBuilder, PerimeterOutputBuilder, etc.).

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod builders;
pub mod coords;
pub mod error;
pub mod host;
pub mod prelude;
pub mod traits;
pub mod views;

/// Re-export of the shared IR crate used by host and modules.
pub use slicer_ir as ir;
