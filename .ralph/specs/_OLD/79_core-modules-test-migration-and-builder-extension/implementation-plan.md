# Implementation Plan — Packet 79

## Execution Rules

- This packet hard-depends on packet 78 (`status: implemented`). Step 1 verifies.
- TDD discipline applies: every builder extension lands TDD-first (red → green). Steps 2-8 each write the TDD file BEFORE the production code.
- The hard sequencing constraint: ALL half-one steps (2-9) complete green before Group-D tightening (step 10) OR any Group-B migration (steps 15-18) starts. Group-A migrations (steps 11-13) can run in parallel with Group-D tightening (step 10) once half-one closes; the plan documents them sequentially for clarity.
- All cargo invocations delegated; return `FACT: pass/fail + ≤ 5 lines`.
- Narrow tests only DURING implementation. The closure gate (step 21) is the rare `cargo test --workspace` invocation per project `CLAUDE.md` §Test Discipline — bulk migration packet, all 20 core-modules' tests touched.
- Per-module narrow `cargo test -p <module>` MUST run after each migration step; halt on first failure.

## Steps

### Step 1 — Preflight: verify packet 78 closed; capture baseline test count from P78's closure log

- **Task IDs**: TASK-227
- **Objective**: Confirm packet 78's `status: implemented`; capture pre-packet-79 baseline test count by reading P78's closure-ceremony record (commit message, `packet.spec.md` closure annotation, or `docs/07_implementation_status.md` row). Do **NOT** re-run `cargo test --workspace` for baseline — the closure ceremony in Step 21 is the sole workspace-test invocation allowed in this packet per project `CLAUDE.md` §Test Discipline.
- **Precondition**: Packet 78's `packet.spec.md` exists with `status: implemented` AND its closure log records a `cargo test --workspace` count.
- **Postcondition**: Baseline count recorded in the implementation log; P78 closure verified.
- **Files to read**: `.ralph/specs/78_slicer-test-fold-into-slicer-sdk/packet.spec.md` frontmatter + closure log; `git log` for the P78 closure commit if the count is in the commit message.
- **Files to edit**: none.
- **Expected dispatches**: none (this is a metadata read, not a build/test invocation).
- **Context cost**: S
- **Narrow verification**: `grep -E '^status:' .ralph/specs/78_slicer-test-fold-into-slicer-sdk/packet.spec.md | grep -q implemented` then manual extraction of baseline test count from P78 closure record.
- **Exit condition**: P78 status confirmed `implemented`; baseline count noted in implementation log. If P78 didn't capture a workspace-test count at closure, log it as a P78 closure-log defect and either patch P78's record retroactively OR mark this packet's AC-11 audit as "no pre-baseline available — post-baseline becomes the new floor".

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

### Step 7 — Add `rect_polygon` TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-12 satisfied. Closes the `make_narrow_rect`-style ExPolygon gap surfaced by packet 78's arachne-perimeters migration where `square_polygon` was too symmetric (single side parameter, not width+height) and `rect_path` returned the wrong type (`ExtrusionPath3D`, not `ExPolygon`). `rect_polygon` mirrors `square_polygon`'s style: axis-aligned rectangle, CCW winding, mm-space → units conversion via `mm_to_units`, empty `holes`.
- **Precondition**: Step 6 complete.
- **Postcondition**: `test_support_rect_polygon_tdd.rs` red-confirmed (asserts 4-vertex contour, corner coordinates at `(cx ± w/2, cy ± h/2)` in scaled-units, CCW winding, holes empty), then green; `rect_polygon` exists in `fixtures.rs`; re-exported via `test_prelude`.
- **Files to read**: `crates/slicer-sdk/src/test_support/fixtures.rs` (existing `square_polygon` impl for style mirror); ExPolygon Rust shape via dispatch 1 (cached, extended in this packet).
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_rect_polygon_tdd.rs` (new — `rect_polygon(0.0, 0.0, 4.0, 6.0).contour.points` has exactly 4 entries; x-range = ±2mm (in units); y-range = ±3mm (in units); CCW winding asserted via signed-area > 0; `holes.is_empty()`)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (add `pub fn rect_polygon(cx_mm: f32, cy_mm: f32, width_mm: f32, height_mm: f32) -> ExPolygon`)
  - `crates/slicer-sdk/src/test_prelude.rs` (re-export `rect_polygon`)
- **Expected dispatches**: dispatch 1 (cached).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_rect_polygon_tdd`
- **Exit condition**: red phase observed (TDD compiles but asserts fail because function isn't implemented), then green after production code lands.

### Step 8 — Add `SliceRegionViewBuilder` setters TDD-first

- **Task IDs**: TASK-227
- **Objective**: AC-13 satisfied. Extends the existing `SliceRegionViewBuilder` with seven new setter methods (`top_shell_index`, `top_solid_fill`, `bottom_shell_index`, `bottom_solid_fill`, `is_bridge`, `bridge_areas`, `bridge_orientation_deg`) so that post-build `r.set_*()` chains used by `rectilinear-infill::make_test_region` / `make_bridge_region` (packet 78 migration deviation 3) collapse into single-expression builder chains. The production `SliceRegionView` type itself is unchanged.
- **Precondition**: Step 7 complete.
- **Postcondition**: TDD green; 7 new methods exist on the existing `SliceRegionViewBuilder`; default-field invariant locked (per design.md Invariant G — unset setters leave the field at `SliceRegionViewBuilder::new()`'s initial state).
- **Files to read**: `crates/slicer-sdk/src/test_support/fixtures.rs` (existing `SliceRegionViewBuilder` impl, all field defaults); `SliceRegionView` field surface (via extended dispatch 1).
- **Files to edit**:
  - `crates/slicer-sdk/tests/test_support_slice_region_view_builder_setters_tdd.rs` (new — first asserts a default-built region matches expectations (no setter calls); then asserts each of the 7 setters round-trips via the corresponding accessor on the built `SliceRegionView`; finally asserts idempotency + last-write-wins on one representative setter)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (extend the `impl SliceRegionViewBuilder` block with the 7 setters)
- **Expected dispatches**: dispatch 1 (cached).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk --test test_support_slice_region_view_builder_setters_tdd`
- **Exit condition**: TDD green; existing `SliceRegionViewBuilder` tests still green (verified by Step 9's regression sweep).

### Step 9 — Half-one closure: verify all 7 TDDs together + slicer-sdk regression

- **Task IDs**: TASK-227
- **Objective**: All builder extensions co-exist cleanly; existing `slicer-sdk` tests still pass.
- **Precondition**: Steps 2-8 complete.
- **Postcondition**: `cargo test -p slicer-sdk` green; clippy green for slicer-sdk; doc string for new `docs/05` listing prepared (includes all five new freestanding fixtures plus `add_outer_wall_with_flags` plus the 7 new `SliceRegionViewBuilder` setters).
- **Files to read**: none (verification only).
- **Files to edit**: none (verification only).
- **Expected dispatches**: dispatch 6 (all seven TDDs green).
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk && cargo clippy -p slicer-sdk --all-targets -- -D warnings`
- **Exit condition**: both green. **Half-one boundary** — context handoff if utilization > 50%.

### Step 10 — Group-D tightening: adopt new builders in `arachne-perimeters` and `rectilinear-infill`

- **Task IDs**: TASK-228
- **Objective**: AC-14 satisfied. Replace the workaround forms recorded in packet 78's closure deviations: (a) `arachne-perimeters/tests/arachne_perimeters_tdd.rs::make_narrow_rect`'s 1-line inline `ExPolygon` literal becomes `rect_polygon(0.0, 0.0, width_mm, height_mm)`; (b) `rectilinear-infill/tests/top_bottom_fill_tdd.rs::make_test_region`'s post-build `r.set_top_shell_index(...) / r.set_top_solid_fill(...) / r.set_bottom_shell_index(...) / r.set_bottom_solid_fill(...) / r.set_is_bridge(...)` chain becomes a single builder chain using Step-8 setters; (c) `rectilinear-infill/tests/bridge_infill_emission_tdd.rs::make_bridge_region`'s `r.set_is_bridge(...) / r.set_bridge_areas(...) / r.set_bridge_orientation_deg(...)` chain similarly. Dogfoods the new builders against the workloads that originally surfaced their need.
- **Precondition**: Step 9 (half-one closure) complete; new builders exist + green.
- **Postcondition**: `cargo test -p arachne-perimeters -p rectilinear-infill` green; assertion-snapshot pre/post pairs recorded for `make_narrow_rect`, `make_test_region`, `make_bridge_region` (AC-N1 discipline applies); no `set_top_shell_index` / `set_top_solid_fill` / `set_bottom_shell_index` / `set_bottom_solid_fill` / `set_is_bridge` / `set_bridge_areas` / `set_bridge_orientation_deg` calls remain in `rectilinear-infill`'s test files; `make_narrow_rect` body uses `rect_polygon`.
- **Files to read**:
  - `modules/core-modules/arachne-perimeters/tests/arachne_perimeters_tdd.rs` (capture pre-state of `make_narrow_rect`)
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` (capture pre-state of `make_test_region`)
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (capture pre-state of `make_bridge_region`)
- **Files to edit**: those three files. No `Cargo.toml` changes (both modules already carry the dev-dep from P78).
- **Expected dispatches**: none beyond the pre-migration snapshot capture (manual; same AC-N1 discipline as Groups A+B).
- **Context cost**: S
- **Narrow verification**: `cargo test -p arachne-perimeters -p rectilinear-infill && grep -qE 'rect_polygon' modules/core-modules/arachne-perimeters/tests/arachne_perimeters_tdd.rs && ! grep -rE '\.set_(top_shell_index|top_solid_fill|bottom_shell_index|bottom_solid_fill|is_bridge|bridge_areas|bridge_orientation_deg)\(' modules/core-modules/rectilinear-infill/tests/`
- **Exit condition**: green; the inverse-grep returns empty (no post-build setter calls remain).

### Step 11 — Group-A migration: `layer-planner-default`

- **Task IDs**: TASK-228
- **Objective**: AC-6 (partial). The module's tests pass with `test_prelude::*` imports; helper bodies collapse to single-expression builder chains.
- **Precondition**: Step 9 complete. (Step 10 may run in parallel — Group-A and Group-D touch disjoint module sets.)
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

### Step 12 — Group-A migration: `lightning-infill`, `mesh-segmentation`, `traditional-support`

- **Task IDs**: TASK-228
- **Objective**: AC-6 (partial). Three modules with 2 helpers each.
- **Precondition**: Step 11 complete.
- **Postcondition**: All three `cargo test -p <name>` green; assertion snapshots captured per module.
- **Files to read / edit**: per-module pattern from step 11, repeated.
- **Expected dispatches**: dispatch 2 (config-keys for each) + dispatch 4 (helper bodies for each).
- **Context cost**: M (3 modules in one step — kept as one step for plan brevity; the implementer may sub-divide).
- **Narrow verification**: `cargo test -p lightning-infill -p mesh-segmentation -p traditional-support`
- **Exit condition**: all three green.

### Step 13 — Group-A migration: `tree-support`, `classic-perimeters`; verify `gyroid-infill`

- **Task IDs**: TASK-228
- **Objective**: Finish Group A. `tree-support` (2 helpers) and `classic-perimeters` (7 helpers across two test files) migrate; `gyroid-infill` runs as a regression check (no edits).
- **Precondition**: Step 12 complete.
- **Postcondition**: AC-6 fully satisfied. All 7 Group-A packages green.
- **Files to read / edit**: per-module pattern.
- **Expected dispatches**: dispatch 2 + dispatch 4 (for tree-support, classic-perimeters).
- **Context cost**: M
- **Narrow verification**: `cargo test -p tree-support -p classic-perimeters -p gyroid-infill && for m in tree-support classic-perimeters; do grep -qE 'use slicer_sdk::test_prelude' modules/core-modules/$m/tests/*.rs || exit 1; done`
- **Exit condition**: all green. AC-6 verifiable via its compound command.

### Step 14 — Mid-packet gate: clippy + workspace check (after Groups A and D)

- **Task IDs**: TASK-228
- **Objective**: Mid-packet sanity gate before Group-B starts. Covers Group-A migrations (steps 11-13) AND Group-D tightening (step 10) in one workspace sweep.
- **Precondition**: Steps 10 and 13 both complete.
- **Postcondition**: `cargo check --workspace --all-targets` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Files to read / edit**: none (verification only).
- **Expected dispatches**: cargo check + clippy via sub-agent.
- **Context cost**: S
- **Narrow verification**: as above.
- **Exit condition**: both clean.

### Step 15 — Group-B migration: `path-optimization-default` (3 `make_wall_loop` variants collapse to direct `add_outer_wall`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). Per design.md §Data and Contract Notes: all three `make_wall_loop` variants use `feature_flags: vec![]` + `boundary_type: Interior`, which the EXISTING `PerimeterRegionViewBuilder::add_outer_wall` already covers. Helpers can disappear; call sites use `add_outer_wall` directly.
- **Precondition**: Step 14 complete.
- **Postcondition**: `cargo test -p path-optimization-default` green; the three `make_wall_loop` helpers either gone or shrunk to one-line forwarders.
- **Files to read**: `modules/core-modules/path-optimization-default/tests/{seam_consumption,travel_policy,retract_mode_propagation}_tdd.rs` (assertion snapshots + helper call sites).
- **Files to edit**: same plus `modules/core-modules/path-optimization-default/Cargo.toml`.
- **Expected dispatches**: dispatch 3 (config-keys), dispatch 5 (test counts).
- **Context cost**: M
- **Narrow verification**: `cargo test -p path-optimization-default && for fn in make_wall_loop; do for f in modules/core-modules/path-optimization-default/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 8) exit 1}' || exit 1; done; done`
- **Exit condition**: green; helper bodies ≤ 4 lines.

### Step 16 — Group-B migration: `seam-placer` (uses `seam_candidate` + `add_outer_wall_with_flags`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). `candidate` → `seam_candidate`; `wall_at_z` → `add_outer_wall_with_flags`.
- **Precondition**: Step 15 complete.
- **Postcondition**: `cargo test -p seam-placer` green.
- **Files to read**: `modules/core-modules/seam-placer/tests/{seam_placer_tdd,seam_placer_dispatch_tdd}.rs` (assertion snapshots).
- **Files to edit**: those test files plus `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p seam-placer && for fn in candidate wall_at_z; do for f in modules/core-modules/seam-placer/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 8) exit 1}' || exit 1; done; done`
- **Exit condition**: green.

### Step 17 — Group-B migration: `skirt-brim` (uses `print_entity` + `LayerCollectionFixtureBuilder`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 (partial). Two test files, 4 helpers across them (`make_entity_at` × 2 different signatures, `make_layer_with_entities`, `make_layer`).
- **Precondition**: Step 16 complete.
- **Postcondition**: `cargo test -p skirt-brim` green.
- **Files to read**: `modules/core-modules/skirt-brim/tests/{skirt_brim_tdd,finalization_live_tdd}.rs`.
- **Files to edit**: those + `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p skirt-brim && for fn in make_entity_at make_layer_with_entities make_layer; do for f in modules/core-modules/skirt-brim/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 8) exit 1}' || exit 1; done; done`
- **Exit condition**: green.

### Step 18 — Group-B migration: `wipe-tower` (uses `LayerCollectionFixtureBuilder` + `tool_change`)

- **Task IDs**: TASK-228
- **Objective**: AC-7 fully satisfied after this step.
- **Precondition**: Step 17 complete.
- **Postcondition**: `cargo test -p wipe-tower` green.
- **Files to read**: `modules/core-modules/wipe-tower/tests/{wipe_tower_tdd,finalization_live_tdd}.rs`.
- **Files to edit**: those + `Cargo.toml`.
- **Expected dispatches**: dispatch 3.
- **Context cost**: M
- **Narrow verification**: `cargo test -p wipe-tower && for fn in make_layer; do for f in modules/core-modules/wipe-tower/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 8) exit 1}' || exit 1; done; done`
- **Exit condition**: green; AC-7's compound verification command can now succeed.

### Step 19 — Group-C decisions: `fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`

- **Task IDs**: TASK-228
- **Objective**: AC-8 satisfied. Per-module decision: migrate to prelude only if file gets shorter; otherwise leave untouched.
- **Precondition**: Step 18 complete.
- **Postcondition**: `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing` green; per-module decision documented in implementation log.
- **Files to read**: each module's test files (line counts only; full read only if migration is proceeding).
- **Files to edit**: per-module decision.
- **Expected dispatches**: dispatch 5 (test counts).
- **Context cost**: M
- **Narrow verification**: `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing`
- **Exit condition**: green; decision log entry per module.

### Step 20 — Documentation: append new helpers to `docs/05_module_sdk.md` §Test Support

- **Task IDs**: TASK-228
- **Objective**: Doc Impact Statement satisfied. One paragraph appended listing the seven new fixture surfaces (five freestanding fixtures + `add_outer_wall_with_flags` + the seven new `SliceRegionViewBuilder` setters).
- **Precondition**: Step 19 complete (all migrations done).
- **Postcondition**: `docs/05_module_sdk.md` §Test Support documents the new helpers; the stronger-form grep loop in packet.spec.md's Doc Impact Statement returns clean.
- **Files to read**: `docs/05_module_sdk.md` lines 445-560 (post-packet-78 range).
- **Files to edit**: `docs/05_module_sdk.md` (range edit only).
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `for sym in print_entity tool_change seam_candidate LayerCollectionFixtureBuilder add_outer_wall_with_flags rect_polygon top_shell_index; do grep -qE "$sym" docs/05_module_sdk.md || exit 1; done`
- **Exit condition**: doc grep returns clean for all seven representative symbols.

### Step 21 — Closure ceremony: cargo test --workspace + wasm-target gate + clippy + guest staleness

- **Task IDs**: TASK-228
- **Objective**: AC-10 + AC-11 satisfied. Final closure gate.
- **Precondition**: Steps 1-20 complete.
- **Postcondition**: All five closure gates green; pre/post test-count delta is 0 (or positive only if Group-C added tests — which shouldn't happen per scope).
- **Files to read / edit**: none.
- **Expected dispatches**: dispatch 7 (workspace test ceremony), dispatch 8 (guest staleness), dispatch 9 (wasm-target gate), dispatch 10 (test-count audit).
- **Context cost**: M (workspace test is heavy, but dispatched once).
- **Narrow verification (the closure gates)**:
  1. `cargo xtask build-guests --check` (rebuild if STALE)
  2. `cargo check --workspace --all-targets`
  3. `cargo check --target wasm32-unknown-unknown -p skirt-brim -p seam-placer -p classic-perimeters`
  4. `cargo tree --target wasm32-unknown-unknown -p skirt-brim -e features` (assert no `slicer-sdk feature "test"` edge)
  5. `cargo clippy --workspace --all-targets -- -D warnings`
  6. `cargo test --workspace`
- **Exit condition**: all six gates clean; test-count delta = 0.

## Per-Step Budget Roll-Up

| Step | Cost | Cumulative | Notes |
|---|---|---|---|
| 1 | S | S | Baseline + preflight |
| 2 | S | S | print_entity |
| 3 | S | S | tool_change |
| 4 | S | S | seam_candidate |
| 5 | M | M | LayerCollectionFixtureBuilder — half-one's largest |
| 6 | S | M | wall_loop_with_flags |
| 7 | S | M | rect_polygon (NEW) |
| 8 | S | M | SliceRegionViewBuilder setters (NEW) |
| 9 | S | M | half-one closure |
| **Half-one boundary** | | | Recommended handoff if context > 50% |
| 10 | S | L⁻ | Group-D tightening (NEW; arachne + rectilinear) |
| 11 | M | L | layer-planner-default |
| 12 | M | L | lightning-infill + mesh-segmentation + traditional-support |
| 13 | M | L | tree-support + classic-perimeters + gyroid verify |
| 14 | S | L | Mid-packet gate (covers Groups A and D) |
| 15 | M | L | path-optimization-default |
| 16 | M | L | seam-placer |
| 17 | M | L | skirt-brim |
| 18 | M | L | wipe-tower |
| 19 | M | L | Group-C decisions |
| 20 | S | L | docs/05 append |
| 21 | M | L | Closure ceremony |

**Aggregate**: L. **No single step is L** — every step's individual cost is S or M. The L aggregate is intrinsic to 21 sequential steps of migration work, not to step size. Per the spec-packet-generator rules, this is acceptable — but the implementer SHOULD plan for at least one context handoff at the half-one/half-two boundary (after step 9). Two handoffs are likely needed: one at the boundary, one mid-Group-B (after step 16) if context is constrained.

## Packet Completion Gate

The closure gates from Step 21 (six gates) constitute the completion check. Run in order; halt and resolve at the first failure.

## Acceptance Ceremony

After the closure gates pass:

- Update `packet.spec.md` frontmatter: `status: implemented`, add `closed: <ISO date>`.
- Append closure detail to `docs/07_implementation_status.md`: change TASK-227 and TASK-228 from `[ ]` to `[x]`; add `Closed YYYY-MM-DD — packet 79; verified by cargo test --workspace (<count> tests passed)` suffix to each.
- Record AC-N1's assertion snapshots in the closure commit message (one representative test per migrated module across Groups A + B + D = 12-13 pre/post pairs).
- Open follow-up: packet 80 may proceed (relocate the 2 misplaced runtime tests). Mark its `requires: 79` prerequisite as resolved when 80 is activated.
- The workspace-test invocation in Step 21 (AC-11) is the canonical closure evidence — its summary line goes in the commit message.
