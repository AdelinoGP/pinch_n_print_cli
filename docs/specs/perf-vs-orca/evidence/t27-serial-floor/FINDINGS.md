# T27 work item 1 — slice-outside wall R, same-run decomposition

Wayfinder ticket: [Serial host floor: prepass built-ins and slice-outside wall](../../issues/27-serial-host-prepass-floor.md).
All figures below are **uninstrumented** same-run measurements (map recipe §3.2):
process wall and CPU from the matched-pair rig (`run_scoreboard.ps1`,
`GetProcessTimes`), internal `elapsed_ms` and per-phase `elapsed_ms` from the
same runs' default progress-event JSONL captures on disk. No instrumented run
contributes a number here.

## Method

For every PNP row of the four scoreboard batches (`s1-benchy`,
`s2-base`, `dev174-before`, `dev174-after`; 70 measured captures, 0 missing):

- `wall_seconds`, `cpu_seconds` — from the committed CSVs (process
  creation-to-exit, `Invoke-MeasuredProcess`).
- `elapsed_ms` — `slice_complete.elapsed_ms` (`run_slice_with_collector`'s
  `t0`, `crates/slicer-runtime/src/run.rs`).
- `phases_sum` — validation + prepass + per_layer + postpass
  `phase_complete.elapsed_ms`.
- `tail_gap = elapsed_ms − phases_sum` — inside `elapsed_ms`, outside all
  four phase spans.
- `R = wall − elapsed_ms` — process-outside-slice remainder (before `t0`:
  process start, runtime init, **model ingest** — `load_model`
  (`crates/pnp-cli/src/main.rs`) runs before `run_slice` — and after the
  `slice_complete` emission: profile flush (no-op uninstrumented), G-code
  write, process teardown).

Data tables: `results/run-phase-splits.csv` (per run),
`results/CELL-MEDIANS.md` (medians), `results/compute_phase_splits.py`
(reproduction script).

## Headline: the R figures ticket 12 budgeted are wrong by 15–36x

Ticket 12 anchored R = 51.2 s (pre-repair) / 122.2 s (post-repair) on
`dev174-repair` single runs and derived **c-min = 2.3x / 5.5x** for base
supports-on. Same-run measurement of the *same captures* gives:

| Run | process wall | elapsed_ms | R (same-run) |
| --- | ---: | ---: | ---: |
| `dev174-before` base classic-on ordinary m1 | 538.61 s | 536.76 s | **1.85 s** |
| `dev174-after` base classic-on ordinary m1 | 791.71 s | 788.31 s | **3.40 s** |
| `dev174-after` base classic-on accelerated m1 | 805.50 s | 800.45 s | **5.06 s** |

The 51.2/122.2 s figures were produced by subtracting **instrumented**
stage-phase sums (P+L+Q = 483.7 s / 660.5 s) from **uninstrumented** process
wall — the §3.2 trap ticket 12 itself warns about. Instrumentation inflation
(~1.0 on this pipeline's stage sums, per the dev174 evidence's own
self-consistency note) does **not** apply to the *missing* stages the
instrumented capture omits; the subtraction silently folded the
instrumentation delta of every unbracketed segment into "R".

Corrected corner factors: R/T on base classic-on = **0.15x–0.23x**
(3.4 s / 22.02 s), not 2.3x–5.5x. R is a non-term on every cell of the board.

## R per cell (medians, measured rows)

| Cell | n | median R (s) | % of median wall |
| --- | ---: | ---: | ---: |
| benchy classic off, ordinary | 3 | 0.18 | 0.9% |
| benchy classic off, accelerated | 3 | 0.19 | 0.9% |
| benchy arachne off, ordinary | 3 | 0.17 | 0.8% |
| benchy arachne off, accelerated | 3 | 0.18 | 0.9% |
| benchy classic on, ordinary | 3 | 0.21–0.62 (1 outlier 1.32) | 0.6–1.8% |
| benchy arachne on, ordinary | 3 | 0.20 | 0.6% |
| base classic off, ordinary | 3 | 1.87–2.32 | 0.7–0.9% |
| base arachne off, ordinary | 3 | 1.60–1.97 | 1.0–1.2% |
| base classic on (pre-repair), ordinary | 4 | 1.83–1.85 | 0.3% |
| base classic on (post-repair), ordinary | 1 | 3.40 | 0.4% |
| base arachne on (pre-repair), ordinary | 3 | 1.98 | 0.4% |
| base arachne on (post-repair), ordinary | 1 | 2.44 | 0.5% |
| accelerated counterparts | — | within +0.1–2.4 s of ordinary | ≤0.6% |

R scales with input + output size: benchy (11 MB STL → 7.4 MB G-code) ≈
0.2 s; base (123 MB STL → 21.9–54.5 MB G-code) ≈ 1.6–3.4 s (up to 5.1 s
accelerated). Components by construction: model ingest is entirely inside R
(the mesh is loaded by the caller before `run_slice`); G-code write is inside
R; process startup/teardown is inside R; module discovery + WASM compile
(~0.77 s, measured in `evidence/t15-criterion-refresh/FINDINGS.md` §"Where
`gate_evidence`'s 886 ms actually goes") is **inside `elapsed_ms`**, not R —
it sits between `t0` and `phase_start(validation)`, confirmed by the
slice_id-creation → validation-phase_start timestamp delta (1.14 s base m1,
1.46 s benchy m3).

## Elapsed-tail anatomy (inside `elapsed_ms`, outside the four phases)

Decomposed from event timestamps, three segments:

1. **Pre-validation** (`t0` → `phase_start(validation)`): module discovery +
   compile + config/plan prep. Measured 1.14–1.46 s across captures;
   ~0.77 s of it is the 24 guests' WASM compile (t15 finding), the rest
   config parse, wipe-tower scan, manifest ingestion, execution-plan build.
   Sits at the *head* of `elapsed_ms`, so the "tail_gap" field in the CSVs
   (which only sees phases_sum vs elapsed) actually contains this at the head
   plus segments 2–3 at the end.
2. **Prepass → per_layer handoff** (between `phase_complete(prepass)` and
   `phase_start(per_layer)`): ~0.75 s (benchy m3; ~1 ms elsewhere). Plan
   promotion + dispatcher construction between the two phase brackets.
3. **Post-postpass** (`phase_complete(postpass)` → `slice_complete`):
   `emit_host_support_diagnostics` replay + `layer_count` G-code-text scan +
   `estimate_print` + `slice_stats` build
   (`run_slice_with_collector`, `crates/slicer-runtime/src/run.rs`).
   - **Pre-repair base supports-on: this segment is 32.0–37.6 s and it is
     the 172,181 `module_error` replays** — `emit_host_support_diagnostics`
     (`crates/slicer-runtime/src/run.rs`) re-emits every DEV-174 "body
     rejected" diagnostic at slice end (first error 4.9 s after postpass
     completes; last error 76 ms before `slice_stats`). All 172,181 sit in
     this window; none during per_layer. This tail component is a
     *diagnostic-replay stream cost*, not pipeline work, and it disappears
     post-repair (0 errors).
   - **Post-repair / supports-off: the segment is `estimate_print` +
     scans**, and it does **not** scale with command count alone:
     benchy 0.07–0.23 s (~184–350k commands), base supports-off 0.17 s
     (~550k), base supports-on accelerated 6.8 s, base supports-on ordinary
     16.9 s — the last two on *byte-identical* 54.5 MB outputs
     (54478255 vs 54478478 bytes). The ordinary-vs-accelerated 6.8 → 16.9 s
     gap on identical output is **unexplained by static analysis** and needs
     a same-run probe (candidates: post-pass allocator/cache state,
     `estimate_print`'s segment-junction planning content sensitivity, M73
     elapsed-delta vector differences).

## Consequences for the ticket's model

- **R retires as a budget term** (c-min 0.15–0.23x worst cell). The serial
  floor is P + the elapsed-tail segments, not R.
- Ticket 12's per-cell budgets table needs a revision note: every c-min it
  printed is 10–35x too high; this only *weakens* the "serial terms bind"
  conclusion in R's favour — the P/L corners are unaffected.
- The 172,181-error replay stream (~32 s) is a **pre-repair-only artifact**
  (DEV-174); it is measurement-environment cost, not route work. Post-repair
  captures carry none of it.
- The post-postpass `estimate_print` segment (6.8–16.9 s on base supports-on)
  is a real serial cost *inside* `slice_complete.elapsed_ms` that no phase
  bracket sees. It is ticket-27-scope (it runs before `slice_complete`, on
  the slice critical path) but small against P (295–517 s).