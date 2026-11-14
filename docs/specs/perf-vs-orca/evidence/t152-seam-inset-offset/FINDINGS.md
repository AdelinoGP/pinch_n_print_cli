# t152 — the seam inset offset is load-bearing, not redundant

**Verdict: S3 is not a clean work-set deletion.** Deleting
`seam-planner-default`'s prepass inset offset and letting the per-layer
`seam-placer` project the raw chosen point onto the wall loops moves the G-code
on **456 of 496 layers** of base classic-off. The offset is not an approximation
of a point the next stage re-projects anyway; it decides which loop segment and
which inserted seam vertex the next stage lands on. The measured offset term is
**1,248 ms** (stage **1,779 → 531 ms**, 70.15 %), so the shape is real — but its
canonical replacement is a **per-layer relocation of candidate generation**, not
a prepass-local deletion.

## Setup

The matched job resolves `seam_position = aligned`. The seam path has two
projections:

1. **Prepass** — `run_region_planning_entries`
   (`modules/core-modules/seam-planner-default/src/lib.rs`) picks a candidate on
   the raw region boundary, offsets every region by `-0.5 × width` in one
   `offset_polygons_batch`, and projects the chosen point onto that inset.
2. **Per-layer** — `run_wall_postprocess`
   (`modules/core-modules/seam-placer/src/lib.rs`) takes the injected
   `resolved_seam` and projects it onto the **actual wall loops**
   (`project_onto_wall_segment`), inserts the projected point when it is
   interior, and rotates the loop there. This is canonical `SeamPlacer`'s loop
   projection, already implemented.

The screen's candidate removes (1) and emits the raw chosen position; (2) is
untouched. The preserved patch is
[`candidate.patch`](candidate.patch). The source was restored after the screen;
the in-tree guest was rebuilt and `cargo xtask build-guests --check` exits 0.

## Output screen — base classic-off

| arm | SHA-256 | bytes | lines |
| --- | --- | ---: | ---: |
| baseline (current tree) | `9bb18f0e8173…` | 21,187,100 | 521,108 |
| candidate (no prepass offset) | `4265d619dbf1…` | 21,188,258 | 521,136 |

The baseline reproduces the **t151 active reference** for base classic-off
exactly. The candidate moves: **+1,158 bytes, +28 lines**. The first difference
is the layer-0 **Skirt** — the skirt hull's brim input inherits the wall loop's
start point, so a seam rotation propagates into the skirt ring's first line.

The layer census ([`census.json`](census.json), [`census.py`](census.py)) shows
this is not a pure loop rotation:

| field | measured |
| --- | ---: |
| layers | 496 |
| layers with a different printed-XY multiset | **456** |
| points added / removed | **+1,177 / −1,149** |
| layers whose total `E` changed | **142** |
| first moved layer / section | 1 / Skirt |

A pure rotation would keep each layer's point multiset and total `E`; this
changes both. The candidate inserts a different seam vertex (or none) on most
layers, so the loop's vertex set and path length move.

## Stage price

Both runs used `--instrument-stderr` on base classic-off, 12 threads
([`stage.json`](stage.json), [`stage.py`](stage.py)):

| arm | `PrePass::SeamPlanning` |
| --- | ---: |
| baseline | **1,779 ms** |
| candidate (offset deleted) | **531 ms** |
| offset term | **1,248 ms = 70.15 %** of the stage |

t129's 92.4 % share is **not reproduced at HEAD**. The residual 531 ms is
candidate generation, scoring and entry emission; it is what a relocation would
move, not what it would save.

Route arithmetic, indicative: 1,248 ms is **5.9x** the 0.80 % floor on the
26.254 s single-run cell (floor 210 ms), **4.75 %** of the cell wall, and
**10.5 %** of base classic-off's current required cut (45.27 % at the 1.827x
ratio of record). It is a tranche, not a destination step, and it is **not
bankable without a parity decision** because the output moves.

## Unit-test falsifier

The candidate was built and tested before the source was restored:

| arm | `cargo test --manifest-path modules/core-modules/seam-planner-default/Cargo.toml --lib` |
| --- | --- |
| baseline | **31 passed / 0 failed** |
| candidate | **29 passed / 2 failed** |

The two failures pin the offset as a contract, not an implementation detail:

- `tests::batched_inset_offsets_match_the_serial_reference_per_region` — the
  production entry is now the raw `(0, 0)` instead of the serial reference's
  inset `(0.2, 0.19999981)`; the t85 batching contract is broken.
- `tests::candidate_less_region_still_emits_one_keyed_plan_entry` — the
  fallback must project the first sharpest vertex onto the inset boundary; it
  now emits `(0, 0)`.

See [`unit-tests.json`](unit-tests.json).

## What the canonical replacement actually requires

Canonical `SeamPlacer::collect_points` builds candidates from the **printed
external-perimeter loop points**, and `place_seam` projects onto that loop. Those
loops do not exist in PnP's prepass; they are produced by the per-layer
perimeter modules. PnP's prepass candidate generation is on the **region
boundary**, and the per-layer `seam-placer` only projects the chosen point. So
matching canonical means moving candidate generation and scoring to the
per-layer phase, where the loops exist — a **stage relocation**, not a prepass
deletion.

The relocation's net is not the 1,248 ms. It is the offset term minus the moved
candidate-generation/scoring work (the 531 ms residual, or the part of it that
does not transfer) and minus any per-layer loop-collection cost. It also changes
computed geometry in the per-layer phase, which the map's out-of-scope section
says needs its own t81-style exception. The next step is a human scope decision,
graduated as
[Seam candidate generation on the printed loops: authorize the per-layer relocation?](../../issues/153-seam-loop-relocation-ruling.md).

## Disclosures

- **One box, one run per arm.** The wall numbers are single-run; no paired wall
  claim and no null band are taken from this screen.
- **No production change.** The candidate patch is preserved in this directory;
  the source and guest are restored to HEAD, and both freshness gates are green
  (`cargo xtask build-guests --check` exit 0).
- **The candidate is not more canonical by measurement.** Both arms project onto
  the wall loop; the candidate's source point is the raw boundary vertex, not a
  printed loop point. The screen tests redundancy, not parity.
- **The t151 reference is the currency.** The baseline hash is
  `reference.json`'s `base-classic-off`; no pre-t151 number is quoted.

## Files

- [`candidate.patch`](candidate.patch) — the screened deletion (not landed).
- [`census.py`](census.py) / [`census.json`](census.json) — layer census.
- [`stage.py`](stage.py) / [`stage.json`](stage.json) — stage extraction.
- [`unit-tests.json`](unit-tests.json) — the unit-test falsifier.
- [`rows.csv`](rows.csv) — the two harness rows.
- [`verify.py`](verify.py) — re-derives every claim; `--self-test` rejects 4/4
  mutations.
- [`REPRODUCE.md`](REPRODUCE.md) — the campaign.
