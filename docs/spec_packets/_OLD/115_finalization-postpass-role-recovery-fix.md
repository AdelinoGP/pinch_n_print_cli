---
status: implemented
packet: 115_finalization-postpass-role-recovery-fix
task_ids: []
---

# 115_finalization-postpass-role-recovery-fix

## Goal

Collapse the inbound (WIT→IR) `extrusion-role` converter in `marshal` to the single recovering form so finalization and postpass recover the reserved builtin roles `PrimeTower`/`Skirt` from their `Custom` tags — fixing the latent finalization misclassification — and pin it with round-trip and dispatch regression tests.

## Problem Statement

The WIT `extrusion-role` enum has 12 variants and omits the reserved builtin roles `PrimeTower` and `Skirt`; the IR enum has them. Crossing IR→WIT encodes them as `Custom("<builtin tag>")`. The layer-world WIT→IR converter recovers them (`Custom(tag) => PrimeTower/Skirt`), but the finalization and postpass copies keep `Custom(s) => Custom(s)` — they never recover.

This is a latent bug, traced while implementing packet 113 (full analysis in ADR-0021's 2026-06-16 amendment). `Skirt`/`PrimeTower` entities live in `LayerCollectionIR.ordered_entities` before `PostPassLayerFinalization`. A finalization guest that re-emits paths returns the role as `Custom("…/skirt@1")`; the immediately following `PostPassGCodeEmit` then misclassifies it — `resolve_feedrate` falls back to `outer_wall_speed` instead of `skirt_speed`/`prime_tower_speed`, `orca_type_label` emits `;TYPE:Custom` instead of `;TYPE:Skirt/Brim`, and the skirt-travel-insertion filter (`role == ExtrusionRole::Skirt`) misses the entity entirely. The postpass copy is currently inert (its output feeds `GCodeIR`, which no later stage matches on by typed role) but is the same defect and is fixed for consistency. Existing tests cover only the outbound encoding, so the lossy round-trip is undetected.

## Architecture Constraints

- Host-only change: `marshal/leaf.rs` + a host test. **No path in this packet feeds the guest WASM build**, so no `cargo xtask build-guests` is required (no `wasm-staleness` constraint). No geometry/mm math (no `coord-system` constraint).
- The recovering converter is already the layer-world behaviour; this packet makes finalization/postpass use the *same* function — completing ADR-0021's "one converter per concept".

## Data and Contract Notes

- Recovering converter (single source): map the 12 base variants 1:1, then `Custom(s) if s == PRIME_TOWER_TAG => PrimeTower`, `Custom(s) if s == SKIRT_TAG => Skirt`, else `Custom(s.clone())`. Tags are `BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG` / `..._SKIRT_TAG`.
- Round-trip test asserts: for `r ∈ {PrimeTower, Skirt}`, `convert_extrusion_role(&ir_to_wit_extrusion_role(&r)) == r`.

## Locked Assumptions and Invariants

- The recovering form is the *correct* behaviour for all worlds (layer already uses it; nothing relies on the lossy form — postpass's typed role is read by no later stage).
- Outbound (IR→WIT) encoding is unchanged; only inbound recovery changes.

## Risks and Tradeoffs

- **Behaviour change** (the point): finalization/postpass roles that were `Custom(tag)` become typed. Risk that some test pinned the *lossy* output — search for assertions expecting `Custom("…/skirt@1")`/`prime_tower` on a finalization/postpass path and update them as part of the fix (they were pinning a bug). Mitigated by running the `slicer-wasm-host` contract bucket.
- Low blast radius: one converter, two call sites, two tests.
