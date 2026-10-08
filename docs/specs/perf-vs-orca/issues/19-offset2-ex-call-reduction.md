# offset2_ex call reduction

Type: task
Status: open
Assignee: agpen — reopened by human decision 2026-10-08; below-fold park lifted for this candidate's take (no experiment started by this record)
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

## Comments

### Resumption claim — 2026-10-05

Selected as the first numbered open, unblocked, unclaimed ticket on resumption.
The below-fold park in [Gap budget per cell](12-gap-budget-per-cell.md) still
applies; this claim does not silently reopen candidate implementation or timing.
The human must confirm whether to reopen this lead after the structural takes,
or leave it parked and redirect the route. No experiment, timing campaign,
adoption retry, default switch, or commit has been started by this claim.

After the human's continuation request, the session followed the recommended
scoping-only route: leave this candidate parked and sharpen the signed-distance
lead from the map's fog. Claim released without resolving this ticket or
authorizing its implementation. See [Signed-distance investigation scope](48-signed-distance-investigation-scope.md).

### Resumption route claim — 2026-10-06

Claimed as the first numbered open, unblocked, unclaimed child after
[Prepared winding-coordinate cache: exactness + standing paired A/B](51-prepared-winding-coordinate-cache-experiment.md)
closed with human-approved DROP. The below-fold park remains binding: this
claim is for the live route decision only, not permission to implement or time
this candidate. No cache retry, adoption retry, default change or commit is
authorized by resumption. Await the human's choice between reopening this
lead and scoping a remaining structural investigation from the map's fog.

The human selected **Scope union/simplify**. Leave this candidate parked and
release the routing claim without resolving it or authorizing experiments.
Continue with [Tree-planner union/simplify investigation scope](52-tree-planner-union-simplify-investigation-scope.md).

### Resumption route claim — 2026-10-07

Claimed as the first numbered open, unblocked, unclaimed child after
[Scanbeam TopX projection-cache experiment](57-scanbeam-topx-cache-experiment.md)
closed with a human-accepted DROP. The below-fold park in
[Gap budget per cell](12-gap-budget-per-cell.md) remains binding.

**Route decision (2026-10-07):** the human selected the small non-timing
ticket family. The take is
[mesh_ops fixture limits](29-mesh-ops-fixture-limits.md) (first in number
order; 30/31/34, the family's other open tickets, were resolved later the
same day). This candidate stays parked and
the routing claim is released; no experiment, timing campaign, adoption retry,
default change or commit is authorized by this claim.

### Reopen — 2026-10-08

**Human decision: reopen this lead.** With the small non-timing family complete
([Config padding disclosure boundary](34-config-padding-disclosure-boundary.md)
closed the last take), the human confirmed the reopen the 2026-10-05 and
2026-10-06 claims awaited. The below-fold park in
[Gap budget per cell](12-gap-budget-per-cell.md) is lifted **for this
candidate's take only**; the routing claim is released into the take.

The take evaluates the measured candidate shapes — redundant shrink/expand
pairs around `split_top_surfaces`, result reuse across the top-surface flow,
gap-fill width-band coalescing — under the standing constraints (intended
geometry preserved exactly; `opening_ex` through the same primitive if touched)
and acceptance (paired ordinary + accelerated A/B, interleaved repeats with
starvation exclusion, uninstrumented probe-free runs, isolated host/guest
snapshots; fuel alone is not a wall win). No candidate implementation,
experiment, timing campaign, adoption retry, default change or commit is
started by this record; keep/drop still returns to the human with no
auto-commit.
