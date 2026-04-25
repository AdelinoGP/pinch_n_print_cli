# Requirements: live-travel-retraction-policy

## Packet Metadata

- Grouped task IDs:
  - `TASK-120d` — restore live Benchy travel behavior on the path-optimization or emit path
  - `TASK-120d1` — decide where retract/no-retract policy lives
  - `TASK-120d2` — emit matching retract/unretract pairs and Z-hop interactions on the chosen surface
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`

## Problem Statement

The Workstream 3 travel slice is blocked by ambiguity about ownership. This packet resolves that ambiguity explicitly: retraction policy belongs on `path-optimization-default`, not on `DefaultGCodeEmitter`. The emitter serializes commands; it does not decide whether a move should retract. With that ownership fixed, this packet restores external-travel retract decisions, internal-travel suppression, Z-hop planning, and deterministic host integration coverage.

## In Scope

- retract/no-retract policy on `path-optimization-default`
- matched unretract after retracting travel moves
- deferred Z-hop planning aligned to retracting travels
- deterministic live host regressions for those decisions

## Out of Scope

- final text formatting for travel or retraction commands
- generic path ordering and mixed-tool ordering
- finalization-aware travel reconciliation

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/RetractWhenCrossingPerimeters.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`

## Acceptance Summary

### Positive Cases

- External travel emits one matched retract/unretract pair.
- Internal travel suppresses retraction.
- Retracting travel can also populate one aligned `ZHop` entry.
- Travel-policy decisions are deterministic across repeated runs.

### Negative Cases

- No-retract fixtures emit no orphan retracts and no stray Z hops.

### Measurable Outcomes

- Module-level tests assert exact command presence/absence.
- Host integration tests assert exact `LayerCollectionIR.z_hops` content and command pairing.

### Cross-Packet Impact

- Packet `11` serializes whatever decisions this packet emits.
- Packet `20` depends on this packet's policy surface before it reconciles finalization geometry.

## Verification Commands

- `cargo test -p path-optimization-default --test travel_policy_tdd external_travel_emits_matched_retract_and_unretract -- --exact --nocapture`
- `cargo test -p path-optimization-default --test travel_policy_tdd internal_travel_suppresses_retraction -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd retracting_travel_populates_matching_z_hop_and_retract_pair -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd travel_policy_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test live_travel_policy_tdd no_retract_policy_emits_no_orphan_retracts_or_z_hops -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the chosen policy surface is fixed and the corresponding failing test exists
- Postcondition: one exact travel-policy rule is observable on the live path
- Falsifying check: a focused assertion fails if a retract appears, disappears, or misorders unexpectedly