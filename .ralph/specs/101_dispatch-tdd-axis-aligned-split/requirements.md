# Requirements: 101_dispatch-tdd-axis-aligned-split

## Packet Metadata

- Grouped task IDs:
  - (none — session-derived; not a `docs/07` backlog slice)
- Backlog source: architecture review (session 2026-06-11, `/improve-codebase-architecture` Candidate 4)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After packet 100 lands, `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` still holds ~99 tests organised across nine orthogonal concerns (per the architecture exploration: export-name mapping, per-runner success paths, error-path coverage, pool correctness, typed-path isolation, full pipeline integration, output commitment for infill/perimeter/support/pathopt, config wiring, identity preservation, prepass harvest of the Global layer plan). The file is the single largest in the workspace (4,875 LOC). Even after packet 100 introduces `DispatchFixture` and `ir_builders`, the file remains shallowly organised — readers must linearly scan ≥ 4,000 lines to find tests for a given Claim, and the `make_compiled_module_*` / `make_slice_ir*` / `make_perimeter_ir*` / `make_wall_loop` / `make_loaded_module` / `make_object` helpers continue to occupy ~250 LOC inside the test file.

This packet relocates each surviving test into one of eight axis-aligned files chosen during the grilling session, migrates each test's setup onto `DispatchFixture` + `ir_builders` as it moves, and deletes the original file (and its helper family) once empty. Observable test contracts — assertions, inputs, `#[ignore]` markers — are preserved by construction. The split makes `failure of the Perimeter output contract` mean "read `dispatch_perimeter_output_tdd.rs`" instead of "read 4,875 lines."

## In Scope

- Create eight new files under `crates/slicer-runtime/tests/contract/`:
  - `dispatch_protocol_tdd.rs` — cross-runner protocol: export-name lookup (`slicer_schema::export_for_stage_id`), pool slot release on success and on Trap, `MissingComponent` graceful skip contract for each of the four runners.
  - `dispatch_config_tdd.rs` — `ConfigView` wiring and isolation: per-module config visibility, default-value behavior, isolation between modules in the same run.
  - `dispatch_infill_output_tdd.rs` — `Layer::Infill` / `Layer::InfillPostProcess` output commitment (sparse/solid/ironing paths, cardinality preservation).
  - `dispatch_perimeter_output_tdd.rs` — `Layer::Perimeters` / `Layer::PerimetersPostProcess` output commitment (wall loops, infill areas, seam candidates).
  - `dispatch_support_output_tdd.rs` — `Layer::Support` / `Layer::SupportPostProcess` output commitment (support/interface/raft paths).
  - `dispatch_pathopt_tdd.rs` — `Layer::PathOptimization` overrides (tool changes, z-hops, retracts, unretracts, deferred travel moves, comments, raw fragments).
  - `dispatch_identity_tdd.rs` — region-identity preservation across dispatch: bucket-by-origin for infill / perimeter / support / slice-postprocess; the `Layer::PerimetersPostProcess` round-trip test migrated by packet 100 belongs here.
  - `dispatch_prepass_harvest_tdd.rs` — Global-layer harvest from prepass (layer-plan harvest test cluster).
- Register the eight new files as `pub mod` entries in `crates/slicer-runtime/tests/contract/main.rs`.
- Migrate each remaining test from `dispatch_tdd.rs` onto `DispatchFixture` + `ir_builders`. Where a test today calls `make_compiled_module_no_wasm(...)` it becomes `DispatchFixture::for_stage(...).no_wasm().build()`; where it calls `make_compiled_module_with(...)` it becomes `DispatchFixture::for_stage(...).with_wat(...).build()` (or the default `.build()` when the default real test guest is the right one); where it constructs `SliceIR` / `PerimeterIR` / `WallLoop` by hand it uses `ir_builders::slice_ir::{with_count|with_ids}` / `perimeter_ir::{with_count|with_ids}` / `wall_loop()`.
- Delete `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` and remove its `pub mod dispatch_tdd;` declaration from `tests/contract/main.rs`.
- Preserve every `#[ignore]` marker, every assertion, every input value, and every observable test contract. AC-3 and AC-N1 enforce this mechanically.

## Out of Scope

- Adding new tests (the migration is a pure file move with setup rewrite; new test ideas wait for a follow-up packet).
- Changing observable test semantics (assertions, input values, expected outputs are preserved).
- Editing the `tests/common/` modules (`DispatchFixture`, `ir_builders`, `TestModuleBundle`, `run_layer_and_commit*`). All are packet 100's surface.
- Editing the parallel fixture set under `crates/slicer-wasm-host/tests/common/` (AC-N2 invariant).
- Touching any `e2e` or `executor` or `integration` test bucket.
- Removing the currently-`#[ignore]`'d paint-region tests (they move with their axis but stay ignored).
- Any host or guest source file outside the test buckets.
- Any extraction of `slicer-test-fixtures` as a shared crate (deferred indefinitely).

## Authoritative Docs

- `docs/adr/0007-compiled-module-static-live-split.md` — 175 lines incl. the 2026-06-11 amendment. Load directly. The amendment's "What future architecture reviews must not re-litigate" bullets are the locked conventions governing every migration.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — 142 lines. Load directly. The four runner traits whose `run_stage` calls survive in the eight new files via `DispatchFixture`'s per-runner methods.
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — ≈ 90 lines. Load directly. Governs `dispatch_protocol_tdd.rs`; the file must use `slicer_schema::export_for_stage_id` exclusively (no hardcoded stage→export tables).
- `CLAUDE.md` §Test Discipline — read lines 51–86 only.

## Acceptance Summary

- Positive cases: AC-1 (compile gate after creation), AC-2 (compile gate after deletion), AC-3 (test count preserved + all pass), AC-4 (no legacy `make_*` calls survive in the new files) — all defined in `packet.spec.md`.
- Negative cases: AC-5 (clippy clean), AC-N1 (`#[ignore]` count preserved), AC-N2 (wasm-host common/ untouched) — defined in `packet.spec.md`.
- Cross-packet impact: packet 100 must be implemented first (`DispatchFixture` and `ir_builders` are imported by every new file).
- Refinement (does not fit Given/When/Then): the legacy helpers `make_loaded_module` (line 179), `make_compiled_module` (line 194), `make_compiled_module_with` (line 198), `make_compiled_module_with_config` (line 206), `make_compiled_module_no_wasm` (line 235), `make_slice_ir` (line 1955), `make_slice_ir_with_ids` (line 1991), `make_wall_loop` (line 2023), `make_perimeter_ir` (line 2060), `make_perimeter_ir_with_ids` (line 2419), `make_object` (line 4851) all cease to exist when `dispatch_tdd.rs` is deleted at step 10. The AC-4 grep enforces zero residual references in the new files.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Proves AC-1 (mid-migration) and AC-2 (post-deletion) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Proves AC-5 | FACT pass/fail; SNIPPETS ≤ 20 lines on first warning |
| `cargo test -p slicer-runtime --test contract 2>&1 \| tee target/test-output.log` | Proves AC-3 (test bucket green; preserved count) | FACT pass/fail; on fail SNIPPETS ≤ 20 lines around the first failed assertion |
| `cargo test -p slicer-runtime --test contract dispatch_protocol_tdd::` | Per-axis verification during Step 2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_config_tdd::` | Per-axis verification during Step 3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_infill_output_tdd::` | Per-axis verification during Step 4 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_perimeter_output_tdd::` | Per-axis verification during Step 5 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_support_output_tdd::` | Per-axis verification during Step 6 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_pathopt_tdd::` | Per-axis verification during Step 7 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_identity_tdd::` | Per-axis verification during Step 8 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract dispatch_prepass_harvest_tdd::` | Per-axis verification during Step 9 | FACT pass/fail |
| `! grep -rE 'make_compiled_module\|make_slice_ir\|make_perimeter_ir\|make_wall_loop\|make_loaded_module\|make_object' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs` | Proves AC-4 | FACT pass/fail (exit 0 = no matches = pass) |
| `test ! -f crates/slicer-runtime/tests/contract/dispatch_tdd.rs` | Confirms the legacy file is gone (part of AC-2) | FACT pass/fail |
| `test -z "$(git status --porcelain crates/slicer-wasm-host/tests/common/)"` | Proves AC-N2 | FACT pass/fail |

All commands above produce small parseable output. None invokes `cargo test --workspace`. The closure ceremony runs the three gate commands listed in `packet.spec.md` plus the AC-4 grep and AC-N2 status check.

## Step Completion Expectations

- Cross-step invariant: no migration step (Steps 2 through 9) may alter assertions, input values, or `#[ignore]` markers from the source test. The migration is a pure setup-rewrite + relocate.
- Cross-step invariant: `cargo check --workspace --all-targets` must pass after every step (not only at the final gate). Each per-axis migration step lands a coherent slice; mid-migration broken builds are forbidden.
- Step ordering rationale: Step 1 (skeletons + main.rs wiring) precedes the migration steps so that subsequent steps can use the new files as `pub mod` siblings without scratch builds. Step 10 (delete `dispatch_tdd.rs`) is last because the previous steps still rely on the legacy file's helpers to bridge any not-yet-migrated tests — although the migration is monotonic per step, the file is only safe to delete once every test has moved.
- Cross-step shared scratch state: none.

## Context Discipline Notes

- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` is 4,875 lines. NEVER load in full. Each migration step uses a tightly-scoped range (≈ 300 lines per axis, often less). The expected sub-agent dispatches in `design.md` enumerate the per-axis line ranges and the LOCATIONS dispatches that build them.
- The eight new files start empty (skeleton plus `use crate::common::*;`) and grow incrementally. Each per-axis migration step writes one file end-to-end before running its verification command.
- Sub-agent return-format hints for the heaviest dispatches:
  - "Run `cargo test -p slicer-runtime --test contract dispatch_<axis>_tdd::`": dispatch as `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the first failing assertion. Never dump the full test bucket log.
  - "Identify the line range and test names that belong to axis `<axis>` in `dispatch_tdd.rs`": return `LOCATIONS` (≤ 30 entries: `file:line — test_fn_name`) — never `SUMMARY` and never `SNIPPETS` of full bodies.
