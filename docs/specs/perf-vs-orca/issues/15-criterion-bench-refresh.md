# Criterion bench refresh

Type: task
Status: resolved
Blocked by: 11

## Question

Bring the seven criterion benches current and trustworthy so they can back
optimization decisions (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §7a — explicitly authorized for
refresh).

Work:

- Run each bench and establish on-disk baselines (`target/criterion` is absent
  in this tree): `polygon_ops` (slicer-core), `mesh_ops` (slicer-helpers),
  `pipeline`, `per_stage`, `wasm_modules`, `shell_classification`,
  `gate_evidence` (slicer-runtime) — invocations in `.agents/aux-commands.md`;
  `wasm_modules` needs `cargo xtask build-guests` first.
- Re-validate that each fixture does **real** work before trusting any number:
  the `shell_classification` bench has a first-hand precedent of a green bench
  measuring nothing (its earlier fixture made `difference(layer, neighbour)`
  empty so `apply_opening` short-circuited). Check `git log -p` per bench before
  trusting fixture age — several "last touched" commits were incidental sweeps
  (crate rename, clippy/literals, WIT review).
- Note `gate_evidence`'s shape (per DEV-026 it times a real `pnp_cli slice`
  subprocess against `resources/regression_wedge.stl` and measures rather than
  hard-fails).

Deliverable: refreshed baselines plus a per-bench trust verdict (fixture does
real work / measures what it claims / stale). Timing ticket — chain position 2
after [Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md).

## Answer

Claimed and resolved 2026-09-23 (AFK agent session) — measurement/attribution
only; no code, fixture, or config changed.

**Verdict: all seven benches are current and trustworthy. Six of seven are
trustworthy without qualification; `mesh_ops` is trustworthy for `import_step`
but its `repair` and `decimate` leaves are fixture-limited (they measure real
work that the fixture cannot let succeed). No bench is stale, and none
repeats the `shell_classification` short-circuit class.**

**Baselines established.** `target/criterion` was absent in this tree; it now
holds **82 leaves across the 7 benches**, each with `base/` and `new/`
(first-run, so `base == new`). Tables:
[`evidence/t15-criterion-refresh/BASELINES.md`](../evidence/t15-criterion-refresh/BASELINES.md);
machine-readable twin `baselines.json` (means, medians, 95% CIs, CV, outlier
counts, sample sizes, iteration counts, sampling mode, slope, and the
console-vs-mean divergence per leaf). Run 2026-09-23 22:16–22:48 local on the
Ryzen 5 7600X dev box, one sequential run per bench binary, no concurrent
load. Headline: `polygon_ops` 8.49–242.52 µs; `mesh_ops` import 1.44/2.14 ms,
repair 2.95 µs, decimate 3.75 µs; `pipeline` 67.9 ns–1.85 ms;
`per_stage` 942.58 ns–97.25 µs; `wasm_modules` 6.73 ns–97.0 ms;
`shell_classification` 2.55–131.73 ms; `gate_evidence` **885.9 ms** mean /
890.1 ms median against `docs/12`'s ≤ 10 s bound.

**Per-bench trust verdicts** (vocabulary per this ticket; full basis and
measured magnitudes in FINDINGS.md):

| Bench | Verdict |
| --- | --- |
| `polygon_ops` | does real work / measures what it claims |
| `mesh_ops` | `import_step` clean; `repair/cube` + `decimate/cube_default` fixture-limited (below) |
| `pipeline` | measures what it claims, within declared v1 scope (report-stack overhead; not end-to-end) |
| `per_stage` | measures what it claims; name oversells scope (v2 TODO open, unchanged) |
| `wasm_modules` | real artifacts, real parsing/JIT work; "v1 stub" is scope, not degeneracy |
| `shell_classification` | does real work — no short-circuit; ms-scale and grows with layers |
| `gate_evidence` | real subprocess slice; 13/13 runs clean (`status:ok`, `degraded:false`, 0 fatal, 0 non-fatal; `layer_count:50` in `slice_stats`) |

**Two fixture limits found — both recorded, both deliberately not fixed here**
(ticket 15 authorises refresh; changing a fixture would invalidate baseline
comparability, and neither is a defect in the code under test):

1. **`repair/cube` measures a no-op scan.** The 10 mm cube triangulates to 12
   triangles (verified via `pnp_cli mesh import` against the same
   `crates/slicer-helpers/tests/resources/` file the bench loads), and it is
   perfectly clean: `pnp_cli mesh repair --stats` reports
   `degenerate_removed: 0`, `faces_reoriented: 0`, `open_edges_closed: 0`. The
   2.95 µs is a scan returning the mesh unchanged. The same harness on
   `resources/regression_wedge.stl` repairs real damage (`faces_reoriented:
   12`, `open_edges_closed: 56`), so the gap is the fixture, not the op.
2. **`decimate/cube_default` cannot reach its declared target.** 12 → 12 against
   a target of 6, `achieved_error: 0.0`, because the bench relies on the
   `DecimateConfigBuilder` default `max_error = 0.01` and this box needs > 0.4
   before any edge collapse is cheap enough (swept 0.01 → 0.6 this session:
   0.01–0.4 stays 12; 0.5 and 0.6 give 6). Not a stub — the same call on an
   876-triangle mesh at the same default budget reaches 438/876. The 3.75 µs is
   meshopt work that the budget rejects.

Reproduction for both: [`evidence/t15-criterion-refresh/fixture-probes/PROBES.md`](../evidence/t15-criterion-refresh/fixture-probes/PROBES.md).

**Why the two fixture limits do not taint a matched cell.** `repair` /
`decimate` / `import_step` (`crates/slicer-helpers/src/`) have no production
caller on the slice path — `pnp_cli slice` loads meshes through
`slicer-model-io`'s `load_stl` (`crates/slicer-model-io/src/loader.rs`, straight
`stl_io::read_stl`), and the only production callers of the three helpers are
the `pnp_cli mesh repair|decimate|import` subcommands
(`crates/pnp-cli/src/helpers_cmd.rs`). So `mesh_ops` is a helper-microbench, not
a pipeline-cost instrument, and these fixture limits cannot affect the
matched-pair scoreboard or the serial-floor tickets.

**Measurement-semantics finding, relevant to this chain's cost A/Bs.** The
criterion **console's `time:` line is not the mean** — in Linear sampling mode
criterion sets `Estimates::typical()` to the regression *slope*
(`criterion::estimate::Estimates::typical`) and the report prints that.
Verified exactly: `mesh_ops/import_step/cube`'s console line
`[1.5331 ms 1.5688 ms 1.6009 ms]` matches the JSON slope CI to every printed
digit, while the JSON mean is `1.4445 ms` (+8.61% divergence). 56 of 82 leaves
are Linear; divergence reaches −7.00% on `polygon_ops/union/256`. Worse,
criterion's `change:` line is **mean**-based (`compare.rs::estimates`), so on a
Linear leaf `time:` and `change:` disagree about which statistic they mean.
**A before/after comparison must use the same statistic on both sides** —
all-slope or all-mean, never mixed console output.

**`gate_evidence`'s 886 ms is ~89% pre-validation module compile.** Timestamps
inside the logged run show 775 ms of the 873 ms median slice happening before
the first `phase_start(validation)` event; the bracketed phases sum to only
~92 ms (`validation` 2 / `prepass` 32 / `per_layer` 45 / `postpass` 13). The
gap reconciles to **99.5%** with this ticket's own `wasm_modules` numbers —
769.3 ms of `compile_component` over the same 24 artifacts plus 3.7 ms of
`manifest_load`. Call site: `load_live_modules_for_plan_with_integrated`
(`crates/slicer-runtime/src/run.rs`), at the "Discover and plan every module
under `--module-dir`" step. It is a per-process fixed cost already inside every
PNP scoreboard row (that rig measures process creation-to-exit), invisible to
phase-wall accounting, and it is *not* a new lead — [Serial host
floor](27-serial-host-prepass-floor.md) already owns this term as work item 1
("module discovery/instantiation (24 guests)"); this supplies its measured
starting value. Orca's comparable startup cost is unknown (map: "Orca-side
attribution"), so nothing is subtracted from the Orca side.

**Caveats for consumers of these numbers.** CV median 6.05%, max 16.87%; 13/82
leaves above 10%, the worst being `polygon_ops/union/256` (16.87%) and
`polygon_ops/offset/256` (16.84%) — both are Linear with `iters` down to 5–6,
so **any small-delta A/B on those two leaves needs repeats** before it can
resolve a difference. `target/criterion` is gitignored and regenerable; these
numbers are a dated capture, re-derivable by re-running the seven invocations
in `.agents/aux-commands.md`.

**Downstream:** [clipper2 cost/output verdict](17-clipper2-cost-output-verdict.md)
is now unblocked (it also required [clipper2 1.1.0 upstream
evidence](16-clipper2-1-1-0-upstream-evidence.md), already resolved). Its cost
A/B should use `baselines.json`'s `mean_ns` / `slope_ns` columns for
like-for-like comparison and repeat the two high-CV 256-square leaves.
The two fixture limits (#1 `repair`, #2 `decimate`) are candidate follow-up
work only if a future ticket needs `mesh_ops` repair/decimate numbers to carry
a decision; no ticket was opened here because no current decision does. A
follow-up was opened 2026-09-24 as
[mesh_ops fixture limits](29-mesh-ops-fixture-limits.md) — bench
trustworthiness, off the timing chain.
