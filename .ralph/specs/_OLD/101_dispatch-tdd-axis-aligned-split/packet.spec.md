---
status: implemented
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

- **AC-3. Given** all migrations are complete, **when** `cargo test -p slicer-runtime --test contract` is run with output teed to `target/test-output.log`, **then** the bucket reports `^test result: ok\.` with `0 failed`, **and** the summed `#[test]` count across the eight `dispatch_*_tdd.rs` files equals the recorded pre-split baseline of **86** unique tests (the count of distinct `#[test]` functions in the original `dispatch_tdd.rs`, recoverable via `git show f8e574d^:crates/slicer-runtime/tests/contract/dispatch_tdd.rs`). This count-preservation clause is mandatory: the original `0 failed`-only check could not detect dropped tests (it passed while 22 tests were silently lost — see Deviations). | `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log && rg -q 'test result: ok\. \d+ passed; 0 failed' target/test-output.log && test "86" = "$(grep -rcE '^\s*#\[test\]' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs | awk -F: '{s+=$2} END{print s}')"`

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

## Deviations

The first implementation pass (commits `f8e574d` + `43e7226`) was **not** a faithful
pure refactor and was corrected in a remediation pass:

- **22 tests were silently dropped** during the split (negative/rejection, isolation,
  determinism, and the named packet-100 proof test
  `real_perimeter_region_data_visible_through_infill_postprocess_dispatch`). All 22 were
  restored onto `DispatchFixture` + `ir_builders` with assertions and input values
  preserved byte-for-byte across the infill/perimeter/support/pathopt/identity/
  prepass-harvest axes.
- **4 test-name collisions** (same name in two files with divergent bodies) were
  resolved: 3 spurious copies removed from `dispatch_support_output_tdd.rs` (the faithful
  copies live in `dispatch_identity_tdd.rs`), and 1 invented `Layer::Infill` variant of
  `empty_guest_output_does_not_populate_arena` deleted from `dispatch_infill_output_tdd.rs`
  (the original is a `Layer::SupportPostProcess` test, faithfully kept in
  `dispatch_support_output_tdd.rs`).
- The surviving `support_output` copy of `empty_guest_output_does_not_populate_arena`
  was using `.no_wasm()` (missing-component path) instead of the real default guest the
  original used; corrected to the real-guest no-op-output path.
- **AC-3 was strengthened.** The original AC-3 command only asserted `0 failed`, which
  could not detect dropped tests (it passed green while 22 were lost). It now also
  asserts the summed `#[test]` count across `dispatch_*_tdd.rs` equals the recorded
  pre-split baseline of **86**.

Post-remediation: name-level diff vs the original is exactly 86/86/86 (0 missing,
0 extra, 0 duplicate); `cargo test -p slicer-runtime --test contract` = 164 passed /
0 failed; AC-1/AC-2/AC-4/AC-5/AC-N1/AC-N2 all green.

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
