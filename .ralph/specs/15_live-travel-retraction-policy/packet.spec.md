---
status: draft
packet: live-travel-retraction-policy
task_ids:
  - TASK-120d
  - TASK-120d1
  - TASK-120d2
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: live-travel-retraction-policy

## Goal

Make `path-optimization-default` the canonical decision surface for live travel, retract/no-retract, and Z-hop policy on the Benchy path, while keeping packet `11` responsible only for how those decisions serialize to final GCode text.

## Scope Boundaries

- In scope:
  - explicit decision that retract/no-retract policy lives on `path-optimization-default`
  - external-travel retract decisions and internal-travel suppression decisions on the live path
  - matched unretract after retracting travel moves
  - module-side Z-hop planning using the existing deferred `z_hops` queue
  - live host regressions for deterministic travel decisions
- Out of scope:
  - broader entity ordering heuristics (packet `18`)
  - tool ordering and cooling policy (packet `19`)
  - final text serialization shape (packet `11`)
  - finalization-aware travel reconciliation (packet `20`)

## Prerequisites and Blockers

- Depends on:
  - packet `14` turning the path-optimization surface into something richer than a comment-only seam consumer
  - existing `GcodeOutputBuilder.push_retract`, `push_unretract`, and `push_z_hop` support surfaces
- Unblocks:
  - TASK-135 retract/unretract Benchy evidence
  - packet `20` finalization-aware travel coordination
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** two replayed wall-loop clusters separated by a travel that crosses outside the current interior region and config `retract_length=0.8`, **when** `path-optimization-default` evaluates the travel, **then** the captured output contains one `Retract { length: 0.8, ... }`, one travel `Move` with `e=None`, and one matching `Unretract { length: 0.8, ... }` in that order. | `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture`
- **Given** a travel that remains inside the same internal region, **when** the same module evaluates that move, **then** the captured output contains no `Retract` and no `Unretract` command for that travel. | `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture`
- **Given** config `travel_z_hop=0.2` and a travel that requires retraction, **when** the live path-optimization stage runs, **then** the resulting host layer output contains one `ZHop { hop_height: 0.2 }` aligned to the retracting travel segment and the retract/unretract pair remains present. | `cargo test -p slicer-host --test live_travel_policy_tdd retracting_travel_populates_matching_z_hop_and_retract_pair -- --exact --nocapture`
- **Given** the same travel-policy fixture executed twice, **when** the live path-optimization stage runs both times, **then** the emitted retract/unretract/Z-hop decisions are byte-identical across the two runs. | `cargo test -p slicer-host --test live_travel_policy_tdd travel_policy_is_deterministic_across_repeated_runs -- --exact --nocapture`

## Negative Test Cases

- **Given** a layer fixture whose travel policy chooses no retract for every move, **when** the live path runs, **then** the resulting output contains no duplicate retract without an intervening unretract and contains no stray `ZHop` entry. | `cargo test -p slicer-host --test live_travel_policy_tdd no_retract_policy_emits_no_orphan_retracts_or_z_hops -- --exact --nocapture`

## Verification

- `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture`
- `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd retracting_travel_populates_matching_z_hop_and_retract_pair -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd travel_policy_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd no_retract_policy_emits_no_orphan_retracts_or_z_hops -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — path-optimization and per-layer travel ownership
- `docs/02_ir_schemas.md` — `GCodeCommand`, `LayerCollectionIR.z_hops`, and role contracts
- `docs/04_host_scheduler.md` — `Layer::PathOptimization` order and deferred queue behavior
- `docs/07_implementation_status.md` — TASK-120d / TASK-120d1 / TASK-120d2 scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/RetractWhenCrossingPerimeters.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`