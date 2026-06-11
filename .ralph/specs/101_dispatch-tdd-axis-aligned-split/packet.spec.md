---
status: draft
packet: 101_dispatch-tdd-axis-aligned-split
task_ids: []
backlog_source: architecture review (session 2026-06-11, /improve-codebase-architecture Candidate 4 — DispatchFixture & suite split)
context_cost_estimate: M
---

# Packet Contract: 101_dispatch-tdd-axis-aligned-split

## Goal

Split `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (4,875 LOC) into eight axis-aligned contract files — protocol, config, infill output, perimeter output, support output, pathopt, identity, prepass harvest — migrating each remaining test onto `DispatchFixture` and `ir_builders` (from packet 100) as it moves; delete `dispatch_tdd.rs` and its `make_*` helper family.

## Scope Boundaries

This packet relocates every surviving test in `dispatch_tdd.rs` (the two proof tests migrated by packet 100 are already on the new fixture surface and move with their containing axis) into eight new files in `crates/slicer-runtime/tests/contract/`, removes the legacy `make_compiled_module_*` / `make_slice_ir*` / `make_perimeter_ir*` / `make_wall_loop` / `make_loaded_module` / `make_object` helpers along with the original file, and registers the new files in `tests/contract/main.rs`. The new files contain no semantic test changes — assertions, fixture inputs, and `#[ignore]` markers are preserved. The parallel fixture set under `crates/slicer-wasm-host/tests/common/` remains untouched.

## Prerequisites and Blockers

- Depends on: packet `100_dispatch-fixture-and-ir-builders` must be `status: implemented`. `DispatchFixture` and `ir_builders` are imported by every new file.
- Unblocks: none registered.
- Activation blockers: packet 100 not yet implemented.

## Acceptance Criteria

- **AC-1. Given** the eight new files `dispatch_protocol_tdd.rs`, `dispatch_config_tdd.rs`, `dispatch_infill_output_tdd.rs`, `dispatch_perimeter_output_tdd.rs`, `dispatch_support_output_tdd.rs`, `dispatch_pathopt_tdd.rs`, `dispatch_identity_tdd.rs`, `dispatch_prepass_harvest_tdd.rs` exist under `crates/slicer-runtime/tests/contract/` and are registered in `tests/contract/main.rs`, **when** `cargo check --workspace --all-targets` is run, **then** it returns exit code 0. | `cargo check --workspace --all-targets`

- **AC-2. Given** `dispatch_tdd.rs` has been deleted from `crates/slicer-runtime/tests/contract/` and removed from `tests/contract/main.rs`, **when** the same `cargo check --workspace --all-targets` is run, **then** it still returns exit code 0 (no orphan imports, no dangling `pub mod dispatch_tdd;` declaration). | `cargo check --workspace --all-targets && test ! -f crates/slicer-runtime/tests/contract/dispatch_tdd.rs`

- **AC-3. Given** all migrations are complete, **when** `cargo test -p slicer-runtime --test contract` is run with output teed to `target/test-output.log`, **then** the bucket reports `^test result: ok\.` with `0 failed`, and the test count equals the pre-packet count of `dispatch_tdd.rs` minus the two tests already living on the new fixture in packet 100 (which are now in `dispatch_perimeter_output_tdd.rs` and `dispatch_protocol_tdd.rs` respectively) — i.e. the absolute test count of `cargo test -p slicer-runtime --test contract` is preserved across this packet. | `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log && rg -q 'test result: ok\. \d+ passed; 0 failed' target/test-output.log`

- **AC-4. Given** every test in the eight new files uses `DispatchFixture` and (where IR is needed) `ir_builders` exclusively, **when** `grep -rE 'make_compiled_module|make_slice_ir|make_perimeter_ir|make_wall_loop|make_loaded_module|make_object' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs` is run, **then** zero matches are returned. | `! grep -rE 'make_compiled_module|make_slice_ir|make_perimeter_ir|make_wall_loop|make_loaded_module|make_object' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs`

- **AC-5. Given** `cargo clippy --workspace --all-targets -- -D warnings` is run, **when** the gate completes, **then** it returns exit code 0 (no warnings introduced by the new files, no unused-import warnings from the deletion). | `cargo clippy --workspace --all-targets -- -D warnings`

## Negative Test Cases

- **AC-N1. Given** Step 1 records the pre-migration `#[ignore]` count from `dispatch_tdd.rs` to `target/dispatch-ignore-baseline.txt`, **when** the migration is complete and `grep -rcE '^\s*#\[ignore\]' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs | awk -F: '{s+=$2} END{print s}'` is compared against the baseline, **then** the two counts are equal (no `#[ignore]` markers were added or removed during the split). | `test "$(cat target/dispatch-ignore-baseline.txt)" = "$(grep -rcE '^\s*#\[ignore\]' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs | awk -F: '{s+=$2} END{print s}')"`

- **AC-N2. Given** the parallel fixture set under `crates/slicer-wasm-host/tests/common/` predates both this packet and packet 100, **when** `git status --porcelain crates/slicer-wasm-host/tests/common/` is run after all packet commits land, **then** the output is empty. | `test -z "$(git status --porcelain crates/slicer-wasm-host/tests/common/)"`

## Verification

Gate commands only — the full matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/adr/0007-compiled-module-static-live-split.md` — load directly (175 lines incl. the 2026-06-11 amendment). The amendment's locked conventions govern every migration.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — load directly (142 lines).
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — load directly (≈ 90 lines). Governs `dispatch_protocol_tdd.rs`'s export-name lookup tests; the file MUST use `slicer_schema::export_for_stage_id` only.
- `CLAUDE.md` §Test Discipline — read lines 51–86 only.

## Doc Impact Statement (Required)

**`none`** — this is a pure file-relocation refactor of test code. No IR field, WIT type, scheduler rule, claim ID, manifest schema, host service, or module SDK contract is touched. Observable test contracts (assertions, inputs, `#[ignore]` markers) are preserved by construction; AC-3 and AC-N1 enforce the preservation mechanically.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
