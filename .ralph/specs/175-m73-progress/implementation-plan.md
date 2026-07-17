# Implementation Plan: 175-m73-progress

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Estimator per-command elapsed-time extension

- Task IDs: `TASK-279`
- Objective: add `estimate_print_with_elapsed(...) -> (PrintEstimate, Vec<f64>)` to 169's estimator (cumulative seconds after each command, `len == commands.len()`); `estimate_print` delegates to it.
- Precondition: dispatched FACT confirms the tree still holds `estimate_print` (`estimator.rs:168`), `PrintEstimate` (:91), `EstimatorLimits` (:24) with the shapes named in `design.md`, and packet 169's spec is marked `implemented` (activation gate). If names/shapes differ, STOP and reconcile the design before editing.
- Postcondition: new fn compiles; 169's existing `--test estimator` tests pass unmodified; new test proves the vector is monotonically non-decreasing and its last element equals `total_time_s`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/estimator.rs` (169's file, full — it is this step's edit target)
  - `crates/slicer-ir/src/slice_ir.rs` — lines `2195-2295`
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/estimator.rs`
  - `crates/slicer-gcode/tests/estimator.rs` (append one elapsed-vector test)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-gcode/src/serialize.rs`, `OrcaSlicerDocumented/`
- Expected sub-agent dispatches:
  - Question: confirm 169's landed export names/signatures; scope: `crates/slicer-gcode/src/estimator.rs`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/169-time-estimator-slice-stats/design.md` — delegated SUMMARY (export list only)
- OrcaSlicer refs:
  - none (physics unchanged)
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test estimator 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
- Exit condition: elapsed-vector test passes AND all pre-existing estimator tests pass unmodified; fails if any 169 test needed edits.

### Step 2: `m73.rs` injection + comment block (TDD)

- Task IDs: `TASK-279`
- Objective: create `crates/slicer-gcode/src/m73.rs` (`inject_m73`, `filament_stats_comment_block`, `format_time_dhms`) and its unit tests covering AC-1, AC-2, AC-3, AC-N2.
- Precondition: Step 1 exit met.
- Postcondition: `cargo test -p slicer-gcode --test m73` green; injection detects `Raw { text: ";LAYER_CHANGE" }` boundaries, dedups unchanged `(pct, min)` pairs, emits start `P0 R<total_min>` and end `P100 R0` with adjacent identical Q/S lines; comment block matches AC-3's exact strings; `[g]` omitted without density.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` — lines `315-345`
  - `crates/slicer-gcode/src/serialize.rs` — lines `700-750` (Raw arm)
  - `crates/slicer-gcode/src/estimator.rs` — signatures only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/m73.rs` (new)
  - `crates/slicer-gcode/src/lib.rs` (add `pub mod m73;` + re-exports)
  - `crates/slicer-gcode/tests/m73.rs` (new; per-file binary, no aggregator registration needed)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-ir/src/resolved_config.rs`
- Expected sub-agent dispatches:
  - Question: Orca dedup + first/last M73 + `get_time_dhms` format details; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - none beyond packet files
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test m73 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
- Exit condition: all four unit ACs pass; fails if any M73 pair appears with mismatched P/R vs Q/S values or a duplicate consecutive `(pct, min)` pair is emitted.

### Step 3: `disable_m73` config key

- Task IDs: `TASK-279`
- Objective: add `cli "disable_m73" disable_m73: bool = false => extract_bool;` to the `ResolvedConfig` macro invocation, plus whatever companion entries the neighboring `support_enabled` bool key has (key-list array, `to_config_map`).
- Precondition: Step 2 exit met.
- Postcondition: `ResolvedConfig::default().disable_m73 == false`; `apply_cli_key("disable_m73", Bool(true))` sets it; workspace type-checks.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` — lines `715-800` and `975-1010`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - everything else
- Expected sub-agent dispatches:
  - Question: after the edit, run `cargo xtask build-guests --check`; scope: workspace; return: `FACT` (clean or STALE list)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated grep only (doc edit happens in Step 5)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; never load
- Verification:
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo xtask build-guests --check` (then rebuild if STALE — `slicer-ir` is a universal guest dep) — FACT
- Exit condition: check passes and guests are fresh; fails if the key round-trips as anything but snake_case `disable_m73`.

### Step 4: Emit-site wiring + e2e tests

- Task IDs: `TASK-279`
- Objective: at the estimator call site inside `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs:757-758`), switch to `estimate_print_with_elapsed`, run `inject_m73(&mut gcode_ir, &elapsed)` when `!self.resolved_config.disable_m73`, append `filament_stats_comment_block(&estimate, self.resolved_config.filament_density)` before `Ok(gcode_ir)`; add `crates/pnp-cli/tests/m73_progress_tdd.rs` covering AC-4 and AC-N1 via `run_slice` on a small fixture (e.g. `resources/20mm_cube.obj`), with config setting `disable_m73` for the negative case.
- Precondition: Steps 1-3 exits met; `cargo xtask build-guests --check` clean.
- Postcondition: fixture G-code contains the AC-4 fragments; disable case contains zero `M73` lines but keeps comments; `postpass.rs` untouched (it only stashes the already-filled IR at :49-51).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` — lines `735-760` only
  - `crates/pnp-cli/tests/slice_progress_events_default_tdd.rs` — harness pattern only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/pnp-cli/tests/m73_progress_tdd.rs` (new; per-file binary, no aggregator)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/postpass.rs`, `run.rs`, `progress_events.rs` (169's surface; postpass is not this packet's seam)
- Expected sub-agent dispatches:
  - Question: is the `emit.rs:757-758` call-site shape unchanged since authoring (estimate → metadata fill → `Ok(gcode_ir)`)?; scope: `crates/slicer-gcode/src/emit.rs` lines 735-760; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - none beyond packet files
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p pnp-cli --test m73_progress_tdd 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
- Exit condition: AC-4 and AC-N1 pass; fails if the disable case leaks any `M73` line or the comment block is missing in either case.

### Step 5: Doc rows

- Task IDs: `TASK-279`
- Objective: add the `disable_m73` row to `docs/15_config_keys_reference.md`; flip the implemented marker (❌→✅) on the `disable_m73` *definition row only* (coBool row, `docs/ORCA_CONFIG_REFERENCE.md:863`).
- Precondition: Step 4 exit met.
- Postcondition: AC-5 grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — nearest-bool-key row only (locate via `rg -n support_enabled`)
  - `docs/ORCA_CONFIG_REFERENCE.md` — the `disable_m73` row only (locate via rg)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md`
  - `docs/ORCA_CONFIG_REFERENCE.md`
- Files explicitly out of bounds:
  - all code
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - the two edited files (ranged)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'disable_m73' docs/15_config_keys_reference.md && ! rg -q '"disable_m73".*coBool.*❌' docs/ORCA_CONFIG_REFERENCE.md && echo PASS` — FACT PASS/absent
- Exit condition: grep prints PASS; fails otherwise. Only the definition row (coBool, line 863) is edited; the category-listing rows (1063/1750) carry no marker and stay untouched.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | estimator extension, delegation refactor |
| Step 2 | M | new module + 4 unit ACs |
| Step 3 | S | one macro line + companions; guest rebuild check |
| Step 4 | M | emit-site wiring + 2 e2e ACs |
| Step 5 | S | doc rows |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected; the activation gate already required 169's spec to be `implemented` before this packet ran, and that status is unchanged by this packet).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
