# Implementation Plan: 246-wave-overhang-bridge-fill

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: internal-bridge-areas view accessor

- Task IDs: `TASK-356`
- Objective: expose `SlicedRegion.internal_bridge_areas` through the WIT `slice-region-view` resource,
  the SDK `SliceRegionView`, the macro adapter, and the marshal.
- Precondition: `SlicedRegion.internal_bridge_areas: Vec<ExPolygon>` exists
  (`crates/slicer-ir/src/slice_ir.rs`); the `slice-region-view` resource and SDK view compile today.
- Postcondition: `AC-1` passes; the module can read `internal_bridge_areas` from `SliceRegionView`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines `125-150` (slice-region-view resource)
  - `crates/slicer-sdk/src/views.rs` - lines `20-60` (struct), `260-280` (setter), `440-450` (getter)
  - `crates/slicer-macros/src/lib.rs` - lines `2402-2470` (SliceRegionView projection)
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines `371-520` (SliceRegionData assembly)
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/slice_ir.rs` (the field already exists; do not edit)
- Blast-radius discipline: the SDK `SliceRegionView` gains a field; list the struct-literal blast
  radius — every test/non-test site that constructs `SliceRegionView` exhaustively (e.g.
  `crates/slicer-sdk/tests/test_support_slice_region_view_builder_tdd.rs` and the
  `SliceRegionView::default()`/`from_region` constructors in `views.rs`). Dispatch a `LOCATIONS`
  worker for `SliceRegionView {` literal sites before authoring this step.
- Expected sub-agent dispatches:
  - Question: every `SliceRegionView {` struct-literal site (test and non-test) that must gain the
    new field or a FRU rest. scope: `crates/slicer-sdk/**`, `crates/slicer-runtime/**`,
    `crates/slicer-wasm-host/**`; return: `LOCATIONS` (≤20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - §"Holder identifier matching" (delegated SUMMARY if needed)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-sdk --test test_support_slice_region_view_builder_setters_tdd 2>&1 | tee target/test-output.log | grep -qE "^test result: ok"`
  - `cargo xtask build-guests` then `cargo xtask build-guests --check` (exit 0)
- Exit condition: the SDK setter test passes, the four `rg` checks in AC-1 match, and
  `build-guests --check` exits 0.

### Step 2: module scaffold + manifest + registration + holder selection

- Task IDs: `TASK-356`
- Objective: create `modules/core-modules/wave-overhangs/` (Cargo.toml, wave-overhangs.toml,
  src/lib.rs stub, src/generator.rs stub, tests/, wit-guest/), register it in the workspace,
  `slicer-integrated-modules`, and `pnp-cli`, and author the holder-matching test proving
  `bridge_fill_holder = "wave-overhangs"` selects the module.
- Precondition: Step 1 complete.
- Postcondition: `AC-2` and `AC-3` pass; the module compiles, is discoverable, and is selectable as
  the bridge-fill holder via short-name match.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/gyroid-infill/Cargo.toml` - full
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml` - full
  - `modules/core-modules/gyroid-infill/src/lib.rs` - full (scaffold shape)
  - `modules/core-modules/gyroid-infill/wit-guest/Cargo.toml` - full
  - `Cargo.toml` - workspace members block
  - `crates/slicer-integrated-modules/Cargo.toml` - full
  - `crates/pnp-cli/Cargo.toml` - full
  - `crates/slicer-scheduler/src/validation.rs` - lines `55-105` (`module_id_matches_holder`,
    `resolve_held_claims`)
  - `crates/slicer-scheduler/tests/contract/main.rs` - full (mod list)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wave-overhangs/Cargo.toml` (new)
  - `modules/core-modules/wave-overhangs/wave-overhangs.toml` (new)
  - `modules/core-modules/wave-overhangs/src/lib.rs` (new)
  - `modules/core-modules/wave-overhangs/src/generator.rs` (new)
  - `modules/core-modules/wave-overhangs/wit-guest/Cargo.toml` (new)
  - `Cargo.toml`
  - `crates/slicer-integrated-modules/Cargo.toml`
  - `crates/pnp-cli/Cargo.toml`
  - `crates/slicer-scheduler/tests/contract/holder_matching_tdd.rs` (new)
  - `crates/slicer-scheduler/tests/contract/main.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/rectilinear-infill/**` (no sharing, ADR-0026)
  - `crates/slicer-scheduler/src/validation.rs` (read-only; the matching rule is unchanged)
- Blast-radius discipline: not applicable (new crate, no existing struct literals).
- Expected sub-agent dispatches:
  - Question: the exact `gyroid-infill` `src/lib.rs` `run`/`from_config` shape and the
    `InfillOutputBuilder` API for emitting `ExtrusionPath3D` with a role and width. scope:
    `modules/core-modules/gyroid-infill/src/lib.rs`; return: `SNIPPETS` (≤30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - manifest field reference (lines ~641-744) and §"Holder identifier
    matching" (lines ~746-763)
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p wave-overhangs --all-targets 2>&1 | tee target/test-output.log | grep -qE "Finished|error" && echo P246_SCAFFOLD_COMPILES`
  - `cargo test -p slicer-scheduler --test contract -- module_id_matches_holder_wave_overhangs --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo xtask build-guests` then `cargo xtask build-guests --check` (exit 0)
- Exit condition: the crate compiles, the manifest `rg` checks in AC-2 match, the holder-matching
  test passes, and the guest builds fresh.

### Step 3: generator port + region pipeline

- Task IDs: `TASK-356`
- Objective: port the canonical `WaveOverhangs.cpp` generator as an own copy in `src/generator.rs` and
  wire the region pipeline (supported_fill → anchor_band → wave_domain → waves/fallback) in
  `src/lib.rs`, with the porting header.
- Precondition: Step 2 complete.
- Postcondition: `AC-4`, `AC-5`, `AC-6`, `AC-7`, `AC-9` pass; waves are order-locked, anchor-first,
  internal-excluded, with rectilinear fallback.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wave-overhangs/src/lib.rs` - full (own file)
  - `modules/core-modules/wave-overhangs/src/generator.rs` - full (own file)
  - `crates/slicer-core/src/polygon_ops.rs` - `offset`/`intersection`/`difference`/`union` signatures
  - `crates/slicer-ir/src/slice_ir.rs` - `mm_to_units` (line ~62)
  - `crates/slicer-gcode/src/emit.rs` - `resolve_feedrate` (lines ~144-187) and volumetric-E loop
    (lines ~554-570)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wave-overhangs/src/lib.rs`
  - `modules/core-modules/wave-overhangs/src/generator.rs`
  - `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate only)
  - `modules/core-modules/rectilinear-infill/**`
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: canonical `WaveOverhangs.cpp` seed-extraction, offset-loop, front-extraction, and
    pattern-assembly steps and their constants. scope: `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp`; return: `SUMMARY` (≤200 words, no code)
  - Question: how `rectilinear-infill` resolves bridge orientation, bridge width/nozzle fallback, and
    bridge spacing/flow today (the fallback must mirror it). scope: `modules/core-modules/rectilinear-infill/src/`; return: `SUMMARY` (≤200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - coordinate hazard
  - `docs/ORCASLICER_ATTRIBUTION.md` - porting header
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp` - delegate; never load
- Verification:
  - `cargo test -p wave-overhangs --test wave_overhangs_tdd 2>&1 | tee target/test-output.log | grep -qE "^test result: ok"`
- Exit condition: the full `wave_overhangs_tdd` binary is green (waves, fallback, exclusion,
  speed/flow, determinism).

### Step 4: negative cases (clamp rejection, internal disjointness, no-holes)

- Task IDs: `TASK-356`
- Objective: add the negative tests proving fatal speed-factor rejection, internal-area disjointness,
  and missing/narrow-anchor no-holes fallback.
- Precondition: Step 3 complete.
- Postcondition: `AC-N1`, `AC-N2`, `AC-N3` pass.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs` - full (own file)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/layer_executor.rs` (InternalBridgeInfill constructor untouched)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: none beyond the plan.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p wave-overhangs --test wave_overhangs_tdd -- speed_factor_out_of_clamp_rejected --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p wave-overhangs --test wave_overhangs_tdd -- locked_footprint_disjoint_from_internal --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p wave-overhangs --test wave_overhangs_tdd -- missing_and_narrow_anchor_no_holes --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
- Exit condition: the three named tests pass.

### Step 5: end-to-end discriminator (A_upsidedown.obj)

- Task IDs: `TASK-356`
- Objective: author the end-to-end test proving a contiguous order-locked `BridgeInfill` block in
  typed capture plus emitted wave speed/volume matching config, over `resources/A_upsidedown.obj`.
- Precondition: Steps 1-4 complete; `resources/A_upsidedown.obj` exists.
- Postcondition: `AC-8` passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_arbiter_e2e_tdd.rs` - lines `1-120`
    (typed-capture driver precedent)
  - `crates/slicer-runtime/tests/e2e/main.rs` - full (module list)
  - `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` - lines `200-280`
    (`full_pipeline_with_typed_layer_dispatch` pattern, fallback driver)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/e2e/wave_overhang_bridge_fill_e2e_tdd.rs` (new)
  - `crates/slicer-runtime/tests/e2e/main.rs`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (orchestrator owns it)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does the `pnp_cli visual-debug` typed-capture for a `Layer::InfillPostProcess` tap
    expose `ExtrusionPath3D.order_lock` in its JSON? scope: `crates/slicer-runtime/src/visual_debug_render.rs` + the visual-debug request schema; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` - visual-debug bundle/tap contract (delegated SUMMARY)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test e2e -- wave_overhang_bridge_fill_e2e --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
- Exit condition: the named test passes. If the visual-debug typed-capture does not expose
  `order_lock`, use the `run_pipeline`-based capturing runner instead (the `[FWD]` in `design.md`).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | view accessor + struct-literal blast radius |
| Step 2 | M | scaffold + registration |
| Step 3 | M | generator port (largest) |
| Step 4 | S | negative tests |
| Step 5 | M | end-to-end discriminator |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
