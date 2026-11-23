# Tree-support planner native union/carve batch-or-parallel host contract

Type: task
Label: wayfinder:task
Status: resolved (2026-10-22; measured attribution + contract — the batch
contract exists and is exact by construction; the parallel shapes the fog
pointed at are refuted; one candidate graduated; see ## Answer)
Assignee: 2026-10-22 OpenCode wayfinder session — human selected the
tree-support planner union/carve route from the route options presented after
the frontier re-derivation found no open child ticket
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Does the tree-support planner's serial native union/carve work admit a batch or
parallel host contract — either independence across its many calls (t22's
census) or t76's intra-call partition-and-merge shape — that preserves the
planner's output representation exactly, and what is the measured ceiling?

The fog's entry condition for this lineage is explicit: re-enter only with a
**new batch/parallel host contract** ([base refresh findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)).
This ticket exists to establish or refute that contract. It is not authorized
to implement one.

Current-tree size: the t73 base refresh measures `PrePass::SupportGeometry` at
**119.7–147.8 s** and `com.core.tree-support-planner` at **97.1–120.1 s serial**
on the supports-on base cells, after the t74/t75/t76 keeps landed. That lineage
is the largest remaining measured un-ticketed term.

What is already measured, and what is not:

- **Call census (t22).** Batched host offsets run (50 batches on base) but
  batched clips/simplify never execute; the carve alone makes **147,993 clips**
  and the whole stage **171,636 core boolean calls**. That census is why a
  batching contract is a live question rather than a new union algorithm.
- **Chain shape (t53).** Base's chains execute **472 real unions** (Benchy 181),
  simplification activations are **zero** in both modes on both jobs, and the
  native union-core brackets are **74.65 s / 77.71 s** ordinary/accelerated base
  as single instrumented serial diagnostic observations.
- **Where the native time goes (t55).** Inside one selected base layer-9
  request, `execute` is 417,204,300 ns of a 420,270,500 ns wrapper — the sweep,
  not flattening or output reconstruction.
- **The scanbeam cache is spent (t57).** The one projection-cache shape this
  lineage produced was dropped: slower on both wall (+2.14 %) and process CPU
  (+2.78 %) in 6 of 6 retained pairs. t70's in-tree clipper keep has since
  taken the sweep's hot loop.
- **The neighbouring template (t76).** Above 100,000 facets, Morton-ordered
  centroids partitioned into 48 chunks, unioned in parallel, merged by one final
  union: stage **19.5–20.0 → 4.8–5.9 s**, outputs byte-identical. But its
  tolerance argument rested on the downstream consumers being exact predicates
  whose verdicts were unchanged by sub-resolution slivers (4.2e-8 relative) and
  polygon ordering. **The planner's union/carve output *is* support geometry**,
  so that argument must not be assumed to transfer.

Deliverables:

1. Re-derive at HEAD the planner's serial union/carve cost, call census and
   **dependency shape**: which calls are independent (batchable), which form a
   dependent chain, and which single calls are large enough to be worth
   partitioning internally. The t22/t53/t55 numbers predate the t70 clipper
   keep and the t67/t74/t75/t76 keeps and are motivation, not current cost.
2. State the exactness contract each shape needs. Cross-call batching merges
   trivially (independent results) but must preserve call order, role
   separation and the carve-repair behaviour t55 pinned. Intra-call partition
   must name its merge **and** a downstream-consumer argument that holds when
   the consumer is geometry rather than a predicate.
3. Verdict: graduate one candidate ticket with a measured, stated ceiling — or
   record no-safe-contract with the measured reason, so the fog entry can be
   rewritten rather than left open indefinitely.
4. Update the map's fog entry for this lineage to whatever this take
   establishes.

Constraints: exactness first; no bbox substitution, approximation, or
"sub-resolution is close enough" reasoning for geometry that is printed; no
production edit, no timing campaign and no auto-commit in this take unless a
candidate independently justifies one; profile/worker elapsed is never process
CPU or uninstrumented wall; `cargo xtask build-guests --check` must exit 0
before any module-dispatch conclusion.

## Answer

**A batch host contract exists and is exact by construction; the parallel shapes
the fog pointed at are refuted.** Measured on the ordinary base classic
supports-on matched cell at HEAD with a temporary host/guest probe
(`probe.patch`, reverted), output byte-identical to the preserved t58 reference
`f60e5e9b…`:

- Stage `PrePass::SupportGeometry` **147,040 ms**, of which
  `com.core.tree-support-planner` **126,674 ms**.
- Planner `clip_polygons`: **92,804 calls, 79.659 s clipper core = 62.9 % of the
  module wall**.
- **`build_roles` owns 99.03 % of that core (78.885 s)**, in two shapes:
  **472 role unions** (52.4 %; median 79.2 ms, mean 312 input polygons, max
  1,023) and **69,161 per-region carves** (44.9 %; median 0.474 ms, mean 1.96
  clip polygons).
- **The `layer_outlines_below` prefix chain is 494 calls / 0.102 s / 0.13 %** —
  refuted as an owner despite being the obvious prefix-scan candidate.
- `ladder_union` 0.662 s, `set_diff` 1.290 s, cross-role difference 0.078 s,
  simplify union 0.009 s, move union 0.001 s.

Verdict on the three candidate shapes:

1. **Cross-call batching — yes, and it is the contract.** Both owner buckets are
   independent per item. `clip_polygons_batch` already exists, fans out on
   estimated work (`crate::batch::map_batch`, threshold 4,096 primitives), and
   returns results in input order. Exactness is **per-call identical**, not a
   tolerance argument: each batch item is the same single operation with the
   same arguments as today, and the serial/parallel paths are byte-compared by
   the batch service's own tests; t65's batched linker is the precedent
   (12,022–19,290 subjects per cell, byte-identical). The missing piece is
   guest-side structure — buffer each layer's role inputs, issue batched carves
   and unions, chunk to bound memory — not host capability.
2. **Intra-call partition (t76's Morton-chunk-and-merge) — no.** There is no
   large call to partition: `role_union`'s median is 79 ms and its max 436 ms.
   Its output is printed support geometry, so t76's sub-resolution-sliver
   tolerance (justified there because the consumer was an exact predicate) does
   not transfer, and the exactness bar would be byte-identical G-code.
3. **The prefix-scan parallelization of `layer_outlines_below` — no.** 0.13 % of
   the bucket.

Idealized ceiling (**arithmetic, not measured**): perfect 12-way scaling of the
78.885 s `build_roles` core removes ≈ 72.3 s, taking the 147.0 s stage to
≈ 74.7 s. It excludes batch marshalling of 69k request records, buffering
memory, imperfect scaling, and the 47.0 s of the module that is not clip work
(offsets, MST, node moves). The real win belongs to the standing paired gate.

Graduated: [Tree-support planner emit-pass batched carve and role union](78-tree-support-planner-emit-batching.md).

Evidence: [`evidence/t77-planner-contract/`](../evidence/t77-planner-contract/FINDINGS.md)
(`summary.json`, `reduce.py`, `probe.patch`, `verify.py` exits 0 checkout-only
with 8/8 negative controls rejected). Raw capture under `target/t77-probe2/`
(gitignored, reproducible from `probe.patch` + `reduce.py`).

Restoration: probe reverted with `git apply -R`, no probe string left in
`crates/` or `modules/`, guests rebuilt, both freshness gates exit 0,
`target/release/pnp_cli.exe` rebuilt from the reverted tree. No production edit,
no timing campaign, no commit beyond this ticket's records.

