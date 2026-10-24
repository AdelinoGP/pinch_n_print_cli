# Arachne wall-simplification tolerance pricing

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Fairness-key parity audit](82-fairness-key-parity-audit.md)

## Question

How much slice wall does matching canonical's Arachne wall-simplification
tolerances recover on the four Arachne cells — and does it clear a pre-declared
gate?

## Measured basis

[Ticket 82](82-fairness-key-parity-audit.md) read the matched Orca job's
resolved-config appendix in all eight cells and compared it against PnP's
counterparts. The Arachne tolerance pair is the **only** unmatched geometry key
where PnP does materially more work than the matched job:

| key | Orca resolves | PnP resolves | effect |
|---|---|---|---|
| `wall_maximum_resolution` | 0.5 mm | 0.05 mm | PnP keeps 10x more Arachne wall detail |
| `wall_maximum_deviation` | 0.025 mm | 0.005 mm | PnP keeps 5x more Arachne wall detail |

Both are wired directly into `ArachneParams.smallest_line_segment_squared` /
`allowed_error_distance_squared`, i.e. they change how aggressively Arachne's
wall toolpaths are simplified before beading, rendering and emission. They are
already a logged deviation (`D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`;
re-derive the row from `docs/DEVIATION_LOG.md`), which states the change "moves
every arachne fixture, so it needs its own predicted-movement re-record" — that
re-record has not been done and the wall cost has never been measured.

**Nothing here is timed.** The magnitude is unmeasured, and the map's `~1.5x`
park does not release on unmeasured leads.

## Why this is not an output-preserving optimization

Changing these two values changes Arachne geometry by construction: fewer
retained wall points means different beading input, so the byte-identity proof
regime cannot apply. The gate therefore has to be the ticket-81 shape —
canonical parity (the simplified toolpaths must match what canonical produces at
the same tolerance), stage confinement on typed IR, per-cell determinism, and a
fresh reference set — plus a paired ordinary + accelerated wall/CPU campaign.

## Scope

1. **Price before porting.** On the four Arachne cells, slice with
   `wall_maximum_resolution = 0.5` / `wall_maximum_deviation = 0.025` and measure
   `com.core.arachne-perimeters` module fuel and stage wall against the current
   defaults, with the output delta recorded (bytes, TYPE counts, degraded flag).
   If the module-fuel delta is below the map's measurement floor, stop and
   record DROP — no parity work is justified.
2. **Only if the price is material:** define the parity condition (canonical
   `WallToolPaths::simplifyToolPaths` at the same squared tolerances), then take
   the standard candidate route — proof gate, confinement, per-cell references,
   paired campaign, human KEEP/DROP.
3. **Disclose** the deviation's closure (or its retention) in
   `docs/DEVIATION_LOG.md`.

## Non-goals

No candidate, no timing campaign, no production edit until step 1 says the
price is worth paying. No change to the default without a human ruling: the
canonical value and PnP's tighter fallback are both defensible, and this ticket
prices the difference rather than asserting one.

## Evidence standard

Per the map's evidence policy: the measurement lives under
`docs/specs/perf-vs-orca/evidence/t83-arachne-tolerance-price/`, checkout-only
verifiable, raw captures regenerable and never committed.
