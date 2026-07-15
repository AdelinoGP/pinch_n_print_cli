# ADR-0038 — Visual-Debug Skill Pairs With `debug-pipeline`, Without Replacing It

## Status

Accepted.

## Context

`debug-pipeline` provides low-cost DAG, manifest, and timing diagnosis through
`pnp_cli dag`, `pnp_cli module diagnose`, and `slice --instrument-stderr`.
Visual PNGs answer a different question: where in the geometry pipeline a
human-visible defect first appears. Neither source of evidence subsumes the
other.

## Decision

Add an independent visual-debug skill and guide. It is documented beside
`debug-pipeline` and uses `pnp_cli visual-debug`, but it does not require an
agent to run timing or DAG diagnosis first. Agents choose the cheapest surface
that answers the reported problem.

## Consequences

- Geometry investigations can start directly from visual evidence.
- Performance, module wiring, and manifest questions remain owned by
  `debug-pipeline`.
- Both guides must cross-link and state their distinct evidence boundaries.

## Alternatives Considered

- **Replace `debug-pipeline`.** Rejected: images do not expose static DAG
  edges, validation diagnostics, or runtime timing.
- **Require `debug-pipeline` before visual debugging.** Rejected: this adds
  unnecessary work when a reported defect is plainly geometric.
