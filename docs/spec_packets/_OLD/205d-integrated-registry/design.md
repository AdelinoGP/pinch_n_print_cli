# Design: 205d-integrated-registry

## Controlling Code Paths

- Primary code path: `slicer_integrated_modules::integrated_registrations`, `native_entries`, manifest constants, and feature-gated coverage tests.
- Neighboring tests/fixtures: `full_coverage_tests`, hybrid pilot tests, and external override integration test.
- OrcaSlicer comparison: none; registry identity is PnP-owned.

## Architecture Constraints

- Preserve deterministic registration order unless the existing callers explicitly treat order as irrelevant; if order changes, add the exact ordering invariant to the packet tests.
- Preserve `com.core.<name>` IDs and `integrated://<name>` origin labels.
- Preserve the empty default feature set and all 21 Cargo feature names.

## Code Change Surface

- Selected approach: use one declarative registry metadata source and generate both registration and native-entry surfaces plus coverage expectations.
- Exact functions: `integrated_registrations`, `native_entries`, `manifest_const!`, `full_coverage_tests`.
- Rejected alternative: only extract helper functions; that would move repetition without making the interface deep.

## Files in Scope (read + edit)

- `crates/slicer-integrated-modules/src/lib.rs` - registry authority, generated surfaces, and coverage tests.
- `crates/slicer-integrated-modules/Cargo.toml` - only if the chosen declarative representation requires feature metadata adjustment; feature names must not change.
- `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` - only if it can consume the registry authority without a second list.

## Read-Only Context

- `crates/pnp-cli/Cargo.toml:20-48` - passthrough feature contract.
- `dist/editions.toml:35-47` - edition membership contract.
- `crates/slicer-integrated-modules/src/lib.rs:48-400, 402-678` - current registry and tests.
- `docs/adr/0056...` and `docs/adr/0057...` - direct reads.

## Out-of-Bounds Files

- `crates/pnp-cli/Cargo.toml`, `dist/editions.toml`, `xtask/**` - no feature or edition changes.
- `crates/slicer-wasm-host/**` and module algorithms.
- `target/`, `Cargo.lock`, generated code, vendored dependencies.

## Expected Sub-Agent Dispatches

- Question: enumerate all consumers of `integrated_registrations` and `native_entries` and whether they depend on vector order; scope: workspace source/tests; return: `LOCATIONS`.
- Question: verify the 21 feature-to-module identity mapping after the edit; scope: Cargo manifests and registry source; return: `FACT`.

## Data and Contract Notes

- Registry rows carry manifest text/origin and native entry identity; both must agree on module ID.
- Cargo feature gating remains the compile-time selector.
- External modules still win by existing search priority.

## Locked Assumptions and Invariants

- Exactly 21 core module feature names remain available.
- Default features remain empty.
- Registry vectors contain one entry per enabled feature and no duplicate module IDs.

## Risks and Tradeoffs

- Macro-generated function pointers may complicate feature-gated imports; prefer a small local declarative macro over a proc-macro or build script.
- A table storing function pointers may require explicit cfg blocks for imports; those cfg blocks are acceptable if the module row itself is not repeated across outputs.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: consumer/order inventory, `LOCATIONS`.

## Open Questions

- `[FWD]` Does any consumer rely on the current registration order, or can the generated table use canonical module-ID order?
