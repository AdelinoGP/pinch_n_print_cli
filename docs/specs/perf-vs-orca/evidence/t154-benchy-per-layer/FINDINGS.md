# t154 — Benchy per-layer carriers and the destination arithmetic at HEAD

Ticket: [`154-benchy-per-layer-carrier-attribution.md`](../../../issues/154-benchy-per-layer-carrier-attribution.md)

**Headline: the benchy per-layer program is no longer on the route.** At HEAD,
with the prepass held at canonical `v2.5.0` cost and every other phase at its
measured share, **7 of 8 cells win** — the four benchy cells by **9.9 %–33.2 %**
and the two base supports-off cells by 25.7 %/30.3 %. The only losing cell is
**base classic-on at 1.051x**, which needs **1.096 s** from something other than
the prepass, i.e. **9.6 %** of its per-layer phase. The per-layer program's
remaining requirement therefore sits on the **base supports-on** cells, not the
benchy cells; on the benchy cells its carriers are in-module kernels and no
named shape is required.

This is a re-derivation, not a new scoreboard: the phase shares are measured
here, the walls are t145/t148 carried by t150's deltas, and the arithmetic is
labelled an estimate throughout (`arithmetic.json`). The decision it drives is
insensitive to that chain — the benchy margins are 10–33 %, far larger than the
cross-take ledger error the map has measured on this rig.

## 1. The arm and the gates

- HEAD `71c03a3e` (`feature/perf-vs-orca`), tree clean apart from this ticket's
  tracker/evidence files.
- `target/release/pnp_cli.exe` SHA-256
  `e1b8fa69e59e47c9bba408cc34b9273b57f92946d641c75b6ec39c67cfc7eaa0`;
  `cargo xtask build-guests --check` exit **0** before the campaign.
- Eight instrumented ordinary passes, `RAYON_NUM_THREADS=12`, the standing
  matched-pair configs, 24 external core modules. Every pass exits 0, is
  `degraded=false` with `fatal_error_count=0` and `non_fatal_error_count=0`, and
  its G-code SHA-256 equals **t151's active set** ([`reference.json`](../t151-skirt-hull-contour/reference.json)):
  `3d28d1b3…` / `eb202973…` / `39ad6289…` / `77c3c578…` and
  `9bb18f0e…` / `eccbd523…` / `2f249cdb…` / `1811425f…`.
- Processor load sampled **7 %** immediately before the campaign.
- Instrumented walls are attribution only and are never wall claims.

## 2. The split at HEAD

| cell | slice ms | prepass | per-layer | postpass | outside | validation |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3,726 | 2,402 (64.5 %) | 971 (26.1 %) | 154 (4.1 %) | 196 (5.3 %) | 3 |
| benchy classic-on | 7,439 | 5,610 (75.4 %) | 1,290 (17.3 %) | 455 (6.1 %) | 81 (1.1 %) | 3 |
| benchy arachne-off | 3,709 | 2,356 (63.5 %) | 1,097 (29.6 %) | 167 (4.5 %) | 86 (2.3 %) | 3 |
| benchy arachne-on | 7,426 | 5,526 (74.4 %) | 1,338 (18.0 %) | 481 (6.5 %) | 77 (1.0 %) | 4 |
| base classic-off | 26,332 | 17,878 (67.9 %) | 7,675 (29.1 %) | 693 (2.6 %) | 83 (0.3 %) | 3 |
| base classic-on | 79,585 | 65,566 (82.4 %) | 11,463 (14.4 %) | 2,464 (3.1 %) | 89 (0.1 %) | 3 |
| base arachne-off | 31,031 | 21,427 (69.1 %) | 8,661 (27.9 %) | 857 (2.8 %) | 83 (0.3 %) | 3 |
| base arachne-on | 83,383 | 68,064 (81.6 %) | 13,033 (15.6 %) | 2,198 (2.6 %) | 86 (0.1 %) | 2 |

Every cell reconciles: the four brackets plus the outside residual equal
`slice_complete` (`verify.py` claim 1). The per-layer phase's module-busy union
equals its wall in 8 of 8 — the host gap is **0–56 ms (≤ 0.65 %)** everywhere, and
zero on both benchy classic cells — so there is no cross-worker idle pool to
reclaim, the same conclusion t126 reached by a different route.

**Since t126 (2026-11-08) the per-layer phase moved in the opposite direction
to the slice:**

| cell | slice | per-layer | per-layer share |
| --- | ---: | ---: | ---: |
| benchy classic-off | −19.6 % | **−33.0 %** | 31.3 % → 26.1 % |
| benchy classic-on | −16.3 % | **−21.6 %** | 18.5 % → 17.3 % |
| benchy arachne-off | −25.8 % | **−34.0 %** | 33.3 % → 29.6 % |
| benchy arachne-on | −30.6 % | **−40.1 %** | 20.9 % → 18.0 % |
| base classic-off | −37.5 % | −4.7 % | 19.1 % → 29.1 % |
| base classic-on | −32.5 % | −8.0 % | 10.6 % → 14.4 % |
| base arachne-off | −31.2 % | +5.3 % | 18.2 % → 27.9 % |
| base arachne-on | −28.4 % | +2.3 % | 10.9 % → 15.6 % |

The benchy per-layer phase fell by a third because the tranches between the two
takes changed its **inputs**, not its modules. Two tree-verified candidates:
`t139`'s canonical mid-layer sampling (`10114830`) changes the sampled region
geometry, and `t142`'s distance-derived quartile (`59be878f`) removes the
four-band polygon partition that `prepare_regions`
(`crates/slicer-wasm-host/src/marshal/prepared.rs`) intersects per region per
layer. **This take does not attribute the fall** and quotes no cause.

## 3. The destination arithmetic at HEAD

`arithmetic.py` substitutes canonical's measured prepass cost (t110: 50.5 /
549.8 / 2,103.0 / 10,444.8 ms; benchy-off is a bound) and holds every other
phase at this take's measured share of the wall of record.

| cell | ratio of record (est.) | PnP wall (est.) | Orca wall | prepass implied | other than prepass | **at canonical prepass** | **required prepass cut** |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy arachne-off | 1.763x | 3.50 s | 1.983 s | 2.22 s | 1.28 s | **0.668x** | 68.1 % |
| base arachne-off | 1.798x | 26.83 s | 14.92 s | 18.53 s | 8.30 s | **0.698x** | 64.3 % |
| benchy classic-off | 2.007x | 3.84 s | 1.915 s | 2.48 s | 1.37 s | **0.740x** | 77.8 % |
| base classic-off | 1.827x | 24.51 s | 13.41 s | 16.64 s | 7.87 s | **0.743x** | 66.7 % |
| benchy classic-on | 2.571x | 6.28 s | 2.442 s | 4.74 s | 1.54 s | **0.857x** | 81.0 % |
| benchy arachne-on | 2.691x | 6.98 s | 2.591 s | 5.19 s | 1.78 s | **0.901x** | 84.5 % |
| base arachne-on | 3.060x | 79.02 s | 25.822 s | 64.50 s | 14.52 s | **0.967x** | 82.5 % |
| base classic-on | 3.220x | 69.54 s | 21.598 s | 57.29 s | 12.25 s | **1.051x** | 83.7 % |

**Three ledger corrections follow.**

1. **t128/t129's "prepass at canonical cost loses both benchy cells
   (1.053x / 1.075x)" is superseded.** It used t126's per-layer phases (1,450 /
   1,644 ms) against t124's Orca walls (1,716 / 2,462 ms). Today's benchy
   per-layer phases are 971 / 1,080 ms and the arms are t145's (1,915 / 2,442
   ms), so the same substitution lands at **0.740x / 0.857x**. The benchy
   per-layer phase is no longer "part of the first program" on the strength of
   that arithmetic — it is margin.
2. **The required prepass cut has fallen from 72–101 % (t128) to 64.3–84.5 %**
   on this take's denominators, and it is **largest on the base supports-on and
   benchy arachne-on cells (83.7 % / 82.5 % / 84.5 %)**, not on benchy
   classic-off. `t128`'s "benchy classic-off cannot be won by prepass deletion
   alone (1.02x at zero prepass)" is superseded by the same re-derivation: its
   zero-prepass result would now be well under 1.0x, and it is not the binding
   cell.
3. **The per-layer program's remaining requirement is base classic-on —
   1.096 s, 9.6 % of its per-layer phase — and base arachne-on is only 3.3 %
   inside the line.** Everything else is margin. A prepass rewrite that beats
   canonical by a few percent closes base classic-on without any per-layer
   change; a per-layer shape that removes ~10 % of that cell's phase also does.

**Disclosures.** The arithmetic chains two windows: phase shares are this
take's instrumented run; walls are t145/t148 medians carried by t150's measured
per-cell deltas; the canonical prepass is t110's tap split from 2026-11-03. No
ratio here is a ratio of record. `verify.py` re-derives every row from
`summary.json` and the encoded denominators.

## 4. What carries the per-layer phase

`carriers.json` ranks every per-layer module by thread-accumulated time and
prices it as `thread_sum / measured effective parallelism` — a **wall-equivalent
ceiling** that charges 100 % of the carrier's parallel work to the critical
path, never a price.

| cell | effective parallelism | top carrier | share of per-layer thread | ceiling (% of cell wall, x floor) |
| --- | ---: | --- | ---: | ---: |
| benchy classic-off | 6.74x | `classic-perimeters` | 46.8 % | 11.8 %, 14.8x |
| benchy arachne-off | 6.95x | `arachne-perimeters` | 54.8 % | 17.2 %, 21.5x |
| benchy classic-on | 9.65x | `tree-support` | 47.8 % | 9.8 %, 12.3x |
| benchy arachne-on | 9.77x | `tree-support` | 44.0 % | 8.4 %, 10.6x |
| base classic-off | 10.91x | `classic-perimeters` | 52.8 % | 16.5 %, 20.7x |
| base arachne-off | 10.90x | `arachne-perimeters` | 55.1 % | 17.8 %, 22.2x |
| base classic-on | 11.18x | `tree-support` | 36.7 % | 6.0 %, 7.5x |
| base arachne-on | 11.30x | `arachne-perimeters` | 36.0 % | 5.9 %, 7.4x |

The second carrier on the supports-off cells is the **first `Layer::Infill`
dispatch**, `com.core.raft-default` — 21.2 % of benchy classic-off's per-layer
thread, **37.3 % of base classic-off's** — whose own work is nil (the matched job
resolves no raft band, and t127 measured the dispatch as output-free while the
wall did not move when it was declined). The third is
`com.core.infill-linker` (18.5–20.5 % on the benchy supports-off cells, 5.3 % on
base classic-off).

**The per-op decomposition the ticket asked for reaches a hard limit.** The
fuel profile (ADR-0055) exposes exactly one scope row inside `classic-perimeters`
(`polygon_ops::offset2_ex`, 1,486 calls, **22.8 %** of the module's fuel) and one
inside `infill-linker` (`polygon_ops::clip_polygons`, **0.2 %**); everything else
is `self_fuel`. The same holds on the supports-on cells: `tree-support` is
**96.6–96.8 %** self fuel, its `clip_polygons` scope 2.9–3.2 %. So the carriers
are **in-module kernel and algorithm work**, not host plumbing that a marshalling
change can remove. That is a finding, not an omission: the plumbing hypothesis
is dead for these carriers on the same evidence t131 used for the prepass.

## 5. Canonical's per-layer work set

Source audit at the pin `v2.5.0` (`40ecc66f78`), by file + function:

| canonical | PnP at HEAD | phase |
| --- | --- | --- |
| `PrintObject::make_perimeters` → `Layer::make_perimeters`, tbb over layers | `Layer::Perimeters` (one WASM module per layer) | per-layer both |
| `PrintObject::infill` → `Layer::make_fills`, tbb over layers | `Layer::Infill` (raft-default + generator modules per layer, plus the stage-shared block) | per-layer both |
| — | `Layer::InfillPostProcess` (`infill-linker`) | **PnP-only per layer** |
| `SeamPlacer::place_seam`, called from `GCode::process_layers` (emit) | `Layer::PerimetersPostProcess` (`seam-placer`) | canonical emit, PnP per layer |
| `PrintObject::_generate_support_material` → `TreeSupport::generate` (whole object) | `PrePass::SupportGeometry`/planner **and** `Layer::Support` (`tree-support`) | canonical prepass, PnP prepass **plus** per layer |
| — | `Layer::PathOptimization` | PnP-only per layer |
| no counterpart | `prepare_regions` via `prepare_slice_regions`/`ensure_prepared_regions` | **PnP-only host work** |

Two structural readings:

- **The supports-on cells pay support work twice.** Canonical generates tree
  support in one whole-object pass and consumes it at emit; PnP's prepass plans
  it and then `Layer::Support` renders it per layer (36.7 % of base classic-on's
  per-layer thread). That is a **work-set/phase difference**, not a per-op
  inefficiency, and it is the largest named term on the one cell the destination
  still needs.
- **The Infill-stage block has no canonical counterpart and its identity is
  open.** t127 proved by declining `raft-default` that the first dispatch of
  `Layer::Infill` pays a stage-shared block and that the cost transfers to
  whichever Infill module runs first; it named the block "the shared
  prepared-region view materialisation". At HEAD that name cannot be right
  without an invalidation: `prepare_dispatch_region_views`
  (`crates/slicer-runtime/src/layer_executor.rs`) asks
  `ensure_prepared_perimeter_source_regions`
  (`crates/slicer-runtime/src/blackboard.rs`) first for `Layer::Perimeters`, and
  for an identity projection (`perimeter_source_projection_is_identity`, every
  matched job) that call materialises `prepared_regions` and sets the sharing
  flag; `Layer::Infill` then hits the memo. The block's measured shape is real
  (p50 **63 ms** on base classic-off, 5 ms on benchy classic-off, and `gyroid`'s
  p50 is 1 ms when it is not first), but **what it is remains unmeasured**.

## 6. Verdict

1. **The benchy per-layer program is margin, not route.** No benchy carrier is
   required to win its cell at canonical prepass cost; all four benchy cells
   clear the line by 9.9 %–33.2 %.
2. **The per-layer requirement that remains is base classic-on's 1.096 s**, and
   its named carriers are `tree-support` (36.7 %, a work-set difference),
   `classic-perimeters` (33.4 %, kernel-bound) and the Infill-stage block
   (23.5 %, identity unmeasured). A ~10 % cut of that phase would close the cell.
3. **The benchy carriers are kernel-bound.** `classic-perimeters` and
   `arachne-perimeters` are 77–98 % self fuel, `infill-linker` 99.8–100 %, and
   the named scope rows explain at most 22.8 % of a carrier's fuel.

**What this take does not claim.** No wall claim from any instrumented or
profiled run; no ratio of record (the arithmetic is an estimate chaining two
windows, disclosed per cell); no cause for the t126→t154 per-layer fall; no
identity for the `Layer::Infill` block; no production change and no G-code
reference move.

## 7. Reproduction

`REPRODUCE.md` beside this file. The committed set is `summary.json` (8 cells),
`arithmetic.json`, `carriers.json`, eight `profile-*.json` reduced scope
tables, `runs.json`, the three runners and reducers, and `verify.py`
(checkout-only; `--self-test` rejects 11/11 negative controls).
