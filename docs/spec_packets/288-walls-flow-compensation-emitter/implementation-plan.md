# Implementation Plan: 288-walls-flow-compensation-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the seven scalar-global fields + schema guard

- Task IDs: `TASK-000`
- Objective: Declare all seven keys in `ResolvedConfig` with canonical defaults/bounds and pin them with a schema guard; prove the struct-literal blast radius is fully owned. `set_other_flow_ratios` must not appear in the diff (draft-287 FORWARD-DEP — referenced, never redeclared).
- Precondition: No `ResolvedConfig` field exists for any of the seven (verified at authoring: zero-occurrence outside `serialize.rs`'s `reduce_crossing_wall` padding twin, which is not evidence per rule 2; re-verify with the Step-1 dispatch before editing).
- Postcondition: `ResolvedConfig` carries 5× `f32` (four min-0 + print min-0.01) + 1× `bool = false` + 1× model string (canonical ten-pair default) with CLI ingestion and bounds arms; `walls_p55_flow_emission_tdd::schema_declares_all_seven_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool/string row syntax + one scalar precedent only
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/walls_p55_flow_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — including `reduce_crossing_wall`'s twin)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam)
  - `docs/spec_packets/287-walls-flow-ratios-emitter/` (gate producer — reference only, never edit)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `ResolvedConfig { .. }` literal + every test asserting `ResolvedConfig::default()` float/bool/string values; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `declare_resolved_config!` float/bool/string row + CLI arm syntax for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/config/host-keys.toml` - `[resolved_config]` section (direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd schema_declares_all_seven_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; `set_other_flow_ratios` appears nowhere in the Step-1 diff; no other test file changed behaviour (defaults identity — AC-2 still unwritten, so no E assertion runs yet).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (seven fields + host-keys.toml rows landed).
- Postcondition: `docs/15_config_keys_reference.md` contains all seven keys; `cargo xtask gen-config-docs --check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/gen_config_docs.rs` - invocation + `--check` gate spelling only (delegate FACT first)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated only, via the xtask — never hand-edited)
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (Step 4)
  - Hand edits to the generated reference (forbidden — regen only)
- Expected sub-agent dispatches:
  - Question: `gen-config-docs` invocation + `--check` spelling; scope: `xtask/src/gen_config_docs.rs`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated output (direct, grep-only)
- OrcaSlicer refs:
  - None (no canonical read in this step)
- Verification:
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `rg -q 'support_flow_ratio' docs/15_config_keys_reference.md && rg -q 'support_flow_ratio' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the role-gated multiplier + small-area compensator + bounds

- Task IDs: `TASK-000`
- Objective: Scale emitted E by the role-selected ratio (global print first, then the gated/unconditional role arm) and by the line-length model on solid roles with compensation enabled; enforce bounds including strict model-parse rejection; bypass locked paths for both products.
- Precondition: Step 1b green (schema + docs landed); E-computation anchor + lock-bypass spelling located via the Step-2 dispatch before editing; draft-287 gate field name/shape confirmed from its `packet.spec.md` (same `ResolvedConfig`, no new threading).
- Postcondition: `emit_gcode` multiplies the `flow_factor`-derived E by `resolve_p55_flow_ratio_for(...)` and, on solid roles with compensation on, by `small_area_factor(distance, model)`; AC-2 (identity), AC-3 (role+gate), AC-4 (small-area), AC-5 (global), AC-N1 (bounds), AC-N2 (lock bypass) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the stage exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` E computation + `order_lock` handling only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` (`ExtrusionRole`) only
  - `crates/slicer-gcode/src/emit.rs` - lines `594-660` only (per-point `distance` source for the compensator)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/path-optimization-default/src/lib.rs` (returned `reduce_crossing_wall` context — do not touch)
  - `crates/slicer-runtime/src/layer_executor.rs` (ordering seam — do not touch)
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — host-only omitted)
  - `docs/spec_packets/287-walls-flow-ratios-emitter/` (gate producer — do not touch)
- Expected sub-agent dispatches:
  - Question: E-computation anchor lines + `order_lock` bypass spelling at the call site; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: per-point `distance` binding name visible at the call site (reuse, do not recompute); scope: `crates/slicer-gcode/src/emit.rs`; return: `FACT` (≤5 lines)
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (stage is in-module scaling, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::extrude_entity` mapping order + gate split + composition position; `GCode::_needSAFC` role set + per-segment position - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Stage compiles clippy-clean; multiplier is a pure function of (role, cfg, gate) and compensator of (distance, role, model, bool) with locked-path bypass at the caller; no CONFIG_BLOCK edit; no ordering or travel logic touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the identity, role/gate, small-area, global, bounds, and lock-bypass cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)).
- Precondition: Step 2 green (stage compiles; helpers callable from tests; draft-287 gate readable in tests for AC-3's gate-`true` arm — otherwise pin the gate-`false` arms first and hold the `true` arms behind the 287 FORWARD-DEP).
- Postcondition: `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd` is fully green (AC-1–AC-5, AC-N1–AC-N2); each of the seven keys has at least one non-default behaviour assertion.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signatures + call sites only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/walls_p55_flow_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (schema frozen after Step 1)
  - Production emission logic (frozen after Step 2 — test-only step; a red that needs production change sends the packet back to Step 2)
  - `docs/` (Step 4 owns docs)
- Expected sub-agent dispatches:
  - None (test-only step; no new authority needed — all semantics pinned in Steps 1–2).
- Context cost: `M` (split an L step)
- Authoritative docs:
  - None new (all authority consumed in Steps 1–2)
- OrcaSlicer refs:
  - None new (mapping already borrowed in Step 2)
- Verification:
  - `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS (≤20 lines)
- Exit condition: Full guard binary green; disposition table holds zero declaration-only keys (all seven drive E at non-defaults); default fixture E byte-identical to pre-packet (AC-2).

### Step 4: DEV row, queue annotations, and closure gates

- Task IDs: `TASK-000`
- Objective: Record DEV-180, annotate the returned key + P55 boundary in the 04/05 assets, and prove the workspace gates.
- Precondition: Step 3 green (all ACs pass).
- Postcondition: DEV-180 row landed (re-derived collision-free); 04 tier rows for the seven + returned key annotated; 05 P55 9→7+1 row annotated (gate shed already recorded by 287's Step 4 — reference it, do not re-edit 287); `check`, `clippy`, guard binary, and `gen-config-docs --check` all green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail rows only (DEV-171 shape model + max-ID re-derivation)
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P55/returned-key rows only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P55 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - Any other packet directory (never modify another packet — 287's shed is an annotation reference here, not an edit there)
  - `crates/` + `modules/` production code (frozen after Step 2)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Expected sub-agent dispatches:
  - Question: DEV-171 row shape + current `max(DEV-*)` over LOG + `docs/spec_packets/*/`; scope: `docs/DEVIATION_LOG.md` + `docs/spec_packets/2*/`; return: `FACT` (≤5 lines: max ID + next free)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - tail rows (direct, ranged)
- OrcaSlicer refs:
  - None (no new canonical read; DEV text cites the Step-2 reads)
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: All four gates green; DEV-180 greps (`rg -q 'DEV-180' docs/DEVIATION_LOG.md`); 05 greps for the re-sized P55 row; disposition table lists zero declaration-only keys and every key has a non-default behaviour AC (map gate (a)+(b)).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guard skeleton + blast-radius dispatch |
| Step 1b | S | Generated-docs regen only |
| Step 2 | M | Emitter stage + model parser (largest step; still splittable by ratio arms vs compensator if it grows) |
| Step 3 | M | Behaviour pins (test-only) |
| Step 4 | S | DEV + queue annotations + gates |

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
