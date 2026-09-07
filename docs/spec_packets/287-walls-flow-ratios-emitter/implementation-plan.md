# Implementation Plan: 287-walls-flow-ratios-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the eight scalar-global fields + schema guard

- Task IDs: `TASK-000`
- Objective: Declare all eight keys in `ResolvedConfig` with canonical defaults/bounds and pin them with a schema guard; prove the struct-literal blast radius is fully owned.
- Precondition: No `ResolvedConfig` field exists for any of the eight (verified at authoring: zero-occurrence; re-verify with the Step-1 dispatch before editing).
- Postcondition: `ResolvedConfig` carries 7× `f32 = 1.0` + 1× `bool = false` with CLI ingestion and bounds arms; `flow_ratio_emission_tdd::schema_declares_all_eight_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool row syntax + one scalar precedent only
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/flow_ratio_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `ResolvedConfig { .. }` literal + every test asserting `ResolvedConfig::default()` float/bool values; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `declare_resolved_config!` float/bool row + CLI arm syntax for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/config/host-keys.toml` - `[resolved_config]` section (direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test flow_ratio_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (defaults identity — AC-2 still unwritten, so no E assertion runs yet).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (eight fields + host-keys.toml rows landed).
- Postcondition: `docs/15_config_keys_reference.md` contains all eight keys; `cargo xtask gen-config-docs --check` is green.
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
  - `rg -q 'outer_wall_flow_ratio' docs/15_config_keys_reference.md && rg -q 'outer_wall_flow_ratio' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the role-gated E multiplier stage + bounds

- Task IDs: `TASK-000`
- Objective: Scale emitted E by the role-selected ratio (+ first-layer modifier) with gate, overhang point-marking selection, and locked-path bypass; enforce `min 0 / max 2` as reject-the-slice.
- Precondition: Step 1b green (schema + docs landed); E-computation anchor + lock-bypass spelling located via the Step-2 dispatch before editing.
- Postcondition: `emit_gcode` multiplies the `flow_factor`-derived E by `resolve_flow_ratio_for(...)`; AC-2 (identity), AC-3 (role+gate), AC-4 (first-layer), AC-5 (overhang), AC-N1 (bounds), AC-N2 (lock bypass) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the stage exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` E computation + `order_lock` handling only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` (`ExtrusionRole`) + lines `2340-2360` (point `flow_factor`/`overhang_quartile`) only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/layer_executor.rs` (returned `is_infill_first` home — do not touch)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (returned detour context — do not touch)
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — host-only omitted)
- Expected sub-agent dispatches:
  - Question: E-computation anchor lines + `order_lock` bypass spelling at the call site; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: layer-0 index source visible at the call site (`global_layer_index` vs local); scope: `crates/slicer-gcode/src/emit.rs`; return: `FACT` (≤5 lines)
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (stage is in-module scaling, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::extrude_entity` mapping order + first-layer exclusion + composition position - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Stage compiles clippy-clean; helper is a pure function of (role, point marking, layer index, cfg) with locked-path bypass at the caller; no CONFIG_BLOCK edit; no ordering or travel logic touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the identity, role/gate, first-layer, overhang, bounds, and lock-bypass cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)).
- Precondition: Step 2 green (stage compiles; helper callable from tests).
- Postcondition: `cargo test -p slicer-gcode --test flow_ratio_emission_tdd` is fully green (AC-1–AC-5, AC-N1–AC-N2); each of the eight keys has at least one non-default behaviour assertion.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signature + call site only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/flow_ratio_emission_tdd.rs`
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
  - `cargo test -p slicer-gcode --test flow_ratio_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS (≤20 lines)
- Exit condition: Full guard binary green; disposition table holds zero declaration-only keys (all eight drive E at non-defaults); default fixture E byte-identical to pre-packet (AC-2).

### Step 4: DEV row, queue annotations, and closure gates

- Task IDs: `TASK-000`
- Objective: Record DEV-179, annotate the returned keys + P55 boundary adjustment in the 04/05 assets, and prove the workspace gates.
- Precondition: Step 3 green (all ACs pass).
- Postcondition: DEV-179 row landed (re-derived collision-free); 04 tier rows for the eight + two returned keys annotated; 05 P54 9→8 + P55 9→8 rows annotated; `check`, `clippy`, guard binary, and `gen-config-docs --check` all green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail rows only (DEV-178 shape model + max-ID re-derivation)
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P54/P55/gate/returned-key rows only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P54/P55 sections only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - Any other packet directory (never modify another packet — P55's shed is an annotation here, not an edit there)
  - `crates/` + `modules/` production code (frozen after Step 2)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Expected sub-agent dispatches:
  - Question: DEV-178 row shape + current `max(DEV-*)` over LOG + `docs/spec_packets/*/`; scope: `docs/DEVIATION_LOG.md` + `docs/spec_packets/2*/`; return: `FACT` (≤5 lines: max ID + next free)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - tail rows (direct, ranged)
- OrcaSlicer refs:
  - None (no new canonical read; DEV text cites the Step-2 reads)
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test flow_ratio_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: All four gates green; DEV-179 greps (`rg -q 'DEV-179' docs/DEVIATION_LOG.md`); 05 greps for the re-sized P54/P55 rows; disposition table lists zero declaration-only keys and every key has a non-default behaviour AC (map gate (a)+(b)).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guard skeleton + blast-radius dispatch |
| Step 1b | S | Generated-docs regen only |
| Step 2 | M | Emitter stage (largest step; still splittable by role arms if it grows) |
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
