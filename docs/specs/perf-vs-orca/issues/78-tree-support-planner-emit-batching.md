# Tree-support planner emit-pass batched carve and role union

Type: task
Label: wayfinder:task
Status: resolved (2026-10-22; human-approved KEEP — candidate committed `73a6f1e8`; proof 16/16; standing gate PASS in all four classic supports-on cells; see ## Comments)
Assignee: 2026-10-22 OpenCode wayfinder session — human selected "continue" (only frontier ticket)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Can the planner's emit pass buffer each layer's role inputs and issue its
per-region carves and per-role unions through `clip_polygons_batch` — keeping
every per-call result identical — and does the resulting wall win pass the
standing paired ordinary + accelerated gate?

Measured reason ([t77](../issues/77-tree-support-planner-batch-parallel-contract.md),
[findings](../evidence/t77-planner-contract/FINDINGS.md)): on base classic
supports-on at HEAD, `com.core.tree-support-planner` is **126,674 ms** of the
147,040 ms `PrePass::SupportGeometry` stage, and **78.885 s (99.03 %) of its
79.659 s clipper core sits inside `build_roles`** — 472 role unions (median
79.2 ms, mean 312 input polygons) plus 69,161 per-region carves (median
0.474 ms). Both are independent per item and serial today only because the
guest calls the singular import inline.

Why this is exact by construction, not by tolerance: each batch item is the same
single clipper operation with the same arguments as the serial path.
`crate::batch::map_batch` returns the i-th result for the i-th request and
byte-compares its serial and parallel paths; t65's batched linker is the
precedent (12,022–19,290 subjects per cell, outputs byte-identical). **Out of
scope**, because t77 measured them dead: partitioning a single union (t76's
Morton shape — no call is large enough and the output is printed geometry) and
parallelizing the `layer_outlines_below` prefix chain (0.13 % of the bucket).

## Work

1. Restructure the emit pass so the per-layer role inputs are collected first
   and the expensive phases become batches: per-region carve, role union,
   set-wide difference, cross-role difference. Chunk the buffering (a bounded
   number of layers at a time) so a 431-layer job does not materialize every
   layer's polygons at once.
2. Keep the ordering and role-assembly semantics exactly as they are: a batch
   result must be the value the serial call returns today, in input order. The
   t32 bounding-box gate and the largest-part `max_by` selection stay.
3. Prove it before timing: the 16-cell output-identity matrix against the
   preserved t58 references, byte-identical in both modes.
4. Then run the standing paired ordinary + accelerated A/B on the pre-declared
   cells and return KEEP/DROP to the human. No auto-commit, no default switch.

Pre-declaration is the human's at take time; the natural cells are the four
classic supports-on cells, where the planner's cost lives, with the idealized
ceiling (≈ 72.3 s of a 147.0 s stage, arithmetic and unmeasured) as the upper
bound on any claim.

## Constraints

Exactness first: every batch item must reproduce the serial call bit-for-bit,
and the output gate is byte identity, not a geometric tolerance. Profile/worker
elapsed is never process CPU or uninstrumented wall. Guest freshness gates must
exit 0 before any module-dispatch conclusion. Chunk size, batch payload
marshalling and buffering memory are part of the measured result, not
assumptions to be waved through.

## Comments

**Take opened 2026-10-22.** Frontier re-derived on resume: this is the only open
child (77 children, 76 resolved). Claimed before any work.

**Pre-declaration (recorded before any candidate measurement).** Cells: the four
**classic supports-on** cells of the standing gate —
`ordinary-benchy-classic-on`, `ordinary-base-classic-on`,
`accelerated-benchy-classic-on`, `accelerated-base-classic-on`. This follows the
ticket's own "natural cells" sentence and the standing harness, which refuses
any other job in its measure phase (`run-cell.ps1`: `$Generator -ne 'classic' -or
$Supports -ne 'on'` throws `unapproved timing job`). Gate: paired medians in both
modes, 6/6 retained pairs per cell, every pair's output byte-identical, process
CPU corroborating (never contradicting) wall; KEEP/DROP stays the human's.

**Pre-declaration amendment (still before any campaign run).** The CPU
corroboration clause is a **ceiling of +3.0 %** on the paired-median CPU delta,
not CPU equality: the candidate is a parallelization, so per-item marshalling
may legitimately add CPU while wall falls. A CPU *drop* with byte-identical
output is reported, not failed — identity rules out skipped work. Wall must
fall in all four cells, 6/6 retained pairs each.

**Candidate implemented; full gate run; measured recommendation KEEP
(2026-10-22).** `build_roles` is kept as the serial reference;
`build_roles_batched` is production and the emit pass buffers four layers at a
time (then flushes) so the carve, union, set-wide difference, three
body-minus-interface rounds and territory intersect each become one cross-item
`clip_polygons_batch` call. `largest_part` is shared by both carve paths.
Evidence: [FINDINGS](../evidence/t78-planner-emit-batching/FINDINGS.md),
`candidate.patch`, `qualification.json`, `ab-reduction.json` (`verify.py` exits 0
with 19/19 negative controls rejected).

**Proof matrix — 16/16 cells pass.** Baseline and candidate are byte-identical
in every cell *and* equal to the preserved t58 reference; every run is
`status=ok`, `degraded=false`, zero fatal and zero non-fatal, with non-vacuous
role counts. The snapshots differ from each other in exactly one module file
per mode (`tree-support-planner.wasm`); the binaries and the other 23 modules
are byte-identical.

**Standing gate — PASS in all four pre-declared classic supports-on cells**,
6 alternating pairs each, 24/24 retained, zero starvation exclusions, every
pair's outputs byte-identical:

| cell | wall (paired median) | CPU | occupancy |
| --- | ---: | ---: | ---: |
| `ordinary-benchy-classic-on` | **−13.41 %** | +0.51 % | 3.46 → 4.12 |
| `ordinary-base-classic-on` | **−12.30 %** | +1.51 % | 4.06 → 4.76 |
| `accelerated-benchy-classic-on` | **−7.22 %** | +0.74 % | 3.79 → 4.50 |
| `accelerated-base-classic-on` | **−16.66 %** | +2.80 % | 5.14 → 6.34 |

`accelerated-base`'s **+2.80 % CPU sits 0.2 pp under the pre-declared +3.0 %
ceiling** — a real per-item marshalling cost in that cell, disclosed rather than
smoothed. In `ordinary-base`, pairs 3–4 are the window where both arms slowed
(baseline 351/385 s against 307–336 s elsewhere); the paired median is −12.30 %.

Same-instrument attribution (ordinary base classic supports-on, attribution
only): `PrePass::SupportGeometry` **127,723 → 81,362 ms (−46.4 s)** and
`com.core.tree-support-planner` **109,018 → 60,038 ms (−49.0 s)**. Against t77's
79.7 s clip core, the ~29 s of non-clip module time is unchanged, so the clip
core fell to ~30.7 s — a **2.6x** effective speedup rather than the 12x idealized
ceiling (eight dependent batch phases per chunk plus marshalling).

Verification: `build_roles_batched_matches_the_serial_path_per_item` compares
the batched path to `build_roles` item-for-item; guest suite **146 passed / 0
failed**; guest and workspace clippy clean, `check-literals` 0,
`check-test-quality --report` 0, both freshness gates exit 0.

**Human-approved KEEP (2026-10-22); candidate committed `73a6f1e8`.** Guests were
rebuilt and both freshness gates exit 0; the standing `target/dist` and
`target/dist-accelerated` artifacts are built from the kept HEAD.

