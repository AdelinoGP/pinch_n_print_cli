# offset2_ex call reduction

Type: task
Status: resolved (2026-10-08; take complete — standing paired A/B passes both modes, **human-approved KEEP, committed with this ticket** — see ## Answer)
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

## Answer — 2026-10-08

**KEEP — approved by the human 2026-10-08 and committed with this ticket.** The
ticket's stated call graph is stale — two of its three named shapes are dead
sites — and the surviving candidate is a fourth shape the ticket did not name:
the `classify_narrow_island` opening-emptiness short-circuit.

### Re-derived call graph at HEAD `ca6841fc`

The ticket's 720-call `min_width_top` shrink/expand does not execute on the
route job. The probe (see
[`probe-callsites.patch`](../evidence/t19-offset2-call-graph/probe-callsites.patch))
measures the site at **0 calls** on the matched classic supports-off job and
**13 calls / 0.037% of module fuel** with the gate switched on. The three live
sites are gap-fill `opened_max` (49.94% of `offset2_ex`), narrow-island
`opening_ex` (40.84%), and gap-fill `opened_min` (9.22%); they close the
instrumented total within 0.00007%. Candidate shapes: shrink/expand and
top-surface reuse are dead; gap-fill coalescing is not output-preserving (the
two band ops run at different distances, as does canonical `process_classic`'s
gap block). Full re-derivation:
[`FINDINGS.md`](../evidence/t19-offset2-call-graph/FINDINGS.md).

### Candidate and exactness

`opening_is_empty` (`crates/slicer-core/src/polygon_ops.rs`) answers
`opening_ex(..).is_empty()` from the shared single erosion pass
(`offset2_ex_first_pass`), which the two-pass entry point now calls too — the
same primitive, not a re-parameterised copy. For `delta_mm > 0` the identity
`opening(X, d) = ∅ ⟺ erode(X, d) = ∅` is exact (a positive offset maps
`∅ → ∅` and contains its input). For `delta_mm <= 0` or NaN the predicate
delegates to the unchanged two-pass opening; two measured boundaries make that
load-bearing — `d == 0` clones a degenerate ring that the opening's union
cleanup drops (reachable: `smaller_perimeter_threshold_mm` min is 0.0), and
`d < 0` has opposite-sign passes. `classify_narrow_island` calls it with its old
arguments exactly. Exactness: 4 tests against the unchanged two-pass oracle and
independently extracted erosion emptiness (`crates/slicer-core/tests/opening_emptiness_tdd.rs`,
14-fixture mixed-polarity family); a live-wiring guest test
(`narrow_island_classification_selects_outer_wall_width_in_both_polarities`)
that goes red under a mutated predicate (checked, reverted). `offset2_ex` calls
at the site drop 720 → 480 on the route job; attributed module fuel
750.03 B → 732.43 B (≈ −2.35%).

### Standing paired A/B — passes both modes

Matched classic supports-off benchy, 12 threads, one warmup per arm, six
alternating AB/BA pairs per mode, external-only probe-free runs, arm-isolated
snapshots differing only in `modules/classic-perimeters/classic-perimeters.wasm`.

| mode | baseline wall | candidate wall | wall Δ | baseline CPU | candidate CPU | CPU Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary | 16.980 s | 16.870 s | −0.65% | 120.086 s | 117.586 s | −2.08% |
| accelerated | 15.708 s | 15.478 s | −1.47% | 99.500 s | 98.469 s | −1.04% |

No starvation exclusions; both modes clear the protocol's median keep clause
with agreeing wall/CPU. All 32 slices byte-identical to the preserved reference
`34886ccb…` (4,332,737 bytes, `status=ok`, `degraded=false`, zero fatals/
non-fatals, probe-free). The effect is small and cannot close the 6.28–26.24x
destination gap, but the standing gate is directional and it passes. Evidence:
[`RESULTS.md`](../evidence/t19-offset2-call-graph/RESULTS.md),
[`protocol.json`](../evidence/t19-offset2-call-graph/protocol.json),
[`summary.json`](../evidence/t19-offset2-call-graph/summary.json),
[`verify-t19.py`](../evidence/t19-offset2-call-graph/verify-t19.py) (standalone,
no raw tree; five negative controls reject drift, CPU-only wins, starvation
pair loss, fabricated references and non-isolated arms).

### Provenance note

The measured source fails `clippy::neg_cmp_op_on_partial_ord` on its
deliberate `!(delta_mm > 0.0)` NaN-inclusive test. Measured: **any text change
to `polygon_ops.rs` shifts the accelerated guest's artifact hash** (its
`.llvm.<hash>` symbol suffixes in the name section), even a comment insertion;
the ordinary artifact is text-insensitive. A function-level `#[allow]`
reproduced the timed ordinary artifact but not the timed accelerated one; a
positive-first rewrite changed both. The shipped form is therefore the measured
source byte-for-byte plus a **manifest-level** lint allow
(`Cargo.toml [workspace.lints.clippy]`), which is clippy-clean and reproduces
**both** timed artifacts under forced rebuilds. Verified by experiment
(`target/t19-ab/provenance2.py`, gitignored).

### Gates

`cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo xtask check-literals`, `cargo xtask check-test-quality --report` (0
findings) and both mode freshness checks pass. All 41 narrow tests pass (8 core
+ 33 guest). KEEP approved by the human 2026-10-08 and committed with this
ticket; no adoption retry, default-mode switch or Orca-scoreboard claim is
part of the keep.
