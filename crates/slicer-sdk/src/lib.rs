//! ModularSlicer SDK foundations.
//!
//! This crate currently provides:
//! - stable re-exports for shared IR types,
//! - host service wrapper functions,
//! - coordinate conversion helpers.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod coords;
pub mod host;
pub mod prelude;

/// Re-export of the shared IR crate used by host and modules.
pub use slicer_ir as ir;
