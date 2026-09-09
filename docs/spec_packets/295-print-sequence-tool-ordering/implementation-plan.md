# Implementation Plan: 295-print-sequence-tool-ordering

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config surface — three `ResolvedConfig` rows + guards

- Task IDs: none (wayfinder P66 config surface)
- Objective: declare `first_layer_print_sequence` / `other_layers_print_sequence` (`Vec<f64>`, defaults `vec![0.0]`, `extract_float_list` arm) and `other_layers_print_sequence_nums` (`u32`, default `0`, `extract_u32_or_first` arm) in `declare_resolved_config!`, with non-negative validation on list entries rejecting with the key named; host-only omission (no `to_config_map` arm, no manifest row, no padding edit).
- Precondition: `filament_density` (`Vec<f64>`) and `filament_flush_temp` (`u32`) DSL rows exist as mirror arms.
- Postcondition: `ResolvedConfig::default()` carries `[0.0]` / `[0.0]` / `0`; `apply_cli_key` round-trips `1,0` lists and `2` count; negative entries reject naming the key.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `2100-2230`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_print_sequence_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**`, `modules/**`, `crates/slicer-gcode/src/serialize.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New `ResolvedConfig` fields break exhaustive struct literals. Dispatch a `LOCATIONS` worker for struct-literal sites compiling against `ResolvedConfig` before editing; cite the result inline below. Result (pre-authored 2026-09-08): literals are constructed via `Default` + field overrides or the macro-generated paths — no exhaustive literal compiles against all fields outside the macro itself; the `PartialEq`-over-all-fields drift guards + `overlay_onto` macro arms update automatically (ticket-126 precedent). The Step verification re-runs the defaults/overlay guards to prove it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
- Expected sub-agent dispatches:
  - Question: list every struct literal over `ResolvedConfig` outside `resolved_config.rs`; scope: `crates/`; return: `LOCATIONS` ≤20 entries
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (`from_declared` whitelist: why no manifest row)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_print_sequence_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-ir --test resolved_config_defaults_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (drift-guard re-run)
- Exit condition: AC-1 and AC-N2 pipe commands PASS; defaults-guard binary still green.

### Step 2: Ordering kernel + guard fixture + behaviour tests (TDD)

- Task IDs: none (wayfinder P66 ordering kernel)
- Objective: implement pure `sort_layer_tools_by_sequence` (entity-permutation over `PrintEntity.tool_index`; `path.order_lock.is_some()` entities pinned at authored indices; stable partition of unlocked entities, absent-tools-last) and `other_layer_covered` (layer-0 never; `nums > 0 && (idx-1)/len < nums`) as free functions in the emitter module file, plus the new `crates/slicer-gcode/tests/print_sequence_ordering_tdd.rs` fixture (synthetic two-tool layers driving `DefaultGCodeEmitter::with_resolved_config`, incl. one locked run via `path.order_lock = Some(tag)`) with AC-2..6 tests written FIRST and failing.
- Precondition: Step 1 exit holds (fields constructible).
- Postcondition: kernel unit-pinned (stable order, absent-last, lock pinning, coverage math incl. beyond-coverage false, empty-seq false, nums-0 false); guard binary exists with AC-2..6 tests failing on un-wired emission (proving the tests actually drive the stage).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines `64-210`
  - `crates/slicer-gcode/src/emit.rs` - lines `1031-1130`
  - `crates/slicer-gcode/tests/emit_tool_guard_tdd.rs` - first 80 lines (fixture shape to mirror; whole file only if under 600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/print_sequence_ordering_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `modules/**`, `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-runtime/**`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or schema constant added in this step.
- Expected sub-agent dispatches:
  - Question: `LayerCollectionIR.tool_changes` element shape + entity tool-index accessor; scope: `crates/slicer-ir/src/slice_ir.rs`; return: `LOCATIONS` ≤10 entries
  - Question: synthetic-layer fixture constructors in `emit_tool_guard_tdd.rs`; scope: `crates/slicer-gcode/tests/emit_tool_guard_tdd.rs`; return: `SNIPPETS` ≤2 snippets ≤30 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated SUMMARY (emission-stage ownership)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test print_sequence_ordering_tdd 2>&1 | tee target/test-output.log | tail -8` - FACT: all 4 behaviour tests FAIL pre-wire (falsifying proof), kernel unit tests PASS
- Exit condition: kernel functions pinned green; AC-2..6 tests fail for the right reason (baseline order observed where sorted order asserted; locked entities unmoved) — recorded in the step log before Step 3.

### Step 3: Wire the re-sort into `emit_gcode`

- Task IDs: none (wayfinder P66 emission wiring)
- Objective: call the Step 2 kernel from `DefaultGCodeEmitter::emit_gcode` after `apply_cross_layer_tool_rotation`: layer 0 via the first-layer list under the length-gate, layers ≥ 1 via the other-layer list under the covered-gate; recompute through the existing tail; no geometry touched.
- Precondition: Step 2 exit holds (kernel green, tests failing pre-wire).
- Postcondition: AC-2..6 pipe commands PASS; rotation-only tests (`apply_cross_layer_tool_rotation_*`) still green (sequence inert at defaults, authoritative when configured).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines `320-500`
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `modules/**`, `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-gcode/src/estimator.rs`, `crates/slicer-runtime/**`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or schema constant added in this step.
- Expected sub-agent dispatches:
  - Question: none (single call-site edit; no dispatch)
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - Step 2 SUMMARY reused; no new read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test print_sequence_ordering_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-gcode --lib emit 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (rotation regression)
- Exit condition: AC-2, AC-3, AC-4, AC-5, AC-6 pipe commands PASS; lib emit tests green.

### Step 4: Deviation row, asset annotations, generated docs, gates

- Task IDs: none (wayfinder P66 closure)
- Objective: file DEV-187 ((a) float-list shape, (b) cycle-coverage model, (c) single-stack first layer without area base, (d) bounds enforcement) after re-deriving collision-freedom; annotate 04 Quality/Layer-height rows (owner → `crates/slicer-gcode` emission stage) and 05 P66 (3 in at 295); regen `gen-config-docs`; run workspace gates.
- Precondition: Steps 1–3 exits hold.
- Postcondition: `rg -q 'DEV-187' docs/DEVIATION_LOG.md` hits one row with four clauses; 04/05 greps hit; AC-N1 holds; check + clippy + both guard binaries green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail 40 lines only (row shape to mirror)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/**`, `modules/**`, `docs/spec_packets/295-print-sequence-tool-ordering/*.md` (packet text frozen except preflight fixes)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or schema constant added in this step.
- Expected sub-agent dispatches:
  - Question: re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/`; scope: `docs/DEVIATION_LOG.md`, `docs/spec_packets/`; return: `FACT` (max ID + whether DEV-187 collides)
  - Question: run `gen-config-docs` regen + `check-literals` + guest `--check`; scope: repo root; return: `FACT` pass/fail per command
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - regen output only; no direct read
- OrcaSlicer refs:
  - none (no new canonical reads; DEV clauses cite Steps 1–3 groundings)
- Verification:
  - `rg -q 'DEV-187' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'print_sequence' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0` - FACT pass/fail (AC-N1)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all four verification lines PASS; packet ready for `/spec-review --preflight`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | 1 file read range + 2 edits; one LOCATIONS dispatch |
| Step 2 | M | 3 read ranges + kernel + new fixture; two dispatches |
| Step 3 | S | 1 read range + 1 call-site edit; no dispatch |
| Step 4 | S | ledger writes + regen; two FACT dispatches |

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
