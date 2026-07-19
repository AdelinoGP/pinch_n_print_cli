# Implementation Plan: 140_lightning-module-rewrite

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

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
- Precondition: packets 137–139 closed.
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
    `lightning_tree_segments_for(object_id, region_id) -> Vec<…>` (the 137 view
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
  green.
- Precondition: Step 1 exit condition.
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
  - AC-1, AC-2, AC-N2 pipe commands — FACT each
- Exit condition: module green; stub grep-gone.

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

### Step 4: Closure — DEV-081, contained bless, roadmap ceremony

- Task IDs: `TASK-265`
- Objective: resolve the `[FWD]` (FACT on the live DEV-081 row's status field —
  if `Closed` already, leave it; otherwise flip to `Closed` and add a reference
  note per the log's convention); re-bless lightning-affected expectations (two
  consecutive identical runs; per-expectation justification); docs/07 closure
  sweep for TASK-262…265 (delegated — never load the full file); run the
  roadmap-close `cargo xtask test --workspace --summary` ceremony.
- Precondition: Step 3 exit condition (bless only after geometry/pipeline green).
- Postcondition: AC-4, AC-5 green; ceremony PASS recorded; packet + roadmap closed.
- Files allowed to read: none directly (all delegated).
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (DEV-081 row, if not already Closed)
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
  - "LOCATIONS ≤ 10: which expectation files mention lightning-infill or
    `sparse_fill_holder == 'lightning-infill'` (the bless-wave targets)."
  - "Bless sweep: per expectation, FACT old→new + justification".
  - "Run `cargo xtask build-guests --check` then `cargo xtask test --workspace
    --summary`; verdict block ONLY".
  - "Doc edits + the two Doc Impact greps; FACT each".
- Context cost: `S` (all delegated)
- Authoritative docs: `CLAUDE.md` §Test Discipline.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 + AC-5 pipe commands + the ceremony verdict — FACT each
- Exit condition: DEV-081 Closed; ceremony PASS; TASK-262…265 closed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
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
