# Base refresh at post-t70 HEAD

**Three new investigation questions are justified, and the below-fold park now
guards the largest measured per-layer term.** The four base cells re-derive as
prepass-dominated (54–84 % of process wall on the instrumented runs), with a
supports-on finalization residual that costs 45–49 s of module brackets for
~0.04 % of the slice's guest fuel, a 26–31 s serial ShellClassification commit,
and a 19–22 s serial MeshAnalysis union. The existing annotation park (ticket
20) is unchanged — the best cell is still 3.30x — but its base-classic target is
now measured at 94–98 s of per-layer wall.

Route: [Base current bottleneck refresh](../../issues/73-base-current-bottleneck-refresh.md).
This take is attribution and route expansion: no candidate, acceptance
campaign, adoption retry, default switch or production edit.

## Method and evidence

- Rebuilt `target/release/pnp_cli.exe` at HEAD `86c15182` (the slicer sources
  are the t70 keep `7127b4d6`; the HEAD commit is the t72 docs commit) with
  `cargo build --release --bin pnp_cli`. No production source was edited.
- `cargo xtask build-guests --check` exited 0 before the batch. Isolated
  `module diagnose` passed with 24/24 modules `external`. Input/artifact hashes
  and the captured revision are in `inputs.json`.
- Eight sequential ordinary-mode base slices, 12 rayon threads, the map's
  existing matched configs: four fuel profiles
  (`--profile --profile-verbose --instrument-stderr`) and four
  instrumentation-only corroborations (`--instrument-stderr`), one per
  generator × support state.
- Every output is byte-identical to its accepted t58 qualification reference
  (`../t58-finalization-copies/qualification.json`), including TYPE counts.
  Every completion is `ok`, `degraded=false`, zero fatal/non-fatal errors.
  **This is not a warning-free or geometry-correctness claim**; see the warning
  disclosure below.
- Optional fuller raw source: `target/perf-refresh-base-2026-10-18/` (8 G-code
  outputs + stderr JSONL, gitignored). The in-repo `captures/` retains phase
  and serial brackets, every layer's start/end, every per-layer module
  completion/fuel count, process measurements, output census and the full
  aggregate profile. `reduce.py` reconstructs that set from raw captures and
  hashes their G-code/JSONL; `verify.py` runs using only in-repo evidence and
  re-derives `summary.json`; `analyze.py` prints the tables below.

**Units:** fuel is guest instructions, not native cost. Host profile values and
per-layer module sums are accumulated thread-time, not slice wall or process
CPU. Instrumented and profiled runs are attribution only; no fresh
uninstrumented timing, candidate A/B, accelerated run or Orca run was
performed. No speedup or scoreboard revision follows from these captures.

## Fresh phase anatomy

Instrumented (observer-mode corroboration) and profile values are separate
columns; do not combine them into a median. Stage values are diagnostic
brackets in seconds.

| base cell | observer | wall | prepass | per-layer | postpass |
| --- | --- | ---: | ---: | ---: | ---: |
| classic supports-off | instrumented | 214.5 | 115.0 | 94.3 | 2.4 |
| classic supports-off | fuel profile | 238.4 | 120.6 | 110.6 | 3.0 |
| classic supports-on | instrumented | 408.7 | 237.6 | 98.2 | 69.1 |
| classic supports-on | fuel profile | 487.2 | 288.4 | 117.3 | 76.7 |
| arachne supports-off | instrumented | 130.2 | 109.2 | 16.1 | 1.4 |
| arachne supports-off | fuel profile | 139.8 | 116.0 | 16.6 | 1.4 |
| arachne supports-on | instrumented | 360.2 | 259.9 | 19.8 | 76.6 |
| arachne supports-on | fuel profile | 368.0 | 269.4 | 22.9 | 71.8 |

Stage table, instrumented runs (seconds):

| stage | classic-off | classic-on | arachne-off | arachne-on |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::MeshAnalysis` | 22.04 | 19.40 | 21.78 | 22.02 |
| `PrePass::Slice` | 43.42 | 40.02 | 41.68 | 40.20 |
| `PrePass::OverhangAnnotation` | 10.59 | 9.48 | 9.84 | 9.27 |
| `PrePass::ShellClassification` | 30.00 | 26.02 | 26.87 | 28.23 |
| `PrePass::SupportAnalysis` | 0.00 | 14.01 | 0.00 | 13.17 |
| `PrePass::SupportGeometry` | 0.24 | 119.68 | 0.24 | 136.02 |
| `PrePass::SeamPlanning` | 8.62 | 8.88 | 8.68 | 10.85 |
| `PostPass::LayerFinalization` | 1.45 | 55.65 | 0.31 | 60.22 |
| `PostPass::GCodeEmit` | 0.17 | 7.13 | 0.13 | 8.29 |

The **common prepass floor** (MeshAnalysis + Slice + OverhangAnnotation +
ShellClassification + SeamPlanning) is 104–115 s on every base cell; the
supports-on cells add 134–149 s of SupportAnalysis + SupportGeometry. The
per-layer phase is 94–117 s on classic and 16–23 s on Arachne; the supports-on
postpass adds 56–62 s of LayerFinalization and 7–8 s of GCodeEmit.

The low-layer finishing tail measured on Benchy (ticket 63) **does not
reproduce on base**: the low-layers-only finishing interval is 0 ms in all
eight captures.

## New lead: supports-on finalization view cost

The four whole-print finalization modules are called once per slice in a fixed
order. Their module brackets jump by 30–150x between support states, while
their guest fuel is negligible:

| cell | classifier | part-cooling | skirt-brim | wipe-tower | module sum | stage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| classic-off (inst.) | 0.36 s | 0.34 s | 0.34 s | 0.35 s | 1.39 s | 1.45 s |
| arachne-off (inst.) | 0.09 s | 0.07 s | 0.07 s | 0.07 s | 0.30 s | 0.31 s |
| classic-on (inst.) | 10.11 s | 11.61 s | 11.07 s | 12.25 s | 45.04 s | 55.65 s |
| arachne-on (inst.) | 11.42 s | 12.20 s | 12.48 s | 12.40 s | 48.50 s | 60.22 s |

The four modules' total guest fuel on the classic-on profile is **5.45 B of
12,964 B (0.042 %)** — each module is ~1.35 B. Ticket 58 already measured the
same order (module fuel ≤ 1.4 B of 13,090 B) and localized the old cost to
whole-print host copying; its clone-once fix removed the inter-bracket gaps,
not this per-module bracket. The `overhang-classifier-default` early-return
audit (ticket 20) says the matched configs return before reading a stamp, yet
its bracket is 10.1–12.1 s on supports-on. The brackets are therefore
**host-side view construction / dispatch / output merge**, not module compute,
and they scale with the supports-on entity volume rather than the module's
declared reads.

Precise question graduated: [LayerFinalization supports-on whole-print view
cost](../../issues/74-layerfinalization-supports-on-view-cost.md) — decompose
the four brackets into host input-view construction, guest dispatch, and
output merge; explain the supports-off/supports-on jump; test a
representation-preserving reduction (materialize only the entity classes each
module declares, skip a module whose own liveness predicate is false, or share
an immutable input view where the contract permits). Output identity + the
standing paired A/B decide keep/drop. Ranked first: the largest un-attributed
supports-on term, 45–49 s of module brackets on both base supports-on cells.

## New lead: ShellClassification phase-B serial commit

`PrePass::ShellClassification` is 26.0–31.4 s of wall in every base cell. Ticket
27's 2026-09-27 probe measured its `gate_phase_b_commit_wall` at **30.29 s
(base-off) / 25.07 s (base-on)** — a serial commit loop — and named the
per-surface `expand(closing(spacing))` / `intersection` chain plus
`internal_bridge_angles` over three areas as its interior. The fresh stage
walls are the same order, so the serial commit is still the largest
unaddressed serial block outside `PrePass::Slice`.

The host scope split at HEAD (profile, accumulated): `polygon_ops::offset`
96.6–117.2 s, `clip_polygons` 11.4–13.3 s, `closing_ex` 2.3–2.6 s. The stage's
~4x worker sum over wall is consistent with a serial commit surrounded by a
parallel region.

Precise question graduated: [ShellClassification phase-B serial commit
reduction](../../issues/75-shellclassification-phase-b-commit.md) — re-derive
the phase-B split at HEAD and test the t27-named input pre-reduction
(`internal_bridge_angles` intersects each qualified polygon against three
areas) and, if the per-surface operations are independent, a parallel commit.
Output identity + the standing paired A/B. Ranked second: 26–31 s on all four
base cells (and a smaller Benchy term), with a named candidate shape rather
than a new search.

## New lead: MeshAnalysis exact footprint union

`PrePass::MeshAnalysis` is 19.4–22.2 s on every base cell, and
`host:mesh_analysis`'s only scope is `polygon_ops::clip_polygons` at 19.2–20.1 s
accumulated — a **serial** union (concurrency ≈ 1x). Ticket 27 localized it to
`compute_xy_footprint`'s exact union over 326,546 projected facets (~75 % of
the stage) and documented that the consumers only use the footprint through
bbox tests or exact intersection. Its proposed bbox/coverage replacement is
**unsafe as stated** (the exact-intersection consumer was verified in the
premise audit), so the surviving question is an exact, representation-preserving
reduction, not a bbox substitution.

Precise question graduated: [MeshAnalysis exact footprint union
reduction](../../issues/76-meshanalysis-footprint-union.md) — determine whether
the exact union can be partitioned/merged in parallel or its input reduced
while preserving the downstream footprint representation; bbox-only
replacement is out of scope. Output identity + the standing paired A/B. Ranked
third: 19–22 s on the four base cells, with the exactness constraint the gate.

## Existing leads updated, not duplicated

- [Consume-only-when-read annotations](../../issues/20-consume-only-when-read-annotations.md):
  on base classic the per-layer phase is 94.3 s (off) / 98.2 s (on) instrumented
  — 44 % / 24 % of process wall — and `com.core.classic-perimeters` is 98.8 % /
  95.0 % of whole-job guest fuel with 10,573 B self fuel and 1,186.7 / 1,195.1 s
  accumulated wall. Ticket 18's accelerated stage-2 capture put the two exact
  scan families at 57.9 % of the base classic module; the t20 audit says the
  matched configs return before reading the stamps. The below-fold park is
  **not lifted**: the t72 best cell is still 3.30x, and the park's condition is
  unchanged. This ticket records the base size of the parked term for the
  human's route decision; the measured scan share is an attribution share, not
  a wall prediction.
- [PrePass::Slice flat-bridge closing reduction](../../issues/59-prep-slice-flat-bridge-closing-reduction.md):
  `PrePass::Slice` remains 40.0–44.9 s on every base cell; t59's
  no-representation-safe-reduction verdict is unchanged. No new ticket.
- The tree-support planner lineage (tickets 22/53/55/57): `PrePass::SupportGeometry`
  is 119.7–147.8 s and `com.core.tree-support-planner` 97.1–120.1 s serial on
  supports-on. Ticket 53 measured the native body union as the dominant term and
  ticket 55's exact corpus produced the projection-cache candidate that t57
  dropped as slower. At HEAD the remaining planner cost is native clipper work;
  the t70 engine keep already took the hot loop. No new ticket without a new
  batch/parallel host contract; the lineage stays in the map's fog.
- [Tree-support renderer attribution](../../issues/64-tree-support-renderer-attribution.md):
  `com.core.tree-support` is only 2.1 s accumulated on base supports-on, so the
  Benchy renderer ranking does not transfer. No new ticket.
- [Independent low-layer dispatch](../../issues/63-low-layer-dispatch-granularity.md):
  the base low-layer finishing interval is 0 ms; the Benchy tail does not
  reproduce. No new ticket.
- [Classic-perimeters host round trips](../../issues/68-classic-perimeters-host-round-trips.md):
  already 97–99 % native clipper ops; nothing new to attribute here.

## Output and warning disclosure

| cell | bytes | SHA-256 prefix | open-loop warnings | malformed-layer-marker warnings |
| --- | ---: | --- | ---: | ---: |
| classic supports-off | 22,428,863 | `20e7204c…` | 0 | 367 |
| classic supports-on | 43,912,792 | `f60e5e9b…` | 0 | 429 |
| arachne supports-off | 21,180,367 | `3ba16419…` | 82 | 361 |
| arachne supports-on | 42,664,440 | `a1bc8110…` | 82 | 431 |

The Arachne cells also carry one `preprocess_input_outline dropped a feature
smaller than epsilon_offset (0.012499 mm)` warning at the same centroid. TYPE
counts and complete hashes live in each reduced capture. Marker warnings report
fallback to the prior layer-Z context; open-loop warnings explicitly report a
missing closing edge. Their semantic impact was not investigated in this
performance-only refresh; no warning is hidden by the clean completion status.

## Verification and scope

```bash
python docs/specs/perf-vs-orca/evidence/perf-refresh-base-2026-10-18/verify.py
python docs/specs/perf-vs-orca/evidence/perf-refresh-base-2026-10-18/analyze.py
# Optional reconstruction of captures from the existing raw tree, no re-slice:
python docs/specs/perf-vs-orca/evidence/perf-refresh-base-2026-10-18/reduce.py --raw-root target/perf-refresh-base-2026-10-18
```

The verifier passes from the reduced set alone and rejects altered fuel,
output hash, missing layer and integrated provenance. Rust tests and workspace
clippy were not run: no Rust, WIT, manifest or production configuration was
changed and no production commit is being made. The rebuilt release executable
and raw files under `target/` are the only non-documentation artifacts produced.

Limits: ordinary mode only; no accelerated, Orca or fresh uninstrumented
comparison; profile brackets inflate the instrumented phase walls (e.g. classic
per-layer 110.6 vs 94.3 s) and are never acceptance timings; the four supports-on
module brackets include host dispatch and output merge, and their internal
split is the graduated ticket's job. No speedup, regression, scoreboard win,
geometry-correctness verdict, default change or production-source edit is
claimed. This take closes only the refresh; the three new tickets are
unclaimed investigations and ticket creation is not execution approval.
