# Design: 205e-integrated-parity-harness

## Controlling Code Paths

- Primary code path: `tests/common/mod.rs`, `parity_invariants.rs`, and `contract/integrated_parity_*_tdd.rs`.
- Neighboring tests/fixtures: `parity_invariants_selftest_tdd.rs`, `native_infill_claim_resolution_tdd.rs`, and `full_coverage_external_override_tdd.rs`.
- OrcaSlicer comparison: existing structural parity only; no new reference required.

## Architecture Constraints

- ADR-0042 forbids weakening structural invariants into byte equality or self-captured snapshot equality.
- ADR-0056 requires both dispatch paths for every integrated module.
- Keep `ParityTolerance` defaults exact: `1e-3`, `1e-3`, and `2.0`.

## Code Change Surface

- Selected approach: introduce a family-aware harness that accepts module-specific setup closures/values, then migrate tests without moving fixture ownership or changing assertions.
- Exact surfaces: `tests/common/mod.rs`, new or existing parity harness helpers, `parity_invariants.rs` container helpers, and 21 parity test files.
- Rejected alternative: one macro generating all tests; it would hide module-specific fixtures and make failures less local.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/common/mod.rs` - shared harness exports and setup helper.
- `crates/slicer-runtime/tests/common/parity_invariants.rs` - shared comparator scaffolding only.
- `crates/slicer-runtime/tests/contract/integrated_parity_*_tdd.rs` - migrate all 21 tests; exact files are the existing 21 mounted modules in `contract/main.rs:25-45`.
- `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs` - only if helper signatures require test fixture adaptation; assertions are locked.

## Read-Only Context

- `crates/slicer-runtime/tests/common/mod.rs:1-40, 455-495` - current helper exports and input builders.
- `crates/slicer-runtime/tests/common/parity_invariants.rs:1-180, 384-565, 643-773, 780-1248` - tolerance and comparator families.
- `crates/slicer-runtime/tests/contract/main.rs:20-50` - 21-test inventory.
- One representative parity file from each family: layer, prepass, finalization, postpass.

## Out-of-Bounds Files

- Production crates, module algorithms, WIT, registry, edition config.
- OrcaSlicerDocumented source; delegate if a reference question appears.
- `target/`, Cargo.lock, generated code, vendored dependencies.

## Expected Sub-Agent Dispatches

- Question: inventory each parity file's module-specific inputs versus repeated setup; scope: 21 parity files; return: `SUMMARY` <=200 words.
- Question: identify comparator self-test names and counts before migration; scope: `parity_invariants_selftest_tdd.rs`; return: `LOCATIONS`.
- Question: run the contract test binary after migration and report only pass/fail and failing test names; scope: cargo test; return: `FACT`.

## Data and Contract Notes

- Native and WASM inputs remain equivalent and are constructed by the harness, not by production code.
- Structural comparator diagnostics must retain family names and region/stage identity.
- Test fixtures remain module-specific to preserve locality of failures.

## Locked Assumptions and Invariants

- 21 integrated parity modules remain mounted and executed.
- Six comparator families and their negative self-tests remain.
- No tolerance loosening, ignored tests, or byte-equality substitution.

## Risks and Tradeoffs

- Over-generalizing setup can make failures less local; keep stage-family adapters explicit.
- A closure-based harness can produce opaque type errors; use small concrete helper structs where compiler diagnostics are clearer.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: full contract test, `FACT`.

## Open Questions

- `[FWD]` Which four family-specific harness inputs produce the smallest helper surface while retaining module-specific fixture locality?
