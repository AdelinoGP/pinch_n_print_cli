# Clip-universe preparation hoist: implementation + standing paired A/B

Type: task
Status: open
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
Fuel ceiling: 94.7% of the inflate term = 61.4% of linker fuel ≈ 28.1% of
slice fuel (239.5B baseline). Wall transfer unmeasured.

## Work (only after explicit human authorization of this take)

- Choose the shape: a prepared-universe entry point in
  `crates/slicer-core/src/polygon_ops.rs` (`clip_polylines` keeps its current
  one-shot API for `lightning/layer.rs`, `lightning/mod.rs`,
  `prepass_slice.rs`), versus a guest-side precomputed universe, versus a
  batched multi-polyline entry that flattens+inflates once. Whichever is
  chosen, `inflate_paths_64`'s inputs and outputs must be bit-identical to
  today's per-call computation, and AC-1…AC-7 of the `clip_polylines` contract
  must hold.
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

[Linker subcost findings](../evidence/t21-linker-subcost/FINDINGS.md),
probe preserved at `evidence/t21-linker-subcost/probe.patch`, re-derivation in
`evidence/t21-linker-subcost/verify-t21.py`.
