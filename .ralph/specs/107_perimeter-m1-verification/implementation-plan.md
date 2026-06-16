# Implementation Plan: 107_perimeter-m1-verification

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: T-100 — Build the parity harness with self-test

- Task IDs:
  - `T-100` — Build reference-fixture parity harness
- Objective: implement `crates/slicer-runtime/tests/integration/perimeter_parity.rs` containing the harness + per-field tolerance comparator + a self-test that constructs a synthetic mismatch and asserts the comparator detects it.
- Precondition: P102..P106 implementations landed (status: implemented); workspace builds clean.
- Postcondition: AC-1 + AC-N1 pass.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — range-read `PerimeterIR` + `PerimeterRegion` + `WallLoop` definitions for the comparator.
  - `crates/slicer-runtime/tests/common/mod.rs` (if it exists) — discover existing test helpers.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (NEW; includes harness + self-test)
  - `crates/slicer-runtime/tests/integration/main.rs` (register the new test target if needed)
- Files explicitly out-of-bounds: all fixtures (Step 2); other tests.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test`; FACT pass/fail."
- Context cost: `M` (harness implementation + comparator + self-test)
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-100 row.
- OrcaSlicer refs: none for Step 1 (the harness is workspace-internal).
- Verification: `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1 + AC-N1 green; harness compiles standalone.

### Step 2: T-101 — Record 6 reference fixtures

- Task IDs:
  - `T-101` — Record OrcaSlicer reference outputs for 6 M1 fixtures
- Objective: for each of {solid_square, holed_square, multi_tool_triangle, overhang_ramp, bridge, spiral_vase_cone}, dispatch an OrcaSlicer SUMMARY for expected `PerimeterIR` shape, hand-author `mesh.stl` (or similar) + `config.toml` + `expected_perimeter_ir.json` under `crates/slicer-runtime/tests/fixtures/perimeter_parity/<name>/`, and verify the harness loads each.
- Precondition: Step 1 exit condition met.
- Postcondition: AC-2 passes for all 6 fixtures.
- Files allowed to read:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (just authored in Step 1).
- Files allowed to edit (≤ 3 per sub-step):
  - 2a-2f (one sub-step per fixture): create the 3 files under `tests/fixtures/perimeter_parity/<fixture_name>/`. Maximum 3 files per sub-step.
- Files explicitly out-of-bounds: source code (Steps 5+).
- Expected sub-agent dispatches:
  - 6× "Summarize expected `PerimeterIR` shape for <fixture> (mesh + config); SUMMARY ≤ 100 words. No code." (Implementer can dispatch in parallel via single message with 6 sub-agent calls.)
  - "Run `cargo test -p slicer-runtime --test integration perimeter_parity`; FACT pass/fail per fixture."
- Context cost: `M` (6 dispatches + 6 fixture authorings; manageable if dispatched in parallel)
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-101 row.
- OrcaSlicer refs: per-fixture SUMMARYs (≤ 100 words each).
- Verification: `cargo test -p slicer-runtime --test integration perimeter_parity 2>&1 | tee target/test-output.log` — FACT per fixture.
- Exit condition: AC-2 green for all 6 fixtures.

### Step 3: T-102 — Edge-case TDD sweep

- Task IDs:
  - `T-102` — TDD sweep for 7 edge cases
- Objective: implement `crates/slicer-runtime/tests/integration/perimeter_edge_cases.rs` containing 7 distinct `#[test]` functions, one per audit case (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override).
- Precondition: Step 2 exit condition met.
- Postcondition: AC-3 passes (all 7 cases).
- Files allowed to read:
  - Both perimeter modules' `lib.rs` (range-read to confirm expected behavior for each case).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/perimeter_edge_cases.rs` (NEW)
  - `crates/slicer-runtime/tests/integration/main.rs` (register if needed)
- Files explicitly out-of-bounds: perimeter modules' source (verification only, no fixing here).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration perimeter_edge_cases`; FACT pass/fail per case."
- Context cost: `M` (7 small TDDs in one file)
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-102 row.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test integration perimeter_edge_cases 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-3 green.

### Step 4: T-P96-A — Rename + reshape `cube_4color_gcode_output_tdd`

- Task IDs:
  - `T-P96-A` — Reshape AC-22b assertion + rename test
  - `T-P96-C3` — Golden-file parity for cube_4color (lands as part of this step's reshape)
- Objective: rename the existing test function to `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes`; rewrite the assertion to match ADR-0013 (per-color fragmentation, tool changes between fragments, union coverage exact).
- Precondition: Step 3 exit condition met; P105's MMU consumption is green (T-P96-C1/C2).
- Postcondition: AC-4 passes.
- Files allowed to read:
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` — full (≤ 300 lines).
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — read full.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (rename + reshape)
- Files explicitly out-of-bounds: `cube_4color_paint_tdd.rs` (separate file, not in scope).
- Expected sub-agent dispatches:
  - "Find call sites or doc references to `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`; LOCATIONS ≤ 10 entries."
  - "Run `cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes`; FACT pass/fail."
- Context cost: `S` (one file rename + reshape)
- Authoritative docs: `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`.
- OrcaSlicer refs: cited in ADR-0013; no new dispatch.
- Verification: `cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-4 green; old function name not referenced anywhere.

### Step 5: T-P96-D — Delete `external_contour` IR field

- Task IDs:
  - `T-P96-D` — Delete unused `external_contour` field across ~5 files; schema bump 4.3.0 → 4.4.0
- Objective: remove `external_contour` from `SlicedRegion` IR, WIT, host populator, SDK view, and the paint-segmentation `union_ex` computation call site; bump schema; keep `#[serde(default)]` on the now-absent field to parse old fixtures.
- Precondition: Step 4 exit condition met; LOCATIONS dispatch confirms zero remaining callers.
- Postcondition: AC-5 passes; `cargo build --tests --workspace` clean; no STALE guests.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'external_contour'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full.
  - `crates/slicer-wasm-host/src/host.rs` — range-read.
  - `crates/slicer-sdk/src/views.rs` — range-read.
  - `crates/slicer-core/src/algos/paint_segmentation/<file>.rs` — range-read by `rg -n 'external_contour|union_ex'`.
- Files allowed to edit (≤ 3 per sub-step):
  - 5a (IR + WIT): `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`.
  - 5b (host + view): `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-sdk/src/views.rs`.
  - 5c (paint-seg): `crates/slicer-core/src/algos/paint_segmentation/<file>.rs`.
- Files explicitly out-of-bounds: perimeter modules (P105's revert is canonical; no further edits).
- Expected sub-agent dispatches:
  - "Find all callers of `region.external_contour()` or field reads `SlicedRegion.external_contour` across the workspace; LOCATIONS ≤ 10 entries (expected zero)."
  - "Run `cargo build --tests --workspace`; FACT pass/fail."
  - "Run `cargo xtask build-guests --check`; FACT clean / STALE list."
- Context cost: `M` (5 files; schema bump + WIT removal + cascade)
- Authoritative docs: `docs/02_ir_schemas.md` schema-versioning; `CLAUDE.md` WIT checklist.
- OrcaSlicer refs: none.
- Verification:
  - `! rg -q 'external_contour' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `! rg -q 'external-contour' crates/slicer-schema/wit/deps/ir-types.wit` — exit 0.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT.
  - `cargo xtask build-guests --check` — no STALE.
- Exit condition: AC-5 green; cascade complete; guests not stale.

### Step 6: T-103/T-104/T-P96-F — Closure pass + M1 marker

- Task IDs:
  - `T-103` — Walk every M1 deviation; close or justify
  - `T-104` — Mark Classic parity complete in `docs/07_implementation_status.md` + flip roadmap M1 marker
  - `T-P96-F` — Capture cube_4color SHA + register `D-<packet>-AC22-PARITY-RESHAPE` superseding `D-96-AC22-EXTERNAL-CONTOUR`
- Objective: close-stamp every M1 deviation entry; update status doc; flip M1 marker in roadmap; capture cube_4color SHA + register PARITY-RESHAPE deviation.
- Precondition: Step 5 exit condition met.
- Postcondition: AC-6 passes; all Doc Impact Statement greps pass.
- Files allowed to read:
  - `docs/DEVIATION_LOG.md` — range-read recent M1 entries.
  - `docs/07_implementation_status.md` — range-read current state.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read Milestone Summary.
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- Files explicitly out-of-bounds: `docs/02_ir_schemas.md` schema entry — handled with Step 5's edit; if a final ratification edit is needed it goes here in sub-step 6b.
- Expected sub-agent dispatches:
  - "Capture the SHA of the current `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` test output (e.g., via `cargo test … && sha256sum target/...`); FACT (SHA string)."
  - "For each Doc Impact grep, run `rg -q`; FACT pass/fail."
- Context cost: `S` (three doc edits; one SHA capture)
- Authoritative docs: the three docs being edited.
- OrcaSlicer refs: none.
- Verification:
  - All five Doc Impact Statement greps return hits.
  - `rg -q 'D-96-AC22-EXTERNAL-CONTOUR.*superseded' docs/DEVIATION_LOG.md` — exit 0.
- Exit condition: AC-6 green; status doc + roadmap reflect M1 close.

### Step 7: T-105 — Workspace test ceremony

- Task IDs:
  - `T-105` — Run `cargo test --workspace` at M1 close (CLAUDE.md exception)
- Objective: run the full workspace test suite as the M1 closure ceremony.
- Precondition: Step 6 exit condition met; all per-target tests green; clippy clean; no STALE guests.
- Postcondition: AC-7 passes (workspace suite green).
- Files allowed to read: none.
- Files allowed to edit: none (test run only).
- Expected sub-agent dispatches:
  - "Run `cargo test --workspace 2>&1 | tee target/test-output.log`; return FACT (last 5 lines of output containing pass count + any failures)."
- Context cost: `S` (test execution only; no file edits).
- Authoritative docs: `CLAUDE.md` §"Test Discipline" — confirms exception.
- OrcaSlicer refs: none.
- Verification: `tail -5 target/test-output.log` shows "test result: ok" for every test target.
- Exit condition: AC-7 green; M1 closure ceremony complete.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Harness implementation + comparator + self-test. |
| Step 2 | M | 6 SUMMARYs + 6 fixture authorings; parallel-dispatchable. |
| Step 3 | M | 7 TDDs in one file. |
| Step 4 | S | One-file rename + reshape. |
| Step 5 | M | 5-file IR-removal cascade; schema bump + guest WASM gate. |
| Step 6 | S | Three doc edits + SHA capture. |
| Step 7 | S | Workspace test execution only. |

Aggregate context cost: `M`. No step `L`. Per-step file edit count ≤ 3 (Step 5 splits into sub-steps a/b/c).

## Packet Completion Gate

- All seven steps complete; each exit condition met.
- AC-1 through AC-7 + AC-N1 all PASS.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` Classic parity entry committed.
- `packet.spec.md` ready to move `draft` → `implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm the three gate commands green.
- Record the captured cube_4color SHA in the closure log and in `docs/DEVIATION_LOG.md` as `P107_CUBE_4COLOR_PARITY_SHA` (or `P<packet>_CUBE_4COLOR_PARITY_SHA` matching the packet number).
- Record the OrcaSlicer commit-hash (or version) the 6 fixture SUMMARYs were derived from.
- Confirm M1 close: `docs/07_implementation_status.md` reflects "M1 — Classic perimeter parity (P102..P107): complete".
- Confirm implementer's peak context usage < 70%.
