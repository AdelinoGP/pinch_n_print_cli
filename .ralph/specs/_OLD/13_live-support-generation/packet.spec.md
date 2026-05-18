---
status: implemented
packet: live-support-generation
task_ids:
  - TASK-120b
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: live-support-generation

## Goal

Restore support generation on the live Benchy path by making the real `Layer::Support` stage commit non-empty `SupportIR` content on the production host path, with tree-support as the canonical acceptance target for the final Benchy run and traditional-support retained as the control generator for unit-level role and paint-precedence coverage.

## Scope Boundaries

- In scope:
  - live support-stage dispatch from `SliceRegionView` and `PaintRegionLayerView` into the canonical support generators
  - production-path commitment of `SupportIR.support_paths` on the host
  - control coverage for paint blocker/enforcer precedence and disabled/no-eligible-region cases
  - tree-support live-path regression sufficient to unblock the Phase H Benchy path that expects tree supports enabled
- Out of scope:
  - Orca-facing support comment emission (packet `11`)
  - top/bottom fill generation (packet `12`)
  - seam placement and travel policy (packets `14` and `15`)
  - final Benchy feature-evidence assertions (packet `21`)

## Prerequisites and Blockers

- Depends on:
  - live `Layer::Support` dispatch in `crates/slicer-host/src/dispatch.rs`
  - `modules/core-modules/tree-support` and `modules/core-modules/traditional-support`
- Unblocks:
  - TASK-135 support evidence on Benchy
  - TASK-119 support role emission checks once support entities are present
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a support-eligible layer fixture and `tree-support` selected as the `support-generator` claim holder, **when** the real host `Layer::Support` dispatch runs, **then** the committed `SupportIR.support_paths` is non-empty and every committed path uses `ExtrusionRole::SupportMaterial`. | `cargo test -p slicer-host --test live_support_generation_tdd tree_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- **Given** the same live dispatch fixture with `traditional-support` selected instead, **when** the support stage runs, **then** the committed `SupportIR.support_paths` is non-empty and every committed path uses `ExtrusionRole::SupportMaterial`. | `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- **Given** a support-eligible region overlapped by `PaintSemantic::SupportBlocker`, **when** the production support path runs, **then** `SupportIR.support_paths.len()` is `0` even if `needs_support=true`. | `cargo test -p tree-support --test enforcer_blocker_tdd blocker_overrides_needs_support_true -- --exact --nocapture`
- **Given** a region with `needs_support=false` but overlapped by `PaintSemantic::SupportEnforcer`, **when** the support generator runs, **then** the resulting support output is non-empty and the live host path commits those paths into `SupportIR.support_paths`. | `cargo test -p slicer-host --test live_support_generation_tdd enforcer_forces_live_support_commit_even_when_needs_support_is_false -- --exact --nocapture`
- **Given** the same support-stage fixture executed twice with the same module/config selection, **when** the host commits `SupportIR`, **then** the resulting path count and per-path point coordinates are byte-deterministic across both runs. | `cargo test -p slicer-host --test live_support_generation_tdd live_support_dispatch_is_deterministic_across_repeated_runs -- --exact --nocapture`

## Negative Test Cases

- **Given** a layer with `support_enabled=false` or no support-eligible regions, **when** the real support stage runs, **then** `SupportIR.support_paths`, `interface_paths`, and `raft_paths` are all empty. | `cargo test -p slicer-host --test live_support_generation_tdd disabled_or_ineligible_support_stage_commits_empty_support_ir -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test live_support_generation_tdd tree_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd enforcer_forces_live_support_commit_even_when_needs_support_is_false -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd live_support_dispatch_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd disabled_or_ineligible_support_stage_commits_empty_support_ir -- --exact --nocapture`
- `cargo test -p tree-support --test enforcer_blocker_tdd blocker_overrides_needs_support_true -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — support-stage semantics and paint precedence
- `docs/02_ir_schemas.md` — `SupportIR`, `SupportType`, and `ExtrusionRole::SupportMaterial`
- `docs/04_host_scheduler.md` — `Layer::Support` dispatch contract
- `docs/07_implementation_status.md` — TASK-120b scope and Workstream 3 sequencing

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp`
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.hpp`
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.hpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`