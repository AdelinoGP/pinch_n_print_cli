# t65 — infill-linker batch-execute experiment: batch one engine + one execute
# per `link_paths_without_offset` invocation. Evidence record.

## Pre-declared protocol (written down before any measured run; updated only
## before the campaign, never after seeing timing)

Candidate (single change set, uncommitted):
1. `PreparedPolylineClip::clip_batched`
   (`crates/slicer-core/src/polygon_ops.rs`): one `Clipper64` (universe added
   once, all of the invocation's valid polylines added as open subjects once,
   one execute), then per-subject output attribution re-derived by tolerant
   segment incidence: every emitted open-path segment must ride exactly one
   subject's own segments (both ends and midpoint); each concatenated path is
   cut at subject boundaries into per-subject runs in encounter order.
   Ambiguous/unattributable segments (coincident / collinear-overlapping
   subjects are the known class) fall back to the per-subject `clip` chain so
   the per-path equivalence contract holds unconditionally. Single-subject
   invocations short-circuit to `clip` (identical engine shape either way).
2. `link_paths_without_offset`
   (`modules/core-modules/infill-linker/src/orchestrate.rs`): the per-path
   engine round trip is replaced by one batched call per invocation; every
   downstream step (per-subject `remove_short_polylines` filtering,
   `point_with_metadata`, `source_segments`/owner, subject-major order in
   `clipped`, `chain_or_connect_infill` on the same boundary graph, the
   ran-and-empty contract and the raw-boundary fallback arm) keeps its exact
   existing semantics.

In-guest replay oracle (probe build, [T65-PROBE] marks, removed before any
measured build): per invocation, the production path keeps the batched
result while the probe replays the pre-change per-path chain on the exact
same inputs and compares subject-for-subject; counters leave through the
module-log channel per layer (invocations/subjects/matches/mismatches/
batch_spans/replay_spans/singles/fallbacks). Fuel scopes t65::batch and
t65::replay bracket the batch call and the replayed per-path chain in the
same real runs.

Qualification gate (before timing): fresh 16-cell output matrix
{benchy, base} x {classic, arachne} x {supports off/on} x {ordinary,
accelerated}, baseline vs candidate byte-identical (sha256), clean outputs
(status ok, degraded=false, 0 fatal, 0 non-fatal), non-vacuous TYPE counts;
completed ordinary cells reproduce the preserved production references.

Timing gate (standing, unchanged): paired ordinary + accelerated A/B per
cell with 6 alternating baseline/candidate pairs plus a 2-run warmup per
arm; external GetProcessTimes process wall and kernel+user CPU at
RAYON_NUM_THREADS=12; per-sample cpu/wall quoted; starved pairs excluded
whole (ratio < 0.5x the arm's own median ratio), never resampled; a cell
with retained pairs < 6 is inconclusive, not a pass.

Timing cells (pre-declared): the four ARACHNE cells — benchy + base x
supports off/on — both modes. Justification (from t62's measured split,
re-derived 2026-10-13): the linker is 34.06-57.09% of guest guest fuel on
the arachne cells; on the classic cells it is 9.21-9.77%, so even a ~62%
linker-fuel cut is an expected wall effect below the paired spread (~0.5%
of wall) and cannot produce a decisive verdict; declaring it would only
risk an inconclusive noise cell. Measured keep/drop to the human;
candidates are never auto-committed. (This pre-declaration was recorded
2026-10-13 after the oracle passed, before any qualification/timing run.)

## Oracle (replayed on real inputs, in vivo)

**PASS on all captured cells (2026-10-13; probed candidate build, both
modes).** The probe replays the pre-change per-path chain on every real
`link_paths_without_offset` invocation and compares subject-for-subject;
production keeps only the batched result and outputs stayed byte-identical
to the t58 qualification references throughout, so the probe never changed
output (11 cells × 2 capture classes, outputs sha-compared below).

| cell | invocations | subjects | matches | mismatches | singles | whole-call fallbacks | repaired subjects | output ref-ok |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic off (ord.) | 396 | 12,951 | 12,951 | 0 | 10 | 0 | 2,193 | ✅ `34886ccb…` |
| benchy classic on (ord.) | 396 | 12,951 | 12,951 | 0 | 10 | 0 | 2,193 | ✅ `76eb4e1d…` |
| benchy arachne off (ord.) | 393 | 12,022 | 12,022 | 0 | 4 | 0 | 2,037 | ✅ `b76d846a…` |
| benchy arachne on (ord.) | 393 | 12,022 | 12,022 | 0 | 4 | 0 | 2,037 | ✅ `ae9d9061…` |
| benchy classic off (acc.) | 396 | 12,951 | 12,951 | 0 | 10 | 0 | 2,193 | ✅ |
| benchy classic on (acc.) | 396 | 12,951 | 12,951 | 0 | 10 | 0 | 2,193 | ✅ |
| benchy arachne off (acc.) | 393 | 12,022 | 12,022 | 0 | 4 | 0 | 2,037 | ✅ |
| benchy arachne on (acc.) | 393 | 12,022 | 12,022 | 0 | 4 | 0 | 2,037 | ✅ |
| base classic off (ord.) | 852 | 19,290 | 19,290 | 0 | 115 | 1 | 4,284 | ✅ `20e7204c…` |
| base arachne off (ord.) | 908 | 18,896 | 18,896 | 0 | 129 | 0 | 4,220 | ✅ `3ba16419…` |
| base arachne on (ord.) | 908 | 18,896 | 18,896 | 0 | 129 | 0 | 4,220 | ✅ `a1bc8110…` |

- Accelerated benchy outputs are hash-identical to the ordinary ones
  (mode-invariant output, per the t58 references).
- The v1 probe (whole-invocation fallback on ambiguity) measured 149/396
  fallback invocations on benchy arachne; the per-subject repair replaced
  it: the same invocations now repair only their flagged junction-pair
  subjects (2,037 subjects repaired, ~83% of subjects take the batch on
  benchy, ~78% on base) and whole-call fallbacks collapsed to 1 across all
  cells (one base-classic invocation).
- Batch-vs-replay clip fuel inside the same real instrumented runs
  (t65::batch vs t65::replay; attribution-only, instrumented): benchy
  arachne-off 23.668 B vs 67.736 B (**−65.1%**), benchy classic-off
  25.454 B vs 74.135 B (**−65.7%**), base arachne-off 63.078 B vs
  111.858 B (**−43.6%**). The same-cost ceiling from ticket 62 assumed the
  batched execute costs one per-path execute; the measured batch cost sits
  above that ceiling (bigger subject sets per execute) — which is exactly
  the caveat t62 recorded and why the wall verdict is the campaign's.
- Captures (raw, gitignored): `target/t65/oracle/`; in-repo reduced set:
  `oracle.json` (this directory), produced by `reduce_oracle.py` from the
  captured stderr JSONL + the t58 references; capture driver preserved as
  `capture-oracle.sh`; probe patch `probe-candidate-combined.patch` +
  `probe-t65-counters.rs.txt`.

## Test and gate record (static, run at the candidate tree)

- `cargo check --workspace --all-targets` — pass (1m47s measured wall).
- `cargo clippy --workspace --all-targets -- -D warnings` — pass
  (re-run per-crate after the probe removal: slicer-core + infill-linker
  clean).
- `cargo fmt` — clean on both changed crates.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask check-test-quality --report` — 0 findings
  (`polygon_ops_clip_batched_tdd.rs` included).
- `cargo test -p slicer-core --features host-algos` — 91 binaries, 642
  tests, 0 failures (feature-unified invocation per the AGENTS.md
  feature-gate discipline).
- `cargo test -p infill-linker` — all binaries ok.
- `cargo test -p slicer-runtime --test executor` — 234/234.
- `cargo test -p slicer-runtime --test integration` — 374 passed, 1 failed:
  `perimeter_spatial_capture_requires_test_support`, the documented
  feature sentinel that panics by design without
  `perimeter-spatial-test-support` (same pre-existing failure t58
  disclosed). An intervening e2e run initially showed 57 failures — all one
  root cause, the `pnp-cli-locator` stale-debug-binary artifact
  (`target/debug/pnp_cli.exe` was older than `crates/*/src` after the probe
  removals); rebuilt with `cargo build -p pnp-cli` and re-ran.
- `cargo test -p slicer-runtime --test e2e` — 147/147 (after the debug
  binary rebuild).
- `cargo xtask build-guests --check` and `--accelerated --check` exit 0
  before the probe removal; guests rebuilt from the probed and candidate
  trees at each capture/build step.

## Qualification (16 cells)

## Qualification (16 cells)

**PASS 16/16 (2026-10-13, snapshot binaries, before the campaign).** Fresh
baseline and candidate snapshot builds for both modes sliced all 16 cells
({benchy, base} x {classic, arachne} x {supports off/on}); every cell was
byte-identical between the arms, every ordinary-cell sha256 equals the
preserved production references
(`evidence/t58-finalization-copies/qualification.json`), and accelerated
benchy cells are hash-identical to their ordinary counterparts
(mode-invariant output, per the same preserved references). Full per-cell
record: `qualification.json` in this directory (produced by
`build-qualification.py`; `problems: []`).

All 16 cells: `status: ok`, `degraded: false`, 0 fatal, 0 non-fatal, and
non-vacuous per-TYPE counts (e.g. benchy arachne-off: Outer wall 692,
Sparse infill 225; base arachne-on: Support 431, Support interface 70 — the
supports-on cells reproduce their known counts).

The eight distinct sha256 values (each identical across both modes, and to
the preserved t58 references):

| cell (both modes) | sha256 |
| --- | --- |
| benchy classic off | `34886ccb56b214d57c28aa58bdcc8d371a41f440200f5b49276b96b53e18cc40` |
| benchy classic on | `76eb4e1df3a47abf9a973be4ac99cd6bac41efa9fb65240786251ec0f7cdf6a8` |
| benchy arachne off | `b76d846acdb13f3255c515878a43082da160c01a4155ae6904d2d473c30e1ba1` |
| benchy arachne on | `ae9d90612bf87ae9f76cb6faf341482e1abbaf5897bb2b981c3247d1646c3b9f` |
| base classic off | `20e7204cfdbb813f642e7e1669d5ba1e30ed1d0ff7f90635af95d967196975ac` |
| base classic on | `f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319` |
| base arachne off | `3ba164191a22f0f10f1cc62f038a07c632d3cea31b68aae11d7aeba820431326` |
| base arachne on | `a1bc81107fc5475a81324b54bda4275d5e963fca6cb155f09f9a227fadcff4f7` |

Snapshot provenance (sha256 of the four snapshot exes,
`.local-artifacts/perimeter-reference-preparation/t65-linker-batch/snapshots/`):
ordinary baseline `8390c8b8…`, accelerated baseline `e1ea2d98…`,
ordinary candidate `26086786…`, accelerated candidate `0ec1a220…`.

Note: the wall/cpu columns inside `qualification.json` are proof-phase
samples taken while other work ran on the box; they are load-noisy and
non-evidentiary. Only the sha256 equality, output cleanliness, and TYPE
counts above are evidentiary from this gate — timing comes from the
dedicated campaign below.

## Paired A/B (timing)

## Paired A/B (timing)

**Verdict: GATE NOT MET (pre-declared both-clauses/both-modes rule).**
Detached campaign 2026-10-13 16:22:22 → 21:01:13 (4h39m, measured from the
driver's START/END lines): all 8 pre-declared cells ran to completion
(exit=0 each), no timeout, no output-drift throw, no external interruption;
14 samples per cell (2 warmups + 6 alternating pairs per arm), every sample
clean (`status ok`, `degraded false`, 0/0 errors), every sha equal to the
qualification reference; **6/6 pairs retained in every cell — zero
starvation exclusions** (cpu/wall medians e.g. 3.3–6.9, no sample below
0.5× its arm's median). The one ` WARN ` line per sample
(`machine-gcode-emit: ERR_MALFORMED_LAYER_MARKER (code 12)`) is identical
on both arms, both modes, and identical to the t58 baseline campaign — a
pre-existing production characteristic, not a candidate artifact.

Reduction (`reduce_ab.py` — paired per-pair ratio of medians,
`ab-reduction.json`; raw rows under
`.local-artifacts/perimeter-reference-preparation/t65-linker-batch/measure/<cell>/rows.csv`):

| cell | wall Δ% | cpu Δ% | pairs | clauses |
| --- | ---: | ---: | ---: | --- |
| ordinary-benchy-arachne-off | **−20.01** | −4.69 | 6/6 | pass/pass |
| ordinary-benchy-arachne-on | **−12.24** | −2.97 | 6/6 | pass/pass |
| ordinary-base-arachne-off | −0.07 | −0.49 | 6/6 | pass/pass |
| ordinary-base-arachne-on | **+0.43** | −0.33 | 6/6 | **fail**/pass |
| accelerated-benchy-arachne-off | **−21.08** | −4.77 | 6/6 | pass/pass |
| accelerated-benchy-arachne-on | **−10.84** | −3.83 | 6/6 | pass/pass |
| accelerated-base-arachne-off | −0.52 | −0.69 | 6/6 | pass/pass |
| accelerated-base-arachne-on | −1.65 | −0.76 | 6/6 | pass/pass |

Wall medians (baseline → candidate): benchy-arachne-off 14.0 → 11.1 s
(ordinary) / 14.8 → 12.1 s (accelerated); benchy-arachne-on 23.0 → 20.3 s /
24.5 → 21.9 s; base-arachne-off 140.3 → 140.7 s / 153.6 → 152.7 s;
base-arachne-on 381.7 → 381.9 s / 429.6 → 424.8 s.

The failing cell in detail: ordinary-base-arachne-on paired wall ratio
1.00426 (+0.43 %, CPU corroborating −0.33 %) with heavily overlapping
ranges (baseline 379.4–433.4 s, candidate 380.6–421.7 s) — a wash, but a
median loss, and the pre-declared gate requires a median wall win in every
timed cell and both modes.

Measured decomposition (all numbers above measured): the batch executes
the ticket's promise exactly where the linker dominates slice wall —
benchy arachne, both modes, both support settings, wall −10.8…−21.1 % —
while on the base fixtures the effect is not measurable through the wall
(−1.65 % accelerated, −0.07/+0.43 % ordinary, inside the job's own
spread) despite the measured clip-fuel cut (−43.6 %, oracle section). The
interpretation (not a measurement): on base, the linker is a far smaller
share of a ~10–27× longer slice, and the same fuel cut is below the
wall-transfer threshold there.

## Restoration

## Restoration

- Standing accelerated snapshot `target/dist-accelerated/developer/` is
  **byte-identical to the pre-campaign archive**
  `target/t65/archive-dist-accelerated-dev/` (whole-tree sha256 diff, 50
  files, zero differences). `target/t65/build-snapshots.sh` re-restored it
  after each arm's build (lines 33-35 / 45-46).
- Both freshness gates measured after campaign end: `cargo xtask
  build-guests --check` exit 0 and `cargo xtask build-guests --accelerated
  --check` exit 0 — green **against the current tree, which still carries
  the uncommitted candidate diff** (`crates/slicer-core/src/polygon_ops.rs`,
  `modules/core-modules/infill-linker/src/orchestrate.rs`; probe fully
  removed, grep-verified). Nothing is committed or merged until the human
  rules on the recommendation.
- The ordinary standing dist path (`target/dist/developer`) was consumed by
  the snapshot dance (baseline-arm build → snapshot copy → candidate-arm
  build; the candidate-built ordinary dist is parked outside the standing
  path at `target/t65/candidate-dist-ordinary/`, mirroring t58's pattern).
  The standing ordinary dist is regenerable from the current tree at any
  time with `cargo xtask dist --edition developer`; no archive of it exists
  or is needed (the four campaign snapshots, kept under
  `.local-artifacts/perimeter-reference-preparation/t65-linker-batch/snapshots/`,
  are the durable both-arm binaries).
- Caveat tied to the pending ruling: if the human rules DROP, the candidate
  source diff must be retired to HEAD (`git diff` preserved as
  `probe-candidate-combined.patch` covers the candidate; tests file is
  untracked) and the guests + debug binaries rebuilt before the freshness
  checks mean "restored to HEAD". If the human rules KEEP, the current
  artifact state is already the production state.