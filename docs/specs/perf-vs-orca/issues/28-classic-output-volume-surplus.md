# Classic output-volume surplus at matched settings

Type: task
Status: open

## Question

At the matched job on benchy, PNP classic emits 7.44 MB where PNP arachne
emits 4.71 MB (+58%) and Orca classic 3.89 MB (+91%); on base the surplus is
only +6–10% (`evidence/matched-pair/SCOREBOARD.md` output disclosure). Which
emission owns the surplus, and can it be reduced at identical intended
geometry?

The surplus is a work term, not just bytes: benchy arachne-off is the
output-matched pair (4.71 vs 4.68 MB, +0.5%) whose 6.68x gap is pure speed,
while the classic cells carry inflated CPU-per-output-byte on top of that
([Gap budget per cell](12-gap-budget-per-cell.md), §Work volume vs speed).
Per-tool section counts from the ticket-11 CSVs (`type_counts_json`) show
classic emitting 110 `Gap infill` sections where arachne emits none at ~2.8x
fewer wall sections — suggestive but not byte-attributed (cross-tool counts are
not comparable, and the measured G-code was scratch and is not retained).

Work:

- Re-run a minimal benchy pair (classic + arachne configs from
  `evidence/matched-pair/configs/`) and account bytes per `;TYPE:` section per
  tool; classify the surplus (gap-fill emission volume vs wall-segment density
  vs discretization/arc policy vs top/bottom solid area).
- Repeat on base (surplus only +6–10% there) to explain the fixture
  divergence — overlaps the budget's superlinearity question.
- State which surplus classes are semantically required (classic gap fill
  exists for thin gaps; arachne's width-variable walls legitimately differ)
  and which are emission-shape waste.

Disclosure/analysis only; no optimization authorized until the surplus is
classified. Parallel-takeable (not a timing/acceptance ticket). Any candidate
it produces touches output geometry and must pass the fairness contract's
disclosure rules before joining the timing chain.
