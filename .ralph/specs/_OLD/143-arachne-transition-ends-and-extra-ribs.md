---
status: implemented
packet: 143-arachne-transition-ends-and-extra-ribs
task_ids:
  - none
---

# 143-arachne-transition-ends-and-extra-ribs

## Goal

Port canonical transition-end machinery (N3 — `filterTransitionMids` + `generateAllTransitionEnds` + `applyTransitions` at ends + fractional `transition_ratio` + beading interpolation at emission) and `generateExtraRibs` (N8), so bead-count changes ramp over the configured `wall_transition_length` instead of snapping at a single point, and long spine edges get intermediate width-sampling points at nonlinear-strategy breakpoints.

## Problem Statement

PNP's `apply_transitions` (`propagation.rs:646-740`) converts every
`TransitionMiddle` directly into a single `insert_node` split at the MID
position, with `transition_ratio` hard-set to `0.0` everywhere (`:714`, `:723`).
There is no `filterTransitionMids`, no `generateTransitionEnds`/`generateAllTransitionEnds`,
and `generate_junctions` calls `strategy.compute(2R, bead_count)` directly with
an integer bead count — no interpolation. Canonical
(`SkeletalTrapezoidation.cpp:881-915` `generateTransitioningRibs`) instead runs
`generateTransitionMids` → `filterTransitionMids` (`:1007-1076`) →
`generateAllTransitionEnds` (`:1247-1403`) → `applyTransitions` (`:1487-1543`):
each mid spawns a lower end walking backward on `edge.twin` and an upper end
walking forward, spread over `beading_strategy.getTransitioningLength(lower_bead_count)`
around the anchor `getTransitionAnchorPos`; ends recursively travel onto
successor edges, assigning every traversed node a fractional `transition_ratio`;
`applyTransitions` inserts nodes at END positions with `bead_count = lower` or
`lower + 1` per `is_lower_end` (`:1525-1526`); `generateSegments` (`:1712-1721`)
interpolates the beading of any node with nonzero `transition_ratio` between
`compute(thickness, bead_count)` and `compute(thickness, bead_count + 1)`. Net
effect of the gap: PNP snaps the bead count at a single point (abrupt width step
at every transition — visible bumps), and keeps every raw transition mid (extra
churn on noisy geometry). Separately, `generateExtraRibs` (`:1579-1633`) is
absent — long spine edges get no intermediate width-sampling points at
nonlinear-strategy breakpoints, so widths along long spine edges are linearly
interpolated across nonlinear-strategy breakpoints (visible width error on
wide regions). The `BeadingStrategy` trait (`beading/mod.rs:64-108`) lacks
`getTransitioningLength` / `getTransitionAnchorPos` / `getNonlinearThicknesses`
entirely — N3 (and N8) require a trait extension. (Note: `wall_transition_angle`
already exists on the trait at `mod.rs:93`; B must not add a duplicate.)

This packet supersedes `D-112-PROPAGATION-ADAPT` for the transition machinery;
A1/A2's junction generation and emission remain canonical and untouched.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`BeadingStrategy` trait extension is `slicer-core`-internal.** The trait is not exposed across the WIT host boundary (the host boundary marshals `Vec<ExtrusionLine>`, not `BeadingStrategy` trait objects); no WIT/IR schema change. Confirmed during grilling: `beading/` is entirely `slicer-core`-internal.
- Packet-specific constraint: **`wall_transition_angle` already exists** on the trait at `mod.rs:93` (with `DistributedBeadingStrategy` override at `distributed.rs:195` and the 4 decorators delegating). B does NOT add a duplicate; disambiguate during grilling.
- Packet-specific constraint: **`EdgeType::TRANSITION_END` is a PNP invention, currently unused.** B prefers delete unless the rewrite needs an edge marker for the new `TransitionEnd` type.
- Packet-specific constraint: **WASM staleness does NOT apply** — B's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `BeadingStrategy` trait is `slicer-core`-internal; not exposed across the WIT boundary. `TransitionEnd` is a new `skeletal_trapezoidation`-internal type (not in `slicer-ir`).
- WIT boundary considerations: **none**. No WIT/IR schema change. The host boundary marshals `Vec<ExtrusionLine>`, not `BeadingStrategy` trait objects.
- Determinism: B's rewrite preserves determinism (the recursive travel is index-ordered; the fractional `transition_ratio` is a deterministic function of the edge geometry + `get_transitioning_length`; `filterTransitionMids`'s dissolve is deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- `get_transitioning_length` returns `self.default_transition_length` from `DistributedBeadingStrategy` (line 43, `#[allow(dead_code)]` removed); the 4 decorators delegate to `self.parent`.
- `wall_transition_angle` already exists (`mod.rs:93`); B does NOT add a duplicate.
- `EdgeType::TRANSITION_END` is deleted unless the rewrite needs an edge marker.
- N3 red-test call sites are updated (assertions untouched per grilling decision).
- B keeps N1, N2, N4 red tests GREEN (gated).
- B does NOT remove the π hack or the 0.1× filter-dist fudge (Packet C's scope).
- Beading-stack audit is mandatory (B's author confirms the 5 concrete strategies' readiness before implementation).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- `transition_ratio` is fractional (strictly between 0 and 1) on traversed nodes, not `0.0`.

## Risks and Tradeoffs

- **The `generateAllTransitionEnds` recursive travel is the most complex new code in B.** Risk is contained by the N3 red tests (the fractional-ratio observable) + the `propagation` regression suite.
- **The `BeadingStrategy` trait extension could break the 5 concrete strategies if the default impls are wrong.** Mitigated by AC-N1 (`cargo check --all-targets`) + the grilling-confirmed fact that `DistributedBeadingStrategy` already stores `default_transition_length` and the 4 decorators already follow the `self.parent` delegation pattern for `wall_transition_angle`.
- **Beading-stack audit gap.** `crates/slicer-core/src/beading/` was out of the audit's read scope. B's author must confirm readiness; if a strategy needs a non-delegating override, that's a discovery during implementation, not a blocker.
- **`EdgeType::TRANSITION_END` deletion could ripple if downstream code references it.** The audit says it's currently unused; B confirms via grep before deleting.
