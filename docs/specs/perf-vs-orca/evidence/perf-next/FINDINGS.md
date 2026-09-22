# Perf-next findings (measurement-only scope, scratch summary)

Sources verified on disk this session: `tmp/perf-next/MEASUREMENTS.md`, `tmp/perf-next/EXPERIMENT.md`,
`tmp/perf-next/baseline.csv`, `tmp/perf-next/summary.json`. No builds, tests, or new timings were run.
All numbers below were read from those files; units are never summed or mixed (see EXPERIMENT.md "Baseline protocol").

## 1. Verified baseline medians/ranges (supports-off Benchy, 12 Rayon workers, n=4 non-warmup per generator)

| generator | median wall (s) | wall range (s) | median CPU (s) | CPU range (s) | CPU/wall ratios |
| --- | ---: | ---: | ---: | ---: | --- |
| classic | 21.9882 | 21.2958–24.2644 | 143.4141 | 141.4062–144.3906 | 6.497, 6.455, 5.951, 6.735 |
| arachne | 18.1533 | 17.9063–19.7151 | 74.8828 | 73.5312–75.5469 | 4.089, 3.832, 4.171, 4.098 |

- Warmup samples (`baseline-classic-warmup`, `baseline-arachne-warmup`) are excluded from medians.
- These are independent per-generator baselines, not a cross-generator speed comparison (EXPERIMENT.md "Interpretation limits").

## 2. Timing scope: current harness vs older polled harness

- Current: process CPU is final kernel+user time from Windows `GetProcessTimes` on the retained
  process handle; wall is that process's creation-to-exit interval, excluding harness startup/cleanup
  and avoiding the working-set polling tail.
- This is a **different timing scope** from historical harness Stopwatch values. The difference is
  scope, not a measured optimization, and must not be compared across scopes.

## 3. Selection validation (verified per-row in baseline.csv)

- Every sample has `expected_generator == validated_generator` (classic→classic, arachne→arachne),
  explicit config under `tmp/perf-flags/quiet-validation/benchy-supports-off-*.json`,
  `exit_code 0`, `completion_status ok`, 0 fatal / 0 non-fatal, `degraded False`.
- Per EXPERIMENT.md, accepted samples agree across explicit config, stderr claim-holder evidence,
  and the emitted G-code generator marker; missing/contradictory evidence would have been rejected.
- Arachne G-code hash is stable across all samples (`e39d8084107b…`); Classic hashes differ across
  samples (known same-binary Classic nondeterminism, `Inner wall` 244–248) — smoke evidence only,
  no fine parity proof (EXPERIMENT.md "Interpretation limits").

## 4. Attribution (separate captures, never acceptance wall measurements)

### Classic
- Worker elapsed (instrumented): `com.core.classic-perimeters` **90.893 s**; next `com.core.infill-linker` 10.733 s.
- Guest fuel (profile): `com.core.classic-perimeters` **1,012,844,470,264** of fuel_total
  **1,134,565,724,056** (89.272%); inside it, scope `polygon_ops::offset2_ex`
  (`crates/slicer-core/src/polygon_ops.rs`, `offset2_ex`) self-fuel **172,131,023,924** =
  **16.995%** of the module's total_fuel (960 calls).
- The **unmarked remainder** (classic-perimeters self-fuel beyond `offset2_ex`, ≈668.6 B fuel)
  **must not be attributed** to wall-flags fast path or clone costs without marked probes.

### Arachne
- Worker elapsed (instrumented): `com.core.arachne-perimeters` **11.363 s** vs
  `com.core.infill-linker` **10.626 s** — the two are near-parity, not one dominant site.
- Fuel-level attribution **inside** the arachne host algorithm was not measured (arachne-perimeters
  has no scoped self-fuel rows in `summary.json`); the historical "~75.3% preprocessing/graph"
  share **cannot be reaffirmed** from current data.

## 5. Verified source findings from investigators (costs unmeasured unless noted)

- **Classic wall assembly** (`modules/core-modules/classic-perimeters/src/lib.rs`, `emit_walls`):
  borrowing the last inset plus moving it into the collection can avoid an explicit clone and the
  initial "current owner" binding without reordering operations, retaining the borrowed wall
  assembly. Cost impact unmeasured.
- **Arachne 9-stage retention** (`crates/slicer-core/src/arachne/preprocess.rs`): production
  `preprocess_input_outline` calls `run_nine_stage_pipeline`, which materializes all 9 stage
  `Vec`s (stage 7 is even cloned into stage 8 via `stage7.clone()`), so peak retention holds all
  stages simultaneously even though only stage 9 is returned. Future streaming must **release**
  old stages; merely naming `stage1..9` locals does not ensure early drop. Cost unmeasured.
- **Avoid-duplicate-algorithm-pipelines**: proposal stands as finalized scope; its design still
  needs review.
- **Paint**: the ordinary production paint-segmentation path routes material/fuzzy paint through
  variants, not `material`/`fuzzy` `segment_annotations`; the direct path remains reachable from
  tests/adapters. Defer any change pending a demonstrated live reprojection workload
  (EXPERIMENT.md "Interpretation limits").
- **`offset2_ex` host migration**: now has a fuel-measured lead (scope rows above), but no
  process-wall measurement and no approved contract; not a candidate claim.

## 6. Recommendation

- Next step: **temporary marked probes** in Classic `emit_walls` and in the Arachne
  pipeline/preprocess/graph path — requires explicit user approval (EXPERIMENT.md approved scope;
  source probes and optimization edits need another confirmation).
- No KEEP/DROP optimization conclusion is possible without an implemented candidate.
- Continue ownership attribution; defer painted-path and contract work.

## 7. Validation status and limits

- Release build passed; `cargo xtask build-guests --check` exit 0; generator regression script and
  harness parse checks passed; all current baseline rows reviewed, no invalidator found.
- No Rust tests/check/clippy/full suite were run in this scope (scratch/documentation only).
- Raw logs and reports live under `tmp/perf-next/attribution/` and `tmp/perf-next/baseline-artifacts/`.
- Known Classic same-binary nondeterminism means no fine parity proof; environment snapshots are
  point observations, not proof of quiet; no tree/base cross-checks were in this bounded scope.
- Source identity is recorded in `tmp/perf-next/baseline-artifacts/artifact-manifest.json`; do not
  freeze a commit ID here — re-derive it from the manifest at point of use.