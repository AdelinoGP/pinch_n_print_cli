//! Target-private reference model for `H-HELPERS-SPLIT`.
//!
//! Mounted by `tests/property_helpers.rs` and
//! `tests/property_helper_controls.rs`; never compiled as its own test binary.
//! It therefore holds no `proptest_` tests and needs no aggregated registration:
//!
//! * `bounds` — checked registry-bound readers and the validated profile limits,
//! * `mesh` — scene construction, the independent mesh predicates, and the local
//!   value tree,
//! * `split` — the property, the pairwise-reachability partition oracle and the
//!   deterministic witness pass,
//! * `controls` — the deterministic control bodies.
//!
//! Both targets compile every module, so each target would otherwise warn about
//! the half it does not call.

#![allow(dead_code)]

pub mod bounds;
pub mod controls;
pub mod mesh;
pub mod split;
