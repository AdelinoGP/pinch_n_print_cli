# t156 — Prepared-region memo preserved across the fill sync

Ticket: [`156-prepared-memo-across-fill-sync.md`](../../issues/156-prepared-memo-across-fill-sync.md)

**Verdict: DROP.** The candidate is exact and removes the redundant derivation
(process CPU is lower in all eight cells), but it fails the standing eight-cell
wall gate: **benchy Arachne-on regresses −2.94 %** and **base Arachne-on saves
only +0.17 %**. The other six cells clear the 0.80 % floor, base Arachne-off by
**+10.09 %**, but the stop rule admits no sub-floor cell. The production change
and its tests are reverted; the patch and the red test are preserved here.

## 1. What changed

`LayerArena` gains
`mutate_slice_preserving_prepared_regions` (`crates/slicer-runtime/src/blackboard.rs`),
an in-place slice-mutation seam that preserves `prepared_regions`,
`prepared_perimeter_source_regions` and
`prepared_regions_serve_perimeter_source`. It exposes the already-staged
`PerimeterIR` read-only in the same borrow, because the fill sync joins the two
staged IRs and must not clone the perimeter walls to do it.

`sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`) now uses that seam instead of
`take_slice` + `set_slice`. It reads `MODIFIER_FOOTPRINT_REGION_ID` before the
mutation; when a footprint is present it still runs
`split_modifier_footprints`, then calls `clear_prepared_regions` because that
path can mint/remove regions and reduce base `polygons`. The matched job has
zero footprints, so the clear never fires there.

The candidate added two contract tests to
`crates/slicer-runtime/tests/contract/region_preparation_sharing_tdd.rs`,
preserved in [`candidate.patch`](candidate.patch) and reverted with the
production change:

- `fill_only_sync_preserves_both_prepared_region_projections` pinned both
  prepared allocations across the public sync and checked the sync actually
  assigned sparse fill. It was **red on the pre-change tree**
  ([`red-test.log`](red-test.log)).
- `modifier_footprint_sync_clears_both_prepared_region_projections` pinned the
  invalidation path: the footprint is consumed, a modifier child is minted, and
  both derivations are gone.

## 2. Why it is exact

t155's sync audit hashed every field `prepare_slice_regions` reads before and
after the sync — region order/count, `region_id`, `object_id`, `nonplanar_surface`
presence and every polygon coordinate — and found **0 changed of 1,488** syncs
on base classic-off, base classic-on and the decline arm. The sync writes only
the four fill polygons (`bridge_areas`, `bottom_solid_fill`, `top_solid_fill`,
`sparse_infill_area`), and `prepare_regions`
(`crates/slicer-wasm-host/src/marshal/prepared.rs`) does not read them:
`derive_needs_support` reads `region.polygons` and the object overhang footprint,
the bbox and overhang-area terms read `region.polygons`, and
`perimeter_source_projection_is_identity`
(`crates/slicer-wasm-host/src/marshal/mod.rs`) reads only region ids. The
modifier split is the only sync branch that changes an input, and it clears.

## 3. Output gate — byte-identical in 8/8 cells

`-Mode output` ran one baseline and one candidate run per cell. Every run exited
0 with `degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0` and no
diagnostic event. Both arms emit the active t151 reference hash in every cell:

| cell | sha256 (both arms) | bytes | t151 reference |
| --- | --- | ---: | --- |
| benchy classic-off | `3d28d1b32f45…` | 4,085,138 | `3d28d1b3…` ✓ |
| benchy classic-on | `eb202973a1ec…` | 8,673,267 | `eb202973…` ✓ |
| benchy arachne-off | `39ad6289b681…` | 4,227,129 | `39ad6289…` ✓ |
| benchy arachne-on | `77c3c578aa28…` | 8,814,878 | `77c3c578…` ✓ |
| base classic-off | `9bb18f0e8173…` | 21,187,100 | `9bb18f0e…` ✓ |
| base classic-on | `eccbd5238d2d…` | 39,732,305 | `eccbd523…` ✓ |
| base arachne-off | `2f249cdb8fd2…` | 21,046,764 | `2f249cdb…` ✓ |
| base arachne-on | `1811425ffccd…` | 39,594,635 | `1811425f…` ✓ |

The reference set does not move, so there is no delta to audit.

## 4. Paired eight-cell gate

The first pass took **3 interleaved pairs per cell** on a quiet box (CPU load
0–4 %). That pass showed a high-variance cell — benchy classic-on's third pair
was **−2.46 %** while its first two were **+1.83 % / +1.56 %** and the output
arm was **+2.80 %** — so **two extra pairs were pre-declared for every cell
before pairs 4–5 ran**, not only for the failing one. That extension window
started at **85 % CPU load** and is **invalid as timing evidence**; its full
reduction is preserved as [`reduced/loaded-window.json`](reduced/loaded-window.json)
and is not used for the gate. The complete 5-pair gate was then re-run from
pair 1 on a quiet box; **all five pairs of that window enter the median**, no
pair is discarded. The same-arm null control is 3 pairs on three representative
base cells.

### Measured table (quiet 5-pair window)

Wall is uninstrumented process creation-to-exit; CPU is the process's
kernel+user total. The quiet window sampled **2 % CPU load before / 12 % after**
(`Win32_Processor` average; 6 physical / 12 logical cores).

| cell | pairs | baseline wall (s) | candidate wall (s) | saving | floor multiple | baseline CPU (s) | candidate CPU (s) | pairs faster |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 5 | 3.443 | 3.369 | **+2.146 %** | 2.68x | 18.80 | 17.75 | 4/5 |
| benchy classic-on | 5 | 7.245 | 6.959 | **+3.949 %** | 4.94x | 39.84 | 39.75 | 4/5 |
| benchy arachne-off | 5 | 3.529 | 3.415 | **+3.239 %** | 4.05x | 19.81 | 18.56 | 4/5 |
| benchy arachne-on | 5 | 7.311 | 7.526 | **−2.938 %** | **−3.67x** | 41.55 | 40.58 | 1/5 |
| base classic-off | 5 | 24.777 | 23.809 | **+3.905 %** | 4.88x | 206.39 | 182.97 | 5/5 |
| base classic-on | 5 | 74.714 | 71.073 | **+4.873 %** | 6.09x | 594.08 | 570.55 | 5/5 |
| base arachne-off | 5 | 25.951 | 23.333 | **+10.085 %** | 12.61x | 211.44 | 187.30 | 4/5 |
| base arachne-on | 5 | 74.394 | 74.265 | **+0.173 %** | **0.22x** | 601.39 | 577.36 | 2/5 |

**The wall gate is a DROP in two cells.** Bench Arachne-on is a real median
regression (four of five pairs slower), and base Arachne-on is +0.17 %, below
the 0.80 % floor. CPU is lower in all eight cells — including the two failing
cells — so the redundant work was removed, but the supports-on Arachne wall did
not follow it: the saved derivation was not on those cells' critical path.
CPU lower than baseline does not rescue a wall regression under the standing
gate.

### Null band

Same-arm control, three base cells × three pairs (18 runs):

| | wall | CPU |
| --- | ---: | ---: |
| null band (any-pair absolute) | **9.52 %** | **0.75 %** |
| null median (base classic-off / -on / arachne-off) | −0.15 / +2.33 / +0.15 % | −0.08 / +0.48 / +0.26 % |

The wall instrument is noisy: base classic-off's third null pair moved −9.52 %
with no arm difference. The measured wall effects are therefore reported
against this band, not as precise prices: base Arachne-off (+10.09 %) is the
only cell outside the null max, base classic-off (+3.91 %) and the other passing
cells sit inside it, and the two failing cells are consistent with "no wall
benefit / a small regression", not with a hidden win. The CPU band (0.75 %) is
tight; every cell's CPU median is lower than baseline.

### The invalid extension window

The first three-pair pass was quiet (0–4 % load) and is preserved in
[`reduced/loaded-window.json`](reduced/loaded-window.json) together with its
loaded extension (85 % load at start). That extension was discarded **before
the fresh gate** because the protocol requires an idle box; no loaded-window
pair is used in the table above. The fresh window was started from pair 1 and
all 40 runs are retained in `reduced/campaign.json`.

## 5. Disclosures

- **No accelerated arm.** The change is host-only and output neutral; the
  ordinary eight-cell set is the ticket's contract. No accelerated timing or
  output claim is made.
- **No tranche and no ratio of record.** The DROP leaves the destination at
  0/8 and the active scoreboard untouched; no production edit survives.
- **The pre-existing `blackboard_layer_arena_tdd` red is not caused here.**
  With the candidate fully reverted, `layer_arena_prepares_both_region_views_once_and_reuses_them`
  still fails in `assert_prepared_region_fields`
  (`crates/slicer-runtime/tests/unit/blackboard_layer_arena_tdd.rs`) on
  `overhang_areas.len()` (0 vs 1). The test/fixture predates this take and the
  production diff never touched `prepare_regions`; the isolated red is preserved
  in [`preexisting-blackboard-red.log`](preexisting-blackboard-red.log).
- **Final tree gates:** with the production change reverted,
  `cargo xtask build-guests --check` exits 0, `cargo xtask check-literals`
  reports 0 violations, `cargo xtask check-test-quality --report` reports 0
  findings, and `cargo clippy --workspace --all-targets -- -D warnings` is
  clean.
- Box load, pair count and the same-arm null band are recorded in
  `reduced/campaign.json` and the tables above, not assumed.

## 6. Reproduction

[`REPRODUCE.md`](REPRODUCE.md), [`run_t156.ps1`](run_t156.ps1),
[`reduce_t156.py`](reduce_t156.py), [`verify.py`](verify.py).
