# Implementation Plan: support-modules-paint-segment-annotations-migration

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-261`.
- Helper-first: Step 2 lands the shared `slicer_core::paint_policy` module + tests; Steps 3-5 migrate consumers to use it.
- Honors context-discipline preamble.

## Steps

### Step 1: Confirm baseline state (P95 stubs, post-P95 IR shape, intersection helper)

- Task IDs: `TASK-261`
- Objective: confirm the actual state of `support_paint_policy` in each module post-P95; locate the `SliceRegionView::segment_annotations` accessor; confirm which polygon-intersection helper exists in `slicer-core`.
- Precondition: workspace at HEAD with P95/96/97 implemented.
- Postcondition: implementer knows exactly what each migration target looks like today.
- Files allowed to read:
  - `docs/specs/support-modules-orca-port.md` §C2
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14
  - `docs/01_system_architecture.md` §"Support Stage Paint Precedence"
- Files allowed to edit (≤ 3): none in this step.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**`
  - Other paint consumers (`fuzzy-skin`, `seam-placer`)
- Expected sub-agent dispatches:
  - "Locate `SliceRegionView::segment_annotations` in `crates/slicer-sdk/src/views.rs`; return LOCATIONS + SNIPPETS ≤ 20 lines showing the accessor signature." — purpose: confirm helper input type.
  - "Confirm whether `crates/slicer-core/src/polygon_ops.rs` defines `intersection_ex` (ExPolygon-aware) or only `intersection` (flat-polygon). Return FACT (which) + file:line." — purpose: choose helper.
  - "Return current state of `fn support_paint_policy` in `modules/core-modules/tree-support/src/lib.rs`; SNIPPETS ≤ 30 lines." — purpose: confirm baseline.
  - "Return current state of `support_paint_policy` in `modules/core-modules/traditional-support/src/lib.rs`; SNIPPETS ≤ 30 lines." — purpose: confirm baseline.
  - "Return current state of `collect_paint_enforcer_contacts` + `collect_paint_blocker_polygons` in `modules/core-modules/support-planner/src/lib.rs`; SNIPPETS ≤ 60 lines combined." — purpose: confirm baseline.
  - "Return current `[ir-access].reads` values for `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`; FACT per-manifest." — purpose: manifest baseline.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C2
- OrcaSlicer refs: none.
- Verification:
  - Implementer can recite (a) the helper input type, (b) the intersection helper to use, (c) the three manifest baseline values.
- Exit condition: baseline notes captured.

### Step 2: Create `slicer_core::paint_policy` module + RED tests + implementation

- Task IDs: `TASK-261`
- Objective: create `crates/slicer-core/src/paint_policy.rs` with `SupportPaintPolicy` and `support_eligibility`, plus `crates/slicer-core/tests/paint_policy.rs` with AC-1 through AC-5. Iterate to GREEN.
- Precondition: Step 1 complete.
- Postcondition: AC-1 through AC-5 GREEN. `slicer-core` exports `paint_policy`.
- Files allowed to read:
  - `crates/slicer-sdk/src/views.rs` — accessor only
  - `crates/slicer-core/src/polygon_ops.rs` — intersection signature only
  - `docs/01_system_architecture.md` §"Support Stage Paint Precedence"
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/paint_policy.rs` (new)
  - `crates/slicer-core/src/lib.rs` (one-line module export)
  - `crates/slicer-core/tests/paint_policy.rs` (new)
- Files explicitly out-of-bounds for this step:
  - All `modules/core-modules/**` — consumed in later steps.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test paint_policy`; return FACT (per-test pass/fail); SNIPPETS ≤ 20 lines on failure." — purpose: gate AC-1 through AC-5.
  - "Run `cargo build -p slicer-core`; return FACT pass/fail." — purpose: confirm core lib still compiles.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C2
  - `docs/01_system_architecture.md` §"Support Stage Paint Precedence"
- OrcaSlicer refs: none.
- Verification:
  - AC-1 through AC-5 FACT all GREEN.
- Exit condition: shared helper exists, tested, exported.

### Step 3: Migrate `tree-support` and `traditional-support` to use the shared helper

- Task IDs: `TASK-261`
- Objective: delete `fn support_paint_policy` from both modules; import and call `slicer_core::paint_policy::support_eligibility` at the existing `run_support` call site.
- Precondition: Step 2 complete.
- Postcondition: AC-6, AC-7 grep evidence holds. Both modules compile.
- Files allowed to read:
  - `modules/core-modules/tree-support/src/lib.rs` — around the deleted call site (range)
  - `modules/core-modules/traditional-support/src/lib.rs` — same
- Files allowed to edit (≤ 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/traditional-support/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `support-planner/src/lib.rs` — Step 4 owns.
  - manifests — Step 5 owns.
- Expected sub-agent dispatches:
  - "Run `cargo build -p tree-support -p traditional-support`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: compile gate.
  - "Run `! rg -q 'fn support_paint_policy' modules/core-modules/tree-support/src/lib.rs && ! rg -q 'fn support_paint_policy' modules/core-modules/traditional-support/src/lib.rs && rg -q 'use slicer_core::paint_policy::' modules/core-modules/tree-support/src/lib.rs && rg -q 'use slicer_core::paint_policy::' modules/core-modules/traditional-support/src/lib.rs`; return FACT pass/fail." — purpose: AC-6 + AC-7 gate.
- Context cost: `S`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-6 + AC-7 FACT PASS.
- Exit condition: both modules use the shared helper.

### Step 4: Migrate `support-planner` contact extraction to `segment_annotations`

- Task IDs: `TASK-261`
- Objective: replace `collect_paint_enforcer_contacts` and `collect_paint_blocker_polygons` with new functions that source contacts/polygons from per-region `segment_annotations`. Update `plan_for_object` call sites.
- Precondition: Step 3 complete.
- Postcondition: AC-8 grep evidence holds. Planner compiles and existing planner unit tests pass.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` — around the two functions (range)
  - `crates/slicer-sdk/src/views.rs::SliceRegionView::segment_annotations` — accessor only
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - manifest — Step 5 owns.
  - other modules — not touched here.
- Expected sub-agent dispatches:
  - "Run `cargo build -p support-planner`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: compile gate.
  - "Run `cargo test -p support-planner`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: existing planner unit tests don't regress.
  - "Run `! rg -q 'paint_layers\\.facet_values' modules/core-modules/support-planner/src/lib.rs && rg -q 'segment_annotations' modules/core-modules/support-planner/src/lib.rs`; return FACT pass/fail." — purpose: AC-8 gate.
- Context cost: `M`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-8 FACT PASS.
  - `cargo test -p support-planner` FACT PASS.
- Exit condition: planner reads new shape; existing tests still pass.

### Step 5: Update three manifests' `[ir-access].reads`; rebuild guests

- Task IDs: `TASK-261`
- Objective: drop `"PaintRegionIR"` from `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`; add the post-P95 source declared by Step 1's manifest-baseline dispatch (likely `"SliceIR"` — confirm).
- Precondition: Step 4 complete; Step 1 captured the target source key.
- Postcondition: AC-9 grep evidence holds. `cargo xtask build-guests --check` reports clean.
- Files allowed to read:
  - the three manifests
- Files allowed to edit (≤ 3):
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out-of-bounds for this step:
  - module source — not touched here.
- Expected sub-agent dispatches:
  - "Run AC-9 multiline grep; return FACT pass/fail." — purpose: manifest gate.
  - "Run `cargo xtask build-guests`; return FACT pass/fail. Do NOT paste rebuild log." — purpose: rebuild after manifest changes.
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`)." — purpose: rebuild gate.
- Context cost: `S`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-9 FACT PASS.
  - `cargo xtask build-guests --check` FACT `up to date`.
- Exit condition: manifests aligned; guests fresh.

### Step 6: Author live integration tests AC-10, AC-N1, AC-N2

- Task IDs: `TASK-261`
- Objective: extend `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` with three new test functions exercising the end-to-end behavior (paint kernel produces segment_annotations → support module reads them → emission decision matches expected).
- Precondition: Step 5 complete; guests fresh; planner emission path uses segment_annotations.
- Postcondition: AC-10, AC-N1, AC-N2 GREEN.
- Files allowed to read:
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — range-read existing setup pattern
  - `crates/slicer-runtime/tests/common/` — fixture helpers (delegated)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`
  - (optional) a new fixture file under `resources/test_models/` if no existing fixture has the right paint configuration — confirm in Step 1's `[FWD]` open question
- Files explicitly out-of-bounds for this step:
  - other test files — not extended here.
- Expected sub-agent dispatches:
  - "Confirm whether `resources/bridge_support_enforcers.3mf` (or equivalent) carries painted support-enforcer/blocker annotations; return FACT yes/no + 1-line description." — purpose: fixture decision.
  - "Run `cargo test -p slicer-runtime --test live_layer_support_tdd -- enforcer_forces_support_against_classification blocker_suppresses_support_against_classification no_paint_no_classification_no_support`; return FACT pass/fail per-test; SNIPPETS ≤ 30 lines on failure." — purpose: AC-10 / N1 / N2 gate.
- Context cost: `M`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-10, AC-N1, AC-N2 FACT all PASS.
- Exit condition: integration confirms the live path works.

### Step 7: Doc Impact + Final packet verification

- Task IDs: `TASK-261`
- Objective: update `docs/05_module_sdk.md` per Doc Impact Statement; re-dispatch the AC matrix; lint.
- Precondition: Steps 2-6 complete.
- Postcondition: all ACs PASS; workspace clippy clean; Doc Impact grep PASS.
- Files allowed to read:
  - `docs/05_module_sdk.md` — locate the "Shared helpers" section (delegate LOCATIONS if > 300 lines)
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
- Files explicitly out-of-bounds for this step:
  - `target/**`
- Expected sub-agent dispatches:
  - "Locate the 'Shared helpers' section in `docs/05_module_sdk.md`; return LOCATIONS ≤ 3 entries." — purpose: insertion point.
  - "Run AC-1 through AC-10 + AC-N1 + AC-N2 commands sequentially; return FACT (PASS / FAIL list)." — packet gate.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — lint gate.
  - "Run `rg -q 'slicer_core::paint_policy::support_eligibility' docs/05_module_sdk.md`; return FACT pass/fail." — Doc Impact gate.
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- OrcaSlicer refs: none.
- Verification:
  - Full AC matrix PASS.
  - Workspace clippy PASS.
  - Doc Impact grep PASS.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery dispatches. |
| Step 2 | M | New helper + tests. |
| Step 3 | S | Two consumers swap. |
| Step 4 | M | Planner extraction rewrite. |
| Step 5 | S | Three TOMLs + guest rebuild. |
| Step 6 | M | Integration tests + fixture decision. |
| Step 7 | S | Doc + final verification. |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All seven steps complete; each exit condition met.
- AC-1 through AC-10 + AC-N1 + AC-N2 PASS.
- Doc Impact Statement satisfied.
- `cargo xtask build-guests --check` clean.
- `docs/07_implementation_status.md` marks `TASK-261` `[x]` (via worker dispatch).
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands: `cargo xtask build-guests --check`, `cargo build --workspace`, the three test commands, `cargo clippy --workspace -- -D warnings`.
- Mark `TASK-261` `[x]`; transition `packet.spec.md` to `status: implemented`.
