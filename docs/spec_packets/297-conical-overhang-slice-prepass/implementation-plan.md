# Implementation Plan: 297-conical-overhang-slice-prepass

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Bind the two canonical decisions (read-only discovery)

- Task IDs: `TASK-000`
- Objective: record the layer-height source and the hole-cut operand order so Steps 2–4 build on locked decisions.
- Precondition: packet files exist; no code changed yet.
- Postcondition: `design.md` §Locked Assumptions names (a) the per-layer height source (named `LayerPlanIR`/slice field or the global-`layer_height` fallback) and (b) the confirmed hole-cut order (cut-then-offset-then-union or the verified alternative), each with its delegation return quoted.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - grep for height/Z fields only (no full load)
  - `crates/slicer-runtime/src/prepass.rs` - lines 820-960
- Files allowed to edit (at most 3):
  - `docs/spec_packets/297-conical-overhang-slice-prepass/design.md` (append the two bindings)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate only)
  - `crates/slicer-gcode/src/serialize.rs` (ticket 132)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: read-only step, no field or constant added.
- Expected sub-agent dispatches:
  - Question: hole-cut operand order + per-region skip rule in `PrintObject::apply_conical_overhang`; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SUMMARY`
  - Question: per-layer height field for `tan(angle) * layer_h`; scope: `crates/slicer-ir/src` + Slice commit in `prepass.rs`; return: `LOCATIONS`
  - Question: portable `tests/fff_print` assertions for conical overhang; scope: `OrcaSlicerDocumented/tests/fff_print/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load
- Verification:
  - `rg -q 'layer-height source:' docs/spec_packets/297-conical-overhang-slice-prepass/design.md && rg -q 'hole-cut order:' docs/spec_packets/297-conical-overhang-slice-prepass/design.md 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: both bindings recorded verbatim in `design.md`, or the documented fallback (global `layer_height`; cut-then-offset-then-union) recorded with the reason the delegation was inconclusive.

### Step 2: Declare the three config fields (TDD)

- Task IDs: `TASK-000`
- Objective: `ResolvedConfig` carries the trio at canonical defaults with round-trip, and the whole-struct drift guard covers them.
- Precondition: Step 1 bindings recorded.
- Postcondition: `apply_cli_key("make_overhang_printable", "true")` (and angle/hole-size equivalents) round-trips; defaults are `false`/`55.0`/`0.0`; drift guard extended; AC-1 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines 76-110, 1830-1850, 2035-2060, 2890-2960
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_conical_overhang_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK spot fix)
  - `docs/15_config_keys_reference.md` (generated)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - `LOCATIONS` (authoring-time re-derivation, packet 296 precedent confirmed 2026-09-09): `ResolvedConfig` is built only through `declare_resolved_config!` (field + `Default` + `PartialEq` + `Hash` arms generated) and `..Default::default()` / `..ResolvedConfig::default()` rest patterns in tests — no exhaustive literal enumerates every field, so the blast radius is the macro arms plus the whole-struct `PartialEq` drift guard (`explicit_overrides_reach_the_composed_config_for_every_declared_field`, `crates/slicer-ir/src/resolved_config.rs`), which this step extends rather than breaks. Re-verify with `rg -U 'ResolvedConfig\s*\{[^}]*\}'` returning no exhaustive literal before editing.
- Expected sub-agent dispatches:
  - Question: any exhaustive `ResolvedConfig { ... }` literal without a `..` rest; scope: `crates/ modules/ xtask/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P71 rows
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (defaults `false`/`55.0`/`0.0`)
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_conical_overhang_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-1 command green; `cargo check -p slicer-ir --all-targets` green (no literal fallout).

### Step 3: Build the kernel + producer + stage registration (TDD)

- Task IDs: `TASK-000`
- Objective: the conical pass mutates per-object footprint pairs per the Step-1 bindings; stage wiring lands in Step 4.
- Precondition: Step 2 green (keys exist); Step-1 bindings locked.
- Postcondition: AC-2/3/4/5 green on the kernel binary; non-finite angle maps to default, never to `tan(NaN)`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines 1-120
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - lines 86-197
  - `crates/slicer-runtime/src/prepass.rs` - lines 820-960
  - `crates/slicer-runtime/src/blackboard.rs` - lines 170-330
  - `crates/slicer-core/src/polygon_ops.rs` - lines 370-560
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/conical_overhang.rs` (new: kernel + `ConicalOverhangParams`)
  - `crates/slicer-core/src/algos/mod.rs` (one `pub mod` line, `#[cfg(feature = "host-algos")]`-gated like its siblings)
  - `crates/slicer-core/tests/conical_overhang_tdd.rs` (new: opens with `#![cfg(feature = "host-algos")]` per the 296 precedent — auto-discovered binary, no `Cargo.toml` entry; AC-2/3/4/5 tests plus disabled-with-angle-set no-op)
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` (no WIT change)
  - `crates/slicer-ir/src/slice_ir.rs` (no schema change, no version bump)
  - `modules/core-modules/*` (no module changes)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: new module + pure function; no existing struct gains a field, no version constant moves.
- Expected sub-agent dispatches:
  - Question: exact `offset`/`union_ex`/`difference_ex`/`intersection_ex` signatures at use time; scope: `crates/slicer-core/src/polygon_ops.rs`; return: `SNIPPETS` (≤3, ≤30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - `mm_to_units` / `UNITS_PER_MM` range
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load (Step-1 order binding)
- Verification:
  - `cargo test -p slicer-core --features host-algos --test conical_overhang_tdd 2>&1 | tee target/test-output.log | tail -8` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-2/3/4/5 commands green; disabled input is byte-identical output; `offset`/`union_ex` are the only geometric primitives used (no new boolean code).

### Step 4: Wire the producer into the prepass order

- Task IDs: `TASK-000`
- Objective: the builtin exists at the right slot with the `MissingSliceIr` guard; behaviour is proven in Step 5.
- Precondition: Steps 2–3 green (keys exist, kernel proven).
- Postcondition: `commit_conical_overhang_builtin` compiles, is registered strictly between the `Slice` and `OverhangAnnotation` `run_builtin_stage` calls with a `slice_ir().is_some()` guard, and returns `MissingSliceIr` (committing nothing) when no `SliceIR` is present.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - lines 86-197
  - `crates/slicer-runtime/src/prepass.rs` - lines 820-960
  - `crates/slicer-runtime/src/blackboard.rs` - lines 170-330
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/conical_overhang_producer.rs` (new)
  - `crates/slicer-runtime/src/builtins/mod.rs` (one `pub mod` line)
  - `crates/slicer-runtime/src/prepass.rs` (one `run_builtin_stage` registration)
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` (no WIT change)
  - `crates/slicer-ir/src/slice_ir.rs` (no schema change, no version bump)
  - `modules/core-modules/*` (no module changes)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: new module + one call site; no existing struct gains a field, no version constant moves.
- Expected sub-agent dispatches: none (sibling pattern is in-context).
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated SUMMARY (only if order disputes arise)
- OrcaSlicer refs: none (no new canonical fact needed in this step).
- Verification:
  - `cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (registration compiles; behaviour proven in Step 5)
- Exit condition: check green; registration text sits textually between the `Slice` and `OverhangAnnotation` calls.

### Step 5: Prove stage order, isolation, and end-to-end reach (TDD)

- Task IDs: `TASK-000`
- Objective: the builtin runs in the right slot, respects object isolation, honours explicit raw-source values, and refuses without slices.
- Precondition: Step 4 green (producer registered).
- Postcondition: AC-6 and AC-N1 green on the executor binary; `main.rs` registration present; no other executor test regresses.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` - full (pattern source; under 600 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/prepass_conical_overhang_stage_order_tdd.rs` (new: order + isolation + raw-source-reach + refuses-without-slices)
  - `crates/slicer-runtime/tests/executor/main.rs` (one `mod` line)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/unit/main.rs` (different bucket; not this packet's home)
  - `target/`, `Cargo.lock`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: test-only additions plus a one-line aggregator registration.
- Expected sub-agent dispatches: none (pattern test is in-context).
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none (no new canonical fact needed in this step).
- Verification:
  - `cargo test -p slicer-runtime --test executor prepass_conical_overhang 2>&1 | tee target/test-output.log | tail -8` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (sibling no-regression)
- Exit condition: AC-6 + AC-N1 green; sibling stage-order suite still green.

### Step 6: Ledger linkage + gates (read-only plus two one-line doc edits)

- Task IDs: `TASK-000`
- Objective: close the map loop for P71 and prove the workspace gates.
- Precondition: Steps 1–5 green.
- Postcondition: tier-table and packet-list rows link P71 → 297; AC-7 green; check + clippy green; no `docs/07` edit (queue packets track in the map assets, not the backlog).
- Files allowed to read, with ranges when over 300 lines: none (targeted greps only).
- Files allowed to edit (at most 3):
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (P71 rows: append packet linkage)
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (P71 entry: append `297-conical-overhang-slice-prepass`)
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (zero new rows; do not touch)
  - `docs/15_config_keys_reference.md` (generated)
  - `docs/07_implementation_status.md` (queue packets do not edit the backlog)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: prose-only ledger edits.
- Expected sub-agent dispatches:
  - Question: run the two gate commands and the AC-7 grep; scope: repo root; return: `FACT`
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'declaration-only keys: 0' docs/spec_packets/297-conical-overhang-slice-prepass/requirements.md 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (AC-7)
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: all three green; ticket 78's resolution comment can cite the packet path.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Delegation-only discovery + 2-line design append |
| Step 2 | S | Macro-arm config + 1 new test file |
| Step 3 | M | Kernel + tests; largest step |
| Step 4 | S | Producer + registration; compiles |
| Step 5 | S | Executor tests + 1 mod line |
| Step 6 | S | Ledger linkage + delegated gates |

Aggregate `M`. No step is `L`; no split required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read. (Queue-packet exception per Step 5: the map asset rows are the update; no `docs/07` row exists for P71.)
- Reconcile reopened/superseded status transitions. (None: no predecessor packet; overlap grep at authoring returned zero hits for `conical`/`make_overhang` under `docs/spec_packets/`.)
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
