---
status: implemented
packet: 101_dispatch-tdd-axis-aligned-split
task_ids: []
---

# 101_dispatch-tdd-axis-aligned-split

## Goal

Split `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (4,875 LOC) into eight axis-aligned contract files — protocol, config, infill output, perimeter output, support output, pathopt, identity, prepass harvest — migrating each remaining test onto `DispatchFixture` and `ir_builders` (from packet 100) as it moves; delete `dispatch_tdd.rs` and its `make_*` helper family.

## Problem Statement

After packet 100 lands, `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` still holds ~99 tests organised across nine orthogonal concerns (per the architecture exploration: export-name mapping, per-runner success paths, error-path coverage, pool correctness, typed-path isolation, full pipeline integration, output commitment for infill/perimeter/support/pathopt, config wiring, identity preservation, prepass harvest of the Global layer plan). The file is the single largest in the workspace (4,875 LOC). Even after packet 100 introduces `DispatchFixture` and `ir_builders`, the file remains shallowly organised — readers must linearly scan ≥ 4,000 lines to find tests for a given Claim, and the `make_compiled_module_*` / `make_slice_ir*` / `make_perimeter_ir*` / `make_wall_loop` / `make_loaded_module` / `make_object` helpers continue to occupy ~250 LOC inside the test file.

This packet relocates each surviving test into one of eight axis-aligned files chosen during the grilling session, migrates each test's setup onto `DispatchFixture` + `ir_builders` as it moves, and deletes the original file (and its helper family) once empty. Observable test contracts — assertions, inputs, `#[ignore]` markers — are preserved by construction. The split makes `failure of the Perimeter output contract` mean "read `dispatch_perimeter_output_tdd.rs`" instead of "read 4,875 lines."

## Architecture Constraints

- The eight axis files MUST mirror the trait-and-Claim structure documented in the ADR-0007 amendment: per-runner protocol on one file, per-stage-output IR contracts on four files, per-Claim concerns (config, identity, pathopt, prepass harvest) on three files. The amendment explicitly enumerates this split.
- `dispatch_protocol_tdd.rs` MUST use `slicer_schema::export_for_stage_id` exclusively for export-name lookup. ADR-0006 forbids any hardcoded stage→export table in dispatcher tests; the migration must preserve this behavior and may not reintroduce a parallel table even as a "convenience" inline `const`.
- No file in the new set may instantiate a `WasmRuntimeDispatcher`, `Blackboard`, or `LayerArena` directly. All such state lives inside the `DispatchFixture` per the ADR-0007 amendment's locked invariant.
- The eight files share no scratch state with each other and may run in any order. `cargo test -p slicer-runtime --test contract` parallelises across them.
- The wasm-staleness snippet does NOT apply: this packet edits only files under `crates/slicer-runtime/tests/contract/` and `tests/contract/main.rs`. No path under `wit/`, `slicer-macros/`, `slicer-sdk/`, `slicer-ir/`, `slicer-schema/`, `modules/core-modules/`, or `slicer-runtime/test-guests/` source is touched. Pre-built test-guest `.wasm` artifacts are loaded but not modified.
- The coord-system snippet does NOT apply: every test's polygon and point coordinates are preserved bit-identically across migration. Where `ir_builders` constructs the default 1mm-square polygon it uses the same `Point2 { x: 10_000, y: 10_000 }` values the legacy `make_slice_ir` used.

## Data and Contract Notes

- IR or manifest contracts touched: none. Migrated tests instantiate the same IR shapes via `ir_builders` instead of `make_*`; the IR struct fields, the WIT boundary, and the runner trait inputs/outputs are unchanged.
- WIT boundary considerations: none. `dispatch_protocol_tdd.rs` uses `slicer_schema::export_for_stage_id` (ADR-0006); no dispatcher-side parallel table is introduced.
- Determinism or scheduler constraints: every test that today asserts a deterministic count or ordering continues to do so. The default `ir_builders` synthetic IDs (`obj-0`, `obj-1`, …) match the legacy `make_*` IDs exactly, so identity-sensitive tests in `dispatch_identity_tdd.rs` see bit-identical inputs.

## Locked Assumptions and Invariants

- `crates/slicer-wasm-host/tests/common/` is byte-identical at packet end vs packet start (AC-N2).
- Every migrated test preserves its observable contract: same `assert*!` content, same input values, same `#[ignore]` markers (AC-N1).
- The eight new files use `DispatchFixture` and `ir_builders` exclusively for module / arena / Blackboard / IR setup; no legacy `make_*` call survives in the new files (AC-4).
- `dispatch_tdd.rs` is deleted only after every test it once held has moved; the deletion step is last and is preceded by a successful run of `cargo test -p slicer-runtime --test contract` to confirm zero coverage loss.
- Per the ADR-0007 amendment: per-runner methods only; two distinct constructors per IR type; no generic `run::<R>` method; no type-state fixture parameter. The amendment's locked conventions govern every migrated test.

## Risks and Tradeoffs

- The packet touches 10 files, which is far above the "≤ 3 primary files" template target. Justification: a file split is irreducibly multi-file work. Per-step file count stays at 1 or 2.
- Per-axis test classification (which test belongs to which axis) is the highest-risk decision in the packet. A test misclassified into the wrong file produces an immediate AC-3 / per-axis test failure (the test runs against a fixture it doesn't fit and panics), so the cost of misclassification is bounded but visible. Step 1's LOCATIONS dispatches per axis serve as a sanity check before any migration begins.
- The `make_*` helpers and `dispatch_tdd.rs` are deleted together in Step 10. If a not-yet-migrated test still references a helper at that point, Step 10's `cargo check --workspace --all-targets` fails immediately. This is the intended forcing function — the implementer cannot delete the file until the migration is complete.
- The architecture exploration estimated ≈ 99 tests across the eight axes; the actual count is confirmed at Step 1 via a LOCATIONS dispatch enumerating every `#[test]` and `#[ignore]` line in `dispatch_tdd.rs`. The estimate is allowed to drift modestly (±10 tests) without changing the packet plan; a larger drift would warrant a packet revision.
