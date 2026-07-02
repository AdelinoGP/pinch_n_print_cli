# Requirements: 137_lightning-prepass-contract

## Packet Metadata

- Grouped task IDs:
  - `TASK-262`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's lightning generator is per-object and cross-layer — `generateTrees` makes two
full top-down passes over ALL layers before any layer can be filled
(`Generator.cpp:189-190,342`) — while PnP's `Layer::Infill` hook sees one layer at a time.
Without a cross-layer home, the canonical algorithm cannot be ported, which is why the current
module is a single-layer approximation (DEV-081). PnP's own precedent solves this
(`PrePass::SupportGeometry` → `SupportPlanIR`); this packet builds the lightning equivalent so
138/139 can port the algorithm into a stable seam and 140 can slim the module to a sampler.

## In Scope

- `PrePass::LightningTreeGen` in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`),
  positioned per ADR-0029; scheduler stage-order test update.
- `LightningTreeIR` in `crates/slicer-ir/src/slice_ir.rs`: schema-versioned; per object, per
  layer: `Vec` of 2-point integer-unit tree-edge segments (compact storage per ADR-0029's
  memory note); `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`; blackboard commit slot +
  accessor (pattern `SupportPlanIR`).
- Host producer skeleton in `crates/slicer-core/src/algos/lightning/mod.rs` +
  runtime builtin wrapper (pattern: the support-geometry producer): skip when no region's
  `sparse_fill_holder` resolves to `lightning-infill`; commit an empty-but-valid IR when
  configured (algorithm arrives in 139).
- WIT read-view for `Layer::Infill` guests (tree segments for the dispatching
  (object, layer)); SDK accessor; macros glue; drift-test coverage; guest rebuild ceremony.
- Docs: `docs/02` IR section; `docs/03` view contract.

## Out of Scope

- The generator algorithm (`DistanceField`, `TreeNode`, `Layer`, `Generator`) — packets
  138/139.
- Any `lightning-infill` module change — packet 140 (the view exists; the module doesn't call
  it yet).
- Overhang detection changes — the producer consumes existing slice/shell outputs.

## Authoritative Docs

- `docs/adr/0029-…` — binding; full read (short).
- `docs/specs/lightning-infill-parity.md` §L1 — full read (short).
- `docs/02_ir_schemas.md` — delegate; `SupportPlanIR` section (shape precedent) only.
- `CLAUDE.md` §WIT/Type Changes Checklist + §Guest WASM Staleness.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-4` in `packet.spec.md`. Refinements: AC-3's skip case asserts NO
  blackboard commit (not an empty commit) — the zero-cost promise; AC-4's roundtrip is
  count + endpoint equality against host-committed fixture trees.
- Negative cases: `AC-N1` (wedge byte-identity — stage present, producer skipped), `AC-N2`
  (WIT drift assertion).
- Cross-packet impact: 138/139 fill the producer; 140 consumes the view. Field names locked
  once 138 activates.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-ir -- lightning_tree_ir 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-2 IR shape | FACT |
| `cargo test -p slicer-runtime --test executor -- lightning_prepass 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 skip/commit | FACT |
| `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-4 roundtrip | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 byte-identity | FACT |
| `cargo check --workspace --all-targets` + `cargo clippy --workspace --all-targets -- -D warnings` | gates | FACT each |
| `cargo xtask build-guests --check` (rebuild if STALE) | guest freshness after WIT | FACT |

## Step Completion Expectations

None. (Step order follows the 130 pattern: IR/stage → producer → WIT/SDK → tests → docs; each
step's contract is self-contained.)

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-ir/src/slice_ir.rs` (the `SupportPlanIR` region ~1046 and the IR-version
  constants region only), `crates/slicer-core/src/algos/support_geometry.rs` (producer
  skeleton pattern — entry fn + commit only, ~lines 80-140).
- Likely temptation reads: `OrcaSlicerDocumented/Fill/Lightning/**` — NOT needed in this
  packet (no algorithm here); any "what will the IR need" question is answered by ADR-0029
  and the L1 spec section.
- Sub-agent return-format hints: cargo gates FACT; the one structural question ("how does a
  Layer::Infill guest read SupportPlanIR-style views today?") returns LOCATIONS ≤10.
