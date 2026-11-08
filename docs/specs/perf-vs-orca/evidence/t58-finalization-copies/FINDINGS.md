# t58 — LayerFinalization whole-print copying: clone-once + owned pre-emit
# reconcile. Evidence record.

## Pre-declared protocol (frozen before any measured run)

Candidate (single change set, uncommitted):
1. `execute_layer_finalization_with_instrumentation`
   (`crates/slicer-runtime/src/layer_finalization.rs`): ONE whole-print
   working copy per stage run; every module runs on it; the stage commits
   once after the last module succeeds and passes monotonic validation.
   The old shape paid one `layers.clone()` inside every module bracket plus
   one replaced-vector drop in every inter-bracket gap (8 whole-print
   copy/drop ops per 4-module slice).
2. `execute_postpass_with_owned_capture`
   (`crates/slicer-runtime/src/postpass.rs`): the production pipeline hands
   the finalized layers by value, so the pre-emit
   `reconcile_finalization_travel` pass mutates in place — the historical
   `to_vec` whole-print copy is gone. The `&[LayerCollectionIR]` variants
   (`execute_postpass`, the capture variant) delegate unchanged via
   `to_vec` and keep their exact external behavior; the visual-debug taps
   and every existing test call path are untouched.

Failed-atomicity contract preserved: a module that fails or writes
non-monotonic layers cannot commit (`layers` keeps its pre-stage value;
pinned by `finalization_executor_rejects_non_monotonic_layers`,
`finalization_fatal_error_is_instrumented_and_mutating_runner_rolls_back`).
The one semantic delta is recorded in the code comment: on a mid-stage
abort `layers` holds the pre-stage snapshot (the caller aborts the slice
either way via `fin_result?`).

Shapes considered and why not taken: per-module `ir_write_mask()` gating
(all four stage modules write layer IR; no opportunity), zero-copy
execution on `layers` directly (would change the pinned rollback
semantics), per-layer copy-on-write (complexity without a second win
beyond the removed per-module copies).

Qualification gate (before timing): fresh 16-cell output matrix
{benchy, base} x {classic, arachne} x {supports off/on} x
{ordinary, accelerated}, baseline vs candidate byte-identical
(sha256), clean outputs (status ok, degraded=false, 0 fatal, 0
non-fatal), non-vacuous TYPE counts. Completed-cell ordinary-mode shas
must also match the preserved production references where those exist
(benchy/classic `34886ccb…`/`76eb4e1d…`, base/classic
`20e7204c…`/`f60e5e9b…`).

Timing gate (standing, unchanged from t19/t27/t51/t57): paired ordinary +
accelerated A/B per cell with 6 alternating baseline/candidate pairs
plus a 2-run warmup per arm; external GetProcessTimes process wall and
kernel+user CPU at RAYON_NUM_THREADS=12; per-sample cpu/wall quoted;
starved pairs excluded whole (ratio < 0.5x the arm's own median ratio),
never resampled; a cell with retained pairs < 6 is inconclusive, not a
pass. Timing cells (pre-declared by the ticket): classic supports-on,
benchy + base, both modes. The full-cell wall/CPU verdict (KEEP/DROP)
is measured and returned to the human; candidates are never
auto-committed.

Baseline/candidate snapshot provenance:
- ordinary-baseline exe sha256 prefix `377e90a5…`
- ordinary-candidate exe sha256 prefix `58cff3e3…`
- accelerated-baseline exe sha256 prefix `82ee7fd0…`
- accelerated-candidate exe sha256 prefix `8ced73b3…`
Both arms built from the identical working tree; only the three-file
candidate change differs between arms (stash/pop around the baseline
build; prior uncommitted tree work shared by both arms). 24-module
external snapshots; slices run with
`--module-dir <snapshot>/modules --no-default-module-paths
--no-integrated-modules`; stderr checked for probe/isolation failures
and integrated-module shadowing per the t57 harness.

## Test and gate record (static, run at the candidate tree)

- `cargo check --workspace --all-targets` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask check-test-quality --report` — 0 findings.
- executor bucket 234/234 (`--test executor`); visual-debug tap tests
  (`visual_debug_postpass_tap_tdd`, `visual_debug_agent_overhead_tdd`) ok;
  integration 373 plus 1 pre-existing feature sentinel
  (`perimeter_spatial_capture`: panics by design without
  `perimeter-spatial-test-support`) and 1 `pnp-cli-locator` staleness
  artifact (fixed by rebuilding the debug binary, re-run 3/3 ok); e2e 147/147.
- `cargo xtask build-guests --check` exit 0 (before builds);
  `cargo xtask build-guests --accelerated --check` exit 0.

## Qualification (16 cells)

**PASS — 16/16 cells: baseline and candidate outputs byte-identical
(sha256 per cell in `qualification.json`), all clean (status ok,
degraded=false, 0 fatal, 0 non-fatal), non-vacuous TYPE counts.**
The four ordinary-mode cells with preserved production references
reproduced them exactly: benchy-classic-off `34886ccb…`,
benchy-classic-on `76eb4e1d…`, base-classic-off `20e7204c…`,
base-classic-on `f60e5e9b…` (run via
`python docs/specs/perf-vs-orca/evidence/t58-finalization-copies/build-qualification.py`,
exit 0; per-cell shas/walls/type counts in
[qualification.json](qualification.json)).

Recovery disclosures (no gate-failing cell rerun; both interruptions
before any recorded verdict, matching the t57 recovery pattern):
- The 2026-10-10 pause (human request) stopped the driver mid-warmup on
  `ordinary-base-arachne-on`; its partial dir was deleted and the cell
  re-ran clean.
- On the final cell `accelerated-base-arachne-on` the candidate warmup
  died mid-prepass with no panic/error in stderr (silently terminated,
  absent from the process list; the same binary's own baseline and seven
  other accelerated cells ran clean in the same batch). The partial dir
  was deleted, the cell re-ran both arms, and the checker re-proved all
  16 cells from the raw rows. Recovered cell: baseline 449.601 s /
  candidate 398.339 s, both `a1bc8110…`.
- 2026-10-10 ~21:4x (opencode server restart): the measure-phase driver
  was killed during its second cell `ordinary-base-classic-on`. The cell
  had completed 6 clean rows (both warmups + pairs 1–2 in interleaved
  order); `baseline-m3` was killed mid-run (its stderr.jsonl is
  truncated at 25,847 bytes versus ~337,02x for the six complete runs;
  the last line is `phase_start` for `prepass`; no `.gcode`, no
  `rows.csv` entry, and no error in stderr — consistent with external
  termination, not a slicer failure). The whole partial cell dir was
  deleted and the cell re-ran from scratch per the one-shot-guard
  discipline; the completed `ordinary-benchy-classic-on` cell predates
  the interruption and is retained unmodified (14/14 rows). No timing
  verdict was recorded at interruption time, so no gate-failing cell
  rerun is implied by the rerun. A second restart killed the first relaunch
  of the campaign in the identical way (2026-10-11 ~15:48, again mid
  `baseline-m3`, after 6 clean rows; stderr frozen just past
  `phase_start postpass` at ~435 s into the run, no panic/error — the
  600 s per-run timeout cannot fire below 600 s and the same binary had
  just completed three qualifying baseline runs). The partial dir was
  again deleted whole. After the second interruption the campaign is
  launched **detached from the agent process tree**
  (`Start-Process pwsh` with its own output log
  `measure-restart3.log` under the artifacts root), so opencode server
  restarts no longer terminate the measurement; only a machine reboot
  or explicit kill can.
- 2026-10-11 18:49 (third interruption, self-detected): the detached
  relaunch's first `ordinary-base-classic-on` warmup was killed by the
  harness's own 600 s `WaitForExit` timeout (`timeout:
  ordinary-base-classic-on-baseline-w0` in `measure-restart3.err`).
  This was environmental contention, not a slicer failure: the killed
  run had completed the `per_layer` phase and was entering `postpass`
  (stderr frozen at the same ~206 KB boundary as the restart-kill
  freeze), and baseline walls on this cell had crept 526.8 s
  (2026-10-10, quiet machine) → 559.1 s (2026-10-11 14:48, Opera
  autostart active since 14:27) → >600 s (18:39, Opera plus a test
  binary from the *other* checkout `D:\slicerProject\pinch_n_print_cli`
  at ~2.5 cores). The partial dir was deleted whole. In response the
  per-run harness timeout was raised 600000 ms → 1800000 ms in
  `run-cell.ps1` — an infrastructure-only change that cannot affect any
  measured number (GetProcessTimes measures the child identically; the
  timeout only decides when an already-abnormal run is killed; it never
  fires for healthy runs) and is equally inherited by both arms of the
  remaining cells. Machine-load observations are recorded because they
  gate the paired design: heavy concurrent work inflates both arms'
  walls and can starve pairs toward the documented inconclusive rule
  (starved pairs excluded, <6 retained pairs = inconclusive).

## Paired A/B (timing)

**VERDICT: PASS — both clauses (candidate median wall win + CPU not
contradicting) in all four pre-declared cells; 6/6 pairs retained in
every cell; zero starved-pair exclusions.**

Reduced by `reduce_ab.py` (fixed root-path bug before the run — same
defect class as the qualification checker; reduction-plumbing only, no
measurement-affecting change) from the raw `rows.csv` files:

| cell | baseline wall med | candidate wall med | paired-median Δwall | Δcpu | pairs |
|---|---|---|---|---|---|
| ordinary-benchy-classic-on | 26.4 s | 26.1 s | **−1.21 %** | −0.52 % | 6/6 |
| ordinary-base-classic-on | 610.0 s | 539.5 s | **−10.48 %** (−70.5 s) | −2.33 % | 6/6 |
| accelerated-benchy-classic-on | 28.8 s | 28.2 s | **−1.44 %** | −0.37 % | 6/6 |
| accelerated-base-classic-on | 601.0 s | 547.9 s | **−8.76 %** (−53.1 s) | −2.45 % | 6/6 |

Every measured run of all four cells produced byte-identical outputs
within its cell (baseline vs candidate sha256 equal per pair;
`ordinary-base` runs all `f60e5e9b…`, ordinary-benchy runs all
`76eb4e1d…`, matching the preserved production references). Full
per-pair detail: [ab-reduction.json](ab-reduction.json).

Machine-load context (measured): the entire campaign ran with Opera
browser churn (~1.5 cores aggregate) and, briefly, a test binary from
the *other* checkout; wall medians inflated vs the quiet-machine proof
runs but the paired alternation shared the load within each pair, and
the cpu/wall corroboration clause held everywhere. Candidate CPU
medians are lower alongside the wall wins, so nothing here is
wall-only luck.

### Attribution (corroboration only; single instrumented runs, ordinary base-classic-on)

Run with `--instrument-stderr` on the same snapshot binaries, ~8 min
apart, output under `attribution/` (gitignored). Not gate evidence.

- `PostPass::LayerFinalization` `stage_complete`: baseline **113,163 ms**
  (reproduces the 2026-10-09 refresh headline 100.2–121.3 s) →
  candidate **70,425 ms** (−42.7 s, −37.7 %).
- Mechanism, exactly as the ticket hypothesized: baseline pays the four
  module brackets **plus three inter-bracket commit-drop gaps
  (8,040 / 13,934 / 13,073 ms) and a 13,552 ms stage-commit tail**; the
  candidate's inter-bracket gaps are **0 / 0 / 0 ms** (each
  `module_start` begins the instant the previous `module_complete`
  fires — the working copy is carried through), with the whole stage
  paying **one 2,661 ms upfront clone and one 10,129 ms commit tail**.
- `postpass` phase: 132,304 → 87,376 ms (the stage delta plus the
  removed pre-emit whole-print copy at the handoff into emit).
- Noise floor, measured in the same pair of runs: the untouched
  `prepass` phase differs by −86.3 s between the two arms (435,684 →
  349,355 ms), so single-run phase deltas below that scale carry no
  evidentiary weight; only the stage-structure claim (gaps → 0) is
  robust at single-run resolution. The benchy cells' small wall wins
  (−1.2 / −1.4 %) are consistent with the effect being real but small
  relative to the base fixture's large whole-print copies.

## Restoration

- `restore-production.ps1` after campaign end: standing accelerated
  snapshot restored from `target/t58/standing-dist-accelerated-archive/`;
  freshness gates re-checked — ordinary `build-guests --check` exit 0,
  accelerated `--accelerated --check` exit 0. The working tree keeps the
  three-file candidate diff **uncommitted**; nothing is merged until the
  human accepts the recommendation.