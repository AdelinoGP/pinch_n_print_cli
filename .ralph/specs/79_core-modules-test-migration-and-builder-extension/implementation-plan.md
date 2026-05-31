# Implementation Plan — Packet 79

## Execution Rules

- This packet hard-depends on packet 78 (`status: implemented`). Step 1 verifies.
- TDD discipline applies: every builder extension lands TDD-first (red → green). Steps 2-6 each write the TDD file BEFORE the production code.
- The hard sequencing constraint: ALL half-one steps (2-7) complete green before ANY Group-B migration starts (steps 12-15). Group-A migrations (steps 9-11) can run in parallel with half-one's tail end (steps 5-7) but the plan documents them sequentially for clarity.
- All cargo invocations delegated; return `FACT: pass/fail + ≤ 5 lines`.
- Narrow tests only DURING implementation. The closure gate (step 18) is the rare `cargo test --workspace` invocation per project `CLAUDE.md` §Test Discipline — bulk migration packet, all 20 core-modules' tests touched.
- Per-module narrow `cargo test -p <module>` MUST run after each migration step; halt on first failure.

## Steps

### Step 1 — Preflight: verify packet 78 closed; capture baseline test count

- **Task IDs**: TASK-227
- **Objective**: Confirm packet 78's `status: implemented`; capture pre-packet-79 baseline of `cargo test --workspace` total count (for AC-11 regression check).
- **Precondition**: Packet 78's `packet.spec.md` exists with `status: implemented`.
- **Postcondition**: Baseline count recorded in the implementation log.
- **Files to read**: `.ralph/specs/78_slicer-test-fold-into-slicer-sdk/packet.spec.md` frontmatter.
- **Files to edit**: none.
- **Expected dispatches**: dispatch 7 (workspace test ceremony — baseline run).
- **Context cost**: M (the workspace test takes time but only once).
- **Narrow verification**: `grep -E '^status:' .ralph/specs/78_slicer-test-fold-into-slicer-sdk/packet.spec.md | grep -q implemented && cargo test --workspace 2>&1 | tail -5`
- **Exit condition**: P78 closed; baseline count noted.

### Step 2 — Add `print_entity` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-1 satisfied. Red TDD then green production code.
- **Precondition**: Step 1 complete.
- **Postcondition**: `test_support_print_entity_tdd.rs` red-confirmed, then green; `print_entity` function exists in `fixtures.rs`.
- **Files to read**: `crates/slicer-sdk/src/test_support/fixtures.rs` (existing structure to mirror); `docs/02_ir_schemas.md` IR-9 PrintEntity field list.
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_print_entity_tdd.rs` (new — red phase first; assert all 5 input fields round-trip)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (add `pub fn print_entity(...)`)
  - `crates/slicer-sdk/src/test_prelude.rs` (re-export)
- **Expected dispatches**: dispatch 1 (IR field-surface confirmation).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_print_entity_tdd`
- **Exit condition**: red phase observed first (TDD compiles but assert fails because function returns wrong field), then green after production code lands.

### Step 3 — Add `tool_change` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-2 satisfied.
- **Precondition**: Step 2 complete.
- **Postcondition**: TDD green; `tool_change` exists.
- **Files to read**: `docs/02_ir_schemas.md` ToolChange subsection of IR-12.
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_tool_change_tdd.rs` (new)
  - `crates/slicer-sdk/src/test_support/fixtures.rs`
  - `crates/slicer-sdk/src/test_prelude.rs`
- **Expected dispatches**: dispatch 1 (already cached from step 2 likely).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_tool_change_tdd`
- **Exit condition**: green.

### Step 4 — Add `seam_candidate` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-5 satisfied.
- **Precondition**: Step 3 complete.
- **Postcondition**: TDD green; `seam_candidate` exists.
- **Files to read**: `docs/02_ir_schemas.md` SeamCandidate definition.
- **Files to edit**: same trio as above.
- **Expected dispatches**: dispatch 1 (cached).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_seam_candidate_tdd`
- **Exit condition**: green.

### Step 5 — Add `LayerCollectionFixtureBuilder` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-3 satisfied. New struct + 6 methods (new, global_layer_index, z, add_entity, add_tool_change, build); production `LayerCollectionBuilder` unchanged.
- **Precondition**: Steps 2-4 complete (the builder may use `print_entity` and `tool_change` internally in its TDD).
- **Postcondition**: TDD green; new struct exists; production builder file untouched.
- **Files to read**: `crates/slicer-sdk/src/layer_collection_builder.rs` (confirm no overlap, 97 lines per recon); `docs/02_ir_schemas.md` IR-12 LayerCollectionIR.
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_layer_collection_fixture_builder_tdd.rs` (new)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (add the struct + impl)
  - `crates/slicer-sdk/src/test_prelude.rs` (re-export)
- **Expected dispatches**: dispatch 1.
- **Context cost**: M (largest of the half-one steps because the struct is multi-method).
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_layer_collection_fixture_builder_tdd && [ "$(grep -c 'pub fn' crates/slicer-sdk/src/layer_collection_builder.rs)" = "5" ]`
- **Exit condition**: TDD green; production builder line count unchanged.

### Step 6 — Add `PerimeterRegionViewBuilder::add_outer_wall_with_flags` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-4 satisfied.
- **Precondition**: Step 5 complete.
- **Postcondition**: TDD green; new method exists on existing struct.
- **Files to read**: `crates/slicer-sdk/src/test_support/fixtures.rs` lines around the existing `add_outer_wall` impl (mirror its style).
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_wall_loop_with_flags_tdd.rs` (new — exercises seam-placer's wall_at_z shape: 3 points, non-empty flags, ExteriorSurface boundary)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (add the new method)
- **Expected dispatches**: dispatch 1.
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_wall_loop_with_flags_tdd`
- **Exit condition**: green.

### Step 7 — Half-one closure: verify all 5 TDDs together + slicer-sdk regression

- **Task IDs**: TASK-227
- **Objective**: All builder extensions co-exist cleanly; existing `slicer-sdk` tests still pass.
- **Precondition**: Steps 2-6 complete.
- **Postcondition**: `cargo test -p slicer-sdk` green; clippy green for slicer-sdk; doc string for new `docs/05` listing prepared.
- **Files to read**: none (verification only).
- **Files to edit**: none (verification only).
- **Expected dispatches**: dispatch 6 (all five TDDs green).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk && cargo clippy -p slicer-sdk --all-targets -- -D warnings`
- **Exit condition**: both green. **Half-one boundary** — context handoff if utilization > 50%.

### Step 8 — Group-A migration: `layer-planner-default`

- **Task IDs**: TASK-228
- **Objective**: AC-6 (partial). The module's tests pass with `test_prelude::*` imports; helper bodies collapse to single-expression builder chains.
- **Precondition**: Step 7 complete.
- **Postcondition**: `cargo test -p layer-planner-default` green; helper body ≤ 4 lines; assertion-snapshot captured (AC-N1).
- **Files to read**:
  - `modules/core-modules/layer-planner-default/tests/*.rs` (full read for pre-migration assertion snapshot + helper-body confirmation)
  - `modules/core-modules/layer-planner-default/src/lib.rs` lines containing `config.get_*` calls (delegated via dispatch 2 for FACT return).
- **Files to edit**:
  - `modules/core-modules/layer-planner-default/Cargo.toml` (+1 dev-dep line)
  - `modules/core-modules/layer-planner-default/tests/*.rs` (`use slicer_sdk::test_prelude::*;`; rewrite `make_config` + `make_config_with_per_object_lh` bodies)
- **Expected dispatches**: dispatch 2 (config-key extraction for layer-planner-default), dispatch 4 (helper-body verbatim).
- **Context cost**: M
- **Narrow verification**: `cargo test -p layer-planner-default && grep -qE 'use slicer_sdk::test_prelude' modules/core-modules/layer-planner-default/tests/*.rs`
- **Exit condition**: green; assertion snapshot recorded.

### Step 9 — Group-A migration: `lightning-infill`, `mesh-segmentation`, `traditional-support`

- **Task IDs**: TASK-228
- **Objective**: AC-6 (partial). Three modules with 2 helpers each.
- **Precondition**: Step 8 complete.
- **Postcondition**: All three `cargo test -p <name>` green; assertion snapshots captured per module.
- **Files to read / edit**: per-module pattern from step 8, repeated.
- **Expected dispatches**: dispatch 2 (config-keys for each) + dispatch 4 (helper bodies for each).
- **Context cost**: M (3 modules in one step — kept as one step for plan brevity; the implementer may sub-divide).
- **Narrow verification**: `cargo test -p lightning-infill -p mesh-segmentation -p traditional-support`
- **Exit condition**: all three green.

### Step 10 — Group-A migration: `tree-support`, `classic-perimeters`; verify `gyroid-infill`

- **Task IDs**: TASK-228
- **Objective**: Finish Group A. `tree-support` (2 helpers) and `classic-perimeters` (7 helpers across two test files) migrate; `gyroid-infill` runs as a regression check (no edits).
- **Precondition**: Step 9 complete.
- **Postcondition**: AC-6 fully satisfied. All 7 Group-A packages green.
- **Files to read / edit**: per-module pattern.
- **Expected dispatches**: dispatch 2 + dispatch 4 (for tree-support, classic-perimeters).
- **Context cost**: M
- **Narrow verification**: `cargo test -p tree-support -p classic-perimeters -p gyroid-infill && for m in tree-support classic-perimeters; do grep -qE 'use slicer_sdk::test_prelude' modules/core-modules/$m/tests/*.rs || exit 1; done`
- **Exit condition**: all green. AC-6 verifiable via its compound command.

### Step 11 — Group-A intermediate gate: clippy + workspace check

- **Task IDs**: TASK-228
- **Objective**: Mid-packet sanity gate before Group-B starts.
- **Precondition**: Step 10 complete.
- **Postcondition**: `cargo check --workspace --all-targets` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Files to read / edit**: none (verification only).
- **Expected dispatches**: cargo check + clippy via sub-agent.
- **Context cost**: S
- **Narrow verification**: as above.
- **Exit condition**: both clean.

### Step 12 — Group-B migration: `path-optimization-default` (3 `make_wall_loop` variants collapse to direct `add_outer_wall`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). Per design.md §Data and Contract Notes: all three `make_wall_loop` variants use `feature_flags: vec![]` + `boundary_type: Interior`, which the EXISTING `PerimeterRegionViewBuilder::add_outer_wall` already covers. Helpers can disappear; call sites use `add_outer_wall` directly.
- **Precondition**: Step 11 complete.
- **Postcondition**: `cargo test -p path-optimization-default` green; the three `make_wall_loop` helpers either gone or shrunk to one-line forwarders.
- **Files to read**: `modules/core-modules/path-optimization-default/tests/{seam_consumption,travel_policy,retract_mode_propagation}_tdd.rs` (assertion snapshots + helper call sites).
- **Files to edit**: same plus `modules/core-modules/path-optimization-default/Cargo.toml`.
- **Expected dispatches**: dispatch 3 (config-keys), dispatch 5 (test counts).
- **Context cost**: M
- **Narrow verification**: `cargo test -p path-optimization-default && for fn in make_wall_loop; do for f in modules/core-modules/path-optimization-default/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done`
- **Exit condition**: green; helper bodies ≤ 4 lines.

### Step 13 — Group-B migration: `seam-placer` (uses `seam_candidate` + `add_outer_wall_with_flags`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). `candidate` → `seam_candidate`; `wall_at_z` → `add_outer_wall_with_flags`.
- **Precondition**: Step 12 complete.
- **Postcondition**: `cargo test -p seam-placer` green.
- **Files to read**: `modules/core-modules/seam-placer/tests/{seam_placer_tdd,seam_placer_dispatch_tdd}.rs` (assertion snapshots).
- **Files to edit**: those test files plus `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p seam-placer && for fn in candidate wall_at_z; do for f in modules/core-modules/seam-placer/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done`
- **Exit condition**: green.

### Step 14 — Group-B migration: `skirt-brim` (uses `print_entity` + `LayerCollectionFixtureBuilder`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). Two test files, 4 helpers across them (`make_entity_at` × 2 different signatures, `make_layer_with_entities`, `make_layer`).
- **Precondition**: Step 13 complete.
- **Postcondition**: `cargo test -p skirt-brim` green.
- **Files to read**: `modules/core-modules/skirt-brim/tests/{skirt_brim_tdd,finalization_live_tdd}.rs`.
- **Files to edit**: those + `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p skirt-brim && for fn in make_entity_at make_layer_with_entities make_layer; do for f in modules/core-modules/skirt-brim/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done`
- **Exit condition**: green.

### Step 15 — Group-B migration: `wipe-tower` (uses `LayerCollectionFixtureBuilder` + `tool_change`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 fully satisfied after this step.
- **Precondition**: Step 14 complete.
- **Postcondition**: `cargo test -p wipe-tower` green.
- **Files to read**: `modules/core-modules/wipe-tower/tests/{wipe_tower_tdd,finalization_live_tdd}.rs`.
- **Files to edit**: those + `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p wipe-tower && for fn in make_layer; do for f in modules/core-modules/wipe-tower/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done`
- **Exit condition**: green; AC-7's compound verification command can now succeed.

### Step 16 — Group-C decisions: `fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`

- **Task IDs**: TASK-228
- **Objective**: AC-8 satisfied. Per-module decision: migrate to prelude only if file gets shorter; otherwise leave untouched.
- **Precondition**: Step 15 complete.
- **Postcondition**: `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing` green; per-module decision documented in implementation log.
- **Files to read**: each module's test files (line counts only; full read only if migration is proceeding).
- **Files to edit**: per-module decision.
- **Expected dispatches**: dispatch 5 (test counts).
- **Context cost**: M
- **Narrow verification**: `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing`
- **Exit condition**: green; decision log entry per module.

### Step 17 — Documentation: append new helpers to `docs/05_module_sdk.md` §Test Support

- **Task IDs**: TASK-228
- **Objective**: Doc Impact Statement satisfied. One paragraph appended listing the five new fixture surfaces.
- **Precondition**: Step 16 complete (all migrations done).
- **Postcondition**: `docs/05_module_sdk.md` §Test Support documents the new helpers.
- **Files to read**: `docs/05_module_sdk.md` lines 445-560 (post-packet-78 range).
- **Files to edit**: `docs/05_module_sdk.md` (range edit only).
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `grep -qE 'print_entity|tool_change|seam_candidate|LayerCollectionFixtureBuilder|add_outer_wall_with_flags' docs/05_module_sdk.md`
- **Exit condition**: doc grep returns at least 5 matches.

### Step 18 — Closure ceremony: cargo test --workspace + wasm-target gate + clippy + guest staleness

- **Task IDs**: TASK-228
- **Objective**: AC-10 + AC-11 satisfied. Final closure gate.
- **Precondition**: Steps 1-17 complete.
- **Postcondition**: All five closure gates green; pre/post test-count delta is 0 (or positive only if Group-C added tests — which shouldn't happen per scope).
- **Files to read / edit**: none.
- **Expected dispatches**: dispatch 7 (workspace test ceremony), dispatch 8 (guest staleness), dispatch 9 (wasm-target gate), dispatch 10 (test-count audit).
- **Context cost**: M (workspace test is heavy, but dispatched once).
- **Narrow verification (the closure gates)**:
  1. `cargo xtask build-guests --check` (rebuild if STALE)
  2. `cargo check --workspace --all-targets`
  3. `cargo check --target wasm32-unknown-unknown -p skirt-brim -p seam-placer -p classic-perimeters`
  4. `cargo tree --target wasm32-unknown-unknown -p skirt-brim` (assert no `feature="test"`)
  5. `cargo clippy --workspace --all-targets -- -D warnings`
  6. `cargo test --workspace`
- **Exit condition**: all six gates clean; test-count delta = 0.

## Per-Step Budget Roll-Up

| Step | Cost | Cumulative | Notes |
|---|---|---|---|
| 1 | M | M | Baseline + preflight |
| 2 | S | M | print_entity |
| 3 | S | M | tool_change |
| 4 | S | M | seam_candidate |
| 5 | M | L⁻ | LayerCollectionFixtureBuilder — half-one's largest |
| 6 | S | L⁻ | wall_loop_with_flags |
| 7 | S | L⁻ | half-one closure |
| **Half-one boundary** | | | Recommended handoff if context > 50% |
| 8 | M | L | layer-planner-default |
| 9 | M | L | lightning-infill + mesh-segmentation + traditional-support |
| 10 | M | L | tree-support + classic-perimeters + gyroid verify |
| 11 | S | L | Mid-packet gate |
| 12 | M | L | path-optimization-default |
| 13 | M | L | seam-placer |
| 14 | M | L | skirt-brim |
| 15 | M | L | wipe-tower |
| 16 | M | L | Group-C decisions |
| 17 | S | L | docs/05 append |
| 18 | M | L | Closure ceremony |

**Aggregate**: L. **No single step is L** — every step's individual cost is S or M. The L aggregate is intrinsic to 18 sequential steps of migration work, not to step size. Per the spec-packet-generator rules, this is acceptable — but the implementer SHOULD plan for at least one context handoff at the half-one/half-two boundary (after step 7). Two handoffs are likely needed: one at the boundary, one mid-Group-B (after step 13) if context is constrained.

## Packet Completion Gate

The closure gates from Step 18 (six gates) constitute the completion check. Run in order; halt and resolve at the first failure.

## Acceptance Ceremony

After the closure gates pass:

- Update `packet.spec.md` frontmatter: `status: implemented`, add `closed: <ISO date>`.
- Append closure detail to `docs/07_implementation_status.md`: change TASK-227 and TASK-228 from `[ ]` to `[x]`; add `Closed YYYY-MM-DD — packet 79; verified by cargo test --workspace (<count> tests passed)` suffix to each.
- Record AC-N1's assertion snapshots in the closure commit message (10-11 short pre/post pairs).
- Open follow-up: packet 80 may proceed (relocate the 2 misplaced runtime tests). Mark its `requires: 79` prerequisite as resolved when 80 is activated.
- The workspace-test invocation in Step 18 (AC-11) is the canonical closure evidence — its summary line goes in the commit message.
