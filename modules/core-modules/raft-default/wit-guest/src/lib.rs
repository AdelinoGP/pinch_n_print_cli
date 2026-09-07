//! Component-model guest wrapper for `raft-default`.
//!
//! Exists solely to compile the real `raft-default` crate for the
//! `wasm32-unknown-unknown` target as a `cdylib` so the
//! `#[slicer_module]`-emitted component-export module is preserved
//! in the final `.wasm`. No logic lives here.

#[allow(unused_imports)]
pub use raft_default::RaftDefault;
