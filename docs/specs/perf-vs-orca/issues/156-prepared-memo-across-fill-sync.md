# Prepared-region memo across the fill sync: does the redundant derivation pay for the 8-cell gate?

Type: task / measured candidate + standing gate
Label: wayfinder:task
Status: resolved (2026-11-15; **DROP** — exact and CPU-lower in 8/8, but the
  standing wall gate fails on benchy Arachne-on (−2.94 %) and base Arachne-on
  (+0.17 %); production change and tests reverted, patch preserved)
Assignee: 2026-11-15 OpenCode wayfinder session — the human said "Continue the perf vs orca map" (no ticket named; 156 was the only unblocked, unclaimed frontier ticket)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [The `Layer::Infill` first-dispatch block and base classic-on's
  1.096 s residual](155-infill-stage-block-base-on-residual.md) — t155 measured
  the block as `prepare_slice_regions` → `derive_needs_support` (**97.5 %** of
  the first-Infill bracket, p50 52.4 ms on base classic-off), paid **twice per
  layer** because `sync_perimeter_infill_areas_into_slice`
  (`crates/slicer-runtime/src/region_partition.rs`) takes and re-stages the
  slice after every perimeter commit (three times per layer) and each
  `set_slice` clears the arena's prepared slots. The fill sync was measured not
  to change any field `prepare_regions` reads (**0/1,488** changed read-field
  digests), and the stage transfer was re-proven by declining `raft-default`.

## Question

Does preserving the prepared-region memo across the fill-polygon sync — and
clearing it only on the modifier-footprint path — pass the standing eight-cell
gate, and does the measured wall land near the **2.38 s / 2.33 s** ceiling?

The candidate is the second derivation removal only. The first derivation (the
`Layer::Perimeters` side) stays; its value is delivered to both stages.

## Scope

1. **Shape.** Add a slice-mutation seam to `LayerArena`
   (`crates/slicer-runtime/src/blackboard.rs`) that mutates the staged slice in
   place and keeps `prepared_regions` / `prepared_regions_serve_perimeter_source`,
   documented as safe only for mutations that leave `region.polygons`,
   `region.nonplanar_surface`, `region.object_id` and the region order/count
   untouched. Have `sync_perimeter_infill_areas_into_slice` use it, and clear
   the memo explicitly when `split_modifier_footprints` runs (it can mint or
   remove regions and reduce `polygons`; the prepared vector is index-zipped
   with `slice.regions` by `assemble_layer_transport_for_stage`). The matched job
   has zero footprints, so the clear never fires on it.
2. **Guard the invariant with a test.** A regression test that a modifier
   footprint split still clears the memo (the stale-derivation rule), and that
   the fill-only sync preserves it. Prefer a failing-on-HEAD falsifier: assert
   the memo is populated after the sync on a staged two-region slice.
3. **Output identity.** The candidate must be byte-identical in **all eight**
   cells against the active reference set
   ([`evidence/t151-skirt-hull-contour/reference.json`](../evidence/t151-skirt-hull-contour/reference.json)),
   on the ordinary arm at minimum, plus the delta audit if any cell moves.
4. **Standing eight-cell gate.** Paired wall and CPU medians, interleaved
   baseline/candidate, against the same-run 0.80 % floor, in **all eight** cells
   (the four-cell planner-only form does not apply: this candidate is not
   planner-only and is not inert on supports-off). Record the measured null
   band, pair count, box load sample, and a CPU reading that is *not
   contradicted* when the null band makes "lower" unprovable.
5. **Price the realised saving** against the ceiling (base-off 2.38 s / 12.15x
   floor; base-on 2.33 s / 4.20x floor / 2.13x the 1.096 s requirement) and
   against the per-layer phase's share (36.5 % / 23.1 % of its thread). If the
   realised saving is far below the ceiling, attribute the gap or record it as
   unmeasured — do not restate the ceiling as a price.
6. **Stop rule.** If the gate fails in any cell, record DROP with the measured
   per-cell table and do **not** bank a sub-floor saving. If it passes, bank it
   as a tranche (the destination's 8/8 is unchanged) and re-derive the affected
   ratios of record.

## Acceptance

- Candidate tests: the memo is preserved across the fill sync and cleared on
  the footprint-split path; at least one of them is red on the pre-change tree.
- 8/8 G-code hashes equal the active set; every run exit 0, `degraded=false`,
  zero diagnostics.
- A paired eight-cell wall/CPU table with the measured null band and all pairs
  retained; floor multiples reported per cell.
- `cargo xtask build-guests --check` exits 0 after the change; `cargo clippy
  --workspace --all-targets -- -D warnings` and `cargo xtask check-literals`
  pass.
- `evidence/t156-prepared-memo/` holds `FINDINGS.md`, `REPRODUCE.md`, the
  candidate patch, the reduced tables and a `verify.py` that exits 0
  checkout-only and rejects its negative controls.
- The map's destination status and fog are updated; if the realised saving is
  below the floor in any cell, the ticket closes DROP and the fog keeps the
  cheaper-predicate shape.

## Evidence starts

- [`evidence/t155-infill-stage-block/`](../evidence/t155-infill-stage-block/FINDINGS.md)
  — site splits, the 0/1,488 sync audit, the predicate census and the pricing.
- `crates/slicer-runtime/src/blackboard.rs` — `LayerArena::set_slice`,
  `take_slice`, `clear_prepared_regions`, `ensure_prepared_regions`.
- `crates/slicer-runtime/src/region_partition.rs` —
  `sync_perimeter_infill_areas_into_slice`, `split_modifier_footprints`.
- `crates/slicer-wasm-host/src/marshal/prepared.rs` — `prepare_regions`
  (the derivation whose inputs the seam must preserve).
- `crates/slicer-wasm-host/src/marshal/transport.rs` —
  `assemble_layer_transport_for_stage` (index-zips `prepared` with
  `slice.regions`, which is why a region-list change must clear).

## Answer

**Resolved 2026-11-15: DROP. The candidate is exact and removes the redundant
derivation — process CPU is lower in all eight cells — but the standing wall
gate fails, so it is not banked.**

The candidate added
`LayerArena::mutate_slice_preserving_prepared_regions`
(`crates/slicer-runtime/src/blackboard.rs`) and routed
`sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`) through it instead of
`take_slice` + `set_slice`; the modifier-footprint branch still clears the memo.
Two contract tests pinned the preserved memo and the invalidation path; the
first is red on the pre-change tree. **Output is byte-identical in 8/8 cells**
and equal to t151's active set, with every run clean.

The quiet 5-pair paired gate is decisive:

| cell | wall saving | floor multiple | CPU median | pairs faster |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | +2.146 % | 2.68x | 18.80 → 17.75 s | 4/5 |
| benchy classic-on | +3.949 % | 4.94x | 39.84 → 39.75 s | 4/5 |
| benchy arachne-off | +3.239 % | 4.05x | 19.81 → 18.56 s | 4/5 |
| benchy arachne-on | **−2.938 %** | **−3.67x** | 41.55 → 40.58 s | 1/5 |
| base classic-off | +3.905 % | 4.88x | 206.39 → 182.97 s | 5/5 |
| base classic-on | +4.873 % | 6.09x | 594.08 → 570.55 s | 5/5 |
| base arachne-off | +10.085 % | 12.61x | 211.44 → 187.30 s | 4/5 |
| base arachne-on | **+0.173 %** | **0.22x** | 601.39 → 577.36 s | 2/5 |

The two failing cells are supports-on Arachne. CPU is lower there (the redundant
work is gone), but the wall does not follow: the saved derivation was not on
that cell's critical path. The same-arm null control (3 base cells × 3 pairs)
measures a wall band of **9.52 % any-pair absolute** (medians −0.15 / +2.33 /
0.15 %; CPU band 0.75 %), so the passing wall effects are small relative to the
instrument except base Arachne-off. The quiet window sampled 2 % load before /
12 % after. A 5-pair extension that began at 85 % load was invalidated and
preserved as `reduced/loaded-window.json`; it is not the gate.

**Disposition:** production code and tests are reverted to HEAD;
[`candidate.patch`](../evidence/t156-prepared-memo/candidate.patch) is preserved
and `git apply --check` succeeds. Final-tree gates pass: guest freshness exit 0,
`check-literals` 0 violations, `check-test-quality --report` 0 findings, and
`clippy --workspace --all-targets -- -D warnings` clean. No ratio of record, no
destination step, no accelerated arm. The map's fog keeps the cheaper
`needs_support` predicate as the unpriced survivor.

Evidence: [`evidence/t156-prepared-memo/`](../evidence/t156-prepared-memo/FINDINGS.md)
— `verify.py` PASS (DROP evidence, drop cells re-derived) with 6/6 negative
controls rejected; `red-test.log` preserves the pre-change red.
