# t155 — The first `Layer::Infill` dispatch block is the prepared-region derivation, paid twice

Ticket: [`155-infill-stage-block-base-on-residual.md`](../../issues/155-infill-stage-block-base-on-residual.md)

**Headline: the ~63 ms block is `prepare_slice_regions` →
`derive_needs_support` (94 %), not marshalling, not the instance pool, and not
the runner.** It is paid **twice per layer**: `Layer::Perimeters` derives it,
then `sync_perimeter_infill_areas_into_slice` takes and re-stages the slice (up
to three times, once per perimeter commit) and each `set_slice` clears the
arena's prepared-region memo; the first `Layer::Infill` module re-derives
**byte-identical** records. The fill sync is measured not to change any field
the derivation reads (**0 of 1,488 syncs** on every probed cell), so the second
derivation is redundant work whose ceiling is **2.38 s** on base classic-off
(**12.2x** the 0.80 % floor) and **2.33 s** on base classic-on (**4.2x** the
floor, **2.13x** the 1.096 s the destination still needs on that cell).

The proposed candidate is to preserve the prepared-region memo across the
fill-polygon sync, clearing it only on the modifier-footprint path where the
region list can change. It is a proposal with a ceiling, not an adopted change;
the standing gate is a separate ticket.

## 1. The arm and the gates

- HEAD `d94a88bd` (`feature/perf-vs-orca`), tree restored after the take (the
  probe patch is preserved, not applied).
- Pre-probe binary `e1b8fa69…` (the t154 build). Probe builds:
  `bcb75fb4…` (v1: site split), `23462477…` (v2: + `prepare_regions`
  sub-timers, sync audit, raft decline gate), `50418d2e…` (v3: +
  `derive_needs_support` census). Restored clean binary `ef3f5e51…`.
- `cargo xtask build-guests --check` exit **0** after restoration, and the
  restored binary reproduces t151's active set on base classic-off
  (`9bb18f0e…`).
- **Eight probe passes**, all exit 0, `degraded=false`,
  `fatal_error_count=0`, `non_fatal_error_count=0`, every G-code SHA-256 equal
  to t151's active set (`reference.json`): base-off × 4 (+ one decline arm) and
  base-on × 3. `RAYON_NUM_THREADS=12`, the standing matched-pair configs,
  24 external core modules.
- Instrumented walls are attribution only and are never wall claims. The box's
  load was **not sampled** (disclosed); run-to-run spread is visible in the
  committed `runs` lists: the pre-probe pass ran at slice 31,964 ms / per-layer
  8,678 ms against the later passes' 21,863–22,582 ms / 6,201–6,532 ms.

## 2. What the block is

The probe splits each module bracket (`module_start` → `module_complete`) into
host sites. On both base cells the whole block lands on exactly one site —
`prepare_dispatch_region_views`, i.e. `ensure_prepared_regions` /
`ensure_prepared_perimeter_source_regions` — and only on the **first** module
dispatched in a stage:

| cell | first Perimeters dispatch (`classic-perimeters`) | first Infill dispatch (`raft-default`) | every other Infill module | every other stage's first module |
| --- | ---: | ---: | ---: | ---: |
| base classic-off | **26,044.6 ms** (p50 53.0 ms) | **25,986.3 ms** (p50 52.4 ms) | 3.0–4.0 ms total each | 3.1–3.8 ms total each |
| base classic-on | **26,340.9 ms** (p50 53.5 ms) | **26,101.2 ms** (p50 53.3 ms) | 2.9–3.2 ms total each | 2.9–3.4 ms total each |

Sums are thread-accumulated over 496 calls per module. The memo is real: after
the first dispatch of the stage, every later module reads the prepared slots in
**5 µs**.

The bracket's own host plumbing is falsified as the block. For `raft-default`
on base classic-off (p50 µs, 496 calls):

| site | p50 µs | sum ms |
| --- | ---: | ---: |
| `prep_views` (derivation) | **52,364** | **25,986.3** |
| `input_ctor` (`LayerStageInput` clones) | 5 | 4.1 |
| `run_stage` (total) | 1,257 | 661.5 |
| — `rs_selection` (target/config maps) | 13 | 8.4 |
| — `rs_transport` (`assemble_layer_transport_for_stage`) | 109 | 68.9 |
| — `rs_envelope` | 6 | 4.6 |
| — `rs_dispatch` (linker→call→commit) | 1,080 | 562.7 |
| — `dl_acquire` (pool lease) | 5 | 4.3 |
| — `dl_linker` | 196 | 98.6 |
| — `dl_instantiate` (component instantiation) | 83 | 47.4 |
| — `dl_push_regions` (region marshal) | 58 | 32.6 |
| — `dl_call` (typed export) | 593 | 309.6 |
| — `dl_return` (profile stash + teardown) | 25 | 14.2 |

The derivation is **97.5 %** of the bracket; the entire dispatch path —
linker, instantiation, region marshalling, instance-pool lease and the export
call — is **~0.95 ms/p50 (1.8 %)**. The base classic-on bracket is the same
shape (97.3 % derivation; `dl_call` p50 801 µs).

## 3. Why it is paid twice

`LayerArena::set_slice` calls `clear_prepared_regions`; so does `take_slice`.
`sync_perimeter_infill_areas_into_slice` takes the slice, writes the four
canonical fill polygons, optionally splits modifier footprints, and re-stages —
so every perimeter commit invalidates the memo. On the matched job that happens
**three times per layer**: once after `Layer::Perimeters` and once per
`PerimetersPostProcess` commit (`fuzzy-skin`, `seam-placer`). `Layer::Infill`
runs next and its first module re-derives the same records. The stage transfer
is re-proven at HEAD, not carried from t127: declining `raft-default` with the
probe-only gate moves the whole block to `gyroid-infill` (**24,533.1 ms**,
p50 50.0 ms) while every other Infill module stays at 3.1–3.5 ms total, and the
output hash does not move (`9bb18f0e…`).

**The derivation's inputs survive the sync.** The probe hashes, before and
after the sync, the fields `prepare_regions` reads — region count and order,
`region_id`, `object_id` length, `nonplanar_surface` presence, and every
`polygons` coordinate (contour and holes): **0 of 1,488 syncs changed any of
them** on base classic-off, base classic-on and the decline arm (1,488 = 3 ×
496; polygon points p50 5,656 per region set). `prepare_regions` does not read
the fill fields the sync writes; `object_id` *content* is not hashed (the sync
never writes it — code-verified).

## 4. What the derivation spends

`prepare_regions` is `derive_needs_support` almost entirely (sums over the 992
per-layer calls = two derivations × 496 layers):

| segment | base classic-off | base classic-on |
| --- | ---: | ---: |
| `SliceRegionView::from_ir` + `derive_needs_support` | **49,021.2 ms (94.3 %)** | **49,465.0 ms (94.4 %)** |
| bbox + overhang-quartile intersection | 19.8 ms | 20.6 ms |
| overhang-areas intersection | 2,919.8 ms | 2,887.7 ms |
| `prev_layer_boundary` clone | 30.3 ms | 27.0 ms |
| `surface_group` resolve | 7.4 ms | 6.3 ms |
| record build | 11.9 ms | 17.5 ms |

`derive_needs_support` loops the object's non-empty overhang footprints and
returns `true` on the first non-empty intersection. The census is unambiguous:
**one** non-empty footprint per object on every layer (max 1, 992/992 and
1,488/1,488 calls), **all-true** (0 false answers), and the single
`intersection_ex(region.polygons, footprint)` p50 is **48.5–49.1 ms**. On base
classic-on the predicate is evaluated a **third** time per layer (1,488 calls =
3 × 496; 72,811.8 ms total, of which 49,465.0 ms is the 992 per-layer calls and
**23,346.8 ms** the 496 prepass calls) by the prepass eligibility pass in
`support_analysis_producer.rs`; supports-off has no third call (no support
analysis).

The value is delivered, not dead: the guest adapter
`__slicer_adapt_slice_regions` (`crates/slicer-macros/src/lib.rs`) calls
`needs-support()` for every region it adapts, so the bool must be present in
`SliceRegionData` on every layer. That is why the candidate below is *memo
preservation* (exact) rather than removing the field.

## 5. `Layer::Support` is separated, and is not a host block

The ticket asked for the tree-support bracket to be split if the Infill block
turned out necessary. It is not, but the split was taken anyway:
`tree-support`'s `prep_views` is **5 µs** (memo hit) on both cells. On base
classic-on its p50 bracket is 85.9 ms, of which `rs_dispatch` is 85.6 ms and
`dl_return` (linker→instantiate→push→call→teardown, the render) is 85.4 ms;
sum 41,396 ms thread. It is **in-module render work**, not a host-side block,
and is a work-set/phase difference already recorded by t154.

## 6. Pricing

Ceilings are `thread_sum / effective parallelism` (t154's measured 10.91x and
11.18x) and charge 100 % of the parallel work to the critical path. Walls of
record are t154's (base-off 24.51 s vs Orca 13.41 s; base-on 69.54 s vs Orca
21.598 s); the 0.80 % floor is 0.008 × the PnP wall of record.

| | base classic-off | base classic-on |
| --- | ---: | ---: |
| redundant Infill-side derivation (thread) | 25,986.3 ms | 26,101.2 ms |
| first Perimeters-side derivation (thread, retained) | 26,044.6 ms | 26,340.9 ms |
| share of the per-layer phase's thread | 36.5 % | 23.1 % |
| **ceiling** (`/10.91x`, `/11.18x`) | **2.382 s** | **2.335 s** |
| 0.80 % floor | 0.1961 s | 0.5563 s |
| floor multiple | **12.15x** | **4.20x** |
| share of the cell's PnP wall of record | 9.72 % | 3.36 % |
| share of the cell's required wall cut | 21.5 % (of 11.10 s) | 4.9 % (of 47.94 s) |
| the destination's per-layer requirement | — | **1.096 s** |
| ceiling over that requirement | — | **2.13x** |

Indicatively (a chain, not a claim), subtracting the ceiling from t154's
canonical-prepass projection moves base classic-on from **1.051x to ~0.943x** —
the only losing cell in that arithmetic would clear the line. That composition
is an estimate; the paired gate decides.

**Ledger note.** The ticket pre-declared base classic-off's bracket ceiling as
**2.86 s / 14.6x** from t154's 30,089 ms bracket at 10.91x. This take's bracket is
26,652 ms on a faster box state (slice 22.3 s against the pre-probe pass's
32.0 s), and the removable share is 97.5 % of it — so the ceiling here is
**2.38 s / 12.15x**, 83 % of the pre-declared figure. Both are ceilings from
different runs; neither is a price.

## 7. The proposed candidate (not adopted)

**Preserve the prepared-region memo across the fill-polygon sync.** Add a
slice-mutation seam to the arena that keeps `prepared_regions` (and the
perimeter-source sharing flag) when the mutation cannot change the
derivation's inputs, and have `sync_perimeter_infill_areas_into_slice` use it;
clear the memo explicitly when the modifier-footprint split runs, because that
path can mint/remove regions and change `polygons` (the prepared vector is
index-zipped with `slice.regions` by
`assemble_layer_transport_for_stage`). On the matched job there are **zero**
footprints, so the clear never fires.

- **Exactness:** the derivation is pure over the fields the audit hashed; the
  sync writes only the four fill polygons. Measured 0/1,488 changed read-field
  digests. No G-code can move.
- **Expected saving:** exactly one derivation per layer — the Infill-side
  25,986.3 / 26,101.2 ms thread above, i.e. the ceilings in §6.
- **Gate:** the standing eight-cell gate (0.80 % paired wall/CPU improvement and
  byte-identical output in all eight) is a **separate ticket**; the four-cell
  planner-only form does not apply (this candidate is not planner-only and is
  not inert on supports-off).

A second, larger shape is **not** part of the proposal and goes to the map's
fog: the remaining 94 % of the derivation is one full-polygon
`intersection_ex` per layer whose answer was true in every measured call. A
cheaper exact predicate, or computing the value once per `(layer, region)`
across the prepass eligibility pass and the per-layer stages, would remove both
remaining derivations — but the prize and the exactness argument change
depending on this candidate's outcome, so it is not ticketed here.

## 8. What this take does not claim

- No wall claim from any probe or instrumented pass; the ceilings are
  upper bounds.
- No ratio of record and no scoreboard re-derivation.
- No load sample and no accelerated arm.
- No exactness argument for the second shape (a cheaper `needs_support`); its
  94 % share is measured, its replacement is not designed.
- No cause claim for the t154→t155 p50 movement (63 ms pre-probe against
  52.4 ms here): the pre-probe box state was slower across every phase.
- No claim about the prepass eligibility pass beyond its call count and thread
  sum.

## 9. Reproduction

[`REPRODUCE.md`](REPRODUCE.md). The committed set is `probe.json` (eight runs,
site splits, segments, census and audit), `summary.json` (the pricing table),
`probe.patch` (the five-file temporary probe, reverted), `reduce.py`,
`price.py`, `run_probe.py`, and `verify.py` (checkout-only; `--self-test`
rejects 9/9 negative controls).
