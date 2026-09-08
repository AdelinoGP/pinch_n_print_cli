---
status: superseded
packet: 214-support-fallback-overhang-clip
task_ids:
  - TASK-323
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
superseded_by: 220-support-analysis-family-contracts + 222-traditional-support-family (fallback fillers removed; traditional family planner replaces clipping `region.overhang_areas()` inside a per-layer filler)
superseded_on: 2026-08-12
---

# Packet Contract: 214-support-fallback-overhang-clip

> **SUPERSEDED 2026-08-12 by the support-families and anchored-entities sequence** (`docs/specs/support-families-anchored-entities-plan.md`). The fallback fillers are removed entirely; the traditional family planner (`traditional-support-planner`, packet 222) performs contact detection and body planning, and the host marshalling `needs_support` derivation moves into `PrePass::SupportAnalysis` (packet 220). Directory retained intact for provenance; do not implement as-is.

## Goal

Restrict fallback support generation to each region's pre-filtered `overhang_areas()` and derive `needs_support` from whether those overhang areas are non-empty, while preserving full-polygon filling for enforced regions.

## Scope Boundaries

This packet changes the DefaultEligible fallback fill paths in traditional-support and tree-support plus the existing host marshalling assignment of `SliceRegionData.needs_support`. It does not change planner propagation, IR/WIT shapes, paint precedence, raft, interfaces, support variants, or G-code emission.

## Prerequisites and Blockers

- Depends on: approved `docs/specs/support-generation-remediation-plan.md`; verified `docs/specs/support-generation-defect-verified-findings.md`
- Unblocks: support G-code end-to-end packet after the independent geometry packets
- Activation blockers: none

## Acceptance Criteria

- **AC-1. Given** a DefaultEligible region with non-empty `region.overhang_areas()` and a larger `region.polygons()` footprint, **when** either fallback `run_support` path fills the region, **then** it invokes the filler only for `overhang_areas()` while Enforced still iterates `polygons()`. | `rg -q 'overhang_areas\(\)' modules/core-modules/traditional-support/src/lib.rs && rg -q 'overhang_areas\(\)' modules/core-modules/tree-support/src/lib.rs && rg -q 'SupportPaintPolicy::Enforced' modules/core-modules/traditional-support/src/lib.rs && rg -q 'SupportPaintPolicy::Enforced' modules/core-modules/tree-support/src/lib.rs`
- **AC-2. Given** `sliced_region_to_data` has already collected `overhang_areas`, **when** it constructs `SliceRegionData`, **then** the exact field assignment is `needs_support: !overhang_areas.is_empty()` and no hardcoded `needs_support: true` remains at that construction site. | `rg -q 'needs_support:\s*!overhang_areas\.is_empty\(\)' crates/slicer-wasm-host/src/marshal/in_.rs && ! rg -n 'needs_support:\s*true' crates/slicer-wasm-host/src/marshal/in_.rs`
- **AC-3. Given** `tmp/visual-debug-support.json` requests `Layer::Support` at layers 10, 24, and 30, **when** the fixed fallback path is rendered, **then** the output manifest contains all three requested layer captures and the support paths are clipped to overhang geometry rather than the pillar footprint on those non-overhang layers. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support.json --output target/vd-support-fixed --overwrite >/dev/null && test -f target/vd-support-fixed/manifest.json && rg -q '"layer": 10' target/vd-support-fixed/manifest.json && rg -q '"layer": 24' target/vd-support-fixed/manifest.json && rg -q '"layer": 30' target/vd-support-fixed/manifest.json`

## Negative Test Cases

- **AC-N1. Given** a region with a `SupportPaintPolicy::Blocked` policy or a non-overhang DefaultEligible region with empty `overhang_areas()`, **when** `run_support` evaluates it, **then** it emits no fallback support paths, while `SupportPaintPolicy::Enforced` remains eligible to fill `polygons()`. | `rg -q 'SupportPaintPolicy::Blocked' modules/core-modules/traditional-support/src/lib.rs && rg -q 'SupportPaintPolicy::Blocked' modules/core-modules/tree-support/src/lib.rs && rg -q 'SupportPaintPolicy::Enforced' modules/core-modules/traditional-support/src/lib.rs && rg -q 'SupportPaintPolicy::Enforced' modules/core-modules/tree-support/src/lib.rs`

## Verification

- `cargo xtask build-guests --check`
- `cargo check --workspace --all-targets`
- `cargo test -p slicer-wasm-host --all-targets`

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct read, lines 14-53 and 55-64
- `docs/specs/support-generation-defect-verified-findings.md` - direct read, lines 43-55, 88-127, 178-231, and 255-284
- `docs/01_system_architecture.md` - support paint precedence and stage contracts
- `docs/02_ir_schemas.md` - `needs_support` and overhang-area contracts

## Doc Impact Statement (Required)

**`none`** - the existing `needs_support` and `overhang_areas` fields are reused; no schema, WIT, scheduler, manifest, SDK, or documentation contract changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
