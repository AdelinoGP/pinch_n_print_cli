# Strong planner occupancy-offset batching: representation-safe four-cell campaign

Type: task / candidate campaign
Label: wayfinder:task
Status: resolved (2026-11-07; measured **DROP** — per-origin batching is
  byte-identical in all eight cells, but only **2/8 cells pass** the amended
  paired gate; candidate and WASM retired, production restored. See ## Answer)
Assignee: 2026-11-07 OpenCode wayfinder session — the human selected this ticket explicitly ("great, continue with 125")
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](117-strong-planner-carry-ruling.md)'s
  human ruling (2026-11-07). This is a representation-safe candidate, not a
  canonical-parity change.

## Question

Can the Strong planner's `inflate_model_occupancy` offsets be executed through
`offset_polygons_batch` without changing output, such that the candidate clears
the 0.80 % paired wall floor with CPU corroboration in each of the four
supports-on cells and does not regress any supports-off cell?

## Exact shape

In `inflate_model_occupancy`
(`modules/core-modules/tree-support-planner/src/lib.rs`), buffer the raw polygon
requests from the per-layer emit check and the per-origin role carve, and execute
them through `host::offset_polygons_batch` in bounded chunks. Preserve each
request's distance, Miter join, zero arc tolerance and input-order result
association. The host's `HostExecutionContext::offset_polygons_batch`
(`crates/slicer-wasm-host/src/host.rs`) must continue to run the same
`slicer_core::polygon_ops::offset` operation for each request; do not substitute
a different geometry operation.

The per-origin half may be staged inside the existing emit batch if that keeps
the change small, but the campaign must report which call family was actually
converted. Do not borrow the existing emit-batching campaign's 2.6x as the
expected speedup: measure this
request set's own batch occupancy and resulting wall/CPU effect.

No canonical-parity exception is needed. The expected candidate-vs-baseline
output delta is **zero in all eight cells**.

## Acceptance gate (human-amended by the carry ruling)

1. **Output before timing.** Capture a fresh 16-cell set. Baseline ordinary ≡
   baseline accelerated ≡ the current post-t123 output; candidate ordinary ≡
   candidate accelerated; and candidate ≡ baseline byte-for-byte in all eight
   cells. Any candidate output delta is a defect and stops the campaign. Both
   guest-freshness gates must exit 0, and every PnP run must be non-degraded with
   no non-fatal diagnostics.
2. **Improvement cells.** For `benchy classic-on`, `benchy arachne-on`,
   `base classic-on` and `base arachne-on`, run at least three interleaved
   baseline/candidate pairs with alternating order. In each cell, the candidate's
   median wall must improve by at least **0.80 % of that cell's same-run baseline
   median wall**, and its median process CPU must decrease. Report every pair,
   the medians and ranges, and the floor multiple; disclose rather than hide any
   spoiled pair or starvation exclusion.
3. **No-regression cells.** For `benchy classic-off`, `benchy arachne-off`,
   `base classic-off` and `base arachne-off`, use the same interleaved protocol.
   The candidate's median wall and median process CPU must not regress in any
   cell. Report the full eight-cell wall/CPU table even though these four cells
   have no positive improvement-floor requirement.
4. **Destination accounting.** Passing this gate is one route step, not map
   closure. The destination remains PnP winning all eight cells against Orca;
   re-derive the scoreboard separately if this candidate lands.

## Tests and evidence

- Add focused coverage that batched occupancy requests preserve input order and
  exactly match the singular offset path, including empty and single-request
  batches and both occupancy call families converted by the candidate.
- Run the narrowest planner/host tests that cover the edited path, then the
  required check/clippy/literal/test-quality gates for the touched code. Follow
  the repo's guest-WASM freshness discipline before attributing any failure.
- Commit an in-repo evidence bundle with a checkout-only verifier and negative
  controls for the output, provenance and paired-gate claims.

## Stop rule

Stop and report the measured result without weakening the gate if any output
byte changes, any supports-on cell misses its wall floor or has contradicting
CPU, or any supports-off cell regresses on median wall or CPU. Do not fold in
bridge expansion, another prepass change, or a new geometry rule to rescue the
candidate.

## Evidence starts

- [Strong support planner Phase 1: sub-term decomposition and a representation-safe shape](113-strong-planner-phase1.md)
  and [`evidence/t113-strong-planner-phase1/`](../evidence/t113-strong-planner-phase1/FINDINGS.md)
  — the sub-term decomposition and exactness argument. Its absolute walls, floor
  multiples and three-cell hashes are historical and must be re-derived.
- [Tree-support planner emit-pass batched carve and role union](78-tree-support-planner-emit-batching.md)
  and [`evidence/t78-planner-emit-batching/`](../evidence/t78-planner-emit-batching/FINDINGS.md)
  — the existing batch-execution precedent, not this request set's price.
- [Facet bridge-path retirement: canonical classification over the whole unsupported set](123-facet-bridge-path-retirement.md)
  and [`evidence/t123-facet-retirement/`](../evidence/t123-facet-retirement/FINDINGS.md)
  — the current output references and recent paired-proof method.
- [Post-t123 Orca scoreboard: re-derive the eight matched-cell ratios](124-post-t123-orca-scoreboard.md)
  — the current destination scoreboard; not a source of candidate gate floors.

## Answer

**DROP. The per-origin occupancy-offset batch is output-exact but fails the
human-amended paired gate in six of eight cells.** The candidate converted only
the per-origin role-carve call family inside the existing bounded emit flush;
the per-layer emit-check offsets remained singular. No rescue shape was added.

### Output and run proof

- Fresh 16-cell output: baseline ordinary ≡ baseline accelerated ≡ candidate
  ordinary ≡ candidate accelerated ≡ the post-t123 reference in **8/8 cells**.
- Output campaign: **32 runs**. Paired campaign: **48 runs** (three alternating,
  interleaved pairs per cell). All **80** exited 0 with `degraded=false` and
  `non_fatal_error_count=0`; all generator/claim-holder checks passed. Zero
  starvation exclusions and zero discarded campaign pairs.
- Host binaries are identical between arms; only the planner WASM changes.
  Both freshness checks passed before the campaign and again after retirement.

### Paired result

Wall improvement is baseline minus candidate; CPU is the median process-CPU
delta in seconds. Supports-on cells require at least the 0.80 % wall floor and a
CPU decrease; supports-off cells require no median wall or CPU regression.

| cell | wall delta | floor multiple | CPU delta | gate |
| --- | ---: | ---: | ---: | --- |
| benchy classic-off | **−0.19 %** | −0.24x | +1.2031 s | **FAIL — wall regressed** |
| benchy classic-on | **−17.90 %** | −22.37x | +0.1406 s | **FAIL — below floor** |
| benchy arachne-off | **−2.75 %** | −3.44x | −0.1718 s | **FAIL — wall and CPU regressed** |
| benchy arachne-on | **+1.89 %** | +2.37x | +0.5782 s | PASS |
| base classic-off | +2.16 % | +2.71x | −0.2813 s | **FAIL — CPU regressed** |
| base classic-on | +6.13 % | +7.67x | −1.7656 s | **FAIL — CPU contradicted wall** |
| base arachne-off | **−3.03 %** | −3.79x | +0.9532 s | **FAIL — wall regressed** |
| base arachne-on | **+10.88 %** | +13.60x | +2.3125 s | PASS |

The gate was not weakened for the visible wall spread. Machine load was not
sampled; no idle-box or standalone occupancy-fan-out claim is made, and the old
2.6x screen is not quoted as this request set's measured price.

### Tests and retirement

Two focused alignment/no-op tests passed, followed by the full planner crate:
**148 tests passed** (64 unit + 84 integration). The candidate was then manually
reverted, both baseline guests rebuilt, and the production source diff became
empty. The in-tree ordinary planner WASM matches the baseline again. No scoreboard
refresh is needed because nothing landed.

Evidence: [`evidence/t125-occupancy-offset-batching/`](../evidence/t125-occupancy-offset-batching/FINDINGS.md),
including the candidate patch, raw-row reducer and checkout-only verifier
(`verify.py`: **442 checks**; `--self-test` rejects **10/10** negative controls).
The per-layer emit-check half remains unmeasured and is not authorized as a
rescue by this ticket.
