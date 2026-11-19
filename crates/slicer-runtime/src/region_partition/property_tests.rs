//! Deterministic controls and registry-backed properties for the fill-partition kernel
//! (`H-RUNTIME-MODIFIERS`, `H-RUNTIME-PARTITION`).
//!
//! This is a real submodule file rather than an inline `mod property_tests { ... }`
//! because `#[path]` on a nested module is resolved against the module's directory:
//! an inline module contributes its name to the path even when no such directory
//! exists, and POSIX path resolution fails on `<missing-dir>/../..`. Keeping the
//! module in `region_partition/property_tests.rs` makes every `..` step a real
//! directory on every host.
//!
//! The `#[cfg(test)]` and `#[cfg(feature = "property_testing")]` attributes are
//! stacked separately on the declaration in `region_partition.rs`, never combined
//! into `#[cfg(all(...))]`: `xtask`'s literal scanner recognizes only an exact
//! `#[cfg(test)]` ident and treats the module as test-only code, so the stacked
//! form keeps these fixtures visible to both the struct-literal and
//! test-quality scanners while compiling exactly the same code as the combined
//! gate would.
//!
//! The shared cell comparator and the two charter bodies live under `tests/` and
//! are mounted here by `#[path]` so the AC-N2 integration control certifies the
//! exact comparator these libtest records execute.

#[path = "../../tests/common/property_runtime_cells.rs"]
mod cells;
#[path = "../../tests/common/property_runtime_modifiers.rs"]
mod modifiers;
#[path = "../../tests/common/property_runtime_partition.rs"]
mod partition;

#[test]
fn proptest_runtime_modifier_transactions() {
    modifiers::run_property();
}

#[test]
fn proptest_runtime_fill_partition() {
    partition::run_property();
}

#[test]
fn runtime_modifier_ownership_and_angle_controls() {
    modifiers::runtime_modifier_ownership_and_angle_controls();
}

#[test]
fn runtime_modifier_identity_and_transaction_rejections() {
    modifiers::runtime_modifier_identity_and_transaction_rejections();
}

#[test]
fn runtime_modifier_collision_restores_taken_slice() {
    modifiers::runtime_modifier_collision_restores_taken_slice();
}

#[test]
fn runtime_partition_precedence_and_identity_controls() {
    partition::runtime_partition_precedence_and_identity_controls();
}

#[test]
fn runtime_partition_stage_and_domain_rejections() {
    partition::runtime_partition_stage_and_domain_rejections();
}
