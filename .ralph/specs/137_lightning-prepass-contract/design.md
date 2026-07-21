# Design: 137_lightning-prepass-contract

## Controlling Code Paths

- Primary code path: `crates/slicer-scheduler/src/execution_plan.rs` (`STAGE_ORDER` string
  slice at line 19) → new `crates/slicer-core/src/algos/lightning/mod.rs` (producer
  skeleton) → new runtime builtin wrapper at
  `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` →
  `crates/slicer-ir/src/slice_ir.rs` (`LightningTreeIR` + entry + version constant) →
  `crates/slicer-runtime/src/blackboard.rs` (commit slot + accessor, pattern:
  `commit_support_plan` / `support_plan` at lines 190, 200) →
  `crates/slicer-schema/wit/deps/ir-types.wit` (add `lightning-tree-segments` method to
  `paint-region-layer-view` resource at line 206; precedent: `support-plan-segments` at
  line 210) + `world-layer.wit` (bump package version) →
  `crates/slicer-sdk/src/traits.rs` (`PaintRegionLayerView` at line 58 — new
  `with_lightning_tree_ir` builder, `lightning_tree_ir()` getter, and
  `lightning_tree_segments_for(object_id, region_id)` method, mirroring
  `with_support_plan` / `support_plan_segments_for` at lines 88, 144).
- Neighboring tests or fixtures: scheduler stage-order tests (likely
  `crates/slicer-scheduler/tests/` — author if absent; the
  `layer_stage_commit_stages_tdd.rs` is at `crates/slicer-runtime/tests/contract/`); new
  `crates/slicer-ir/tests/lightning_tree_ir_tdd.rs` (shape test, pattern the IR-precedent
  tests); new `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (skip/commit)
  + `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs` (host
  commit + SDK accessor + a small WASM test guest) + `wit_drift_detection_tdd.rs` (existing
  drift suite, plus new assertion for the lightning method).
- OrcaSlicer comparison surface: none in this packet (contract only; ADR-0029 records the
  cross-layer facts).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- ADR-0029 is binding: host-side producer (not a WASM prepass module); skip-when-unused;
  compact per-layer 2-point segment storage.
- CLAUDE.md §WIT/Type Changes Checklist governs the read-view addition (search
  `wit_host.rs`, `dispatch.rs`, `wit_guest` modules; verify type identity at the
  component boundary; run `cargo build --tests` after the WIT edit; edit the canonical
  source at `crates/slicer-schema/wit/deps/ir-types.wit`).
- The 136-blessed golden baseline must survive untouched (AC-N1 wedge byte-identity).

## Code Change Surface

- Selected approach: clone the `SupportPlanIR` seam shape — IR struct + version constant +
  blackboard slot/commit + host producer registered at the new stage + a read-view method
  guests reach from `Layer::Infill` dispatch context. The skip predicate reuses the
  fill-holder resolution (`sparse_fill_holder == lightning-infill` for ANY region of the
  print) computed once at producer entry; the wrapper is a no-op when the predicate fails.
- Exact changes:
  1. `execution_plan.rs:19` — one string appended to `STAGE_ORDER`; corresponding
     stage-order test update.
  2. `slice_ir.rs` — `LightningTreeIR` struct + `LightningTreeEntry` struct +
     `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION` constant; re-export from `lib.rs`.
  3. New `crates/slicer-core/src/algos/lightning/mod.rs` — declares the module +
     `generate_lightning_trees(...) -> LightningTreeIR` returning empty trees (the 139
     wiring point is marked with a `// 139 wiring point` comment).
  4. New runtime builtin wrapper
     `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` — skip predicate +
     commit; registers in `crates/slicer-runtime/src/prepass.rs` (pattern: the
     support-geometry builtin invocation at line 654 and dispatch at line 798); re-export
     from `crates/slicer-runtime/src/lib.rs` (line 112 re-exports the support wrapper —
     mirror that).
  5. `ir-types.wit:206` — add `lightning-tree-segments: func(object-id, region-id) ->
     list<list<point3-with-width>>` to the `paint-region-layer-view` resource (exact
     signature mirrors `support-plan-segments` at `:210`); bump the `world-layer`
     package version in `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`.
  6. `crates/slicer-sdk/src/traits.rs:58` — add `lightning_tree_ir: Option<Arc<LightningTreeIR>>`
     field to `PaintRegionLayerView`; add `with_lightning_tree_ir`, `lightning_tree_ir()`,
     and `lightning_tree_segments_for(object_id, region_id)` methods; add necessary re-
     exports in `crates/slicer-sdk/src/lib.rs`.
  7. Macros glue: the macro already embeds `ir-types.wit` via `include_str!`; the
     `lightning-tree-segments` declaration is automatically visible to generated guests
     (verify in `crates/slicer-macros/src/lib.rs` per the WIT checklist).
  8. New test guest: extend `crates/slicer-wasm-host/test-guests/layer-infill-guest/` to
     call `lightning-tree-segments` and echo the count; or add a small
     `lightning-tree-view-guest/` if the layer-infill guest doesn't fit. Drift-test rows
     added to `wit_drift_detection_tdd.rs`.
  9. Docs: `docs/02_ir_schemas.md` new `## IR 9c — LightningTreeIR` section (pattern the
     `SupportPlanIR` section at line 1316); `docs/03_wit_and_manifest.md` new
     `### Lightning tree read-view` subsection.
- Rejected alternatives: (a) WASM prepass module for generation — rejected by ADR-0029
  (recorded trade-off); (b) lazily generating trees at first `Layer::Infill` touch —
  rejected: hides a whole-print computation inside a per-layer dispatch, breaking
  layer-parallel-safety expectations; (c) storing full tree topology in the IR — rejected:
  per-layer 2-point segments are what the module needs (ADR-0029 memory note); (d) a new
  WIT resource for lightning-tree views — rejected: piggybacking on the existing
  `paint-region-layer-view` resource matches the `support-plan-segments` precedent and
  keeps the host view struct (`PaintRegionLayerView`) a single per-layer bag of plan IRs.

## Files in Scope (read + edit)

- `crates/slicer-scheduler/src/execution_plan.rs` — stage entry; one string appended.
- `crates/slicer-ir/src/slice_ir.rs` (+ `crates/slicer-ir/src/lib.rs` re-export) —
  `LightningTreeIR` + entry + version constant.
- `crates/slicer-core/src/algos/lightning/mod.rs` (new) +
  `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` (new) +
  `crates/slicer-runtime/src/prepass.rs` (builtin invocation registration) +
  `crates/slicer-runtime/src/blackboard.rs` (commit slot + accessor) +
  `crates/slicer-runtime/src/lib.rs` (re-export) + `crates/slicer-core/src/algos/mod.rs`
  (`pub mod lightning;`).
- `crates/slicer-schema/wit/deps/ir-types.wit` (read-view method) +
  `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (package version bump).
- `crates/slicer-sdk/src/traits.rs` (+ `crates/slicer-sdk/src/lib.rs` re-exports) +
  `crates/slicer-macros/src/lib.rs` (verify include_str coverage).
- Tests: new `crates/slicer-scheduler/tests/stage_order_tdd.rs` (or extend an existing
  scheduler test); new `crates/slicer-ir/tests/lightning_tree_ir_tdd.rs`; new
  `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`; new
  `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs` + register
  in `crates/slicer-runtime/tests/contract/main.rs` (mod line); new test guest
  `crates/slicer-wasm-host/test-guests/lightning-tree-view-guest/` (or extension of an
  existing guest); extend `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
  with the lightning method assertion.
- Docs: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`.

## Read-Only Context

- `crates/slicer-core/src/algos/support_geometry.rs` — lines 80-140 (producer pattern) +
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs:37-60` (wrapper pattern).
- `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR` region (~1100-1150) + version
  constants block (~250-260) + the `SupportPlanEntry` struct (~1110-1130) for field shape.
- `crates/slicer-runtime/src/blackboard.rs:62,190,200` — slot + commit + accessor for
  `SupportPlanIR`; mirror the shape.
- `crates/slicer-sdk/src/traits.rs:50-170` — `PaintRegionLayerView` + accessor methods
  (`with_support_plan` line 88, `support_plan_segments_for` line 144).
- `crates/slicer-schema/wit/deps/ir-types.wit:206-212` — `paint-region-layer-view` resource
  + `support-plan-segments` method (precedent for the lightning method).
- How a `Layer::Infill` module reaches the `PaintRegionLayerView` today — one LOCATIONS
  dispatch, then ranged reads of the named sites.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not needed; never load.
- `modules/core-modules/lightning-infill/**` — packet 140's surface.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/algos/lightning/{distance_field,tree_node,layer,generator}.rs`
  — packet 138/139 surface (the mod.rs skeleton returns empty IR until 139).

## Expected Sub-Agent Dispatches

- "FACT: which stage currently sits last in `STAGE_ORDER` (so the lightning entry's
  position is unambiguous); LOCATIONS ≤ 5 entries from `execution_plan.rs:19-45`" — Step 1
  driver.
- "FACT: does adding a NEW IR type require bumping a global `max_ir_schema` constant
  (packet 91 precedent)? ≤ 5 lines" — Step 1 IR-shape FACT.
- "FACT: which concrete symbol/method exposes `sparse_fill_holder` on the host-side
  print/region config (likely `ResolvedConfig.sparse_fill_holder: String` at
  `crates/slicer-ir/src/resolved_config.rs:691` and the dispatch consumer at
  `crates/slicer-wasm-host/src/dispatch.rs:1929`); LOCATIONS ≤ 5" — Step 2 driver for
  the skip-predicate (the producer's skip predicate iterates the regions of the print
  and reads this field; record the exact field path in the implementation).
- "Run `cargo test -p slicer-ir -- lightning_tree_ir …` +
  `cargo test -p slicer-scheduler --test stage_order_tdd …`; FACT each" — Step 1
  verification.
- "LOCATIONS ≤ 10: where the support-geometry producer is registered + committed in
  `prepass.rs`" — Step 2 driver.
  - "Run `cargo test -p slicer-runtime --test executor -- lightning_prepass …`; FACT" —
    AC-3.
- "Run `cargo build --tests 2>&1 | tail -40`; FACT or LOCATIONS ≤ 30" — after the WIT
  edit (Step 3).
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE" — Step 3 guest
  freshness.
- "Run `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip …`;
  FACT" — AC-4.
- "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N1.
- "Run `cargo test -p slicer-runtime --test contract -- wit_drift_detection …`; FACT" —
  AC-N2.

## Data and Contract Notes

- IR: new top-level IR `LightningTreeIR` + entry `LightningTreeEntry` + version constant.
  Schema-versioned per `SemVer` (matches `SupportPlanIR` precedent at `slice_ir.rs:251`).
  No global `max_ir_schema` bump is needed (packet 91 precedent is a separate question —
  resolve by FACT at Step 1).
- WIT: additive method on existing `paint-region-layer-view` resource; bump the
  `world-layer` package version (e.g. `2.1.0` → `2.2.0`); full guest rebuild via
  `cargo xtask build-guests`.
- Determinism: the IR's per-layer segment ordering is producer-defined and must be stable
  (`Vec` order, no hash containers) — 139's determinism test builds on this.
- Layer↔view wiring: `with_lightning_tree_ir` is a host-only builder (mirrors
  `with_support_plan` at `traits.rs:88`); the live dispatch path attaches the IR through
  the layer_executor pattern at `crates/slicer-runtime/src/layer_executor.rs:330,1042`
  (those attach `support_plan: blackboard.support_plan().cloned()` — mirror for lightning).

## Locked Assumptions and Invariants

- Producer skipped (no commit) when no lightning holder — the zero-cost promise (AC-3).
- `LightningTreeIR` stores per-layer 2-point integer segments, not topology (ADR-0029).
- The view method exposes exactly the dispatching (object, region, layer)'s segments — no
  whole-print guest visibility.
- Non-lightning output byte-identical (AC-N1).
- The `world-layer` package version bump is the only WIT version delta in this packet;
  `world-prepass` and other worlds do not need to be touched.

## Risks and Tradeoffs

- The "which world exposes the view" bump ripples like 130/131's — smaller surface, same
  ceremony; front-loaded knowledge from those packets applies. Mitigation: the WIT edit
  is one method addition, not a new interface, and the macro is already embedding
  `ir-types.wit` via `include_str!` (verify before editing).
- An empty-trees producer is temporarily misleading (lightning configured → no trees →
  module still uses its stub until 140): acceptable and explicit — the stub path is
  untouched until 140, so behavior is unchanged for lightning users during 137-139.
- Adding a field to `PaintRegionLayerView` touches every existing builder call
  (mostly test-only — `with_support_plan` is only used in tests, per
  `live_layer_support_tdd.rs:1027,1058,1364`); the live dispatch path attaches via
  `layer_executor.rs:330/1042`. Step 3 must add a `with_lightning_tree_ir` builder call
  to the live dispatch path alongside `support_plan` (or use a shared setter) so the
  field is populated at runtime. This is the "struct-literal blast radius" for the
  `PaintRegionLayerView` field addition.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 3 — WIT view + plumbing + struct-literal blast radius)
- Highest-risk dispatch: the view-pattern LOCATIONS — must stay ≤ 10 entries.

## Open Questions

- `[FWD]` Whether a global `max_ir_schema` constant must be bumped for a NEW IR type
  (packet 91 precedent) — resolve by FACT dispatch at the IR step; record either way.
- `[FWD]` Exact position of `"PrePass::LightningTreeGen"` in `STAGE_ORDER` (after the
  current last prepass stage and before `"Layer::Infill"`) — resolve at Step 1 by
  FACT.
- `[FWD]` Whether the existing `layer-infill-guest` can be extended to call
  `lightning-tree-segments`, or a new minimal `lightning-tree-view-guest/` is required
  — resolve at Step 3 by inspecting the existing guest source (LOCATIONS dispatch).
