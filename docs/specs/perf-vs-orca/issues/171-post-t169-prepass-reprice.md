# Re-price the prepass work-set, the phase split and S5/S6 after t169

Type: task / attribution + re-price (instrumented, no candidate)
Label: wayfinder:task
Status: **resolved 2026-11-20** — the split re-derives, the route re-aims (the
supports-on cluster is boolean-bound, the supports-off cluster is t172's), and
S5/S6 are landed with no residual worth a ticket. No production edit.
Assignee: 2026-11-20 OpenCode wayfinder session — the human said "Continue the
perf vs orca map" (no ticket named); 171 was first on the frontier
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t169 offset edge decimation](169-canonical-offset-edge-decimation-and-flat-opening.md)
([`evidence/t169-offset-decimation-flat-opening/FINDINGS.md`](../evidence/t169-offset-decimation-flat-opening/FINDINGS.md)),
which cut whole-cell wall by **21.91–25.08 %** on the four `base` cells and
**5.74–15.21 %** on the four benchy cells and cut `polygon_ops::offset`'s own
self wall by **41.0–55.5 %**. Every operand-volume price in the map was taken
against the pre-t169 primitive.

## Question

**After t169's decimation, what do the prepass work-set, the phase split, and
the two priced operand shapes (S5 windowed internal-bridge offsets, S6 exact
occupancy memo) cost — and does any of them still clear the 0.80 % floor on all
eight cells?**

Concretely:

1. **Phase split.** Re-derive prepass / per-layer / postpass / outside shares on
   all eight cells. t164's split (prepass **67.0–87.2 %**, per-layer 9.3–26.3 %,
   postpass 3.2–6.8 %) was measured on the pre-t169 tree, where the prepass was
   the offset-heavy phase; t169 removed 41–55 % of the offset primitive's
   self-time, so the shares have moved and every projection built on them is
   stale.
2. **Prepass stage split.** Which stages now carry the supports-off cluster and
   the supports-on cluster? t126/t164's `ShellClassification` (14.7 / 15.3 s) and
   `SupportGeometry` + `SupportAnalysis` (68.9 s of 126.3 s) figures are
   pre-decimation. In particular, is `PrePass::InternalBridges` still worth
   anything after the `Internal Bridge` section itself fell **72.0 / 73.2 %** in
   path on the two base supports-off cells?
3. **S5 and S6 re-price.** t131 priced S5 at **21.0 s thread / 2.5 s wall** on
   base classic-off and S6 at **14,227.7 ms thread / 6,504 ms wall** on base
   classic-on. S5's operand is the internal-bridge offset path, whose downstream
   section already collapsed; S6's occupancy offsets run through the same
   decimated harvest. Neither number may be quoted again until it is re-measured
   on the current tree.
4. **Per-op primitive census.** Re-run the t131-style per-op split (sweep vs
   wrapper) or its cheaper `--profile` analogue, so the next engine question is
   asked about costs that survive canonical wrapper parity rather than costs
   decimation already removed.

## What is already known

- **t169 landed** the canonical `append_offset_path` harvest
  (`0.005 * |delta|`, both passes of `offset2_ex` with pass 2 at
  `|delta2| * 0.005`) and it is the currency.
- **`polygon_ops::offset` self wall (one `--profile` pass/cell, attribution
  only):** 40.3 / 110.3 / 39.3 / 115.2 s for base classic-off / classic-on /
  arachne-off / arachne-on, against 82.1 / 247.9 / 80.5 / 245.8 s before t169.
  `offset2_ex` fell 12.6–25.1 % on the same passes.
- **t163's board** is the last full two-tool scoreboard (ordinary/accelerated):
  benchy 1.782 / 2.802 / 1.940 / 2.829, base 1.389 / 2.710 / 1.457 / 2.618. It
  is pre-t169; t169's §9 route table is an **estimate** built from it, not a
  measurement of record. A fresh two-tool board is a separate question and is
  not in this ticket's scope.
- **Guest fuel is not uniformly lower:** it falls 13.5–21.5 % on the classic
  cells and rises 1.4–3.1 % on the three Arachne cells, so a re-price must not
  assume the guest half got cheaper everywhere.

## Scope

1. One instrumented window, three passes per cell (`--instrument-stderr`, the
   t164 harness is the template), all eight cells, every pass validated
   (exit 0, `degraded: false`, G-code hash equal to
   [`reference.json`](../evidence/t169-offset-decimation-flat-opening/reference.json)).
2. A `--profile` pass per cell per arm (or a cheaper per-op census) to attribute
   the surviving `polygon_ops` scopes.
3. Re-price S5 and S6 from the measured stage and op splits, with the arithmetic
   shown, and state for each whether the 0.80 % all-8-cell floor is still
   reachable.
4. A short list of shapes that survive, each with its measured carrier share —
   or the explicit finding that no operand-volume shape survives and the route
   must be re-aimed.

## Acceptance

- `evidence/t171-post-t169-reprice/` with the instrumented rows, the re-derived
  phase and stage splits, the S5/S6 re-price arithmetic, `FINDINGS.md`, and a
  checkout-only `verify.py --self-test` that re-derives every quoted number from
  the committed reduced set.
- Map: destination-status block, Decisions-so-far line, and an updated
  **Not yet specified** (either the surviving shapes with their new prices, or
  the re-aim).
- No production edit. This ticket prices; it does not build.

## Answer

**Resolved 2026-11-20: the split re-derives and the route re-aims. The
supports-off cluster is offset-bound and is t172's; the supports-on cluster is
**boolean**-bound (`clip_polygons` 53.1–60.9 % of host clipper thread-time) and
needs its own program, charted as [t173](173-support-geometry-boolean-volume.md);
S5 and S6 are landed keeps with no residual worth a ticket. No production edit,
no reference move.**

**Two premise corrections first.** (a) There is no `PrePass::InternalBridges`
stage on any cell and there was none at t164: the internal-bridge work runs inside
`PrePass::ShellClassification` (`commit_shell_classification_builtin` →
`compute_internal_bridge_layer`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`).
(b) **S5 and S6 are landed**, not candidates: `internal_bridge_offset_window` and
`inflate_model_occupancy_memoized` are in the production path since t133/t132, so
neither can be re-measured by ablation without reverting a keep; what can be
re-priced is what each is worth today and what it still costs after landing.

**Phase split** (median of three instrumented passes per cell; shares, never
walls):

| cell | prepass | per-layer | postpass | outside | Δprepass vs t164 |
| --- | --- | --- | --- | --- | --- |
| benchy classic off | 58.2 % | 32.4 % | 6.0 % | 3.3 % | −12.3 pp |
| benchy classic on | 71.7 % | 18.5 % | 8.3 % | 1.4 % | −5.4 pp |
| benchy arachne off | 55.9 % | 34.7 % | 6.2 % | 3.1 % | −11.1 pp |
| benchy arachne on | 70.0 % | 20.4 % | 8.1 % | 1.3 % | −5.5 pp |
| base classic off | 76.0 % | 18.6 % | 5.0 % | 0.5 % | −6.7 pp |
| base classic on | 83.7 % | 12.0 % | 4.2 % | 0.2 % | −3.5 pp |
| base arachne off | 74.4 % | 20.3 % | 4.8 % | 0.6 % | −6.2 pp |
| base arachne on | 83.1 % | 12.6 % | 4.0 % | 0.2 % | −3.8 pp |

The prepass share fell and the per-layer share rose on **all eight** cells, by an
amount that tracks each cell's t169 saving — the decimation cut the phase that
owned the offset work.

**Stage split, as "what fraction of the largest stage would have to go".** Required
cuts are t169's §9 estimate (3.9 / 10.3 / 52.8 / 50.8 % on the base cells, 39.0 /
45.3 / 57.9 / 58.5 % on benchy):

| cell | largest stage | share of cell | needs this much of it | ratio if the stage were free |
| --- | --- | --- | --- | --- |
| base classic off | `ShellClassification` | 44.0 % | **8.9 %** | 0.58x |
| base arachne off | `ShellClassification` | 42.7 % | **24.2 %** | 0.64x |
| base classic on | `SupportGeometry` | 54.9 % | **96.0 %** | 0.95x |
| base arachne on | `SupportGeometry` | 54.9 % | **92.4 %** | 0.92x |
| benchy classic off | `ShellClassification` | 34.1 % | **114.5 %** | 1.08x |
| benchy arachne off | `ShellClassification` | 32.2 % | **140.7 %** | 1.24x |
| benchy classic on | `SupportGeometry` | 40.5 % | **143.1 %** | 1.41x |
| benchy arachne on | `SupportGeometry` | 38.9 % | **150.3 %** | 1.47x |

So each **base** cell has a single sufficient stage — a *fraction* of one on the
supports-off pair (t172's cluster), nearly *all* of one on the supports-on pair —
and **no benchy cell has one**: the largest stage cannot cover its own cut, and
the benchy cells need combinations (top two leave 0.93x / 1.08x / 1.06x / 1.12x;
top three 0.83x / 0.97x / 0.90x / 0.96x). Zeroing the per-layer phase leaves
1.11x–1.94x on the four benchy cells; zeroing the postpass leaves 1.54x–2.22x.

**Per-op census (`--profile`, thread-time attribution only).** The surviving
clipper mass is a *different op* on each cluster:

| cluster | `offset` | `clip_polygons` | `offset2_ex` | `closing_ex` |
| --- | --- | --- | --- | --- |
| supports-off (4 cells) | **42.9–51.2 %** | 29.5–34.4 % | 16.1–18.3 % | 3.2–4.7 % |
| supports-on (4 cells) | 34.1–39.4 % | **53.1–60.9 %** | 4.2–6.0 % | 0.8–1.6 % |

Host clipper thread-time is 7.2–7.6 s on the benchy supports-off cells, 20.6–21.3 s
on the benchy supports-on cells, 82.5–88.8 s on the base supports-off cells and
**343.4 s** on both base supports-on cells. On base classic-on the owner split is
`host:native` **236.3 s** (169,586 calls: clip 186.6 s, offset 49.7 s),
`host:shell_classification` 53.9 s (100 % marked clipper work, `<module self>` =
0.000 s), `host:support_analysis` 35.7 s (offset 32.6 s), `host:slice` 12.7 s.
`host:native` is the tool's *unattributed* bucket; its 8.6x thread-to-wall ratio
fits `SupportGeometry` (27.6 s) and cannot fit the per-layer phase (5.94 s), so
the consistent reading is prepass-tier native work that no built-in bracket
claims — **a labelled inference, disclosed**, on which no number here depends.
Guest fuel agrees: the supports-on cells are a support-engine cluster
(`tree-support` 57.7–68.0 % of fuel), the supports-off cells the
perimeters/infill-linker cluster.

**S5 re-price.** t131's census rescaled by t169's measured `polygon_ops::offset`
ratio (×0.4905 / ×0.5901 / ×0.4451) and by t131's 8.34x stage parallelism: the
landed win is worth **9.01 %** of base classic-off's wall today (1,237 ms), the
residual windowed erosion **0.83 %** (950.3 ms thread) — at the 0.80 % floor and
0.41 % / 0.22 % on the other two cells. The window already harvests **91.6 %** of
the whole-layer cost, so no further localization of that mechanism can pay.

**S6 re-price.** The `+0.35 mm` occupancy family (864 calls / 433 distinct on
base classic-on, 365 / 186 on benchy classic-on) rescaled by ×0.4451 / ×0.5486 and
by 2.19x parallelism: the landed win is **5.86 %** of base classic-on's wall
(2,892 ms), the residual distinct-payload work **5.89 %** — **11 %** of that
cell's 52.8 % cut. And S6 is **inert on the four supports-off cells by
construction** (`enable_support: false` returns before either memo site), so it
can never satisfy an all-eight-cell gate at any price.

**What graduated.** [t173](173-support-geometry-boolean-volume.md) — attribute
and price the support path's boolean volume, the supports-on cluster's only
carrier. The benchy combination stays in the map's **Not yet specified** with its
numbers: no single stage covers a benchy cut.

Evidence: [`evidence/t171-post-t169-reprice/`](../evidence/t171-post-t169-reprice/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8 negative controls rejected).
