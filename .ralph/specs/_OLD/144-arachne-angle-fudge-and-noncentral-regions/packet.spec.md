---
status: implemented
packet: 144-arachne-angle-fudge-and-noncentral-regions
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 144-arachne-angle-fudge-and-noncentral-regions

## Goal

Remove the temporary π-cap workaround (`pipeline.rs:334`) and the 0.1× filter-dist fudge (`pipeline.rs:272-277`) so the configured `wall_transition_angle` threads through `filter_central`, and port `filterNoncentralRegions` (`SkeletalTrapezoidation.cpp:811-862`) so non-central gaps between same/±1-bead-count central regions are promoted back to central.

## Scope Boundaries

Delete two load-bearing fudges in `pipeline.rs` (N5) that A1/A2/B left in place because they sustained the centrality-gated scheme, and add `filter_noncentral_regions` to `centrality.rs` (N6). Thread the configured `wall_transition_angle` through `filter_central`. Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `142-arachne-canonical-connectjunctions-emission` (A2) strictly — the π hack is load-bearing for A1's centrality-gated scheme until A1/A2 land; C removes it strictly after A2.
- Unblocks: nothing directly (D/E/F don't depend on C's specifics, but D's `generateLocalMaximaSingleBeads` reads the normalized centrality).
- Activation blockers: none.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** A1/A2/B are `status: implemented` (the canonical junction scheme is in place), **when** `run_arachne_pipeline` runs on a 20×4 mm rectangle and a 10 mm square with the configured `wall_transition_angle` (default 10°) threaded through `filter_central` (no π cap), **then** the N1 red tests (`arachne_parity_red_junction_bands`) still pass GREEN — removing the hack does not regress A1's junction placement.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-c-ac1.log`
- **AC-2. Given** a dumbbell-shaped polygon (two 3 mm-wide pads joined by a 0.35 mm neck), **when** `run_arachne_pipeline` runs, **then** the output contains a single stitched inset-0 ring pair (not four fragments) — `filterNoncentralRegions` promotes the neck's non-central gap back to central so canonical produces one continuous region.
  | `cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions --nocapture 2>&1 | tee target/test-output-c-ac2.log`

## Negative Test Cases

- **AC-N1. Given** the π hack (`pipeline.rs:334`) and the 0.1× filter-dist fudge (`pipeline.rs:272-277`) are deleted, **when** `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs` runs, **then** it returns no match (exit 1) — the workaround is gone, not merely commented out.
  | `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-c-gate.log`

## Authoritative Docs

- `docs/15_config_keys_reference.md` — §"Arachne beading strategy stack" (lines ~479-521); `wall_transition_angle` (10.0°), `wall_transition_filter_deviation`. Read directly.
- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines); 0.4 mm = 4000 units (the hardcoded `filterNoncentralRegions` distance).
- `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — read full; addendum target.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-144-ANGLE-FUDGE-NONCENTRAL` documenting the N5+N6 fix, with an addendum on `D-141-JUNCTION-BANDS` noting C removes the π hack A1 left in place. Supersession pattern.
  - `rg -q 'D-144-ANGLE-FUDGE-NONCENTRAL' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:811-862` — `filterNoncentralRegions` (promote non-central gaps within 0.4 mm back to central, copy bead counts).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:633` — call site (after `updateBeadCount`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:716-730` — dead `filterCentral` whisker-dissolve (self-contradictory condition — DO NOT wire).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h:78` — canonical `getTransitioningAngle` default (60°).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategyFactory.hpp:49` — `getTransitioningAngle` default (π/4).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.