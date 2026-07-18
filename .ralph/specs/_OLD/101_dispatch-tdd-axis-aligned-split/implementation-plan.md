# Implementation Plan: 101_dispatch-tdd-axis-aligned-split

## Execution Rules

- One atomic step at a time.
- This packet is session-derived; no `docs/07` `TASK-###` ids apply. The "Task IDs" field in each step references the packet itself.
- Steps 2 through 9 are file-creation + per-axis test migrations. Each step writes one axis file end-to-end and runs that file's narrow test command before exiting.
- `cargo check --workspace --all-targets` must pass after every step (not only at the final gate). Mid-migration broken builds are forbidden.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Setup — skeletons, main.rs wiring, axis enumeration, baseline record

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: create the eight empty skeleton files; register each as `pub mod` in `tests/contract/main.rs`; dispatch eight `LOCATIONS` queries (one per axis) to enumerate which tests belong to which axis; record the baseline `#[ignore]` count to `target/dispatch-ignore-baseline.txt` so AC-N1 can compare it later.
- Precondition: packet 100 is `status: implemented`. `crates/slicer-runtime/tests/common/dispatch_fixture.rs` and `ir_builders.rs` exist.
- Postcondition: eight new skeleton files exist (each with a doc-comment header naming its axis and a `use crate::common::*;` line); `tests/contract/main.rs` declares all eight as `pub mod`; the implementer's working notes record per-axis test inventories (≤ 30 entries each); `target/dispatch-ignore-baseline.txt` exists and contains an integer count.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/contract/main.rs` — full file
  - `crates/slicer-runtime/tests/common/dispatch_fixture.rs` — full file (created by packet 100)
  - `crates/slicer-runtime/tests/common/ir_builders.rs` — full file (created by packet 100)
- Files allowed to edit — this step touches 10 files. The deviation from the ≤ 3-file target is justified per the design.md template clause "If more than 3 are unavoidable, justify each one": each new skeleton is ≈ 5 LOC of mechanical scaffolding, `main.rs` gets exactly eight added `pub mod` lines, and the baseline file is a single integer under git-ignored `target/`. Restructuring this into nine separate steps would multiply the LOCATIONS-dispatch overhead nine-fold without reducing the work per step. The atomic alternative — fold this setup into Step 2 — would put Step 2 above `S` cost. Setup-step deviation is documented and bounded.
  - `crates/slicer-runtime/tests/contract/main.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_config_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_perimeter_output_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_support_output_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_identity_tdd.rs` (NEW)
  - `crates/slicer-runtime/tests/contract/dispatch_prepass_harvest_tdd.rs` (NEW)
  - `target/dispatch-ignore-baseline.txt` (NEW; git-ignored, not in source tree)
- Files explicitly out-of-bounds for this step:
  - `dispatch_tdd.rs` body (only the section-header comments are needed for the LOCATIONS dispatches; the dispatches themselves do the file read)
  - Any path under `crates/slicer-wasm-host/`
- Expected sub-agent dispatches (run each as a separate dispatch with the narrow return format):
  - "In `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`, return `LOCATIONS` of every test belonging to the **Protocol** axis (export-name lookup, per-runner success/error/pool, MissingComponent contract). Format: `<line>: <test_fn_name>`. ≤ 30 entries."
  - (Repeat the dispatch with axis name replaced for each of the other seven axes: Config, InfillOutput, PerimeterOutput, SupportOutput, PathOpt, Identity, PrepassHarvest.)
  - "Return `FACT`: the count of lines in `dispatch_tdd.rs` matching `^\\s*#\\[ignore\\]`. Then write that integer to `target/dispatch-ignore-baseline.txt`."
- Context cost: `S` (skeletons are mechanical; the LOCATIONS dispatches do the heavy lifting and their return format is bounded).
- Authoritative docs:
  - `docs/adr/0007-compiled-module-static-live-split.md` — read lines 113+ (the amendment) to confirm the axis names.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets` — dispatch as `FACT pass/fail` (eight empty modules must compile).
  - `test -s target/dispatch-ignore-baseline.txt && grep -qE '^[0-9]+$' target/dispatch-ignore-baseline.txt` — dispatch as `FACT pass/fail` (baseline file present, single integer).
- Exit condition: eight skeletons compile under `cargo check`; eight LOCATIONS returns captured; baseline file valid.

### Step 2: Migrate Protocol axis → `dispatch_protocol_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_protocol_tdd.rs` with every test from the Protocol axis enumerated in Step 1, each migrated onto `DispatchFixture`. Tests that previously called `slicer_runtime::dispatch::export_name_for_stage` use `slicer_schema::export_for_stage_id` per ADR-0006. The `missing_component_gracefully_skipped` proof test migrated in packet 100 belongs in this file.
- Precondition: Step 1 exit condition met. Per-axis test inventory recorded.
- Postcondition: `dispatch_protocol_tdd.rs` is populated; every Protocol-axis test passes; no test from this axis remains in `dispatch_tdd.rs`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — the line ranges identified in Step 1's Protocol LOCATIONS dispatch. Read each test body via a per-test SNIPPETS dispatch when the body is needed; do NOT bulk-read the file.
  - `docs/adr/0006-export-for-stage-id-sole-lookup.md` — full ≈ 90 lines
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (remove the migrated tests; keep the file present for later steps)
- Files explicitly out-of-bounds for this step:
  - All `dispatch_tdd.rs` line ranges NOT in the Protocol inventory
  - Any path under `crates/slicer-wasm-host/`
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` ≤ 50 lines of test `<test_fn_name>` from `dispatch_tdd.rs:<line>`; one test per dispatch." — repeat for each Protocol test.
  - "Run `cargo check --workspace --all-targets`; return `FACT pass/fail`." — after writing the new file.
  - "Run `cargo test -p slicer-runtime --test contract dispatch_protocol_tdd::`; return `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the first failing assertion."
- Context cost: `S` (Protocol axis is small, ≈ 10 tests).
- Authoritative docs:
  - `docs/adr/0006-export-for-stage-id-sole-lookup.md` — governs the `export_for_stage_id` usage in this file.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract dispatch_protocol_tdd::` — `FACT pass/fail`.
- Exit condition: every Protocol test passes in the new file; `cargo check --workspace --all-targets` passes; the migrated tests are gone from `dispatch_tdd.rs`.

### Step 3: Migrate Config axis → `dispatch_config_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_config_tdd.rs` with every Config-wiring test from the inventory.
- Precondition: Step 2 exit condition met.
- Postcondition: every Config-axis test passes in the new file; the tests are gone from `dispatch_tdd.rs`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — the line ranges identified in Step 1's Config LOCATIONS dispatch.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/dispatch_config_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`
- Files explicitly out-of-bounds for this step: as Step 2.
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` ≤ 50 lines of test `<test_fn_name>` from `dispatch_tdd.rs:<line>`; one test per dispatch." — repeat per Config test.
  - "Run `cargo test -p slicer-runtime --test contract dispatch_config_tdd::`; return `FACT pass/fail`."
- Context cost: `S` (Config axis is small, ≈ 5 tests).
- Authoritative docs: `docs/adr/0007-compiled-module-static-live-split.md` (amendment).
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_config_tdd::` — `FACT pass/fail`.
- Exit condition: Config tests pass in the new file; `cargo check --workspace --all-targets` passes.

### Step 4: Migrate Infill Output axis → `dispatch_infill_output_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_infill_output_tdd.rs` with every `Layer::Infill` / `Layer::InfillPostProcess` output-commitment test from the inventory.
- Precondition: Step 3 exit condition met.
- Postcondition: every Infill-output test passes in the new file; gone from `dispatch_tdd.rs`.
- Files allowed to read:
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — the line ranges identified in Step 1's InfillOutput LOCATIONS dispatch
- Files allowed to edit (≤ 3): `dispatch_infill_output_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds for this step: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS` dispatches; per-axis `cargo test` verification.
- Context cost: `M` (≈ 20 tests; each exercises `ir_builders::slice_ir` and the `convert_infill_output` round-trip).
- Authoritative docs: `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` lines 42–77.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_infill_output_tdd::` — `FACT pass/fail`.
- Exit condition: all Infill-output tests pass; check is green.

### Step 5: Migrate Perimeter Output axis → `dispatch_perimeter_output_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_perimeter_output_tdd.rs` with every `Layer::Perimeters` / `Layer::PerimetersPostProcess` output-commitment test.
- Precondition: Step 4 exit condition met.
- Postcondition: tests pass in the new file; gone from `dispatch_tdd.rs`.
- Files allowed to read: `dispatch_tdd.rs` Perimeter line ranges from Step 1.
- Files allowed to edit (≤ 3): `dispatch_perimeter_output_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS`; per-axis `cargo test`.
- Context cost: `M` (≈ 15 tests; some exercise `ir_builders::wall_loop()` escape hatch for seam-candidate shape tests).
- Authoritative docs: `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` lines 42–77.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_perimeter_output_tdd::` — `FACT pass/fail`.
- Exit condition: all Perimeter-output tests pass.

### Step 6: Migrate Support Output axis → `dispatch_support_output_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_support_output_tdd.rs` with every `Layer::Support` / `Layer::SupportPostProcess` output-commitment test.
- Precondition: Step 5 exit condition met.
- Postcondition: tests pass; gone from `dispatch_tdd.rs`.
- Files allowed to read: `dispatch_tdd.rs` Support line ranges from Step 1.
- Files allowed to edit (≤ 3): `dispatch_support_output_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS`; per-axis `cargo test`.
- Context cost: `S` (≈ 5 tests).
- Authoritative docs: `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` lines 42–77.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_support_output_tdd::` — `FACT pass/fail`.
- Exit condition: all Support-output tests pass.

### Step 7: Migrate PathOpt axis → `dispatch_pathopt_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_pathopt_tdd.rs` with every `Layer::PathOptimization` override test (tool changes, z-hops, retracts, unretracts, deferred travel moves, comments, raw fragments).
- Precondition: Step 6 exit condition met.
- Postcondition: tests pass; gone from `dispatch_tdd.rs`.
- Files allowed to read: `dispatch_tdd.rs` PathOpt line ranges from Step 1.
- Files allowed to edit (≤ 3): `dispatch_pathopt_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS`; per-axis `cargo test`.
- Context cost: `M` (≈ 25 tests — the largest cluster; care needed because `GcodeCommandCollected` has many variants and each is exercised).
- Authoritative docs: `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` lines 42–77.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_pathopt_tdd::` — `FACT pass/fail`.
- Exit condition: all PathOpt tests pass.

### Step 8: Migrate Identity axis → `dispatch_identity_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_identity_tdd.rs` with every region-identity-preservation test (perimeter-region wiring, bucket-by-origin, slice-postprocess identity). The `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` proof test migrated in packet 100 belongs here.
- Precondition: Step 7 exit condition met.
- Postcondition: tests pass; gone from `dispatch_tdd.rs`.
- Files allowed to read: `dispatch_tdd.rs` Identity line ranges from Step 1.
- Files allowed to edit (≤ 3): `dispatch_identity_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS`; per-axis `cargo test`.
- Context cost: `M` (≈ 15 tests; most use real dispatch with `ir_builders::slice_ir::with_ids` for explicit identity).
- Authoritative docs: `docs/adr/0007-compiled-module-static-live-split.md` amendment (the `with_ids` constructor).
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_identity_tdd::` — `FACT pass/fail`.
- Exit condition: all Identity tests pass.

### Step 9: Migrate Prepass Harvest axis → `dispatch_prepass_harvest_tdd.rs`

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: populate `dispatch_prepass_harvest_tdd.rs` with every Global-layer harvest test (layer-plan extraction from prepass).
- Precondition: Step 8 exit condition met.
- Postcondition: tests pass; gone from `dispatch_tdd.rs`.
- Files allowed to read: `dispatch_tdd.rs` PrepassHarvest line ranges from Step 1.
- Files allowed to edit (≤ 3): `dispatch_prepass_harvest_tdd.rs`, `dispatch_tdd.rs`.
- Files explicitly out-of-bounds: as Step 2.
- Expected sub-agent dispatches: per-test `SNIPPETS`; per-axis `cargo test`.
- Context cost: `S` (≈ 5 tests).
- Authoritative docs: `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` lines 42–77 (the Prepass runner trait shape).
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test contract dispatch_prepass_harvest_tdd::` — `FACT pass/fail`.
- Exit condition: all Prepass-harvest tests pass.

### Step 10: Delete `dispatch_tdd.rs` + final gate

- Task IDs:
  - (packet-scope: 101_dispatch-tdd-axis-aligned-split)
- Objective: delete `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`, remove its `pub mod dispatch_tdd;` declaration from `tests/contract/main.rs`, and run the packet-level gate commands. The `make_*` helper family (`make_loaded_module`, `make_compiled_module*`, `make_slice_ir*`, `make_perimeter_ir*`, `make_wall_loop`, `make_object`) goes with the file.
- Precondition: Steps 2–9 exit conditions all met. `dispatch_tdd.rs` should contain only the legacy `make_*` helpers and any non-test top-level items, with zero `#[test]` functions remaining.
- Postcondition: `dispatch_tdd.rs` is gone; `main.rs` no longer references it; all gate commands pass; AC-N1's `#[ignore]` count matches the Step 1 baseline; AC-N2's wasm-host directory status is clean.
- Files allowed to read:
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — final state, only to confirm zero `#[test]` remains (read via a single `FACT` dispatch counting `#[test]` lines; do not load).
  - `crates/slicer-runtime/tests/contract/main.rs` — small file.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/main.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (deletion)
- Files explicitly out-of-bounds for this step: all eight new axis files.
- Expected sub-agent dispatches:
  - "Return `FACT`: count of `#[test]` annotations in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`. Pass = 0."
  - "Run `cargo check --workspace --all-targets`; return `FACT pass/fail`."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the first warning."
  - "Run `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log`; return `FACT pass/fail` plus the line matching `^test result: ok\\.`."
  - "Run `! grep -rE 'make_compiled_module|make_slice_ir|make_perimeter_ir|make_wall_loop|make_loaded_module|make_object' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs`; return `FACT pass/fail`. Pass = no matches."
  - "Return `FACT`: count of `^\\s*#\\[ignore\\]` lines across `crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs`. Assert equals the Step 1 baseline."
  - "Run `test -z \"$(git status --porcelain crates/slicer-wasm-host/tests/common/)\"`; return `FACT pass/fail`."
- Context cost: `S` (pure deletion + verification).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log`
  - The AC-4, AC-N1, AC-N2 commands above.
- Exit condition: all gate commands return exit 0; AC-N1 count match; AC-N2 status clean.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | skeletons + 8 LOCATIONS dispatches; no test bodies read |
| Step 2 | S | Protocol axis (~10 tests) |
| Step 3 | S | Config axis (~5 tests) |
| Step 4 | M | Infill output (~20 tests) |
| Step 5 | M | Perimeter output (~15 tests) |
| Step 6 | S | Support output (~5 tests) |
| Step 7 | M | PathOpt (~25 tests; many `GcodeCommandCollected` variants) |
| Step 8 | M | Identity (~15 real-dispatch tests) |
| Step 9 | S | Prepass harvest (~5 tests) |
| Step 10 | S | delete + verify |

Aggregate: `M` (6 S + 4 M). No step is L.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green: AC-1, AC-2, AC-3, AC-4, AC-5 and AC-N1, AC-N2 verification commands returned `pass`.
- `docs/07_implementation_status.md` not modified (this packet is session-derived).
- No prior packet status to reconcile.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm `FACT pass`.
- Confirm the three packet-level gate commands are green.
- Record any packet-local risk explicitly. None expected.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
