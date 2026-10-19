# Clip-universe preparation hoist — KEEP recommended

Route ticket: [Clip-universe preparation hoist](../../issues/44-clip-universe-hoist.md).

**KEEP for this output-preserving optimization experiment.** On the tested
supports-off Benchy Arachne job, median uninstrumented wall decreases in both
modes, corroborated by lower process CPU. All six pairs in each mode favor the
candidate in both metrics; ranges also strictly separate. No default
mode switch is performed. This is not accelerated-adoption acceptance or a
PNP-vs-Orca scoreboard result.

## Change and exactness

`PreparedPolylineClip` (`crates/slicer-core/src/polygon_ops.rs`) owns immutable
flattened, 1-unit-inflated clip paths. Its constructor preserves the original
contour/hole order, winding, and every `inflate_paths_64` parameter. Each `clip`
call creates a fresh `Clipper64`; subjects are **not batched across owners**.
`clip_polylines` keeps the one-shot signature and early-outs for its other
callers.

`link_paths_without_offset`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) lazily creates one
prepared universe at the first valid path. Offset and raw-boundary fallback
invocations each get their own universe. Invalid paths, short-path filtering,
metadata conversion, owner tracking, connectivity and fallback decisions retain
their existing semantics. No WIT, IR or manifest contract changes.

The regression fixture was recorded **before production extraction**, using
the original one-shot function. It pins raw and inflated integer path arrays
and per-call outputs for square, hole+island, concave+degenerate, dense-ring and
empty universes. Subject cases include all contour edges, a hole edge, interior,
crossing and outside paths, empty/single-point inputs and multiple subjects.

- `prepared_polyline_clip_universe_matches_pre_hoist_reference`
  (`crates/slicer-core/src/polygon_ops.rs`) compares both flatten input and every
  inflated vertex against that captured reference.
- `prepared_clip_repeated_calls_match_pre_hoist_one_shot_outputs`
  (`crates/slicer-core/tests/polygon_ops_tdd.rs`) checks each entry point against
  recorded outputs, revisiting calls in reverse order on the same prepared
  object. It does **not** use the refactored one-shot function as its oracle.

These are representation/reuse pins, not canonical geometry-parity claims.
The pre-hoist recorder is preserved as [`record-pre-hoist.rs`](record-pre-hoist.rs).
It was run as a temporary `slicer-core` example before editing production code;
re-recording on the candidate is not independent baseline evidence.

## Frozen-reference drift and human-approved gate amendment

The external-only recon commands use both `--no-default-module-paths` and
`--no-integrated-modules`, with 12 threads and identical model/config inputs:

| control | output SHA-256 | bytes | existing open-loop warnings |
| --- | --- | ---: | ---: |
| frozen ordinary snapshot | `7049a06d68ee8ab012adc8f26c831bd8cc1a59d0d10d737d1257f58a8719c161` | 3,821,661 | 975 |
| fresh HEAD ordinary baseline | `1b71f83db378d33c2c52ee84028a7180b8b65d26edffde7eff0b06b40b2d403b` | 3,806,297 | 1,110 |

The frozen snapshot reproduces the reference byte-for-byte. HEAD drift persists
with exact isolation flags and clean ordinary freshness; it includes moves and
TYPE counts, not just comments. For example, Outer-wall TYPE counts are 703 vs
698, Inner-wall 754 vs 744, Sparse-infill 124 vs 139. Its cause is **not
attributed** by this experiment.

The human approved **candidate == fresh HEAD baseline**, byte-identical, as
this take's isolated equivalence gate, disclosing the old-reference difference
separately. The frozen corpus/reference is neither overwritten nor re-frozen.
Both mode-specific proof pairs passed before any measurement batch began.

The existing emitter warnings explicitly report unclosed loop entities and
missing closing edges. They remain unchanged baseline vs candidate (1,110 in
every proof, warmup and measured slice). `degraded=false` is not claimed to
prove their absence. Consequently this experiment demonstrates unchanged job
output and a local speed improvement, **not correctness or matched-work
acceptance against Orca**. Follow-up:
[Frozen-job output drift boundary](../../issues/46-frozen-job-output-drift-boundary.md).

## Standing paired A/B

Settings are the frozen attribution job, preserved in
[`benchy-arachne.json`](benchy-arachne.json): 0.5 mm nozzle, 0.2 mm layers,
3 walls, 25% sparse infill, Arachne, supports off. They are **not** the map's
separate 0.4 mm / 2-wall / 20% matched-Orca configuration.

[`run-ab.ps1`](run-ab.ps1) uses the ticket-37 measurement lineage:
`GetProcessTimes` creation-to-exit wall and kernel+user CPU. Every slice is
external-only, probe-free and uninstrumented. Baseline snapshots were archived
before production edits; candidate snapshots copy their matching baseline and
replace **only** `modules/infill-linker/infill-linker.wasm`. Executables, other
components and manifests are byte-identical within each pair. Full before/after
snapshot inventories are reduced into [`evidence.json`](evidence.json).

Six measured pairs per mode, one excluded warmup per arm, 12 threads. Arm order
alternates AB/BA by pair, ordinary batch followed by accelerated. Starvation
policy was declared before measurement in [`protocol.json`](protocol.json):
below 0.75 × best CPU/wall **within the same mode and arm**, exclude the whole
pair; require at least five retained pairs, no automatic resampling. Separate
arm references avoid mistaking a changed serial/parallel work mix for starvation.
No pair was excluded.

| mode | baseline median wall | candidate median wall | wall reduction | baseline median CPU | candidate median CPU | CPU reduction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ordinary | 18.635 s | 14.230 s | **23.6%** | 74.578 s | 68.602 s | **8.0%** |
| accelerated | 19.383 s | 14.017 s | **27.7%** | 72.148 s | 66.266 s | **8.2%** |

Paired median candidate-minus-baseline deltas: ordinary wall −5.206 s / CPU
−5.938 s; accelerated wall −5.448 s / CPU −5.852 s. Both metrics favor the
candidate in 6/6 pairs in each mode, with strict range separation in both.
Paired-delta medians and differences of arm medians are distinct statistics.

Measured per-sample CPU/wall ranges: ordinary baseline 3.752–4.034, candidate
4.785–5.121; accelerated baseline 3.627–3.817, candidate 4.539–4.925. Every
individual sample and ratio, including excluded warmups, is in `evidence.json`.

### Output disclosure

Every proof, warmup and measured output in both arms/modes:

- SHA-256 `1b71f83db378d33c2c52ee84028a7180b8b65d26edffde7eff0b06b40b2d403b`;
  **3,806,297 bytes**, byte-identical to the proved fresh baseline.
- Validated Arachne marker; `status=ok`, `degraded=false`, zero fatal/non-fatal
  errors; zero integrated-shadow warnings and no instrumentation events/probes.
- TYPE counts: Skirt 1, Brim 1, Outer wall 698, Inner wall 744, Bottom surface
  23, Bridge 18, Sparse infill 139, Internal solid infill 6, Top surface 32,
  Internal Bridge 12.
- Existing open-loop warnings: 1,110, unchanged (see limitation above).

## Validation and reproduction

Passed in this session:

| command | result |
| --- | --- |
| `cargo test -p slicer-core --features host-algos --test polygon_ops_tdd` | 14 passed |
| `cargo test -p slicer-core --features host-algos --lib prepared_polyline_clip_universe_matches_pre_hoist_reference` | 1 passed |
| `cargo test -p infill-linker` | 39 integration tests passed; no feature-gated files absent |
| `cargo build --workspace` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; dependency future-incompatibility advisory only |
| `cargo xtask check-literals` | no violations |
| `cargo xtask check-test-quality --report crates/slicer-core/src/polygon_ops.rs crates/slicer-core/tests/polygon_ops_tdd.rs` | no findings |
| `cargo xtask build-guests --check` and accelerated counterpart | both exit 0 after mode-specific rebuilds |

All test invocations tee'd to `target/test-output.log`; results were read before
the next invocation overwrote the log. An initial compile error in the private
helper's slice argument was fixed to pass the owned vector expected by Clipper;
the successful tests above ran afterwards. No assertion or coverage was weakened.
The workspace-wide test suite was **not** run.

Verify headline claims using only tracked files:

```bash
python docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/verify-t44.py
```

The verifier re-derives the keep recommendation, starvation exclusions,
medians/deltas, exactness, output disclosure, interleaving and one-variable
inventory check. It must exit 0 without any raw artifact tree. Reduced inputs
are `evidence.json`; derived outputs are [`summary.json`](summary.json).

Optional full raw captures/snapshots/logs: `target/t44-ab/`. To re-reduce that
existing capture without launching any slices:

```bash
python docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/verify-t44.py --reduce target/t44-ab --write-summary
```

The measurement driver never builds or overwrites a capture directory. Its
proof phase for **both** modes precedes its measurement phase for either mode.
See its parameter contract and `protocol.json`; do not treat recon/proof wall
as acceptance timing, mix modes, or reuse mutable in-tree guests as an arm.

## Scope limits and handoff

Only this frozen-settings supports-off Benchy Arachne job has a real-output and
timing claim. Classic, base, supports-on, alternate thread counts and the
matched-Orca job were **not** measured. No new fuel profile or memory claim.
No output change was observed on the tested job; the frozen-reference drift and
emitter warnings predate the hoist and remain unresolved. Native coverage is
the narrow geometry/linker tests; no full native slice equivalence claim.

**KEEP approved (2026-10-02).** Ordinary remains the default; the adoption
campaign remains inconclusive.
