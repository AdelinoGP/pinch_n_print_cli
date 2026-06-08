# Implementation Plan: 92_region-split-manifest-and-dispatch

## Execution Rules

- One atomic step at a time.
- TDD for validators: write the test fixture + the failing assertion first, then implement the validator.
- Test output teed to `target/test-output.log`.
- Pre-packet baseline SHA on `regression_wedge.stl` must be captured (Step 0) — AC-11 depends on it.

## Steps

### Step 0: Capture pre-packet baseline g-code SHA

- Task IDs:
  - `TASK-242`
- Objective: record byte-identical baseline before any edit (AC-11 prerequisite).
- Precondition: P91 closed; working tree at its parent commit.
- Postcondition: baseline SHA recorded.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-baseline.gcode && sha256sum /tmp/p92-baseline.gcode`; return FACT (sha256 hash)".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: FACT returned a single hash.
- Exit condition: hash recorded.

### Step 1: Inventory manifest types crate + error enum location

- Task IDs:
  - `TASK-242`
- Objective: locate the manifest TOML parser, `ManifestEntry`, and `ManifestParseError` enum.
- Precondition: Step 0 complete.
- Postcondition: file:line for each landed in implementer's notes.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `rg -nE 'pub struct ManifestEntry|ManifestParseError|fn parse_manifest' crates/slicer-scheduler/src/`; return LOCATIONS (≤ 15 entries)".
  - "Run `rg -nE 'pub mod manifest|pub use manifest' crates/slicer-scheduler/src/`; return LOCATIONS" — purpose: confirm the manifest module's export path.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: LOCATIONS non-empty.
- Exit condition: file paths recorded.

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

### Step 3: Add `RegionSplitDeclaration`, `RegionSplitValueType`, `ManifestEntry.region_splits`, and the 4 new `ManifestParseError` variants

- Task IDs:
  - `TASK-242`
- Objective: enable parsing + structured error reporting.
- Precondition: Step 2 complete.
- Postcondition: types exist; default-empty `region_splits` deserialize cleanly on manifests with no `[[region_split]]` section; the error enum has the 4 new variants.
- Files allowed to read:
  - `crates/slicer-scheduler/src/manifest.rs` (or located file from Step 1) — read the existing `ManifestEntry` def + nearby `ManifestParseError`.
- Files allowed to edit (≤ 3):
  - The manifest types file located in Step 1.
- Files explicitly out-of-bounds for this step:
  - Other scheduler files; runtime files.
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-scheduler`; return FACT pass/fail with first error" — purpose: gate.
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` §"Manifest TOML Schema".
- OrcaSlicer refs: none.
- Verification: `cargo check` clean; an inline `#[test]` parsing the minimal `basic.toml` succeeds.
- Exit condition: types defined; default-empty parse works.

### Step 4: Write failing-validator tests (TDD); implement validators; pass tests

- Task IDs:
  - `TASK-242`
- Objective: AC-1, AC-3, AC-4, AC-5, AC-6, AC-N3.
- Precondition: Step 3 complete.
- Postcondition: 6 validator branches green.
- Files allowed to read:
  - `crates/slicer-scheduler/src/manifest.rs` — full read OK if small.
- Files allowed to edit (≤ 3 per commit):
  - `crates/slicer-scheduler/src/manifest.rs` — add validation logic.
  - `crates/slicer-scheduler/tests/fixtures/region_split_manifests/*.toml` — 6 new tiny TOML files (CREATE).
  - `crates/slicer-scheduler/tests/region_split_manifest_tdd.rs` — new test file (CREATE).
- Files explicitly out-of-bounds for this step:
  - `slicer-runtime` source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-scheduler region_split 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: gate.
- Context cost: `M`.
- Authoritative docs: roadmap §"P1b" validator subsection.
- OrcaSlicer refs: none.
- Verification: each of the 6 tests passes; the error-message content matches the AC's described shape.
- Exit condition: AC-1, AC-3, AC-4, AC-5, AC-6, AC-N3 satisfied.

### Step 5: Implement `aggregate_region_splits` + tied-priority WARN + canonical-order accessor

- Task IDs:
  - `TASK-242`
- Objective: AC-7, AC-8, AC-N2.
- Precondition: Step 4 green.
- Postcondition: aggregation function returns BTreeMap in canonical order; tied priorities emit a WARN event; empty-input case yields empty BTreeMap.
- Files allowed to read:
  - `docs/09_progress_events.md` — read in full only if needed (≤ 200 lines; SUMMARY-dispatch acceptable).
  - The existing event-emission helper (locate via `Grep`).
- Files allowed to edit (≤ 3):
  - `crates/slicer-scheduler/src/region_split.rs` (NEW).
  - `crates/slicer-scheduler/src/lib.rs` — `pub mod region_split;` declaration.
  - `crates/slicer-scheduler/tests/region_split_aggregation_tdd.rs` (NEW) — tests for AC-7, AC-8, AC-N2.
- Files explicitly out-of-bounds for this step:
  - Runtime files (Step 6).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-scheduler region_split_aggregation 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: gate.
  - "Locate the structured-event emission helper in `crates/slicer-scheduler/src/` or `crates/slicer-runtime/src/`; return FILE:LINE" — purpose: pinpoint WARN emission entry.
- Context cost: `M`.
- Authoritative docs:
  - `docs/09_progress_events.md` for WARN-event shape.
- OrcaSlicer refs: none.
- Verification: 3 tests pass.
- Exit condition: AC-7, AC-8, AC-N2 satisfied.

### Step 6: Wire host-filtered dispatch + empty-polygon guard into `layer_executor.rs`

- Task IDs:
  - `TASK-242`
- Objective: AC-9, AC-10.
- Precondition: Step 5 complete.
- Postcondition: layer executor consults each module's declared semantics before invocation; empty-polygon regions skip all module invocation.
- Files allowed to read:
  - `crates/slicer-runtime/src/layer_executor.rs` — RANGED lines 480-540 only.
  - `crates/slicer-runtime/src/lib.rs` or wherever `ModuleMetadata` is defined — RANGED.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/layer_executor.rs`.
  - The `ModuleMetadata` definition file — add `region_splits: HashSet<String>` (or pre-computed accessor) cached at module-load.
  - `crates/slicer-runtime/tests/integration/region_split_dispatch_filter.rs` (NEW).
  - `crates/slicer-runtime/tests/integration/empty_polygon_dispatch_guard.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - The rest of `layer_executor.rs`. If the filter requires a structural change outside the 480-540 range, ESCALATE and split the step.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first error" — purpose: gate.
  - "Run `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test integration empty_polygon_dispatch_guard 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `M`.
- Authoritative docs:
  - `docs/04_host_scheduler.md` §"Module Dispatch".
- OrcaSlicer refs: none.
- Verification: both integration tests pass.
- Exit condition: AC-9, AC-10 satisfied.

### Step 7: Behavior-preservation check — AC-11 byte-identical g-code; AC-N1 grep

- Task IDs:
  - `TASK-242`
- Objective: confirm no production manifest changed; g-code matches Step 0 baseline.
- Precondition: Steps 0-6 complete.
- Postcondition: AC-11 and AC-N1 satisfied.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-post.gcode && sha256sum /tmp/p92-post.gcode`; return FACT (sha256)" — purpose: AC-11.
  - "Run `! rg -q '\[\[region_split\]\]' modules/core-modules/`; return FACT pass/fail" — purpose: AC-N1.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: sha256 matches Step 0 baseline; AC-N1 grep PASS.
- Exit condition: AC-11, AC-N1 satisfied.

### Step 8: Guest WASM rebuild + `--check`

- Task IDs:
  - `TASK-242`
- Objective: AC-12.
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
- Exit condition: AC-12 satisfied.

### Step 9: Final acceptance ceremony — narrow test gates + clippy

- Task IDs:
  - `TASK-242`
- Objective: final gate.
- Precondition: Step 8 complete.
- Postcondition: clippy clean; slicer-scheduler + slicer-runtime integration tests all green.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall count".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all three PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture. |
| Step 1 | S | Manifest inventory dispatch. |
| Step 2 | S | Constants. |
| Step 3 | M | New types + error variants. |
| Step 4 | M | Validators + 6 fixtures + tests. |
| Step 5 | M | Aggregation + WARN. |
| Step 6 | M | Dispatch hook + 2 integration tests. |
| Step 7 | S | Behavior preservation check. |
| Step 8 | S | Guest rebuild. |
| Step 9 | S | Workspace gate. |

Aggregate: M (no L step).

## Packet Completion Gate

- All 10 steps complete; each exit condition satisfied.
- AC-1 through AC-12 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: baseline SHA, post-packet SHA (match), per-validator test names.
- `docs/07_implementation_status.md` updated for `TASK-242` (delegate edit).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC verification command; confirm PASS.
- Confirm clippy + targeted test buckets green.
- Confirm byte-identical g-code (AC-11).
- Confirm no core module declares `[[region_split]]` (AC-N1).
- Peak context usage under 70%.
