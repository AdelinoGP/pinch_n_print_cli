# Implementation Plan: 283-printer-timing-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the four scalar-global fields + mirror + guards scaffold

- Task IDs: `TASK-000`
- Objective: the four P49 keys resolve from raw config with effective canonical default `0.0`; schema guard proves it; no behaviour change yet.
- Precondition: the 4 names are absent from `crates/slicer-ir/src/resolved_config.rs` and `docs/config/host-keys.toml` (dispatched FACT); `declare_resolved_config!` still carries the `cli_opt @printer` machine-key window and the Option-field `to_config_map` loop.
- Postcondition: `ResolvedConfig` carries the 4 `Option<f32> = None` fields (`extract_float_or_first`); `to_config_map` omits them when unset; `host-keys.toml` `[resolved_config]` mirrors them at `0.0`; new test file's schema test passes; estimator and footer untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `2180-2200`
  - `crates/slicer-ir/src/resolved_config.rs` - lines `274-297`
  - `docs/config/host-keys.toml` - lines `48-77`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/estimator.rs` (Step 2)
  - `crates/slicer-gcode/src/m73.rs` (Step 3)
  - `crates/slicer-gcode/src/emit.rs` (Step 3 owns validation)
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` (Step 4 owns the lock arms with the DEV row)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New `ResolvedConfig` fields ripple into every struct literal of that struct. Dispatch a `LOCATIONS` worker for `ResolvedConfig {` literal sites before editing; cite the result inline below and include every site in this step's edit list (split the step if it exceeds 3 edits — prefer one step-per-blast-radius over a wide edit).
  - LOCATIONS result (pre-authored 2026-09-07, re-derive at implementation): worker not yet run — implementer MUST dispatch `LOCATIONS: rg 'ResolvedConfig\s*\{' --type rust` (≤20 entries) and extend the edit list before writing fields.
- Expected sub-agent dispatches:
  - Question: list every `ResolvedConfig {` struct-literal site that must gain the 4 fields; scope: `crates/ modules/ xtask/`; return: `LOCATIONS`
  - Question: what extractor do neighbouring `cli_opt @printer` Option<f32> machine fields use?; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `FACT`
- Context cost: `M` (blast radius unknown until dispatch; split if L)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P49 rows only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test printer_timing_stats_tdd schema_declares_four_keys 2>&1 | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: schema test names all 4 keys with type `Option<f32>`, effective defaults `0.0`, and TOML mirror spellings; `cargo check --workspace --all-targets` green.

### Step 2: Wire the per-toolchange time charge + timing test pins

- Task IDs: `TASK-000`
- Objective: each `ToolChange` charges `load + unload + tool_change` seconds into the estimate total and the elapsed timeline; timing ACs green.
- Precondition: Step 1 exit holds (fields resolve); `estimate_command_deltas`'s `ToolChange` arm still only counts (dispatched FACT).
- Postcondition: one toolchange at 2/3/5 adds exactly 10.0 s to `total_time_s` and shifts every post-change elapsed value by 10.0 s; defaults still identity; pre-existing estimator binary green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/estimator.rs` - lines `228-245` only (command-walk state + limits preamble; the `ToolChange` arm itself is small — read directly)
  - `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs` - full (new file, <600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/estimator.rs`
  - `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 1 owns it)
  - `crates/slicer-gcode/src/m73.rs` (Step 3)
  - `crates/slicer-gcode/src/emit.rs` (Step 3 owns validation)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered (no new field or version constant in this step).
- Expected sub-agent dispatches:
  - Question: did each timing test pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (emitter seam only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` - delegate; never load (`process_filament_change` conditional table quoted in design.md)
- Verification:
  - `cargo test -p slicer-gcode --test printer_timing_stats_tdd toolchange_adds_configured_sum 2>&1 | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test printer_timing_stats_tdd elapsed_shifts_after_toolchange 2>&1 | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test estimator 2>&1 | tail -3` - FACT pass/fail (no regression)
- Exit condition: both timing tests pass with the exact `10.0` deltas; the pre-existing estimator binary green; no non-`ToolChange` command gains seconds.

### Step 3: Wire the cost footer + emit-time validation + remaining pins

- Task IDs: `TASK-000`
- Objective: `time_cost > 0` emits the printer-cost footer line at the canonical value; negatives reject through the real emitter error; cost, negative, and default-identity ACs green.
- Precondition: Steps 1–2 exits hold; `emit_gcode`'s error type admits a stable validation code (dispatched SNIPPETS); `filament_stats_comment_block` still emits only filament + time lines.
- Postcondition: cost line value equals `time_cost * total / 3600` at two decimals and is absent at default; both negative guards reject via the stable code; pre-existing m73/emit binaries green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines `889-922` only (estimate + M73 + footer call sequence)
  - `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs` - full (new file, <600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/m73.rs`
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (Step 1 owns it)
  - `crates/slicer-gcode/src/estimator.rs` (Step 2 owns it)
  - `docs/config/host-keys.toml` (Step 1 owns it)
  - `OrcaSlicerDocumented/...`, `target/`, `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered (no new field or version constant in this step; validation reuses the emitter's existing error type).
- Expected sub-agent dispatches:
  - Question: what stable error code does `emit_gcode` return for config validation, and which existing test asserts one?; scope: `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/tests/`; return: `SNIPPETS` (≤1 snippet, ≤30 lines)
  - Question: did each cost/negative test pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (emitter seam only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load (`update_print_estimated_stats` formula quoted in design.md)
- Verification:
  - `cargo test -p slicer-gcode --test printer_timing_stats_tdd 2>&1 | tail -5` - FACT pass/fail (all 9 tests: schema + default + sum + elapsed + cost value + cost absent + 2 negatives + padding-absence doc check)
  - `cargo test -p slicer-gcode --test m73 2>&1 | tail -3` - FACT pass/fail (no regression)
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tail -3` - FACT pass/fail (no regression)
- Exit condition: all 9 new tests pass; both pre-existing binaries green; validation tests assert the stable code through the real `emit_gcode` error (not a helper-level string match).

### Step 4: Docs, deviation, regen, gates

- Task IDs: `TASK-000`
- Objective: DEV-175 row lands collision-free; lock-test arms land with the mirror; `gen-config-docs` regen threads the keys; padding absence pinned; full gates green.
- Precondition: Steps 1–3 exits hold; DEV-175 absent from both `docs/DEVIATION_LOG.md` and every `docs/spec_packets/*/` (re-derive `max(DEV-*)` over both at implementation — drafts 276/277/281/282 already propose DEV-171/172/173/174).
- Postcondition: DEV-175 row present with clauses (a)(b)(c)(d); `docs/15_config_keys_reference.md` contains the 4 keys at effective `0.0`; `ORCA_CONFIG_PADDING` untouched; lock test green; clippy + literals + check green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - lines `last 25 only`
  - `docs/config/host-keys.toml` - `[resolved_config]` tail only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
- Files explicitly out of bounds:
  - `crates/...`, `modules/...` (Steps 1–3 own them)
  - `crates/slicer-gcode/src/serialize.rs` (AC-N3 forbids touching it)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered.
- Expected sub-agent dispatches:
  - Question: re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` — is DEV-175 still free, else next free?; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/`; return: `FACT`
  - Question: did each gate pass?; scope: none (run only); return: `FACT` or `SNIPPETS` on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - Notes §Authoring rules 1–6 (zero declaration-only keys; every key drives a behaviour change at non-default value)
- OrcaSlicer refs:
  - None (no new canonical reads in this step).
- Verification:
  - `rg -q 'DEV-175' docs/DEVIATION_LOG.md && rg -q 'machine_tool_change_time' docs/15_config_keys_reference.md && rg -q 'machine_tool_change_time' docs/config/host-keys.toml` - FACT pass/fail (use `rg -q 'crates\/slicer-gcode\/src\/estimator\.rs::estimate_command_deltas' crates/slicer-gcode/src/estimator.rs || rg -q 'estimate_command_deltas' crates/slicer-gcode/src/estimator.rs || rg -q 'fn estimate_command_deltas' crates/slicer-gcode/src/estimator.rs for any symbol-path static check in this packet)
  - `cargo test -p slicer-runtime --test unit host_keys_doc_lock 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
- Exit condition: the triple-`rg` conjunction passes; lock test + `check-literals` + clippy green; ticket-56 answer records the packet dir + preflight verdict (written by the wayfinder session, not the swarm).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | field declarations + map-loop extension + TOML mirror + blast-radius dispatch |
| Step 2 | S | per-toolchange charge + 2 timing tests + 1 no-regression binary |
| Step 3 | M | footer line + validation + 5 remaining tests + 2 no-regression binaries |
| Step 4 | S | DEV row + lock arms + regen + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
