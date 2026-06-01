//! Test utilities for module-level unit tests.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod assert_paths;
pub mod capture;
pub mod fixtures;
pub mod mock_host;
pub mod prelude;

pub use mock_host::MockHost;
