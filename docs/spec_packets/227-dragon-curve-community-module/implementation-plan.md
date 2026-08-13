# Implementation Plan: 227-dragon-curve-community-module

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Read the recorded 225 verdict and select the authoring branch

- Task IDs: `TASK-338`
- Objective: Recover the 225 feasibility verdict from the living record and lock the branch (Go vs Rust fallback) before authoring any branch-specific artifact.
- Precondition: `docs/14_submodule_programming_languages.md` §Community-module context carries 225's recorded verdict (or a missing-verdict condition is observed and reported).
- Postcondition: The verdict is recorded verbatim in the step log; the branch selector is `BRANCH_A` (Go loadable-and-correct) or `BRANCH_B` (fallback), with the one-line rationale. The module directory `modules/community-modules/dragon-curve/` is confirmed empty.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/14_submodule_programming_languages.md` - delegated SUMMARY of §Community-module context only (not a full read).
  - `modules/community-modules/` - directory listing (confirm empty parent).
- Files allowed to edit (at most 3):
  - (none — read-only discovery step)
- Files explicitly out of bounds:
  - `docs/spec_packets/225-*/**` (does not exist yet; never attempt to read)
  - `docs/feasibility-probes/*` (225's raw records; the living verdict is authoritative)
- Blast-radius discipline: not applicable (no struct field or schema constant added).
- Expected sub-agent dispatches:
  - Question: what is the current recorded Go/MoonBit verdict in `docs/14_submodule_programming_languages.md` §Community-module context? scope: that section only; return: `SUMMARY` (≤100 words, verdict verbatim).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - direct read of the Central Symbol Contract + Grounding Facts.
- OrcaSlicer refs:
  - none.
- Verification:
  - `ls modules/community-modules/dragon-curve 2>/dev/null` - FACT: directory absent or empty before authoring.
  - `rg -n 'not loadable|loadable-and-correct|not correct' docs/14_submodule_programming_languages.md` - FACT: the verdict line exists and is quoted in the step log.
- Exit condition: branch selector recorded and the empty-directory precondition confirmed.

### Step 2: Author `dragon-curve.toml` (manifest + config schema)

- Task IDs: `TASK-338`
- Objective: Write the complete manifest mirroring `rectilinear-infill.toml` with the dragon's claims, compatibility, stage, and six config keys.
- Precondition: Step 1 branch selected (manifest is branch-independent).
- Postcondition: `dragon-curve.toml` exists with `[module] id = "com.example.dragon-curve"`, `[stage] id = "Layer::Infill"`, `[claims].holds = ["claim:sparse-fill", "claim:authored-coloring"]`, `[compatibility]` min-host/min-ir/max-ir mirrored from rectilinear, and `[config.schema]` keys `infill_density`, `infill_angle`, `infill_speed`, `line_width`, `tiling_depth`, `color_map` in snake_case. AC-1 and AC-2 green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - full read (manifest model).
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full read (the `float-list` precedent for `color_map`).
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/dragon-curve.toml`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/manifest.rs` (the parser is read-only context; delegate the supported-type question if needed).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what list-valued config types does the manifest schema parser accept (is a tool-index list expressible as `float-list`)? scope: `crates/slicer-scheduler/src/manifest.rs`; return: `FACT` (supported type strings for list fields).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1, §5 - direct read.
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q '"com\.example\.dragon-curve"' modules/community-modules/dragon-curve/dragon-curve.toml` - FACT pass/fail.
  - `rg -q '\[config\.schema\.(infill_density|infill_angle|infill_speed|line_width|tiling_depth|color_map)\]' modules/community-modules/dragon-curve/dragon-curve.toml` - FACT pass/fail (AC-2).
- Exit condition: AC-1 and AC-2 commands pass.

### Step 3: Author the pure tiling + color-mapping logic and its tests (TDD)

- Task IDs: `TASK-338`
- Objective: Implement the deterministic dragon tiling over an `ExPolygon` and the pure `map_tiling_index_to_tool` wrap, TDD-first, with no `tool_count`/`tool_index` dependency.
- Precondition: Step 2 manifest exists (the config keys it declares are the `from_config` source).
- Postcondition: `src/lib.rs` exposes the pure helpers (e.g. `tile_dragon_curve(expoly, line_spacing, tiling_depth) -> Vec<(usize, Point2, Point2)>` and `map_tiling_index_to_tool(ordinal, generation, color_map, tool_count) -> Option<u32>`); three test files (`dragon_tiling_tdd.rs`, `dragon_color_map_tdd.rs`, `dragon_config_override_tdd.rs`) are green for AC-3, AC-4, AC-5, AC-6, AC-N1.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/test_prelude.rs` - delegated LOCATIONS for builder names.
  - `crates/slicer-ir/src/slice_ir.rs` - lines `700-727` (`ConfigValue` variants) and lines `1941-1948` (`ExtrusionPath3D` current fields).
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/src/lib.rs`
  - `modules/community-modules/dragon-curve/tests/dragon_tiling_tdd.rs`
  - `modules/community-modules/dragon-curve/tests/dragon_color_map_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/host.rs` (the 226 `tool_count` wrapper — not yet referenced in this step).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: which `slicer_sdk::test_prelude` builders construct an `ExPolygon` with holes and a `SliceRegionView` with a per-region `ConfigView`? scope: `crates/slicer-sdk/src/test_prelude.rs`, `crates/slicer-sdk/src/views.rs`; return: `LOCATIONS` (≤20 entries).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §4 - direct read (the tiling semantics + edge cases).
- OrcaSlicer refs:
  - none.
- Verification:
  - `cd modules/community-modules/dragon-curve && cargo test --test dragon_tiling_tdd -- --exact` - FACT pass/fail.
  - `cd modules/community-modules/dragon-curve && cargo test --test dragon_color_map_tdd -- --exact` - FACT pass/fail.
- Exit condition: AC-3, AC-4, AC-5, AC-N1 green; `dragon_config_override_tdd` authored and passing in Step 4.

### Step 4: Wire the module `from_config` + `run_infill` and the per-region override test

- Task IDs: `TASK-338`
- Objective: Wrap the pure helpers in `#[slicer_module] impl LayerModule` (Branch B) or the Go port (Branch A), reading config through `from_config` and per-region overrides through `slicer_sdk::config_resolution::resolve_float`, emitting sparse paths over `sparse_infill_area()` gated by `should_emit(ExtrusionRole::SparseInfill)`.
- Precondition: Step 3 pure helpers green; Step 1 branch selected.
- Postcondition: `run_infill` emits `SparseInfill` paths over the sparse polygon only, honors per-region `tiling_depth`, and (in Branch B) the `dragon_config_override_tdd.rs` test is green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `48-287` (module shape + `run_infill` + per-region resolution).
  - `crates/slicer-sdk/src/config_resolution.rs` - full read (`resolve_float`).
  - `crates/slicer-sdk/src/views.rs` - lines `440-572` (`sparse_infill_area`, `should_emit`).
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/src/lib.rs`
  - `modules/community-modules/dragon-curve/tests/dragon_config_override_tdd.rs`
  - `modules/community-modules/dragon-curve/Cargo.toml` (Branch B: crate + deps + `[workspace]` sentinel)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/traits.rs` (delegate the `run_infill` signature if unsure; do not load the 1821-line file).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: exact `LayerModule::run_infill` signature and the `InfillOutputBuilder` sparse-push method? scope: `crates/slicer-sdk/src/traits.rs::run_infill`, `crates/slicer-sdk/src/builders.rs::push_sparse_path`; return: `LOCATIONS` (≤10 entries).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §5 - direct read (minimal scope + per-region overrides).
- OrcaSlicer refs:
  - none.
- Verification:
  - `cd modules/community-modules/dragon-curve && cargo test --test dragon_config_override_tdd per_region_tiling_depth_override -- --exact` - FACT pass/fail (AC-6).
  - `cd modules/community-modules/dragon-curve && cargo test` - FACT: module suite green (excluding deferred AC-7).
- Exit condition: AC-6 green and the module suite (non-226) is green.

### Step 5: Build script, committed `.wasm`, banner README, and the deferred emission wiring

- Task IDs: `TASK-338`
- Objective: Author the `Makefile` (branch-appropriate build + `wasm-tools` componentize), produce the committed `dragon-curve.wasm`, write the banner `README.md`, and (only after 226 lands) wire `tool_index = Some(map_tiling_index_to_tool(...))` into the emitted paths and add `dragon_emission_tdd.rs`.
- Precondition: Step 4 green; locked assumption L1 (xtask discovery) re-verified via dispatch.
- Postcondition: `Makefile` + `dragon-curve.wasm` + `README.md` exist; the manual slice command is documented against `resources/regression_wedge.stl`; AC-7's emission test is authored but stays gated behind the 226 FORWARD-DEP.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` - lines `509-623` (`build_one_inner` strip + componentize sequence).
  - `xtask/src/build_guests.rs` - lines `107-287` (`discover_guests` roots, for L1).
  - `modules/core-modules/rectilinear-infill/Cargo.toml` - full read (dependency path + `[workspace]` sentinel model).
- Files allowed to edit (at most 3):
  - `modules/community-modules/dragon-curve/Makefile`
  - `modules/community-modules/dragon-curve/README.md`
  - `modules/community-modules/dragon-curve/tests/dragon_emission_tdd.rs` (FORWARD-DEP on 226; authored but deferred)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/host.rs` (the `tool_count` wrapper body — read only via 226's packet at activation).
  - `docs/*.md` (packet 228 owns docs).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does `discover_guests` walk only `modules/core-modules` and `crates/slicer-wasm-host/test-guests`, never `modules/community-modules`? scope: `xtask/src/build_guests.rs::discover_guests`; return: `FACT`.
  - Question: what is the exact `wasm-tools` strip pattern and `component new` invocation for a core guest? scope: `xtask/src/build_guests.rs::build_one_inner`; return: `SUMMARY` (≤100 words).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-infill.md` §1 - direct read (packaging + build/artifact + banner README).
- OrcaSlicer refs:
  - none.
- Verification:
  - `ls modules/community-modules/dragon-curve/{Makefile,README.md,dragon-curve.wasm}` - FACT: all three present.
  - `rg -q 'pnp_cli slice --module-dir modules/community-modules/dragon-curve --input resources/regression_wedge.stl' modules/community-modules/dragon-curve/README.md` - FACT pass/fail.
  - `cd modules/community-modules/dragon-curve && cargo test --test dragon_emission_tdd -- --exact` - deferred; note the FORWARD-DEP on 226 in the step log.
- Exit condition: the module directory is complete and the manual slice command is documented; AC-7 remains deferred by the 226 blocker.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | read-only discovery; delegated verdict read |
| Step 2 | S | manifest + one dispatch |
| Step 3 | M | tiling + color logic, TDD |
| Step 4 | M | module wiring + per-region override |
| Step 5 | M | build + componentize + banner + deferred emission |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS (AC-7 excepted — it is gated on the 226 FORWARD-DEP and moves to PASS when 226 lands).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (packet 228 owns the backlog rows; this packet only touches TASK-338's status if 228 has not yet landed).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (the two `[FWD]` questions in `design.md`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
