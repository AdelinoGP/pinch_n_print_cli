# Tree-support planner emit-pass batched carve and role union

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed
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
