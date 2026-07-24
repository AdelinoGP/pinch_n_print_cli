# Implementation Plan: 176-support-preview-verb

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Fixture + config lock (read-only discovery)

- Task IDs: `TASK-291`
- Objective: determine which fixture (`resources/bridge_support_enforcers.3mf` or `resources/bridge.obj` + explicit config) plus which config keys (`enable_support`, support type, etc.) cause prepass to commit a non-empty `SupportGeometryIR`; record the pair as the AC-1/AC-2 test input.
- Precondition: `cargo xtask build-guests --check` clean (rebuild if `STALE:`).
- Postcondition: a written decision (fixture path + exact config map) proven by a dispatched probe showing `support_geometry()` is `Some` with ≥1 non-sentinel entry.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/` — directory listing + the support-geometry module's manifest TOML only
- Files allowed to edit (at most 3):
  - none (decision recorded in the worker log / scratch note)
- Files explicitly out of bounds:
  - module `src/` bodies, `crates/slicer-runtime/src/prepass.rs`
- Expected sub-agent dispatches:
  - Question: with fixture X and config Y, is the committed `SupportGeometryIR` non-empty after `prepare_prepass_context`? (worker may write a 10-line throwaway probe under the scratchpad, not the repo); scope: probe run; return: `FACT` (fixture + keys + entry count)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated grep for support keys
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` — FACT clean/STALE
- Exit condition: FACT names a fixture + config with non-empty geometry; fails if no fixture/config combination yields entries (then escalate: packet premise broken).

### Step 2: Handler + CLI variant

- Task IDs: `TASK-291`
- Objective: create `crates/pnp-cli/src/support_preview.rs` (`run_support_preview`, `build_preview_doc`, serde structs per `design.md`) and wire `Cmd::SupportPreview` in `main.rs`.
- Precondition: Step 1 exit met.
- Postcondition: `cargo run --bin pnp_cli -- support-preview --input <fixture> --output <tmp>.json` (with Step 1's config) exits 0 and writes parseable JSON; `cargo check -p pnp-cli --all-targets` passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — lines `1325-1375`
  - `crates/slicer-runtime/src/run.rs` — lines `694-800`
  - `crates/slicer-runtime/src/blackboard.rs` — lines `255-275`
  - `crates/slicer-ir/src/slice_ir.rs` — lines `60-100`, `990-1010`, `1155-1200`, `1333-1346`
  - `crates/pnp-cli/src/main.rs` — lines `40-130` (Cmd enum) + the Slice dispatch arm
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/support_preview.rs` (new)
  - `crates/pnp-cli/src/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (read-only ranges above; zero edits), `modules/**`
- Expected sub-agent dispatches:
  - Question: `cargo check -p pnp-cli --all-targets` result; scope: workspace; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — units table range only
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check -p pnp-cli --all-targets` — FACT pass/fail
- Exit condition: manual verb run writes valid JSON with `schema_version: "1.0.0"`; fails if any coordinate exceeds plausible mm magnitude (>10⁴ — symptom of missed unit conversion).

### Step 3: TDD test binary (all ACs)

- Task IDs: `TASK-291`
- Objective: create `crates/pnp-cli/tests/support_preview_tdd.rs` with tests `preview_json_schema_and_nonempty_support` (AC-1), `coordinates_are_mm_not_internal_units` (AC-2), `no_gcode_side_effects_exit_zero` (AC-3), `intermediate_sentinel_entries_skipped_and_counted` (AC-4, synthetic `SupportGeometryIR` through `build_preview_doc`), `support_disabled_yields_empty_layers_exit_zero` (AC-N1), `missing_input_errors_without_output` (AC-N2).
- Precondition: Step 2 exit met; `cargo xtask build-guests --check` clean.
- Postcondition: all six tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/slice_progress_events_default_tdd.rs` — harness pattern only
  - `crates/pnp-cli/src/support_preview.rs` (own step output)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/support_preview_tdd.rs` (new; per-file binary — no aggregator registration exists or is needed in `crates/pnp-cli/tests/`)
  - `crates/pnp-cli/src/support_preview.rs` (fixes surfaced by TDD)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/pnp-cli/src/main.rs` (frozen after Step 2 unless a dispatch arm bug is proven)
- Expected sub-agent dispatches:
  - Question: full test-run result; scope: `cargo test -p pnp-cli --all-targets --test support_preview_tdd`; return: `FACT` pass/fail + failing names
- Context cost: `M`
- Authoritative docs:
  - none beyond packet files
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
- Exit condition: 6/6 pass; AC-2 explicitly fails when the ×1e-4 conversion is removed (worker must confirm the test is falsifiable by asserting against the committed IR's `Point2` values, not against constants derived from the JSON itself).

### Step 4: Fork-facing contract doc

- Task IDs: `TASK-291`
- Objective: author `docs/20_support_preview.md` (schema_version 1.0.0, full record shape with an example document, mm-units rule citing 1 unit = 100 nm internals, sentinel-skip rule, sparse-layers rule, determinism guarantee, "no interface split at this stage" statement, approximate-by-design + latency/debounce note) and add the `.claude/doc-index.md` row.
- Precondition: Step 3 exit met (doc describes shipped behavior, not intent).
- Postcondition: AC-5 grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `.claude/doc-index.md` (full — index file)
  - `docs/19_visual_debug.md` — via SUMMARY dispatch only (precedent citation)
- Files allowed to edit (at most 3):
  - `docs/20_support_preview.md` (new)
  - `.claude/doc-index.md`
- Files explicitly out of bounds:
  - all code
- Expected sub-agent dispatches:
  - Question: SUMMARY of docs/19's partial-pipeline precedent section; scope: `docs/19_visual_debug.md`; return: `SUMMARY`
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — units table range only
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'schema_version' docs/20_support_preview.md && rg -q 'interface' docs/20_support_preview.md && rg -q '20_support_preview' .claude/doc-index.md && echo PASS` — FACT PASS/absent
- Exit condition: grep prints PASS; fails if the doc's example JSON disagrees with the serde structs' field names.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | fixture/config discovery, delegated probe |
| Step 2 | M | handler + CLI variant |
| Step 3 | M | 6 tests incl. 2 negative |
| Step 4 | S | contract doc + index row |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
