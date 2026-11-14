//! Registry-backed scheduler properties: `H-DAG`, `H-CLAIMS`, `H-REGISTRY`.
//!
//! Each test reads its exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative bodies
//! live in the target-private model modules this file mounts. None of them may
//! hardcode a Proptest configuration.
//!
//! The charters, oracles and failure destinations are documented in
//! `docs/testing/properties.md#h-dag`, `#h-claims` and `#h-registry`; the
//! executed evidence is `docs/testing/proptest-scheduler-evidence.md`.

#![allow(missing_docs)]

#[path = "common/property_scheduler_dag.rs"]
mod dag;

#[path = "common/property_scheduler_claims.rs"]
mod claims;

#[path = "common/property_scheduler_registry.rs"]
mod registry;

#[test]
fn proptest_scheduler_dag_models() {
    dag::run_property();
}

#[test]
fn proptest_scheduler_claim_models() {
    claims::run_property();
}

#[test]
fn proptest_scheduler_region_split_registry() {
    registry::run_property();
}
