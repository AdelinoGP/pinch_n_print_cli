# t124 — post-t123 Orca scoreboard

Ticket: [Post-t123 Orca scoreboard](../../issues/124-post-t123-orca-scoreboard.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-07, the only takeable AFK frontier child at HEAD `6dd88675`
(t117 is the human's grilling). Measurement-only: no production edit.

## Verdict

**The post-t123 scoreboard is re-derived: PnP still loses all eight matched
cells, now by 2.36x–4.65x (ordinary) and 2.40x–4.67x (accelerated).** Ticket
123 moved the PnP side substantially — the base supports-off cells are ~24–26%
cheaper than t121's scoreboard — but the destination's 8/8 is not met, and the
required cut to beat Orca is **58%–79%** of today's PnP wall (**72%–86%** for a
1.5x margin).

## Method

The standing matched-pair rig
([`../matched-pair/run_scoreboard.ps1`](../matched-pair/run_scoreboard.ps1))
ran 8 cells × 3 tools × (1 warmup + 3 measured), Orca and PnP interleaved inside
each cell, with the t103 pins and the t105 `tree_strong` support-style pin
asserted per Orca run. Both PnP arms are the current tree's binaries and hash
exactly to t123's candidate artifacts:

| arm | provenance | sha256 |
| --- | --- | --- |
| ordinary | `cargo build --release -p pnp-cli` on the current tree | `7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0` |
| accelerated | `cargo xtask dist --accelerated --edition developer` on the current tree | `a6ae2adc998572217664a124ac0ac62c58977cfd46804379dcf5153fbf53e41b` |

`git diff c598f098..HEAD` touches docs only, so those artifacts are the
committed tree's. Guest freshness: `cargo xtask build-guests --check` and
`--accelerated --check` both exit 0; the Orca vendor tree's declared
`02.08.00.16` equals the installed `.opc` stamp `2.8.0.16`.

**Output gates:** 96 rows; PnP ordinary ≡ accelerated bytes in 8/8; every PnP
SHA equals the t123 candidate set; 64/64 PnP runs `degraded=false`,
`non_fatal=0`; 32/32 Orca runs resolve the three pins and the repaired filament
leg. **No starvation exclusions** (all 24 groups retain n=3).

## Results

Full table and disclosures: [`SCOREBOARD.md`](SCOREBOARD.md). Headline rows:

| cell | ordinary / Orca | accelerated / Orca | required cut to beat |
| --- | ---: | ---: | ---: |
| benchy classic-off | 2.70x | 2.46x | 63% |
| benchy classic-on | 3.61x | 3.52x | 72% |
| benchy arachne-off | 2.50x | 2.52x | 60% |
| benchy arachne-on | 3.59x | 3.47x | 72% |
| base classic-off | 2.95x | 2.84x | 66% |
| base classic-on | 4.63x | 4.45x | 78% |
| base arachne-off | 2.36x | 2.40x | 58% |
| base arachne-on | 4.65x | 4.67x | 79% |

**Against t121's pre-t123 scoreboard (2.78x–5.11x), every cell's ratio
improved.** The improvement is a mix of both arms: PnP ordinary medians moved
−25.7% … +0.7%, and the Orca medians moved −12.3% … +11.5% across the same
windows. The base supports-off PnP deltas (−24.2% / −25.7%) agree with t123's
paired A/B (−22.2% / −20.9%); the benchy and base-on cells are the noisier
windows. **No causal claim is made from the cross-take comparison** — t123's
interleaved A/B is the causal measurement, and this table is the ratio of
record.

## Disclosures

- **The first batch was killed by the host mid-cell.** `t124-post-t123` ran
  six complete cells plus the three warmups and the first two measured rounds
  of `base classic-on` before the process disappeared at 13:14 local (no
  `RIG_EXIT`, no live slicer process — consistent with the box sleeping). The
  partial cell is discarded whole and named in
  [`scoreboard-summary.json`](scoreboard-summary.json)
  (`discarded_partial_sources`); the two remaining cells were taken in their
  own interleaved batches. Each cell is internally paired, so no ratio spans
  windows.
- **Load.** The shared dev box had the owner's browser processes active
  (pre-campaign load sample 13%). The schedule interleaves arms and the
  starvation rule is the control; no exclusion was needed. Not an idle-box
  claim.
- **Adhesion asymmetry.** PnP prints its 8 mm skirt-brim default in every cell;
  pinned-back Orca prints none. The jobs are not identical; the price was
  measured at ~235 ms (t103) and closing the asymmetry is a separate human
  decision.
- **Support style is a run-time assertion, not a CSV column.** The rig asserts
  `support_style = tree_strong` on supports-on and `default` on supports-off
  and aborts on mismatch; the committed CSV cannot re-check it.
- **No per-stage attribution is claimed.** The take measures end-to-end
  process wall/CPU only.

## Route implication

The destination (8/8) is unchanged and still far: the prepass campaign
(t111–t123) moved the base supports-off cells by ~22–26% but left the required
cut at 58%–79%. The map's **Not yet specified** entry for the per-layer/emit
program remains the next stage — t104's counterfactual says deleting the entire
per-layer phase still leaves 3.02x–7.60x. No ratio here changes that arithmetic.

## Evidence index

- [`SCOREBOARD.md`](SCOREBOARD.md) — ratios, proof gates, cuts, disclosures.
- [`scoreboard.csv`](scoreboard.csv) — 96 rows, merged from three batches.
- [`scoreboard-summary.json`](scoreboard-summary.json) — medians, retained
  counts, ratios, discarded partial source.
- [`scoreboard.txt`](scoreboard.txt) — per-sample detail and gap table.
- [`reduce_t124.py`](reduce_t124.py), [`verify.py`](verify.py) — reducer and
  checkout-only verifier (`--self-test` rejects 10/10 controls).
- [`REPRODUCE.md`](REPRODUCE.md) — rebuild and re-run commands.
