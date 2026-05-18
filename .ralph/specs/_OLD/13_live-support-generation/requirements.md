# Requirements: live-support-generation

## Packet Metadata

- Grouped task IDs:
  - `TASK-120b` — restore support generation on the live Benchy path
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Support generators exist and have unit coverage, but the live Benchy path still lacks committed support content. The missing slice is the production `Layer::Support` handoff and commit path. This packet keeps the scope small by treating tree-support as the canonical live acceptance target for the final Benchy run while using traditional-support as a control generator to guard the shared host path and the documented paint precedence rules.

## In Scope

- live support-stage dispatch and host commitment into `SupportIR`
- tree-support live-path acceptance coverage
- control coverage for traditional-support and paint blocker/enforcer precedence
- deterministic support-stage regressions

## Out of Scope

- support text emission and Orca labels
- top/bottom fill restoration
- seam placement, travel policy, and final Benchy feature evidence

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — tree-support behavior and expectations
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.hpp` — organic-tree reference surface
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — control generator reference
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.hpp` — shared support toolpath expectations

## Acceptance Summary

### Positive Cases

- Tree-support commits non-empty `SupportIR.support_paths` through the live host path.
- Traditional-support commits non-empty `SupportIR.support_paths` through the same host path.
- SupportBlocker wins over default eligibility.
- SupportEnforcer can force live support commitment even when `needs_support=false`.
- Repeated identical support-stage runs are deterministic.

### Negative Cases

- Disabled or ineligible support fixtures commit empty `SupportIR` collections.

### Measurable Outcomes

- Acceptance tests assert exact `SupportIR.support_paths` emptiness/non-emptiness and exact `ExtrusionRole::SupportMaterial` roles.
- The control paint-precedence cases stay on executable tests, not prose.

### Cross-Packet Impact

- Packet `11` serializes these restored support roles into Orca-facing output.
- Packet `21` uses this packet to assert support presence on the Benchy path.

## Verification Commands

- `cargo test -p slicer-host --test live_support_generation_tdd tree_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd enforcer_forces_live_support_commit_even_when_needs_support_is_false -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd live_support_dispatch_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd disabled_or_ineligible_support_stage_commits_empty_support_ir -- --exact --nocapture`
- `cargo test -p tree-support --test enforcer_blocker_tdd blocker_overrides_needs_support_true -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the current host support-stage gap is localized to one dispatch or generator surface
- Postcondition: one real support generator commits the exact expected `SupportIR` shape
- Falsifying check: a focused test fails if support remains empty or loses its role semantics