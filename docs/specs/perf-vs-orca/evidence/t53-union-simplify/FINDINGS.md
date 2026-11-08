# T53 — current-job union/simplify attribution

## Outcome

**The four diagnostic cells qualified. Simplification never executed. Native
body union dominates the measured chain. Park the simplification / four-role
batching call-reduction lead: this evidence identifies no expensive removable
call.** No candidate, speed verdict, commit or default change was made.
Production source and the inventoried active artifacts are restored exactly;
ordinary and accelerated restoration freshness checks both exited **0**.

This is fresh diagnostic evidence, not ticket 22's historical combined bucket.
One baseline and one probe slice ran per fixture/mode; no retries or substitutes.

## Current job and qualification

Both modes used freshly built external **Developer** snapshots, the actual
`tmp/3dbenchy.stl` / `tmp/base.stl`, and
`docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json`.
`identity.json` records full input hashes, compiler, source hashes, build HEAD,
baseline snapshot byte inventories and external-module diagnoses.
`ordinary-probe-inventory.json` / `accelerated-probe-inventory.json` record probe
artifacts. No integrated/default-module fallback was permitted; `SLICER_MODULE_PATH`
was removed from child environments. `RAYON_NUM_THREADS=12` was supplied.

| Job, both modes | Baseline = probe SHA-256 | Bytes | Support TYPEs | Support-interface TYPEs | Actual Support extruding XY moves | Interface extruding XY moves | Fan commands |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Benchy | `7e345b1a51f8ca50def4ea24e39bc74e333640e3ef9b78e6e33f73debccf80cb` | 8,658,776 | 179 | 4 | 101,304 | 274 | 284 |
| base | `7a4d7f0f081c331b3fe5289dec5286dd093ec68a5ebd547325aff6987e336e2e` | 43,912,905 | 431 | 70 | 505,179 | 3,536 | 628 |

Every baseline/probe: exit 0, exactly one `slice_complete`, `status=ok`,
`degraded=false`, fatal=0, non-fatal=0, `; wall_generator = classic`.
Complete TYPE histograms and completion events are in the eight
`<mode>-<job>-<arm>-output.json` files. Nonvacuity counts require actual positive
relative-E XY motion under the verified `M83` mode, not merely TYPE comments.
`byte-comparison.json` additionally records streamed **actual byte comparisons**,
not just hash agreement. These are fresh fan-bearing, post-repair references.

## Measured split

Milliseconds from the qualified probe runs. These are **instrumented serial
wall brackets**, never process CPU or a speed comparison between modes.

| Cell | Native union core (exclusive) | Host input/output conversions (exclusive) | SDK/WIT round-trip + diagnostic residual (exclusive combined bucket) | Union calls, guest bracket (inclusive; do not add again) | Active guest simplify |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary Benchy | 2,427.7426 | 6.1556 | 44.3378 | 2,478.2360 | 0 |
| accelerated Benchy | 1,816.1515 | 4.2218 | 26.8047 | 1,847.1780 | 0 |
| ordinary base | 74,648.8767 | 43.0839 | 280.5694 | 74,972.5300 | 0 |
| accelerated base | 77,705.3785 | 49.3764 | 264.8591 | 78,019.6140 | 0 |

The three exclusive columns sum to the inclusive union-call column. Empty /
singleton bypass wall is separate: ordinary/accelerated Benchy **0.125/0.117 ms**;
ordinary/accelerated base **0.431/0.394 ms**. The inactive simplify branch/clock
bracket measures **0.230/0.158 ms** on Benchy and **0.484/2.565 ms** on base;
none is simplification execution and none supports a simplification-savings claim.

Separate surrounding sibling brackets:

| Cell | Structural regions | Existing carve | Final collision Difference | Body-minus-interface differences |
| --- | ---: | ---: | ---: | ---: |
| ordinary Benchy | 10.144 | 2,717.638 | 297.777 | 1.845 |
| accelerated Benchy | 6.049 | 1,726.705 | 197.967 | 1.387 |
| ordinary base | 75.004 | 68,942.362 | 2,406.589 | 132.794 |
| accelerated base | 76.276 | 68,766.829 | 2,475.980 | 138.154 |

### Boundaries and correlation

- Guest clocks use `host::now_us()` around sibling operations inside
  `build_roles::with_areas`
  (`modules/core-modules/tree-support-planner/src/lib.rs`). Shapes and selected
  geometry serialization/logging occur outside those brackets.
- Only multi-polygon `union_expolys` calls are tagged immediately before the
  import. `HostExecutionContext::clip_polygons`
  (`crates/slicer-wasm-host/src/host.rs`) recognizes that context's immediately
  preceding tag, emits its matching ID, and thereby excludes unions from other
  planner call sites. Exactly one matching host record exists per real union.
- Host `Instant` brackets separate WIT→IR allocation/conversion, the
  `slicer_core::polygon_ops::clip_polygons` invocation (including the operation
  enum mapping), and IR→WIT conversion. `full_ns` is their sum. It starts after
  correlation lookup and ends before diagnostic log formatting, local-vector
  destruction and return/lowering.
- The guest union bracket additionally includes SDK conversion, WIT
  lifting/lowering, import transitions, vector destruction, tagged host-log
  formatting and clock/probe overhead. **Subtracting host full gives this
  combined residual, not pure transport overhead.** No reliable further split
  was attempted. Microsecond timestamp quantization permits an aggregate error
  below one microsecond per measured guest union call: below 0.181 ms on
  Benchy / 0.472 ms on base. Clock/import observer overhead is not eliminated.
- The planner is one serial dispatch per slice. No accumulated worker spans
  are called CPU. Host native boolean execution is not guest fuel; this take
  did not enable fuel metering. Whole emit-pass occupancy inflation, node
  generation and other scaffolding were **not** re-attributed; the surrounding
  table covers only the listed chain siblings, not the entire emit pass.

## Counts, shape and exact representation

Role order is body/roof/base/floor. Counters and all shapes agree across modes.

| Job | Role/layer rows | Real union calls | Empty bypass | Singleton bypass | Simplify activations | avg_node_per_layer |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Benchy | 716 (179 layers × 4) | 181 | 533 | 2 | 0 | 5 |
| base | 1,724 (431 layers × 4) | 472 | 1,220 | 32 | 0 | 9 |

Real calls by body/roof/base/floor: Benchy **177/2/0/2**; base **430/38/0/4**.
The base-interface role is empty throughout these jobs. The body owns almost
all measured native union cost; exact per-role quantities are in `evidence.json`.
No hypothetical four-nonempty-request batching assumption is justified.

Body pre-union totals (summed real representations, not unique geometric area):
Benchy **21,680 polygons / 2,154,791 vertices**, base **147,779 polygons /
14,629,069 vertices**, with no input holes. Their union results contain,
respectively, **831 polygons / 18 holes / 356,649 vertices** and **2,261 polygons /
510 holes / 1,302,435 vertices**. Maximum per-body input is **182 polygons /
18,200 vertices** on Benchy and **1,024 polygons / 102,400 vertices** on base.
These are substantial real merging/normalization requests, not empty calls.

Each `<mode>-<job>-records.jsonl` preserves compact per-role/per-layer shapes
(polygons, holes, vertices, largest ring), timings, request IDs, object/region
identity, and selected **full integer ordered** pre-union / post-union /
pre-simplify / post-simplify / post-collision-difference captures. `RETURN`
records preserve selected actual role results after body-minus-interface cuts.
Sampling selected the first nonempty singleton and first multi-polygon chain
per role when present: **4 Benchy + 6 base specimens**. Both modes' selected
full captures and returned roles agree exactly, not just by area.

`replay.rs` and `replay.txt` replay these ten specimens against native current
`union_expolys`, conditional `expolygons_simplify`, and the final collision
difference, asserting exact polygon/hole/vertex order. The executed native
test passed. This is an unmodified-chain replay basis, **not a candidate oracle
that has already certified an optimization**.

Representation hazards remain: the current union merges polygons and creates
holes; `simplify_ring` anchors Douglas–Peucker at `points[0]`; the existing
carve winding guard and active-simplify lone-region repair remain untouched.
Future changes must retain integer coordinates, order, role separation,
tolerance/activation and subsequent differences, not merely filled-area equality.

## Coverage gaps and recommendation

- Positive simplify activation is absent; do not speculate savings or retry
  with another fixture. The existing coarse-density tests, rather than these
  real jobs, can falsify anchor-sensitive simplification defects.
- Selected full captures do **not** cover the largest unions or hole-bearing
  union outputs. Shape census observes **16 Benchy / 158 base** hole-bearing
  requests, but selected exact captures have zero output holes (`coverage.json`).
- Native replay stops at each role's collision difference. Selected returned
  role arrays are recorded, but the full four-role body-subtraction chain is
  not independently replayed for every layer. Final G-code identity qualifies
  these probes, not all future intermediate transformations.
- Single diagnostic observations with mode-to-mode wall variation are not
  acceptance timings. No optimization benefit or impact is measured.

**Park this call-reduction lead.** Removing simplify calls is irrelevant here;
batching roof/base/floor requests cannot remove the dominant body boolean work.
No expensive union was shown to be unnecessary. If the human selects a new
lead, scope **large body-union input/algorithm representation analysis**, first
adding exact hole-bearing and largest-request replay coverage. That is a new
diagnostic/candidate scope, not implementation authorization from this take.

Existing falsifiers for a later proposal include
`t32_gate_chain_gate_matches_unconditional`,
`t32_gate_coarse_single_disc_is_the_unsafe_case`,
`build_roles_merges_overlapping_node_cross_sections_into_one_outline` and the
coarse-density simplification tests in
`modules/core-modules/tree-support-planner/src/lib.rs`. The structural
`integrated_parity_support_planner_native_matches_wasm`
(`crates/slicer-runtime/tests/contract/integrated_parity_support_planner_tdd.rs`)
is not a substitute for ordered intermediate and matched-job output oracles.

## Validation and restoration

Executed commands/results are recorded in `validation.json` and adjacent logs:

- Both initial, fresh-baseline, probe and restored mode-specific
  `cargo xtask build-guests [--accelerated] --check`: **exit 0 independently**.
- `cargo xtask dist --edition developer [--accelerated]` for baseline and probe:
  **exit 0**. Four `module diagnose` calls: **exit 0, external provenance**.
- Eight actual slices: **exit 0**, clean output/status; four streamed byte
  comparisons pass.
- Narrow native replay via
  `cargo test -p tree-support-planner --lib -- t53_current_chain_native_replays_exact_ordered_wasm_intermediates --nocapture`:
  **1 passed**. Combined output was tee'd to `target/test-output.log`, then read
  and preserved as `native-replay.log`. This native-only test uses no guest
  artifacts. Its build emitted a source-mtime stale-test-guest advisory;
  both artifact-verified freshness gates nevertheless exited 0, including after
  restoration. No workspace tests ran.
- `python docs/specs/perf-vs-orca/evidence/t53-union-simplify/verify.py --falsify`:
  **PASS without raw captures**; wrong timing totals and wrong output identities
  are deliberately rejected. `reduce.py --reduce <raw-root> --falsify
  --replay-fixture` regenerates reductions from streamed logs.
- Re-applicable `probe.patch` and `replay-hook.patch`: `git apply --check`
  **exit 0** against restored production sources. `git diff --check`: **exit 0**.

Exact pre-probe sources/artifact inventories were saved before any build.
Restoration copied only this take's saved original sources, rebuilt guests
against restored inputs, then restored the original inventoried artifact bytes.
`restoration.json` confirms source hashes, empty production-source diff, all
inventoried artifact bytes, complete active distribution file sets, and both
freshness exits. Existing tracker edits and prior untracked evidence were not
edited or removed. Only this evidence directory is new working-tree work.

Raw snapshots/logs/outputs/originals remain optional at the existing gitignored
`.local-artifacts/perimeter-reference-preparation/t53-union-simplify/`.
They were initially created at `.local-artifacts/t53-union-simplify/`, then
moved intact into the repository's actually ignored namespace after inspection
of `.gitignore`; no payload was discarded. Heavy models were not copied into
tracked evidence. Active production artifacts are restored; isolated probe
snapshots remain explicitly diagnostic. No known production side effects remain.
