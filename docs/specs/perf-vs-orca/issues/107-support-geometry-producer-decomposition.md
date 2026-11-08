# `PrePass::SupportGeometry`: producer decomposition and reduction pricing

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; measurement-only Phase-1 screen — **the host producer is
9 ms on base classic-on and 0 ms on benchy classic-on, 0.0068x and 0.00x the
0.80 % floor**, so no shape on it can be a route; the 91,902 ms / 40.1 % that
ticket 104 attributed to `PrePass::SupportGeometry` is the **planner module
stage** that shares the stage id, not the producer. Stop rule fired: no
candidate, no campaign, no production edit — see ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier held **no open child**, five
fog routes were offered, and the human selected **screen
`PrePass::SupportGeometry`**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 104](104-prepass-vs-per-layer-attribution.md)'s prepass
arithmetic (the stage is **91,902 ms = 40.1 %** of base classic-on and
**3,260 ms = 26.2 %** of benchy classic-on) plus the human's 2026-11-02 route
selection. The planner half of the same cells was attributed by
[ticket 88](88-supports-on-increment-attribution.md) (55.5 s classic) and stays
its own fog entry; this ticket does not re-open it.

## Question

**Where does the host `PrePass::SupportGeometry` producer's time go on the two
supports-on classic cells, and which representation-safe reduction shapes clear
the map's 0.80 % floor?**

Two premises have to be verified before anything is priced:

1. **The stage id is shared.** `SUPPORT_GEOMETRY_PRODUCER`
   (`crates/slicer-runtime/src/builtins/support_geometry_producer.rs`) runs as a
   host built-in under `PrePass::SupportGeometry`, and the scheduler's module
   stage for the support planner uses the *same* stage id (the
   `(PrePass::SupportGeometry, PrepassStageOutput::SupportPlan(_))` arm in
   `crates/slicer-runtime/src/prepass.rs`). The instrumentation keys
   `stage_start`/`stage_complete` on `(stage, layer)` only, so **two brackets
   can land under one name** and t104's reducer sums them. If that is what
   happened, t104's 91,902 ms is **producer + planner**, not one stage, and the
   map's table needs the correction. The planner half should land near t88's
   55.5 s.
2. **The producer's work is a deep copy of already-committed slice geometry.**
   `execute_support_geometry` (`crates/slicer-core/src/algos/support_geometry.rs`)
   walks every layer/region in the emit schedule, calls
   `collect_polygons_at_z` → `extract_region_polys`, and **clones** each
   region's polygons into `SupportGeometryIR.entries`. The guest consumer
   (`TreeVolumes::new`, `modules/core-modules/tree-support-planner/src/lib.rs`)
   then clones every outline again and unions/simplifies them. The question is
   how much of the producer's price is that clone, how much is the
   `add_intermediate_model_layers` sentinel-key work (which the planner's
   `layer_idx >= layer_count` guard discards), and how much is schedule/hash
   overhead.

Shapes to price (each must be exact — the take is output-preserving):

- **S1 — do not materialise the duplicate IR.** The guest already receives
  `layer_plan`, `region_segmentation`, `support_analysis`; it does **not**
  receive a slice view in `run_support_geometry`. Price removing the clone from
  the host producer (and what the guest would need instead), not the whole
  planner.
- **S2 — reuse the already-cloned polygons for the sentinel
  intermediate-layer entries** instead of re-collecting them from `SliceIR`.
- **S3 — cheaper per-entry construction** (reserve capacity, avoid a second
  clone on insert, `Vec::extend` from a borrowed slice) that is exact by
  construction.

The screen is a **Phase-1 screen**: counters plus in-place `Instant` windows on
one probe build, one instrumented pass per supports-on classic cell. If no shape
clears **0.80 %** of its cell, the ticket stops with no candidate and no
campaign (the t101/t99 pattern).

## Method (pre-declared; deviations disclosed in ## Answer)

- Binaries: `target/release/pnp_cli.exe` rebuilt from HEAD with a temporary
  probe patch; the patch is reverted before resolution and preserved as
  `probe.patch` in the evidence directory.
- Cells: `base-classic-on` (the 40.1 % cell) and `benchy-classic-on` (26.2 %).
  Supports-off cells are unaffected (`PrePass::SupportGeometry` = 231 ms
  base-off, 43 ms benchy-off): the emit schedule is empty without supports.
- Passes: one `--instrument-stderr` run per cell, ordinary mode,
  `RAYON_NUM_THREADS=12`, matched-pair configs, 24 external modules. Output must
  be byte-identical to the t87 post-t81 reference set
  (`22200671…` base classic-on, `4b…` as recorded in
  `evidence/t91-exact-z-memo/references.json`) and clean
  (`degraded=false`, `non_fatal_error_count=0`).
- Regimes: instrumented walls and probe windows are **attribution only** — never
  quoted as wall. Shares are converted to cell fractions with t106's ratios of
  record and t103's PnP medians; the floor is **0.80 % of the cell** (t91's
  walls, unchanged since t102 because the pin is Orca-side).
- Counter set: entries (main vs sentinel), polygons, points, cloned bytes,
  schedule size; the JSONL stage brackets are **paired per
  `stage_start`/`stage_complete`** rather than summed per stage id, so a shared
  id cannot silently double-count (this is the correction the take itself had to
  make — per-id summing *does* double-count).

## Stop rule

If every shape prices below the floor, close as **measured-dead / keep**: make no
candidate, run no campaign, revert the probe. If a shape clears the floor, name
the candidate and its pre-declared acceptance gate; the campaign is a separate
session, not this one.

## Answer

**Verdict: measured-dead — the premise was the wrong stage.** The screen needed
no new slice pass and no probe build; the decisive measurement is a re-pairing
of ticket 104's own committed raw streams, because **`PrePass::SupportGeometry`
is two stages that share one stage id** and t104's reducer summed them.

| cell | host producer | planner module stage | t104's sum |
| --- | ---: | ---: | ---: |
| benchy classic-off | 0 ms | 43 ms | 43 ms |
| benchy classic-on | 0 ms | 3,260 ms (26.17 % of cell) | 3,260 ms |
| base classic-off | 4 ms | 227 ms | 231 ms |
| base classic-on | **9 ms** | **91,893 ms** (40.13 % of cell) | 91,902 ms |

The producer is **0.0068x the 0.80 % floor** on base classic-on (9 ms against
1.318424 s) and **0.00x** on benchy classic-on (0 ms against 0.0992 s). The whole
stage's producer side cannot move a cell by a measurable amount, so **no
split, cache, batch or ownership shape on it can be a route** — the stop rule
fires immediately, before any candidate pricing. The `add_intermediate_model_layers`
sentinel entries are inside the same 0–9 ms bracket (exactly one on benchy-on),
so they are not a lever either.

Why the producer is cheap is measurable too: `pnp_cli support-preview` on benchy
classic-on renders the non-sentinel `SupportGeometryIR.entries` as **577
polygons / 444,259 points over 240 layers (~7.1 MB of heap)** — the IR is a coarse
outline subset at support layers, not a copy of the slice geometry — and a deep
copy of that is sub-millisecond. The producer's bracket is therefore real work,
not a skipped stage.

**Ledger correction (the take's real output).** Ticket 104's
`PrePass::SupportGeometry` figures are the **planner module stage**: base-on
91,893 ms splits `com.core.tree-support-planner` **87,824 ms (95.6 %)** +
`com.core.traditional-support-planner` 300 ms; benchy-on 3,260 ms splits 2,943 +
30. Implied uninstrumented wall: **64.5 s** of base classic-on's 160.857 s and
**3.25 s** of benchy classic-on's 12.403 s. That is the same term ticket 88
attributed at 55.5 s and the map already carries as the tree-support planner
fog entry ("support geometry, not a host batch shape"); the supports-on lever is
unchanged, and the producer was never part of it.

Two consequences for the map: the `PrePass::SupportGeometry` row must be read as
producer **plus** planner, and the supports-on side of the "prepass-rewrite
decision" is the **planner** (t88's term), not the geometry producer. The
supports-off cells make the split visible from the other direction: their
231 ms / 43 ms brackets are the planner merely declining (227 ms / 43 ms) with
the producer at 4 ms / 0 ms — cheap in both halves, and still not the producer's
doing.

**Method deviation, disclosed.** The ticket pre-declared a probe build and one
instrumented pass per cell. Those were not run: a probe patch inside
`execute_support_geometry` was authored and **reverted unbuilt** once the bracket
split answered the question, so no probe measurement is claimed, no probe patch
is preserved, and `target/release/pnp_cli.exe` remains the recorded `4cb96942…`
with a clean tree. The reducer's bracket pairing — not per-stage-id summing —
is the method that found the split; `verify.py` reconciles it against t104's
committed values (**71/71 claims, 9/9 negative controls**) and fails loudly if
the two-bracket shape ever changes.

Evidence:
[`evidence/t107-support-geometry-producer/`](../evidence/t107-support-geometry-producer/FINDINGS.md)
(`summary.json`, `preview-counts.json`, `reduce.py`, `count-preview.py`,
`verify.py`; checkout-only verify exits 0).
