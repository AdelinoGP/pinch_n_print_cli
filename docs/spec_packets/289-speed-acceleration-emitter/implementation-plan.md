# Implementation Plan: 289-speed-acceleration-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the eleven scalar-global fields + schema guard

- Task IDs: `TASK-000`
- Objective: Declare all eleven keys in `ResolvedConfig` with canonical defaults/bounds (three as float-or-percent over canonical bases) and pin them with a schema guard; prove the struct-literal blast radius is fully owned.
- Precondition: No `ResolvedConfig` field exists for any of the eleven (verified at authoring: zero-occurrence outside `flavor.rs`'s unwired builders, which take a `u32` parameter and read no config; re-verify with the Step-1 dispatch before editing).
- Postcondition: `ResolvedConfig` carries 1× `bool = true` + 7× `f32` bounded scalars + 3× `ResolvedFloatOrPercent` (canonical percent defaults + bases) with CLI ingestion and bounds arms; `speed_p56_accel_emission_tdd::schema_declares_all_eleven_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool/float-or-percent row syntax + one scalar precedent only
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/speed_p56_accel_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — zero twins for the eleven, rule 2)
  - `crates/slicer-gcode/src/flavor.rs` (reuse-only; no form change — P59 keeps canonical names)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `ResolvedConfig { .. }` literal + every test asserting `ResolvedConfig::default()` float/bool values; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `declare_resolved_config!` float/bool/float-or-percent row + CLI arm syntax for one scalar precedent; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/config/host-keys.toml` - `[resolved_config]` section (direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd schema_declares_all_eleven_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (no emission assertion runs yet — AC-2 still unwritten).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (eleven fields + host-keys.toml rows landed).
- Postcondition: `docs/15_config_keys_reference.md` contains all eleven keys; `cargo xtask gen-config-docs --check` is green.
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
  - `rg -q 'travel_acceleration' docs/15_config_keys_reference.md && rg -q 'travel_acceleration' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the selection stage + flavor rendering + bounds

- Task IDs: `TASK-000`
- Objective: Select one accel per print entity via the canonical precedence chain (plus first-layer and percent-base resolution), render print/travel lines through the existing flavor arms with change-dedup and the Klipper decel suffix, and enforce bounds including the master gate; travels follow the separate-travel flavor split.
- Precondition: Step 1b green (schema + docs landed); per-entity loop anchor + travel site + flavor-call spelling located via the Step-2 dispatch before editing; 281's envelope position read from its `packet.spec.md` at implementation time (draft — re-derive, do not freeze).
- Postcondition: `emit_gcode` selects and emits accel lines through `set_acceleration` / `set_travel_acceleration` (no forked forms); AC-2 (default emit), AC-3 (role chain), AC-4 (percent bases), AC-5 (flavor split), AC-6 (decel suffix), AC-N1 (bounds), AC-N2 (master gate) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the stage exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::emit_gcode` per-entity loop + travel emission only
  - `crates/slicer-gcode/src/flavor.rs` - lines 74–120 only (reuse the three arms; do not fork)
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` (`ExtrusionRole`) only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — host-only omitted)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (travel-shape context — do not touch)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam — do not touch)
  - `docs/spec_packets/281-machine-motion-limits-emitter/` (stream-position reference — do not touch)
- Expected sub-agent dispatches:
  - Question: per-entity loop anchor lines + travel-emission site + last-emitted-state spelling at the call site; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: 281's envelope stream position (which lines open the stream, where per-path lines follow); scope: `docs/spec_packets/281-machine-motion-limits-emitter/packet.spec.md`; return: `FACT` (≤5 lines)
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (stage is in-module parameter, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude` chain order + gates + composition position; `GCode::travel_to` default + flavor split - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - `GCodeWriter::set_acceleration_internal` forms + `GCodeWriter::supports_separate_travel_acceleration` set - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Stage compiles clippy-clean; selection is a pure function of (role, layer index, travel-vs-print, cfg) with print/travel dedup keyed separately; no E logic touched; no CONFIG_BLOCK edit; no ordering or travel-path logic touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the default-emit, role-chain, percent-base, flavor-split, decel-suffix, bounds, and master-gate cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)); re-baseline the default-output churn with measured justification (the M204 lines appearing is the stage, not geometry moving).
- Precondition: Step 2 green (stage compiles; helpers callable from tests).
- Postcondition: `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd` is fully green (AC-1–AC-6, AC-N1–AC-N2); each of the eleven keys has at least one non-default behaviour assertion.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signatures + call sites only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/speed_p56_accel_emission_tdd.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (default-churn re-baseline with measured justification only — AC-2's exact lines pin what changed; a third golden breaking sends the packet back for a split, not a fourth edit)
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
  - None (no new canonical read in this step)
- Verification:
  - `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (default-churn re-baseline proof)
- Exit condition: Guard fully green; every key has a non-default assertion; any re-baselined golden cites the AC-2 lines as the measured justification; no production file changed in this step.

### Step 4: Record deviations + annotate the queue ledger

- Task IDs: `TASK-000`
- Objective: File the DEV-181 row (bounds-as-divergence, scalar-not-vector, stateless restore) and annotate the 04 tier table + 05 packet list so P56 reads 11-in at packet 289; prove doc freshness.
- Precondition: Steps 1b–3 green (schema, stage, and pins landed).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-181 with (a)+(b)+(c) clauses; 04's eleven Speed/Acceleration rows point at packet 289; 05's P56 section reads 11 keys in; `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` (if the repo gates it) are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-row format + max-ID region only
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Speed/Acceleration section only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P56 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/` (production frozen — test/docs-only step)
  - `docs/spec_packets/281-machine-motion-limits-emitter/` (never edit another packet)
  - `docs/15_config_keys_reference.md` (regen-only; Step 1b owns it)
- Expected sub-agent dispatches:
  - Question: DEV-row exact format + current max DEV id at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (direct, grep-only)
- OrcaSlicer refs:
  - None (deviation text was grounded in Steps 1–2)
- Verification:
  - `rg -q 'DEV-181' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (final re-dispatch)
- Exit condition: DEV-181 greps; ledger rows read 11-in at 289; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guard; blast-radius dispatch owned here |
| Step 1b | S | Regen only |
| Step 2 | M | Selection stage + flavor rendering + bounds |
| Step 3 | M | Behaviour pins + default-churn re-baseline |
| Step 4 | S | DEV row + ledger annotation |

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
