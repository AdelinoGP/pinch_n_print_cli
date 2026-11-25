# t173 findings — support-path boolean volume

**Verdict: the support path's boolean mass is one module and two batched sites — `com.core.tree-support-planner`'s phase-1 carve and phase-3 union — and no single boolean-volume shape clears either base supports-on cell.** The phase-1 bbox/tile-window shape is the cluster's largest lever at **60.8 % / 61.6 %** of `base classic-on` / `base arachne-on`'s required cuts, leaving **1.150x / 1.120x**; a **combination** of that shape and a phase-3 union shape is the first projected crossing (≈**0.97x / 0.95x**) but rests on phase-3 going to zero, which no shape in this take delivers. The **benchy supports-on pair does not ride this program**: its whole planner clip wall is **0.73 / 0.72 s** against cuts of **2.08 / 2.15 s**, and zeroing all of it leaves **1.21x / 1.23x**. The route's re-aim is therefore **phase-1 tiling plus a phase-3 companion for the base pair, and a different program entirely for the benchy pair**; [`175`](../../issues/175-tiled-collision-carve.md) carries the phase-1 candidate and the phase-3 shape stays in the map's fog.

Resolved 2026-11-20. Ticket: [173](../../issues/173-support-geometry-boolean-volume.md).

Attribution only: the probe's thread-time is inflated by its own bookkeeping and `--profile` inflates wall, so no number here is a wall claim or a ratio of record. The shape prices are **ceilings** from measured operand counts, not gate results; no candidate was landed.

## 1. The census: who issues `clip_polygons`

One probe pass per cell on the quiet window, `PNP_T173_PROBE=1` with `--profile`; every run reproduces t172's active reference set in **5/5** (`runs-probe.json`). Rows are accumulated thread-time; the batch wall is the calling thread's blocked span.

| cell | planner clip thread | planner clip wall | planner offset wall | planner share of all clip thread |
| --- | ---: | ---: | ---: | ---: |
| base classic on | **196.6 s** | **18.00 s** | 1.00 s | **85.2 %** |
| base arachne on | 187.0 s | 17.18 s | 1.01 s | 86.0 % |
| benchy classic on | 7.0 s | 0.73 s | 0.10 s | 71.6 % |
| benchy arachne on | 6.9 s | 0.72 s | 0.10 s | 69.7 % |
| base classic off (control) | 0.0 s | 0.00 s | 0.00 s | 0.0 % |

The non-planner clip rows are host built-ins (`host:shell_classification` 13.5 s thread, `host:support_analysis` 3.8 s, `host:mesh_analysis` 2.9 s, `host:slice` 3.2 s, `host:overhang_annotation` 2.5 s on base classic-on) and per-layer host calls the probe could not name (`<unknown>`, 7.8 s). t171's **186.6 s** `host:native` clip bucket is this planner; the probe reads 196.6 s because its bookkeeping and the post-t172 tree both move the number.

**Base classic-on, ranked by site** (the arachne cell is within 1 % on every row):

| site | op | calls | thread | wall | fan-out | in-points | clip-points | in-bbox points |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `planner::batched_phase1_carve` | difference | **68,381** | **142.7 s** | **12.41 s** | **11.5x** | 433.6 M | **426.9 M** | **285,368** |
| `planner::batched_phase3_union` | union | 480 | 51.6 s | 5.21 s | 9.9x | 13.5 M | 13.5 M | 324,934 |
| `planner::batched_phase5_set_difference` | difference | 517 | 1.68 s | 0.20 s | 8.4x | 4.09 M | 2.86 M | 2.15 M |
| `planner::ensure_avoidance_union` | union | 18,810 | 0.45 s | 0.11 s | 4.1x | 2.70 M | 0.70 M | 0.63 M |
| `planner::batched_phase6_body_minus_cut` | difference | 85 | 0.08 s | 0.02 s | 3.2x | 0.29 M | 0.01 M | 0.01 M |

Phase 1 and phase 3 together are **194.3 s of 196.6 s** (98.8 %) of the planner's clip thread and **17.62 s of 18.00 s** (97.9 %) of its wall. The other five sites are noise at this scale.

**The operand is a few huge polygons, not many small ones.** Phase 1 pushes a mean **6,241 clip points per call** over only **4.34 paths** (mean 1,437 points/path), against a **98.7-point** subject. That is why a polygon-level bbox filter is weak (below) and a geometric window is the only large shape.

## 2. `SupportGeometry` decomposition

Stage share from three instrumented passes per cell (probe off), applied to t172's paired wall of record — a cross-window composition. The two base cells also have a **same-window** take (probe and `--instrument-stderr` in one run); that one is the decomposition of record for those cells, the cross-window number is its lower bound.

| cell | stage wall | host clip | host offset | remainder (guest + host non-clipper) |
| --- | ---: | ---: | ---: | ---: |
| base classic on, same window | 32.45 s | **20.18 s (62.2 %)** | 1.07 s (3.3 %) | 11.21 s (34.5 %) |
| base arachne on, same window | 29.13 s | **18.24 s (62.6 %)** | 1.06 s (3.6 %) | 9.82 s (33.7 %) |
| base classic on, cross-window | 28.31 s | 18.00 s (63.6 %) | 1.00 s (3.5 %) | 9.30 s (32.9 %) |
| base arachne on, cross-window | 27.09 s | 17.18 s (63.4 %) | 1.01 s (3.7 %) | 8.90 s (32.9 %) |
| benchy classic on, cross-window | 1.89 s | 0.73 s (**38.8 %**) | 0.10 s (5.1 %) | 1.06 s (**56.1 %**) |
| benchy arachne on, cross-window | 1.90 s | 0.73 s (**38.3 %**) | 0.10 s (5.4 %) | 1.07 s (**56.4 %**) |

The guest side is `com.core.tree-support-planner`: **62.3 B fuel**, **10.8 % / 11.0 %** of the cell's total guest fuel (9.6 % on both benchy cells). Its `polygon_ops` scopes are empty — the planner issues no in-process clipper work; all of its geometry crosses the WIT boundary and is counted above. The **per-layer renderer** (`com.core.tree-support`) is a different module in a different phase: **67.3 % / 68.6 %** of cell fuel on the base cells, with 1.9 % of fuel in its own in-process `polygon_ops::clip_polygons` — it is not part of `SupportGeometry` and is not in this census.

So on the base pair the stage is **two-thirds host clipper, one-third remainder**; on the benchy pair it is **under 40 % host clipper, over half remainder**. That is the sharpest single statement of why the four supports-on cells are not one program.

## 3. Shape prices

Every price is a ceiling: the reduced operand is assumed to cost its point/edge share, and implementation overhead is zero. `ratio_after` applies the ceiling to t172's estimated post-t172 ratio; the route itself is an estimate (t174 is the fresh board).

| shape | base classic on | base arachne on | benchy classic on | benchy arachne on |
| --- | ---: | ---: | ---: | ---: |
| phase-1 bbox/tile window | **12.21 s → 60.8 % of cut → 1.150x** | **11.49 s → 61.6 % → 1.120x** | 0.48 s → 23.2 % → 1.298x | 0.47 s → 21.8 % → 1.317x |
| phase-1 polygon-bbox filter (exact, cheap) | 3.54 s → 17.6 % → 1.454x | 3.33 s → 17.8 % → 1.398x | 0.28 s → 13.6 % → 1.366x | 0.28 s → 12.8 % → 1.383x |
| phase-3 union free (ceiling, no shape) | 5.21 s → 26.0 % → 1.395x | 5.12 s → 27.4 % → 1.337x | 0.18 s → 8.8 % → 1.400x | 0.19 s → 8.8 % → 1.413x |
| all planner clip free | 18.00 s → 89.7 % → **0.948x** | 17.18 s → 92.1 % → **0.926x** | 0.73 s → 35.2 % → 1.213x | 0.73 s → 33.7 % → 1.229x |
| all planner clip + offset free | 19.01 s → 94.7 % → **0.913x** | 18.19 s → 97.6 % → **0.891x** | 0.83 s → 39.9 % → 1.180x | 0.83 s → 38.5 % → 1.194x |

**The 0.80 % floor is not the binding gate here.** Against a floor of `0.008 ×`
the cell's paired wall, the ceilings clear it by **8.0x–52.7x** (phase-1 window
**33.8x / 32.4x** on the base pair; polygon filter **9.8x / 9.4x**; phase-3 free
**14.4x**; all clip free **49.9x / 48.5x**). What binds is the destination cut,
which is why the phase-1 window can be 34x the floor and still only 61 % of the
way to a win.

The **phase-1 window** is the only large named shape. Its ceiling comes from the operand: the subject's bbox contains **285,368 of 426,868,700** clip points — **0.0669 %** — and the subject's own edges are the floor, so the windowed edge count is **1.62 %** of today's. A polygon-level bbox filter is exact and trivial but removes only **28.5 %** of phase-1's clip points (the four huge polygons mostly overlap the subject bbox), which is why its price is a sixth of the window's.

**The combination** of the phase-1 window and a phase-3 union that reaches zero is **0.969x / 0.945x** — the first crossing — but the phase-3 term is a ceiling, not a shape. Nothing in this take shows a union reduction, and phase 3 is a union of role regions that cannot drop outside-bbox geometry.

**No single boolean shape, and not even all planner clip, is roomy.** On base classic-on, all clip free is **89.7 %** of the cut; the phase-1 window is **60.8 %**. On the benchy pair, all clip free is **35 % / 34 %** — boolean volume is not the benchy lever at all.

## 4. Route implication

- **Base supports-on pair:** the cluster is a **combination**, not a deletion. Phase-1 tiling is the first candidate ([`175`](../../issues/175-tiled-collision-carve.md)); its measured ceiling is 60.8 % / 61.6 % of the cut and leaves 1.150x / 1.120x, so it cannot be banked alone. The phase-3 union shape is the open companion (26.0 % / 27.4 % of the cut at zero) and goes to the map's fog with no named mechanism.
- **Benchy supports-on pair:** the same two sites exist but are a minor term (all clip free is 35 % / 34 % of the cut); more than half of `SupportGeometry` is outside the host clipper. The pair needs the per-layer/guest program, not a boolean program. This answers the fog's "does the benchy supports-on pair ride the same boolean program?" — **no**.
- **Base supports-off control:** the planner is inert (0 ms; `enable_support: false` returns before it), the stage is 0.21 s, and the cell is estimated **0.82x**; no shape applies.

## 5. Disclosures

- **One probe pass per cell, one box, one window.** Phase 1 carries **identical operands** on the two base cells (same 68,381 calls / 426.87 M points) yet its thread-time ranged **134–216 s** across takes (wall 11.7–18.9 s), so the spread is box state, not code. The price uses the quietest take; the slowest take would push the phase-1-only ceiling to the line on base arachne-on but not past it. All probe walls are inflated by the probe's own per-call bookkeeping and the `clip_locality` traversal.
- **The phase-1 window ceiling is not a candidate.** It assumes tiles/cuts cost nothing and the operand falls to the in-bbox point count; a real tiled implementation pays a partition per layer and boundary edges. The polygon-bbox filter is the only shape here that is exact, cheap, and directly implementable.
- **Cross-window composition.** Stage shares are instrumented; walls are t172's paired medians; the route is t163 × (1 − t169) × (1 − t172). t161 forbids carrying a required cut across windows as a measurement, so every `ratio_after` is an estimate and [174](../../issues/174-post-t172-scoreboard-refresh.md) is the fresh board.
- **No accelerated arm.** The probe build was ordinary; t163 bounds the accelerated effect at −13.2…+4.4 % wall, which cannot change the direction of any ceiling here.
- **`--profile` and the probe never share a wall claim.** The `--profile` totals are accumulated thread-time; the batch rows are the only wall numbers and they include probe overhead.
- **No production edit landed.** The probe is reverted and preserved as `probe.patch`; all 5 probe runs and 15 instrumented passes reproduce t172's active reference set byte-for-byte.

## 6. Verification

- `python verify.py` — **PASS**, every quoted number re-derives from the committed reduced set.
- `python verify.py --self-test` — **PASS**, **8/8** negative controls rejected (route ratio, census thread, stage share, phase-1 price, in-bbox share, all-clip price, reference hash, probe exit).
- `cargo xtask build-guests --check` — exit 0 before the campaign (and re-checked after the probe revert).

## 7. Reproduce

See [`REPRODUCE.md`](REPRODUCE.md). Short form: apply `probe.patch`, rebuild the host and guests, run `run_probe_t173.py` and `run_instrumented_t173.py`, reduce, then `python arithmetic.py` and `python verify.py --self-test`.
