# Implementation Plan: 169-time-estimator-slice-stats

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config plumbing — optional machine-limit and density fields

- Task IDs: `TASK-275`
- Objective: Add optional `machine_max_acceleration_extruding`, `machine_max_acceleration_travel`, `machine_max_speed_x/y/z/e` (or the crate's existing per-axis convention), `machine_max_jerk_x/y/z/e`, `filament_density`, and (if absent) `first_layer_height` fields to `ResolvedConfig` via its cli-key macro, all snake_case, all `Option`-shaped or defaulted so absent keys are representable.
- Precondition: grep confirms `machine_max_` has zero matches in `crates/` (grounded 2026-07-17); `filament_density` exists only as `serialize.rs` header default `filament_density_g_cm3 = 1.24`.
- Postcondition: `ResolvedConfig` exposes the new fields; `cargo check` green; absent keys are distinguishable from supplied ones.
- Files allowed to read, with ranges:
  - `crates/slicer-ir/src/resolved_config.rs` — lines 600-660 plus macro-definition region located by dispatch
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (padding surface is packet 167's), `docs/07_implementation_status.md`
- Expected sub-agent dispatches:
  - Question: exact macro syntax for an optional f32 cli-key field and whether `first_layer_height` already exists; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2, ≤20 lines)
- Context cost: `S`
- Authoritative docs:
  - CLAUDE.md §Config Key Naming Convention (snake_case) — already in context
- OrcaSlicer refs: none
- Verification:
  - `cargo check -p slicer-ir --all-targets` — FACT pass/fail
- Exit condition: a test or doctest constructing `ResolvedConfig` without the new keys yields the absent/None state; with keys set, values round-trip. Falsified if absent keys are indistinguishable from zero.

### Step 2: Estimator module (TDD) in slicer-gcode

- Task IDs: `TASK-275`
- Objective: Write failing tests in new `crates/slicer-gcode/tests/estimator.rs` (analytic single-segment trapezoid, two-tool volume map + toolchange count, fallback defaults, metadata fill), then implement `crates/slicer-gcode/src/estimator.rs` (`EstimatorLimits` with documented `Default` fallbacks, `PrintEstimate`, `estimate_print`) and wire `estimated_print_time_s` at `emit.rs:739`; export from `lib.rs`.
- Precondition: Step 1 merged; `GCodeCommand` shapes confirmed (`Move { x,y,z,e,f,role }`, `ToolChange { after_entity_index, from, to }`, `ExtrusionMode`, `Retract`, `Unretract`).
- Postcondition: all `--test estimator` tests pass; `estimated_print_time_s > 0` for any non-empty move stream; volumes keyed per extruder as `BTreeMap`.
- Files allowed to read, with ranges:
  - `crates/slicer-gcode/src/emit.rs` — lines 60-80, 130-190, 360-380, 700-760
  - `crates/slicer-ir/src/slice_ir.rs` — lines 2190-2300
  - `crates/slicer-gcode/src/serialize.rs` — lines 60-140 (density-default hazard only)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/estimator.rs` (new)
  - `crates/slicer-gcode/tests/estimator.rs` (new)
  - `crates/slicer-gcode/src/emit.rs` (plus the 1-2-line `lib.rs` export, justified as trivial)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (Step 3), `crates/slicer-gcode/src/serialize.rs` beyond lines 60-140
- Expected sub-agent dispatches:
  - Question: how serialize.rs interprets `ExtrusionMode` + `Move.e` (absolute vs relative; reset semantics); scope: `crates/slicer-gcode/src/serialize.rs`; return: `FACT` (≤5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/fork-gaps-wave1-plan.md` — Packet A section only
- OrcaSlicer refs: none (fresh Marlin-style implementation; no port, no attribution header)
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test estimator 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail; failure SNIPPETS from the log
- Exit condition: AC-1, AC-2, AC-3, AC-N1 commands all PASS. Falsified if the analytic trapezoid test only matches with tolerance >2% or the two-tool map has extra/missing keys.

### Step 3: slice_stats event + schema 1.2.0 + end-of-slice emission in run.rs (TDD)

- Task IDs: `TASK-275`
- Objective: Write failing tests in `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (slice_stats shape/ordering, weight omission without density, 1.1.0 round-trip compat), then: add `ProgressEventType::SliceStats` + optional fields + `slice_stats` ctor, bump `PROGRESS_EVENT_SCHEMA_VERSION` (line 29) and `PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED` (line 35) to `"1.2.0"`, have `postpass.rs` run the estimator, fill `estimated_print_time_s`, and surface `PrintEstimate` in the pipeline output, and wire the end-of-slice path in `run.rs` (emitter/collector site, lines 168-174) to emit `slice_stats` then **create** the `slice_complete` emission from `SliceEventCollector`'s counts — grounding confirmed `ProgressEvent::slice_complete` has no production call site today.
- Precondition: Step 2 merged; `PrintEstimate` exported from slicer-gcode; `run.rs:168-174` wires `JsonLinesEmitter` + `SliceEventCollector`.
- Postcondition: JSONL stream contains exactly one `slice_stats` before exactly one `slice_complete` with all amended fields; `gcode_weight_grams` omitted when density absent; all pre-1.2.0 events serialize byte-identically.
- Files allowed to read, with ranges:
  - `crates/slicer-runtime/src/progress_events.rs` — lines 20-200, 340-380, 500-560
  - `crates/slicer-runtime/src/postpass.rs` — lines 150-320
  - `crates/slicer-runtime/src/run.rs` — lines 150-220
  - `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` — existing test shapes
- Files allowed to edit (at most 3, plus the test file):
  - `crates/slicer-runtime/src/progress_events.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/src/postpass.rs`
  - `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (test-only; justified extra per template rule)
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` — any needed change here is a `[BLOCK]`, stop and report
  - `crates/slicer-gcode/**` (frozen after Step 2), `crates/slicer-runtime/src/{pipeline.rs,progress_instrumentation.rs}` (Step 4)
- Expected sub-agent dispatches:
  - Question: at run.rs's post-pipeline return path, which bindings are live (pipeline output struct, config handle, collector) for the two emissions; scope: `crates/slicer-runtime/src/run.rs`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/09_progress_events.md` — lines ~100-160 (additive rule, version table)
- OrcaSlicer refs: none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- slice_stats 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- progress_event_1_1_0_roundtrip_unchanged 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
- Exit condition: AC-4, AC-N2, AC-N3 commands all PASS. Falsified if `slice_stats` appears after `slice_complete`, `slice_complete` appears zero or multiple times, the event carries a `cost`/`gcode_cost` key, or `gcode_weight_grams` serializes as `0`/`null` without density.

### Step 4: layer_count through the instrumentation layer (TDD)

- Task IDs: `TASK-275`
- Objective: Write a failing test (`phase_start_per_layer_carries_layer_count`) in `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (or extend the colocated `#[cfg(test)]` block in `progress_instrumentation.rs` where `phase_start_and_phase_end_emit_paired_events` lives), then: add `layer_count: Option<u32>` to `ProgressEvent::phase_start` (if not already landed in Step 3's struct fields), add an additive default-implemented `PipelineInstrumentation::on_phase_start_with_layer_count(&self, phase: Phase, layer_count: Option<u32>)` delegating to `on_phase_start`, override it in `ProgressPipelineInstrumentation` to thread the count into the event construction at `progress_instrumentation.rs:104`, and call it with `Some(plan.global_layers.len() as u32)` at `pipeline.rs:352-356`.
- Precondition: Step 3 merged (struct fields and 1.2.0 consts in place); `PipelineInstrumentation` trait definition located via dispatch.
- Postcondition: per-layer `phase_start` carries `layer_count == plan.global_layers.len()`; all other phases omit the key; every other `PipelineInstrumentation` impl compiles unchanged.
- Files allowed to read, with ranges:
  - `crates/slicer-runtime/src/progress_instrumentation.rs` — lines 40-130, plus its test module
  - `crates/slicer-runtime/src/pipeline.rs` — lines 340-370, plus the trait definition site from the dispatch
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/progress_instrumentation.rs`
  - `crates/slicer-runtime/src/pipeline.rs` (or wherever the `PipelineInstrumentation` trait lives, per dispatch)
  - `crates/slicer-runtime/tests/integration/progress_events_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**`, `crates/slicer-runtime/src/{run.rs,postpass.rs}` (frozen after Step 3), `crates/slicer-schema/wit/**`
- Expected sub-agent dispatches:
  - Question: definition site of the `PipelineInstrumentation` trait (file:line, method list) and every impl of it; scope: `crates/slicer-runtime/src/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/09_progress_events.md` — line ~113 (additive rule) only
- OrcaSlicer refs: none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- phase_start_per_layer_carries_layer_count 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"` — FACT pass/fail
  - `cargo check -p slicer-runtime --all-targets` — FACT pass/fail (proves no trait impl broke)
- Exit condition: AC-5 command PASSes and slicer-runtime check is green. Falsified if any pre-existing `PipelineInstrumentation` impl required edits or non-per-layer `phase_start` events serialize a `layer_count` key.

### Step 5: docs/09 amendment + workspace gates

- Task IDs: `TASK-275`
- Objective: Amend `docs/09_progress_events.md`: replace the reserved 1.2.0 row (line ~153, "Reserved for `pinch_n_print_studio` T-096") with the shipped `slice_stats` definition (five reserved fields + `extruded_volume_mm3` + `toolchange_count`; no cost field, with the fork-computes-cost rationale), set the current version to 1.2.0, document `layer_count` on `phase_start(per_layer)` as additive-optional, the weight-omission semantics, and the fallback machine-limit defaults, and rewrite the `--instrument-stderr` section (lines ~160-167): it currently claims the instrumented stream is `"1.3.0"` while the const was `"1.1.0"` — state that the instrumented stream is now `"1.2.0"` (same additive payload) and `"1.3.0"` remains reserved for the future stage/module-event schema; then run the workspace gates.
- Precondition: Steps 1-4 merged and green.
- Postcondition: doc matches shipped behavior; AC-6 grep PASS; check/clippy green.
- Files allowed to read, with ranges:
  - `docs/09_progress_events.md` — lines 100-165
- Files allowed to edit (at most 3):
  - `docs/09_progress_events.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (closure dispatch only), all code files (frozen)
- Expected sub-agent dispatches:
  - Question: run `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT pass/fail` + ≤20-line failure SNIPPETS
- Context cost: `S`
- Authoritative docs:
  - `docs/09_progress_events.md` — the file under edit
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'extruded_volume_mm3' docs/09_progress_events.md && rg -q 'toolchange_count' docs/09_progress_events.md && ! rg -q 'Reserved for \`pinch_n_print_studio\` T-096' docs/09_progress_events.md && echo PASS` — FACT PASS/absent
  - `cargo check --workspace --all-targets` — FACT pass/fail (delegated)
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail (delegated)
- Exit condition: AC-6 command prints PASS and both workspace gates pass. Falsified if any "Reserved" text for the 1.2.0 row survives or the doc omits weight/fallback semantics.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Single-file macro addition |
| Step 2 | M | Estimator core + analytic tests |
| Step 3 | M | Schema bump + slice_stats/slice_complete emission in run.rs/postpass.rs |
| Step 4 | S | layer_count via additive trait method + progress_instrumentation.rs |
| Step 5 | S | Doc amendment (incl. instrumented-version divergence) + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (mint the TASK-275 row per `task-map.md`), never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Plan-mandated G+A end-to-end (delegated, FACT pass/fail): slice `resources/benchy.stl` with fork-realistic raw_config supplying `machine_max_*` / `filament_density` per the docs/02 "CONFIG_BLOCK viewer-key contract"; assert the `slice_stats` JSONL line is present with all fields and `estimated_print_time_s > 0`.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
