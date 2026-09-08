# Implementation Plan: 290-speed-advanced-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the three scalar-global fields + schema guard

- Task IDs: `TASK-000`
- Objective: Declare all three keys in `ResolvedConfig` with canonical defaults/bounds and pin them with a schema guard; prove the struct-literal blast radius is fully owned.
- Precondition: No `ResolvedConfig` field exists for any of the three (verified at authoring: zero-occurrence across `crates/` `modules/` `xtask/` outside the ticket/asset/map mentions — re-verify with the Step-1 dispatch before editing).
- Postcondition: `ResolvedConfig` carries 1× `bool = false` + 2× `f32` bounded scalars with CLI ingestion and bounds arms; `speed_p57_ers_emission_tdd::schema_declares_all_three_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering the `declare_resolved_config!` float/bool row syntax + one scalar precedent only
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/speed_p57_ers_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — zero twins for the three, rule 2)
  - `crates/slicer-gcode/src/estimator.rs` (no change — it times whatever the stage leaves)
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
  - `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd schema_declares_all_three_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (no smoothing assertion runs yet — AC-2 still unwritten).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (three fields + host-keys.toml rows landed).
- Postcondition: `docs/15_config_keys_reference.md` contains all three keys; `cargo xtask gen-config-docs --check` is green.
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
  - `rg -q 'max_volumetric_extrusion_rate_slope' docs/15_config_keys_reference.md && rg -q 'max_volumetric_extrusion_rate_slope' docs/config/host-keys.toml` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the smoothing stage + bounds

- Task IDs: `TASK-000`
- Objective: Limit volumetric-rate slope across print moves (F-only retime, E conserved), honour the skip list and the external-only gate, split long moves at the configured segment length under the trivial floor, and enforce bounds including the master gate; the call sits after the entity loop, before the estimator/M73 tail.
- Precondition: Step 1b green (schema + docs landed); post-loop tail anchor + sticky-feedrate spelling located via the Step-2 dispatch before editing; no draft packet owns the position (verified at authoring — no FORWARD-DEP to read).
- Postcondition: `emit_gcode` smooths through `smooth_extrusion_rates` (F-only, E-conserved, travels/retracts untouched); AC-2 (identity), AC-3 (retime), AC-4 (gate), AC-5 (split), AC-N1 (bounds), AC-N2 (master gate) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the stage exists and compiles).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering the post-entity-loop tail + the per-point `F` emission site only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2340-2420` (`Point3WithWidth.overhang_quartile` + `ExtrusionRole`) only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — host-only omitted)
  - `crates/slicer-gcode/src/estimator.rs` (times the smoothed stream automatically — do not touch)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (travel-shape context — do not touch)
  - `modules/core-modules/machine-gcode-emit/` (wrong seam — do not touch)
- Expected sub-agent dispatches:
  - Question: post-entity-loop tail anchor lines (where `commands` is complete, where the estimator/M73 block begins) + sticky-feedrate spelling at the call site; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: `overhang_quartile: Some` producers (do wall producers stamp it on OuterWall/InnerWall points today?); scope: `crates/ modules/`; return: `LOCATIONS` (≤10 entries) — missing stamp is a follow-up packet, not a blocker
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (stage is in-module parameter, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/PressureEqualizer.cpp` - ctor + `adjust_volumetric_rate` skip list + limiter shape + `output_gcode_line` split rule - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_do_export` gate + marker emission + `GCode::process_layers` position - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Stage compiles clippy-clean; smoothing is a pure function of (move geometry, sticky feedrate, role, `overhang_quartile`, cfg) with the master gate as a stage precondition; no `e` touched (F-only diff review passes); no CONFIG_BLOCK edit; no estimator/ordering/travel-path logic touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the identity, retime, gate, split, bounds, and master-gate cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)); the identity cases need no re-baseline (defaults are byte-identical by construction — unlike packet 289's emitting default).
- Precondition: Step 2 green (stage compiles; helper callable from tests).
- Postcondition: `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd` is fully green (AC-1–AC-5, AC-N1–AC-N2); each of the three keys has at least one non-default behaviour assertion.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signatures + call sites only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/speed_p57_ers_emission_tdd.rs`
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
  - `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (neighbour-identity proof — defaults unsmoothed, so no churn expected)
- Exit condition: Guard fully green; every key has a non-default assertion; no production file changed in this step.

### Step 4: Record deviations + annotate the queue ledger

- Task IDs: `TASK-000`
- Objective: File the DEV-182 row (bounds-as-divergence, always-on markers, IR-native port) and annotate the 04 tier table + 05 packet list so P57 reads 3-in at packet 290; prove doc freshness.
- Precondition: Steps 1b–3 green (schema, stage, and pins landed).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-182 with (a)+(b)+(c) clauses; 04's three Speed/Advanced rows point at packet 290; 05's P57 section reads 3 keys in; `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` (if the repo gates it) are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-row format + max-ID region only
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Speed/Advanced section only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P57 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/` (production frozen — test/docs-only step)
  - `docs/15_config_keys_reference.md` (regen-only; Step 1b owns it)
- Expected sub-agent dispatches:
  - Question: DEV-row exact format + current max DEV id at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (direct, grep-only)
- OrcaSlicer refs:
  - None (deviation text was grounded in Steps 1–2)
- Verification:
  - `rg -q 'DEV-182' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (final re-dispatch)
- Exit condition: DEV-182 greps; ledger rows read 3-in at 290; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guard; blast-radius dispatch owned here |
| Step 1b | S | Regen only |
| Step 2 | M | Smoothing stage + skip/split arms |
| Step 3 | M | Behaviour pins, no re-baseline (defaults identical) |
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
