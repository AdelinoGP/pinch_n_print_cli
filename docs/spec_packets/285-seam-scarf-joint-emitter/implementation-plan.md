# Implementation Plan: 285-seam-scarf-joint-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1a: Declare the eight fields (no behaviour)

- Task IDs: `TASK-000`
- Objective: all eight keys exist with canonical defaults; compiles; no behaviour change yet (no reader wired).
- Precondition: none of the eight keys is declared (verified 2026-09-07 — seven zero-occurrence, `seam_gap` padding-only).
- Postcondition: `ResolvedConfig` carries eight scalar-global fields (macro defaults = canonical defaults; seven deliberately omitted from `to_config_map` — host-only emission control, ticket-42 P35 precedent — `seam_gap` carried by an arm in the same file so its live value shadows the padding twin via the serializer's emitted-set dedup, 284 precedent); `machine-gcode-emit.toml` holds eight tables with dead-default comments; `host-keys.toml` holds eight rows.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - the `declare_resolved_config!` invocation block only (field-syntax neighbourhood) - purpose: exact insertion syntax.
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - P35 rows neighbourhood only - purpose: table comment + row format precedent.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `docs/config/host-keys.toml`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (no readers until Step 2)
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — rule 2)
  - `docs/spec_packets/277-*/**` and every other packet dir
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `ResolvedConfig` fields are macro-declared: the ticket-126 generated `__drc_overlay_arms!` + whole-struct `PartialEq` arms expand automatically — no hand-written overlay site exists to update. Test-side struct literals over `ResolvedConfig` must use `..` rest or waiver — enforced by `cargo xtask check-literals` in Step 1b's verification, not by pre-listing every literal.
  - Dispatch a `LOCATIONS` worker for `ResolvedConfig {` literal sites under `crates/*/tests/` before editing; cite the count inline in the commit message.
- Expected sub-agent dispatches:
  - Question: list `ResolvedConfig {` literal construction sites under crates/*/tests; scope: `crates/`; return: `LOCATIONS ≤20`
- Context cost: `S`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (config-surface conventions)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (confirm the eight defaults/bounds in the table — re-derive, the authoring-time values are in `design.md`)
- Verification:
  - `cargo check -p slicer-ir --all-targets 2>&1 | tail -3` - FACT pass/fail (declaration compiles, all targets)
- Exit condition: check green; no behaviour wired (test file does not exist yet — Step 1b creates it).

### Step 1b: DEV-177 row + schema guard test + doc regen

- Task IDs: `TASK-000`
- Objective: DEV-177 row logged (ID re-derived at implementation — never trust `177` if the log moved); new guard test file proves AC-1 (all eight declarations); `docs/15` regenerated. Bounds rejection (AC-N1) is NOT green here — its validator is built in Step 2.
- Precondition: Step 1a fields exist.
- Postcondition: DEV-177 greppable; `seam_scarf_joint_emission_tdd` binary exists with the schema filter green; generated tables fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - last 5 rows only - purpose: ID format + next-free re-derivation at implementation time.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs`
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (still no readers)
  - `crates/slicer-gcode/src/serialize.rs` (padding table untouched — rule 2)
  - every other packet dir
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step (test file + log row + generated doc only).
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: none packet-specific.
- OrcaSlicer refs: none (declaration confirmation rides Step 1a).
- Verification:
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask gen-config-docs 2>&1 | tail -3` - FACT pass/fail (regen after the schema/host-keys edits; `docs/15_config_keys_reference.md` is generator output — running the generator IS the edit)
- Exit condition: schema filter passes; `rg -q 'DEV-177' docs/DEVIATION_LOG.md` passes with the ID re-derived free at implementation time; `cargo xtask gen-config-docs --check` green.

### Step 2: `seam_gap` clipping + bounds validation + `order_lock` bypass

- Task IDs: `TASK-000`
- Objective: closed loops clip start/end by the resolved gap at emission; the five ranged keys reject out-of-range values at the emitter gate with stable errors (284's DEV-176(c) precedent — canonical bounds are GUI hints, the port rejects); locked paths bypass untouched (AC-2 + AC-N1 + AC-N2).
- Precondition: Step 1 fields readable from the emitter's resolved config.
- Postcondition: default config clips every unlocked loop by exactly 10% of nozzle diameter (absolute spelling exact mm); `scarf_angle_threshold = 200`, `scarf_joint_flow_ratio = 3.0`, `scarf_joint_speed = 0`, `scarf_overhang_threshold = -1`, `seam_gap = -1` each reject naming the key; `order_lock` paths byte-identical; scarf still unwired (zero scarf paths).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - per-entity loop neighbourhood only - purpose: clip insertion site + locked-path signal.
  - `crates/slicer-gcode/src/serialize.rs` - `emit_config_kv` dedup neighbourhood only - purpose: shadow-without-touching proof for the padding twin.
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/layer_executor.rs` (lock machinery consumed, never moved)
  - `crates/slicer-gcode/src/serialize.rs` (read-only this step)
  - infill modules (Fill-side concentric arm stays unimplemented — DEV-177(b))
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step.
- Expected sub-agent dispatches:
  - Question: run the three filters and report pass/fail + failing assertion ≤20 lines; scope: `crates/slicer-gcode`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - gap mm↔unit conversion range only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`GCode::extrude_loop`) - delegate; never load (clip + slope-termination shape)
- Verification:
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd default_path_clips_gap_no_scarf 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (bounds gate lives at emission — 284 DEV-176(c) precedent — so its filter goes green here, not in Step 1b)
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd locked_paths_bypass_gap_and_scarf 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: all three filters pass; unlocked loops shorten by exactly the resolved gap with unchanged move count; all five ranged keys reject (incl. negative overhang); locked paths byte-identical.

### Step 3: Scarf stage — master gate + conditional gates + overlap emission

- Task IDs: `TASK-000`
- Objective: `has_scarf_joint_seam = true` emits tapered scarf overlaps of exactly the resolved-gap length; `seam_slope_conditional` restricts to smooth, supported loops (AC-3 + AC-4).
- Precondition: Step 2 clip live (overlap bridges the clipped gap).
- Postcondition: default config still emits zero scarf paths; enabled config scars smooth loops always, sharp/unsupported loops only when unconditional; smoothness uses radians-converted `scarf_angle_threshold`, overhang uses `scarf_overhang_threshold` × line width.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - Step-2 clip site neighbourhood only - purpose: stage insertion order (bypass → clip → gate → overlap).
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/seam-placer/*` (placed seam consumed, never moved)
  - P53 key names (`seam_slope_type`, `seam_slope_*`, `wipe_*`) — never declared or read here
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step.
- Expected sub-agent dispatches:
  - Question: resolve the [FWD] overlap vertex placement against canonical `extrude_loop`; scope: `OrcaSlicerDocumented/src/libslic3r/`; return: `SUMMARY ≤200 words`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - overlap-length conversion range only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`GCode::extrude_loop`) - delegate; never load (gate order + smoothness/overhang test shapes)
- Verification:
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd scarf_enable_emits_sloped_paths 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd conditional_gates_on_smoothness 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: both filters pass; scarf overlap length equals resolved gap at defaults; [FWD] closed with the delegated shape cited in a code comment naming file + function.

### Step 4: Scarf flow + speed modulation

- Task IDs: `TASK-000`
- Objective: scarf segments carry `flow_factor = scarf_joint_flow_ratio` and `min(role, cap)` feedrate from `scarf_joint_speed` (AC-5); `SPEED_KEYS` untouched.
- Precondition: Step 3 overlap emission live.
- Postcondition: ratio `0.5` → factor exactly `0.5`; speed `50%` → half role speed; absolute `30` → `1800` mm/min; defaults (`1.0`, `100%`) change nothing versus Step 3.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - Step-3 overlap site neighbourhood only - purpose: factor/cap application point.
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/feedrate.rs` (`SPEED_KEYS` explicitly not extended — ticket-109 precedent)
  - `crates/slicer-gcode/src/serialize.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - feedrate mm/s→mm/min range only if touched
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`GCode::_extrude`) - delegate; never load (factor/cap application point)
- Verification:
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd scarf_flow_and_speed_modulate 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: filter passes with exact factor/cap assertions for percent AND absolute spellings; no emitter role multiplier introduced (factor rides the scarf segments' own `flow_factor`).

### Step 5: `role_based_wipe_speed` arm (FORWARD-DEP on draft 277)

- Task IDs: `TASK-000`
- Objective: 277's wipe `Move` takes the role speed when `true`, `FeedrateConfig::wipe_speed` when `false` (AC-6).
- Precondition: draft packet 277's wipe `Move` EXISTS in `emit.rs` — check first; if absent, SKIP this step entirely at implementation (re-enter after 277 lands) and say so in the commit message. Do not build a second wipe path under any circumstance.
- Postcondition: wipe feedrate source selects per the flag; default `true` preserves 277's role-speed behaviour (joint default unchanged).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - 277's wipe-`Move` site only - purpose: feedrate-source hook (reconcile names with 277's plan before touching).
  - `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/design.md` - wipe-site section only - purpose: name/shape reconciliation (ledger — 277 may have moved).
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_scarf_joint_emission_tdd.rs`
- Files explicitly out of bounds:
  - `docs/spec_packets/277-*/**` (read-only reconciliation; never edit another packet)
  - every other emitter decision (no drive-by changes at the wipe site)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: none packet-specific.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`Wipe::wipe`, `Wipe::calculateWipeRetractionLengths`) - delegate; never load (selection semantic only)
- Verification:
  - `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd wipe_speed_selects_role_or_configured 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (or SKIP-recording if 277 absent)
- Exit condition: filter passes, OR step skipped with the skip recorded; never a second wipe path.

### Step 6: Fixture re-baseline + full gates

- Task IDs: `TASK-000`
- Objective: every golden/integration fixture that moves under live `seam_gap` shows EXACTLY the clip delta with measured justification; full gates green.
- Precondition: Steps 1–4 complete (Step 5 as applicable).
- Postcondition: no unexplained fixture delta remains; packet-level gate commands green.
- Files allowed to read, with ranges when over 300 lines:
  - Failing-fixture diffs only, ≤20 lines each - purpose: prove each delta is exactly the gap clip.
- Files allowed to edit (at most 3):
  - Fixture files whose delta is proven to be exactly the `seam_gap` clip (enumerate in the commit; any other delta is a defect — fix the code, not the fixture).
- Files explicitly out of bounds:
  - All production code (frozen this step — a red fixture re-opens the owning step instead).
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant added in this step.
- Expected sub-agent dispatches:
  - Question: which fixtures shift and by what measured per-fixture delta; scope: `crates/slicer-gcode/tests/`; return: `LOCATIONS ≤20` (fixture + delta each).
- Context cost: `S`
- Authoritative docs: none packet-specific.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (only intended clip deltas)
  - `cargo check --workspace --all-targets 2>&1 | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tail -3` - FACT pass/fail (generated tables fresh at closure)
- Exit condition: every pipe-suffixed AC in `packet.spec.md` re-dispatched green; remaining packet-local risk recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1a | S | declaration only, no behaviour |
| Step 1b | S | DEV row + guard tests + doc regen |
| Step 2 | S | clip + bypass |
| Step 3 | M | scarf build (largest step — single file, single stage; splitting would strand gates from overlap) |
| Step 4 | S | factor/cap |
| Step 5 | S | conditional 277 arm (may skip) |
| Step 6 | S | re-baseline + gates |

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

`cargo check` and `cargo clippy` gate commands must use `--all-targets` so the test, bench, and example targets compile (repo policy); narrow `cargo test -p <crate> --test <file>` commands stay binary-targeted and tee combined output to `target/test-output.log` per the test-output rule.
