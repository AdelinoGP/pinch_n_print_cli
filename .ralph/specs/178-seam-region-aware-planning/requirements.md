# Requirements: 178-seam-region-aware-planning

## Packet Metadata

- Grouped task IDs: `TASK-291` (re-derived 2026-07-22 against `docs/07_implementation_status.md`; the previously quoted `TASK-284` row is the closed `claim:raft-fill` row of packet 124, see `docs/07_implementation_status.md:243`; the original `TASK-281` row is closed under packet 117, see `docs/07_implementation_status.md:241`)
- Supersedes: `.ralph/specs/168-seam-aligned-modes/packet.spec.md`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

Packet 168 implemented aligned planning over mesh-derived contours before
region mapping and assigned `perimeter_idx.to_string()` as `region_id`. That
works for the single-region prism fixture but cannot address PNP active regions,
painted variants, or their `variant_chain` identities. The follow-up must replace
that identity/source mismatch without moving cross-layer alignment into a
parallel per-layer module or a host builtin.

## In Scope

- Extend the world-prepass seam-planning input with active-region identity, per-region `SliceIR` polygons, segment annotations, real layer metadata, and prepass scoring width.
- Execute seam planning only after the required region/slice/paint products are committed and before layer dispatch.
- Preserve `RegionKey.variant_chain` through WIT seam-plan output, SDK types, host harvest, blackboard `SeamPlanIR`, perimeter-region views, and injection lookup.
- Make `seam-planner-default` emit plans keyed by actual active regions, with no contour ordinal identity.
- Add multi-region, painted-variant, inactive-region, duplicate-key, and host-injection tests.
- Bump affected WIT/IR schema versions according to `docs/11_operational_governance_and_acceptance_gate.md` and record all struct-literal fallout in the implementation step.

## Out of Scope

- Canonical comparator, visibility, overhang, alternative-start retry, or B-spline solver changes; packet 179 (`179-seam-canonical-algorithm-fidelity`) owns those.
- Continuous final-wall projection, path-point insertion, flag/width rotation changes, default-mode changes, and degraded fallback diagnostics; packet 180 (`180-seam-final-placement-default`) owns those.
- Changes to OrcaSlicer source or direct final-perimeter generation.
- Host-native alignment policy or a second cross-layer state channel.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — final perimeter polygon/candidate ownership that motivates the PNP `SliceIR` input substitution.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — canonical candidate/perimeter identity fields.

## Acceptance Summary

- Positive: `AC-1` through `AC-5` prove the versioned input contract, full active-region identity, region-bound candidates, non-broadcast injection, and inactive-region omission.
- Negative: `AC-N1` through `AC-N2` prove duplicate and malformed identity rejection.
- Cross-packet impact: packet 179 consumes the new region view and full identity; packet 180 consumes variant-aware resolved seams and the new wall identity.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p seam-planner-default --test seam_region_aware_planning_tdd` | Per-region SliceIR candidate and inactive-region behavior | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test contract -- dispatch_prepass_harvest_tdd` | WIT harvest, duplicate identity, and commit contract | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test contract -- prepass_seam_planning_commits_seam_plan_ir` | Real prepass guest-to-blackboard path | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask build-guests --check` | Guest artifact freshness after WIT/SDK/module edits | FACT pass/fail |
| `cargo check --workspace --all-targets` | Struct/WIT blast-radius compilation | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate | FACT pass/fail |

## Step Completion Expectations

The full active-region key is the identity invariant shared by every step. No
step may temporarily use contour ordinals, object-only keys, or empty variant
chains as a compatibility shortcut.

## Context Discipline Notes

WIT and IR changes require delegated cross-crate trait/macro tracing and a guest
freshness check. `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md` must
be read through bounded ranges or delegated summaries only.