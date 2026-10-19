# Classic output-volume surplus at matched settings

Type: task
Status: resolved
Assignee: wayfinder session (ses_f14ba59b7ffeteaZ0tDfSNEjpc), 2026-09-29

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

## Answer

**Resolved 2026-09-29: Benchy's classic surplus is mostly actual sparse
extrusion; base's is mostly wall-section bytes, with a separate cross-tool
serialization cost.** Fresh matched PNP supports-off classic/Arachne slices
were measured on both fixtures (ordinary guests checked fresh; release host
rebuilt). The Benchy classic file exceeds PNP Arachne by 2,738,863 bytes;
2,481,329 bytes (90.6%) are labelled `Sparse infill`, with 106,442 versus
47,659 printed XY segments. At layer 104 (Z 21 mm), classic prints 2,562
sparse segments while Arachne prints one and the retained matched Orca
classic/Arachne captures print none. This **does not prove** which fill claim
is correct. Gap infill contributes only 30,140 bytes to the Benchy PNP delta;
it is a legitimate classic feature, not the dominant surplus.

On base, PNP classic is 941,196 bytes larger than PNP Arachne: outer+inner
wall sections add 1,370,922 bytes, while sparse infill is 463,583 bytes
*smaller*. This is a different fixture shape, not a universal gap-fill tax.
Across tools, PNP writes Z and F on every XY+E move; the historical Orca
captures omit most repeated modal tokens. That is demonstrable formatting
redundancy, but no safe-removal count or wall/CPU benefit has been measured.

Methods, raw-capture paths, full role-bytes JSON and per-layer CSVs:
[`FINDINGS.md`](../evidence/t28-output-volume/FINDINGS.md). The four fresh
captures completed without degraded/non-fatal events, but had existing
`ERR_MALFORMED_LAYER_MARKER` warnings; Orca comparison files are retained
historical captures, not a same-day rerun. No optimization, acceptance A/B or
keep/drop verdict was made. Follow-ups are [Benchy classic sparse-fill domain
at middle layers](35-benchy-classic-sparse-fill-domain.md) (real-layer IR and
canonical role oracle) and [Modal G-code Z/F token redundancy and speed
gate](36-modal-gcode-token-redundancy.md) (state semantics before a paired A/B).
