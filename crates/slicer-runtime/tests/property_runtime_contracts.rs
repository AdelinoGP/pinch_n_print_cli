//! Deterministic controls for the runtime charters (`H-RUNTIME-*`).
//!
//! Every control here runs under a plain
//! `cargo test -p slicer-runtime --features property_testing --test property_runtime_contracts`.
//! They are hand-built witnesses that prove the independent models and the real
//! validating APIs can reject the faults the randomized properties must catch.
//! No control in this binary substitutes for a registry execution.
//!
//! Each control module is mounted explicitly below; an unmounted file would
//! silently report zero tests.

#![allow(missing_docs)]

#[path = "common/property_runtime_cells.rs"]
mod cells;

#[allow(dead_code)]
#[path = "common/property_runtime_dispatch.rs"]
mod dispatch;

#[allow(dead_code)]
#[path = "common/property_runtime_selection.rs"]
mod selection;

#[path = "common/property_runtime_contracts/cells_controls.rs"]
mod cells_controls;

#[path = "common/property_runtime_contracts/dispatch_controls.rs"]
mod dispatch_controls;

#[path = "common/property_runtime_contracts/selection_controls.rs"]
mod selection_controls;
