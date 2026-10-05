# t15 Criterion bench refresh — baselines and per-bench trust verdicts

Ticket: [Criterion bench refresh](../../issues/15-criterion-bench-refresh.md).
Captured 2026-09-23 (AFK agent session). All numbers measured this session on
the Windows dev box unless dated otherwise; every figure is a ledger fact,
re-derive at the point of use.

## Protocol

- Guest freshness gate first: `cargo xtask build-guests --check` → **exit 0**
  (fresh). In-tree artifacts under `modules/core-modules/*/*.wasm` all present
  (24 guests, non-placeholder).
  - One in-tree-vs-source mtime hit (`tree-support-planner`) was adjudicated
    benign: the newer file is `tests/tree_family_tdd.rs` (dev-side; commit
    `3f51b7b9` touched only that test plus `traditional-support-planner/src/lib.rs`,
    whose shared-target artifact was rebuilt 2026-09-23 01:50). Not a code
    input of the artifact — consistent with the exit-0 fingerprint.
- Bench targets compiled once (`cargo bench --workspace --no-run`); `pnp_cli`
  built in the matching (release) profile for `gate_evidence`
  (`pnp_cli_bin` (`crates/pnp-cli-locator/src/lib.rs`) resolves the
  same-profile sibling binary and hard-fails on staleness — no fallback).
- One run per bench, sequential (no concurrent CPU load from this session;
  see Caveats), output teed to `target/bench-<name>.log`, criterion on-disk
  baselines land in `target/criterion/**/base/` (gitignored, regenerable).
- Invocations per `.agents/aux-commands.md` / `PERF-HANDOFF` §7a.
- Numbers are read back from `target/criterion/**/new/estimates.json` plus
  `sample.json` / `tukey.json` / `benchmark.json` — never transcribed from the
  criterion console (see finding #1 under "What the read-back-from-disk
  discipline changed"). Rendered tables: [BASELINES.md](BASELINES.md);
  machine-readable twin: [baselines.json](baselines.json).
- Run window 2026-09-23 22:16–22:48 local; `tasklist` re-checked at start
  (`NO_CARGO_PROCS`) and again at the end. `git status` shows only docs
  changes — the benches wrote nothing to tracked files.
- The fixture-age audit below is the pre-run (handoff session) record; the two
  bolded content commits (`aa7f0487`, `5c16b6ee`) and the "later touches are
  incidental" claim for `shell_classification` / `gate_evidence` were
  re-verified against `git log` in the run session.

## Fixture-age audit (the "last touched" trap)

`git log` name-only plus pickaxe (`-S`) on each fixture's defining function to
separate content changes from incidental sweeps (crate renames, clippy/FRU
literal sweeps, WIT vocabulary fixes):

| Bench | Fixture content since | Later touches (all incidental) |
| --- | --- | --- |
| `polygon_ops` | 2026-05-17 scaffold `f806f31d` | 2026-05-18 `21eadc85` (TASK-201 precision config wiring only) |
| `mesh_ops` | 2026-05-17 scaffold `f806f31d` | 2026-05-17 `373243ff` (DecimateConfigBuilder wiring) |
| `pipeline` | scaffold (slicer-host era, ≤2026-05-29 `858aa0bf` rename) | 2026-06-02 gitignore chore; 2026-06-08 `5fbb786d` doc fixture name (benchy→regression_wedge) |
| `per_stage` | scaffold (≤2026-05-29 `858aa0bf` rename) | 2026-07-17 `2053aa65`, 2026-07-30 `f80755e0` (WIT vocabulary) |
| `wasm_modules` | scaffold (≤2026-05-29 `858aa0bf` rename) | 2026-06-01/02 P83 + gitignore; 2026-06-13 `23880a47` (stage-name list edit only) |
| `shell_classification` | **2026-07-24 `aa7f0487`** (fixture rewrite + timeline parallelization) | 2026-07-25 rustfmt; 2026-08-09 ×3 packet-197 FRU/clippy sweeps |
| `gate_evidence` | **2026-07-03 `5c16b6ee`** (DEV-026 downgrade + wedge fixture) | 2026-07-28 WIT review; 2026-07-31 / 2026-08-05 locator moves |

## Per-bench trust verdicts

Verdict vocabulary (ticket 15): *does real work* / *measures what it claims* /
*stale*.

| Bench | Verdict | Basis |
| --- | --- | --- |
| `polygon_ops` | measures what it claims (real work) | see below |
| `mesh_ops` | measures what it claims (real work) | see below |
| `pipeline` | measures what it claims (within declared v1 scope) | see below |
| `per_stage` | measures what it claims, but its name oversells scope (v2 TODO open) | see below |
| `wasm_modules` | measures what it claims (real artifacts; "v1 stub" note is scope, not degeneracy) | see below |
| `shell_classification` | measures what it claims (real work; see measured magnitudes) | see below |
| `gate_evidence` | measures what it claims (real subprocess slice) | see below |

Basis per bench (source audit; measured magnitudes under Baselines corroborate):

- **`polygon_ops`** — synthetic square grids (12 mm side, 10 mm pitch → 2 mm
  neighbour overlap) so union/intersection/difference have real intersections;
  16/64/256-square sizes; `offset(..., 0.4, Miter, 0.0)` walks the TASK-201
  arc-tolerance parameter at legacy precision. Fixtures built outside
  `b.iter`; the iteration body is the op alone. Real work: yes (Clipper2 set
  ops on overlapping grids). Limitation to record: axis-aligned squares are
  not real perimeter geometry — no thin slivers, no arc-tolerance > 0 paths —
  so this bench is a polygon-op cost instrument (e.g. clipper2 version A/B,
  ticket 17's use), not a pipeline-cost predictor.
- **`mesh_ops`** — real STEP fixtures (`cube.step`, `assembly.step` under
  `crates/slicer-helpers/tests/resources/`); `iter_batched` clone-per-iteration
  for the by-value ops (`repair`, `decimate`), which is the correct batch
  shape. Cube-scale meshes only; decimate runs at `target_ratio(0.5)`.
  **Scope note:** these helpers have no production caller on the slice path —
  `pnp_cli slice` loads meshes via `slicer-model-io`'s `load_stl` — so this is
  a helper-microbench, not a pipeline-cost instrument; the two fixture limits
  in findings #2/#3 cannot move a matched-pair cell.
- **`pipeline`** — v1 scope is instrumentation overhead: synthetic bracket
  sequences (10/100/1000 layers × 3 modules) driving the
  `PipelineInstrumentation` trait directly (Noop vs `Collector`), no real
  pipeline or mesh. It measures what it claims (report-stack overhead); it is
  **not** an end-to-end pipeline bench despite the file name, and its module
  doc says so (TODO (v2) open — real `run_pipeline` driver still unbuilt).
- **`per_stage`** — only `compute_serial_edges_for_stage` over synthetic
  module chains (8/32/128 modules, N−1 IrWriteRead edges) — the plan-freeze
  serial-edge helper, called once per stage in production. Real work but a
  tiny surface: the name oversells scope, and the module doc's TODO (v2)
  (per-stage executors against a snapshotted `Blackboard`) is explicitly
  unimplemented. Trust the number for the helper; do not read it as stage cost.
- **`wasm_modules`** — real staged artifacts (24 guests; discovery asserts
  non-placeholder bytes), benching manifest ingestion
  (`load_module_from_paths`) and `WasmEngine::compile_component` per
  component, plus an `export_for_stage_id` micro. The "v1 stub" note in the
  handoff refers to scope (full `run_stage` dispatch bench deferred to v3),
  not degeneracy — every measured path executes real parsing/JIT work on real
  artifacts. The discovered set is current by construction (it benches
  whatever is staged at run time).
- **`shell_classification`** — the rewritten fixture (2026-07-24 `aa7f0487`)
  gives every layer a breathing radius and laterally drifting hole, so
  `difference(current, neighbour)` is a non-empty thin high-vertex ring and
  `apply_opening` executes its two dominant `offset` passes (the exact
  short-circuit that invalidated the previous fixture — empty diffs, µs
  numbers — is designed out). Default shell count 3 (Pass 2 walks two steps
  per seed); sweeps timeline length 120/240/480 at 1 object (the real-print
  shape: `timelines=1 lengths=[480]` on a 0.1 mm benchy) with 16-object rows
  as the control axis. `sample_size(10)` with `iter_batched(PerIteration)`.
- **`gate_evidence`** — real `pnp_cli slice` subprocess against
  `resources/regression_wedge.stl` with `resources/test_config/gate_evidence_50l.json`
  (coerces the 40 mm fixture to exactly 50 layers), `sample_size(10)`;
  measure-and-report against the `docs/12` ≤ 10 s bound (DEV-026 downgrade
  decision — wall only, never peak RSS). Real work by construction: every
  iteration is a full slice through the real WASM pipeline. The binary is
  same-profile and staleness-gated by `pnp_cli_bin`, so a plausible-but-stale
  measurement cannot happen silently.

## Baselines

All 82 leaves measured; `base/` and `new/` both exist for every leaf (first
run, so `base == new`). Full tables: [BASELINES.md](BASELINES.md).
Machine-readable: [baselines.json](baselines.json).

Headline per bench (mean of the per-iteration sample, from `estimates.json`):

| Benchmark | Leaves | Representative magnitudes |
| --- | --- | --- |
| `polygon_ops` | 12 | union/difference/intersection 16 → 256 squares: 8.49 → 199.29 µs (union); `offset` 10.86 / 44.28 / 242.52 µs |
| `mesh_ops` | 4 | `import_step` cube 1.44 ms, assembly 2.14 ms; `repair/cube` 2.95 µs; `decimate/cube_default` 3.75 µs |
| `pipeline` | 8 | noop bracket drive 1.6 / 16.8 / 160 µs; collector 19.3 / 190.4 µs / 1.85 ms; allocator fast-path ns-scale |
| `per_stage` | 3 | `compute_serial_edges` 942.58 ns / 8.32 µs / 97.25 µs at 8 / 32 / 128 modules |
| `wasm_modules` | 49 | `export_name_for_stage` 6.73 ns; `manifest_load` 85.9–400.6 µs across 24 modules; `compile_component` 9.1–97.0 ms across 24 modules |
| `shell_classification` | 5 | 1 obj × 120/240/480 layers = 2.55 / 3.92 / 7.15 ms; 16 obj × 120/480 = 49.1 / 131.7 ms |
| `gate_evidence` | 1 | full 50-layer `pnp_cli slice` subprocess: **885.9 ms** mean, 890.1 ms median, 10 samples |

`gate_evidence`'s 885.9 ms sits comfortably inside `docs/12`'s ≤ 10 s bound
(one order of magnitude of headroom) and every one of the 13 logged slice runs
reported clean: `slice_complete` — `status: ok`, `degraded: false`,
`fatal_error_count: 0`, `non_fatal_error_count: 0` (13/13), and `slice_stats` —
`layer_count: 50`, `status: ok` (13/13). That is the DEV-026 gate's
measure-and-report contract, satisfied.

### Where `gate_evidence`'s 886 ms actually goes — ~89% is pre-validation

Read from the log's own timestamps (the `slice_id` embeds its creation epoch in
ms, and `phase_start(validation)` is the first event): **775 ms of the 873 ms
median run happens before the `validation` phase begins** — 88.8% (the gap
ranges 747–849 ms across the 13 runs). The bracketed phases account for only
~92 ms:

| Segment | Median (ms, 13 runs) |
| --- | --- |
| module load + compile (pre-`validation`) | **775** |
| `validation` | 2 |
| `prepass` | 32 |
| `per_layer` | 45 |
| `postpass` | 13 |
| **`slice_complete.elapsed_ms`** | **873** |

(Phase medians recomputed over all 13 logged slice runs. The log has 13 runs
while criterion kept `sample_size(10)`: the extra 3 are warm-up iterations,
which is where the 977 ms first run lives — it is *not* in criterion's 10
samples, whose wall times span 854.8–900.2 ms. Two clocks are in play and
should not be conflated: criterion's per-sample wall (mean 885.9 ms) is
`Command::status()` around the whole subprocess, while the JSONL
`slice_complete.elapsed_ms` (mean 877.8 ms, median 873 ms) starts inside
`run_slice` after process start, STL load, and config resolution — an ~8 ms
mean difference, which is that pre-`run_slice` process overhead.)

The pre-validation gap is not unattributed mystery work: it is
**99.5% accounted for** by the sum of this run's own `wasm_modules`
`compile_component` measurements over the same 24 artifacts — 769.3 ms
compiled + 3.7 ms manifest-ingested = 773.0 ms against a ~775 ms gap (the
`compile_component` leaf means were measured in a separate process, so treat
99.5% as an order-of-magnitude reconciliation, not an identity). The call
site is `load_live_modules_for_plan_with_integrated`
(`crates/slicer-runtime/src/run.rs`), invoked at the "Discover and plan every
module under `--module-dir`" step that runs *before* the first
`phase_start(validation)` event.

**Why this matters for this map (and is recorded rather than judged):** it is
the first measured instance of the compile-at-startup cost profile — a
per-invocation, one-off, ~0.77 s **fixed** term that is *not* inside any
bracketed phase and therefore invisible to *phase-wall* accounting (it is
inside `slice_complete.elapsed_ms`, which is why that reads 873 ms while the
phases sum to ~92 ms).

**It is already inside the scoreboard's numbers on the PNP side.** The
matched-pair rig measures process creation-to-exit
(`evidence/matched-pair/run_scoreboard.ps1`'s "process creation-to-exit wall
clock"), so every PNP cell in `SCOREBOARD.md` already carries this ~0.77 s,
constant across cells (same 24 modules). It does not change any cell's winner.
Whether Orca pays anything comparable at startup is **not known** — the map
records Orca-side attribution as unmeasured ("GUI-subsystem binary, little
introspection"), so no subtraction from the Orca side is justified.

It is not a new ticket: [Serial host
floor](../../issues/27-serial-host-prepass-floor.md) already owns this exact term — its work
item 1 asks for `slice-outside wall` to be attributed and names "module
discovery/instantiation (24 guests)" as one of the components. This section
supplies a measured 769 ms (compile) + 4 ms (manifest) starting value for that
component on the 50-layer wedge, which is the shape ticket 27 needs for the
per-cell figures. Recording it changes no decision.
Also note the DEV-026 log row's
"~438 ms median" vintage (2026-07-03, debug profile) is roughly half this
number: do not compare the two figures — different profile, different machine
state, and the row predates the current module set.

## What the read-back-from-disk discipline changed (four findings)

These came out of reading criterion's own artifacts — and, for the
pre-validation term above, the log's own timestamps — rather than criterion's
console summary. #1 is a measurement-semantics trap; #2 and #3 are
fixture-scope limits on the code under test; #4 retires a stale "stub" label.
None is a defect in the benched code.

### 1. The criterion console's `time:` is the **slope**, not the mean

In Linear sampling mode criterion sets `Estimates::typical()` to the regression
slope (`criterion::estimate::Estimates::typical` — `self.slope.as_ref().unwrap_or(&self.mean)`)
and the console `time: [...]` line prints `typical`, not `mean`. Verified
exactly on `mesh_ops/import_step/cube`: the console line is
`time: [1.5331 ms 1.5688 ms 1.6009 ms]`, which matches the JSON **slope** CI
(`1.5331 / 1.5688 / 1.6009 ms`) to all printed digits, while the JSON **mean**
is `1.4445 ms` (CI `1.4043–1.4849 ms`). A reader who quotes the console is off
by **+8.61%** there, and by −7.00% on `polygon_ops/union/256`. 56 of the 82
leaves are Linear (slope present); the remaining 26 are Flat, where
`typical()` falls back to `mean` and the console is exact. Per-leaf divergence
column: `baselines.json`'s `console_typical_vs_mean_pct`.

Criterion's console is *internally* inconsistent on this: the `change:` line
that accompanies a `--baseline` comparison is computed from **means**
(`compare.rs::estimates`'s `stats` → `a.mean() / b.mean() - 1.`), not slopes.
So on a Linear leaf "console said X ms, now says Y ms" is a slope narrative
while "change: +N%" is a mean comparison. Here they differ by 8.61% on
`mesh_ops/import_step/cube` and 7.00% on `polygon_ops/union/256` — and
[clipper2 1.1.0 upstream
evidence](../../issues/16-clipper2-1-1-0-upstream-evidence.md) concluded the version bump is
purely additive, so ticket 17's *expected* cost delta is near zero and a 7%
statistic mismatch is enough to swamp it. Compare **like-for-like** — all-slope
or all-mean on both sides, never a slope `time:` against a mean `change:`.

### 2. `mesh_ops/repair/cube` measures a no-op scan on this fixture

The fixture is real work for `import_step` (a 10 mm cube triangulates to 12
triangles; assembly 24 — verified this session via `pnp_cli mesh import`
against the same `crates/slicer-helpers/tests/resources/` files the bench
loads), but the cube is *perfectly clean*, so `repair`'s three phases all find
nothing: `pnp_cli mesh repair --stats` on the same fixture reports
`degenerate_removed: 0`, `faces_reoriented: 0`, `open_edges_closed: 0`. The
2.95 µs is the cost of **scanning** a 12-triangle mesh and returning it, not the
cost of repairing anything. Contrast `resources/regression_wedge.stl` (80
triangles, 5 components): `faces_reoriented: 12`, `open_edges_closed: 56`. The
number is reproducible and not degenerate-µs-in-the-shell-classification-sense
(µs is the right order for a 12-triangle scan), but it does not represent
repair's cost on meshes that need repair. **Ticket 15 verdict for this leaf:
measures what it claims (the scan), but the fixture exercises no repair path —
a non-defective fixture-scope limitation to record, not a regression.**

### 3. `mesh_ops/decimate/cube_default` cannot reach its declared target

`decimate(cube, target_ratio = 0.5)` returns `final_triangle_count: 12` from
`original_triangle_count: 12` — target 6 — with `achieved_error: 0.0`. It is
*not* a stub: the bench's config reaches `meshopt::simplify_decoder`
(`crates/slicer-helpers/src/decimate.rs`) and the same call on an 876-triangle
mesh (`resources/calicat.stl`) reaches its 50% target (438, `target_reached:
true`).

The measured mechanism is the **error budget**, not a code path: the bench uses
the `DecimateConfigBuilder` default `max_error = 0.01` (it never calls
`.max_error(...)`), and collapsing any edge of this 10 mm box moves a vertex
far enough to blow that budget. Swept this session on the same fixture —

| `max_error` | final triangles | achieved error | target reached |
| --- | --- | --- | --- |
| 0.01 (bench default) … 0.4 | 12 | 0.0 | false |
| 0.5, 0.6 | 6 | 0.4472 | true |

— so the threshold sits between 0.4 and 0.5. So the 3.75 µs is real meshopt
work that is *rejected at the configured budget*, not simplification work.
**Ticket 15 verdict: measures what it claims (the call), but the fixture cannot
exhibit a successful simplification at the budget the bench configures — same
class as #2.** Fixing it is fixture-or-config work, not a bench-contract change;
no such fix is in scope here.

### 4. `export_name_for_stage` is a 13-lookup micro, not a single lookup

`wasm_modules/export_name_for_stage`'s iteration body loops over 13 stage-id
strings and calls `slicer_schema::export_for_stage_id` on each (12 real stage
ids plus `"Unknown::Made::Up"` as a deliberate miss). Each call is a linear
scan over the 17-entry `STAGES` table (`crates/slicer-schema/src/lib.rs`,
`export_for_stage_id`'s `.iter().find(...)`), returning `&'static str` with no
allocation. So the measured 6.73 ns is **the 13-lookup loop, not one lookup** —
roughly 0.5 ns per call on 12 hits and 1 miss, which is the right order for a
scan of 17 short string comparisons. Read it as a loop, not a per-call figure;
the leaf is otherwise fine and the handoff's "v1 stub" note refers to the
deferred `run_stage` dispatch bench (module doc's v3 TODO), not to this micro.
No action.

## Per-bench trust verdicts at HEAD (measured)

The source-derived verdicts above were written pre-run; these are the measured
counterparts. Only two change, both by narrowing (cases #2, #3).

| Bench | Measured verdict |
| --- | --- |
| `polygon_ops` | **trustworthy** — real Clipper2 work; µs scale grows monotonically with grid size for all four ops (union 8.49 → 35.03 → 199.29 µs at 16/64/256; offset 10.86 → 44.28 → 242.52 µs) |
| `mesh_ops` | **import_step trustworthy**; **`repair/cube` and `decimate/cube_default` trustworthy as calls but fixture-limited** (cases #2, #3 above) |
| `pipeline` | **trustworthy within v1 scope** — ns/µs/ms scale, collector vs noop separation is real and grows with layer count |
| `per_stage` | **trustworthy for the helper** — 942.58 ns → 97.25 µs across 8 → 128 modules; still not stage cost |
| `wasm_modules` | **trustworthy** — 24 real components; compile_component 9.1–97.0 ms is JIT work on real artifacts |
| `shell_classification` | **trustworthy** — ms-scale, grows with layers (2.55 → 7.15 ms for 120 → 480 layers at 1 object) and with objects; no repeat of the µs-scale short-circuit that invalidated the old fixture |
| `gate_evidence` | **trustworthy** — real subprocess; 13/13 slice runs clean (`status:ok`, `degraded:false`, 0 fatal, 0 non-fatal; `layer_count:50`); 885.9 ms against a 10 s bound |

## Caveats

- **Machine:** AMD Ryzen 5 7600X (6 cores / 12 logical), 33,979,981,824 bytes
  (~31.6 GiB) RAM, Windows 11 Home 10.0.26200. HEAD `3f51b7b9`.
- **Contention:** the handoff flagged a parallel `cargo test -p slicer-runtime
  --test integration` session that had lock-contended the earlier build phase.
  `tasklist` was checked before the first run and was clean (`NO_CARGO_PROCS`);
  no builds or tests were launched concurrently with the runs. The run window
  is 2026-09-23 22:16:15–22:48:30 local (first and last criterion directory
  mtimes). Nothing in this session can be shown to have perturbed the numbers,
  but no CPU-idle trace was captured either — treat the outlier counts in
  `BASELINES.md` as the per-leaf evidence that noise stayed bounded.
- **Noise:** CV (= `std_dev`/`mean`) median 6.05%, max 16.87%; 13 of 82 leaves
  exceed 10%. The two noisiest are `polygon_ops/union/256` and
  `polygon_ops/offset/256` (16.87% / 16.84%) — both are the largest fixtures
  and both fall in Linear mode with `iters` as low as 5–6, so the sample count
  per unit of work is low. **Any A/B on those two leaves needs repeats before
  it can resolve a small delta** — directly relevant to ticket 17, whose cost
  surface they are.
- **Console-vs-disk:** see finding #1. Criterion's console output is a
  formatted summary; the authoritative per-leaf numbers are on disk under
  `target/criterion/`. This file and `BASELINES.md` quote `estimates.json`.
- **Slope-vs-mean for small deltas:** with 56/82 leaves Linear, a "before vs
  after" comparison must use the same statistic on both sides. Note criterion's
  own console is *internally* inconsistent here in Linear mode: the `time:`
  line prints the **slope** (`absolute_estimates.typical()`), while the
  `change:` line is computed from **means**
  (`compare.rs::estimates`'s `stats` — `a.mean() / b.mean() - 1.`). So
  "console said 1.5688 ms, now says …" is a slope-to-slope narrative while
  "change: +N%" is mean-to-mean; the two can disagree. Read
  `estimates.json` (`mean`/`median`/`slope` all stored per leaf) and state which
  statistic you are quoting. `baselines.json` carries both columns plus their
  divergence.
- **Fixture-scope limits:** findings #2 and #3 are properties of the fixtures,
  not the code under test. Both would be repaired by choosing exercising
  fixtures (e.g. a non-manifold or open mesh for `repair`; a dense mesh or a
  raised `max_error` for `decimate`) — deliberately **not** done here: ticket
  15 authorises refresh, not bench-fixture redesign, and changing a fixture
  invalidates baseline comparability with any future work that assumed it.
- **`target/criterion` is gitignored and regenerable.** These baselines are the
  first on-disk set in this working tree (`PERF-HANDOFF` §7a records
  `target/criterion` as absent before this ticket). Re-derive them by re-running
  the seven invocations; the numbers in `BASELINES.md` are a dated capture, not
  a fixture.
