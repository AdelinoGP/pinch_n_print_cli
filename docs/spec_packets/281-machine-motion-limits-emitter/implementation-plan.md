# Implementation Plan: 281-machine-motion-limits-emitter

## Execution Rules

- Work one atomic step at a time; every step maps to wayfinder ticket 54 (no `docs/07` task IDs exist for queue work).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config surface — eight fields, map arms, bounds, defaults test

- Task IDs: `[]` (ticket 54)
- Objective: Declare the eight P47 fields in `ResolvedConfig` with vector-tolerant ingest, publish them through `to_config_map` like the ten siblings, enforce minimum-0 bounds, and pin defaults.
- Precondition: `crates/slicer-ir/src/resolved_config.rs` contains the `cli_opt @printer` machine-limit window (~lines 2186–2204) and the `to_config_map` window (~lines 278–292).
- Postcondition: All eight fields resolve from scalar and single-entry-vector spellings (first wins), appear in the config map when set, reject negatives with `OutOfRange`, and default `None`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines 270–300 (`to_config_map` sibling arms)
  - `crates/slicer-ir/src/resolved_config.rs` - lines 2180–2215 (`cli_opt` sibling arms)
  - `crates/slicer-ir/src/resolved_config.rs` - lines 830–850 (macro doc comment mentioning `machine_max_*` time modes — update if it enumerates fields)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` (append `machine_motion_limits_defaults`)
  - `crates/slicer-scheduler/src/config_resolution.rs` or the bounds-seam file the dispatch confirms (bounds rows only)
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/*` (no manifest work)
  - `crates/slicer-gcode/src/serialize.rs` (no padding work)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - Blast-radius dispatch result: `ResolvedConfig {` literal sites exist in ~20 files (`crates/pnp-cli`, `crates/slicer-core` algos + tests, `crates/slicer-gcode` emit + tests, `crates/slicer-ir`, `crates/slicer-runtime`); the `declare_resolved_config!` macro generates construction via `Default` + field assignment, so field addition is additive — no literal site breaks unless it asserts exhaustive field counts. The worker must confirm none asserts a field count; if one does, that file joins "allowed to edit" with a count update.
- Expected sub-agent dispatches:
  - Question: list every `ResolvedConfig {` literal + every test asserting a field count or a `to_config_map` length; scope: `crates/ modules/`; return: `LOCATIONS` ≤20 entries
  - Question: where do minimum-0 bounds rows for non-speed host keys live after ticket-113; scope: `crates/slicer-scheduler/src/ crates/slicer-ir/src/`; return: `LOCATIONS` ≤10 entries
- Context cost: `M` (macro file windows + two dispatches + blast-radius confirmation)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - Notes Authoring rules 1–2 + ticket-140 vector-ingest fix + ticket-113 GUI-hint ruling (delegated SUMMARY if over budget, else direct)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (defaults/minima for the eight keys)
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_defaults_tdd machine_motion_limits_defaults 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo test -p slicer-scheduler --test integration machine_limits_range_rejection 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-N1; the new test lands in `tests/integration/config_bounds_enforcement_tdd.rs`, already aggregated by `tests/integration/main.rs` — no new file, no registration risk)
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (blast-radius compile proof)
- Exit condition: `machine_motion_limits_defaults` passes (eight fields `None`, map-absent-when-`None`, map-present-when-set), negative spellings reject with `OutOfRange`, and the workspace check is green.

### Step 2: Envelope extension — M201, M204 R, M205 J in canonical order

- Task IDs: `[]` (ticket 54)
- Objective: Extend 267's envelope builder with the three P47 components behind the existing `emit_machine_limits_to_gcode` + flavor gates, in canonical order, with canonical rounding and the RRF factor rule.
- Precondition: Packet 267's envelope builder exists in `crates/slicer-gcode` (location + function name confirmed by dispatch). If absent: STOP, record `[BLOCK]` in `design.md`, keep packet `draft`, do not reimplement the envelope here.
- Postcondition: With the eight fields set under `marlin2`, the stream opens `M201 …`, `M203 …`, `M204 P… R… T…`, `M205 …`, `M205 J…` in that order; legacy-Marlin travel fallback, RRF factor/comment forms, JD omit-when-zero, and flavor/`false` gates all behave per AC-1/2/3/5/6.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/flavor.rs` - lines 74–120 (helper signatures to reuse)
  - `crates/slicer-gcode/src/emit.rs` - envelope builder window only (exact lines from the precondition dispatch)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs` (or 267's envelope submodule if factored out)
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (append 6 tests: m201, m204r + legacy fallback, JD emit/omit, flavor gate, RRF factor, default identity)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no padding)
  - `modules/core-modules/machine-gcode-emit/*` (no placeholder work)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or version constant is added in this step.
- Expected sub-agent dispatches:
  - Question: what is 267's envelope builder function name, file, gate shape, and line order on disk today; scope: `crates/slicer-gcode/src/`; return: `FACT` ≤5 lines
- Context cost: `M` (one builder window + one dispatch + 6 tests)
- Authoritative docs:
  - `docs/spec_packets/267-printer-machine-power-recovery-emitter/packet.spec.md` - Goal + AC-3/AC-4 (order + flavor contract to preserve)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`GCode::print_machine_envelope`) - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (`set_junction_deviation`) - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (267's tests + 6 new, no regression)
- Exit condition: All `machine_envelope*` tests pass (267's unchanged + AC-1/2/3/5/6/7 green), and no other `gcode_emit_tdd` test changed output.

### Step 3: Estimator minimum-rate clamps + tests

- Task IDs: `[]` (ticket 54)
- Objective: Add `min_extruding_rate` / `min_travel_rate` to `EstimatorLimits` with `0.0` (disabled) defaults and clamp segment feedrates per class.
- Precondition: Step 1 fields exist (`cfg.machine_min_extruding_rate`, `cfg.machine_min_travel_rate`).
- Postcondition: `from_config` maps the two fields (fallback `0.0`); estimation clamps extruding-class segments to at least the extruding minimum and travel-class segments to at least the travel minimum when the minimum is > 0; emitted G-code feedrates are untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/estimator.rs` - lines 27–100 (`EstimatorLimits` + `Default` + `from_config`)
  - `crates/slicer-gcode/src/estimator.rs` - feedrate-resolution window only (exact lines found by grep for the segment `v_target` computation)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/estimator.rs`
  - `crates/slicer-gcode/tests/estimator.rs` (append `minimum_rate_clamp`: extruding + travel two-run comparisons)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (no emission change in this step)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `EstimatorLimits {` struct-literal sites: `crates/slicer-gcode/tests/estimator.rs` line ~49 constructs one literally — that site joins this step (add the two new fields or convert to `..Default::default()` rest, whichever the file's existing style uses). A `LOCATIONS` grep for `EstimatorLimits {` across `crates/ modules/` runs first; every hit is updated in this step, none deferred to check.
- Expected sub-agent dispatches:
  - Question: list every `EstimatorLimits {` literal and every assertion on `EstimatorLimits::default()` field values; scope: `crates/ modules/`; return: `LOCATIONS` ≤10 entries
- Context cost: `S` (one struct window + clamp site + 1 test file)
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated SUMMARY (estimator lives host-side; no scheduler interaction)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` (`minimum_feedrate` / `minimum_travel_feedrate` clamp-only, no G-code) - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test estimator 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (AC-4 + no regression)
- Exit condition: `minimum_rate_clamp` passes (both classes strictly increase estimate time at non-default minima; zero minima reproduce pre-packet estimates exactly).

### Step 4: Divergence row, reference regen, ledger close-out

- Task IDs: `[]` (ticket 54)
- Objective: Record DEV-173, regenerate the config reference, annotate the 04/05 P47 rows, and prove the gates.
- Precondition: Steps 1–3 green.
- Postcondition: `DEVIATION_LOG.md` carries DEV-173 (number re-derived at execution time); the generated reference documents all eight keys; 04/05 P47 rows point at packet 281; clippy + literals + workspace check green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail only (re-derive next free `DEV-###`; assumption DEV-173 must be re-verified, never frozen)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (append DEV-173 row)
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (P47 rows → packet 281, 8 scalars live)
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (P47 → packet 281)
- Files explicitly out of bounds:
  - All `crates/` + `modules/` implementation files (close-out only; no behaviour change here)
  - `crates/slicer-gcode/src/serialize.rs` (no padding, even here)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or version constant is added in this step.
- Expected sub-agent dispatches:
  - Question: re-derive the next free `DEV-###` and the exact row format of the last three rows; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (one number + one format line)
  - Question: run the config-docs regen command and report its exact name + output path; scope: `xtask/src/`; return: `FACT` ≤3 lines
- Context cost: `S` (tail reads + two dispatches + regen)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - Notes ledger-facts rule (re-derive, never freeze)
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'machine_max_acceleration_x' docs/15_config_keys_reference.md && echo PASS || echo FAIL` - FACT PASS
  - `rg -q 'silent_mode' crates modules && echo FAIL || echo PASS` - FACT PASS (AC-N2)
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo xtask check-literals 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: Reference grep PASS, silent grep PASS (prints PASS), clippy + literals clean, and every pipe-suffixed AC command re-dispatched green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Macro-file windows + blast-radius + bounds dispatches |
| Step 2 | M | Envelope builder window + 6 tests; largest step |
| Step 3 | S | Struct window + clamp + 2 tests |
| Step 4 | S | Tail reads + regen + ledger |

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
