# Allocator × Thread-Count Experiment — Plan and Execution Record

Started 2026-09-04. Status: executing.

This file records the approved plan and its live execution state. Ledger facts
here (versions, hashes, run counts) are snapshots taken at the time written;
re-derive before trusting.

## 1. Approved plan (user-confirmed Q17)

- Keep the reviewed optimizations and staircase fix uncommitted; **no default
  allocator change**.
- Build separate opt-in release variants: System (default), then mimalloc,
  then snmalloc automatically regardless of mimalloc's result. Preserve
  `AccountingAllocator` and identical compiler settings.
- Workload: `tmp/3dbenchy.stl` + `tmp/base.stl`, snapshotted
  `tmp/gpu-probe-tree.json` config, explicit `--module-dir modules/core-modules`.
- Thread counts **1** and **12** (machine reports 12 logical processors; 6
  physical cores).
- Per combination: 1 excluded warm-up + 5 measured runs, one slice at a time,
  interleaved allocator order.
- Measured runs uninstrumented: external wall (primary), OS process CPU, OS
  peak working set. Separate instrumented attribution runs, diagnostic only.
- Fresh System controls alongside the snmalloc trial.
- Validate builds, focused correctness, clippy, check-literals, guest
  freshness; retain outputs and investigate toolpath differences. Unexplained
  geometry differences block a correctness-qualified win.
- Report improvement / regression / inconclusive per workload. No adoption,
  commit, or pivot without user decision.

## 2. Verified facts at setup

- rustc 1.96.0, host x86_64-pc-windows-msvc. CMake present.
- Machine: 6 cores / 12 logical processors. `RAYON_NUM_THREADS` was unset.
- `RAYON_NUM_THREADS` is honored by rayon's global pool; all production
  parallel sites are `par_iter` on that pool. `num_cpus_guess`
  (`crates/slicer-runtime/src/run.rs`) only sizes arena bookkeeping, so the
  env var is the whole-CLI thread control.
- Verified latest versions (crates.io / upstream, 2026-09-04):
  mimalloc **0.1.52** (MIT, plain wrapper, GlobalAlloc, Send+Sync),
  snmalloc-rs **0.7.5** (MIT, default backend `build_cmake`).
- jemalloc (tikv-jemallocator) excluded: upstream marks Windows MSVC
  unsupported. rpmalloc-rs last published 0.2.2 (2021) — weaker maintenance.

## 3. Build identities (sha256, release profile, rustc 1.96.0)

| Variant | Feature | sha256 (prefix) | File |
| --- | --- | --- | --- |
| system | (default) | `51ee6c8f…` | `tmp/alloc-bench/bin/pnp_system.exe` |
| mimalloc | `alloc-mimalloc` | `e7e60a29…` | `tmp/alloc-bench/bin/pnp_mimalloc.exe` |
| snmalloc | `cargo build --release -p pnp-cli --features alloc-snmalloc` | `12c356ff…` | `tmp/alloc-bench/bin/pnp_snmalloc.exe` |

- Wiring: `crates/pnp-cli/src/main.rs` `ALLOC` statics are cfg-exclusive;
  compile_error on both features. `crates/pnp-cli/Cargo.toml` optional deps.
  AccountingAllocator stays outermost in all builds (report contract intact).
- snmalloc built with its default CMake backend **without** host CRT changes —
  no plan blocker fired.
- Config snapshot sha256: `17b7de02…` (`tmp/alloc-bench/config.json`,
  copied from `tmp/gpu-probe-tree.json`).
- Model hashes: benchy `6a07f34c…`, base `b26c8c79…`.
- `cargo xtask build-guests --check` exit 0 (fresh) before any timing.

## 4. Schedule (implemented in `tmp/alloc-bench/run_matrix.ps1`)

Phase 1 (system vs mimalloc), then phase 2 (fresh system vs snmalloc).

Per (threads, model) pair: `warmup(system), warmup(candidate)` then measured
slots 1–5 with alternating order (slot odd: system first, slot even: candidate
first). One slice at a time. Budget: phase 1 = 8 warmups + 40 measured;
phase 2 = same (system re-measured fresh). Attribution runs: separate
instrumented pass per combination (16+16), diagnostic only, excluded from
wall-time conclusions.

Runner fixes before launch (author deviations caught in review):
- `--module-dir` pointed at `target/guests/...` → corrected to
  `modules/core-modules` (matches plan + validated smokes).
- `-Warmup 1/0` passed as a value, but the harness declares `-Warmup` as a
  **switch** → warmup invocations now pass the bare switch, measured omit it.
- Trailing-comma parse error at the rebuilt `$argList` → fixed; parser-clean
  (0 errors) before launch.
- Phase-2 runner `tmp/alloc-bench/run_matrix_phase2.ps1` authored by agent,
  parse-clean, NOT executed; identical schedule/counts with snmalloc variant
  and `results/matrix_phase2.csv`.

- Phase-2 runner `tmp/alloc-bench/run_matrix_phase2.ps1` authored by agent,
  parse-clean, NOT executed; identical schedule/counts with snmalloc variant
  and `results/matrix_phase2.csv`.

Phase 1 launched (background) 2026-09-05 ~02:41 UTC; preflight passed; the
printed execution list matches the approved interleave token-for-token
(48 invocations: 8 warmups + 40 measured). Output stream:
`C:\Users\agpen\.local\share\opencode\shell\a7850842f4f5d01268dba5a2423b22f5441812bc\sh_06f720bc4001L1P8HRrLjr3amd.out`.
Phase 1 completed 2026-09-05: 48/48, exit 0, PASS summary.

Phase 2 first launch was **cancelled by a server restart** mid-matrix after
13 rows (benchy_t1 complete + base_t1 warmup; that partial CSV is archived by
the runner on relaunch). The restart also revealed the machine had quieted:
benchy@1t now measures ~126 s flat for BOTH variants (phase 1 same block:
153–346 s) — corroborating the starvation diagnosis of phase 1's 1-thread
samples. Phase 2 relaunched fresh (archived partial as
`matrix_phase2.csv.prev-*`).

Second server restart interrupted only the parent's monitoring shell; the
phase-2 matrix process (pwsh running `run_matrix_phase2.ps1`) survived and
continues. A first watcher shell reported the matrix dead — its exit-code condition was
inverted (`Get-Process` success → exit 1 → loop stopped). Corrected: the
matrix process was and is alive; new watcher waits with the fixed condition
(watch shell `sh_0731a13a4001OUrel7D5EuBESV`). Phase-2 partial through
`system_benchy_t1_r3` (7 rows): benchy@1t ~130–140 s for both variants on
the quiet machine — phase 1's 1-thread starvation diagnosis holds.

## 5. Correctness / compatibility checks

State: **complete — all green** (logs `target/alloc-check-*.log`).

- Focused bridge tests (host-algos): 24/24 pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo xtask check-literals`: 0 violations.
- Variant smokes (calicat, config snapshot, core-modules): all three exit 0;
  G-code **byte-identical across system / mimalloc / snmalloc**
  (sha256 `9350f2fe…9073`, 593071 bytes; identical slice_stats).
- `--report` on System and mimalloc: exit 0, nonzero HTML both; accounting
  enable path with a non-System inner allocator does not crash.
- `--help` on mimalloc and snmalloc: exit 0.
- Pre-existing, allocator-independent: repeated non-fatal
  `ERR_MALFORMED_LAYER_MARKER (code 12)` warnings from machine-gcode-emit,
  identical pattern across variants.
- Single-run caveat: byte-identity above is one run; DEV-093 same-binary
  run-to-run nondeterminism (medial-axis sliver) can still produce a few
  differing bytes — matrix output comparison judges per run.
- Not yet exercised: snmalloc `--report` (can be added before phase 2).

### Harness (`tmp/alloc-bench/run_bench.ps1`), validated on calicat
- CLI flag correction: the slice verb takes **`--model`**, not `--input`
  (`--input` is rejected). `tmp/PERF-HANDOFF.md` §4's command sketch uses the
  stale flag name. Harness maps `-InputModel` → `--model`.
- Validation (System, threads=2, calicat): exit 0; wall 2.42 s / 2.85 s
  (pre-fix) and 2.89 s (post-fix); identical G-code sha256 `9350f2fe…`
  across all validation runs, 593071 bytes.
- Post-validation fix: stderr is redirected for **both** modes so the default
  progress JSONL never writes to console during timing (I/O-noise control);
  re-validated, output unchanged.
- `RAYON_NUM_THREADS` is passed to the child explicitly (pwsh
  `Start-Process -Environment`); CPU = `TotalProcessorTime` (user+kernel)
  sampled on the last poll before exit; peak = max sampled
  `PeakWorkingSet64` at 100 ms polling (sampling can miss short spikes —
  reported as a limitation).
- Instrumented mode writes `<label>.jsonl` and appends no CSV metric row.

## 7. Experiment stopped (2026-09-05, user decision)

User stopped the experiment before phase 2 finished. What exists:

- **Phase 1 complete** (40 measured): results above.
- **Phase 2 partial, clean machine**: 11 measured rows
  (`matrix_phase2.csv.prev-20260905-192329`): benchy_t1 5+5 complete,
  base_t1 only system_r1. All clean-machine samples sit at cpu/wall 0.98–1.00
  (no starvation).
- The relaunch after the machine restart was killed per user request;
  zero slices ran from the aborted third attempt (CSV had 0 rows). All
  slicing processes were force-stopped (`tmp/alloc-bench/stop_matrix.ps1`,
  0 pnp processes remain).
- Attribution (instrumented) runs were **not** executed.

**Decision-relevant data:**

| comparison | clean-machine data | median delta |
| --- | --- | --- |
| benchy@1t system vs snmalloc | 5 + 5, all cpu/wall ≈ 1.0 | 131.1 s vs 132.7 s → **snmalloc +1.6 s (+1.2%)** |
| benchy@1t system vs mimalloc | phase-1 only (contended) | — |
| base@1t, base@12t, benchy@12t | snmalloc samples: none/insufficient | **unmeasured** |

**Answer to "does snmalloc improve anything at all?"** — No evidence of
improvement exists in the data collected. The only combination measured with
enough clean samples (benchy@1t, 5+5) shows snmalloc *slightly slower*
(+1.2% median wall, CPU equal within 1%, peak working set ~+5%: 406 vs 385 MB
median). Nothing measured so far suggests a wall-time, CPU, or memory win;
the 12-thread blocks — where the original contention hypothesis pointed —
were never reached for snmalloc, and phase 1 already showed mimalloc ≈ system
there. Stopping is sound; the allocator-contention hypothesis has no
supporting evidence from any measurement taken in this experiment.

State: phase 1 in progress.

**Environment event, 2026-09-05:** user reports other agents disputing compute
power; machine CPU is at 100% from external load during phase 1. Consequence
per user instruction: **base decisions on medians** (means reported too).
Additional load detector available in the recorded data: per-run
`cpu_seconds / wall_seconds` — a single-threaded slice should sit near ~1.0
when uncontended; low ratios mark samples where the child was starved. Affected
samples get a starved flag in the report rather than silent inclusion.

### Phase 1 complete (system vs mimalloc) — 48/48 invocations, exit 0

Wall-clock medians and supporting metrics (5 measured runs each):

| variant | block | med wall s | mean | min–max | med CPU s | med cpu/wall | med peak WS MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| system | benchy_t1 | 201.9 | 203.1 | 153–230 | 163.6 | 0.78 | 382 |
| mimalloc | benchy_t1 | 225.9 | 257.7 | 201–346 | 165.2 | 0.70 | 430 |
| system | base_t1 | 2250.6 | 2545.0 | 2240–3292 | 2241.3 | 1.00 | 2391 |
| mimalloc | base_t1 | 2896.3 | 3229.5 | 2216–4599 | 2498.4 | 0.86 | 2507 |
| system | benchy_t12 | 27.3 | 27.3 | 26.9–27.9 | 176.0 | 6.49 | 435 |
| mimalloc | benchy_t12 | 26.7 | 26.7 | 26.5–26.9 | 174.7 | 6.51 | 537 |
| system | base_t12 | 451.6 | 451.6 | 450–454 | 2908.3 | 6.44 | 2486 |
| mimalloc | base_t12 | 438.8 | 438.8 | 438–440 | 2887.5 | 6.58 | 2709 |

Starvation flags (cpu/wall far below expectation → child descheduled):
- base_t1: system r3 (0.77), mimalloc r1 (0.62), r2 (0.60) — these three
  samples are load-distorted; the *other seven* base_t1 runs all sit at
  cpu/wall 0.94–1.00.
- benchy_t1: ratios 0.49–0.90 across both variants (block started under
  heavier contention; ratios recover over the run).
- 12-thread blocks: cpu/wall ≈ 6.5 for all (12 logical CPUs, normal).

**Reading, medians only (per user instruction):**
- **12 threads: no wall-clock difference.** benchy_t12: 27.3 vs 26.7 s
  (−2%, but the spread within a variant is <1.5 s and the CPU medians are
  equal — this is not an allocator signal). base_t12: 451.6 vs 438.8 s
  (−3%, again ~4.5 s/3.5% median gap, CPU essentially equal). Both point to
  **mimalloc ≈ system** for wall time at high parallelism.
- **1 thread: mimalloc is slower on medians** (benchy +24 s; base +646 s),
  but for base_t1 the three starved samples are exactly where mimalloc's
  large values sit (r1, r2 at 4205/4599 s with ratio 0.60–0.62), while the
  two clean mimalloc runs (2216–2232 s) are **faster than every clean
  system run**. On benchy_t1 the slow mimalloc runs are also the starved
  ones. So the 1-thread "mimalloc slower" medians are contaminated by
  starvation; the clean-sample comparison suggests mimalloc ≈ system (or
  slightly better) when the machine is quiet, and CPU medians are near-equal
  everywhere (differences ≤ 12%).
- **Memory: mimalloc consistently uses more** (peak working set +13% on
  benchy_t12, +9% base_t12, +5% base_t1 medians) — real, systematic, and a
  genuine tradeoff signal worth reporting.
- Conclusion so far: **no allocator-driven wall-time win at 12 threads; the
  1-thread deltas are starvation-contaminated.** Proceed to phase 2
  (fresh System vs snmalloc) per plan regardless, since the snmalloc trial
  is user-mandated.

### Phase 1 output comparison

- Per-run G-code sha256 is **not stable within a variant** (4–5 distinct
  hashes across 5 runs in every block) — DEV-093 nondeterminism is active on
  these models at this config. Only system_base_t1 repeated a hash (2/5).
- Same-binary pairs differ by tens of bytes / a handful of lines (e.g.
  system_base_t12 r1 vs r2: +92 bytes; mimalloc_base_t1 r1 vs r2: −317 bytes).
- Cross-variant size deltas are the same magnitude as same-binary deltas, so
  **byte-level comparison cannot attribute any difference to the allocator**;
  the calicat smoke (byte-identical across all three variants) remains the
  strongest single-shot parity evidence. slice_stats and section counts
  match within the known sliver mechanism. No allocator-induced toolpath
  difference found; also none ruled out by hashing alone — structural
  section-count check to be added to the final report.