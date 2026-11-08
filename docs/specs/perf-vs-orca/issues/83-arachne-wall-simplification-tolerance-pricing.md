# Arachne wall-simplification tolerance pricing

Type: task
Label: wayfinder:task
Status: resolved (2026-10-24; measurement-only pricing take — DROP, below the
map's pre-declared 0.80 % wall floor; no candidate, no parity port, no
production edit; D-168 retained with a measured magnitude — see ## Answer)
Assignee: 2026-10-23 OpenCode wayfinder session — human said "continue the perf
vs orca map" (no ticket named); the frontier re-derivation found ticket 83 as the
only open child
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

## Pre-declared pricing gate (declared 2026-10-23, before any run)

Step 1 decides whether the parity port is worth scoping. The gate is the map's
established below-floor bound ([ticket 68](68-classic-perimeters-host-round-trips.md)
/ [ticket 69](69-clipper-vendoring-optimization.md)): a reduction whose ceiling
is below **0.80 % of run wall** is below any gate this map has applied, so no
parity work is justified.

- **Arms — same binary, same modules, config-only.** `default` = the committed
  matched Arachne config (both keys unset → code fallback 0.05 / 0.005 mm).
  `canonical` = the same JSON plus `wall_maximum_resolution = 0.5` and
  `wall_maximum_deviation = 0.025` (Orca's resolved values, ticket 82).
- **Cells:** the four Arachne cells ({benchy, base} × {supports off, on}),
  ordinary mode, 12 rayon threads.
- **Runs:** two alternating pairs per cell (`A B` then `B A`),
  `--profile --profile-verbose --instrument-stderr` so one run yields fuel,
  module brackets and phase walls, plus external process wall/CPU.
  Instrumented runs are attribution-only and are **never** a wall claim; their
  A/B delta is a direction check.
- **Metrics:** `com.core.arachne-perimeters` guest fuel (instructions) and
  accumulated elapsed; total per-layer module accumulated elapsed; `per_layer`
  phase wall; process wall and CPU; output bytes, SHA-256, TYPE counts,
  degraded / fatal / non-fatal.
- **Ceiling (measured, not assumed):**
  `occupancy-normalized wall ceiling = Δmodule_accumulated_elapsed /
  measured_phase_parallelism`, where
  `measured_phase_parallelism = total_per_layer_module_accumulated_elapsed /
  per_layer_phase_wall`, both measured per run. The idealized serial ceiling
  (`Δmodule_accumulated_elapsed` itself) is reported as an upper bound.
- **Material iff** in at least one Arachne cell the occupancy-normalized
  ceiling is **≥ 0.80 % of that cell's measured process wall** and the fuel
  delta is outside the arm's repeat spread. **Otherwise DROP** — step 1 stops,
  the deviation keeps a measured magnitude, and no parity port is scoped.
- **No candidate in this take:** no production edit, no default change, no
  code KEEP/DROP. If material, step 2's parity condition and campaign are a
  separate take.

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

## Answer

**Resolved 2026-10-24 (measurement-only pricing take — DROP, below the map's
pre-declared floor).** Matching canonical's Arachne wall-simplification
tolerances is a real *work* reducer and a real *G-code size* reducer, but it
cannot move cell wall by the map's floor. Step 1's stop rule fires: no parity
port is scoped, `D-168` is retained with a measured magnitude, and there is no
candidate, no default change and no production edit.

### What was measured

One binary and one module tree (HEAD `40eefc5a557b3d5ad81a9a4757bbf4ec59e36735`,
`cargo build --release --bin pnp_cli`, guests rebuilt and freshness exit 0,
24/24 modules `external`), two config-only arms:

- `default` = the committed matched Arachne job (both keys unset → code fallback
  **0.05 / 0.005 mm**);
- `canonical` = the same job plus `wall_maximum_resolution = 0.5` and
  `wall_maximum_deviation = 0.025` (Orca's resolved values, ticket 82).

The emitted G-code appendix discloses the applied values in both arms
(`; wall_maximum_resolution = 0.05` / `0.5`), so the override is confirmed at
the resolver. Instrumented runs: 2 alternating pairs per cell,
`--profile --profile-verbose --instrument-stderr`, 12 threads, attribution only.
Direct wall check: 6 alternating uninstrumented pairs on the two borderline
cells.

### The price

| cell | arachne fuel | fuel Δ | module accum | module share of wall | occupancy-normalized ceiling | idealized serial ceiling | fuel-proportional bound |
|---|---:|---:|---:|---:|---:|---:|---:|
| benchy arachne-off | 10.183 → 9.087 B | **−10.76 %** | 3.59 → 3.46 s | 6.87 % | **0.261 %** | 1.329 % | 0.739 % |
| benchy arachne-on | 10.183 → 9.087 B | **−10.76 %** | 3.52 → 3.33 s | 3.39 % | **0.190 %** | 1.240 % | 0.365 % |
| base arachne-off | 127.428 → 106.766 B | **−16.21 %** | 49.47 → 49.09 s | 5.20 % | **0.039 %** | 0.403 % | 0.844 % |
| base arachne-on | 127.428 → 106.766 B | **−16.21 %** | 64.89 → 56.54 s | 2.92 % | **0.376 %** | 4.023 % | 0.474 % |

The fuel delta is deterministic (repeat spread ≤ 0.0026 %) and lies entirely in
module *self* fuel: the host scope `polygon_ops::offset` is unchanged (313.2 M
on benchy, 2.43 B on base). Whole-job guest fuel falls −3.41 % / −1.41 %
(benchy off/on) and −11.56 % / −3.08 % (base off/on). The accumulated-elapsed
column is load-noisy (its sample ranges overlap on three cells; base-off's
median even moves the wrong way), so fuel is the work signal and elapsed is the
attribution signal.

**All four occupancy-normalized ceilings are below the pre-declared 0.80 %
floor** (0.039–0.376 %). The most generous bound — assuming every guest
instruction costs the same wall — reaches 0.844 % on base-off only, and the
measured elapsed there moved +0.8 % the wrong way. The idealized serial bound
assumes no parallelism and is not attainable on this phase.

The ticket's step-1 wording keys the stop on the module-fuel delta; here that
delta is large (−10.8 / −16.2 %), so the wall question was priced explicitly
under the pre-declaration above. The floor is the map's established bound
applied to **run wall** (tickets 68/69), and the measured price does not reach
it in any cell — which is the DROP condition.

### Direct wall check (uninstrumented)

| cell | wall medians default → canonical | paired wall deltas | median wall | CPU median |
|---|---:|---|---:|---:|
| base arachne-off | 87.68 → 88.30 s | +0.58, −2.33, −0.23, +3.68, −2.42, +1.68 % | **+0.17 %** | +0.09 % |
| benchy arachne-on | 13.67 → 14.08 s | +3.16, +3.21, +0.14, +5.19, −4.68, +13.42 % | **+3.18 %** | +0.17 % |

No win in either cell. Benchy-on's *instrumented* −2.9 % was load drift: the
clean A/B is nominally slower on wall with flat CPU, i.e. scheduling noise on a
13.7 s cell.

### Output delta

G-code bytes fall **−7.93 %** (benchy-off), **−3.84 %** (benchy-on), **−8.23 %**
(base-off) and **−4.05 %** (base-on), with **identical TYPE counts** in every
cell. Every run is `ok`, not degraded, 0 fatal / 0 non-fatal, and each arm is
byte-deterministic across its repeats. The default arm reproduces the t81
post-port reference for base arachne-off (`01a1434c…`), so the price is measured
at the geometry the map currently scores.

### Verdict

- **DROP.** The tolerance match is real work and real bytes, but the module is
  only 2.9–6.9 % of cell wall and runs at 5.1–10.7x phase parallelism while the
  serial prepass dominates. It cannot recover the map's floor, so no parity port
  is justified by this destination.
- **`D-168` is retained with this measured magnitude** (row updated in
  `docs/DEVIATION_LOG.md`). A future decision about G-code size is separate and
  follows the t36 precedent (bytes are not this destination's verdict).
- **Observation, not a ticket:** `tree_support_branch_distance` (Orca 5 mm vs
  PnP 1 mm, supports-on only) is the other ticket-82 "PnP does more work"
  geometry key, magnitude unmeasured. It is parity-scoped work and needs the
  human's route; it is not implicitly authorised by this take.

Evidence: [`evidence/t83-arachne-tolerance-price/`](../evidence/t83-arachne-tolerance-price/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only with 5/5 negative controls
rejected; `summary.json`, `wall-pairs.json`, 16 reduced captures).

## Non-goals

No candidate, no timing campaign, no production edit until step 1 says the
price is worth paying. No change to the default without a human ruling: the
canonical value and PnP's tighter fallback are both defensible, and this ticket
prices the difference rather than asserting one.

## Evidence standard

Per the map's evidence policy: the measurement lives under
`docs/specs/perf-vs-orca/evidence/t83-arachne-tolerance-price/`, checkout-only
verifiable, raw captures regenerable and never committed.
