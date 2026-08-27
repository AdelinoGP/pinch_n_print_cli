---
status: draft
packet: 244-order-locked-extrusion-sequences
task_ids:
  - TASK-354
depends_on:
  - 243-object-scoped-overhang-annotation
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 244-order-locked-extrusion-sequences

## Goal

Land the generic `order_lock` contract — the `ExtrusionPath3D.order_lock: Option<u64>` carrier, its
WIT/`OrderedEntityView` projection, the SDK local-tag allocator, and host-side tag remapping plus
enforcement at all four mutation points — provably changing nothing for existing slices (all-`None`
paths take the old-equivalent branches).

## Scope Boundaries

This packet lands the carrier and the host enforcement contract only. It does NOT make any consumer
honor locks: the infill linker, path optimizer, and G-code emitter keep their current behavior for
locked paths (they are Packet 3's scope). No module emits `order_lock` yet, so every existing slice
runs the all-`None` path and must be byte-identical to today. The enforcement points validate and
reject violations, but with no producer minting locks, no violation can occur in production — the
enforcement is exercised only by the new unit tests.

## Prerequisites and Blockers

- Depends on: 243-object-scoped-overhang-annotation (the object-scoped `prev_layer_boundaries` shape
  is the `prev_object_boundary` source Packet 4 consumes; this packet's WIT/IR change is independent
  of it but the queue is dependency-ordered).
- Unblocks: 245-lock-aware-infill-consumers (which makes the linker/optimizer/emitter honor locks).
- Activation blockers: none known; the change is additive (`#[serde(default)]`, `option<u64>`) and
  all-`None` preserves today's behavior.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the tree, **when** the schema constant is inspected, **then**
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` equals `SemVer { major: 1, minor: 4, patch: 0 }` and
  `LayerCollectionIR::default().schema_version` equals the constant. |
  `rg -q 'minor: 4,' crates/slicer-ir/src/slice_ir.rs && cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_SCHEMA_1_4_0`
- **AC-2. Given** the tree, **when** the carrier is inspected end-to-end, **then**
  `ExtrusionPath3D.order_lock: Option<u64>` (`#[serde(default)]`) exists in
  `crates/slicer-ir/src/slice_ir.rs`, WIT `extrusion-path3d` carries `order-lock: option<u64>` in
  `crates/slicer-schema/wit/deps/types.wit`, WIT `ordered-entity-view` carries
  `order-lock: option<u64>` in `crates/slicer-schema/wit/deps/ir-types.wit`, and the SDK
  `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`) plus the host `OrderedEntityView`
  (`crates/slicer-runtime/src/layer_executor.rs`) each carry `order_lock: Option<u64>`. |
  `rg -q 'pub order_lock: Option<u64>' crates/slicer-ir/src/slice_ir.rs && rg -q 'order-lock: option<u64>' crates/slicer-schema/wit/deps/types.wit && rg -q 'order-lock: option<u64>' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'pub order_lock: Option<u64>' crates/slicer-sdk/src/views.rs && rg -q 'pub order_lock: Option<u64>' crates/slicer-runtime/src/layer_executor.rs && echo P244_CARRIER_END_TO_END`
- **AC-3. Given** a slice with every path carrying `order_lock: None`, **when** the full pipeline
  runs, **then** the produced `LayerCollectionIR` / G-code is structurally identical to the
  pre-packet output (all-`None` neutrality — no new branch changes any existing path). |
  `cargo test -p slicer-runtime --test executor -- order_lock_all_none_neutrality --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_ALL_NONE_NEUTRAL`
- **AC-4. Given** a `LayerCollectionIR` whose `ordered_entities` contain a locked block (two or more
  adjacent paths sharing a non-`None` `order_lock`), **when** `apply_entity_order_proposal` is given a
  proposal that splits, interleaves, reverses, or internally reorders that block, **then** the call
  returns `Err` naming the violation and the prior `LayerCollectionIR` is preserved unchanged. |
  `cargo test -p slicer-runtime --test unit -- order_lock_proposal_split_rejected --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_PROPOSAL_ENFORCED`
- **AC-5. Given** module output carrying local tags (`Some(t)` with bit 63 clear) and a host remap
  counter, **when** `remap_order_locks_to_global` runs, **then** each local tag is rewritten to a
  layer-unique global tag (bit 63 set, monotonically increasing from the counter), `Some(0)` is
  rejected, and an unknown global tag (bit 63 set but not previously minted) is a contract error. |
  `cargo test -p slicer-runtime --test unit -- order_lock_remap_local_to_global --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_REMAP_ENFORCED`
- **AC-6. Given** a fresh `OrderLockAllocator`, **when** `allocate()` is called repeatedly, **then**
  it returns `Some(1)`, `Some(2)`, `Some(3)` in deterministic order and `None` once the local-tag
  space (`1..2^63-1`) is exhausted. |
  `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_allocator_sequence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_ALLOCATOR_SEQUENCE`

## Negative Test Cases

- **AC-N1. Given** an `InfillPostProcess` replacement `InfillIR` that drops, reorders, reverses, or
  alters the widths of a locked block present in the prior `InfillIR`, **when** the commit runs,
  **then** the commit returns a fatal `LayerStageError::OrderLockViolation` and the prior `InfillIR`
  is preserved (atomic). |
  `cargo test -p slicer-runtime --test executor -- order_lock_infill_postprocess_preserves_block --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_INFILL_POSTPROCESS_ENFORCED`
- **AC-N2. Given** a finalization `modify_entity` that changes the points or widths of a locked
  entity, or a `sort_layer_by` that splits or internally reorders a locked block, **when** `apply_to`
  runs, **then** it returns `Err` naming the violation and the prior layers are preserved. |
  `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_finalization_rejects_geometry_change --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P244_FINALIZATION_ENFORCED`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests` (after the WIT change; then `cargo xtask build-guests --check` must exit 0 before attributing any failure)

## Authoritative Docs

- `docs/02_ir_schemas.md` - direct range read of §"IR 12 — LayerCollectionIR" (lines ~1185-1195) and
  §"IR Versioning Contract" (lines ~1633-1641); the doc is over 300 lines so only these ranges are
  read directly.
- `docs/specs/wave-overhangs-bridge-fill-plan.md` - normative plan; §"Packet 2 — Order-lock carrier
  and host enforcement" and Appendix A (ADR draft) / Appendix B (glossary) are the governing brief.

## Doc Impact Statement (Required)

- `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` - new ADR, content from
  the plan's Appendix A draft (re-derived number 0062, the next free after 0061). |
  `rg -q '^# ADR-0062' docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md && echo P244_ADR_LANDED`
- `CONTEXT.md` - add the "Order lock" and "Anchor band" glossary terms to the infill/fill cluster
  (plan Appendix B, verbatim). |
  `rg -q '^### Order lock' CONTEXT.md && rg -q '^### Anchor band' CONTEXT.md && echo P244_GLOSSARY_LANDED`
- `docs/02_ir_schemas.md` §"IR 12 — LayerCollectionIR" - update `Current schema_version: 1.3.0` to
  `1.4.0` and document the additive `ExtrusionPath3D.order_lock` field (packet-226 `tool_index`
  precedent). |
  `rg -q 'Current schema_version: 1\.4\.0' docs/02_ir_schemas.md && rg -q 'order_lock' docs/02_ir_schemas.md && echo P244_IR_DOCS_UPDATED`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
