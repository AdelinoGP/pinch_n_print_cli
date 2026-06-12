# ADR-0016 — `test_prelude` Lives in a Whole-Module-Gated File, Separate from the Production `prelude`

## Status

Accepted (Packet 78 / TASK-227).

## Context

`slicer-sdk` exposes two convenience re-export modules: `prelude` (production helpers — types, traits, the `host` facade) and `test_prelude` (test helpers — `MockHost`, fixture builders, assertion helpers, captures). Both are intended to be `use slicer_sdk::*::*;`-ed at the top of files that need them.

The question was how to gate `test_prelude` so that production guest WASM builds pay zero cost. Two approaches were considered:

1. **Per-item `#[cfg]` gates inside a single `prelude` module.** One file, items selectively enabled or disabled by `#[cfg(any(test, feature = "test"))]`. Compact in the source tree.
2. **A whole-module gate on a separate `test_prelude.rs` file.** One `#![cfg(any(test, feature = "test"))]` at the top of the file; either the entire module exists or none of it does.

Per-item gates have a real downside: IDE jump-to-definition shows the definition with a conditional grey-out around it; rustdoc emits the item with a "feature flag required" badge; reading the prelude source means mentally projecting the cfg state at every line. Whole-module gates are blunter but cleaner — the module is either fully present (test build) or fully absent (production build), with no middle state.

This decision is small in scope but high in repetition cost. It will be reapplied every time a new test-support artefact is added (test_prelude itself, test_support submodules, future test-only helpers).

## Decision

**Test-support APIs live in whole-module-gated files separate from the production `prelude`.** Concretely:

- `slicer_sdk::test_prelude` is a separate file (`crates/slicer-sdk/src/test_prelude.rs`) whose first non-comment line is `#![cfg(any(test, feature = "test"))]`. The `pub mod test_prelude;` declaration in `lib.rs` carries the same cfg.
- `slicer_sdk::test_support` and its submodules (`mock_host`, `fixtures`, `captures`, `asserts`) follow the same pattern — whole-module gated, never per-item.
- The production `slicer_sdk::prelude` stays test-free. Test imports come via `use slicer_sdk::test_prelude::*;` from `#[cfg(test)]`-gated test modules or from integration test files, never from production module source.
- The `test` Cargo feature is declared with an empty deps list (`test = []`) on `slicer-sdk`. Dependents enable it as a dev-dep:

  ```toml
  [dependencies]
  slicer-sdk = { path = "...", default-features = false }
  [dev-dependencies]
  slicer-sdk = { path = "...", features = ["test"] }
  ```

  Cargo's feature unification ensures dev builds see the `test` feature and production guest builds do not.

## Consequences

- **IDE and rustdoc surfaces are clean.** Reading `prelude.rs` or `test_prelude.rs` shows the actual module contents without conditional greyouts.
- **Production guest WASM is provably test-free.** `cargo xtask build-guests` never enables the `test` feature; `crates/slicer-wasm-host/test-guests/.../tests/...` (which DO enable it) target only the host-side test runner. The acceptance check is `grep` for `slicer_sdk::test_` symbols in the `.wasm` artefact — none should appear.
- **The dev-dep dual-line pattern becomes part of the SDK contract.** Module authors are told to follow it; CLAUDE.md (Guest Build Invariants) documents the rationale.
- **Future test-only helpers go in whole-module files, not in `prelude` with per-item gates.** Every reviewer who proposes the latter should be redirected to this ADR.

## Rejected alternatives

- **Per-item `#[cfg]` gates inside `prelude`.** Cleaner source-tree footprint (one file) but worse IDE/docs experience and harder to reason about. Rejected.
- **Single `slicer_sdk::sdk` umbrella that branches by cfg.** Hides the test/production split from the import path; module authors lose the explicit `use slicer_sdk::test_prelude::*;` signal. Rejected.
- **No prelude at all — re-export everything from `lib.rs`.** Forces users to either glob-import everything (noisy) or fully-qualify every type (verbose). Rejected — the prelude is a real ergonomics win, just one that needs disciplined gating.

## Future reviewers

- Do not propose collapsing `test_prelude` back into `prelude` with per-item gates; the wins are bigger than they look.
- Do not add test-only items to `prelude.rs`; if a helper is test-shaped, it goes in `test_prelude.rs` or `test_support/`.
- Do not weaken the gate to `#[cfg(test)]` alone — the `feature = "test"` half is what makes the dev-dep pattern work for downstream crates.
