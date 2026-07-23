# Implementation Plan: 137_lightning-prepass-contract

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

### Step 1: `LightningTreeIR` + stage registration

- Task IDs: `TASK-262`
- Objective: add `LightningTreeIR` + `LightningTreeEntry` +
  `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION` + `Default` impl in
  `crates/slicer-ir/src/slice_ir.rs`; re-export from `crates/slicer-ir/src/lib.rs`;
  append `"PrePass::LightningTreeGen"` to `STAGE_ORDER` in
  `crates/slicer-scheduler/src/execution_plan.rs:19` (position: after the current last
  prepass entry, before `"Layer::Infill"` — both positions resolved by FACT at step
  start); add a scheduler stage-order test (or extend an existing one) asserting
  presence + position. Resolve the `[FWD]` `max_ir_schema` question by FACT.
- Precondition: packet 136 closed; clean tree.
- Postcondition: AC-1 and AC-2 verification commands green; workspace compiles.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — lines 245-260 (version-constants block) +
    lines 1100-1150 (`SupportPlanIR` shape).
  - `crates/slicer-scheduler/src/execution_plan.rs` — lines 19-45 (`STAGE_ORDER`).
  - `crates/slicer-scheduler/src/stage_order.rs` — full (small file, forwarding helpers
    live here).
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs` (+ `lib.rs` re-export — counts as 1 file)
  - `crates/slicer-scheduler/src/execution_plan.rs` (one string appended)
  - `crates/slicer-scheduler/tests/stage_order_tdd.rs` (new; counts as 1 file) — or
    extend the existing scheduler test if one already exists (FACT first)
- Blast-radius discipline (mandatory for the new IR struct + new `STAGE_ORDER` entry):
  - **Struct-literal blast radius for `LightningTreeIR`:** none today (the struct is
    net-new). No existing code constructs a `LightningTreeIR`.
  - **Struct-literal blast radius for the new `STAGE_ORDER` entry:** the `prepass.rs`
    builtin dispatch at `crates/slicer-runtime/src/prepass.rs:654,798` switches on
    `STAGE_ORDER` strings today; the new entry is added in Step 2's wrapper, not
    here. Confirmed: no Step-1 code site needs to construct a `LightningTreeIR`.
  - Dispatch a `LOCATIONS` FACT for the string `"PrePass::SupportGeometry"` callers
    (lines 654, 798) before Step 1 closes — the new entry must slot in the same
    pattern.
- Files explicitly out-of-bounds for this step: WIT files (Step 3), producer (Step 2).
- Expected sub-agent dispatches:
  - "FACT: which stage currently sits last in `STAGE_ORDER` (so the lightning entry's
    position is unambiguous); LOCATIONS ≤ 5 entries from `execution_plan.rs:19-45`" —
    Step 1 driver.
  - "FACT: does adding a NEW IR type require bumping a global `max_ir_schema` constant
    (packet 91 precedent)? ≤ 5 lines" — `[FWD]` resolution.
  - "Run `cargo test -p slicer-ir -- lightning_tree_ir …` + `cargo test -p
    slicer-scheduler --test stage_order_tdd …`; FACT each" — Step 1 verification.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0029-lightning-prepass-tree-generator.md` — full (short).
  - `docs/02_ir_schemas.md` — `SupportPlanIR` section (delegate).
- OrcaSlicer refs: none.
- Verification:
  - AC-1 pipe command — FACT
  - AC-2 pipe command — FACT
- Exit condition: IR + stage in; tests green; `[FWD]`s recorded resolved.

### Step 2: Producer skeleton + skip predicate

- Task IDs: `TASK-262`
- Objective:
  - new `crates/slicer-core/src/algos/lightning/mod.rs` —
    `generate_lightning_trees(...) -> LightningTreeIR` returning empty trees (the 139
    wiring point is marked with `// 139 wiring point`); `pub mod lightning;` added to
    `crates/slicer-core/src/algos/mod.rs`;
  - new `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` — the builtin
    wrapper (skip predicate, commit, re-export from
    `crates/slicer-runtime/src/lib.rs`);
  - register the new builtin in `crates/slicer-runtime/src/prepass.rs` (pattern: the
    support-geometry builtin at line 654 + dispatch at line 798);
  - add the blackboard commit slot + accessor in `crates/slicer-runtime/src/blackboard.rs`
    (pattern: `commit_support_plan` line 190 + `support_plan` accessor line 200 +
    slot line 62);
  - executor skip/commit test (AC-3) in new
    `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` + register in
    `crates/slicer-runtime/tests/executor/main.rs`.
- Precondition: Step 1 exit condition.
- Postcondition: AC-3 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/support_geometry.rs` — lines 80-140 (pattern).
  - `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` — full
    (small file).
  - `crates/slicer-runtime/src/prepass.rs` — lines 640-810 (builtin invocation +
    dispatch + idempotency check at 728).
  - `crates/slicer-runtime/src/blackboard.rs` — lines 60-210 (slot + commit + accessor).
- Files allowed to edit (at most 3 per wave):
  - Wave A (producer): `crates/slicer-core/src/algos/lightning/mod.rs` (new) +
    `crates/slicer-core/src/algos/mod.rs` (one `pub mod` line).
  - Wave B (wrapper + wiring): `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs`
    (new) + `crates/slicer-runtime/src/lib.rs` (re-export) + `crates/slicer-runtime/src/prepass.rs`
    (register builtin) + `crates/slicer-runtime/src/blackboard.rs` (slot + commit + accessor).
  - Wave C (test): `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (new)
    + `crates/slicer-runtime/tests/executor/main.rs` (mod registration line).
- Files explicitly out-of-bounds for this step: WIT/SDK (Step 3), module files.
- Expected sub-agent dispatches:
  - "LOCATIONS ≤ 10: where the support-geometry producer is registered + committed
    (`prepass.rs` line 654, 728, 798; `builtins/support_geometry_producer.rs` line 37;
    `lib.rs` line 112)" — Step 2 driver.
  - "FACT: how is the executor test binary aggregated
    (`tests/executor/main.rs` mod list); list current mod declarations" — Wave C driver
    (S7 wiring check).
  - "Run `cargo test -p slicer-runtime --test executor -- lightning_prepass …`; FACT" —
    AC-3.
- Context cost: `M`
- Authoritative docs: ADR-0029 (skip promise).
- OrcaSlicer refs: none.
- Verification:
  - AC-3 pipe command — FACT
- Exit condition: skip/commit behavior pinned green.

### Step 3: WIT read-view + SDK accessor + roundtrip + drift

- Task IDs: `TASK-262`
- Objective:
  - add `lightning-tree-segments: func(object-id, region-id) -> list<list<point3-with-width>>`
    to the `paint-region-layer-view` resource in
    `crates/slicer-schema/wit/deps/ir-types.wit:206` (mirror `support-plan-segments`
    at `:210`);
  - bump the `world-layer` package version in
    `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`;
  - SDK: add `lightning_tree_ir: Option<Arc<LightningTreeIR>>` field to
    `PaintRegionLayerView` in `crates/slicer-sdk/src/traits.rs:58` + `with_lightning_tree_ir`
    builder + `lightning_tree_ir()` getter + `lightning_tree_segments_for(object_id, region_id)`
    method; re-export from `crates/slicer-sdk/src/lib.rs`;
  - macros glue verification: confirm the macro at `crates/slicer-macros/src/lib.rs`
    picks up the new method via its existing `include_str!("../../slicer-schema/wit/deps/ir-types.wit")`
    (per `wit_drift_detection_tdd.rs:42-53`); if it does, no macro change; if not,
    update the include_str list and add an assertion to the drift test (S7 wiring);
  - **struct-literal blast radius for the new `PaintRegionLayerView` field:** every
    existing `PaintRegionLayerView` construction site (the
    `with_paint_regions`/`new` builders at `traits.rs:67,78` already initialize
    `lightning_tree_ir: None` once the field is added — single-file change); the live
    dispatch path at `crates/slicer-runtime/src/layer_executor.rs:330,1042` attaches
    `support_plan: blackboard.support_plan().cloned()` — mirror with `lightning_tree_ir:
    blackboard.lightning_tree_ir().cloned()`; tests construct via `with_paint_regions`
    (mostly no change). Budget: ≤ 1 file edit beyond the SDK file.
  - test guest: extend `crates/slicer-wasm-host/test-guests/layer-infill-guest/` to call
    `lightning-tree-segments` and echo the count (decision: FACT at step start — if the
    layer-infill guest's WIT bindings can't reach the new method, author a new
    `lightning-tree-view-guest/` instead);
  - new contract test
    `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`
    (registers in `crates/slicer-runtime/tests/contract/main.rs`); roundtrip =
    host commits fixture → guest dispatches → SDK accessor returns matching segments
    (count + endpoint equality);
  - extend `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` to assert
    the `lightning-tree-segments` method is present in the macro's embedded WIT blob
    (S6 / AC-N2);
  - guest rebuild: `cargo xtask build-guests` (drop `--check` if STALE).
- Precondition: Step 2 exit condition.
- Postcondition: AC-4, AC-N2 green; guests fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/ir-types.wit` — lines 195-213 (the resource
    definition).
  - `crates/slicer-sdk/src/traits.rs` — lines 50-200 (`PaintRegionLayerView` + accessors).
  - `crates/slicer-runtime/src/layer_executor.rs` — lines 320-340 + 1035-1050
    (live dispatch attach sites).
  - `crates/slicer-macros/src/lib.rs` — the `include_str!` list for WIT deps (LOCATIONS
    dispatch).
  - `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` (LOCATIONS
    dispatch — does the existing guest already use `paint-region-layer-view`?).
- Files allowed to edit (at most 3 per wave):
  - Wave A (WIT): `crates/slicer-schema/wit/deps/ir-types.wit` +
    `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`.
  - Wave B (SDK + macros + live attach):
    `crates/slicer-sdk/src/traits.rs` (+ `crates/slicer-sdk/src/lib.rs` re-exports) +
    `crates/slicer-runtime/src/layer_executor.rs` (attach the IR at the two sites) +
    `crates/slicer-macros/src/lib.rs` (only if the include_str list is incomplete;
    verify by FACT first).
  - Wave C (tests + drift + guest): new test guest file (or
    `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` extension) +
    new `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs` +
    `crates/slicer-runtime/tests/contract/main.rs` mod registration +
    `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` (drift assertion
    addition).
- Files explicitly out-of-bounds for this step: `modules/core-modules/**`,
  `crates/slicer-core/src/algos/lightning/{distance_field,tree_node,layer,generator}.rs`
  (138/139 surface).
- Expected sub-agent dispatches:
  - "Run `cargo build --tests 2>&1 | tail -40`; FACT or LOCATIONS ≤ 30" — after the WIT
    edit (Wave A).
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE" — Wave C.
  - "Run `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip …`;
    FACT" — AC-4.
  - "Run `cargo test -p slicer-runtime --test contract -- wit_drift_detection …`; FACT"
    — AC-N2.
- Context cost: `M`
- Authoritative docs: CLAUDE.md §WIT/Type Changes Checklist.
- OrcaSlicer refs: none.
- Verification:
  - AC-4, AC-N2 pipe commands — FACT each
- Exit condition: roundtrip + drift green; guests fresh.

### Step 4: Byte-identity guard + Doc Impact + gates

- Task IDs: `TASK-262`
- Objective: wedge SHA guard (AC-N1); `docs/02_ir_schemas.md` `## IR 9c — LightningTreeIR`
  section (pattern the `SupportPlanIR` section at line 1316);
  `docs/03_wit_and_manifest.md` `### Lightning tree read-view` subsection; packet gates.
- Precondition: Step 3 exit condition.
- Postcondition: all ACs green; Doc Impact greps hit.
- Files allowed to read: the two docs (rg-located sections only).
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md` (new IR section)
  - `docs/03_wit_and_manifest.md` (new read-view subsection)
- Files explicitly out-of-bounds for this step: code.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N1.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings` + the two Doc Impact
    greps; FACT each".
- Context cost: `S`
- Authoritative docs: the two being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'LightningTreeIR' docs/02_ir_schemas.md && echo HIT` — FACT
  - `rg -q 'lightning-tree-segments\|LightningTree' docs/03_wit_and_manifest.md && echo HIT` — FACT
- Exit condition: greps hit; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | IR + stage (FACT-heavy: `max_ir_schema`, last-prepass-stage) |
| Step 2 | M | producer skeleton + wrapper + blackboard wiring |
| Step 3 | M | WIT view + SDK field + struct-literal blast radius + drift |
| Step 4 | S | guard + docs |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (TASK-262 flip),
  never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
