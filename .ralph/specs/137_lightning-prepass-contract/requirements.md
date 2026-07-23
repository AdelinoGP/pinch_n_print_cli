# Requirements: 137_lightning-prepass-contract

## Packet Metadata

- Grouped task IDs: `TASK-262`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's lightning generator is per-object and cross-layer — `Generator::generateTrees`
makes two full top-down passes over **all** layers before any layer can be filled — while
PnP's `Layer::Infill` hook sees one layer at a time. Without a cross-layer home, the
canonical algorithm cannot be ported, which is why the current `lightning-infill` module is
a single-layer approximation that self-links its own output in violation of ADR-0025
(DEV-081). PnP's own precedent solves this: `PrePass::SupportGeometry` produces
`SupportPlanIR` host-side, and a `Layer::Support` guest reads it via a method on
`PaintRegionLayerView`. This packet builds the lightning equivalent so 138/139 can port the
algorithm into a stable seam and 140 can slim the module to a sampler.

## In Scope

- `PrePass::LightningTreeGen` in `STAGE_ORDER`
  (`crates/slicer-scheduler/src/execution_plan.rs:19`), positioned per ADR-0029; the
  scheduler stage-order test is updated; no `STAGE` enum exists today (stages are
  string literals).
- `LightningTreeIR` in `crates/slicer-ir/src/slice_ir.rs`: schema-versioned; per object, per
  layer: `Vec<[Point2; 2]>` of 2-point integer-unit tree-edge segments (compact storage per
  ADR-0029's memory note); `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION: SemVer` constant;
  blackboard commit slot + accessor (pattern `SupportPlanIR` at
  `crates/slicer-runtime/src/blackboard.rs:62,190,200`).
- Host producer skeleton in a new `crates/slicer-core/src/algos/lightning/mod.rs` +
  runtime builtin wrapper (pattern: `support_geometry_producer.rs` at
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs:37`): skip when no
  region's `sparse_fill_holder` resolves to `lightning-infill`; commit an empty-but-valid
  IR when configured (algorithm arrives in 139).
- WIT read-view: add `lightning-tree-segments: func(object-id, region-id) ->
  list<list<point3-with-width>>` to the existing `paint-region-layer-view` resource in
  `crates/slicer-schema/wit/deps/ir-types.wit:206` (precedent: `support-plan-segments` at
  `:210`); bump the `world-layer` package version (`world-layer.wit`); SDK accessor in
  `crates/slicer-sdk/src/traits.rs` `PaintRegionLayerView` (the home of
  `with_support_plan`/`support_plan_segments_for` at lines 88/144); macros glue; drift-test
  coverage; guest rebuild ceremony.
- Docs: `docs/02_ir_schemas.md` IR section; `docs/03_wit_and_manifest.md` view contract.

## Out of Scope

- The generator algorithm (`DistanceField`, `TreeNode`, `Layer`, `Generator`) — packets
  138/139.
- Any `lightning-infill` module change — packet 140 (the view exists; the module doesn't
  call it yet).
- Overhang detection changes — the producer consumes existing slice/shell outputs.

## Authoritative Docs

- `docs/adr/0029-lightning-prepass-tree-generator.md` — binding; full read (short).
- `docs/specs/lightning-infill-parity.md` §L1 — full read (short).
- `docs/02_ir_schemas.md` — `SupportPlanIR` section (shape precedent) only; delegate.
- `docs/03_wit_and_manifest.md` — read-view pattern; delegate.
- `CLAUDE.md` §WIT/Type Changes Checklist + §Guest WASM Staleness.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-4` in `packet.spec.md`. Refinements: AC-3's skip case asserts
  NO blackboard commit (`blackboard.lightning_tree_ir().is_none()` — the zero-cost
  promise); AC-4's roundtrip is count + endpoint equality against host-committed fixture
  trees through the WIT method `lightning-tree-segments`.
- Negative cases: `AC-N1` (wedge byte-identity — stage present, producer skipped),
  `AC-N2` (WIT drift assertion, including the world-layer package version bump).
- Cross-packet impact: 138/139 fill the producer; 140 consumes the view. Field names
  locked once 138 activates.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -n 'LightningTreeGen' crates/slicer-scheduler/src/execution_plan.rs` | AC-1 stage string present | FACT |
| `cargo test -p slicer-scheduler --test stage_order_tdd 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1 stage order green | FACT |
| `cargo test -p slicer-ir -- lightning_tree_ir 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-2 IR shape | FACT |
| `cargo test -p slicer-runtime --test executor -- lightning_prepass 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 skip/commit | FACT |
| `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-4 roundtrip | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 byte-identity | FACT |
| `cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N2 WIT drift | FACT |
| `cargo check --workspace --all-targets` + `cargo clippy --workspace --all-targets -- -D warnings` | gates | FACT each |
| `cargo xtask build-guests --check` (rebuild if STALE) | guest freshness after WIT | FACT |

## Step Completion Expectations

- Cross-step invariant: the read-view method's WIT signature is the contract frozen at this
  packet's close. If 138/139 require a different shape (e.g. a missing per-tree grouping
  field), the contract deviation is recorded here as a WIT bump, not a silent change.
- The 137 producer skeleton returns empty trees — the 139 wiring point must be marked with a
  `// 139 wiring point` comment so the seam is obvious.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-ir/src/slice_ir.rs` (the `SupportPlanIR` region ~1100-1150 and the
  IR-version constants region ~250-260 only), `crates/slicer-core/src/algos/support_geometry.rs`
  (producer skeleton pattern — entry fn + commit only, lines 80-140),
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs:37-60` (wrapper pattern),
  `crates/slicer-sdk/src/traits.rs:50-170` (view accessor pattern).
- Likely temptation reads: `OrcaSlicerDocumented/Fill/Lightning/**` — NOT needed in this
  packet (no algorithm here); any "what will the IR need" question is answered by ADR-0029
  and the L1 spec section.
- Sub-agent return-format hints: cargo gates FACT; the one structural question ("how does a
  Layer::Infill guest read SupportPlanIR-style views today?") returns LOCATIONS ≤10.
