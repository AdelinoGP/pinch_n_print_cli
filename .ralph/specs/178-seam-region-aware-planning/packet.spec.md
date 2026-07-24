---
status: implemented
packet: 178-seam-region-aware-planning
task_ids:
  - TASK-294
supersedes: ../168-seam-aligned-modes/packet.spec.md
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 178-seam-region-aware-planning

## Goal

Make `PrePass::SeamPlanning` consume real per-active-region `SliceIR` geometry and preserve the full `RegionKey` identity through WIT, harvest, blackboard injection, and per-layer seam placement.

## Scope Boundaries

This packet narrows the `D-168-SEAM-PREPASS-SOURCE` deviation by closing part (1) only: it replaces the contour-ordinal `region_id` and mesh-contour source introduced by packet 168 with the full active-region `RegionKey` and per-region `SliceIR` polygons, preserving `variant_chain` through WIT, harvest, blackboard injection, and per-layer seam placement. It extends the prepass input and perimeter-region identity contracts so aligned planning can run after region and paint preparation while remaining a guest module. It does not change Orca scoring, visibility, seam-string retry, spline fitting, or continuous final-wall projection; those belong to packets 179 (parts 2-5 of D-168) and 180.

## Prerequisites and Blockers

- Depends on: implemented `.ralph/specs/168-seam-aligned-modes/` (status `implemented`, closes `TASK-274`). Reads predecessor `packet.spec.md` first per `supersedes:` row.
- Unblocks: the canonical algorithm packet at `.ralph/specs/179-seam-canonical-algorithm-fidelity/` and the final-placement packet at `.ralph/specs/180-seam-final-placement-default/`; their `task_ids` are unassigned and must be re-derived by each packet against `docs/07_implementation_status.md` at refine time (the parent parity plan's `TASK-282`/`TASK-283` row IDs are stale; `TASK-285` is also closed under packet 120).
- Activation blockers: none remaining; the parent plan `docs/specs/seam-canonical-parity-plan.md` has been reconciled 2026-07-22 (queue re-derived against `docs/07_implementation_status.md`: row 1 → `TASK-294`, row 2 → `TASK-292`, row 3 → `TASK-293`; `TASK-284` is the closed `claim:raft-fill` row of packet 124, `TASK-282`/`TASK-283` are closed under packet 117). Packets 179 and 180 stay `status= draft` and each must be re-derived at its own refine time.

## Acceptance Criteria

- **AC-1. Given** the canonical prepass WIT source, **when** `run-seam-planning` is inspected, **then** its input includes a named read-only view carrying active-region identity, per-region `SliceIR` polygons, seam segment annotations, layer Z/height, and prepass scoring width, and the world version is major-bumped from packet 168's `2.0.0`. | `python3 -c "p='crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit'; s=open(p).read(); assert 'export run-seam-planning' in s and 'variant-chain' in s and 'seam-planning' in s; assert 'package slicer:world-prepass@3.0.0;' in s"`
- **AC-2. Given** a two-variant multi-region `SliceIR` fixture with distinct `variant_chain` values, **when** `run_seam_planning` emits plans and `harvest_seam_plan_ir_from` converts them, **then** there is exactly one `SeamPlanIR.entries` record per active `(global_layer_index, object_id, region_id, variant_chain)` key and no contour ordinal is used as a region identity. | `cargo test -p slicer-runtime --test contract -- dispatch_prepass_harvest_tdd::seam_plan_ir_preserves_variant_chain 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-3. Given** per-region square and notched `SliceIR` polygons at nonuniform layer Z values, **when** aligned planning runs, **then** every nonempty `scored_candidates[*].position` lies on the owning region's supplied polygon boundary, every chosen position has the supplied layer Z, and no candidate is sourced from `MeshObjectView` sectioning. | `cargo test -p seam-planner-default --test seam_region_aware_planning_tdd 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-4. Given** two perimeter regions with the same object and numeric region ID but different variant chains, **when** `push_perimeter_regions` and `backfill_resolved_seam` perform lookup, **then** each region receives only its matching `SeamPlanEntry` and neither plan is broadcast to the sibling variant. | `cargo test -p slicer-runtime --test contract -- seam_plan_injection_matches_variant_chain 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-5. Given** a region with no supplied polygon at a layer, **when** aligned planning runs, **then** it emits no `SeamPlanEntry` for that inactive key and does not fabricate a mesh-contour or fallback region identity. | `cargo test -p seam-planner-default --test seam_region_aware_planning_tdd -- inactive_region_emits_no_plan 2>&1 | tee target/test-output.log | grep '^test result'`

## Negative Test Cases

- **AC-N1. Given** two guest seam-plan entries with identical full `RegionKey` identity, **when** the host harvests and commits them, **then** commit fails with the existing duplicate-key validation instead of silently dropping one entry. | `cargo test -p slicer-runtime --test contract -- seam_plan_ir_rejects_duplicate_region_keys 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-N2. Given** a noncanonical region ID or malformed variant-chain value at the WIT boundary, **when** seam-plan output is harvested, **then** the host returns a structured invalid-identity error and does not commit `SeamPlanIR`. | `cargo test -p slicer-runtime --test contract -- seam_plan_ir_rejects_invalid_region_identity 2>&1 | tee target/test-output.log | grep '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated document-map locations for the normative architecture docs.
- `docs/01_system_architecture.md` - delegated PrePass order, stage I/O, claim, and seam-first contract locations.
- `docs/02_ir_schemas.md` - delegated `SeamPlanIR`, `PerimeterRegion`, `SeamCandidate`, and identity locations.
- `docs/03_wit_and_manifest.md` - delegated world-prepass and claim locations.
- `docs/04_host_scheduler.md` - direct bounded stage-order and IR-access policy evidence through delegated locations.
- `docs/08_coordinate_system.md` - direct coordinate contract; supplied polygon coordinates are integer units and seam positions are millimetres.
- `docs/11_operational_governance_and_acceptance_gate.md` - delegated WIT major-bump and closure-gate policy locations.
- `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` - accepted prepass placement decision.
- `docs/DEVIATION_LOG.md` - `D-168-SEAM-PREPASS-SOURCE` predecessor deviation.

## Doc Impact Statement (Required)

- `docs/01_system_architecture.md` PrePass stage order and seam-planning source — must add a sentence tying `SeamPlanning` to per-region `SliceIR` input, not mesh contours. | `rg -q 'SeamPlanning.*per-region.*SliceIR|per-region.*SliceIR.*SeamPlanning' docs/01_system_architecture.md`
- `docs/02_ir_schemas.md` `RegionKey`, `PerimeterRegion`, and `SeamPlanIR` sections — `SeamPlanIR.entries[*]` must carry the full `variant_chain`. | `rg -q 'variant_chain.*SeamPlanIR|SeamPlanIR.*variant_chain' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` world-prepass signature and claim contract — must reference the new per-region input view, not the prior `layer-plan` only. | `rg -q 'run-seam-planning.*variant-chain|seam-planning.*variant-chain' docs/03_wit_and_manifest.md`
- `docs/15_config_keys_reference.md` aligned default and seam mode values — keeps `seam_mode` listed; the `aligned` default change belongs to packet 180, not this one. | `rg -q 'aligned.*default|seam_mode' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` narrow `D-168-SEAM-PREPASS-SOURCE` (close part 1 only; parts 2-5 stay Open for packet 179) — must add a "Narrowed by packet 178" note. | `rg -q 'D-168-SEAM-PREPASS-SOURCE.*Narrowed|Narrowed.*D-168-SEAM-PREPASS-SOURCE' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — canonical final-perimeter candidate source and placement inputs that PNP must approximate through per-region `SliceIR` before final wall projection.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — canonical `Perimeter`/`SeamCandidate` identity and candidate metadata fields.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
