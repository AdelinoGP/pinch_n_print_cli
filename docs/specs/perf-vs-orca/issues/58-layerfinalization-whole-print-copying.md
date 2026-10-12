# LayerFinalization whole-print copying: per-module clone/drop on base supports-on

Type: task
Label: wayfinder:task
Status: claimed (2026-10-10) — measured 2026-10-12, recommendation
returned to the human (KEEP)
Assignee: claimed by the 2026-10-10 session (human-selected take from the four-ticket table)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Progress comment — session 2026-10-10 (paused mid-take)

State at pause (resume instructions inline):

1. **Candidate implemented** in the working tree, all gates green so far:
   `execute_layer_finalization_with_instrumentation`
   (`crates/slicer-runtime/src/layer_finalization.rs`) now takes ONE
   whole-print working copy per stage run (was one clone+drop per module);
   commit happens once after the last module validates. Plus the pre-emit
   whole-print `to_vec` removed via
   `execute_postpass_with_owned_capture` (`crates/slicer-runtime/src/postpass.rs`;
   `&[...]` variants delegate unchanged; `run_postpass_with_thumbnail`
   threads ownership from `run_pipeline_core`).
2. Static gates passed: workspace check all-targets, clippy `-D warnings`,
   check-literals 0, test-quality 0; executor bucket 234 ✓ (pinned
   failure-commit semantics hold); visual-debug tap tests ✓; integration
   373 ✓ (+1 pre-existing feature sentinel, +1 staleness artifact fixed ✓);
   e2e 147 ✓. Freshness: `build-guests --check` and `--accelerated --check`
   exit 0.
3. **Four A/B snapshots** under
   `.local-artifacts/perimeter-reference-preparation/t58-finalization-copies/snapshots/`
   (ordinary/accelerated × baseline/candidate; exes
   `377e90a5…` / `58cff3e3…` / `82ee7fd0…` / `8ced73b3…`).
4. **Qualification (16-cell proof) at 7/16, all identical so far**: every
   completed cell's baseline and candidate outputs byte-identical; the
   completed ordinary sha values match the preserved production references
   (benchy-off `34886ccb…`, benchy-on `76eb4e1d…`, base-off `20e7204c…`,
   base-on `f60e5e9b…`). Remaining: ordinary-base-arachne-on (interrupted
   mid-warmup by the pause, its partial dir was deleted for a clean re-run)
   + all 8 accelerated cells.
5. **Harness**: `run-cell.ps1` (t57 protocol adapted), `run-all-proof.ps1`
   (resumable, skips done cells), `run-all-measure.ps1` (4 timing cells:
   classic supports-on benchy+base, both modes, 6 alternating pairs),
   `build-qualification.py`, `reduce_ab.py`. Raw runs under
   `.local-artifacts/.../t58-finalization-copies/{proof,measure}/`.

**Resume**: re-run
`pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t58-finalization-copies/run-all-proof.ps1`
(completes remaining 9 proof cells), then `build-qualification.py`, then
`run-all-measure.ps1`, then `reduce_ab.py`; restore production
artifacts afterwards (`target/dist-accelerated/developer/` from
`target/t58/standing-dist-accelerated-archive/`; target/release/pnp_cli.exe
is the baseline build after the campaign ends on the production tree). No
commit until the human rules KEEP/DROP.

### Continuation — 2026-10-11 (mid-measure restart recovery)

1. **Qualification completed 16/16 PASS** (see
   `evidence/t58-finalization-copies/FINDINGS.md` + `qualification.json`):
   baseline/candidate byte-identical in every cell, all clean, ordinary
   cells reproduced the preserved production references; fresh
   base-arachne-on reference `a1bc8110…`.
2. **Measure campaign interrupted once** by an opencode server restart
   during `ordinary-base-classic-on` (baseline m3 killed in prepass;
   6 clean rows discarded with the partial dir per the one-shot-guard 
   discipline). `ordinary-benchy-classic-on` had completed 14/14 rows
   pre-interruption and is retained. Full disclosure in FINDINGS.md.
3. **Campaign relaunched** (`run-all-measure.ps1`): remaining cells
   ordinary-base-classic-on (re-run), accelerated-benchy-classic-on,
   accelerated-base-classic-on. The driver skips completed cells.
   (Relaunch 1 was killed by a second server restart, again mid
   baseline-m3; relaunch 2 runs detached from the agent process tree —
   log at `.local-artifacts/.../t58-finalization-copies/measure-restart3.log`
   — so restarts no longer terminate it. (Relaunch 2 self-stopped at
   18:49 when the harness's 600 s per-run timeout killed a warmup under
   real machine load; timeout raised to 1800 s and disclosed in
   FINDINGS.md. Log for relaunch 3:
   `.local-artifacts/.../t58-finalization-copies/measure-restart4.log`.)

**Resume (if interrupted again)**: `run-all-measure.ps1` is idempotent
across cells (skips dirs that exist) and strict within a cell (one-shot
guard; a partially-written cell dir must be deleted wholesale before
re-running, per the t57 recovery discipline). After all 4 measure dirs
exist: `python docs/specs/perf-vs-orca/evidence/t58-finalization-copies/reduce_ab.py`,
then `restore-production.ps1`, then complete FINDINGS.md + tracker.

## Question

`execute_layer_finalization_with_instrumentation`
(`crates/slicer-runtime/src/layer_finalization.rs`) deep-clones the entire
`Vec<LayerCollectionIR>` before **every** finalization module
(`let mut candidate = layers.clone()`), runs the module on the clone, assigns it
back (`*layers = candidate`), and thereby drops the replaced vector. The clone
sits inside the module's instrumented bracket; the drop lands in the measured
inter-bracket gaps. The stage's four modules (`overhang-classifier-default`,
`part-cooling`, `skirt-brim`, `wipe-tower`, forced serial) therefore pay eight
whole-print copy/drop operations per slice. `execute_postpass_with_capture`
(`crates/slicer-runtime/src/postpass.rs`) adds one more whole-print copy
(`layer_irs.to_vec()`) plus a per-layer
`slicer_gcode::reconcile_finalization_travel` pass before `PostPass::GCodeEmit`.

Measured at HEAD (2026-10-09 refresh; raw captures gitignored, reduced tables
in `evidence/perf-refresh-2026-10-09/reduced/`):

- base classic supports-on postpass is **100.2–121.3 s in all 24 probe-free
  2026-10-09 A/B runs** (four arms; medians 106,282 / 107,373 / 114,072 /
  115,848 ms) — never attributed before; the post-repair 2026-09-27 run already
  recorded 113.7 s (`t27-serial-floor/results/run-phase-splits.csv`) but was not
  chased.
- Fresh profiled decomposition: stage 110,109 ms = module brackets 62,749 ms
  (15,062 / 15,055 / 15,982 / 16,650) + gaps 47,360 ms
  (8,258 / 12,595 / 13,340 / 13,167); module guest fuel ≤ 1.4 B of the cell's
  13,090 B, so the brackets are host copy time, not module execution.
- Negative control: base classic supports-off stage 2,040 ms (gaps 60 ms) for
  ~half the output.
- ≈11.1 s more sits between the instrumented stage brackets (the
  `to_vec`+reconcile suspect; code position, not probe-attributed).

Reduce the redundant whole-print copying without changing output. Candidate
shapes to falsify, not to assume: clone once per stage run and reuse the working
copy across the module sequence (the per-module clone reads as failure
atomicity, but `fin_result?` aborts the slice on error); gate the clone on
`module.ir_write_mask()`; per-layer copy-on-write; avoiding or cheapening the
pre-emit `to_vec`+reconcile pass. Preserve the documented failure semantics and
the finalization test surface (`crates/slicer-runtime/tests/executor/finalization_*`,
`crates/slicer-wasm-host` finalization tests).

## Acceptance

Output byte-identical on every measured cell (the take qualifies its matrix; the
ticket-57 precedent is the fresh 16-cell matrix), then the standing paired
ordinary + accelerated A/B on the matched classic supports-on benchy + base
cells; measured keep/drop to the human; no auto-commit. Base arachne
supports-on is the likely second beneficiary (the finalization stage is
generator-independent) and must be qualified inside the take, not assumed.

## Notes

- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md`, `reduced/finalization-gaps.json`,
  `reduced/t27-ab-postpass.json`; raw captures under
  `target/perf-refresh-2026-10-09/` and `target/t27-ab/campaign/measure/*/base/`.
- The stage is serial by construction (`pool size 1`), so these are wall
  seconds on the cell, unlike the accumulated per-layer columns.

## Measured answer (2026-10-11 → 2026-10-12 session)

**Implemented, qualified, timed. Standing paired gate: PASS in all four
cells; recommendation to the human: KEEP.** Full record:
`evidence/t58-finalization-copies/FINDINGS.md` (+ `qualification.json`,
`ab-reduction.json`).

1. Candidate (uncommitted, three files: `layer_finalization.rs`,
   `postpass.rs`, `pipeline.rs` in `crates/slicer-runtime/src/`): clone
   once per `execute_layer_finalization_with_instrumentation` stage run
   + single commit after last-module validation; pre-emit
   `to_vec` removed via `execute_postpass_with_owned_capture` with
   `&[...]` delegates unchanged. Failure semantics pinned by the existing
   executor rollback tests (234/234 ✓).
2. Qualification: fresh 16-cell output matrix, baseline/candidate
   byte-identical, all clean; ordinary cells reproduced the preserved
   production references; base-arachne-on qualified fresh (`a1bc8110…`).
3. Paired A/B (6 alternating pairs/cell, RAYON_NUM_THREADS=12,
   GetProcessTimes wall+cpu): wall medians base-classic-on
   **610.0 → 539.5 s (−10.48 %)** ordinary and
   **601.0 → 547.9 s (−8.76 %)** accelerated; benchy-classic-on
   **−1.21 % / −1.44 %**; cpu corroborates everywhere (−0.37…−2.45 %);
   6/6 pairs retained per cell, zero exclusions; every run
   byte-identical to its cell's reference sha.
4. Attribution (instrumented single runs, corroboration only):
   LayerFinalization stage **113.2 → 70.4 s (−37.7 %)**, mechanism as
   hypothesized — baseline's three inter-bracket drop gaps
   (8.0/13.9/13.1 s) + 13.6 s commit tail become **0 ms gaps**, one
   2.7 s upfront clone, one 10.1 s commit tail; postpass phase
   132.3 → 87.4 s.
5. The three-file diff plus tracker/evidence files remain **uncommitted**
   pending the human's KEEP/DROP ruling (no auto-commit rule).

Measured machine context, disclosed: the campaign ran under real
background load (Opera churn; one other-checkout test binary for a
window); three interruptions (two server restarts → partial cells
deleted and re-run per the t57 recovery discipline; one harness
600 s-timeout self-stop → timeout raised to 1800 s, infrastructure-only
and disclosed). Starved-pair exclusions: none.
