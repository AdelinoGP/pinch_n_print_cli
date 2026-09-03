# Implementation Plan: 240a-support-raft-substrate

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (`TASK-409`..`TASK-413`, `TASK-531`..`TASK-534` only).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled
  independently; never write "see Step N".
- Guest-facing steps: run `cargo xtask build-guests --check` and judge by its
  exit code before attributing any failure; a step that edits guest code
  rebuilds guests in-step.
- WIT-editing steps end with `cargo build --tests`.
- All test commands tee to `target/test-output.log`; read results from the file,
  never re-run for more output.
- A new test file under an aggregated `slicer-runtime` binary gets its
  `mod` registration in the SAME step, or it compiles to zero tests and reports
  a false pass.

## Steps

### Step 1: Author signed-index + carrier IR tests (red)

- Task IDs: `TASK-409`
- Objective: create the failing tests pinning AC-1 and AC-6's Rust half —
  `crates/slicer-ir/tests/signed_layer_indices_tdd.rs` (compile-time type
  assertions on the migrated fields, serde round-trip of a `SliceIR` with
  `global_layer_index: -2`, ordering of `-2 < -1 < 0`) and
  `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` (`raft_fill` defaults
  empty, serde-default backward compat with a 4.8.0 fixture, round-trip
  stability).
- Precondition: clean tree; 236 confirmed `implemented` (re-derive with
  `grep '^status:' docs/spec_packets/236-support-stabilization/packet.spec.md`).
- Postcondition: both files exist and fail to compile / fail asserts for
  exactly the intended reasons (missing `i32` types, missing `raft_fill`).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - symbol-scoped ranges only, located at
    read time via `rg -n 'pub struct (GlobalLayer|ObjectLayerRef|SlicedRegion|SliceIR)'`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/tests/signed_layer_indices_tdd.rs` (new)
  - `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` (new)
- Files explicitly out of bounds: everything else under `crates/`, all modules
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: `docs/specs/support-families-anchored-entities-plan.md` §12
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd 2>&1 | tee target/test-output.log; grep -q 'error\[' target/test-output.log || grep -q 'FAILED\|panicked' target/test-output.log` - FACT: RED confirmed
- Exit condition: log shows the new tests red for missing `i32` types / missing
  `raft_fill` — nothing else broken.

### Step 2a: Signed-index migration, crates half

- Task IDs: `TASK-410`
- Objective: retype every field in `design.md` §Migration Table that lives
  under `crates/`, change `LayerModule::run_infill`'s parameter to `i32`,
  retype `PaintRegionLayerView.layer_index` and its getter, and fix the
  crates-side blast-radius sites so the workspace compiles. Do NOT touch the
  positional-consumer logic yet (Step 3) beyond what compilation demands.
- Precondition: Step 1 red; **LOCATIONS sweep dispatched and its result pasted
  into this step's working notes before editing** (question verbatim in
  `design.md` §Enumerated Blast Radius). If the crates half exceeds ~20 files,
  STOP and split again with fresh IDs re-derived from the free range.
- Postcondition: `cargo check -p slicer-ir -p slicer-sdk -p slicer-macros -p slicer-wasm-host --all-targets` green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - ranged reads around each hit only
  - LOCATIONS sweep output (working notes)
- Files allowed to edit (at most 3 primaries; sweep fallout under `crates/` is
  owned here and listed explicitly in the working notes before the first edit,
  so the cap is a review checkpoint rather than a blank cheque):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-macros/src/lib.rs`
  plus the LOCATIONS-listed call sites under `crates/` edited strictly to
  restore compilation — no behavior change beyond sign semantics.
- Files explicitly out of bounds: `modules/**`, all test files (Step 2b)
- Blast-radius discipline: production struct literals stay exhaustive.
- Expected sub-agent dispatches:
  - LOCATIONS blast-radius sweep; scope `crates/`; return LOCATIONS with
    per-file counts; purpose: edit list
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` - delegated SUMMARY of the
  layer-index sections
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo check -p slicer-ir -p slicer-sdk -p slicer-macros -p slicer-wasm-host --all-targets 2>&1 | tail -5 | tee target/test-output.log` - FACT pass/fail
- Exit condition: the four crates check green with `--all-targets`.

### Step 2b: Signed-index migration, modules + tests half

- Task IDs: `TASK-411`
- Objective: fix the remaining blast-radius sites under `modules/` and in test
  code so `cargo check --workspace --all-targets` is green and AC-1 passes.
- Precondition: Step 2a green.
- Postcondition: `cargo check --workspace --all-targets` green; AC-1 command
  green.
- Files allowed to read, with ranges when over 300 lines:
  - LOCATIONS sweep output (working notes)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - only ranges the
    sweep names; never the whole ~5.9k-line file
- Files allowed to edit (at most 3 primaries plus the LOCATIONS-listed test and
  module files, enumerated in the working notes before editing):
  - `crates/slicer-sdk/src/test_support/fixtures.rs`
  - `crates/slicer-macros/tests/binding_surface_tdd.rs`
  - `crates/slicer-macros/tests/slicer_module_tdd.rs`
- Files explicitly out of bounds: `modules/core-modules/raft-default/**` (240b)
- Blast-radius discipline: any watched-type struct literal in TEST code gains a
  `..` rest or an `// exhaustive: <reason>` waiver per
  `docs/21_data_defaults_and_fixtures.md`; tests hard-asserting `u32`
  wrap/ordering get sign-correct updates, never deletion.
- Expected sub-agent dispatches:
  - LOCATIONS sweep, `modules/` + `tests/` scope; return LOCATIONS; purpose:
    edit list
- Context cost: `M`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tail -5 | tee target/test-output.log` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-1 green
  - `cargo xtask check-literals` - FACT exit 0
- Exit condition: AC-1 green with non-zero count; workspace check green;
  literal gate exit 0.

### Step 3: Kill the sign-truncating bridge cast

- Task IDs: `TASK-412`
- Objective: remove the `paint.layer_index() as u32` truncation in the
  `slicer-macros` paint-view bridge and every other `as u32` applied to a layer
  index, so a negative index survives the WIT→SDK hop. Author
  `negative_layer_index_survives_paint_view_bridge` in the existing
  `crates/slicer-macros/tests/binding_surface_tdd.rs`.
- Precondition: Step 2b green.
- Postcondition: AC-2 green; a repo-wide grep finds no `as u32` applied to a
  layer-index expression.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/src/lib.rs` - the paint-view bridge range only
    (locate with `rg -n 'layer_index\(\) as u32'`)
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-macros/tests/binding_surface_tdd.rs`
  - `crates/slicer-sdk/src/traits.rs` (only if the getter signature needs a
    follow-up touch)
- Files explicitly out of bounds: runtime consumers (Step 4), WIT files
- Expected sub-agent dispatches:
  - LOCATIONS: every `as u32` within 3 lines of a `layer_index` /
    `global_layer_index` identifier; scope `crates/ modules/`; return LOCATIONS
- Context cost: `S`
- Authoritative docs: `docs/03_wit_and_manifest.md` - delegated SUMMARY of the
  guest-bridge section
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-macros --test binding_surface_tdd -- negative_layer_index_survives_paint_view_bridge --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-2
  - `rg -n 'layer_index\(\) as u32' crates/ modules/; test $? -ne 0` - FACT: truncation gone
- Exit condition: AC-2 green; the truncation grep finds nothing.

### Step 4: Repair the positional consumers

- Task IDs: `TASK-413`
- Objective: apply `design.md` §Positional Consumer Ruling verbatim — convert
  `hydrate_slice_arena`'s `slice_vec.get(layer.index as usize)` to an identity
  lookup, re-key `raw_polygons_by_layer` to `HashMap<i32, _>`, convert the
  `support_analysis_producer.rs` Z lookup to find-by-index, re-key the
  `native.rs` resolved-config carry-over by `index`, and LEAVE the two rulings
  marked "leave" untouched. Author
  `negative_index_layer_hydrates_slice_arena` and
  `raft_layer_below_geometry_slices_empty_not_fatal` in
  `crates/slicer-runtime/tests/executor/`, registering both `mod` lines.
- Precondition: Step 3 green.
- Postcondition: AC-3 and AC-N3 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - the `hydrate_slice_arena`
    range only
  - `crates/slicer-runtime/tests/executor/main.rs` - registration list only
- Files allowed to edit (at most 3 primaries + the two new/registered test
  files, which are named here so the registration cannot be forgotten):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  - `crates/slicer-runtime/tests/executor/raft_negative_index_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/main.rs` (add `mod raft_negative_index_tdd;`)
- Files explicitly out of bounds: `crates/slicer-wasm-host/src/marshal/in_.rs`
  (Step 5), the perimeter generators
- Expected sub-agent dispatches:
  - FACT: does `SupportGeometryKey.global_support_layer_index` index a
    `Vec<LayerCollisionCache>` directly anywhere? scope `crates/`; return
    LOCATIONS — resolves the [FWD] open question
- Context cost: `M`
- Authoritative docs: none beyond `design.md`
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- negative_index_layer_hydrates_slice_arena --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N3
  - `grep -q 'mod raft_negative_index_tdd;' crates/slicer-runtime/tests/executor/main.rs` - FACT registration present
- Exit condition: both ACs green with non-zero counts; registration grep passes;
  the two "leave positional" sites are unchanged in `git diff`.

### Step 5: WIT prefix marking + negative index assignment on both legs

- Task IDs: `TASK-531`
- Objective: add `is-raft-prefix: bool` to `layer-proposal`
  (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`);
  in `harvest_layer_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  and the `PrePass::LayerPlanning` arm of
  `crates/slicer-wasm-host/src/marshal/native.rs`, count the leading
  prefix-marked run of length `N`, assign it `-N .. -1` in push order and the
  remainder `0 ..`, re-derive `MAX_LAYERS` signed with a lower bound, re-key
  the native arm's resolved-config carry-over by `index`, and reject a
  non-contiguous prefix run with a typed error naming the push position.
  Author `crates/slicer-wasm-host/tests/marshal_layer_plan_prefix_tdd.rs`.
- Precondition: Step 4 green.
- Postcondition: AC-4 and AC-N1 green; `cargo build --tests` green after the
  WIT edit; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - the `harvest_layer_plan_ir_from` range
  - `crates/slicer-wasm-host/src/marshal/native.rs` - the `PrePass::LayerPlanning` arm
- Files allowed to edit (at most 3 primaries + the new test file):
  - `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-wasm-host/tests/marshal_layer_plan_prefix_tdd.rs` (new;
    auto-discovered, no registration needed)
- Files explicitly out of bounds: `modules/**` (Step 6), `ir-types.wit`
- Expected sub-agent dispatches:
  - OrcaSlicer SUMMARY: `generate_support_layers` below-zero print_z insertion;
    return SUMMARY; purpose: confirm band semantics
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate; never load
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- prefix_band_indices_are_negative_on_both_legs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-4
  - `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- noncontiguous_prefix_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N1
- Exit condition: AC-4 and AC-N1 green; guests fresh (exit 0); both legs assign
  identical indices for the same push sequence.

### Step 6: layer-planner-default emits the raft prefix band

- Task IDs: `TASK-532`
- Objective: teach `com.core.layer-planner-default` to read
  `support_raft_layers` (declared in its manifest `[config.schema]` so E9's
  silent-default trap cannot fire) and push exactly that many
  `is-raft-prefix: true` proposals before any model proposal, with Z computed
  in `f64` and cast once at the end, and at least one `active_regions` entry
  per raft layer. Author the two integration cases plus the monotonic-gate
  case, registering their `mod` line.
- Precondition: Step 5 green.
- Postcondition: AC-5 and AC-N2 green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/layer-planner-default/src/lib.rs` - the
    `generate_object_layers` and `DefaultLayerPlanner::from_config` ranges
    (there is no `LayerPlannerConfig` type; the config is read by
    `DefaultLayerPlanner`'s `from_config`)
  - `crates/slicer-runtime/tests/integration/main.rs` - registration list only
- Files allowed to edit (at most 3 primaries + the new/registered test files):
  - `modules/core-modules/layer-planner-default/src/lib.rs`
  - `modules/core-modules/layer-planner-default/layer-planner-default.toml`
  - `crates/slicer-runtime/tests/integration/raft_prefix_band.rs` (new; holds
    `raft_prefix_band_emitted_before_model_layers`,
    `no_raft_prefix_band_when_raft_layers_zero`,
    `raft_band_satisfies_finalization_monotonic_gate`)
  - `crates/slicer-runtime/tests/integration/main.rs` (add `mod raft_prefix_band;`)
- Files explicitly out of bounds: the support planners, `modules/core-modules/raft-default/**`
- Expected sub-agent dispatches:
  - FACT: what does `derive_layer_output_envelope_from_input` return for a
    layer with empty `active_regions`? scope `crates/slicer-wasm-host/src/dispatch.rs`;
    return SNIPPETS ≤10 lines — resolves the [FWD] seeding question
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` - delegate: `generate_object_layers` f64 discipline; return SUMMARY
- Verification:
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_prefix_band_emitted_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- no_raft_prefix_band_when_raft_layers_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-5
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N2
  - `grep -q 'mod raft_prefix_band;' crates/slicer-runtime/tests/integration/main.rs` - FACT registration present
- Exit condition: AC-5 and AC-N2 green; registration grep passes; guests fresh.

### Step 7: SlicedRegion.raft_fill carrier + WIT accessors + schema bump

- Task IDs: `TASK-533`
- Objective: add `pub raft_fill: Vec<ExPolygon>` with `#[serde(default)]` to
  `SlicedRegion`; add the `raft-fill` accessor to BOTH region resources in
  `crates/slicer-schema/wit/deps/ir-types.wit`; add
  `split_field!(raft_fill);` in `crates/slicer-runtime/src/region_partition.rs`;
  project the field through the host accessor impls, the macro marshal legs,
  the SDK views and both fixture builders, the visual-debug render and the
  pnp-cli manifest emission; minor-bump `CURRENT_SLICE_IR_SCHEMA_VERSION` to
  `4.9.0` with a version-history doc-comment line and update every test
  asserting the old value in the same step. Follow
  `design.md` §`raft_fill` Carrier Footprint as the site checklist.
- Precondition: Step 6 green.
- Postcondition: AC-6 green; `cargo build --tests` green after the WIT edit.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - the `SlicedRegion` and
    `CURRENT_SLICE_IR_SCHEMA_VERSION` ranges, located at read time
  - `crates/slicer-runtime/src/region_partition.rs` - the `split_field!` block
- Files allowed to edit (at most 3 primaries; the remaining footprint sites
  from `design.md` are owned here and enumerated in the working notes before
  the first edit):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-runtime/src/region_partition.rs`
- Files explicitly out of bounds: `modules/**`, the scheduler
- Blast-radius discipline: grep `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion
  sites and update every one in THIS step (bump + fallout together).
- Expected sub-agent dispatches:
  - LOCATIONS: `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion sites; scope
    `crates/`; return LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` - SliceIR section; delegated
  SUMMARY before editing
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-6 Rust half
  - `test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs` - FACT: both resources + region split
- Exit condition: AC-6 green; exactly two WIT accessors; the split line present;
  no test still asserts the pre-bump version.

### Step 8: raft_plan read accessor

- Task IDs: `TASK-534`
- Objective: declare a `raft-plan-view` record in
  `crates/slicer-schema/wit/deps/ir-types.wit` (local mirror of the prepass
  `raft-plan`, no cross-world import) and a `raft-plan` accessor on
  `paint-region-layer-view`; add `PaintRegionLayerData.raft_plan` in
  `crates/slicer-wasm-host/src/host.rs` with an accessor impl that pushes
  `"SupportPlanIR"` to `runtime_reads`; populate it in
  `build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
  with no layer filter; mirror it in the `slicer-macros` guest shim; add
  `PaintRegionLayerView::raft_plan()` in `crates/slicer-sdk/src/traits.rs`. The
  native leg needs no further change. Move the 8 existing
  `PaintRegionLayerData` construction sites to FRU or `Default`. Author
  `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs`.
- Precondition: Step 7 green.
- Postcondition: AC-7 green; `cargo build --tests` green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/host.rs` - the `support_plan_entries` impl range
  - `crates/slicer-wasm-host/src/dispatch.rs` - the
    `build_paint_layer_data_with_plan` range
- Files allowed to edit (at most 3 primaries + the SDK getter and the new test
  file, both named here):
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` (new)
- Files explicitly out of bounds: `modules/**`, `region_partition.rs`
- Expected sub-agent dispatches:
  - FACT: does `ir-types.wit` resolve with a locally-declared `raft-plan-view`
    record (no cross-world import / world-satisfaction failure)? scope
    `crates/slicer-schema/wit/`; return FACT
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` - delegated SUMMARY
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-7
- Exit condition: AC-7 green; guests fresh; a `Layer::Infill` guest can read
  `raft_plan` on both legs.

### Step 9: DEV-124 reopen row + docs

- Task IDs: `TASK-534`
- Objective: file the DEV-124 reopen deviation row per `requirements.md`
  §DEV-124 Reopen — re-derive the next free ID at write time
  (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`), name
  the two pinning tests, and state the corrected predicate. Do NOT edit the
  perimeter generators. Update `docs/02_ir_schemas.md` (SliceIR section:
  `raft_fill`, signed indices, the `-N .. -1` raft prefix band, and the
  `index != Vec position` consequence) and `docs/03_wit_and_manifest.md`
  (`is-raft-prefix`, `raft-plan-view`, the `raft-fill` accessors).
- Precondition: Steps 1-8 green.
- Postcondition: every `packet.spec.md` §Doc Impact Statement grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - SliceIR section only
  - `docs/DEVIATION_LOG.md` - the header row and the last 3 rows only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
- Files explicitly out of bounds: `docs/adr/**` (240b owns the ADR amendment),
  the perimeter generators and their contract tests
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: `docs/adr/0009-raft-as-layer-infill-role.md` - direct read
  (93 lines), for the negative-prefix contract wording only
- OrcaSlicer refs: none this step
- Verification:
  - `rg -q 'raft_fill' docs/02_ir_schemas.md && rg -q 'raft prefix band' docs/02_ir_schemas.md` - FACT
  - `rg -q 'is-raft-prefix' docs/03_wit_and_manifest.md && rg -q 'raft-plan-view' docs/03_wit_and_manifest.md` - FACT
  - `rg -q 'raft prefix band' docs/DEVIATION_LOG.md && cargo xtask check-deviations; echo EXIT:$?` - FACT exit 0
- Exit condition: all Doc Impact greps pass; `check-deviations` exit 0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red tests, IR-only |
| Step 2a | M | crates half of the migration; split again if >~20 files |
| Step 2b | M | modules + tests half; literal gate runs here |
| Step 3 | S | truncation removal; narrow |
| Step 4 | M | positional-consumer ruling applied verbatim |
| Step 5 | M | WIT + both harvest legs |
| Step 6 | M | planner emits band; guest rebuild |
| Step 7 | M | carrier footprint + schema bump |
| Step 8 | M | raft-plan read path |
| Step 9 | S | deviation row + docs |

Aggregate is `L`; no single step is `L`. `design.md` §Why This Packet Carries
An L records why the aggregate cannot be reduced further, and activation
therefore requires the swarm extended band with a logged ESCALATION.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask check-literals` exit 0 (count equal to the count recorded on a
  clean tree immediately BEFORE this packet's first edit — re-derive it then;
  do not trust any number written here).
- Update `docs/07_implementation_status.md` with `TASK-409`..`TASK-413` and
  `TASK-531`..`TASK-534` through a worker dispatch, never a full backlog read.
- Confirm 240b is unblocked: AC-1..AC-7 green.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace` once, dispatched to a sub-agent
  under the FACT contract — this packet retypes a field touched by most of the
  workspace, so the narrow runs alone do not bound the risk.
- Record remaining packet-local risk (leg skew on future transports; any
  layer-parallel Vec added later that re-introduces positional indexing).
- Confirm context stayed within the escalated band; otherwise record a
  packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and
verification commands use `--all-targets` where applicable so test, bench, and
example targets compile.
