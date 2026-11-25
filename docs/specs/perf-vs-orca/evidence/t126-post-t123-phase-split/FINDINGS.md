# t126 — post-t123 phase split and per-layer/emit attribution

Route: [Post-t123 phase split refresh and per-layer/emit attribution](../../issues/126-post-t123-phase-split-refresh.md).
Take: 2026-11-08, ordinary arm, `target/release/pnp_cli.exe` =
`7737a74f…` (ticket 123's candidate artifact), `RAYON_NUM_THREADS=12`, the
standing matched-pair configs, 24 external core modules, one instrumented pass
per cell on **all eight cells** (ticket 104 measured four classic cells and
disclosed Arachne unmeasured). Measurement-only: no production edit.

## Verdict

**The split is re-derived against ticket 124's ratios of record, and it
re-prices the map's next stage.** The prepass still dominates every cell
(**62.1–87.1 %**), the per-layer phase is **10.6–33.3 %**, and the postpass is
**1.8–4.9 %**. But the composition inside the prepass changed completely:
`PrePass::Slice` is no longer a term (**41,472 → 2,110 ms** on base
classic-off, **−94.9 %**), and the prepass is now a cluster of stages rather
than one dominator.

**The destination arithmetic is the headline.** Deleting the entire per-layer
phase never beats Orca in any cell (**1.67x–4.14x** remain). Deleting the entire
prepass reaches **0.48x–1.02x** — and on **benchy classic-off it lands at
1.02x**, i.e. even a zero prepass does not win that cell. The prepass fraction
that must be removed is **72 %–101 %** across the eight cells. No per-layer or
emit program can close the destination; the destination needs a prepass
rewrite of 72–101 % of today's prepass, or it needs re-aiming.

**One clean, exact per-layer shape is graduated:** the raft-fill dispatch pays
**61–63 ms on all 495 layers** of every base cell while its body returns before
doing any work (`base_raft_layers = 1`, `interface_raft_layers = 0` ⇒
`raft_layers = 2`). Every call exceeds 10 ms on base; only 6–20 of 240 exceed
10 ms on benchy. It is module-specific, not stage preparation: on the 17 layers
where another module runs first it still pays ~30 ms. Ceiling
**2.64x–8.25x** the 0.80 % floor. Emit is **not** a route: the postpass is
1.8–4.9 % of every cell and its largest single bracket is 0.85–2.61x floor.

## 1. Phase split (instrumented, attribution only)

| cell | `slice_complete` | prepass | per-layer | postpass | outside phases | layers |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4,634 ms | **62.1 %** | 31.3 % | 2.9 % | 3.7 % | 240 |
| benchy classic-on | 8,887 ms | **76.4 %** | 18.5 % | 4.2 % | 0.9 % | 240 |
| benchy arachne-off | 5,001 ms | **62.2 %** | 33.3 % | 2.9 % | 1.5 % | 240 |
| benchy arachne-on | 10,704 ms | **73.4 %** | 20.9 % | 4.9 % | 0.7 % | 240 |
| base classic-off | 42,150 ms | **78.3 %** | 19.1 % | 2.4 % | 0.2 % | 495 |
| base classic-on | 117,970 ms | **87.1 %** | 10.6 % | 2.3 % | 0.1 % | 495 |
| base arachne-off | 45,089 ms | **79.8 %** | 18.2 % | 1.8 % | 0.2 % | 495 |
| base arachne-on | 116,449 ms | **86.7 %** | 10.9 % | 2.2 % | 0.1 % | 495 |

"outside phases" is `slice_complete` minus the four phase brackets — startup,
mesh IO, teardown. Shares use `slice_complete` as the denominator; the phase
brackets and the residual reconcile with it in every cell.

**Against t104 (2026-11-02, pre-t121 and pre-t123) the per-layer *share* roughly
doubled** — base classic-off 8.7 % → 19.1 %, benchy classic-off 20.1 % → 31.3 % —
because the prepass shrank, not because the per-layer phase grew. The per-layer
phase wall itself is 8,053 ms on base classic-off against t104's 7,016 ms.

## 2. Prepass stages: the dominator is gone

Instrumented stage brackets (ms). Prepass stages are serial within their phase,
so their brackets reconcile with the prepass phase; they are still instrumented
attribution, never a wall claim.

| stage | benchy off | benchy on | base off | base on |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::SupportGeometry` | 39 | 2,842 | 296 | **55,798** |
| `PrePass::ShellClassification` | 1,633 | 1,668 | **14,698** | 15,300 |
| `PrePass::OverhangAnnotation` | 617 | 596 | 9,689 | 10,309 |
| `PrePass::SupportAnalysis` | 0 | 1,024 | 0 | 13,084 |
| `PrePass::MeshAnalysis` | 162 | 179 | 4,260 | 4,278 |
| `PrePass::Slice` | 205 | 232 | 2,110 | 2,117 |
| `PrePass::SeamPlanning` | 171 | 191 | 1,718 | 1,631 |

Arachne cells sit beside their classic siblings: `SupportGeometry` 3,441 /
55,236 ms, `ShellClassification` 1,892 / 17,033 ms, `OverhangAnnotation` 587 /
10,172 ms, `Slice` 207 / 2,189 ms (benchy-on / base-off).

**`PrePass::Slice` fell 41,472 → 2,110 ms on base classic-off (−94.9 %), 2,301 →
205 ms on benchy classic-off (−91.1 %) and 44,558 → 2,117 ms on base
classic-on.** The supports-off prepass is now `ShellClassification` +
`OverhangAnnotation` (**24.4 s of a 42.5 s cell on base classic-off**); the
supports-on prepass is the support planner stages (`SupportGeometry` +
`SupportAnalysis` = **68.9 s of a 126.3 s cell on base classic-on**). No stage is
a single dominator any more.

Cross-take caveat: the t104 base classic-on pass is disclosed at **+43.6 %**
over its quiet median, so the t104→t126 deltas on that cell (including
`SupportGeometry` 91,902 → 55,798 ms) are directional only. The within-take
shares above are the product.

## 3. The counterfactual arithmetic (denominated by t124)

`ratio_if_X_zero = ratio × (1 − share(X))`. Park = 1.5x. The "prepass fraction
needed" column is `cut_to_beat / prepass_share`.

| cell | ratio of record | cut to beat | prepass share | if prepass 0 | if per-layer 0 | if both 0 | prepass fraction needed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.70x | 63 % | 62.1 % | **1.02x** | 1.86x | 0.18x | **101 %** |
| benchy classic-on | 3.61x | 72 % | 76.4 % | 0.85x | 2.95x | 0.18x | 95 % |
| benchy arachne-off | 2.50x | 60 % | 62.2 % | 0.94x | 1.67x | 0.11x | 96 % |
| benchy arachne-on | 3.59x | 72 % | 73.4 % | 0.95x | 2.84x | 0.20x | 98 % |
| base classic-off | 2.95x | 66 % | 78.3 % | 0.64x | 2.38x | 0.08x | 84 % |
| base classic-on | 4.63x | 78 % | 87.1 % | 0.60x | 4.14x | 0.11x | 90 % |
| base arachne-off | 2.36x | 58 % | 79.8 % | 0.48x | 1.93x | 0.05x | 72 % |
| base arachne-on | 4.65x | 79 % | 86.7 % | 0.62x | 4.14x | 0.11x | 91 % |

Two rulings follow directly:

1. **The per-layer/emit program is not a route to the destination.** Deleting
   the whole phase leaves 1.67x–4.14x; the best it can do is close part of a
   gap whose remaining term is the prepass.
2. **benchy classic-off cannot be won by prepass deletion alone.** Its prepass
   share (62.1 %) is smaller than its required cut (63.0 %), so the counterfactual
   lands at 1.02x. Every other cell needs 72–98 % of its prepass removed.

## 4. Per-layer attribution

The per-layer phase is parallel. A module's **wall-equivalent ceiling** is
`thread_sum / parallelism` (the measured parallelism is 5.23–11.71 across the
eight cells); this charges 100 % of that module's parallel work to the critical
path and is an **upper bound**, not a price. The floor is 0.80 % of the cell's
t124 median.

| cell | parallelism | floor | carrying modules (thread ms / p50 ms / ceiling / floor multiple) |
| --- | ---: | ---: | --- |
| benchy classic-off | 5.23 | 46 ms | classic-perimeters 2,683 / 8 / 513 ms / **11.11x**; infill-linker 1,578 / 3 / 302 / **6.54x**; raft-default 1,180 / 4 / 226 / **4.89x** |
| benchy classic-on | 7.70 | 80 ms | tree-support 4,896 / 19 / 636 / **7.93x**; classic-perimeters 2,755 / 9 / 358 / 4.46x; infill-linker 1,746 / 3 / 227 / 2.83x |
| benchy arachne-off | 5.70 | 43 ms | arachne-perimeters 3,854 / 12 / 676 / **15.84x**; infill-linker 2,014 / 3 / 353 / **8.28x**; raft-default 1,260 / 4 / 221 / 5.18x |
| benchy arachne-on | 7.77 | 80 ms | tree-support 5,961 / 21 / 767 / **9.63x**; arachne-perimeters 4,507 / 14 / 580 / 7.28x; infill-linker 2,588 / 3 / 333 / 4.18x |
| base classic-off | 11.71 | 340 ms | classic-perimeters 46,513 / 94 / 3,970 ms / **11.67x**; raft-default 32,547 / 63 / 2,778 / **8.16x**; infill-linker 4,667 / 8 / 398 / 1.17x |
| base classic-on | 11.61 | 1,010 ms | tree-support 54,666 / 105 / 4,708 / **4.66x**; classic-perimeters 44,787 / 93 / 3,857 / 3.82x; raft-default 30,920 / 62 / 2,663 / 2.64x |
| base arachne-off | 11.70 | 329 ms | arachne-perimeters 49,418 / 102 / 4,224 / **12.82x**; raft-default 31,800 / 62 / 2,718 / **8.25x**; infill-linker 4,928 / 9 / 421 / 1.28x |
| base arachne-on | 11.67 | 977 ms | tree-support 53,774 / 105 / 4,608 / **4.72x**; arachne-perimeters 48,928 / 99 / 4,193 / 4.29x; raft-default 30,986 / 61 / 2,655 / 2.72x |

**There is no globally idle wall left inside the per-layer phase.** The
module-busy union equals the phase wall on all four base cells (0 ms gap) and
leaves a 2.0–2.3 % gap on benchy (33–44 ms = 0.47–0.90x floor). This measures
cross-worker idle only; a per-worker host cost between dispatches would be
hidden under other workers' module intervals, so the instrument does not clear
or convict the per-dispatch host path. It does rule out a reclaimable idle pool.

The carriers, read against their own shapes:

- **perimeters** (`classic-perimeters` / `arachne-perimeters`) is the largest
  single term in six of eight cells, at 93–105 ms per call on base. Its work is
  the per-layer perimeter generation, which t68 already measured as **97–99 %
  native clipper ops** with every named round-trip reduction sub-noise; the
  ceiling above is therefore not a candidate shape by itself.
- **tree-support** costs 19–21 ms per call on benchy-on and 105 ms on base-on.
  Its whole call is the planner's per-layer work (it is the only module in
  `Layer::Support`); t113 split the occupancy inflation roughly evenly between
  the per-layer emit check and the per-origin carve, and t125 dropped the carve
  half. The emit-check half is **not** an automatic candidate: it needs its own
  probe, and t125's DROP is a live warning that batching occupancy offsets did
  not transfer to wall.
- **infill-linker** is 3–9 ms per call and above the floor on the benchy
  supports-off cells (6.54x / 8.28x) and marginal on base (1.17x / 1.28x). It is
  the post-batch residual t62/t65 measured (per-path `execute` 85.96 %,
  `engine_add` 4.42 %, connect 4.73 %, prepare 2.60 %, metadata 1.25 %).

## 5. Emit / postpass: not a route

| cell | postpass share | `GCodeSerialize` | `LayerFinalization` | `GCodeEmit` | largest bracket vs floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.9 % | 45 | 36 | 9 | 0.98x |
| benchy classic-on | 4.2 % | 95 | 155 | 38 | 1.94x |
| benchy arachne-off | 2.9 % | 51 | 38 | 10 | 1.19x |
| benchy arachne-on | 4.9 % | 142 | 208 | 61 | 2.61x |
| base classic-off | 2.4 % | 367 | 243 | 86 | 1.08x |
| base classic-on | 2.3 % | 646 | 858 | 612 | 0.85x |
| base arachne-off | 1.8 % | 303 | 223 | 62 | 0.92x |
| base arachne-on | 2.2 % | 677 | 862 | 564 | 0.88x |

The postpass is 1.8–4.9 % of every cell and **no single bracket is a route**:
the largest floor multiple is 2.61x (benchy arachne-on `LayerFinalization`),
while on the four base cells the largest bracket is 0.85–1.08x — at or below the
floor. The whole-print layer-collection drop that used to dominate this phase
was already priced into the mimalloc keep (t102) and the t74/t90 ownership
changes.

## 6. The raft-default anomaly

`com.core.raft-default` declares `Layer::Infill / claim:raft-fill` and its
`run_infill` returns **before any work** on every layer the raft plan cannot
cover:

```text
let Some(plan) = paint.raft_plan() else { return Ok(()) };
if !paint.is_raft() || plan.raft_layers == 0 { return Ok(()) };
if layer_index >= plan.raft_layers { return Ok(()) };
```

The matched job resolves `base_raft_layers = 1`, `interface_raft_layers = 0`,
so `raft_layers = 2`; the guard fires on layers 2–494. The module still costs:

| cell | calls | thread ms | p50 | max | calls > 10 ms | calls > 50 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 240 | 1,180 | 4 | 33 | 6 | 0 |
| benchy classic-on | 240 | 1,171 | 4 | 35 | 6 | 0 |
| benchy arachne-off | 240 | 1,260 | 4 | 32 | 12 | 0 |
| benchy arachne-on | 240 | 1,503 | 5 | 31 | 20 | 0 |
| base classic-off | 495 | 32,547 | **63** | 313 | **495** | 354 |
| base classic-on | 495 | 30,920 | **62** | 183 | **495** | 370 |
| base arachne-off | 495 | 31,800 | **62** | 244 | **495** | 354 |
| base arachne-on | 495 | 30,986 | **61** | 268 | **495** | 346 |

The cost is geometry-proportional (it rises to mid-model and falls at the top)
and **module-specific, not stage preparation**: on the 17 base classic-off layers
where `gyroid-infill` completes first it costs 0 ms there while raft-default
still pays a 30 ms median. Skipping the dispatch is therefore expected to remove
the cost rather than move it to the next module.

`module_start` is emitted immediately before dispatch and `module_complete`
after it returns, so this cost is in the dispatch path (view materialisation /
input construction), not in the module body. The exact mechanism is not
isolated here and is not required for the shape: the selection index already
receives `layer.is_raft` and already emits a `Raft` target only inside the band,
so a raft-fill module can decline outside it by the same predicate its own body
uses. That is output-neutral by construction.

## 7. Shape pricing and the stop rule

| shape | exact? | ceiling | status |
| --- | --- | --- | --- |
| raft-fill dispatch outside the raft band | yes — the body already returns there | 8.16x / 8.25x (base off), 2.64x / 2.72x (base on), 4.89x / 5.18x (benchy off) | **graduated** |
| per-layer emit-check occupancy half | unknown — needs its own probe | ≤ 4.66x / 4.72x for the whole `tree-support` call; the half is smaller | fog, no automatic rescue |
| `infill-linker` residual | no shape identified | 1.17x–8.28x | fog, needs a shape |
| perimeters batch/round-trip | no — t68 measured 97–99 % native clipper ops | 3.82x–15.84x | closed by t68 |
| postpass brackets | no shape identified | ≤ 2.61x | fog |

**Stop rule:** no shape inside the per-layer or emit program is priced as a
candidate other than the raft-fill dispatch, so no candidate campaign other than
that one is carried and no production edit is made by this take.

## 8. Disclosures

- **Ordinary arm only.** The accelerated arm is not measured; every earlier take
  found ordinary and accelerated G-code byte-identical but their walls differ,
  so no accelerated share is claimed.
- **Instrumented walls are not wall claims.** The pass walls were 5.08 / 9.29 /
  5.10 / 11.78 s (benchy classic-off/on, arachne-off/on) and 43.92 / 119.64 /
  46.77 / 118.45 s (base). Base classic-off is **+3.3 %** over ticket 124's
  uninstrumented 42.53 s median; shares are the product.
- **Machine load**: sampled 0.8–7.5 % (median ≈2.1 %) immediately before the
  batch on the 12-core box; no browser or build process was running during it.
- **One pass per cell.** No share-stability repeat was taken; the 0.80 % floor
  arithmetic uses ticket 124's paired medians, not these shares' uncertainty
  (t104's own repeat moved no share by more than 5.92 pp).
- **`ratio_if_*_zero` is a counterfactual bound**, not a prediction: it assumes
  the removed phase disappears with no interaction.
- **Cross-take deltas are directional only** (different windows and, for t104's
  base-on pass, a disclosed +43.6 % instrumented inflation). Every share and
  every counterfactual in this take is within-take.
- **No production edit, no G-code reference move.** All eight instrumented
  outputs equal the ticket-123 candidate-ordinary reference set; both guest
  freshness checks exit 0.
