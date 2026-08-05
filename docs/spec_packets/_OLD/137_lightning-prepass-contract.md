---
status: implemented
packet: 137_lightning-prepass-contract
task_ids:
  - TASK-262
---

# 137_lightning-prepass-contract

## Goal

Land the lightning-parity architecture seam (ADR-0029): a `PrePass::LightningTreeGen` stage
appended to `STAGE_ORDER`, a schema-versioned `LightningTreeIR` (per object, per layer
tree-edge segments), a host producer skeleton that is **skipped (no commit)** when no region's
sparse holder is `lightning-infill`, and a WIT read-view method (`lightning-tree-segments`)
added to the existing `paint-region-layer-view` resource so a `Layer::Infill` module can read
its layer's committed trees.

## Problem Statement

OrcaSlicer's lightning generator is per-object and cross-layer — `Generator::generateTrees`
makes two full top-down passes over **all** layers before any layer can be filled — while
PnP's `Layer::Infill` hook sees one layer at a time. Without a cross-layer home, the
canonical algorithm cannot be ported, which is why the current `lightning-infill` module is
a single-layer approximation that self-links its own output in violation of ADR-0025
(lightning raw-emit deviation). PnP's own precedent solves this: `PrePass::SupportGeometry` produces
`SupportPlanIR` host-side, and a `Layer::Support` guest reads it via a method on
`PaintRegionLayerView`. This packet builds the lightning equivalent so 138/139 can port the
algorithm into a stable seam and 140 can slim the module to a sampler.

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
