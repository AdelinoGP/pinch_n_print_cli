# Accelerated residual query attribution

Type: task
Status: resolved
Blocked by: 12
Assignee: wayfinder session (ses_f29767ae7ffeMtqvHVxUlT8cio), 2026-09-25

## Question

Split the residual per-vertex query cost in accelerated mode — the hot queries
shrank only ~1.9× while total guest fuel fell 35.8%
([Accelerated-mode pair](09-accelerated-mode-pair.md)) — into three parts and
recommend which is the next optimization candidate:

1. **Wasm spatial-tree query internals** (the suspected u128 envelope math) in
   `crates/slicer-core/src/perimeter_spatial.rs`'s accelerated path.
2. **Fallback paths firing**: `bridge_arithmetic_safe` and
   `IndexState::Linear` (same file) — count firings and attribute their cost;
   these should be rare in accelerated mode and may not be.
3. **Any remaining linear scans** reachable in the accelerated build (the
   2026-09-22 attribution found the scans dominating in ordinary mode).

Attribution runs only (ADR-0055 user scopes / `--instrument-stderr`; never
wall claims). Runs use the complete accelerated snapshot recipe.

Deliverable: the measured three-way split plus a recommendation — this scopes
[Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
against accelerated mode (what remains worth avoiding once the query path and
fallbacks are accounted for). Re-rank or drop leads per
[Gap budget per cell](12-gap-budget-per-cell.md)'s budgets where they conflict.

## Answer

Resolved 2026-09-25, AFK session. Evidence:
`evidence/t18-accelerated-residual/` (`FINDINGS.md`, `summary-stage1.json`,
`stage2.json`, `summarize.py`, `probe.patch`, `probe-stage1.patch`). Fuel
attribution only (`--profile --instrument-stderr`, ADR-0055); no wall claim
anywhere. All runs used the complete `cargo xtask dist --accelerated`
snapshot (`target/dist-accelerated/developer/`, `--module-dir` its own
`modules/`), `--check` exit 0 before every capture, `module diagnose` all
external, matched configs from `evidence/matched-pair/configs/`.

**Three-way split, measured (classic supports-off; per-vertex query scopes
are 69–70% benchy / 62% base of the classic module's accelerated fuel):**

1. **Tree-query internals: small, and the u128 premise is false.** The
   suspected u128 envelope math does not exist in the hot path —
   `indexed_distance`/`indexed_inside`/`indexed_quartile` are f64
   (Y-interval envelopes, `next_up`/`next_down`); the only u128 arithmetic
   is the `bridge_arithmetic_safe` precheck, measuring **0.114 B benchy /
   0.971 B base (~0.03% of query fuel)** with its unsafe fallback **never
   firing**. rstar candidate collection (`inside_collect`+`quartile_collect`)
   is **3.9–5.5%** of the scan+collect fuel.
2. **Fallback paths: fire often on benchy, cost almost nothing.**
   `IndexState::Linear` fallbacks fired 241,599 (benchy) / 40,762 (base)
   times — mostly small bridge sets (230,920 / 33,640) below
   `RSTAR_MAX_SIZE`, plus 10,679 / 7,122 quartile; **distance fallbacks
   zero**. Their scan bodies cost **1.96% (benchy) / 0.07% (base) of query
   fuel**. Not the residual.
3. **The residual is the exact winding predicate loop after candidate
   reduction: `inside_scan` 147.13 B benchy / 2,814.51 B base, `quartile_scan`
   141.48 B / 1,004.66 B — together ~89% of base's whole query scope.** The
   trees prune correctly and cheaply (collect is 24–55× cheaper than the
   scan); what remains is O(vertices) winding evaluation per candidate
   polygon, with ~6n+4m `units_to_mm` divisions per candidate per query, and
   `indexed_inside`/`indexed_bridge` not short-circuiting where their legacy
   `.any()` oracles do. This is also where the benchy→base superlinearity
   lives: per-query `inside_scan` fuel grows 3.5× while `inside_collect`
   stays near-constant.

Arachne cross-check (same probe, `pnp-arachne-supports-off.json`): identical
shape at smaller scale — query scopes 79–81% of the arachne module,
`inside_scan`+`quartile_scan` dominating collect 4.4–4.5%. The residual's
location is not a classic-only artifact.

**Scoping [Consume-only-when-read annotations](20-consume-only-when-read-annotations.md):**
the annotation work's per-vertex query saving remains real in accelerated
mode — the queries still run per vertex and their cost is now measured as
~70% of classic's accelerated module fuel. But *within* the query path, the
next candidate is the predicate/index internals, not query avoidance:
(a) 2-D per-polygon bbox records for the inside/quartile trees (currently
X-degenerate — no X pruning by construction), (b) a bit-exact pre-converted
vertex cache for the winding predicate plus restored short-circuits,
(c) descending first-hit for the quartile max. All exact-semantics; all
unmeasured for wall (fuel→wall transfer 0–16%); none reaches a cell's
G = 6.28–26.24× alone, so ticket 12's route and the below-fold parking of
19/20 are unchanged — 18 stays a scoped attribution below the structural
tickets, and the falsified u128 suspect plus the fallback/precheck terms need
no further work.

Recipe notes for successors: probe scope accounting must sum **self** fuel
across leaves and parents and inclusive totals only at the three disjoint
public query scopes — summing `total_fuel` across nested scopes double-counts
(`summarize.py` asserts the identity; residual 0 in all runs). Stage-1 vs
stage-2 G-code byte drift (few hundred bytes on 4–21 MB) is inside the
§3.6 noise band; TYPE section sets identical, benchy `Inner wall` within the
known 244–251 instability. Probe code was fully reverted; both patches apply
cleanly to the probe-free tree.
