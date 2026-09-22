# Slicing Performance Baseline and Handoff

> **Migration note (2026-09-22).** This is the canonical copy, moved from
> `tmp/PERF-HANDOFF.md` into `docs/specs/perf-vs-orca/evidence/` so the
> performance map never depends on gitignored files. Paths to sibling evidence
> below are relative to this directory. The two model fixtures (`tmp/3dbenchy.stl`,
> `tmp/base.stl`) intentionally remain under `tmp/` — user-supplied and
> licence-encumbered, never to be committed. Raw captures referenced
> historically below (G-code corpora, JSONL streams, binaries under these
> directories or `target/perf-*`) were scratch and are **not retained**; the
> distilled records — sibling `FINDINGS.md` / `MEASUREMENTS.md` files and
> `results/*.csv` — carry the evidence.

**When to read this:** before doing any work on slice throughput. It records
what has actually been measured, how to reproduce it, and — most importantly —
the measurement traps that have already produced a wrong conclusion once.

Captured 2026-09-04, at the commit that landed the `mesh_analysis` footprint fix
and bed placement. Every number below was measured in that session on one
machine; none are estimates. They are ledger facts and they rot — re-derive
before trusting them.

## 1. Fixtures

| Fixture | Triangles | Notes |
| --- | --- | --- |
| `resources/calicat.stl` | 876 | In-repo. Authored at origin-positive coords. Smoke fixture only. |
| `tmp/3dbenchy.stl` | 225,786 | **Not in-repo** (user-supplied; 3DBenchy is licence-encumbered — do not commit). |
| `tmp/base.stl` | 2,461,234 | **Not in-repo** (user-supplied). Authored at X ≈ -728, z_min = -75.7. |

Triangle counts read from the binary STL header and cross-checked against file
size, `(bytes - 84) / 50`.

Both `tmp/` fixtures must be supplied before this work can resume. Nothing in
the repo depends on them.

---

## 2. Baseline: PNP vs OrcaSlicer

> **Read §3.1 before quoting any ratio from this table.** These configurations
> are **not matched**. No speed multiple derived from this table is defensible
> yet.

| Fixture | OrcaSlicer | PNP | PNP G-code | Orca G-code |
| --- | --- | --- | --- | --- |
| `calicat.stl` | 0.84 s | 4.81 s | 0.59 MB | 1.01 MB |
| `3dbenchy.stl` | 6.34 s | 42.5 s | 7.6 MB | 4.1 MB |
| `base.stl` | 44.17 s | 11 m 19.6 s | 30.2 MB | 15.3 MB |

PNP times are **uninstrumented** wall clock (see §3.2).

---

## 3. Measurement traps

Each of these has already caused, or nearly caused, a wrong conclusion.

### 3.1 The configurations are not matched — fixing that is job one

OrcaSlicer ran BBL X1C **0.4 mm** nozzle, `0.20mm Standard`, supports at profile
default. PNP ran the repo fixture config with `support_type: tree(auto)` and a
**0.5 mm** nozzle.

The G-code sizes diverge in *both* directions — PNP larger on benchy and base,
smaller on calicat — which is direct evidence the two are doing different
amounts of work, not that one is uniformly slower.

**Do not report a speed multiple until a matched pair exists.** Build one first:
same nozzle, same layer height, same wall and infill counts, supports off on
both sides, then supports on. That is the first deliverable.

### 3.2 `--instrument-stderr` is not free

Measured on `3dbenchy.stl`, same binary and fixture: **42.5 s uninstrumented vs
73.5 s instrumented.** The stream emits per-module events for every layer
(240 layers × ~15 modules).

Use uninstrumented runs for wall-clock claims, instrumented runs for
*attribution only*, and never mix the two in one table.

### 3.3 Per-layer `elapsed_ms` is summed across rayon workers

Tier 2 runs parallel, and each worker's `module_complete` carries its own
elapsed. Summing them gives **accumulated worker wall-clock, not CPU time or
whole-slice wall time**. On benchy the
per-layer module sums exceed the `per_layer` phase wall time by roughly 8×.

Prepass and postpass are serial, so their module elapsed values *are* wall time
and do compare directly against phase totals.

### 3.4 `--module-dir modules/core-modules` is mandatory

Without it `pnp_cli` loads only the 18 **integrated** (natively compiled)
modules and silently skips every WASM core module — including both support
planners. Such a slice does not exercise the same pipeline and generates no
supports at all.

Confirm with `pnp_cli module diagnose --module-dir modules/core-modules`:
expect every core module to report provenance `external` (24 as of 2026-09-22;
the count drifts as modules are added — the invariant is "none falls back to
integrated", not the number).

### 3.5 Guest WASM staleness

`cargo xtask build-guests --check` must return exit `0` before any timing run.
Exit `3` means the checker could not form an opinion and must **not** be read as
clean. See the guest-staleness rule in the root `CLAUDE.md`.

### 3.6 Run-to-run variance: small in G-code bytes, LARGE in wall clock

Two distinct claims; the original version of this section only made the first
and was read as covering both.

**G-code bytes.** Four identical benchy runs produced G-code differing by ~500
bytes out of 6.7 MB — a handful of extrusion segments. Re-confirmed 2026-09-04:
two runs of one binary differed by 577 bytes. Byte-diffing G-code therefore
cannot validate a change at fine granularity; validate at unit level instead.
It *can* still be used as a coarse "did the geometry change wholesale" check,
and section counts (e.g. `grep -c ";TYPE:Internal Bridge"`) are stable.

**Wall clock, `base.stl`.** Measured 2026-09-04: two runs of the *same* binary
took **529 s and 416 s** — 13% apart, i.e. larger than most single
optimisations you will land. A one-run before/after comparison on `base.stl`
is not evidence. This trap produced a false regression signal in the session
that recorded it: 486 s (build A, one run) vs 529 s (build B, one run) looked
like a 9% regression, and build B's second run at 416 s showed it was noise.

**Take repeats on `base.stl`, or compare at stage level rather than whole-slice
level.** Stage-level numbers are quieter but not immune.

## 4. Reproduction commands

All verified working in the capture session.

```bash
# Preconditions
cargo xtask build-guests --check          # MUST be exit 0
cargo build --release --bin pnp_cli

# PNP wall clock (no instrumentation)
time ./target/release/pnp_cli.exe slice \
  --model tmp/3dbenchy.stl \
  --config <config>.json \
  --module-dir modules/core-modules \
  --output out.gcode

# PNP attribution profile
./target/release/pnp_cli.exe slice ... --instrument-stderr 2> profile.jsonl

# Fuel-based module profiling (ADR-0055); costs throughput
./target/release/pnp_cli.exe slice ... --profile
./target/release/pnp_cli.exe profile --from profile.jsonl
```

OrcaSlicer CLI. It is a GUI-subsystem binary, so output must be redirected to a
file, and it needs a self-consistent **vendor** profile triple — the user's own
presets inherit from system profiles and are rejected with `unknown config
type`:

```bash
P="/c/Program Files/OrcaSlicer/resources/profiles/BBL"
"/c/Program Files/OrcaSlicer/orca-slicer.exe" --slice 0 \
  --load-settings "$P/machine/Bambu Lab X1 Carbon 0.4 nozzle.json;$P/process/0.20mm Standard @BBL X1C.json" \
  --load-filaments "$P/filament/Generic PLA.json" \
  --outputdir <ABS_WINDOWS_PATH> <ABS_WINDOWS_PATH_TO_STL> > orca.log 2>&1
```

---

## 5. Where the time goes

### Current refresh — 2026-09-05

Supersedes the historical tables below for hotspot selection. Full raw evidence,
all process CPU/wall ratios, output checks, and methodology are under
`perf-refresh/`; start with `FINDINGS.md` and `MEASUREMENTS.md`.

| Instrumented phase wall | base run 1 | base repeat | benchy |
| --- | ---: | ---: | ---: |
| prepass | 283.773 s | 258.189 s | 11.856 s |
| per_layer | 171.924 s | 176.010 s | 12.810 s |
| postpass | 5.869 s | 5.903 s | 0.715 s |

| Serial module elapsed | base run 1 | base repeat | benchy |
| --- | ---: | ---: | ---: |
| tree-support-planner | 96.630 s | 92.850 s | 3.299 s |
| host:slice | 56.190 s | 46.409 s | 2.403 s |
| host:support_analysis | 39.147 s | 37.791 s | 1.901 s |
| host:shell_classification | 34.660 s | 32.203 s | 2.622 s |
| host:mesh_analysis | 24.203 s | 22.354 s | 0.213 s |
| host:overhang_annotation | 17.580 s | 11.857 s | 0.770 s |

Classic perimeters remains dominant in accumulated worker elapsed: base run 1
1725.920 s; benchy 100.133 s. These are **not CPU seconds**.

Uninstrumented baseline medians, separate batches of three: base 502.827 s
then 462.152 s; benchy 30.163 s then 28.926 s. External load persisted and varied,
so these are **load-qualified observations, not a quiet-machine baseline or a
speedup claim**. Repeats confirmed the base serial-hotspot ranking. Instrumented
runs had different load; do not infer instrumentation overhead from their ratio.

### Historical measurements — retained for context

### `3dbenchy.stl` — instrumented total 73.5 s (42.5 s uninstrumented)

Phase wall time: **prepass 42.6 s (58%)**, per_layer 28.3 s (38%), postpass
1.7 s, validation 3 ms.

Prepass is serial, so these are wall time and the highest-leverage targets:

| Prepass module | Wall |
| --- | --- |
| `host:shell_classification` | **21,948 ms** |
| `com.core.tree-support-planner` | 9,384 ms |
| `host:support_analysis` | 4,912 ms |

Per-layer, as **aggregate CPU across workers** (not wall — see §3.3):

| Per-layer module | CPU over 240 calls |
| --- | --- |
| `com.core.classic-perimeters` | 212,127 ms |
| `com.core.infill-linker` | 23,816 ms |
| `com.core.tree-support` | 9,997 ms |

### `base.stl` — instrumented total 731.9 s (12.2 min); 11 m 19.6 s uninstrumented

**Phase wall time: prepass 522.9 s (71%)**, per_layer 198.5 s (27%), postpass
8.3 s, validation 7 ms. 495 layers.

Prepass is serial, so these are wall time. This is where the slice lives:

| Prepass module | Wall | Share of whole slice |
| --- | --- | --- |
| `host:shell_classification` | **223,022 ms** | **30.5%** |
| `com.core.tree-support-planner` | 106,137 ms | 14.5% |
| `host:slice` | 72,071 ms | 9.8% |
| `host:support_analysis` | 45,286 ms | 6.2% |
| `host:mesh_analysis` | 29,490 ms | 4.0% |
| `host:overhang_annotation` | 15,583 ms | 2.1% |

Those six account for **~67% of total slice wall time**, all of it serial.

Per-layer, as **aggregate CPU across workers** (not wall — see §3.3):

| Per-layer module | CPU over 495 calls |
| --- | --- |
| `com.core.classic-perimeters` | 1,978,962 ms |
| `com.core.tree-support` | 67,817 ms |
| `com.core.rectilinear-infill` | 43,259 ms |
| `com.core.lightning-infill` | 42,687 ms |
| `com.core.top-surface-ironing` | 42,635 ms |

`classic-perimeters` is ~29x the next module by CPU. Note the tight clustering
of five unrelated per-layer modules at ~42-43 s each, which looks more like a
shared fixed per-layer cost (dispatch, marshalling, IR access) than five
coincidentally equal algorithms — worth confirming before optimising any of
them individually.

### `calicat.stl` — instrumented total 1.17 s

Phase wall: prepass 143 ms, per_layer 194 ms, postpass 46 ms. Dominated by
`classic-perimeters` (371 ms CPU over 175 calls). Too small to profile
meaningfully; use as a smoke fixture.

---

## 6. Ranked leads

**Current ranking after the 2026-09-05 refresh:** (for classic-perimeter
internals this is superseded by the 2026-09-22 §14 re-attribution.)

1. Split tree-support planner wall into cache/batched-host work, guest work,
   and surrounding host validation/commit. It is the largest measured serial
   module on both fixtures. The historical collision-cache percentage has not
   been re-established. Host-batch boundaries still lack their own timing.
2. Attribute classic-perimeter work versus shared dispatch/IR overhead. It
   dominates accumulated worker elapsed; six other modules again cluster in
   base run 1 at 34.407–34.672 s each. Shared overhead is a hypothesis.
3. Attribute `host:slice`, now the second-largest base serial module.
4. Follow support analysis, shell classification, and mesh analysis from their
   measured stage sizes; neither allocator changes nor the small `apply_opening`
   anomaly is the next evidence-led target.

Base still emits 29,108 non-fatal errors and degraded output. Any support
experiment must preserve visibility of this correctness problem. See
`perf-refresh/FINDINGS.md` for the proposed next experiment and decision.

**Historical leads follow; their ordering and figures are superseded above.**

1. **`host:shell_classification` — LARGELY ADDRESSED 2026-09-04. See §9.**
   Was 223.0 s on `base.stl` (30.5% of the slice) and 21.9 s on benchy. The
   cost was **not** where this document originally guessed, and not where the
   module's own doc comment said: Pass 1 is ~4% of the stage. ~91% of it was
   `gate_internal_bridge_sites`, and ~81% of *that* was two full-layer
   `offset` calls being computed before the early-outs that discard them.
   Benchy stage cost is now ~3.0 s (was ~17 s). What remains inside it, in
   descending order, is `unsupported_span_areas` (now parallel),
   the `deep_infill_clip_area` offset, and the `retain` erosion — see §9 for
   the current split and the remaining ideas. **Allocator experiments
   (mimalloc, snmalloc; §10) produced no wall-time win and are CLOSED —
   the remaining cost is algorithmic, in Clipper geometry work.**

2. **The surviving overhang union in `mesh_analysis`.**
   `compute_xy_footprint(mesh, transform, &overhang_facets)` still performs an
   exact Clipper union to build `OverhangRegion::xy_footprint`, which is stored
   in the IR. Measured **18.1 s for 326,546 facets on `base.stl`** (97 ms for
   22,137 on benchy). Its sibling *region* union was removed in the capture
   session; this one survives because downstream consumes its output. Question
   to answer: does any consumer need the exact union, or only its bounding
   box / coverage? For scale: `host:mesh_analysis` totals 29.5 s on `base.stl`,
   so this union is roughly 60% of that stage.

3. **`mesh_analysis` is fully serial per object.** `execute_mesh_analysis_with`
   is a plain `for object in &mesh.objects` loop, and there is no `par_iter`
   anywhere in the file. Single-object models get no parallelism. The
   per-triangle classify loop is trivially parallel (only 8.2 ms on benchy, so
   low absolute value, but it scales with triangle count).

4. **`classic-perimeters` — 1,979 s aggregate CPU on `base.stl`** (212 s on
   benchy), ~29x the next per-layer module. Already parallel across layers, so
   any win must be algorithmic or inside its own loops. Note that per-layer is
   only 27% of `base.stl` wall time against prepass's 71%, so this is a smaller
   prize than its raw CPU number suggests — Amdahl favours the serial prepass
   work in leads 1-3.

5. **`tree-support-planner` — 9.4 s in benchy prepass.** ADR-0049 measured its
   collision-cache build at **98.0%** of its runtime — 603 independent
   `offset_polygons` calls. It is the only in-tree caller of the batched host
   services (`slicer_sdk::host_batch::batch_offset`). Whether that batch is
   actually the cost **at HEAD is unmeasured**.

6. **The batched host services have no instrumentation.**
   `offset_polygons_batch`, `clip_polygons_batch`, and `simplify_polygon_batch`
   record nothing; only the two mesh-query batches push `batch_calls` audit
   entries. Adding a timing hook is cheap and is a prerequisite for evaluating
   lead 5.

### Dead end, already investigated — do not repeat

`raycast_z_down_batch` and `surface_normal_at_batch` are brute-force O(triangles)
linear scans and look like obvious optimisation targets. **They have no
production callers** — only SDK plumbing and test mocks. Spatial-indexing them
optimises an unreached path.

**Scalable allocators (mimalloc / snmalloc). Investigated 2026-09-05 — CLOSED,
see §10.** The §9.7 CPU-inflation signature was later attributed to external
machine load, not heap contention. Clean-machine A/B (5+5 samples, cpu/wall
≈ 1.0) shows System ≈ mimalloc ≈ snmalloc on wall time, with replacements
using *more* peak memory (+5–13%). The §9.7 "scalable allocator is the
obvious next experiment" recommendation is retracted. Do not repeat this A/B
without new evidence (e.g. a real contention signature reproduced on a quiet
machine with a profiler).

---

## 7. Not measured / open

- Any matched-configuration comparison against OrcaSlicer (§3.1).
- Whether `clipper2-rust` 1.1.0 changed polygon output or cost versus 1.0.3;
  the bump landed without a parity comparison (DEV-166).
- ~~Where `host:shell_classification`'s 21.9 s actually goes internally~~ —
  answered by §9 (guard ordering; remaining split in §9.7).
- Whether the batched `offset_polygons` path is hot at HEAD.
- Peak RSS. `AccountingAllocator` excludes WASM linear memory by construction,
  so memory bounds remain unmeasurable with current instrumentation (DEV-026).
  §10 supplements: OS peak working set was measured externally for the
  allocator A/B (benchy ~0.35–0.45 GB, base ~2.4–2.7 GB at this config).

---

## 7a. Criterion benchmarks — in scope, and stale

### 2026-09-07 bounded perimeter fastpath experiment

The annotation-free `build_wall_flags` experiment is documented in
`perf-flags/EXPERIMENT.md` and `perf-flags/FINDINGS.md`, with isolated
baseline/candidate artifacts and raw CSV/G-code/JSONL evidence under
`perf-flags/`. It passed focused classic/Arachne and slicer-core tests plus
the all-target check/clippy/literals gates. Recommendation from that bounded
experiment: **PROVISIONAL candidate; acceptance inconclusive; no commit was made.** Follow-up
acceptance evidence now includes interleaved supports-off Benchy, original
tree-support Benchy, and repeated original tree-support base runs for both
generators, with per-sample CPU/wall tables and DEV167's 29,108 non-fatal base
errors preserved in `perf-flags/FINDINGS.md`. CPU evidence is favorable,
but the controlled wall gate was not demonstrated; no further timing runs were
made in this follow-up.

**The next session is explicitly authorised to bring these back up to date.**

Seven criterion benches exist. All seven **compile clean** at HEAD
(`cargo check --workspace --benches`, and `clippy --workspace --all-targets
-D warnings` passes), so this is neglect, not rot. But **no criterion baselines
exist on disk** — `target/criterion` is absent, so nothing has been run in this
working tree.

Last commit touching each, against HEAD at 2026-09-04:

| Bench | Last touched | Note |
| --- | --- | --- |
| `slicer-core/benches/polygon_ops.rs` | 2026-05-18 | oldest; TASK-201 config work |
| `slicer-helpers/benches/mesh_ops.rs` | 2026-05-17 | oldest; DecimateConfigBuilder |
| `slicer-runtime/benches/pipeline.rs` | 2026-06-08 | instrumentation overhead |
| `slicer-runtime/benches/wasm_modules.rs` | 2026-06-13 | **v1 stub**; needs `cargo xtask build-guests` |
| `slicer-runtime/benches/per_stage.rs` | 2026-07-30 | carries an explicit unimplemented `TODO (v2)` scope |
| `slicer-runtime/benches/gate_evidence.rs` | 2026-08-05 | crate-rename commit only |
| `slicer-runtime/benches/shell_classification.rs` | 2026-08-09 | packet-197 sweep commit only |

**Caveat on those dates:** several are *incidental* commits — a crate rename, a
clippy/literals sweep, a WIT review fix — not changes to bench content. The
substantive age is therefore older than the table suggests. Check
`git log -p` on a bench before assuming its fixture reflects current geometry.

Invocations are catalogued in `.agents/aux-commands.md`:

```bash
cargo bench -p slicer-core    --bench polygon_ops
cargo bench -p slicer-helpers --bench mesh_ops
cargo bench -p slicer-runtime --bench pipeline             # instrumentation overhead
cargo bench -p slicer-runtime --bench per_stage            # plan-freeze serial-edge helpers
cargo bench -p slicer-runtime --bench wasm_modules         # v1 stub; needs cargo xtask build-guests
cargo bench -p slicer-runtime --bench shell_classification
cargo bench -p slicer-runtime --bench gate_evidence
```

### Why this is worth doing first, not last

Three of these map directly onto open questions in this document, which makes
refreshing them cheaper than building new instrumentation:

- **`shell_classification`** targets lead 1 — the 21.9 s serial prepass item.
  It already exists and is aimed at exactly the right function
  (`commit_shell_classification_builtin`).

  **Read its module doc before trusting it.** It records that a *previous*
  version of this same fixture built polygons where `difference(layer,
  neighbour)` was empty on all but the outermost layer, so `apply_opening`
  short-circuited and the dominant `offset` calls never executed. The bench
  reported microseconds and its conclusion "did not transfer to real geometry."
  The fixture was rewritten to vary cross-sections per layer. That is a
  first-hand precedent for a bench that ran green and measured nothing —
  re-validate that the current fixture still does real work before acting on
  any number it produces.

- **`polygon_ops`** benches `union` / `difference` / `intersection` / `offset`
  — the exact surface affected by the unverified `clipper2-rust` 1.0.3 -> 1.1.0
  bump. Running it against both versions is the cheapest available answer to
  the open DEV-166 question of whether 1.1.0 changed clipper cost or output.

- **`pipeline`** measures instrumentation overhead, which this session only
  established ad hoc (42.5 s vs 73.5 s on benchy, §3.2). The bench can make
  that rigorous rather than anecdotal.

Note `gate_evidence` is a different animal: per DEV-026 it times a real
`pnp_cli slice` subprocess against `resources/regression_wedge.stl` and
**measures rather than hard-fails**, matching this repo's convention that
benches are "slow; not in CI."

---

## 8. Related records

- **DEV-166** — shipping on `clipper2-rust` with an unbounded recursion in
  `check_split_owner`: a hard process abort with no diagnostic, and no
  regression coverage. Relevant because any new Clipper call site can reach it.
- **DEV-167** — `tree-support-planner` drops ~60 contiguous support layers
  mid-print on `base.stl` (29,108 code-1200 routing rejections). A correctness
  bug rather than a perf bug, but it sits in a module on the perf critical path.
- **ADR-0049** — batched host services over threaded guests; the original
  prepass parallelism measurements.
- `docs/17_agent_debugging.md` and the `debug-pipeline` skill — DAG, timing, and
  manifest diagnosis tooling.

---

## 9. Landed 2026-09-04 — `shell_classification` / internal-bridge gating

Three changes, all in the host built-in `PrePass::ShellClassification` path.
None changes intended geometry; all are ordering, windowing, or parallelism.

### 9.1 What was actually hot (and what wasn't)

Measured with temporary phase counters (since removed) around each block of
`commit_shell_classification_builtin`. Benchy, uninstrumented slice:

| Block | Before |
| --- | --- |
| `gate_internal_bridge_sites` | **14,876 ms (90.7%)** |
| Pass 1 (depth-0 classification) | 644 ms (3.9%) |
| bridge gate loop | 558 ms |
| Pass 2 bottom / top | 280 / 28 ms |

Two prior beliefs were wrong and are corrected in the source:

- The module doc claimed Pass 1 "is the dominant cost of the stage". It is ~4%.
  Corrected in `slice_postprocess_prepass.rs`.
- §6 of this document nominated `apply_opening`'s round-join offsets. They sit
  inside Pass 1, so they cannot be more than that ~4%. Still worth noting for
  a later pass: `apply_opening` passes arc tolerance **0** to a `Round` join,
  while every other round-join morphological pass in
  `slicer_core::polygon_ops` uses `MORPH_ROUND_ARC_TOLERANCE_MM` (0.05 mm).
  Canonical `opening_ex` (`ClipperUtils.hpp`) defaults to `jtMiter`. Untested;
  small prize.

Drilling into `gate_internal_bridge_sites` gave the actual answer:

| Sub-block | Before |
| --- | --- |
| the two `offset`s on `deep_infill_area` | **13,160 ms (81%)** |
| `unsupported_span_areas` | 2,065 ms |
| `gather_areas_w_depth` | 888 ms |
| everything else | < 60 ms |

### 9.2 The fix that mattered — guard ordering

`deep_infill_clip_area` and `internal_unsupported_area` were computed *before*
the `internal_solid_fill_empty` and `unsupported_empty` early-outs. DEV-148's
own skip histogram records that the large majority of layer-visits take one of
those early-outs, so most of that 13.2 s was full-layer clipper work whose
result was immediately dropped.

Both guards now run first, in their original relative order (so the skip
histogram is unchanged), and `internal_unsupported_area` is computed lazily
only when there are surviving candidates to filter.

### 9.3 Supporting changes

- **Depth windowing.** `gather_areas_w_depth` walks down from the target layer
  and breaks after one flow height, so it reads at most ~2 entries; callers
  were materialising a `BridgeDepthLayer` (five polygon-set clones) for *every*
  layer below. New `depth_window_start` / `filled_window_start` in
  `slicer_core::algos::bridge_over_infill` expose the window rule so callers
  build only what the callee reads. Fixed in both callers —
  `gate_internal_bridge_sites` and the per-layer arm in `layer_executor.rs`.
  **Measured on its own this was not the win** (benchy unchanged); it removes a
  quadratic that had not yet dominated. Kept because it is strictly less work
  and it moves the window rule next to the code that defines it.
- **Parallelism.** The bridge-gate loop over slices is now `par_iter_mut`
  (each slice mutates only its own regions): 533 -> 84 ms on benchy.
  `gate_internal_bridge_sites`' per-layer loop is split into a parallel
  **qualification** phase (pure reads of committed geometry) and a serial
  **commit** phase (serial by necessity — `lower_candidates` reads the
  `internal_bridge_areas` earlier layers of the same pass wrote).

### 9.4 Results

Benchy, uninstrumented, one run each — stage cost:

| | before | guards | + parallel |
| --- | --- | --- | --- |
| stage total | 17.8 s | 4.26 s | **2.96 s** |
| `gate_internal_bridge_sites` | 16.2 s | 2.81 s | 1.90 s |
| whole slice | 44.9 s | 27.7 s | 28.5 s |

`base.stl`, uninstrumented — **read §3.6 first, wall clock varies ±13% here:**

| | baseline (this doc, 1 run) | guards (1 run) | + parallel (2 runs) |
| --- | --- | --- | --- |
| whole slice | 679.6 s | 486 s | 529 s / **416 s** |
| stage total | 223.0 s (instrumented) | 63.6 s | 42.2 s / 31.3 s |

The benchy improvement is solid and repeatable. On `base.stl` the direction is
clear but the exact figure is not: two runs of the final binary bracket the
single-run measurement of the intermediate one. Do not quote a `base.stl`
speedup from this table without taking repeats.

### 9.5 A pre-existing e2e failure found (and fixed) along the way

`staircase_ironing_g1_lines_within_top_fill_extents`
(`crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`)
failed at HEAD, **before** any of this session's changes — confirmed by
stashing the working tree, rebuilding the debug `pnp_cli`, and re-running under
`cargo xtask test`: identical failure, identical violating coordinates.

Cause: the assertion was `x.abs() > max_extent + tolerance`, i.e. it assumed the
staircase sits at the origin. Commit `9a558733` ("place bare meshes on the print
bed at load") means a bare mesh is now centred on the plate, so the ironing
strokes land at 115–135 mm — dead centre of the default 250×250 `bed_shape`.
The geometry was correct; the test's frame of reference was stale.

Fixed by anchoring the check to the bed centre, derived through
`slicer_model_io::bed_center_mm(&ResolvedConfig::default().bed_shape)` rather
than re-deriving the placement rule in the test. **The 10 mm bound and the
0.5 mm tolerance are unchanged** — only what they are measured from.

Worth a look: other e2e assertions written against origin-centred coordinates
may be latently wrong in the same way, passing only because they happen not to
check absolute position.

### 9.6 Verification

| Bucket | Result |
| --- | --- |
| `slicer-runtime::unit` | 89 / 0 |
| `slicer-runtime::contract` | 296 / 0 |
| `slicer-runtime::executor` | 225 / 0 |
| `slicer-runtime::integration` | 338 / 0 |
| `slicer-runtime::e2e` | 142 / 0 (after the §9.5 fix; 1 pre-existing failure before it) |
| `slicer-core --features host-algos` | all green |

- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask build-guests --check` — exit 0 before every timing run.
- **Not run:** `cargo test --workspace`. Per the repo's test discipline it is
  reserved for an explicit request or a packet close-out ceremony.

**Harness trap worth knowing.** The `pnp-cli-locator` staleness guard panics
(`crates/pnp-cli-locator/src/lib.rs`) whenever the debug `pnp_cli` is older than
the newest source file, and it takes down ~56 e2e tests at once with a wall of
unrelated-looking panics. A narrow `cargo test -p slicer-runtime` does **not**
rebuild it, and neither does `git stash` / `git stash pop` (which refresh source
mtimes). Run `cargo build --bin pnp_cli` after any of those before believing an
e2e result.
- G-code: benchy differs 46 bytes / 7.64 MB from baseline, but two runs of the
  *same* binary differ 577 bytes (§3.6). `;TYPE:Internal Bridge` section count
  is 13 in every run, before and after.

### 9.7 What is left in this stage

Current benchy split of the 2.96 s stage: `gate_internal_bridge_sites` 1.90 s,
Pass 1 0.64 s, Pass 2 0.33 s, bridge gate 0.08 s. Inside the gate the top items
are `unsupported_span_areas`, the `deep_infill_clip_area` offset, and the
`retain` erosion.

Open observation worth chasing: on `base.stl` the parallel qualification phase
reports **51.4 s of aggregate worker CPU** against a 33.2 s wall for the whole
enclosing block — but the same work measured **23.6 s serially**. Doing it on
12 threads more than doubled the CPU. That is consistent with allocator
contention (Windows system heap, heavy small allocations from Clipper), not
with the work itself changing. `AccountingAllocator` is ruled out: its hot path
is one relaxed atomic load when disabled, and it is disabled without
`--report`. ~~A scalable allocator (mimalloc/jemalloc) is the obvious next
experiment~~ — **superseded 2026-09-05: see §10.** The A/B was run and showed
no wall-time difference on a quiet machine; the "CPU more than doubled"
reading is now attributed to summing worker wall-clock (which includes waits)
under external machine load, not to heap contention (§10.5).

---

## 10. Landed 2026-09-05 — allocator A/B closed the contention hypothesis; tree state and next experiment

This section records the allocator × thread-count experiment authorized from
§9.7's open question, its outcome (negative), the traps it uncovered, and what
the next session should do first. Full plan/execution detail lives in
`alloc-bench/EXPERIMENT.md` (kept; ~350 KB with CSVs and scripts). Ledger
facts below were true when written; re-derive.

### 10.1 What was tested

Release `pnp_cli` variants differing only in the inner allocator inside
`AccountingAllocator` (wrapper kept outermost, `--report` contract intact;
opt-in cargo features `alloc-mimalloc` / `alloc-snmalloc`, cfg-exclusive,
`compile_error` on both; **wiring has been reverted from the tree** — the
features exist only in this session's history and can be re-cut in minutes):

| Variant | Crate (verified current) | Result |
| --- | --- | --- |
| System (default) | `std::alloc::System` | baseline |
| mimalloc | `mimalloc` 0.1.52 (MIT) | builds, runs, `--report` OK |
| snmalloc | `snmalloc-rs` 0.7.5 (MIT, CMake backend) | builds with **no host CRT changes**, runs |

Workload: benchy + base at 1 and 12 Rayon workers, snapshotted
`gpu-probe-tree.json` config, `--module-dir modules/core-modules`,
uninstrumented external measurement (wall = Stopwatch, CPU = process
`TotalProcessorTime`, peak = max-sampled `PeakWorkingSet64` @100 ms; a
per-run CSV row includes gcode sha256). 1 warmup + 5 measured per combination,
allocator order interleaved. jemalloc excluded (no Windows MSVC support);
rpmalloc-rs stale since 2021.

### 10.2 Results (medians of 5; §10.3 flags which samples are load-distorted)

| block | system | mimalloc | snmalloc |
| --- | --- | --- | --- |
| benchy@12t | 27.3 s | 26.7 s | not reached |
| base@12t | 451.6 s | 438.8 s | not reached |
| benchy@1t | 201.9 s (contended) → **131.1 s (clean, phase 2)** | 225.9 s (contended) | **132.7 s (clean)** |
| base@1t | 2250.6 s (contended) | 2896.3 s (contended; 2 of 5 starved) | 1 sample: 2432.7 s |

- **12 threads: no wall win.** mimalloc vs system: −2% / −3% with CPU medians
  equal (176 vs 176 s; 2887 vs 2908 s) — inside run-to-run spread.
- **Clean-machine 1 thread: no wall win either.** Phase-2 benchy@1t (machine
  quiet, all samples cpu/wall ≈ 0.99): system 131.1 s vs snmalloc 132.7 s
  (+1.2%) and vs mimalloc's contended 225.9 s which clean data shows would
  also land ≈ system.
- **Memory: replacements consistently LOSE.** Peak working set medians:
  benchy@12t 537 vs 435 MB (+23%); base@12t 2709 vs 2486 MB (+9%);
  snmalloc benchy@1t 406 vs 385 MB (+5%).
- Correctness: calicat smoke **byte-identical G-code across all three
  variants**; `--report` works under the wrapper with each inner allocator;
  24/24 focused bridge tests, clippy `-D warnings`, check-literals all green
  with the wiring in place. Real-model G-code hashing is uninformative (see
  §10.4).

### 10.3 New measurement trap: external load masquerades as contention

The session ran alongside other agents competing for CPU. Consequences, all
measured:

- The same benchy@1t combination measured **153–346 s (contended)** and
  later **128–139 s (clean)** — a 2.5x band *wider* than any allocator
  effect.
- The load signature is visible in recorded data: `cpu_seconds /
  wall_seconds` drops well below expectation (single-thread slices fell to
  0.49–0.86 when starved; clean runs sit at 0.98–1.00; 12-thread runs at
  ~6.5 on 12 logical CPUs). **Any future timing claim should quote this
  ratio per sample and exclude starved ones.**
- §9.7's "51.4 s aggregate worker CPU vs 33.2 s wall / 23.6 s serial" reading
  is hereby attributed to this trap: summed worker elapsed includes waits and
  descheduling (it is not CPU time — `ProgressPipelineInstrumentation` and
  `Collector::now_ns` both use `std::time::Instant`), and the machine was
  loaded. The allocator-contention hypothesis it motivated is dead.

### 10.4 Known-nondeterministic G-code: hashing cannot prove parity

Per-run sha256 differed across all 5 runs of the same variant on benchy and
base (DEV-093 medial-axis sliver; same-binary size deltas of tens of bytes).
Cross-variant size deltas are the same magnitude, so hash equality is
uninformative on these models; the calicat smoke's byte-identity plus
`slice_stats`/section-count agreement is the parity evidence that stands.
Line-aligned diffs are inflated by line shifts; compare section counts and
stats, not raw aligned lines.

### 10.5 Code changes that REMAIN in the tree (uncommitted, validated)

Not from the allocator experiment — from the §9 follow-through + review:
- `crates/slicer-core/src/algos/bridge_over_infill.rs`:
  `depth_window_start` / `filled_window_start` public window helpers.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs`: hoisted guards,
  lazy erosion, parallel qualification + serial commit, comment fixes, reused
  region lookup, and a new `internal_bridge_gate_is_thread_count_independent_with_mixed_skips`
  unit test (1 vs 4 threads).
- `crates/slicer-runtime/src/layer_executor.rs`: depth windowing in the
  per-layer bridge arm.
- `crates/slicer-core/tests/bridge_over_infill_tdd.rs`: two
  full-prefix-vs-window equivalence tests (cutoffs, gaps, empty windows).
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`:
  staircase ironing assertion anchored to `bed_center_mm` (§9.5 fix).

**Reverted from the tree per decision:** all allocator wiring (Cargo.lock,
`crates/pnp-cli/Cargo.toml`, `crates/pnp-cli/src/main.rs` ALLOC statics) plus
two unrelated reflow-only hunks in `crates/pnp-cli/src/{main,support_preview}.rs`
that were not part of any session's work. `cargo check --workspace
--all-targets` exits 0 after the revert.

### 10.6 Scratch inventory (kept under `alloc-bench/`, ~350 KB)

`EXPERIMENT.md` (plan + execution + results record), `run_bench.ps1`
(measurement harness: wall/CPU/peak-WS, CSV with gcode sha),
`run_matrix.ps1` / `run_matrix_phase2.ps1` (interleaved schedules),
`compare_gcode.py` (metadata-vs-toolpath diff classifier),
`results/*.csv` (all samples; `matrix_phase2.csv.prev-*` are the two partial
phase-2 runs), `config.json` (config snapshot), `smoke_report*.html`.
Deleted: G-code corpus (~943 MB) and the three variant exes. Harness caveats
for reuse: CLI flag is `--model` (not `--input` as §4 sketched); stderr must
be redirected during timing (progress JSONL adds I/O noise); peak-WS sampling
can miss short spikes.

### 10.7 Next experiment — user directive: re-profile, then derive a new hypothesis

Per user decision the next session starts **not** from a handoff guess but
from a fresh profile of the current tree:

1. `cargo build --release -p pnp-cli && cargo xtask build-guests --check`
2. Re-run `pnp_cli slice --model tmp/base.stl --config
   alloc-bench/config.json --module-dir modules/core-modules
   --instrument-stderr 2> profile.jsonl` on a **quiet machine** (see §10.3;
   record process CPU/wall externally before trusting timings; events do not
   contain CPU time).
3. Update the §5 stage tables and re-rank §6 leads from the new data; §9.7's
   benchy split (gate 1.90 s, Pass 1 0.64 s, Pass 2 0.33 s, bridge gate 0.08 s)
   and its `unsupported_span_areas` / `deep_infill_clip_area` / `retain`
   erosion list are the prior expectation to confirm or overturn.
4. Derive the next hypothesis from the measured split — candidate directions
   already on file: the batched host services instrumentation gap (§6 lead 6,
   prerequisite for lead 5), the surviving `compute_xy_footprint` union (lead
   2), and `apply_opening`'s round-join arc-tolerance 0 anomaly (§9.1).
5. Attribution runs only: `--instrument-stderr` inflates wall (§3.2) and
   `--report` enables allocator accounting — keep both out of any wall-time
   claim.

### 10.8 Refresh executed — 2026-09-05

The profile-first directive is executed. See the refreshed §5/§6 and
`perf-refresh/FINDINGS.md`; complete evidence is in
`perf-refresh/MEASUREMENTS.md` and adjacent raw logs. Production code was
unchanged. Release build, guest freshness (exit 0), module diagnosis, slice
completion and saved-log summarization passed. No Rust tests were run.

The machine did not stay quiet, including during the repeat batch. Baseline
medians are retained as load-qualified observations; no controlled speedup or
instrumentation-overhead claim follows. Tree-support planning remains first
among serial modules in both base profiles and the benchy cross-check.

**Timing correction for historical tables:** every per-layer entry labeled
"CPU" above is accumulated worker wall-clock, not process CPU time. Current
process CPU is measured externally with Windows GetProcessTimes after exit.

**Output correction:** TYPE section counts are not universally stable. In this
capture `Inner wall` varied 525–527 on base and 244–251 on benchy, even with the
same executable. Other TYPE counts remained stable. All base runs remained
degraded with 29,108 non-fatal errors; benchy remained clean. These observations
do not prove fine-grained parity or repair the support correctness issue.

Recommended next decision: instrument tree-planner substage and host-batch
boundaries, including surrounding validation/commit, before selecting an
algorithmic optimization. No implementation is authorized by this record alone.

**User decision after reviewing the results:** take the **perimeter and dispatch
split** next, rather than the planner recommendation. Investigate classic
perimeter algorithm cost versus dispatch/conversion/IR setup and the clustered
per-layer module timings. Record a focused attribution experiment before choosing
an optimization. This is the next-session direction; production edits were not
started in the profiling session.

Executed 2026-09-06 — see §11.

---

## 11. Landed 2026-09-06 — arena-owned prepared-region cache (proposal #1 executed)

The perimeter/dispatch split (§10.8 / `perf-split/FINDINGS.md`) attributed
~97% of the clustered per-layer modules' dispatch cost to host-side input
marshalling, with `derive_needs_support` and region-field conversion re-executed
by every module that receives slice regions. Ranked proposal #1 — memoize
per-region host data — was designed, implemented, measured, and **landed at
commit `bf11d055`** ("perf: memoize host region preparation in LayerArena").

### 11.1 What was built

- `PreparedRegionData` (`crates/slicer-ir/src/prepared_regions.rs`): plain IR
  data per region — `needs_support`, resolved `Option<SurfaceGroup>`,
  region-clipped `Vec<QuartileBand>`, flattened overhang `Vec<ExPolygon>`,
  previous-layer boundary. Re-exported from `slicer_ir::lib`.
- Pure derivation: `prepare_regions` / `prepare_slice_regions` /
  `prepare_perimeter_source_regions`
  (`crates/slicer-wasm-host/src/marshal/prepared.rs`), extracted from the
  duplicated logic that was in both `sliced_region_to_data`
  (`crates/slicer-wasm-host/src/marshal/in_.rs`) and
  `populate_surface_classification_fields` / `build_native_layer_request`
  (`crates/slicer-wasm-host/src/marshal/native.rs`).
- `LayerArena` (`crates/slicer-runtime/src/blackboard.rs`) owns two prepared
  slots — ordinary regions and perimeter-source regions — with
  `ensure_prepared_regions` / `ensure_prepared_perimeter_source_regions`
  called on demand. Invalidation rides the existing slice lifecycle:
  `take_slice` discards, successful `set_slice` starts unprepared, failed
  occupied-slot `set_slice` preserves the existing pairing, `reset` clears.
  **No generation counter was needed** — the arena mutation audit found every
  slice mutation passes through take/set (perimeter fill partitioning via
  `sync_perimeter_infill_areas_into_slice`, slice/bridge postprocess), so the
  take/set/reset seam IS the invalidation boundary.
- `prepare_dispatch_region_views` (`crates/slicer-runtime/src/layer_executor.rs`)
  gates preparation per stage: `Layer::Perimeters` prepares the
  perimeter-source projection; `Layer::Infill`, `Layer::SlicePostProcess`,
  `Layer::Support`, `Layer::AnchoredEvents`, `Layer::SupportPostProcess`
  prepare ordinary regions; all other stages prepare nothing. Wired into BOTH
  executor paths (`execute_single_layer_inner` and `execute_captured_stages`).
- `LayerStageInput` (`crates/slicer-wasm-host/src/binding.rs`) carries
  `Option<&[PreparedRegionData]>` for both projections — IR-typed borrows only,
  per ADR-0005.
- Both adapters consume the shared preparation; when the projection is `None`
  (test fixtures, non-standard runners) they fall back to the original
  per-dispatch derivation. **Dispatch semantics unchanged**: per-region config,
  held claims, region filtering, support carriers, WIT handles all remain
  per-dispatch.

### 11.2 Reuse proof (temporary probes, since removed)

Process-wide counters measured during the experiment (removed before commit;
all probe code was `[PERF-CACHE-PROBE]`-tagged and is not in the tree):

- benchy: `derive_calls=642 reuse_hits=1278` — ~66% of preparation calls
  served from cache; remaining derivation ~3.9 s accumulated.
- calicat smoke: 503 derivations vs 897 reuses.

### 11.3 Measured results (the acceptance evidence)

All runs 12 threads, `alloc-bench/config.json` (tree-support config),
`--module-dir modules/core-modules`, external measurement via
`alloc-bench/run_bench.ps1` (wall/CPU/peak-WS, gcode sha).

**Benchy (240 layers), 5-run medians, baseline vs candidate:**

| metric | baseline | candidate | delta |
| --- | ---: | ---: | ---: |
| process CPU | 176.53 s (174.97–176.69) | 169.89 s (168.42–170.44) | **−3.8%, non-overlapping** |
| wall | 31.75 s (30.44–33.55) | 28.95 s (28.06–31.93) | improved but noisy |
| peak WS | 433.4 MB | 433.9 MB | unchanged |
| guest fuel | 1,365,752,927,064 | 1,365,751,065,509 | 0.00014% — guest work untouched |

**Base (495 layers), 2+2 interleaved runs:**

| metric | baseline | candidate | delta |
| --- | ---: | ---: | ---: |
| process CPU | 2935.6 / 2911.1 s | 2735.9 / 2775.8 s | **~−5.8%, non-overlapping (worst candidate beats best baseline by 135 s)** |
| wall | 726.3 / 743.0 s | 1188.1 / 553.4 s | **inconclusive — heavy external load noise; no wall claim either direction** |
| peak WS | 2.471 / 2.464 GB | 2.475 / 2.474 GB | unchanged |
| non-fatal errors | 29,108 | 29,108 | identical — degraded-support condition (DEV-167) preserved |
| gcode bytes | 30,163,402/30,163,567 | 30,163,898/30,163,327 | same variance band (§3.6) |

Baseline binaries preserved at `target/perf-baseline/pnp_cli-baseline.exe`
(HEAD `96be6671`, pre-change) and `target/perf-base-cand/pnp_cli-candidate.exe`
(probe-free candidate). Raw CSVs: `target/perf-baseline/benchy_baseline.csv`,
`perf-cache/benchy_candidate.csv`, `target/perf-base-runs/base_runs.csv`.

**User decision after seeing results:** "too small to be worth keeping" was
considered for benchy alone; the base measurement (~−170 s process CPU on the
495-layer model, plus eliminated ~66% of preparation calls) settled it as
**KEEP**. The tradeoff: contained complexity (two arena slots + one pure
derivation module; safe fallback everywhere) against a repeatable −4 to −6%
process-CPU reduction that scales with layer count and module count per layer.

### 11.4 Verification (what ran at commit time)

- `cargo check --workspace --all-targets` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask build-guests --check` — exit 0.
- `cargo test -p slicer-runtime --test unit` — 94/94.
- `cargo test -p slicer-runtime --test contract` — 296/296.
- `cargo test -p slicer-runtime --test executor` — 225/225.
- `cargo test -p slicer-runtime --test integration` — 338/338 (one
  `pnp-cli-locator` staleness artifact, §9.6 harness trap, re-verified green
  after `cargo build --bin pnp_cli`).
- `cargo test -p slicer-wasm-host --test contract` — 122/122, including the
  new `prepared_region_projection_matches_fallback_projection`
  (`crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs`).
- New arena lifecycle tests: five tests in
  `crates/slicer-runtime/tests/unit/blackboard_layer_arena_tdd.rs` covering
  prepare→reuse, take→invalidate, set→unprepared, reset→clear, failed-set
  preservation.
- **Not run:** `cargo test --workspace` (reserved for explicit request or
  packet close-out), e2e bucket (the one e2e file touched was
  content-identical to HEAD; e2e was green at the same change-set in the
  prior session's runs).

### 11.5 Where the time goes now — and the ranked follow-ups

With marshalling memoized, the 2026-09-05 §5 ranking updates to:

1. **Classic perimeters guest clipper2 work — the dominant remaining cost.**
   The fuel profile at the landed commit still shows
   `com.core.classic-perimeters` at 89.3% of total fuel, with
   `polygon_ops::offset2_ex` at 12.6% (960 calls, 172.1 B fuel on benchy) and
   `<module self>` at 76.7%. The pre-investigation (see §10.8 follow-up
   explorations in this session) identified the ranked candidates:
   - **`offset2_ex` / `opening_ex` host migration** (highest confidence): both
     remain guest-local; ADR-0055's historical profile attributed ~12.9 B fuel
     to `offset2_ex` before and after earlier migrations — the residual never
     moved. Moving it requires a NEW host-service contract decision (it has no
     WIT form today; `crates/slicer-schema/wit/deps/common.wit::host-services`
     covers only singular/batch offset, clip, simplify). Preserve miter limit
     and mm parameters; `opening_ex` must route through the same primitive.
   - **`split_top_surfaces`** (`crates/slicer-core/src/top_surface_split.rs`) —
     plausible secondary; already short-circuits on empty top_solid_fill.
   - **NOT worth attacking first:** batching the serial inset loop
     (`emit_walls`) — each inset consumes the previous result; ADR-0049
     explicitly keeps dependent operations singular. `BatchMode::for_work`
     would keep two-item batches serial anyway.
2. **The integrated-vs-external G-code divergence** (§10.8) is still
   **unresolved and blocks the integrated path as a perf oracle**. Confirmed
   native/WASM asymmetries from this session's explorations:
   - Native builds views for EVERY slice region; WASM filters first via
     `module_receives_slice_region` (`crates/slicer-wasm-host/src/dispatch.rs`).
   - Native does NOT apply per-region `RegionMapIR` config overrides
     (`build_native_layer_request` assigns `module.config_view` only); WASM
     resolves `config_fields_per_region` through `HostExecutionContext`.
     **This is a confirmed semantic parity gap, not just perf.**
   - Surface-field derivation duplication is now SHARED via the prepared cache
     — that suspect is eliminated by construction.
   - Proposed discriminating experiment: extend
     `native_and_wasm_layer_views_are_field_identical`
     (`crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs`) with
     an excluded region + a `RegionMapIR` override + non-empty classification
     + claims, and assert count/ID/config/claim equality.
3. **Tree-support planner** remains the largest serial prepass module
   (§5/§10.8); its substage attribution is still undone and the batched
   host services still lack instrumentation.

### 11.6 Measurement notes for the next session

- The 2+2 interleaved protocol (baseline→candidate→baseline→candidate) worked
  well on base where wall is noisy: process CPU was decisive with
  non-overlapping ranges at n=2 per side. Prefer it over 1+1.
- **Do not chase base wall time.** Candidate run 1 measured 1188 s wall
  against 2736 s CPU (descheduled); run 2 measured 553 s. Both against
  identical binaries. Wall on base is only usable with the CPU/wall ratio
  quoted per sample (§10.3).
- The `pnp-cli-locator` staleness trap (§9.6) fired again during integration
  testing — after touching `crates/*/src`, run `cargo build --bin pnp_cli`
  before believing an integration/e2e red.
- Probe discipline that worked: tag every probe block with one grep-able
  marker comment, make removal a mechanical pass, and re-run the full gate
  set plus grep-confirmation of zero remnants before committing.

### 11.7 Scratch inventory for this session

- `target/perf-baseline/` — baseline binary (HEAD `96be6671`) + benchy
  baseline CSV + one gcode sample.
- `target/perf-base-cand/` — probe-free candidate binary (= `bf11d055`).
- `target/perf-base-runs/` — base 2+2 CSV + per-run stderr JSONL + gcode.
- `perf-cache/` — benchy candidate CSV, probe-run stderr with
  `[PERF-CACHE]` summary lines, fuel summaries (`fuel_base_summary.txt`,
  `fuel_cand_summary.txt`) and their raw profile JSONL.
- All under git-ignored paths; nothing here is committed.

## 12. Perimeter study — 2026-09-06/07

Benchy-first support-off attribution for both external perimeter generators is
recorded in `perf-perimeters-study/FINDINGS.md`. Three probe-free runs per
generator completed with `--module-dir modules/core-modules` after guest
freshness exit 0. Classic temporary scopes locate 85.8% of classic-module fuel
in wall assembly and 8.0% in gap-fill (fuel, not wall/CPU). Arachne's host
probe attributes preprocess/graph construction to 75.3% of the enclosing
pipeline interval; all named stages sum to 92.3%, leaving a measured 364.932 ms
remainder. Input/output conversion was measured separately. No optimization
implementation or permanent probe remained from that attribution study. The
subsequent optimization is recorded below.

## 13. Accepted shared wall-flags fast path — 2026-09-07

### Decision and implementation

The user chose to **KEEP**, then requested a commit. The commit subject is
`perf: skip wall paint reprojection without effective annotations`; re-derive
its identity from `git log` rather than treating the current HEAD as permanent.
Acceptance was explicitly revised **for this candidate**: repeatable process-CPU
savings justified this contained change despite an unproven wall-time benefit.
This is not blanket authorization to relax acceptance for future optimizations.

`build_wall_flags` (`crates/slicer-core/src/perimeter_utils.rs`) now seeds the
usual flags and `variant_fuzzy`, then returns the normal outer/inner boundary
fallback if no effective material or fuzzy annotation can affect the wall.
Material `ToolIndex` and fuzzy `Flag(true)` retain the original processing path.
The predicate follows the original selection rules: index mode examines the
selected polygon; reprojection mode examines all reachable original polygons.
Callers, geometry tolerances, and host/WIT contracts were not changed.

Known defensive behavior change: ineffective annotations plus an empty supplied
reprojection ring and positive point count now return defaults rather than
panicking. Production callers reject empty wall paths. Do not describe this as
equivalence for every malformed input.

Tests were added in
`crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs` for ineffective
annotations, exact flag/boundary outputs, fuzzy variants, polygon selection,
zero points, and the defensive empty-ring case. Existing material/fuzzy and
concave reprojection tests remain the annotated-path checks.

### Evidence and its limits

Start with `perf-flags/FINDINGS.md`, `EXPERIMENT.md`, and their raw CSV/logs.
Baseline host and guest artifacts were preserved separately from the candidate;
an executable snapshot alone does not isolate guest-side changes.

Valid earlier interleaved tree-support results, process-CPU medians in seconds:

| Fixture | Classic baseline → candidate | Arachne baseline → candidate |
| --- | ---: | ---: |
| Benchy | 171.6016 → 165.3829 | 98.5547 → 94.6406 |
| base | 2846.7969 → 2560.8672 | 1376.3125 → 1342.5000 |

These are historical measured samples, not a baseline for future HEAD. Base
tree-support runs preserved `degraded: true`, zero fatal errors, and 29,108
non-fatal errors. The known support defect was not repaired. Arachne captured
output matched baseline; Classic has same-binary output nondeterminism, so
coarse G-code checks are smoke evidence and focused exact-output tests establish
the local flag/boundary invariants.

The later quiet follow-up used warmups and ABBA BAAB. Its valid Classic rows
again showed lower CPU, but overlapping wall ranges and slightly higher wall
medians. **No reliable wall improvement is established.**

### Discovered handoff audit error: quiet Arachne labels were Classic

The quiet scripts `run-measured.ps1` and `run-supports-on.ps1` under
`perf-flags/quiet-validation/` used their `$gen` loop variable for labels but
reused Classic configuration. Files labeled Arachne contain
`; wall_generator = Classic`, and their stderr drops Arachne because Classic
holds `perimeter-generator`. Their Arachne-specific CPU/wall conclusions are
invalid. Corrections are prepended to the supporting reports; raw captures are
preserved. Earlier interleaved Arachne captures selected Arachne correctly and
remain usable. The error was found after acceptance/commit and disclosed to the
user while preparing this handoff; it does not alter the committed source.

**Required measurement fix:** give each generator an explicit config with
top-level `"wall_generator": "classic"` or `"wall_generator": "arachne"`.
Validate actual per-run dispatch from stderr and emitted G-code before accepting
the sample. `module diagnose` lists available manifests and is insufficient to
prove which competing perimeter generator ran. Repair the quiet harness before
reusing it; do not blindly repeat its current scripts.

### Validation at completion

- `slicer-core --features host-algos`: material-boundary target 10 passed;
  concave-reprojection target 2 passed.
- Classic module boundary-paint target: 7 passed; Arachne equivalent: 4 passed.
- `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo xtask check-literals`: passed.
- Guest freshness: exit 0. Final pre-commit diff check passed.
- No full workspace test run. No permanent profiling probes.

### Next optimization session

Continue **both classic and Arachne**, independently. User prefers subagents
for implementation/study execution, with a single owner for builds and timing
to prevent self-induced contention. Start with grilling to select the next
candidate and confirm acceptance; implementation is not authorized by this
handoff alone.

Re-attribute against the committed fast path before trusting §12's hotspot
shares: that profile includes the now-removed annotation-free scans. Remaining
leads, all requiring current-code verification and measured impact
(**re-ranked 2026-09-22 — see §14**; lead 1 below is CLOSED by measurement
and lead 4 is superseded by §14's ranked list):

1. **CLOSED 2026-09-22 (§14), kept for history.** Classic
   `ClassicPerimeters::emit_walls`
   (`modules/core-modules/classic-perimeters/src/lib.rs`): retain only the
   first inset needed for seam generation rather than cloning/storing every
   inset. Preserve first-wall island order and gap geometry. Measured: the
   clone/store is 0.0014% of module fuel.
2. Effective painted walls in `build_wall_flags`
   (`crates/slicer-core/src/perimeter_utils.rs`): reuse nearest-original
   reprojection results for flags and material transitions. The accepted empty
   annotation fast path does not optimize genuinely painted walls; use a
   representative painted workload before selecting this candidate.
3. Arachne `preprocess_input_outline` /
   `run_nine_stage_pipeline` (`crates/slicer-core/src/arachne/preprocess.rs`)
   and `SkeletalTrapezoidationGraph::from_polygons`
   (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`): refine attribution
   before proposing pass removal or indexing changes. Preserve canonical stage
   ordering, topology, ties, and graph invariants.
4. Classic `offset2_ex` / `opening_ex` host migration remains a possible new
   contract decision, not the automatic next optimization. Host contracts are
   allowed for discussion, but fuel share does not prove a wall win.

Maintain equivalent intended geometry. Begin with supports-off Benchy, compare
each generator against its own baseline, then validate promising changes with
tree supports and repeated base runs before retention. Separate process CPU,
whole-slice wall, accumulated worker elapsed, and fuel. Use uninstrumented
probe-free acceptance runs, isolated host/guest snapshots, warmups and balanced
ordering. The integrated/native path still has unresolved filtering/config
asymmetries and is not a parity oracle. Return measured keep/drop recommendations
for user decision; do not automatically commit future candidates.

## 14. 2026-09-22 — emit_walls premise falsified; classic-perimeter re-attribution

Session direction was settled by grilling (decisions Q1–Q15 on file in the
session record): candidate = classic `emit_walls` inset retention (§13 lead 1),
attribution first, strict acceptance gate (CPU **and** wall must win; wall
verdict on supports-off benchy, n≥5 interleaved with starvation exclusion;
borderline CPU-win/wall-noise returns to the user). The attribution **falsified
the premise** and the agreed fallback (Q13) ran: a full re-attribution of
`com.core.classic-perimeters` and a re-ranked lead list. No production code
changed; tree clean at HEAD `bde9b1ba`. Full evidence:
`perf-emit-walls/FINDINGS.md` + raw JSONL captures beside it.

Measured on supports-off benchy (classic config), guest fuel, exact (marks
burn no fuel; temporary ADR-0055 user scopes, removed after):

- `emit_walls`' inset clone/store: **0.0014%** of module fuel — §13 lead 1 is
  **CLOSED** for time (memory-shape idea only; peak RSS unmeasurable, DEV-026).
- `expolygon_to_path3d_indexed` per-vertex `overhang_quartile` /
  `signed_distance_to_boundary` queries: **69.8%** of total guest fuel
  (2,107 rings). These are legacy **linear scans** in ordinary builds —
  `pnp_perimeter_spatial_accelerated` is only injected by the controlled
  rustc driver (packet 254, `docs/23_controlled_perimeter_builds.md`).
- `PerimeterSpatialContext::is_bridge` per-point scan: 4.2%.
- `polygon_ops::offset2_ex`: 15.2% (960 calls — 720 from the `only_one_wall_top`
  `min_width_top` shrink/expand after `split_top_surfaces`, 240 from the
  gap-fill width band).
- `build_wall_flags`: negligible — the accepted empty-annotation fast path
  works on this workload.

Re-ranked leads (user decision required; nothing authorized or implemented):

1. **Accelerated perimeter-spatial adoption** via the packet-254 acceptance
   campaign — addresses the measured 74% (path3d + bridge scans) exactly.
2. **`offset2_ex` call-count reduction** in the `min_width_top` shrink/expand
   (fuel 15.2%; fuel≠wall, gate before keeping).
3. **Consume-only-when-read per-vertex annotations** in
   `expolygon_to_path3d_indexed` (complements lead 1 in accelerated mode).
4. Measurement question: `host:slice` `closing_ex` scope spans (22.3 s
   accumulated) contradict `host:slice`'s `module_complete` wall (3.2 s) —
   likely the §3.3 accumulated-worker trap; verify `fold_marks` thread
   handling before acting.

**Mode-comparative addendum (2026-09-22, same session):** the captures above
were ordinary-mode (accelerated cfg disabled) — fair for the falsification
and for "today's build", but the shares below lead 1 were ordinary-only. An
accelerated pair (`cargo xtask dist --accelerated`, complete snapshot under
`target/dist-accelerated/developer/`; the bare artifacts dir has no manifests
and silently loads integrated modules, and dual `--module-dir` does NOT
shadow) measured: total guest fuel **−35.8%** (1,133.9 B → 727.6 B),
classic-perimeters −39.9%, `offset2_ex` and `infill-linker` bit-identical.
The hot queries shrank only **~1.9×**, so accelerated mode is necessary but
likely **not sufficient**: revised accelerated-mode shares are classic self
60.4%, `offset2_ex` 23.7%, `infill-linker` 15.3% (new entry — untouched by
acceleration). Lead 3 must be scoped against accelerated mode; lead 2 rises
in relative terms (calicat: `offset2_ex` = 53.7% of classic fuel). Coarse
output parity held (173 B on 4.29 MB; TYPE counts within known instability);
one-run wall 27.9 s → 19.0 s under `--profile` (indicative). Full table in
`perf-emit-walls/FINDINGS.md` addendum.

Also this session: the last unrepaired quiet-validation script
(`perf-flags/quiet-validation/run-warmups.ps1`) now uses explicit
per-generator configs with `-ExpectedGenerator` validation; the generator
evidence regression checks (`alloc-bench/regression-check-generator-validation.ps1`)
pass (11 cases, including the exact audit-failure case), and the repaired
script was smoked end-to-end on benchy — 4/4 warmup rows with
`validated_generator == expected_generator`, completion `ok`. Ledger drift
noted: `module diagnose` now reports **24** external
modules (this doc's §3.4 said 23 and has been corrected); the "all core
modules load externally" invariant holds.
