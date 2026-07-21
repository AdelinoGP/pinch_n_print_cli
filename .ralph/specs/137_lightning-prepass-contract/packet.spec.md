---
status: implemented
packet: 137_lightning-prepass-contract
task_ids:
  - TASK-262
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 137_lightning-prepass-contract

## Goal

Land the lightning-parity architecture seam (ADR-0029): a `PrePass::LightningTreeGen` stage
appended to `STAGE_ORDER`, a schema-versioned `LightningTreeIR` (per object, per layer
tree-edge segments), a host producer skeleton that is **skipped (no commit)** when no region's
sparse holder is `lightning-infill`, and a WIT read-view method (`lightning-tree-segments`)
added to the existing `paint-region-layer-view` resource so a `Layer::Infill` module can read
its layer's committed trees.

## Scope Boundaries

Contract and plumbing only: one string entry in `STAGE_ORDER` (positioned after the stages
producing sparse-infill outlines and before `Layer::Infill` dispatch), the `LightningTreeIR`
struct + version constant + blackboard commit/accessor + docs section, the host producer
skeleton (commits an empty-but-valid IR when lightning is configured, no commit otherwise),
the WIT read-view method + SDK accessor (mirrors the `SupportPlanIR` shape on
`PaintRegionLayerView`), the contract roundtrip test, and the wedge byte-identity guard. The
generator algorithm ports land in 138/139; the module rewrite in 140. Non-lightning prints
stay byte-identical (AC-N1).

## Prerequisites and Blockers

- Depends on: `136_infill-parity-integration` (roadmap order; goldens re-blessed — this
  packet must not disturb them).
- Unblocks: `138`, `139`, `140`.
- Activation blockers: none — architecture locked by ADR-0029.

## Acceptance Criteria

- **AC-1. Given** the scheduler, **when** `STAGE_ORDER` is inspected, **then**
  `"PrePass::LightningTreeGen"` appears in the slice, positioned after `"PrePass::ShellClassification"`
  / `"PrePass::SupportGeometry"` (or whichever stage currently sits last in the prepass
  block at the time of authoring — confirmed at the FACT dispatch) and before
  `"Layer::Infill"`. | `rg -n 'LightningTreeGen' crates/slicer-scheduler/src/execution_plan.rs && cargo test -p slicer-scheduler --test stage_order_tdd 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** inspected, **then**
  `LightningTreeIR` exists with fields `schema_version: SemVer` and
  `entries: Vec<LightningTreeEntry>`, where `LightningTreeEntry` carries
  `object_id: ObjectId`, `global_layer_index: i32`, and
  `tree_edge_segments: Vec<[Point2; 2]>` (compact 2-point integer-unit storage per
  ADR-0029); a `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION: SemVer` constant exists and is
  used in the `Default` impl. | `cargo test -p slicer-ir -- lightning_tree_ir 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** an executor test where at least one region's `sparse_fill_holder` is
  `lightning-infill`, **when** the prepass runs, **then** the producer executes and commits
  a `LightningTreeIR` (empty trees are valid at this packet); **given** no lightning
  holder, **when** the prepass runs, **then** the producer is **not invoked** and the
  `LightningTreeIR` slot on the blackboard is `None` (skip promise, no commit). |
  `cargo test -p slicer-runtime --test executor -- lightning_prepass 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a `Layer::Infill` test guest calling the new read-view method
  `lightning-tree-segments`, **when** the layer dispatches, **then** the guest receives
  exactly the tree segments committed for its `(object_id, layer_index)` — count and
  endpoint equality against the host-committed fixture. | `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a default-config slice (no lightning holder) of
  `resources/regression_wedge.stl`, **when** run before and after this packet, **then** the
  g-code SHA is byte-identical (stage added, producer skipped, view absent from
  non-lightning prints). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** the WIT drift-detection suite, **when** run, **then** the
  `lightning-tree-segments` view method is asserted present and the new WIT package
  world-bump is reflected in the canonical `include_str!` paths (a guest built against the
  pre-packet WIT fails the drift check, not runtime instantiation). | `cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (rebuild if `STALE:`)

## Authoritative Docs

- `docs/adr/0029-lightning-prepass-tree-generator.md` — binding; full read (short).
- `docs/specs/lightning-infill-parity.md` §Phase L1 — full read (short).
- `docs/02_ir_schemas.md` — `SupportPlanIR` section as the IR-shape precedent (delegate).
- `docs/03_wit_and_manifest.md` — read-view contract pattern (delegate).
- `CLAUDE.md` §WIT/Type Changes Checklist + §Guest WASM Staleness — binding ceremony.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §LightningTreeIR — new IR section with versioning rules —
  `rg -q 'LightningTreeIR' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` §lightning tree read-view — the view contract —
  `rg -q 'lightning-tree-segments\|LightningTree' docs/03_wit_and_manifest.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
