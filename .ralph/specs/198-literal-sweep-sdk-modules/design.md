# Design: 198-literal-sweep-sdk-modules

## Controlling Code Paths

- Primary code path: none changed — test-scope construction sites only. Read-only anchors: fixtures in `crates/slicer-sdk/src/test_support/fixtures.rs` (packet 195), the `#[cfg(any(test, feature = "test"))]` gate on `test_support` (`crates/slicer-sdk/src/lib.rs`), `shared_input_paths` in `xtask/src/build_guests.rs` (why the sdk manifest edit trips guest staleness).
- Neighboring tests/fixtures: the sdk's existing 17 `[[test]]` `required-features = ["test"]` entries in `crates/slicer-sdk/Cargo.toml` (the gating convention this packet extends); packet 195 adds an 18th (`test_support_fixture_bases_tdd`).
- OrcaSlicer comparison: not applicable — no parity surface.

## Architecture Constraints

- **Sdk manifest gating is the packet's only manifest edit and its only guest-input edit (grounded 2026-08-07).** `crates/slicer-sdk/tests/*.rs` files referencing `test_support` MUST be `[[test]]`-gated with `required-features = ["test"]`, because integration-test binaries do not see `cfg(test)` on the linked sdk lib. Non-gated files whose only violations involve `Default`-able types (`Point3WithWidth`, `GlobalLayer`, `LayerCollectionIR`) convert with plain FRU and stay ungated (grounded candidate: `finalization_module_tdd.rs`). Files needing class-b fixtures get gated (grounded candidates: `layer_module_tdd.rs`, `finalization_builder_tdd.rs`).
- **Module manifests are already correct.** All 21 `modules/core-modules/*/Cargo.toml` carry `[dev-dependencies] slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` (verified 2026-08-07, e.g. `arachne-perimeters`); a dev-dep feature applies to every test target, so module manifests need zero `required-features` entries and zero edits (AC-N4). Guests never enable `test`: it appears only under `[dev-dependencies]`, which wasm guest builds never resolve (ADR-0004 amendment).
- **Fixture bases stay exhaustive-with-waiver.** `crates/slicer-sdk/src/test_support/fixtures.rs` literals are deliberate propagation checkpoints — when a watched type gains a field, the fixture is where the compiler forces one conscious decision. If the Step-1 report flags them, add `// exhaustive: fixture base is the single propagation checkpoint for this type` rather than FRU. Fixture signatures and returned values are packet-195 contract; never adjust them here.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The guest-feeding path in this packet is exactly `crates/slicer-sdk/Cargo.toml` (collected by `shared_input_paths` alongside shared-crate `src/`; verified in `xtask/src/build_guests.rs` 2026-08-07). `STALE:` for all guests immediately after the manifest edit is EXPECTED — the mtime check cannot know a `[[test]]` section is guest-inert. Rebuild, then gate clean (AC-5). Sdk `tests/**` edits do NOT trip the gate (only `src/`, `Cargo.toml`, `build.rs` are collected).

## Code Change Surface

- Selected approach: report-driven sweep — sdk first (gating + conversions), then modules in two batches, then rebuild + gates.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-sdk/Cargo.toml`: append `[[test]]` entries with `required-features = ["test"]` for newly fixture-consuming test files (grounded candidates above; re-derive from report).
  - `crates/slicer-sdk/tests/**`: `PrintEntity` → `print_entity_base`, `WallLoop` → `wall_loop_base`, `OrderedEntityView` → `ordered_entity_view_base` (in-crate path `slicer_sdk::test_support::fixtures::...`); `Point3WithWidth`/`LayerCollectionIR`/`GlobalLayer` → FRU; carrier tests (e.g. roundtrip-style `test_support_*` assertions that every field travels) keep exhaustive literals with waivers.
  - `crates/slicer-sdk/src/test_support/**`: waiver comments only, where reported.
  - `modules/core-modules/*/tests/**`: same conversion vocabulary through the existing dev-dep; per-file helper fns (module tests commonly wrap `Point3WithWidth`/`WallLoop` construction) converted to FRU/fixture bodies.
- Rejected alternatives and reasons:
  - Giving `OrderedEntityView` a `Default` — rejected; packet-195 class (b) decision (fixture, not `Default`), re-guarded by AC-N1.
  - Un-gating `test_support` (making it unconditionally public) to avoid manifest entries — rejected; ADR-0004's disjoint-surfaces contract and the guest-build hygiene depend on the gate.
  - Waiving instead of gating `layer_module_tdd.rs`/`finalization_builder_tdd.rs` — rejected; exhaustiveness is not those tests' intent, and the gating convention already exists (17 entries).

## Files in Scope (read + edit)

Sweep packet: bounded globs replace the 3-file list; each step edits one surface only.

- `crates/slicer-sdk/tests/**/*.rs` + `crates/slicer-sdk/Cargo.toml` + waiver-only edits in `crates/slicer-sdk/src/test_support/**` - role: sdk sweep + gating; expected change: fixture/FRU conversions, `[[test]]` entries, waivers.
- `modules/core-modules/*/tests/**/*.rs` (bounded glob — only files named in the Step-1 report; measured candidates span 10 of 21 modules) - role: module sweep; expected change: fixture/FRU conversions.

## Read-Only Context

- `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures + base values only - purpose: know what each base supplies so equal fields are omitted.
- `crates/slicer-sdk/Cargo.toml` - `[[test]]` tail only - purpose: gating convention + collision check.
- `xtask/src/build_guests.rs` - `shared_input_paths` body only - purpose: confirm the staleness trigger if disputed.
- `docs/21_data_defaults_and_fixtures.md` - conversion rule + waiver format.

## Out-of-Bounds Files

- `modules/core-modules/*/src/**`, `modules/core-modules/*/wit-guest/**`, `modules/core-modules/*/Cargo.toml` - guest-feeding and out of scope (AC-N4 guards manifests).
- `crates/slicer-sdk/src/**` outside `test_support/**` - production SDK surface.
- `crates/slicer-wasm-host/test-guests/**` - different area, rule-exempt; never load.
- `xtask/src/check_literals.rs` + xtask tests - packet 194 owns; defects are deviations.
- `.ralph/specs/194-*/`, `.ralph/specs/195-*/` except `packet.spec.md` - SUMMARY dispatch only.
- `OrcaSlicerDocumented/` - never load. `target/`, `Cargo.lock`, generated code, built `.wasm` artifacts - never load (except `target/sweep-198-*` scratch).

## Expected Sub-Agent Dispatches

- Question: run Step-1 report + baselines (sdk with `--features test`; module loop over the derived list); return per-area violating files (path + count), the module list, baseline greenness; scope: commands in `requirements.md` matrix; return: `LOCATIONS` ≤20 entries per area + `FACT`.
- Question: after the sdk step, does `check-literals crates/slicer-sdk` exit 0, does the `--features test` suite diff clean vs baseline, and does the bare `cargo test -p slicer-sdk` still pass (AC-7)?; scope: `crates/slicer-sdk`; return: `FACT` PASS/FAIL + ≤5 lines.
- Question: after each module batch, does `check-literals modules/core-modules` (or the batch's module paths) exit 0 and do the batch's `cargo test -p <module>` runs pass?; scope: listed modules; return: `FACT` per module.
- Question: run `cargo xtask build-guests` then `--check`; scope: workspace; return: `FACT` clean/STALE-count only.
- Question: workspace gates (`check`/`clippy` `--all-targets`); scope: workspace; return: `FACT` + first error ≤10 lines.

## Data and Contract Notes

- IR/manifest contracts: untouched; conversions value-identical by construction. Fixture base values are packet-195 contract (`print_entity_base`: `entity_id 0`, 1-point path, `speed_factor 1.0`; `wall_loop_base`: `perimeter_index 0`, widths length-matched to points, role mapping Outer→OuterWall / ThinWall→ThinWall / else InnerWall; `ordered_entity_view_base`: `point_count 2`, default endpoints) — conversions must omit exactly the fields equal to these values, nothing else.
- WIT boundary: untouched; guests never see `feature = "test"`.
- Determinism/scheduler constraints: none.

## Locked Assumptions and Invariants

- Test counts, assert counts, and every constructed value invariant; construction syntax (plus manifest gating) only.
- The six no-`Default` locks hold (AC-N1, including `OrderedEntityView`).
- Module manifests unchanged (AC-N4); sdk manifest gains only `[[test]]` entries.
- All sdk suite runs in this packet use `--features test`; the bare-run check (AC-7) is the sole exception, by design.

## Risks and Tradeoffs

- The gating edit widens the sdk's bare-run blind spot (more binaries skipped without `--features test`). Accepted: the convention and its CLAUDE.md reconciliation rule already exist; exported to packet 199's notes.
- Full-guest rebuild after the manifest edit is the packet's slowest operation; it is the guest-staleness rebuild tax the plan's locked decision 3(b) explicitly accepted. Budget exactly one rebuild (manifest edits complete before it runs).
- Module test helper fns sometimes encode geometry meaning in every field; over-eager omission can obscure intent even when values equal the base. The conversion rule's "meaningful fields stay spelled" clause is the guard; reviewers check spelled-field choices, not just greenness.
- Checker defects at module scale (e.g. macro-heavy module tests) are deviations against packet 194, not local patches.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (module batch A: `seam-placer` + `infill-linker` + `path-optimization-default` + `wipe-tower`, ~17 files by 2026-08-07 sizing)
- Highest-risk dispatch and required return format: Step-1 report enumeration — `LOCATIONS` ≤20 entries per area; never the raw report body.

## Open Questions

- `[FWD]` If the report shows a non-gated sdk test file needing exactly one class-b value, the implementer may choose file-local waivered construction over gating that file; record the choice and reason in close notes (both satisfy AC-1/AC-4 — AC-4 only requires gating for files that reference `test_support`).
- `[FWD]` If any module test constructs a watched HOST type with no `Default`/fixture (report-surfaced), use a file-local waivered base fn (packet-195 pnp-cli twin precedent); never add `Default`.
