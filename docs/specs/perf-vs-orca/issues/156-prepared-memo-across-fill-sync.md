# Prepared-region memo across the fill sync: does the redundant derivation pay for the 8-cell gate?

Type: task / measured candidate + standing gate
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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
