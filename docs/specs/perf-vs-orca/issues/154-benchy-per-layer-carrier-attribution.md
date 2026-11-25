# Benchy per-layer program: which carrier holds the wall, and what does canonical do differently?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-14; Phase-1 attribution + destination arithmetic —
  **the benchy per-layer program is margin, not route**: at HEAD, with the
  prepass at canonical cost, **7 of 8 cells win** and the four benchy cells clear
  the line by **9.9 %–33.2 %**; the only loser is **base classic-on at 1.051x**,
  which needs **1.096 s** (9.6 % of its per-layer phase). The benchy carriers
  are kernel-bound (`classic-perimeters` 77–98 % self fuel, `infill-linker`
  99.8–100 %), and the map's t127 attribution of the Infill-stage block to the
  prepared-region view cannot hold at HEAD. The residual graduates as
  [t155](155-infill-stage-block-base-on-residual.md). See ## Answer.)
Assignee: 2026-11-14 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named; all 153 children resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Seam candidate generation on the printed loops: authorize the
  per-layer relocation?](153-seam-loop-relocation-ruling.md)'s close ("the
  per-layer program on the benchy cells remains the largest unpriced member")
  and [Prepass work-set and per-op parity against canonical v2.5.0](129-prepass-work-set-per-op-parity.md)
  finding 2 ("at canonical prepass cost, benchy-off's per-layer phase is
  1,450 ms against a 1,716 ms Orca wall and benchy-on's is 1,644 ms against
  2,462 ms — the per-layer phase is part of the first program on both benchy
  cells, not a last mile only on classic-off").

## Question

On the current tree, what carries the per-layer phase on the four benchy
cells, what are the per-op costs inside the two largest supports-off carriers,
and does any addressable shape pay the per-layer share of those cells' required
cut — given that at canonical prepass cost the per-layer phase alone is about
**85 %** of Orca's whole wall on benchy classic-off?

The destination arithmetic makes this a real gate, not a tidy-up:

- **Benchy arachne-off is now the weakest cell in the matrix.** t145's paired
  scoreboard measured **1.853x** (3.674 s / 1.983 s, required cut **46.03 %**),
  and t150's later `MeshAnalysis` retirement moved it **−4.87 %** and base
  classic-off **−12.08 %**; no full paired scoreboard has been re-taken since
  t150/t151, so the current ratio must be re-derived in this take or quoted
  with t145's denominator and t150's measured delta disclosed.
- **The prepass cannot win benchy alone.** t129 substituted canonical's
  measured prepass cost (0.0505 s benchy-off, 0.5498 s benchy-on) and got
  **1.053x / 1.075x**; only full geometry parity reached 0.550x / 0.704x. So
  the benchy cells need a per-layer cut in the same program.
- **The carrier floors are priced but no shape is named.** t126 measured the
  per-layer phase at 31.3 % (classic-off) / 33.3 % (arachne-off) / 18.5 % /
  20.9 % (supports-on) of each cell, with perimeters **3.82x–15.84x** the
  0.80 % floor, infill-linker **1.17x–8.28x**, tree-support **4.66x–9.63x**
  (supports-on) and the raft bracket **2.64x–8.25x**. t68 then measured the
  perimeters bucket at **97–99 % native clipper ops**, and t127 proved the
  raft bracket is the `Layer::Infill` stage's shared **prepared-region view
  materialisation**, paid by whichever Infill module is dispatched first, so
  its price is not a raft saving.

So the question this ticket must answer is not "is the per-layer phase big"
(it is), but **"is any named per-layer operation reducible under the current
representation, and what is its wall-equivalent ceiling on the benchy cells
today?"** If the answer is no, the map records measured-dead and the benchy
route is a representation change, not a shape.

## Scope

1. **Arm and gates.** Build `pnp_cli` release from the current tree, run
   `cargo xtask build-guests --check` (must exit 0), record the binary
   SHA-256, and confirm the tree is clean apart from tracker/evidence edits.
2. **Four instrumented ordinary passes**, one per benchy cell, with
   `RAYON_NUM_THREADS=12`, the standing matched-pair configs under
   `evidence/matched-pair/configs/`, and 24 external core modules. Validate
   every pass: exit 0, `degraded=false`, `fatal_error_count=0`,
   `non_fatal_error_count=0`, and G-code SHA-256 equal to
   [`reference.json`](../evidence/t151-skirt-hull-contour/reference.json)
   (t151's active set).
   An instrumented wall is never a wall claim.
3. **Re-derive the split at HEAD.** Phase brackets, prepass stages, per-layer
   stage brackets and module brackets with call counts, the module-busy union
   versus the phase wall, and the effective parallelism, on all four benchy
   cells. t126's numbers are 2026-11-08 and predate t135 Stage 1/2, t142,
   t143, t148, t150 and t151; do not carry them forward as current.
4. **Decompose the two largest supports-off carriers per op.** For
   `classic-perimeters` (or `arachne-perimeters`) and `infill-linker`, use the
   profile scopes to separate path conversion, sweep, PolyTree rebuild,
   marshalling and allocation, the way t131 did for the common prepass stages,
   and give per-call distributions (p50/p90/max) and a layer-cost curve.
5. **Canonical per-layer audit at the pin** (`v2.5.0`,
   `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`; cite file + function, never line
   numbers). Name what canonical runs per layer (`Layer::make_perimeters`,
   `Fill`, `PrintObject::infill`, seam placement, support) and which of PnP's
   per-layer operations canonical does not do, or does on smaller operands.
   Source-derived op counts are disclosed as such: the rig has no canonical
   per-stage timing channel (t110).
6. **Price every named shape** against (a) 0.80 % of each cell's wall of
   record and (b) the per-layer budget left at canonical prepass cost. Use
   thread-accumulated sums only as upper bounds divided by measured
   parallelism, never as wall.
7. **Stop rule.** If no shape clears 0.80 % of the cell wall with CPU
   corroboration available, record measured-dead, name the structural blocker,
   produce no candidate ticket and make no production edit.

## Acceptance

- Four instrumented passes, all clean and byte-identical to t151's active set,
  raw streams preserved under gitignored `target/t154-instrumented/`.
- Phase shares reconcile to 1 per cell; the four phase brackets plus the
  outside residual equal `slice_complete`; prepass stage brackets reconcile
  with the prepass phase within the reducer's stated tolerance.
- `evidence/t154-benchy-per-layer/` holds `FINDINGS.md`, `REPRODUCE.md`,
  `run-instrumented.py`, `reduce.py`, the committed reduced `summary.json`,
  `verify.py` and its negative controls. `verify.py` exits 0 checkout-only and
  `--self-test` rejects every negative control.
- Every canonical citation resolves at the pin, by file + function.
- The map's destination status and **Not yet specified** fog are updated, the
  take is recorded in **Decisions so far**, and the ticket's `## Answer` states
  the verdict (KEEP candidate graduated, or measured-dead).
- No production edit and no G-code reference move.

## Evidence starts

- [`evidence/t126-post-t123-phase-split/`](../evidence/t126-post-t123-phase-split/FINDINGS.md)
  — the only per-layer stage/module split (2026-11-08), its reducer and its
  parallelism method.
- [`evidence/t127-raft-band-dispatch/`](../evidence/t127-raft-band-dispatch/FINDINGS.md)
  — the prepared-region-view finding and the "price the module by declining
  it" lesson.
- [`evidence/t145-benchy-rebaseline/`](../evidence/t145-benchy-rebaseline/FINDINGS.md)
  — the current benchy fixture's paired scoreboard (the denominators).
- [`evidence/t129-prepass-work-set-parity/`](../evidence/t129-prepass-work-set-parity/FINDINGS.md)
  — the canonical prepass cost substitution and the per-layer-in-first-program
  arithmetic.
- [`evidence/t68-host-roundtrips/`](../evidence/t68-host-roundtrips/FINDINGS.md)
  — the classic-perimeters bucket's op census and its native-clipper share.
- [`evidence/t150-meshanalysis-retirement/`](../evidence/t150-meshanalysis-retirement/FINDINGS.md)
  — the last measured wall deltas that move the benchy denominators.

## Answer

**Resolved 2026-11-14 as a Phase-1 attribution plus a destination-arithmetic
re-derivation at HEAD. The benchy per-layer program is margin, not route; the
per-layer requirement that remains is base classic-on's 1.096 s.**

Eight instrumented ordinary passes (one per matched cell) are clean
(`degraded=false`, 0 fatal, 0 non-fatal) and byte-identical to t151's active
set; `cargo xtask build-guests --check` exits 0; the ordinary binary is
`e1b8fa69e59e47c9bba408cc34b9273b57f92946d641c75b6ec39c67cfc7eaa0`; box load
7 %. The split at HEAD is prepass **63.5–82.4 %**, per-layer **14.4–29.6 %**,
postpass **2.6–6.5 %**, outside **0.1–5.3 %**; the per-layer phase's host gap is
0–56 ms (≤ 0.6 %) in every cell, so there is no cross-worker idle pool.

**Arithmetic.** Substituting canonical's measured prepass cost (t110: 50.5 /
549.8 / 2,103.0 / 10,444.8 ms) and holding every other phase at its measured
share gives **seven wins**: benchy arachne-off 0.668x, base arachne-off 0.698x,
benchy classic-off 0.740x, base classic-off 0.743x, benchy classic-on 0.857x,
benchy arachne-on 0.901x, base arachne-on 0.967x — and **base classic-on
1.051x**, which needs **1.096 s** (9.6 % of its 11.46 s per-layer phase). The
required prepass cut is **64.3 %–84.5 %**, largest on base classic-on (83.7 %),
base arachne-on (82.5 %) and benchy arachne-on (84.5 %), not on benchy
classic-off. **This supersedes t128/t129's "prepass at canonical cost loses both
benchy cells (1.053x / 1.075x)"**, which paired t126's per-layer phases with
t124's Orca walls.

**Carriers.** The benchy supports-off per-layer phase is carried by the
perimeters module (46.8 % / 54.8 % of per-layer thread), the first
`Layer::Infill` dispatch (21.2 % / 17.6 %) and `infill-linker` (20.5 % / 18.5 %);
the supports-on cells by `tree-support` (47.8 % / 44.0 %). The fuel profile
shows the carriers are **in-module work**: `classic-perimeters` 77–78 % self
fuel with `polygon_ops::offset2_ex` the only scope (21.8 %), `arachne-perimeters`
97.6 % self, `infill-linker` 99.8–100 % self, `tree-support` 96.6–96.8 % self.
The requested per-op split cannot be produced at the current instrumentation
surface, and that limit is itself the finding: no plumbing fat is left inside
these carriers.

**Canonical audit** (pin `v2.5.0`, `40ecc66f78`): canonical runs
`PrintObject::make_perimeters` and `PrintObject::infill` as tbb passes over
layers, generates tree support in one whole-object pass
(`PrintObject::_generate_support_material` → `TreeSupport::generate`), and places
seams at emit (`SeamPlacer::place_seam` from `GCode::process_layers`). PnP
therefore pays support work **twice** on the supports-on cells (prepass plan plus
per-layer `Layer::Support`) and has no canonical counterpart for its per-layer
`infill-linker`, `path-optimization-default` or the prepared-region
materialisation.

**Ledger correction.** t127 named the first-`Layer::Infill` block "the shared
prepared-region view materialisation". At HEAD that cannot be right without an
arena invalidation: `prepare_dispatch_region_views`
(`crates/slicer-runtime/src/layer_executor.rs`) makes `Layer::Perimeters` call
`ensure_prepared_perimeter_source_regions`
(`crates/slicer-runtime/src/blackboard.rs`), whose identity branch
(`perimeter_source_projection_is_identity`, true on every matched job)
materialises `prepared_regions` and sets the sharing flag, so `Layer::Infill`
hits the memo. The block is real and stage-shared (p50 **63 ms** on base
classic-off against its own nil work; `gyroid`'s p50 is 1 ms when it is not
first), but **its identity is unmeasured**; graduated as
[t155](155-infill-stage-block-base-on-residual.md).

**Graduated:** [The `Layer::Infill` first-dispatch block and base classic-on's
1.096 s residual](155-infill-stage-block-base-on-residual.md) (AFK, unblocked).

**Disclosures.** The arithmetic is an estimate chaining two windows (shares from
this take; walls from t145/t148 carried by t150's deltas; canonical prepass from
t110) and no ratio here is a ratio of record. No wall claim is taken from any
instrumented or profiled run. The t126→t154 per-layer fall (−21.6 % to −40.1 % on
the benchy cells) is measured but **not attributed**; t139's mid-layer sampling
and t142's quartile derivation are named candidates only. No production edit and
no G-code reference move.

Evidence: [`evidence/t154-benchy-per-layer/`](../evidence/t154-benchy-per-layer/FINDINGS.md)
(`summary.json` 8 cells; `arithmetic.json`; `carriers.json`; eight reduced
`profile-*.json`; `verify.py` exits 0 checkout-only and `--self-test` rejects
11/11 negative controls).
