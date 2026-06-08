# Implementation Plan: 92_region-split-manifest-and-dispatch

## Execution Rules

- One atomic step at a time.
- TDD for validators: write the test fixture + the failing assertion first, then implement the validator.
- Test output teed to `target/test-output.log`.
- Pre-packet baseline SHA on `regression_wedge.stl` must be captured (Step 0) — AC-11 depends on it.

## Steps

### Step 0: Capture pre-packet baseline g-code SHA into closure-log.md

- Task IDs:
  - `TASK-242`
- Objective: record byte-identical baseline before any edit (AC-10 prerequisite). The SHA is written to `.ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md` as the line `P91_BASELINE_SHA=<hex>` so AC-10's shell command can read it back.
- Precondition: P91 closed; working tree at its parent commit.
- Postcondition: baseline SHA recorded in `closure-log.md`.
- Files allowed to read: none.
- Files allowed to edit:
  - `.ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md` (CREATE or append).
- Files explicitly out-of-bounds: any other file.
- Expected sub-agent dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-baseline.gcode && sha256sum /tmp/p92-baseline.gcode | awk '{print $1}'`; return FACT (single sha256 hash, hex only)".
  - Then write `P91_BASELINE_SHA=<hash>` as a line in `closure-log.md` (delegated edit).
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `grep -q 'P91_BASELINE_SHA=[a-f0-9]\{64\}' .ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md` exits 0.
- Exit condition: closure-log.md carries the baseline SHA.

### Step 0.5: Open TASK-242 row in `docs/07_implementation_status.md`

- Task IDs:
  - `TASK-242`
- Objective: register TASK-242 in the backlog (no row exists at packet activation — verified during refinement). Two-touch sequence: open here (in-progress), close at the Packet Completion Gate (implemented).
- Precondition: Step 0 complete.
- Postcondition: a new row `- [ ] TASK-242 — Manifest [[region_split]] schema + priority registry + per-layer host-filtered dispatch (packet 92). In progress.` appears in `docs/07_implementation_status.md` near the TASK-241 row at line 212.
- Files allowed to read:
  - `docs/07_implementation_status.md` — range-read around line 212 only (≤ 20 lines) to confirm insertion point and style.
- Files allowed to edit (≤ 1):
  - `docs/07_implementation_status.md`.
- Files explicitly out-of-bounds: any other file.
- Expected sub-agent dispatches:
  - "Range-read `docs/07_implementation_status.md` lines 205-225; return SNIPPETS (≤ 20 lines) — purpose: confirm row-format style next to TASK-241."
  - Delegated edit: insert the TASK-242 row immediately below the TASK-241 row, matching the existing checkbox/format.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `rg -q 'TASK-242' docs/07_implementation_status.md` exits 0.
- Exit condition: row visible in the file at the in-progress state.

### Step 1: Inventory manifest types in `slicer-scheduler` (sanity refresh)

- Task IDs:
  - `TASK-242`
- Objective: confirm the type locations recorded during refinement haven't drifted: `LoadedModule` at `crates/slicer-scheduler/src/manifest.rs:29`, `DiagnosticLevel` at :413, `LoadDiagnostic` at :424, `LoadError` at :437, `LoadErrorKind` at :450, `ingest_manifest` at :532, `load_module_from_paths` at :475, `load_modules_from_roots` at :483.
- Precondition: Step 0.5 complete.
- Postcondition: any drift in those line numbers reflected in implementer's notes; no edits.
- Files allowed to read: none directly.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `rg -nE 'pub struct LoadedModule|pub enum LoadErrorKind|pub struct LoadError\\b|fn ingest_manifest|fn load_module_from_paths|fn load_modules_from_roots' crates/slicer-scheduler/src/manifest.rs`; return LOCATIONS (≤ 10 entries)".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: LOCATIONS lists all 6 hits.
- Exit condition: locations confirmed (or drift recorded).

### Step 2: Add `CORE_REGION_SPLIT_PRIORITIES` + `COMMUNITY_PRIORITY_FLOOR` constants in `slicer-schema`

- Task IDs:
  - `TASK-242`
- Objective: AC-2.
- Precondition: Step 1 complete.
- Postcondition: both constants exist with doc-comments; workspace compiles.
- Files allowed to read:
  - `crates/slicer-schema/src/lib.rs` — full read (likely small).
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/src/lib.rs` (or the appropriate sibling module if `slicer-schema` is split — Step 1's LOCATIONS clarifies).
- Files explicitly out-of-bounds for this step: any other file.
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-schema`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b" — priority registry definition.
- OrcaSlicer refs: none.
- Verification: `cargo check -p slicer-schema` passes; the two `rg -q` checks from AC-2 pass.
- Exit condition: AC-2 satisfied.

### Step 3: Add `RegionSplitDeclaration`, `RegionSplitValueType`, `LoadedModule.region_splits` + cached HashSet, and the 4 new `LoadErrorKind` variants

- Task IDs:
  - `TASK-242`
- Objective: enable parsing + structured error reporting.
- Precondition: Step 2 complete.
- Postcondition: types exist; default-empty `region_splits` deserialize cleanly on manifests with no `[[region_split]]` section; the `LoadErrorKind` enum has 4 new variants (`DuplicateRegionSplitSemantic`, `ScalarValueTypeNotAllowedInRegionSplit`, `CommunityPriorityBelowFloor`, `CorePriorityMismatch`); `LoadedModule` carries `region_splits: Vec<RegionSplitDeclaration>` AND a derived `region_split_semantics: HashSet<String>` cached at module-load. NO new `MissingField` or `TypeMismatch` variants — reuse existing `LoadErrorKind::Schema` and `LoadErrorKind::TomlParse`.
- Files allowed to read:
  - `crates/slicer-scheduler/src/manifest.rs` — range-read 25-75 (LoadedModule), 410-470 (Diagnostic / LoadError / LoadErrorKind).
- Files allowed to edit (≤ 1):
  - `crates/slicer-scheduler/src/manifest.rs`.
- Files explicitly out-of-bounds for this step:
  - Other scheduler files; runtime files.
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-scheduler`; return FACT pass/fail with first error" — purpose: gate.
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` §"Manifest TOML Schema".
- OrcaSlicer refs: none.
- Verification: `cargo check` clean; an inline `#[test]` parsing the minimal `basic.toml` succeeds.
- Exit condition: types defined; default-empty parse works; the four new `LoadErrorKind` variants compile.

### Step 4: Write failing-validator tests (TDD); implement validators; pass tests

- Task IDs:
  - `TASK-242`
- Objective: AC-1, AC-3, AC-4, AC-5, AC-6, AC-N3.
- Precondition: Step 3 complete.
- Postcondition: 6 validator branches green (4 new `LoadErrorKind` variants + reused `Schema` for missing-field + reused `TomlParse` for malformed-type).
- Files allowed to read:
  - `crates/slicer-scheduler/src/manifest.rs` — range-read.
  - `crates/slicer-scheduler/src/validation.rs` — range-read 300-350 (existing validation pass pattern that the new validators plug into).
- Files allowed to edit (≤ 3 per commit):
  - `crates/slicer-scheduler/src/validation.rs` — add the four new region-split validators in the post-deserialize pass; reuse existing `Schema` and `TomlParse` paths for missing-field and malformed-type.
  - `crates/slicer-scheduler/tests/fixtures/region_split_manifests/*.toml` — 6 new tiny TOML files (CREATE): `basic.toml`, `duplicate_semantic.toml`, `scalar_value_type.toml`, `community_below_floor.toml`, `core_priority_mismatch.toml`, `priority_type_mismatch.toml`.
  - `crates/slicer-scheduler/tests/region_split_manifest_tdd.rs` — new test file (CREATE).
- Files explicitly out-of-bounds for this step:
  - `slicer-runtime` source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-scheduler region_split 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: gate.
- Context cost: `M`.
- Authoritative docs: roadmap §"P1b" validator subsection.
- OrcaSlicer refs: none.
- Verification: each of the 6 tests passes; the error-message content matches the AC's described shape; AC-N3 asserts `LoadErrorKind::TomlParse` (not a new variant).
- Exit condition: AC-1, AC-3, AC-4, AC-5, AC-6, AC-N3 satisfied.

### Step 5: Implement `aggregate_region_splits` + tied-priority `LoadDiagnostic::Warning` + canonical-order accessor

- Task IDs:
  - `TASK-242`
- Objective: AC-7, AC-8, AC-N2.
- Precondition: Step 4 green.
- Postcondition: aggregation function returns BTreeMap in canonical order; tied priorities push `LoadDiagnostic { level: DiagnosticLevel::Warning, ... }` onto a caller-provided `&mut Vec<LoadDiagnostic>` (NOT the runtime `ProgressEvent` channel); empty-input case yields empty BTreeMap.
- Files allowed to read:
  - `crates/slicer-scheduler/src/manifest.rs` — range-read lines 410-500 (LoadDiagnostic / DiagnosticLevel / one existing emission site at :493).
  - `crates/slicer-scheduler/src/validation.rs` — range-read lines 320-355 (another emission site at :333,342) for pattern reference.
- Files allowed to edit (≤ 3):
  - `crates/slicer-scheduler/src/region_split.rs` (NEW).
  - `crates/slicer-scheduler/src/lib.rs` — `pub mod region_split;` declaration.
  - `crates/slicer-scheduler/tests/region_split_aggregation_tdd.rs` (NEW) — tests for AC-7, AC-8, AC-N2.
- Files explicitly out-of-bounds for this step:
  - Runtime files (Step 6).
  - `docs/09_progress_events.md` — NOT the channel for this WARN.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-scheduler region_split_aggregation 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: gate.
- Context cost: `M`.
- Authoritative docs:
  - `crates/slicer-scheduler/src/manifest.rs:413-450` (DiagnosticLevel / LoadDiagnostic).
- OrcaSlicer refs: none.
- Verification: 3 tests pass; AC-7 asserts a `LoadDiagnostic` is pushed onto the caller's vec with `DiagnosticLevel::Warning`.
- Exit condition: AC-7, AC-8, AC-N2 satisfied.

### Step 6: Wire per-layer host-filtered dispatch guard into `layer_executor.rs`

- Task IDs:
  - `TASK-242`
- Objective: AC-9 (per-layer filter).
- Precondition: Step 5 complete.
- Postcondition: `execute_single_layer_inner` consults `module.region_split_semantics()` at line 362 (filter guard block lines 357-364) and `continue`s past the per-module loop body — skipping `instrumentation.on_module_start`, the WASM-handle lookup, `live_module` construction, and the `runner.run_stage(...)` call at line 394 — when no region's `variant_chain` on the layer matches. NO empty-polygon guard inserted — that was descoped during refinement.
- Files allowed to read:
  - `crates/slicer-runtime/src/layer_executor.rs` — RANGED lines 355-405 only (covers the per-module dispatch loop body, the filter call site at line 362, and the `runner.run_stage` call at line 394).
- Files allowed to edit (≤ 2):
  - `crates/slicer-runtime/src/layer_executor.rs` — insert the per-layer filter check at line 362 (filter block lines 357-364), BEFORE `instrumentation.on_module_start` and well before the `runner.run_stage(...)` call at line 394. The filter helper itself (`pub fn module_invocation_allowed_on_layer`) lives at the bottom of the same file at line 1326.
  - `crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs` (NEW) — synthetic two-layer scenario asserting `{(M_A, Layer_1), (M_B, Layer_1), (M_B, Layer_2)}` exactly.
- Files explicitly out-of-bounds for this step:
  - The rest of `layer_executor.rs` (lines outside 355-405). If the filter requires a structural change outside this range, ESCALATE and split the step.
  - `run_paint_annotation` (lines 626-722) and `assemble_ordered_entities` (lines 746+).
  - Any empty-polygon-guard test file (descoped).
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first error" — purpose: gate.
  - "Run `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `M`.
- Authoritative docs:
  - `docs/04_host_scheduler.md` §"Module Dispatch".
- OrcaSlicer refs: none.
- Verification: the integration test passes with the exact (declared × layer) predicate result matrix — `module_invocation_allowed_on_layer` returns `true` for `(M_A, Layer_1)`, `(M_B, Layer_1)`, `(M_B, Layer_2)` and `false` for `(M_A, Layer_2)`. The dispatch-loop wiring at line 362 is verified by code inspection; see D-92-6 in `packet.spec.md` §Deviations.
- Exit condition: AC-9 satisfied.

### Step 7: Behavior-preservation check — AC-10 byte-identical g-code (baseline-compare); AC-N1 grep

- Task IDs:
  - `TASK-242`
- Objective: confirm no production manifest changed; g-code matches Step 0 baseline byte-for-byte.
- Precondition: Steps 0-6 complete.
- Postcondition: AC-10 and AC-N1 satisfied.
- Files allowed to read:
  - `.ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md` — to retrieve `P91_BASELINE_SHA=<hex>` for the comparison.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any other.
- Expected sub-agent dispatches:
  - "Run the AC-10 baseline-compare shell command (see `packet.spec.md` AC-10): `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode && test \"$(sha256sum /tmp/p92-wedge.gcode | awk '{print $1}')\" = \"$(grep -oE 'P91_BASELINE_SHA=[a-f0-9]+' .ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md | head -1 | cut -d= -f2)\"`; return FACT exit code" — purpose: AC-10.
  - "Run `! rg -q '\\[\\[region_split\\]\\]' modules/core-modules/`; return FACT pass/fail" — purpose: AC-N1.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-10 command exits 0; AC-N1 grep PASS.
- Exit condition: AC-10, AC-N1 satisfied.

### Step 8: Guest WASM rebuild + `--check`

- Task IDs:
  - `TASK-242`
- Objective: AC-11.
- Precondition: Step 7 green.
- Postcondition: guest WASMs clean.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any source.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: `CLAUDE.md` §"Guest WASM Staleness".
- OrcaSlicer refs: none.
- Verification: PASS.
- Exit condition: AC-11 satisfied.

### Step 9: Final acceptance ceremony — narrow test gates + clippy + close TASK-242 row

- Task IDs:
  - `TASK-242`
- Objective: final gate + transition TASK-242 row from in-progress to `[x] implemented` (second touch of the Step 0.5 two-touch sequence).
- Precondition: Step 8 complete.
- Postcondition: clippy clean; slicer-scheduler + slicer-runtime integration tests all green; `docs/07_implementation_status.md` shows `- [x] TASK-242 — ... Closed <date> — packet 92.`
- Files allowed to read:
  - `docs/07_implementation_status.md` — range-read around the TASK-242 row to confirm the existing line.
- Files allowed to edit (≤ 1):
  - `docs/07_implementation_status.md`.
- Files explicitly out-of-bounds: any other.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall count".
  - Delegated edit: flip TASK-242 row to `[x]` with closure-date note.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all three test/clippy dispatches PASS; `rg -q '\[x\] TASK-242' docs/07_implementation_status.md` exits 0.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture into closure-log.md. |
| Step 0.5 | S | Open TASK-242 row in docs/07. |
| Step 1 | S | Manifest type inventory (sanity refresh of refinement findings). |
| Step 2 | S | Constants. |
| Step 3 | M | New types + 4 new `LoadErrorKind` variants + cached HashSet. |
| Step 4 | M | Validators + 6 fixtures + tests (reuse `Schema`/`TomlParse`). |
| Step 5 | M | Aggregation + `LoadDiagnostic::Warning`. |
| Step 6 | M | Per-layer dispatch filter + 1 integration test (no empty-polygon test). |
| Step 7 | S | Behavior preservation (baseline-compare). |
| Step 8 | S | Guest rebuild. |
| Step 9 | S | Workspace gate + close TASK-242 row. |

Aggregate: M (no L step).

## Packet Completion Gate

- All 11 steps complete (Steps 0, 0.5, 1-9); each exit condition satisfied.
- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-10, AC-11 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: `P91_BASELINE_SHA=<hex>` (from Step 0), `P92_POST_SHA=<hex>` (match expected), per-validator test names, the `LoadErrorKind` variants added vs reused.
- `docs/07_implementation_status.md` TASK-242 row transitioned from `[ ]` (Step 0.5) to `[x] — Closed YYYY-MM-DD — packet 92` (Step 9, delegated edit).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC verification command from `packet.spec.md`; confirm PASS / exit 0.
- Confirm clippy + targeted test buckets green.
- Confirm byte-identical g-code (AC-10 baseline-compare exits 0).
- Confirm no core module declares `[[region_split]]` (AC-N1).
- Confirm the TASK-242 row transition is recorded in `docs/07_implementation_status.md`.
- Peak context usage under 70%.
