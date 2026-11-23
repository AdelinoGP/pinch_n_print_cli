# Support paint-policy dead-walk reduction (t66)

**Resolution in one line:** measured GATE NOT MET — CPU reduced in all four
cells (−0.56 %…−1.22 %) and wall reduced on both base cells (−0.4977 %,
−2.1098 %), but the benchy cells' wall deltas are +0.0017 % and +0.1272 %,
failing the pre-declared wall < 0 % clause. **Human-approved DROP
(2026-10-14):** candidate retired to the committed re-appliable patch, tree
restored to `HEAD`, no production edit.

Route: [Support paint-policy dead-walk reduction](../../issues/66-paint-policy-dead-walk-reduction.md).
Graduated from: [Tree-support renderer self-fuel and scan-fill attribution](../../issues/64-tree-support-renderer-attribution.md).

## Candidate

Annotations-empty fast path (t64 shape 1), two source edits plus tests:

- `slicer_core::paint_policy::has_support_paint` (new): true iff the region's
  `segment_annotations` carry at least one `Some(_)` value under
  `SupportBlocker` or `SupportEnforcer`. Conservative: it may be true while
  the classifier's area floor still yields `DefaultEligible` (extra walk only,
  never a changed answer).
- `PaintRegionLayerView::paint_policy_for` (`crates/slicer-sdk/src/traits.rs`):
  returns `DefaultEligible` before the region-ownership walk when no region of
  the attached slice carries support paint.

Exactness argument: `support_eligibility` classifies exclusively from the two
support map entries plus the region-area floor, so when the predicate is false
for every region of the attached slice, every *covering* region is
`DefaultEligible` and the aggregate is `DefaultEligible` independent of the
walk. Views with support paint take the walk unchanged.

Shape 2 (per-ring bbox prefilter) is deliberately not implemented: it shapes
only the painted path, which has no representative workload or fixture (the
map's painted-wall fog); the fast path already covers every matched cell.

Candidate provenance: HEAD `ce5abecd` + `candidate.patch`
(sha256 `8ccfbeb1e81888956c29455c04aa80193b75772d5c95b0efb1bee01b66d8fe52`) +
new test file `candidate-new-file-paint_policy_for_tdd.rs`
(sha256 `d949a9324bb862dbdf7c205620f402fb0a67d8888280dee5ee58ab038ae72ec3`).
Retired per the DROP ruling (2026-10-14): the three source edits and the new
test file were removed from the tree (a retirement stash also holds the
working-tree copies); the two re-appliable artifacts above are committed in
this directory, byte-identical to the as-measured state.

## Tests and gates (candidate tree)

- `cargo test -p slicer-core --features host-algos --test paint_policy` — 10/10
  (4 new: predicate semantics, `Some`-only detection, the
  no-support-paint ⇒ `DefaultEligible` invariant, area-floor conservatism).
- `cargo test -p slicer-sdk --features test --test paint_policy_for_tdd` —
  10/10 (fast-path value contracts; mutation check: an unconditional skip
  fails 3 of them).
- `cargo test -p slicer-runtime --test contract
  real_paint_region_data_visible_through_production_support_dispatch` — pass.
- Module suites: tree-support `enforcer_blocker_tdd` 8/8; traditional-support
  `enforcer_blocker_tdd` 5/5.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations;
  `cargo xtask check-test-quality --report` — 0 findings.
- `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check` — exit 0 at the candidate
  tree (37 guests rebuilt per mode).

## Campaign (standing paired gate + 16-cell qualification)

Pre-declaration: ticket comment 2026-10-14 (recorded before any measurement).
Tooling: this directory (`run-cell.ps1`, `run-all-proof.ps1`,
`run-all-measure.ps1`, `build-qualification.py`, `verify-references.py`,
`reduce_ab.py`, `build-snapshots.sh`); raw runs and snapshots under the
ignored `.local-artifacts/perimeter-reference-preparation/t66-paint-walk/`.

Snapshot executables (sha256):

| arm | ordinary | accelerated |
| --- | --- | --- |
| baseline | `4fd7f3d6df67a465…` | `f80f16ac191d3b70…` |
| candidate | `477c54b4f6fa6277…` | `750cbfd01301d6da…` |

### 16-cell output-identity qualification

Ran 2026-10-14 (proof driver; one harness-side interruption, see Limits).
`build-qualification.py`: **qualified cells: 16/16; pass=True**;
`verify-references.py`: **reference-checked cells: 16/16; pass=True**. In every
cell the baseline and candidate runs produced byte-identical outputs equal to
the preserved t58 reference (single sha256 per cell, e.g. benchy-classic-on
`76eb4e1d…`, base-classic-on `f60e5e9b…`), all runs `status=ok`, not degraded,
0 fatal / 0 non-fatal, non-vacuous wall/infill/support roles.
`qualification.json` (this directory) sealed with 16 cells.

### Standing paired gate (pre-declared cells)

56 timed runs (4 cells × 14: 2 warm-ups + 6 alternating pairs); all four cells
retained 6/6 pairs with zero starvation exclusions; every measured run
byte-identical within its cell. Reduced by `reduce_ab.py` → `ab-reduction.json`:

| cell | baseline wall med | candidate wall med | paired-median Δwall | Δcpu | pairs |
|---|---|---|---|---|---|
| ordinary-benchy-classic-on | 26.407 s | 26.830 s | **+0.0017 %** | −1.15 % | 6/6 |
| ordinary-base-classic-on | 487.119 s | 488.243 s | **−0.4977 %** | −0.56 % | 6/6 |
| accelerated-benchy-classic-on | 22.667 s | 22.657 s | **+0.1272 %** | −0.87 % | 6/6 |
| accelerated-base-classic-on | 447.384 s | 436.638 s | **−2.1098 %** | −1.22 % | 6/6 |

**VERDICT: GATE NOT MET** (wall clause fails on both benchy cells; wall passes
on both base cells; CPU clause passes in all four cells).

Read plainly: the dead walk's CPU work is real and removed in every cell
(−0.6 %…−1.2 % CPU), and it materializes in wall only on the base fixtures
(−2.4 s / −9.4 s there); on the ~23–27 s benchy slices the wall effect is
inside paired noise (±0.13 %), consistent with t64's small freed-wall bound
(0.45–0.55 s on the probed per-layer phase) and its 0–16 % fuel-to-wall
transfer range. The strict wall < 0 % clause is what fails.

## Limits and disclosure

- **Background load.** At measure start (14:35) Opera (five processes),
  Discord, WindowsTerminal and opencode were resident (cumulative CPU
  snapshot taken; same background class disclosed in t58). No pair hit the
  starvation rule: cpu/wall medians 4.2–5.1 in every cell, zero exclusions,
  6/6 pairs retained everywhere.
- **Interruptions (all harness-side; none a slicer failure).**
  - Proof driver killed 2026-10-14 12:37:34 (−03:00) mid-cell-14 candidate
    warm-up by the documented opencode location-services eviction: stderr
    frozen just past `phase_start` `prepass` at 25,842 bytes, no panic, no
    `.gcode`, no `rows.csv` row — the same signature t58 recorded for its
    eviction kills. That cell's baseline arm had already completed
    (`f60e5e9b…`); the partial cell was archived under
    `.local-artifacts/perimeter-reference-preparation/t66-paint-walk/interrupted-2026-10-14T1237-accelerated-base-classic-on/`
    (README, rows.csv, killed stderr) and the cell re-ran from scratch per the
    one-shot-guard discipline. No timing verdict existed at interruption time,
    so no gate-failing cell rerun is implied.
  - The campaign was then relaunched **detached from the agent process tree**
    (`launch-detached.ps1`, the t58 remedy). The harness logged two further
    location evictions during the measure window (15:36 and 16:37 −03:00);
    the detached driver ran through both with no cell boundary disrupted and
    no rerun needed. The harness-side watcher shells were killed by the
    evictions (the completion notification never fired); the campaign itself
    was unaffected — the driver logs are the record.
- **Run counts.** Proof: 16 cells × 2 arms (plus the one archived aborted
  candidate run). Measure: 4 cells × 14 runs = 56 runs, all `ok`, not
  degraded, 0 fatal / 0 non-fatal, single sha256 per cell (byte-identity held
  across all 56 runs and against the proof/qualification hashes).
- **Scope.** Timing cells are the pre-declared classic supports-on set;
  arachne-on cells are excluded by pre-declaration (same generator-invariant
  effect class per t64; extension would be a new pre-declaration) though their
  outputs are covered by the 16-cell qualification. Shape 2 (ring bbox
  prefilter) was not implemented in this take (no painted workload; see
  Candidate).

## Reproduction

```bash
python docs/specs/perf-vs-orca/evidence/t66-paint-walk/build-qualification.py
python docs/specs/perf-vs-orca/evidence/t66-paint-walk/verify-references.py
python docs/specs/perf-vs-orca/evidence/t66-paint-walk/reduce_ab.py
```
