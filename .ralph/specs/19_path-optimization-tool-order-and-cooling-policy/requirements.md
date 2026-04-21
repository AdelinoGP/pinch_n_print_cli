# Requirements: path-optimization-tool-order-and-cooling-policy

## Packet Metadata

- Grouped task IDs:
  - `TASK-152` — expand path optimization beyond comment-only output for the tool-ordering slice
  - `TASK-152b` — emit deterministic tool-change ordering for mixed-tool layers
  - `TASK-152c` — close cooling override policy explicitly
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Workstream 3 still lacks a deterministic mixed-tool ordering surface, and TASK-152c is still unresolved. This packet resolves both together without widening scope into a new cooling API. The selected approach is deliberate: implement tool ordering on the live path, but close cooling overrides on the documentation rejection path because the current live module surface has no clean fan-speed/cooling control contract and adding one would reopen the postpass control plane.

## In Scope

- mixed-tool grouping and deferred `ToolChange` sequencing
- docs-driven rejection path for cooling overrides

## Out of Scope

- generic entity ordering
- retraction policy and Z hops
- finalization-aware travel coordination
- adding new fan-speed/cooling config keys or WIT members

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`

## Acceptance Summary

### Positive Cases

- Mixed-tool layers emit deterministic grouped tool ordering and deferred `ToolChange` entries.
- Single-tool layers emit no synthetic tool changes.
- Docs explicitly state the rejection path for cooling overrides.

### Negative Cases

- Canonical or single-tool sequences do not emit redundant tool changes.

### Measurable Outcomes

- Acceptance tests assert exact tool order and exact `ToolChange` sequence.
- The docs rejection path is verified by exact text grep, not implied prose.

### Cross-Packet Impact

- Packet `20` assumes tool ordering is deterministic when wipe geometry is present.
- Packet `21` uses this packet's decisions when asserting final Benchy travel and tool-change evidence.

## Verification Commands

- `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the mixed-tool or docs policy surface is isolated
- Postcondition: one exact tool-order or docs contract is observable
- Falsifying check: a focused sequence or grep assertion fails if the rule regresses