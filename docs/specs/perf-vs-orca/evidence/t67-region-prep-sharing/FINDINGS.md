# Region-preparation derivation sharing (t67)

**Resolution in one line:** measured GATE NOT MET — the wall (destination)
clause passes in all four pre-declared cells (paired medians −0.11 %,
−4.26 %, −0.50 %, −0.28 %) with the `ordinary-base-classic-on` win
corroborated by CPU (−0.60 %), but the CPU corroboration clause fails in
`accelerated-benchy-classic-on` by +0.19 % (+0.31 s CPU on a 118 s run).
**Awaiting the human's KEEP/DROP ruling (2026-10-16).**

Route: [Region-preparation derivation sharing](../../issues/67-region-preparation-derivation-sharing.md).

## Candidate

`LayerArena` serves the perimeter and infill region projections from one
prepared-region derivation when the perimeter source projection onto the
regions is the identity (`perimeter_source_projection_is_identity`,
`crates/slicer-wasm-host/src/marshal/mod.rs`); when a modifier sub-region
restores base geometry the two derivations stay separate. No second cache,
no clone. Uncommitted, held in `stash@{0}` pending the ruling; tests:
`crates/slicer-runtime/tests/contract/region_preparation_sharing_tdd.rs`
(5 tests) plus 1 marshal unit test, mutation check (forcing the condition
false) fails 3 of 5.

## Qualification (output identity, 16 cells)

Proof phase: every cell ran both arms once; per-cell outputs byte-identical.
`build-qualification.py` → `qualified cells: 16/16; pass=True`;
`verify-references.py` → `reference-checked cells: 16/16; pass=True` (every
hash equals its preserved t58 reference). Sealed as `qualification.json`
(this directory).

## Standing paired gate (pre-declared cells)

56 timed runs (4 cells × 14: 2 warm-ups + 6 alternating pairs); all four
cells retained 6/6 pairs with zero starvation exclusions; every measured run
byte-identical within its cell. Reduced by `reduce_ab.py` →
`ab-reduction.json`:

| cell | baseline wall med | candidate wall med | paired-median Δwall | Δcpu | pairs |
|---|---|---|---|---|---|
| ordinary-benchy-classic-on | 29.066 s | 29.426 s | −0.11 % | −0.44 % | 6/6 |
| ordinary-base-classic-on | 565.441 s | 542.123 s | **−4.26 %** | −0.60 % | 6/6 |
| accelerated-benchy-classic-on | 28.779 s | 28.455 s | −0.50 % | **+0.19 %** | 6/6 |
| accelerated-base-classic-on | 452.481 s | 453.616 s | −0.28 % | −0.11 % | 6/6 |

**VERDICT: GATE NOT MET** (wall clause passes in all four cells; CPU
corroboration fails in accelerated-benchy).

Read plainly: the only delta outside run noise is `ordinary-base-classic-on`
— −4.26 % paired (−4.12 % unpaired), 4 of 6 pairs winning by 2–9 %, CPU
corroborated (−0.60 % ≈ −14 s), consistent with t60's attributed duplicate
derivation (~40 s thread-time per slot per base slice). On the other three
cells the effect is inside the design's noise: the two ±0.3 % cells
(ordinary-benchy, accelerated-base) flip sign between paired and unpaired
medians; the benchy cells were pre-declared at/below the 0.1–0.5 % floor.
The single clause miss is +0.31 s of CPU on the 118 s accelerated-benchy
slice.

## Limits and disclosure

- No background-load snapshot accompanies this campaign (none was taken at
  measure start; the measure window was 20:28–00:41 local). The paired
  alternating design is the control; cross-campaign absolute walls should be
  read accordingly.
- Cross-campaign medians differ: this campaign's ordinary base-classic-on
  baseline median is 565.4 s versus 487.1 s in t66's campaign. The gate is
  the paired comparison within this campaign, never across campaigns.
- Statistic sensitivity: `ordinary-benchy-classic-on` and
  `accelerated-base-classic-on` are negative under the pre-declared paired
  median but positive under unpaired medians (±0.3 %); both are inside noise.
- Proof-phase and warm-up walls are not part of the gate and no inference is
  drawn from them.

Snapshots (`pnp_cli.exe` sha256: `hashes-baseline.txt` / `hashes-candidate.txt`
under the ignored raw tree): ordinary-baseline `f31fe45d…`,
ordinary-candidate `b82f67e4…`, accelerated-baseline `ad575398…`,
accelerated-candidate `109e5765…`.

## Ruling

Pending (2026-10-16): the measured recommendation — including the question of
whether the +0.19 % CPU miss is a strict drop under the t66 precedent or a
noise-scale override — is with the human.
