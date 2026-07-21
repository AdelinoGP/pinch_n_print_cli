# Implementation Plan: 140_lightning-module-rewrite

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

### Step 0: WIT extension bundle (RED→GREEN) — closes `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW`

- Task IDs: `TASK-265`
- Objective: extend the WIT `run-infill` signature at
  `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25` with
  `paint: paint-region-layer-view`; bump the package version 2.2.0 → 2.3.0;
  extend the SDK trait `LayerModule::run_infill` at
  `crates/slicer-sdk/src/traits.rs:369-377` to take the new `_paint:
  &PaintRegionLayerView` parameter; update the slicer-macros `infill_arm`
  at `crates/slicer-macros/src/lib.rs:1779-1794` and the macro-emitted
  `fn run_infill` glue at `:2804-2809` to pass the new arg through; rewrite
  the host dispatch `Layer::Infill` arm at
  `crates/slicer-wasm-host/src/dispatch.rs:442-465` to mirror the
  `Layer::Support` arm at `:584-619` (build a `PaintRegionLayerData` via
  `build_paint_layer_data_with_plan(...)` and push it); update the four
  `run_infill`-implementing core modules (rectilinear/gyroid/lightning/top-
  surface-ironing) to take the new `_paint: &PaintRegionLayerView` argument
  (only `lightning-infill` will later use it; the other three bind and
  ignore); extend `layer-infill-guest/src/lib.rs:113` to add the
  `_paint: PaintRegionLayerView` argument and emit the witness path; re-
  baseline `wit_drift_detection_tdd.rs:592-616` with new assertions for
  the `run-infill` signature and `world-layer@2.3.0`; rebuild 33 guest
  artifacts via `cargo xtask build-guests`.
- Precondition: packet 137 status `implemented` (forward-dep), packet 139
  status `implemented` (forward-dep for the per-region keying contract this
  step depends on).
- Postcondition: AC-3a (WIT signature + version), AC-3c (four-module compile),
  AC-3d (WIT drift re-baseline) green; `cargo check --workspace --all-targets`
  clean; 33 guests fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (full; small).
  - `crates/slicer-sdk/src/traits.rs` (lines 350-400 for `LayerModule` trait;
    lines 50-200 for `PaintRegionLayerView`).
  - `crates/slicer-wasm-host/src/dispatch.rs` (lines 440-470 for the Infill
    arm; lines 580-620 for the Support arm; lines 1330-1410 for the
    `build_paint_layer_data_with_plan` function).
  - `crates/slicer-wasm-host/src/host.rs` (rg only for
    `HostPaintRegionLayerView`).
  - `crates/slicer-macros/src/lib.rs` (lines 1770-1810 for `infill_arm`;
    lines 2800-2820 for the macro-emitted glue).
  - `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs`
    (full; 309 lines).
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
    (rg only for the `run-infill-postprocess` string assertion at
    `:608-612`).
  - `modules/core-modules/{rectilinear,gyroid,lightning,top-surface-ironing}-infill/src/lib.rs`
    (rg only for `fn run_infill` line + 5-line context; do not load full files
    — they are 350-720 LOC each).
- Files allowed to edit (at most 11):
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (signature +
    version).
  - `crates/slicer-sdk/src/traits.rs` (one trait method signature).
  - `crates/slicer-macros/src/lib.rs` (the `infill_arm` + the macro-emitted
    glue; counts as one file).
  - `crates/slicer-wasm-host/src/dispatch.rs` (rewrite the Infill arm).
  - `modules/core-modules/rectilinear-infill/src/lib.rs` (signature update).
  - `modules/core-modules/gyroid-infill/src/lib.rs` (signature update).
  - `modules/core-modules/lightning-infill/src/lib.rs` (signature update —
    the body swap is Step 2, not here; this step only adds the param).
  - `modules/core-modules/top-surface-ironing/src/lib.rs` (signature update).
  - `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` (add
    paint arg + witness call).
  - `crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs`
    (new) + `crates/slicer-wasm-host/tests/contract/main.rs` (one `mod` line;
    counts as a second file in the wave) — role: AC-3b host-side test
    driver.
  - `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` (re-baseline
    the 6 `call_run_infill` call sites at lines 94, 192, 268, 352, 433, 497
    to add the paint arg).
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` (new
    drift assertions).
- Blast-radius discipline (mandatory for the WIT signature change):
  - **The `run-infill` signature change ripples through every guest
    implementation:** all 21 core-module `LayerModule` impls that re-bind
    on `world-layer.wit` (the 4 above + 17 others that don't implement
    `run_infill` but the macro generates the glue) re-stale. The macro-
    generated `fn run_infill` glue is the binding — if the trait
    signature and the WIT signature disagree, the macro-generated guest
    glue fails to compile.
  - **The `wit_boundary_tdd.rs` test at the same path** asserts WIT-boundary
    shapes; the 6 existing `call_run_infill` call sites (lines 94, 192, 268,
    352, 433, 497) MUST be re-baselined to add the paint arg after the
    trait signature change. This is the same shape of re-baseline done in
    packet 137 for `infill_holder_resolution_painted_region_tdd.rs`.
  - **The AC-3b host-side test file
    `lightning_infill_guest_calls_lightning_tree_segments_tdd.rs`** must
    be authored and registered in
    `crates/slicer-wasm-host/tests/contract/main.rs` (currently 12
    modules) — this is the S7 finding from the preflight gate. The
    file instantiates the rebuilt `layer-infill-guest.component.wasm`
    and asserts the guest's witness path encodes the
    `lightning-tree-segments` count. Without this, AC-3b's pipe command
    has no driver.
  - **The 12 test-guest artifacts re-stale** (21 + 12 = 33 total) on the
    macro + WIT + dispatch change; one `cargo xtask build-guests` rebuild
    is required BEFORE running any test that exercises a guest.
  - **The `infill_holder_resolution_painted_region_tdd.rs` test at
    `crates/slicer-wasm-host/tests/contract/`** may construct a
    `LayerStageInput` with the old shape; verify with a `rg` for
    `LayerStageInput \{` BEFORE this step edits.
  - **The `wit_boundary_tdd.rs` test at the same path** asserts WIT-boundary
    shapes; verify whether it pins the `run-infill` signature string and
    re-baseline if so.
- Files explicitly out-of-bounds for this step: `modules/core-modules/lightning-infill/src/lib.rs`
  BODY (the stub swap is Step 2, not here — this step only adds the
  parameter); `crates/slicer-core/src/algos/lightning/**` (139's surface);
  `OrcaSlicerDocumented/**`.
- Expected sub-agent dispatches:
  - "FACT: every call site of `call_run_infill` in
    `crates/slicer-wasm-host/src/dispatch.rs` (rg)" — Step 0 driver.
  - "FACT: every `LayerStageInput` construction site in the tree (rg
    `'LayerStageInput \{'`)" — Step 0 driver.
  - "FACT: every `LayerModule` impl in `modules/core-modules/*/src/lib.rs` (rg
    for `impl LayerModule`); confirm only 4 implement `run_infill`" — Step 0
    driver.
  - "FACT: every `call_run_infill` call site in
    `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` (rg);
    confirm 6 sites" — Step 0 driver.
  - "FACT: every `mod` declaration in
    `crates/slicer-wasm-host/tests/contract/main.rs`; confirm
    `lightning_infill_guest_calls_lightning_tree_segments_tdd` is missing
    (the S7 fix)" — Step 0 driver.
  - "Run `cargo check --workspace --all-targets`; FACT; SNIPPETS ≤ 30 on
    failure" — after all edits.
  - "Run `cargo xtask build-guests`; FACT; SNIPPETS ≤ 30 on failure" —
    post-WIT + macro change.
  - "Run `cargo test -p slicer-runtime --test contract -- wit_drift_detection`;
    FACT" — AC-3d verification.
  - "Run `cargo test -p slicer-wasm-host --test contract --
    lightning_infill_guest_calls_lightning_tree_segments`; FACT" — AC-3b
    verification.
  - "Run `rg -n 'run-infill: func\(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view' crates/slicer-schema/wit/deps/world-layer/world-layer.wit` + `rg -n 'package slicer:world-layer@2.3.0;' crates/slicer-schema/wit/deps/world-layer/world-layer.wit`; FACT each" — AC-3a.
- Context cost: `L` (justified: the WIT + trait + macro + dispatch + four-
  module + test-guest + drift-re-baseline + 33-guest rebuild are all
  coupled — partial state breaks every infill guest's instantiation at
  runtime and leaves the workspace un-compiling at the seam. The 140
  design's `§Context Cost Estimate` carries the justification.)
- Authoritative docs: ADR-0044 (WIT version-bump semantics),
  `docs/DEVIATION_LOG.md` `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` (the deviation
  being closed), `docs/03_wit_and_manifest.md` (the read-view contract).
- OrcaSlicer refs: none.
- Verification:
  - AC-3a, AC-3c, AC-3d pipe commands — FACT each
- Exit condition: WIT + trait + macro + dispatch + 4-module + test-guest +
  drift + 33-guest-rebuild green; per-module `run_infill` signature
  survey done.

### Step 1: Orca sampling-side FACT + RED suite

- Task IDs: `TASK-265`
- Objective: settle the `[FWD]` (delegated `Filler::_fill_surface_single` SUMMARY);
  classify the existing 323-line test file (keep / adapt / delete, each deletion
  naming the stub behavior it encoded); author the new RED tests
  (`samples_tree_ir_raw_emit`, `empty_trees_emit_nothing`); confirm the 137 view
  accessor is callable from the module (FACT: signature in
  `crates/slicer-sdk/src/traits.rs`); confirm the SDK re-exports reach the module
  (FACT: `slicer_sdk::traits::PaintRegionLayerView` is in scope per
  `modules/core-modules/lightning-infill/src/lib.rs:37` precedent).
- Precondition: Step 0 exit condition (WIT + trait + macro + dispatch + 4-module
  signature update in place).
- Postcondition: `[FWD]`s resolved and recorded; test classification recorded in the
  test-file header; new tests RED.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/lightning-infill/src/lib.rs` — full (one read; it gets
    replaced).
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — full
    (one read).
  - `crates/slicer-sdk/src/traits.rs` — lines 50-200 (the view accessor region).
- Files allowed to edit (at most 3):
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` (rewrite
    the suite; add a classification header comment naming each kept/adapted/deleted
    test).
- Blast-radius discipline: the new test file rewrites the existing 323-line file in
  place — no other test file references the stub APIs.
- Files explicitly out-of-bounds for this step: production `lib.rs` (RED first);
  `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the Filler SUMMARY dispatch (design §Expected Sub-Agent Dispatches)
  - "FACT: does `crates/slicer-sdk/src/traits.rs` expose
    `lightning_tree_segments_for(object_id, region_id) -> Vec<…>` (the 139 view
    accessor)? ≤ 5 lines" — `[FWD]` resolution.
  - "FACT: is `slicer_sdk::traits::PaintRegionLayerView` in scope for the module
    today (mirroring `SliceRegionView` at `lib.rs:38`)? ≤ 5 lines" — `[FWD]`
    resolution.
  - "Run `cargo test -p lightning-infill … | grep -E '^test |^test result'`; FACT
    per-test" — RED confirmation.
- Context cost: `M`
- Authoritative docs: `docs/specs/lightning-infill-parity.md` §L4.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — one
  delegated SUMMARY.
- Verification:
  - RED state FACT
- Exit condition: `[FWD]`s resolved; new tests RED; classification recorded in the
  test-file header.

### Step 2: GREEN — the sampler rewrite

- Task IDs: `TASK-265`
- Objective: replace the stub body with the sampler (view →
  `lightning_tree_segments_for` → raw `SparseInfill` emission, mm conversion at the
  boundary via `slicer_ir::units_to_mm`, origin discipline preserved at
  `begin_region`); delete `build_branches` + the grid machinery (`nearest_boundary_point`,
  `polygon_bbox_mm`, `point_in_expolygon`, `point_in_polygon`) + the
  `clip_polyline`/`connect_branches` (if any); adapt kept tests; AC-1, AC-2, AC-N2
  green; AC-3b real test-guest (added in Step 0) green.
- Precondition: Step 1 exit condition (the new `_paint: &PaintRegionLayerView`
  parameter is on the trait from Step 0; the module's `run_infill` impl accepts
  it; the dispatcher threads the view from Step 0).
- Postcondition: module suite green; structural greps clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/traits.rs` — lines 50-200 (the view accessor region).
- Files allowed to edit (at most 3):
  - `modules/core-modules/lightning-infill/src/lib.rs` (the rewrite; deletes the
    4-5 helper functions).
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` (adapt
    kept tests; the new RED tests become GREEN).
- Blast-radius discipline:
  - **The new `slicer_sdk` import** (`use slicer_sdk::traits::PaintRegionLayerView;`)
    is the only new public-symbol use; verify it compiles against the post-137
    `traits.rs` (FACT before edit; this is exactly the Step-1 `[FWD]`).
  - The `BASE_SPEED` constant at `lib.rs:41` and the `on_print_start` config reads
    at `lib.rs:73-95` are kept verbatim — they are reused by the sampler.
  - The new `_paint: &PaintRegionLayerView` parameter (added in Step 0) is the
    new `view`; the sampler reads `paint.lightning_tree_segments_for(object_id,
    region_id)` to get the per-region segments (139's per-region keying).
  - `slicer_module_binding_tdd.rs` (a different test file) is untouched in this
    packet.
- Files explicitly out-of-bounds for this step:
  `crates/slicer-core/src/algos/lightning/**` (triage fence).
- Expected sub-agent dispatches:
  - "Run `cargo test -p lightning-infill …`; FACT + counts; SNIPPETS ≤ 20 on
    failure".
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- Context cost: `M`
- Authoritative docs: ADR-0029 sampler contract (delegate).
- OrcaSlicer refs: none.
- Verification:
  - AC-1, AC-2, AC-N2, AC-3b pipe commands — FACT each
- Exit condition: module green; stub grep-gone; real test-guest traversing
  the WIT seam.

### Step 3: Pipeline uniformity + byte-identity guard

- Task IDs: `TASK-265`
- Objective: add `lightning_pipeline_linked` (AC-3: lightning-configured slice →
  linker → linked multi-point sparse polylines) and run the wedge guard (AC-N1).
- Precondition: Step 2 exit condition.
- Postcondition: AC-3, AC-N1 green.
- Files allowed to read: one neighboring executor test (idiom; ranged).
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) +
    `crates/slicer-runtime/tests/executor/main.rs` (one `mod` line; counts as a
    second file in the wave).
- Blast-radius discipline:
  - **The new test file's `mod` registration in `main.rs`** is a single-line edit;
    confirm `main.rs` exists and follow the existing `mod` declaration pattern
    (FACT before edit).
  - The new test file does not depend on any change to the linker source — the
    linker already handles raw `SparseInfill` paths (per packet 133's precedent).
- Files explicitly out-of-bounds for this step: module + linker sources (triage
  fence: failures are diagnosed to emission vs linking and routed, not patched
  here beyond the ≤ 20-line deviation allowance).
- Expected sub-agent dispatches:
  - "FACT: how is the executor test binary aggregated
    (`tests/executor/main.rs` mod list); list current mod declarations" — S7
    wiring check.
  - "Run `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked …`;
    FACT" — AC-3.
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N1.
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-3, AC-N1 pipe commands — FACT each
- Exit condition: uniformity + identity green.

### Step 4: Closure — DEV-081, D-137, contained bless, roadmap ceremony

- Task IDs: `TASK-265`
- Objective: resolve the `[FWD]` (FACT on the live DEV-081 row's status field —
  if `Closed` already, leave it; otherwise flip to `Closed` and add a reference
  note per the log's convention); resolve the `[FWD]` on the
  `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` row's status field (FACT — if
  `Closed` already, leave it; otherwise flip to `Closed` and add a reference
  note); re-bless lightning-affected expectations (two consecutive identical
  runs; per-expectation justification); docs/07 closure sweep for TASK-262…265
  (delegated — never load the full file); run the roadmap-close
  `cargo xtask test --workspace --summary` ceremony.
- Precondition: Step 3 exit condition (bless only after geometry/pipeline green).
- Postcondition: AC-4, AC-5 green; ceremony PASS recorded; packet + roadmap closed.
- Files allowed to read: none directly (all delegated).
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (DEV-081 row + `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW`
    row, if not already Closed)
  - lightning-affected expectation files (bless waves; identify via dispatch
    against the in-tree bless test pattern — FACT first)
- Blast-radius discipline:
  - **`docs/07_implementation_status.md`** is edited only via worker dispatch
    (per CLAUDE.md Test Discipline; the file is large and never loaded).
  - **The bless-wave files** are identified by the in-tree bless test pattern
    (dispatch a `LOCATIONS` FACT for "lightning" or "lightning-infill" mentions in
    the expectations directory before editing any file).
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "FACT: what is the current status column of the DEV-081 row in
    `docs/DEVIATION_LOG.md` (line 32 region)?" — `[FWD]` resolution.
  - "FACT: what is the current status column of the
    `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` row in
    `docs/DEVIATION_LOG.md`?" — `[FWD]` resolution.
  - "LOCATIONS ≤ 10: which expectation files mention lightning-infill or
    `sparse_fill_holder == 'lightning-infill'` (the bless-wave targets)."
  - "Bless sweep: per expectation, FACT old→new + justification".
  - "Run `cargo xtask build-guests --check` then `cargo xtask test --workspace
    --summary`; verdict block ONLY".
  - "Doc edits + the four Doc Impact greps; FACT each".
- Context cost: `S` (all delegated)
- Authoritative docs: `CLAUDE.md` §Test Discipline.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 + AC-5 pipe commands + the ceremony verdict — FACT each
- Exit condition: DEV-081 Closed; `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW`
  Closed; ceremony PASS; TASK-262…265 closed; packet + roadmap closed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | L (justified) | WIT + trait + macro + dispatch + 4-module + test-guest + drift-re-baseline + 33-guest rebuild — atomic coupled bundle that closes D-137-WIT-RUN-INFILL-NO-PAINT-VIEW |
| Step 1 | M | Orca FACT + classification + RED |
| Step 2 | M | the rewrite |
| Step 3 | S | pipeline + guard |
| Step 4 | S | closure (delegated) |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (TASK-262…265
  closure sweep), never a full backlog read.
- Reconcile reopened/superseded status transitions (DEV-081 closure recorded).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a
  logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
