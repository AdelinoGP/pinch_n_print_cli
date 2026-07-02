---
status: draft
packet: 137_lightning-prepass-contract
task_ids:
  - TASK-262
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 137_lightning-prepass-contract

## Goal

Land the lightning-parity architecture seam (ADR-0029): a `PrePass::LightningTreeGen` stage
in `STAGE_ORDER`, a schema-versioned `LightningTreeIR` (per object, per layer tree-edge
segments), a host producer skeleton that is skipped when no region's sparse holder is
`lightning-infill`, and a WIT read-view letting a `Layer::Infill` module read its layer's
trees.

## Scope Boundaries

Contract and plumbing only: stage registration, IR type + docs, producer skeleton (commits an
empty-but-valid IR when lightning is configured; skipped otherwise), the WIT read-view with
guest plumbing, and drift-test coverage. The generator algorithm ports land in 138/139; the
module rewrite in 140. Non-lightning prints are byte-identical.

## Prerequisites and Blockers

- Depends on: `136_infill-parity-integration` (roadmap order; goldens re-blessed —
  this packet must not disturb them).
- Unblocks: `138`, `139`, `140`.
- Activation blockers: none — architecture locked by ADR-0029.

## Acceptance Criteria

- **AC-1. Given** the scheduler, **when** `STAGE_ORDER` is inspected, **then**
  `PrePass::LightningTreeGen` is present, positioned after the stages producing sparse-infill
  outlines and before `Layer::Infill` dispatch. | `rg -q 'LightningTreeGen' crates/slicer-scheduler/src/execution_plan.rs && cargo test -p slicer-scheduler -- stage_order 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** inspected, **then**
  `LightningTreeIR` exists with `schema_version`, per-object per-layer tree-edge segment
  storage (2-point integer-unit segments), and a `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`
  constant. | `cargo test -p slicer-ir -- lightning_tree_ir 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a model where at least one region's `sparse_fill_holder` is
  `lightning-infill`, **when** the prepass runs, **then** the producer executes and commits a
  `LightningTreeIR` (empty trees are valid at this packet); **given** no lightning holder,
  **then** the producer is skipped (no commit, no IR in the blackboard). | `cargo test -p slicer-runtime --test executor -- lightning_prepass_skip_and_commit 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a `Layer::Infill` test guest calling the new read-view, **when** the layer
  dispatches, **then** the guest reads exactly the tree segments committed for its
  (object, layer) — count and endpoint equality. | `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a default-config slice (no lightning holder) of
  `resources/regression_wedge.stl`, **when** run before and after this packet, **then** the
  g-code SHA is byte-identical (stage present, producer skipped). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** the WIT drift-detection suite, **when** run, **then** the new
  lightning-tree view types are asserted present (a guest built against the old WIT fails the
  drift check, not runtime instantiation). | `cargo test -p slicer-runtime --test contract -- wit_drift 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/adr/0029-lightning-prepass-tree-generator.md` — binding; full read (short).
- `docs/specs/lightning-infill-parity.md` §Phase L1 — full read (short).
- `docs/02_ir_schemas.md` — delegate; `SupportPlanIR` section as the IR-shape precedent.
- `CLAUDE.md` §WIT/Type Changes Checklist — binding ceremony.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §LightningTreeIR — new IR section with versioning rules —
  `rg -q 'LightningTreeIR' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` §lightning tree view — the read-view contract —
  `rg -q 'lightning' docs/03_wit_and_manifest.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
