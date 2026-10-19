# offset2_ex call reduction

Type: task
Status: open
Blocked by: 12, 17

## Question

Reduce `polygon_ops::offset2_ex` (`crates/slicer-core/src/polygon_ops.rs`)
call volume in `com.core.classic-perimeters` — measured 960 calls: 720 from the
`only_one_wall_top` `min_width_top` shrink/expand after `split_top_surfaces`,
240 from the gap-fill width band. Share: 23.7% of classic fuel in accelerated
mode (bit-identical across modes — pure guest clipper), 53.7% on calicat
([emit_walls premise falsified](08-emit-walls-premise-falsified.md),
[Accelerated-mode pair](09-accelerated-mode-pair.md)) — the second-ranked lead
and relatively rising.

Candidate shapes to evaluate from the measured call graph: eliminating
redundant shrink/expand pairs around `split_top_surfaces`, reusing results
across the top-surface flow, and coalescing the gap-fill width-band calls. The
serial inset loop (`emit_walls`) is explicitly **not** the target (each inset
consumes the previous result — ADR-0049 keeps dependent operations singular).

Constraints: intended geometry preserved exactly (miter limit and mm
parameters); `opening_ex` must route through the same primitive if touched.

Acceptance: paired ordinary + accelerated A/B, interleaved repeats with
starvation exclusion, uninstrumented probe-free runs, isolated host/guest
snapshots. Fuel alone is not a wall win — gate before keeping. Keep/drop
recommendation to the human; no auto-commit. Timing chain position 4.
