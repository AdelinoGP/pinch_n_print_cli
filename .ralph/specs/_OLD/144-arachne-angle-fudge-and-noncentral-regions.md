---
status: implemented
packet: 144-arachne-angle-fudge-and-noncentral-regions
task_ids:
  - none
---

# 144-arachne-angle-fudge-and-noncentral-regions

## Goal

Remove the temporary π-cap workaround (`pipeline.rs:334`) and the 0.1× filter-dist fudge (`pipeline.rs:272-277`) so the configured `wall_transition_angle` threads through `filter_central`, and port `filterNoncentralRegions` (`SkeletalTrapezoidation.cpp:811-862`) so non-central gaps between same/±1-bead-count central regions are promoted back to central.

## Problem Statement

Packet 141 (A1) left the π-cap workaround (`pipeline.rs:334`,
`effective_transitioning_angle_rad = std::f64::consts::PI`, self-described
"TEMPORARY") and the 0.1× filter-dist fudge (`pipeline.rs:272-277`, scaling
`transition_filter_dist` by 0.1) in place because they were load-bearing for
the centrality-gated junction scheme — A1's rewrite replaced junction
generation but kept the centrality pipeline that the π hack sustains. The audit
(N5) flags this: canonical `updateIsCentral` uses
`beading_strategy.getTransitioningAngle()` (defaults π/4 per
`BeadingStrategyFactory.hpp:49` / 60° per `BeadingStrategy.hpp:78`, ultimately
sourced from `wall_transition_angle` ~10°); with the canonical angle, a
square's diagonal spokes (`dR/dD = sin 45° ≈ 0.707`) are **non-central**; PNP's
`cap = sin(π/2) = 1` marks every non-degenerate spine edge central. This is
required to keep N1's central-gated junction scheme producing output at all;
once A1 fixed junction generation (junctions no longer centrality-gated), the
hack must be removed or centrality/transition placement will be wrong in the
opposite direction. Separately, `filterNoncentralRegions` (N6) is absent
(`SkeletalTrapezoidation.cpp:811-862`, called unconditionally at `:633` after
`updateBeadCount`): it promotes non-central gaps between same/±1-bead-count
central regions (within a hardcoded 0.4 mm) back to central and copies bead
counts across. Without it, central regions fragment across shallow pinch
points, producing separate domains (extra seams / short lines) where canonical
produces one continuous region. This packet removes both fudges (N5) and ports
`filterNoncentralRegions` (N6), strictly after A2 lands (the fudges are
load-bearing until A1/A2's canonical scheme is in place).

This packet supersedes `D-141-JUNCTION-BANDS` for the centrality-parameter
layer only; A1's junction geometry and A2's emission remain canonical and
untouched. C does not change the `BeadingStrategy` trait (B owns the trait
extension) — C only threads the already-existing `wall_transition_angle` (on
the trait at `beading/mod.rs:93`) through `filter_central`.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **C must NOT extend the `BeadingStrategy` trait.** B (`143`) owns the trait extension. C only threads the **already-existing** `wall_transition_angle` (on the trait at `beading/mod.rs:93`, threaded via `BeadingFactoryParams` at `factory.rs:92,157,192`) through `filter_central`. The strategy's `wall_transition_angle()` is the source of truth, not a hardcoded π.
- Packet-specific constraint: **C must NOT wire the whisker-dissolve `filterCentral`.** It is dead code upstream (`SkeletalTrapezoidation.cpp:716-730`, self-contradictory condition). PNP's un-wired helpers (`centrality.rs:263-389`) correctly mirror this dead code; leave them. The audit explicitly flags this as a gotcha — do not "fix" PNP by wiring the dissolve in.
- Packet-specific constraint: **C's removal of the π hack changes runtime behavior for every polygon.** The configured 10° is now the actual gate, not π. This is the intended behavior change (canonical parity), but it shifts centrality classification for many fixtures. The centrality fixture re-baseline records the drift; the commit message surfaces this as a scope decision.
- Packet-specific constraint: **WASM staleness does NOT apply** — C's change surface is `slicer-core`-internal (`arachne/pipeline.rs`, `skeletal_trapezoidation/centrality.rs`); no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. C's surface is `slicer-core`-internal; no WIT/IR change. C does NOT extend the `BeadingStrategy` trait (B owns the trait extension); C only threads the already-existing `wall_transition_angle`.
- WIT boundary considerations: **none**. No WIT/IR schema change. The host boundary marshals `Vec<ExtrusionLine>`, not centrality parameters.
- Determinism: C's changes preserve determinism (the configured angle is a fixed config value; `filter_noncentral_regions` is a deterministic graph walk with index-ordered tiebreaks).

## Locked Assumptions and Invariants

- `wall_transition_angle` already exists on the `BeadingStrategy` trait at `beading/mod.rs:93` and is threaded via `BeadingFactoryParams` at `factory.rs:92,157,192`. C does NOT add a duplicate; C only changes the `filter_central` call site from a hardcoded π to `strategy.wall_transition_angle()` (or `beading_params.wall_transition_angle`).
- The 0.1× filter-dist fudge is deleted entirely; `to_centrality_params` passes `params.transition_filter_dist * UNITS_PER_MM` directly (no `* 0.1`).
- `filterNoncentralRegions`'s 0.4 mm distance is in slicer units (4000 units; 1 unit = 100 nm per `docs/08_coordinate_system.md`).
- C must NOT wire the whisker-dissolve `filterCentral` (dead code upstream).
- C keeps N1, N2, N3, N4 red tests GREEN (gated).
- C's removal of the π hack changes runtime behavior for every polygon (configured 10° is now the gate, not π); the centrality fixture re-baseline records the drift.
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- `filter_noncentral_regions` is called unconditionally after `assign_bead_counts` in `pipeline.rs`, mirroring `:633`'s "after `updateBeadCount`" ordering.

## Risks and Tradeoffs

- **Removing the π hack changes centrality for every polygon.** With the configured 10°, a square's diagonal spokes (`dR/dD = sin 45° ≈ 0.707`) become non-central (canonical); this is the intended behavior change, but it could surface latent bugs in A1/A2's junction placement that the π hack masked. The N1 red tests gate this (AC-1 must stay green).
- **`filterNoncentralRegions` port risk.** The 0.4 mm hardcoded distance and the same/±1-bead-count condition must be exact; a mis-port could over-promote (fragmenting regions that should stay separate) or under-promote (leaving the fragmentation N6 flags). The dumbbell test (AC-2) is the oracle.
- **Centrality fixture re-baseline may mask regressions.** The self-capture pattern locks in *this* implementation's behavior, not OrcaSlicer ground truth. The N1 red tests + the dumbbell test are the real parity oracles.
- **Bisect across A2→C boundary.** Between A2 and C, the π hack is still in place; C's commit message must record the boundary and the behavior change (configured angle now active).
