---
status: draft
packet: path-optimization-tool-order-and-cooling-policy
task_ids:
  - TASK-152
  - TASK-152b
  - TASK-152c
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: path-optimization-tool-order-and-cooling-policy

## Goal

Add deterministic mixed-tool ordering on the live path-optimization surface and close the cooling-override decision explicitly on the documentation rejection path: tool ordering is implemented here, while fan-speed and cooling overrides are documented as intentionally unsupported on the live `Layer::PathOptimization` surface.

## Scope Boundaries

- In scope:
  - deterministic mixed-tool ordering and deferred `ToolChange` population on the live path-optimization surface
  - one explicit decision for TASK-152c: cooling and fan-speed overrides remain intentionally unsupported on the live path-optimization surface
  - documentation updates in `docs/05_module_sdk.md` and `docs/07_implementation_status.md` that lock the rejection path for cooling overrides
- Out of scope:
  - generic entity ordering heuristics (packet `18`)
  - retract/no-retract policy (packet `15`)
  - finalization-aware wipe/brim travel coordination (packet `20`)
  - implementing a new cooling or fan-control WIT/config surface

## Prerequisites and Blockers

- Depends on:
  - packet `18` providing a stable entity ordering foundation
  - existing deferred tool-change queue support in the host path-optimization surface
- Unblocks:
  - packet `20` and packet `21`, which need deterministic tool sequencing when wipe-related geometry is present
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a layer fixture whose raw entities use tools `0`, `2`, and `1`, **when** the mixed-tool ordering helper runs, **then** the resulting grouped tool sequence is exactly tool `0` entities first, then tool `1`, then tool `2`, and the deferred `LayerCollectionIR.tool_changes` sequence is exactly `0->1`, `1->2`. | `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- **Given** a layer fixture whose entities all use tool `0`, **when** the same helper runs, **then** `LayerCollectionIR.tool_changes` remains empty. | `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- **Given** TASK-152c is closed on the rejection path, **when** `docs/05_module_sdk.md` and `docs/07_implementation_status.md` are inspected, **then** both docs state that fan-speed and cooling overrides are intentionally unsupported on the live `Layer::PathOptimization` surface and no new live-path cooling override surface is introduced in this packet. | `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`

## Negative Test Cases

- **Given** a single-tool layer or a mixed-tool layer already grouped in canonical order, **when** the ordering helper runs, **then** it does not emit redundant tool changes or reorder the already canonical tool grouping. | `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — path-optimization and tool-change ownership
- `docs/04_host_scheduler.md` — deferred tool-change queue behavior
- `docs/05_module_sdk.md` — module-surface documentation for rejection path
- `docs/07_implementation_status.md` — TASK-152b and TASK-152c closure notes

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`