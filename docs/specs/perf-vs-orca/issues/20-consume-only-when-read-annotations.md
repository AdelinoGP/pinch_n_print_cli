# Consume-only-when-read annotations

Type: task
Status: open
Blocked by: 12, 18, 19

## Question

Design and measure consume-only-when-read per-vertex annotations in
`expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`) —
compute each per-vertex quantity (`overhang_quartile`,
`signed_distance_to_boundary`, `is_bridge`) only when a downstream consumer
actually reads it. This is lead 3 of the
[emit_walls premise falsified](08-emit-walls-premise-falsified.md) ranking,
which the mode-comparative addendum **raised in weight**: with acceleration
necessary-but-not-sufficient, avoiding work the consumer never reads compounds
with whatever the query path eventually costs.

Scoping comes first: [Accelerated residual query
attribution](18-accelerated-residual-query-attribution.md) determines what is
still worth avoiding once accelerated queries and fallback suppression are
accounted for — the candidate is that remainder, not the ordinary-mode 69.8%.

Constraints: annotations must be value-identical wherever they are read
(output preserved exactly); the change is guest-side in the classic perimeter
path (and `PerimeterSpatialContext::is_bridge`'s scan belongs in the same
look-before-you-read design if the split says so).

Acceptance: paired ordinary + accelerated A/B per the standing metric, then
keep/drop to the human. Timing chain position 5.
