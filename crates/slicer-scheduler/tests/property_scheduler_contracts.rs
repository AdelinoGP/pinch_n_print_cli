//! Deterministic controls for the scheduler charters (`H-DAG`, `H-CLAIMS`,
//! `H-REGISTRY`).
//!
//! Every test here runs under a plain
//! `cargo test -p slicer-scheduler --features property_testing --test property_scheduler_contracts`.
//! Nothing in this file reads `PNP_PROPERTY_CONTEXT`, builds a `PropertyRun`,
//! or consults the property registry: these are hand-computed witnesses that
//! prove the reference models can reject the faults the randomized properties
//! are meant to catch, plus the negative cases that belong to the real
//! validating APIs.
//!
//! Each control module is mounted explicitly so the file compiles as part of
//! this one aggregated binary (an unmounted file would silently report zero
//! tests).

#![allow(missing_docs)]

// Each target-private model file is mounted exactly once for this binary and
// aliased by the control modules that need it. Mounting the same file from
// several `mod` declarations would compile a private copy per alias
// (`clippy::duplicate_mod`) and let the checked-bound helpers drift apart.
#[path = "common/property_scheduler_dag.rs"]
mod dag_models;

#[path = "common/property_scheduler_claims.rs"]
mod claim_models;

#[path = "common/property_scheduler_registry.rs"]
mod registry_models;

#[path = "common/property_scheduler_contracts/dag_controls.rs"]
mod dag_controls;

#[path = "common/property_scheduler_contracts/plan_controls.rs"]
mod plan_controls;

#[path = "common/property_scheduler_contracts/claims_controls.rs"]
mod claims_controls;

#[path = "common/property_scheduler_contracts/selection_controls.rs"]
mod selection_controls;

#[path = "common/property_scheduler_contracts/registry_controls.rs"]
mod registry_controls;

#[path = "common/property_scheduler_contracts/manifest_controls.rs"]
mod manifest_controls;

#[path = "common/property_scheduler_contracts/bound_controls.rs"]
mod bound_controls;
