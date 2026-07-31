use std::time::{Duration, UNIX_EPOCH};

use crate::common::slicer_cache::staleness_reason;

#[test]
fn older_binary_is_stale() {
    let old_binary = UNIX_EPOCH + Duration::from_secs(10);
    let newer_source = UNIX_EPOCH + Duration::from_secs(20);

    let reason = staleness_reason(Some(old_binary), newer_source).expect("older binary is stale");
    assert!(reason.contains("pnp_cli"));
    assert!(reason.contains("stale"));
    // The remedy wording is part of the contract: a narrow `cargo test -p
    // slicer-runtime` does not rebuild another package's binary, so the reason
    // must tell the reader exactly what to run.
    assert!(
        reason.contains("cargo build -p pnp-cli"),
        "staleness reason must name the remedy: {reason}"
    );
}

#[test]
fn absent_binary_is_stale() {
    let reason = staleness_reason(None, UNIX_EPOCH).expect("absent binary is stale");
    assert!(reason.contains("pnp_cli"));
    assert!(
        reason.contains("cargo build -p pnp-cli"),
        "staleness reason must name the remedy: {reason}"
    );
}

#[test]
fn fresh_binary_is_not_stale() {
    let new_binary = UNIX_EPOCH + Duration::from_secs(20);
    let older_source = UNIX_EPOCH + Duration::from_secs(10);

    assert_eq!(staleness_reason(Some(new_binary), older_source), None);
}
