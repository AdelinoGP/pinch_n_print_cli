# Clip-universe preparation hoist: implementation + standing paired A/B

Type: task
Status: resolved
Assignee: current OpenCode session (wayfinder), 2026-10-02
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 21

## Question

Does hoisting the clip-universe preparation out of `link_paths_without_offset`'s
per-path loop preserve the output exactly and improve median uninstrumented
wall with corroborating process CPU, per the map's standing paired ordinary +
accelerated A/B — or does it fail the fairness/keep gates?

Graduated from [infill-linker attribution](21-infill-linker-attribution.md)'s
measured split (2026-10-01): the linker is 97.65% the per-path re-clip, and
64.84% of all linker fuel is `clip_polylines`'s repeated flatten + 1-unit
`inflate_paths_64` of a clip universe that every call in one invocation shares.
Fuel ceiling — an **estimate under a same-cost assumption**, not a measurement:
the uniform-count bound is 94.7% of the inflate term = 61.4% of linker fuel ≈
28.1% of slice fuel (239.5B baseline); the layer-aware estimate is 98.2% of the
inflate term = 63.7% of linker fuel. Both assume the surviving deduplicated
preparation costs the same as one current per-call preparation, which the fuel
split does not prove. Wall transfer unmeasured.

## Work (only after explicit human authorization of this take)

- Choose the shape: a prepared-universe entry point in
  `crates/slicer-core/src/polygon_ops.rs` (`clip_polylines` keeps its current
  one-shot API for `convert_to_lines`
  (`crates/slicer-core/src/algos/lightning/layer.rs`), the tree-sampling loop
  in `generate_lightning_trees`
  (`crates/slicer-core/src/algos/lightning/mod.rs`), and
  `floating_edges_of_gated_area`
  (`crates/slicer-core/src/algos/prepass_slice.rs`)), versus a guest-side
  precomputed universe, versus a batched multi-polyline entry that
  flattens+inflates once. Whichever is chosen, `inflate_paths_64`'s inputs and
  outputs must be bit-identical to today's per-call computation, and AC-1…AC-7
  of the `clip_polylines` contract must hold.
- Prove output equivalence first: the frozen supports-off Benchy Arachne
  reference (`7049a06d…`) is byte-identical, plus the classic and base cells'
  outputs if the take wants a broader exactness claim; add a regression test
  that fails if the prepared universe diverges from the per-call one.
- Then the standing measurement: paired ordinary + accelerated, interleaved
  repeats with starvation exclusion, uninstrumented probe-free runs, isolated
  host/guest snapshots, output disclosure per the fairness contract. Fuel
  alone is not a wall win — gate before keeping.
- Keep/drop recommendation returns to the human; no auto-commit.

## Not authorized

Any implementation, benchmark, or production change until the human explicitly
authorizes this take; the ticket 21 study authorized attribution only.

## Evidence

### Authorization / equivalence-gate amendment (2026-10-02)

The human authorized implementation and the standing paired A/B, then approved
using **candidate == fresh HEAD baseline**, byte-identical, after the exact
external-only command reproduced a pre-existing drift from the frozen reference.
The frozen snapshot still reproduces `7049a06d…`; fresh HEAD produces
`1b71f83d…`. Differences include moves and TYPE counts, not just comments.
The drift is disclosed separately and is not attributed to this hoist. This is
an isolated optimization experiment, not an adoption-campaign retry, reference
replacement, or authorization to repair that drift. No auto-commit.

[Linker subcost findings](../evidence/t21-linker-subcost/FINDINGS.md),
probe preserved at `evidence/t21-linker-subcost/probe.patch`, re-derivation in
`evidence/t21-linker-subcost/verify-t21.py`.

## Resolution comment — 2026-10-02

**KEEP — approved by the human 2026-10-02, implemented + resolved here.**
Implemented
`PreparedPolylineClip` (`crates/slicer-core/src/polygon_ops.rs`) and lazy
per-invocation reuse in `link_paths_without_offset`
(`modules/core-modules/infill-linker/src/orchestrate.rs`). Other callers retain
the one-shot API; each owner still gets an independent intersection.

Recorded pre-hoist universe/output regressions, existing polygon contracts and
linker tests pass; workspace build, all-targets Clippy, literal/test-quality
gates and both mode-specific guest freshness checks pass. The standing A/B
passes on the **frozen-settings supports-off Benchy Arachne job**: six pairs
per mode, no starvation exclusions, every output byte-identical to its fresh
baseline, median wall lower in both modes with CPU corroborating. Full results,
per-sample ratios, output disclosure and the scoped recommendation:
[Hoist findings](../evidence/t44-clip-universe-hoist/FINDINGS.md). Re-derive with
`verify-t44.py` in that evidence directory, requiring no optional raw tree.

The old frozen reference is preserved. Its pre-existing HEAD drift and
unclosed-loop emitter warnings are disclosed separately; this is not a
matched-Orca correctness result or an accelerated-adoption retry. Classic/base
and supports-on outputs/timings are not claimed. The unresolved route question
is [Frozen-job output drift boundary](46-frozen-job-output-drift-boundary.md).
