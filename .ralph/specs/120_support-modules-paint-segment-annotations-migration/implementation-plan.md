# Implementation Plan: support-modules-paint-segment-annotations-migration

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-285` (renumbered from source-plan `TASK-261`).
- Helper-first: Step 2 lands the shared `slicer_core::paint_policy` module + tests; Step 3 refactors the SDK wrapper; Steps 4-6 clean manifests + host shim.
- TDD on the geometric regression: Step 4 authors the L-shape regression test as RED, Step 6 confirms GREEN.
- Honors context-discipline preamble.

## Steps

### Step 1: Confirm baseline state (current `paint_policy_for`, host shim, intersection helper)

- Task IDs: `TASK-285`
- Objective: confirm the actual state of `paint_policy_for` in `crates/slicer-sdk/src/traits.rs`; locate the `SliceRegionView::segment_annotations` accessor; confirm which polygon-intersection helper exists in `slicer-core`; audit guest-side `regions_by_semantic` callers for kebab-case dependency.
- Precondition: workspace at HEAD with TASK-245 + TASK-246 implemented.
- Postcondition: implementer knows exactly what the migration target looks like today, and whether Step 6's kebab→snake cleanup is safe.
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
  - "Return current state of `fn paint_policy_for` in `crates/slicer-sdk/src/traits.rs`; SNIPPETS ≤ 60 lines (function body + the two helpers `expolygon_centroid` and `regions_cover_point`)." — purpose: confirm baseline.
  - "Return current state of `HostPaintRegionLayerView` impl in `crates/slicer-wasm-host/src/host.rs` lines 3054-3120; SNIPPETS ≤ 70 lines." — purpose: confirm Step 6 baseline.
  - "Confirm `rg -c 'expolygon_centroid|regions_cover_point' crates/` returns 0 callers outside the helpers themselves; return FACT." — purpose: confirm safe-to-delete.
  - "Search `crates/slicer-wasm-host/test-guests/` for any `regions_by_semantic.get` or `get_regions` calls; return LOCATIONS ≤ 20 entries. If any use kebab-case keys (`"support-enforcer"`, `"support-blocker"`, `"fuzzy-skin"`), flag them." — purpose: confirm Step 6 kebab→snake is safe.
  - "Search `crates/` for any test that asserts `support-planner` reads `PaintRegionIR`; return LOCATIONS ≤ 10 entries." — purpose: confirm Step 5 manifest cleanup is safe.
  - "Return current `[ir-access].reads` values for `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`; FACT per-manifest." — purpose: manifest baseline.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C2
- OrcaSlicer refs: none.
- Verification:
  - Implementer can recite (a) the helper input type, (b) the intersection helper to use, (c) the current `paint_policy_for` body shape, (d) whether guest-side kebab-case keys exist, (e) the three manifest baseline values.
- Exit condition: baseline notes captured; no [BLOCK] open questions.

### Step 2: Create `slicer_core::paint_policy` module + RED tests + implementation

- Task IDs: `TASK-285`
- Objective: create `crates/slicer-core/src/paint_policy.rs` with `SupportPaintPolicy` (re-exported alias) and `support_eligibility`, plus `crates/slicer-core/tests/paint_policy.rs` with AC-1 through AC-5 + AC-N3. Iterate to GREEN.
- Precondition: Step 1 complete.
- Postcondition: AC-1 through AC-5 + AC-N3 GREEN. `slicer-core` exports `paint_policy`. The `SupportPaintPolicy` enum is a `pub use slicer_sdk::traits::SupportPaintPolicy;` re-export so consumer match arms don't change.
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
  - `crates/slicer-sdk/src/traits.rs` — Step 3 owns.
  - `crates/slicer-wasm-host/src/host.rs` — Step 6 owns.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test paint_policy`; return FACT (per-test pass/fail); SNIPPETS ≤ 20 lines on failure." — purpose: gate AC-1 through AC-5 + AC-N3.
  - "Run `cargo build -p slicer-core`; return FACT pass/fail." — purpose: confirm core lib still compiles.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C2
  - `docs/01_system_architecture.md` §"Support Stage Paint Precedence"
- OrcaSlicer refs: none.
- Verification:
  - AC-1 through AC-5 + AC-N3 FACT all GREEN.
- Exit condition: shared helper exists, tested, exported, re-exported.

### Step 3: Refactor `crates/slicer-sdk/src/traits.rs::paint_policy_for` to thin wrapper; delete centroid helpers

- Task IDs: `TASK-285`
- Objective: replace the centroid-based body of `paint_policy_for` (lines 172-204) with a wrapper that iterates `SliceIR.regions`, calls `slicer_core::paint_policy::support_eligibility` per region, and aggregates with blocker-wins precedence. Delete `expolygon_centroid` (line 220) and `regions_cover_point` (line 238).
- Precondition: Step 2 complete.
- Postcondition: AC-6 grep evidence holds. `slicer-sdk` compiles. The two consumer modules' `match` arms compile unchanged (the enum is unchanged).
- Files allowed to read:
  - `crates/slicer-sdk/src/traits.rs` — lines 165-250 only
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/traits.rs`
- Files explicitly out-of-bounds for this step:
  - manifests — Step 5 owns.
  - host shim — Step 6 owns.
  - the two consumer modules — they don't change in this packet (only their import path stays the same; the helper moved, not the call shape).
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: compile gate.
  - "Run `! rg -q 'expolygon_centroid|regions_cover_point' crates/slicer-sdk/src/traits.rs && rg -q 'slicer_core::paint_policy::support_eligibility' crates/slicer-sdk/src/traits.rs`; return FACT pass/fail." — purpose: AC-6 gate.
  - "Run `cargo build -p tree-support -p traditional-support`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: confirm consumer modules still compile (their match arms didn't change).
- Context cost: `M`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-6 FACT PASS.
  - Consumer modules compile FACT PASS.
- Exit condition: SDK wrapper live; centroid helpers gone; consumer modules unaffected.

### Step 4: Author L-shape regression test as RED in both `enforcer_blocker_tdd.rs` files

- Task IDs: `TASK-285`
- Objective: add `fn enforcer_works_when_centroid_outside_paint_region` to BOTH `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` and `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs`. The test uses an L-shaped expoly whose vertex-mean centroid lies provably outside the painted region; the test must FAIL against the pre-Step-3 `paint_policy_for` (RED). The existing 8 tests in each file continue to pass.
- Precondition: Step 3 complete; consumer modules' import path unchanged.
- Postcondition: AC-8 RED state confirmed on Step 3's wrapper body. (If GREEN on first try, the implementer has chosen an L-shape whose centroid lies inside the painted region; pick a different L-shape.)
- Files allowed to read:
  - `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` — existing test pattern (range-read the `enclosing_square()` helper at lines 56-68 and one of the 8 existing tests to copy the setup pattern)
  - `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` — same
- Files allowed to edit (≤ 3):
  - `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs`
  - `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - planner — Step 5 owns.
  - host shim — Step 6 owns.
  - The SDK traits.rs — already refactored in Step 3.
- Expected sub-agent dispatches:
  - "Run `cargo test -p tree-support --test enforcer_blocker_tdd -- enforcer_works_when_centroid_outside_paint_region`; return FACT (expected: FAIL on pre-Step-3 logic; will become FAIL with current code if the new test's centroid is provably outside the painted region)." — purpose: AC-8 RED gate.
  - "Run `cargo test -p traditional-support --test enforcer_blocker_tdd -- enforcer_works_when_centroid_outside_paint_region`; return FACT (expected: FAIL)." — purpose: AC-8 mirror RED gate.
  - "Run `cargo test -p tree-support --test enforcer_blocker_tdd` (all 9 tests); return FACT (the 8 existing pass; the new one fails)." — purpose: existing 8 tests don't regress.
  - "Run `cargo test -p traditional-support --test enforcer_blocker_tdd` (all 9 tests); return FACT." — purpose: existing 8 tests don't regress.
- Context cost: `S`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-8 RED: the new test FAILS, the 8 existing tests PASS.
  - The test comment block records the computed centroid coordinate so the test author can verify the L-shape is correctly chosen.
- Exit condition: RED state confirmed on both modules' `enforcer_blocker_tdd.rs`.

### Step 5: Update three manifests' `[ir-access].reads` (drop `"PaintRegionIR"`)

- Task IDs: `TASK-285`
- Objective: drop `"PaintRegionIR"` from `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`. The post-P95 sources (`"SliceIR"` for tree/traditional; `"MeshIR"` for planner) are already declared.
- Precondition: Step 3 complete.
- Postcondition: AC-9 grep evidence holds. `cargo xtask build-guests --check` reports clean (if the manifest change requires a rebuild, the implementer runs without `--check` first).
- Files allowed to read:
  - the three manifests
- Files allowed to edit (≤ 3):
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out-of-bounds for this step:
  - module source — not touched here.
  - host shim — Step 6 owns.
- Expected sub-agent dispatches:
  - "Run AC-9 multiline grep (`for m in tree-support traditional-support support-planner; do ! rg -q 'PaintRegionIR' modules/core-modules/$m/$m.toml || exit 1; done`); return FACT pass/fail." — purpose: manifest gate.
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`). If STALE, run `cargo xtask build-guests` (without --check) and re-run the check." — purpose: WASM gate.
- Context cost: `S`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-9 FACT PASS.
  - `cargo xtask build-guests --check` FACT `up to date`.
- Exit condition: manifests aligned; guests fresh.

### Step 6: Clean `HostPaintRegionLayerView` host shim (kebab→snake; drop dead `runtime_reads.push`)

- Task IDs: `TASK-285`
- Objective: in `crates/slicer-wasm-host/src/host.rs::HostPaintRegionLayerView` (lines 3054-3120), drop the three `self.runtime_reads.push(String::from("PaintRegionIR"))` calls and replace the kebab-case semantic-name keys (`"support-enforcer"`, `"support-blocker"`, `"fuzzy-skin"`, and any other kebab-case key) with snake_case (`"support_enforcer"`, `"support_blocker"`, `"fuzzy_skin"`). If Step 1's audit found test-guests that read kebab-case keys, update them in this step too.
- Precondition: Step 5 complete; Step 1 audit confirmed whether guest-side kebab-case keys exist.
- Postcondition: AC-N4 grep evidence holds. `slicer-wasm-host` and the updated test-guests compile; `cargo test -p slicer-wasm-host` is clean.
- Files allowed to read:
  - `crates/slicer-wasm-host/src/host.rs` — lines 3054-3120 only
  - any test-guest file flagged by Step 1's audit (range-read)
- Files allowed to edit (≤ 3):
  - `crates/slicer-wasm-host/src/host.rs`
  - test-guest source files flagged by Step 1 audit (if any)
- Files explicitly out-of-bounds for this step:
  - manifests — Step 5 owns.
  - other host shims — out of scope.
- Expected sub-agent dispatches:
  - "Run `! rg -q 'PaintRegionIR' crates/slicer-wasm-host/src/host.rs && ! rg -q '"support-enforcer"|"support-blocker"|"fuzzy-skin"' crates/slicer-wasm-host/src/host.rs`; return FACT pass/fail." — purpose: AC-N4 gate.
  - "Run `cargo build -p slicer-wasm-host`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: host compile gate.
  - "Run `cargo test -p slicer-wasm-host`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: host test gate (catches any guest→host dispatch contract regression).
- Context cost: `S`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-N4 FACT PASS.
  - `cargo test -p slicer-wasm-host` FACT PASS.
- Exit condition: host shim cleaned; contract tests still pass.

### Step 7: Confirm AC-8 GREEN; run live integration tests; re-anchor L-shape test

- Task IDs: `TASK-285`
- Objective: confirm that the L-shape regression test from Step 4 now PASSES (GREEN) against the Step-3 wrapper. Then run the existing live integration tests `enforcer_forces_live_support_commit_even_when_needs_support_is_false`, `blocker_overrides_needs_support_true_at_commit_level`, and `disabled_or_ineligible_support_stage_commits_empty_support_ir` to confirm AC-10 / AC-N1 / AC-N2 still pass.
- Precondition: Steps 3, 4, 5, 6 complete.
- Postcondition: AC-8, AC-10, AC-N1, AC-N2 all GREEN.
- Files allowed to read:
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — lines 200-380 (the three test bodies) only
- Files allowed to edit (≤ 3): none in this step (the live integration tests already exist; this step just runs them).
- Files explicitly out-of-bounds for this step:
  - all source — already done in earlier steps.
- Expected sub-agent dispatches:
  - "Run `cargo test -p tree-support --test enforcer_blocker_tdd`; return FACT (all 9 tests pass)." — purpose: AC-8 GREEN gate.
  - "Run `cargo test -p traditional-support --test enforcer_blocker_tdd`; return FACT (all 9 tests pass)." — purpose: AC-8 mirror GREEN gate.
  - "Run `cargo test -p slicer-runtime --test live_layer_support_tdd -- enforcer_forces_live_support_commit_even_when_needs_support_is_false blocker_overrides_needs_support_true_at_commit_level disabled_or_ineligible_support_stage_commits_empty_support_ir`; return FACT per-test pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: AC-10 / N1 / N2 gate.
- Context cost: `M`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - AC-8 GREEN; AC-10, AC-N1, AC-N2 PASS.
- Exit condition: L-shape regression exposed and fixed; live integration confirmed.

### Step 8: Doc Impact + Final packet verification

- Task IDs: `TASK-285`
- Objective: update `docs/05_module_sdk.md` per Doc Impact Statement; re-dispatch the AC matrix; lint.
- Precondition: Steps 2-7 complete.
- Postcondition: all ACs PASS; workspace clippy clean; Doc Impact grep PASS.
- Files allowed to read:
  - `docs/05_module_sdk.md` — locate the "Shared helpers" section (delegate LOCATIONS if > 300 lines)
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
- Files explicitly out-of-bounds for this step:
  - `target/**`
- Expected sub-agent dispatches:
  - "Locate the 'Shared helpers' section in `docs/05_module_sdk.md`; return LOCATIONS ≤ 3 entries." — purpose: insertion point.
  - "Run AC-1 through AC-10 + AC-N1 + AC-N2 + AC-N3 + AC-N4 commands sequentially; return FACT (PASS / FAIL list)." — packet gate.
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
| Step 3 | M | SDK wrapper refactor + centroid helpers deleted. |
| Step 4 | S | RED L-shape regression test in both modules. |
| Step 5 | S | Three TOMLs + guest rebuild check. |
| Step 6 | S | Host shim kebab→snake; dead `runtime_reads.push` removed. |
| Step 7 | M | AC-8 GREEN + live integration. |
| Step 8 | S | Doc + final verification. |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All eight steps complete; each exit condition met.
- AC-1 through AC-10 + AC-N1 + AC-N2 + AC-N3 + AC-N4 PASS.
- Doc Impact Statement satisfied.
- `cargo xtask build-guests --check` clean.
- `docs/07_implementation_status.md` records the new `TASK-285` row as `[x]` (via worker dispatch).
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands: `cargo xtask build-guests --check`, `cargo build --workspace`, the four test commands, `cargo clippy --workspace --all-targets -- -D warnings`.
- Mark `TASK-285` `[x]`; transition `packet.spec.md` to `status: implemented`.
