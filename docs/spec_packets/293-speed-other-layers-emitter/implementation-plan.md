# Implementation Plan: 293-speed-other-layers-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the table row + twin + pair with schema guards

- Task IDs: `TASK-000`
- Objective: Declare `internal_solid_infill_speed` in the `FeedrateConfig` table (with aligned meta/bounds/default) plus its `ResolvedConfig` twin and the `small_perimeter_speed`/`small_perimeter_threshold` pair, and pin all three spellings with schema guards; prove the struct-literal blast radius is fully owned.
- Precondition: No `FeedrateConfig` row and no `ResolvedConfig` field exists for any of the three spellings (verified at authoring: `internal_solid_infill_speed` is module-parse-only dead input, `small_perimeter*` is zero-occurrence outside map prose and the unrelated module width twin; re-verify with the Step-1 blast-radius dispatch before editing).
- Postcondition: `FeedrateConfig` carries `internal_solid_infill_speed = 100.0` with `SPEED_META None` + `SPEED_BOUNDS (1.0, None)`; `ResolvedConfig` carries the twin `100.0` (`extract_float_or_first`) with its `to_config_map` arm plus the small-perimeter pair (`ResolvedFloatOrPercent 50%-percent` + `0.0` threshold); `speed_p60_other_layers_emission_tdd::schema_declares_both_keys` (AC-1) passes; `cargo check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/feedrate.rs` - `SPEED_KEYS` + `SPEED_META` + `SPEED_BOUNDS` + `Default` region only
  - `crates/slicer-ir/src/resolved_config.rs` - lines covering one `extract_float_or_first` scalar row + the `support_threshold_overlap` `ResolvedFloatOrPercent` row + the sparse `to_config_map` arm only
  - `docs/config/host-keys.toml` - `[speeds]` + `[resolved_config]` sections only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/feedrate.rs`
  - `crates/slicer-ir/src/resolved_config.rs`
  - `docs/config/host-keys.toml`
  - `crates/slicer-gcode/tests/speed_p60_other_layers_emission_tdd.rs` (new guard binary: AC-1 schema case only in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (Step 2 owns the arms)
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — two honest lines via the live map, never the table)
  - `modules/core-modules/rectilinear-infill/` (dead tuple stays dead — do not touch)
  - `modules/core-modules/classic-perimeters/` (width twin untouched — do not touch)
  - `OrcaSlicerDocumented/` (delegate; never load)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch (run first): Question: every `FeedrateConfig { .. }` + `ResolvedConfig { .. }` literal + every test asserting `FeedrateConfig::default()` / `ResolvedConfig::default()` speed values + every test asserting `SPEED_KEY_COUNT`/`SPEED_KEYS.len()`; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries). Result: [implementer pastes the ≤20-entry return here and adds any listed file to "Files allowed to edit" above before editing].
- Expected sub-agent dispatches:
  - Question: `SPEED_KEYS` row + `SPEED_META None (Orca identity)` + `SPEED_BOUNDS (1.0, None)` + `Default 100.0` insertion syntax for one existing row; scope: `crates/slicer-ir/src/feedrate.rs`; return: `SNIPPETS` (≤2 snippets, ≤30 lines each)
  - Question: `declare_resolved_config!` scalar row + `ResolvedFloatOrPercent` row + sparse `to_config_map` arm syntax; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤3 snippets, ≤30 lines each)
  - Question: struct-literal blast radius (above); scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/config/host-keys.toml` - `[speeds]` + `[resolved_config]` sections (direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd schema_declares_both_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (blast-radius proof)
- Exit condition: AC-1 passes; `cargo check --workspace --all-targets` is green; no other test file changed behaviour (no emission assertion runs yet — AC-2 still unwritten).

### Step 1b: Regen generated host-keys docs

- Task IDs: `TASK-000`
- Objective: Regenerate the derived host-keys reference from the Step-1 schema so the doc-impact greps hold.
- Precondition: Step 1 green (table row + twin + pair + host-keys.toml rows landed).
- Postcondition: `docs/15_config_keys_reference.md` contains `internal_solid_infill_speed` and `small_perimeter_speed`; `cargo xtask gen-config-docs --check` is green.
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
  - `rg -q 'internal_solid_infill_speed' docs/config/host-keys.toml && rg -q 'small_perimeter_speed' docs/15_config_keys_reference.md` - FACT pass/fail
- Exit condition: Both greps pass and `--check` is green; no hand edit to the generated file.

### Step 2: Build the reseat + gate + percent resolution + bounds

- Task IDs: `TASK-000`
- Objective: Reseat the internal-solid arm onto the new table field, gate small closed wall loops through the threshold/circumference test with all three value arms, and enforce bounds including the zero-threshold silence.
- Precondition: Step 1b green (schema + docs landed); base-speed match anchor + per-entity `F` call chain (role selection → 291-blend → filament cap) + 291's draft position located via the Step-2 dispatch before editing.
- Postcondition: `resolve_feedrate` maps `InternalSolidInfill` to the new field; the per-entity site overrides small closed wall loops at/under the circumference gate (auto/absolute/percent arms, wall-loops-only qualification); AC-2 (near-identity), AC-3 (independence), AC-4 (gate), AC-N1 (bounds), AC-N2 (silence) all have runnable pins (assertions may still be red until Step 3 fills fixtures — the arms exist and compile).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines covering `DefaultGCodeEmitter::resolve_feedrate` base-speed match + the per-entity `F` resolution site only
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2373-2419` only (`ExtrusionRole` variants) + `is_closed` + `is_loop`
  - `docs/spec_packets/291-slow-down-layers-initial-layer-speed/packet.spec.md` - blend position only (draft — re-derive, do not freeze)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no CONFIG_BLOCK change — two honest lines via the live map)
  - `crates/slicer-ir/src/feedrate.rs` (table frozen after Step 1)
  - `crates/slicer-ir/src/resolved_config.rs` (schema frozen after Step 1)
  - `modules/core-modules/rectilinear-infill/` (dead tuple stays dead — do not touch)
  - `modules/core-modules/classic-perimeters/` (width twin untouched — do not touch)
  - `docs/spec_packets/291-slow-down-layers-initial-layer-speed/` (blend-position reference — do not touch)
  - `docs/spec_packets/289-speed-acceleration-emitter/` (stage-shape reference — do not touch)
- Expected sub-agent dispatches:
  - Question: base-speed match anchor lines + per-entity `F` call chain order (role selection → blend → cap) + `is_closed` call-site spelling at the entity loop; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS` (≤10 entries, one context line each)
  - Question: 291's blend anchor (which call the gate must wrap); scope: `docs/spec_packets/291-slow-down-layers-initial-layer-speed/packet.spec.md`; return: `FACT` (≤5 lines)
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/01_system_architecture.md` - Claim System section, delegated SUMMARY (arms are in-module parameters, not selection)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `erSolidInfill` speed arm + `extrude_loop` gate shape (entry, length test, three value arms, perimeter-only application) - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` - `SMALL_PERIMETER_LENGTH` `* 2 * PI` conversion - delegate; never load
- Verification:
  - `cargo check -p slicer-gcode --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy -p slicer-gcode --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Arms compile clippy-clean; internal-solid resolves from the new field; the gate is a pure function of (role, closed-ness, planar length, threshold, speed value, live outer speed) wrapping the blended base; no E logic touched; no CONFIG_BLOCK edit; no wall/infill-module file touched.

### Step 3: Pin behaviour at non-default values (AC-2–AC-N2)

- Task IDs: `TASK-000`
- Objective: Fill the TDD guard with the near-identity, independence, gate+arms, bounds, and silence cases; prove each key changes behaviour at a non-default value (map Authoring-rule gate (b)); confirm the default F stream carries no new values and the CONFIG_BLOCK carries exactly the one honest twin line (AC-2 pins both halves — no re-baseline is owed beyond the one line).
- Precondition: Step 2 green (arms compile; helpers callable from tests).
- Postcondition: `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd` is fully green (AC-1–AC-4, AC-N1–AC-N2); each of the two speed keys has at least one non-default behaviour assertion (the threshold rides AC-4/AC-N2 as the gate input, not a third behaviour key).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - helper signatures + call sites only (no re-read of the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/speed_p60_other_layers_emission_tdd.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (only if a golden breaks — and a break is a packet defect, not churn: default F values are identical, so any `F` diff fails review; the only honest diff class is the +1 CONFIG_BLOCK line in a block-counting test, and a second golden breaking sends the packet back for a split, not a third edit)
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/feedrate.rs` (table frozen after Step 1)
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
  - `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (near-identity proof — F values already green, not re-baselined; only the +1-line block class may move)
- Exit condition: Guard fully green; every speed key has a non-default assertion; AC-2's F-identity holds with exactly +1 config line; no production file changed in this step.

### Step 4: Record deviations + annotate the queue ledger

- Task IDs: `TASK-000`
- Objective: File the DEV-185 row (bounds-as-divergence, scalar-not-vector, gate porting notes) and annotate the 04 tier table + 05 packet list so P60 reads 2-in at packet 293; prove doc freshness.
- Precondition: Steps 1b–3 green (schema, arms, and pins landed).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-185 with (a)+(b)+(c) clauses; 04's two Speed/Other-layers rows point at packet 293; 05's P60 section reads 2 keys in; `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` (if the repo gates it) are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - DEV-row format + max-ID region only
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Speed/Other layers speed section only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P60 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/` (production frozen — test/docs-only step)
  - `docs/spec_packets/289-speed-acceleration-emitter/` (never edit another packet)
  - `docs/spec_packets/291-slow-down-layers-initial-layer-speed/` (never edit another packet)
  - `docs/spec_packets/292-speed-jerk-xy-emitter/` (never edit another packet)
  - `docs/15_config_keys_reference.md` (regen-only; Step 1b owns it)
- Expected sub-agent dispatches:
  - Question: DEV-row exact format + current max DEV id at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines)
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (direct, grep-only)
- OrcaSlicer refs:
  - None (deviation text was grounded in Steps 1–2)
- Verification:
  - `rg -q 'DEV-185' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (final re-dispatch)
- Exit condition: DEV-185 greps; ledger rows read 2-in at 293; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + guards; table-alignment + struct-literal dispatches owned here |
| Step 1b | S | Regen only |
| Step 2 | M | Reseat + gate + percent resolution + bounds |
| Step 3 | M | Behaviour pins + near-identity proof |
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
