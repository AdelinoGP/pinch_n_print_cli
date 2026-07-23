# Plan: Close Packet-137 Deviations

## Source

The packet 137 cold review (post-implementation) returned CHANGES REQUESTED with
6 defects; 5 were closed in the same session, 1 was deferred as a packet-scope
gap. The two open DEVIATION entries (`docs/DEVIATION_LOG.md`) bind to the
existing `draft` packets:

- `D-137-LIGHTNING-PER-OBJECT-COLLAPSE` → packet 139 (`lightning-layer-generator`)
- `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` → packet 140 (`lightning-module-rewrite`)

Both packets exist as `draft` and were authored before 137's deviations were
known. They need to be **refined in place** (not replaced) to absorb the new
scope and pin the deviation closures in their acceptance criteria.

## User-Approved Decisions

- **139:** add `region_id: u64` to `LightningTreeEntry`; update the dispatch
  HashMap keying in `crates/slicer-wasm-host/src/dispatch.rs` from
  `wildcard_region = "*"` to the actual `region_id`; update the SDK accessor
  `lightning_tree_segments_for` to honor `region_id` (no longer `_region_id`).
  No WIT signature change (the method already accepts `region-id`; the IR
  carrying it is host-side).
- **140:** extend the WIT `run-infill` signature with a
  `paint: paint-region-layer-view` argument; bump
  `slicer:world-layer@2.2.0` → `@2.3.0`; thread the paint view through the SDK
  trait, the macro glue, the host dispatch, and the four `run_infill`-
  implementing core modules (rectilinear/gyroid/lightning/top-surface-ironing);
  add a real `Layer::Infill` test-guest calling `lightning-tree-segments`
  through the WIT boundary (reuse `layer-infill-guest`).
- **Status:** both refined packets stay `status: draft` (preflight gate is the
  activation check; we do not activate in this refinement).
- **Batch protocol:** single combined author subagent for both packets (user
  chose this; not the parallel-double-dispatch option). I (the planner) own
  the batch plan, the preflight dispatch, and the final report.

## Investigation Summary (140's Run-Infill Paint-View Blast Radius)

The canonical D-137 fix (extend `run-infill` + bump package) was investigated
against four alternatives. The blast radius is concrete and bounded:

- **13 core-module `LayerModule` impls re-bind on `world-layer.wit`** (3
  actually implement `run_infill`: rectilinear, gyroid, lightning, top-surface-
  ironing — 4 of them, not 3 as the planner initially wrote; the other 9 are
  perimeters/postprocess/pathopt/support and are mechanical rebinds).
  - `rectilinear-infill/src/lib.rs:94` (527 LOC)
  - `gyroid-infill/src/lib.rs:138` (716 LOC)
  - `lightning-infill/src/lib.rs:97` (512 LOC; the D-137 fixee)
  - `top-surface-ironing/src/lib.rs:305` (355 LOC)
- **1 module implements only `run_infill_postprocess`**: support-surface-ironing
  (line 193) — NOT affected by `run-infill` signature change.
- **3 test-guests** with `run_infill`:
  - `layer-infill-guest/src/lib.rs:113` (raw WIT-bindgen) — reuse, one-line add.
  - `sdk-layer-infill-guest/src/lib.rs:28` (SDK-wrapped) — reuse, parallel
    trait + impl change.
  - `infill-postprocess-echo-guest` — implements `run_infill_postprocess` only;
    NOT affected.
- **WIT version bump 2.2.0 → 2.3.0** (consistent with the 2.0.0→2.1.0→2.2.0
  chain). One test pins the version string
  (`wit_drift_detection_tdd.rs:592-616`); must re-baseline.
- **Host dispatch arm** at `dispatch.rs:442-465` mirrors the existing
  `Layer::Support` arm at `dispatch.rs:584-619` (which already wires the
  paint view through `build_paint_layer_data_with_plan(...)` +
  `push_paint_region_layer_view(...)`). The `lightning_tree_ir` field is
  already plumbed in `LayerStageInput` (137).
- **SDK trait** at `traits.rs:369-377` must add the paint-view arg to
  `LayerModule::run_infill`.
- **slicer-macros** `infill_arm` at `crates/slicer-macros/src/lib.rs:1779-1794`
  and the macro-emitted `fn run_infill` glue at `:2804-2809` must be updated.
- **33 guest artifacts** re-stale (21 core-module + 12 test-guests).
- **Build cost:** one `cargo xtask build-guests` rebuild after the WIT + macro
  changes; AC-1 verification runs on the rebuilt guests.

### Alternatives Evaluated

- (a) Second export `run-infill-with-paint`: viable but adds permanent desync
  from the perimeters/support pattern D-137 explicitly invoked; rejected.
- (b) `option<paint-region-layer-view>` parameter: not viable in this WIT
  tree (zero `option<>`-as-top-level-arg precedents; only as record field or
  return type); rejected.
- (c) Import function `get-paint-region-layer-view(layer-idx)`: risky; the
  host call is per-call, not per-stage, so the paint-build cost is the same;
  zero surface win; rejected.
- (d) Promote `lightning-tree-segments` to its own resource: risky; breaks
  the perimeters/support symmetry; strictly larger surface than (a); rejected.

**Canonical D-137 fix selected** (extend `run-infill` + bump to 2.3.0).

## Packet Queue

| # | Slug | Status (start) | Status (end) | Closes | Closes-priority | Cost |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 139_lightning-layer-generator | draft | draft | D-137-LIGHTNING-PER-OBJECT-COLLAPSE | high (1) | M (1 M step bumped to L by IR-field addition) |
| 2 | 140_lightning-module-rewrite | draft | draft | D-137-WIT-RUN-INFILL-NO-PAINT-VIEW | high (1) | M (1 M step bumped to L by WIT + trait + 4-module surface) |

### Per-Packet Refinement Targets

#### 139 (refine in place)

- `packet.spec.md`:
  - Goal: amend to call out the per-region commitment explicitly.
  - AC-3 (currently "trees_for_layer output == producer-committed
    LightningTreeIR"): add an explicit per-region assertion
    ("regions on the same `(object, layer)` get distinct segment buckets keyed
    by `region_id`; the wildcard `*` keying is removed").
  - Add `AC-5`: the dispatch HashMap in
    `crates/slicer-wasm-host/src/dispatch.rs:1383` keys on the actual
    `region_id` (mirroring `support-plan-segments` at `dispatch.rs:1353`).
  - Add `AC-N3`: the SDK accessor `lightning_tree_segments_for(object_id,
    region_id)` honors its `region_id` argument (no longer `_region_id`).
  - Add `AC-4-bis` (or new AC): the per-layer segment ordering frozen at 139
    is stable across re-runs (determinism extended to the new
    `region_id` dimension).
- `requirements.md`:
  - In Scope: add `LightningTreeEntry.region_id: u64` field to
    `slicer-ir::slice_ir` (mirrors `SupportPlanEntry.region_id` precedent at
    `slice_ir.rs:1394`); update the WIT host dispatch to thread the
    per-region key; update the SDK accessor to honor `region_id`; update the
    `lightning_tree_ir` field on `PaintRegionLayerView` to keep the per-region
    projection.
  - Out of Scope: keep current "no WIT signature change" — the
    `lightning-tree-segments` method already accepts `region-id`; only the
    IR + dispatch + SDK projection change.
  - Acceptance Summary: add the deviation-closure language for
    `D-137-LIGHTNING-PER-OBJECT-COLLAPSE`.
- `design.md`:
  - Code change surface: add `slice_ir.rs` `region_id` field; add
    `dispatch.rs` per-region HashMap keying; update `traits.rs` accessor to
    take and use `region_id`.
  - Architecture Constraints: add a `[FWD]` → `[FWD-resolved]` line: "the
    per-region skip predicate is print-wide at the host boundary (no per-
    region `ResolvedConfig.sparse_fill_holder` iteration); the per-region
    IR + dispatch + SDK projection are the granularity for the read-view".
    This is the deviation's resolution: predicate stays print-wide
    (intentional), view becomes per-region (refinement).
  - Read-only context: add `slicer-ir::slice_ir::SupportPlanEntry` region_id
    field (the precedent; line 1394 in the current tree) as the IR-field
    shape reference.
- `implementation-plan.md`:
  - Add a Step 0 (or split Step 4): per-region IR + dispatch + SDK projection.
    Owner of `LightningTreeEntry.region_id` addition, the
    `dispatch.rs:1383` per-region keying fix, the `traits.rs:198` signature
    change, and the 137 test that was pinned on wildcard keying must be
    updated to a per-region test. The new test name: a per-region roundtrip
    (two regions on the same `(object, layer)`, each with distinct
    segments; the accessor returns only the queried region's segments).
  - Update existing steps to mark the per-region commit as a step-1 (or
    step-0) precondition, not a step-4 afterthought.
- New ACs land a `region_id` dimension on every existing assertion.
- `task-map.md`: not needed (single task ID TASK-264).

#### 140 (refine in place)

- `packet.spec.md`:
  - Goal: amend to call out the WIT extension + per-region view reachability.
  - AC-1 (currently "module emits exactly the layer's tree segments"): no
    change in shape; the per-region keying lands via 139, and 140's module
    reads via `lightning_tree_segments_for(object_id, region_id)`.
  - Add `AC-3a` (new): the WIT `run-infill` signature takes
    `paint: paint-region-layer-view`; world-layer package is `@2.3.0`; the
    `wit_drift_detection_tdd` `paint_region_layer_view_has_lightning_tree_segments_method`
    test passes.
  - Add `AC-3b` (new): `Layer::Infill` test-guest (the existing
    `layer-infill-guest` extended at `src/lib.rs:113` to add
    `_paint: PaintRegionLayerView` and emit a witness path encoding the
    `lightning-tree-segments` count for the dispatched region) satisfies
    D-137's original AC-4 wording — a real `Layer::Infill` guest traversing
    the host↔guest component seam.
  - Add `AC-3c` (new): all four `run_infill`-implementing core modules
    (rectilinear/gyroid/lightning/top-surface-ironing) compile with the
    new `paint: &PaintRegionLayerView` argument (lightning uses it; the
    other three take a `_paint` and ignore it).
- `requirements.md`:
  - In Scope: add the WIT signature change at
    `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25`; add the
    package version bump 2.2.0→2.3.0; add the SDK trait extension at
    `crates/slicer-sdk/src/traits.rs:369`; add the slicer-macros
    `infill_arm` update at `crates/slicer-macros/src/lib.rs:1779-1794` and
    the macro-emitted glue at `:2804-2809`; add the host dispatch update
    at `crates/slicer-wasm-host/src/dispatch.rs:442-465` (mirror the
    perimeters/support arm at `:529-559`/`:584-619`); add the four
    `run_infill`-implementing core modules' arg-list updates; add the
    `layer-infill-guest` extension at `src/lib.rs:113`; add the test
    re-baselining for `wit_drift_detection_tdd.rs:592-616` and
    `wit_boundary_tdd.rs` consumers.
  - Out of Scope: keep current exclusions (no generator/primitive changes;
    no claims/manifest changes; no linker changes). Also exclude: the
    support-surface-ironing module (it implements `run_infill_postprocess`
    only, not `run_infill`).
  - Acceptance Summary: add the deviation-closure language for
    `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW`. The canonical-fix chosen
    (extend `run-infill` + bump 2.3.0) is recorded with the rejected
    alternatives summary.
- `design.md`:
  - Code change surface: add the WIT edit, the version bump, the trait
    edit, the macro edit, the dispatch edit, the four-module edit, the
    test-guest extension, the test re-baseline.
  - Architecture Constraints: add a bullet on the version-bump semantics
    (2.2.0→2.3.0 follows the 2.0.0→2.1.0→2.2.0 chain; consistent with
    packet 130's DEV-084 precedent on a minor→major correction for an
    additive export's argument change).
  - Read-only context: add the perimeters/support dispatch arms
    (`dispatch.rs:529-559` and `dispatch.rs:584-619`) as the
    right-mirror-shape; add the SDK trait
    (`traits.rs:369-377`) and macro glue (`slicer-macros:1779-1794,
    2804-2809`) as the change surface.
  - Code-Change-Surface must include the test-guest extension; the
    Out-of-Bounds list adds the support-surface-ironing module
    (NOT in scope).
- `implementation-plan.md`:
  - Add a Step 1a (or split existing Step 1) that performs the WIT +
    trait + macro + dispatch + four-module + test-guest + test-re-baseline
    bundle as one atomic step (these are all coupled — a partial application
    leaves the workspace un-compiling). Cost: L (this is the step that
    serializes everything else). The packet 137 review noted that the
    standard band tolerates L steps only with explicit justification; this
    packet's `design.md` must justify the L step (the four-module
    `run_infill` arg-list update + the macro glue + the WIT bump are
    inextricably coupled — a partial state would break every infill
    guest's instantiation at runtime).
  - Subsequent steps (2-N) mirror the existing 140 plan (module rewrite,
    test re-baseline, DEV-081 closure, bless, ceremony).
- `task-map.md`: not needed (single task ID TASK-265).

### Cross-Packet Sequencing

- 139 lands first; 140 reads 139's per-region keying.
- 140 is structurally independent of 139's algorithm body — the per-region
  IR field is sufficient input; the algorithm details do not affect 140's
  module rewrite.
- 138's primitive API freeze is unaffected by both refinements (the IR
  field add is at the entry level, not the tree-node or distance-field
  level).

### Self-Review Triggers

- `implementation-plan.md` L step requires explicit `design.md` justification
  (per the swarm skill: "extended band tolerates a single L step when
  design.md justifies why it cannot be split"). Both packets' L steps
  serialize on the WIT+IR+macro bundle — splitting them is not safe.
- The refinement must NOT silently change 139's algorithm scope or 140's
  module rewrite; the deviation closures are isolated additions.
- The refinement must NOT auto-flip status to `active`; both stay `draft`
  until preflight passes.

## Expected Sub-Agent Dispatch

One **author** subagent. The subagent:

1. Refines 139 in place: 4 files (packet.spec.md, requirements.md,
   design.md, implementation-plan.md).
2. Refines 140 in place: 4 files (same names).
3. Self-reviews against the swarm skill's preflight gates:
   - every AC has a pipe-suffixed command
   - validation/enforcement packets have a negative case (140 has AC-N1,
     AC-N2; 139 has AC-N1, AC-N2; both must remain)
   - `design.md` resolves `[BLOCK]`s (or keeps them as `[FWD]` if a known
     forward-dep is acknowledged)
   - `implementation-plan.md` steps have explicit pre/postconditions,
     verification, exit conditions, files-to-read, files-to-edit, context
     cost (S/M/L); L steps justified
   - no L step in standard band without justification
4. Returns a per-packet structured summary: changed files, added ACs,
   per-step costs, L-step justifications, preflight self-check, any
   remaining `[BLOCK]`s.

After the subagent returns, I (the planner) dispatch the
`spec-review --preflight` skill on each packet. If preflight PASSes, the
packets stay at `status: draft` (per user decision). If preflight
BLOCKS, the planner fixes in place and reruns.

## Resume Instruction

If interrupted: reopen this plan, find the last completed row in the
queue, dispatch the author subagent for the next packet, then preflight.
Commit together at the end of the refinement (one commit for both packet
directories + this plan).
