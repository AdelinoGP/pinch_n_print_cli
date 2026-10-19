# Support-correctness repair, DEV-174 class

Type: task
Status: resolved

## Question

Repair the DEV-174-class support defect so the supports-on cells of the matrix
compare honest work: `com.core.tree-support-planner` drops ~60 contiguous
support layers mid-print on base.stl behind 29,108 code-1200 routing
rejections (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §8 — its "DEV-167" label is stale; the
ledger row is **DEV-174** in `docs/DEVIATION_LOG.md`).

Work:

- Diagnose the code-1200 routing rejection path (the `diagnosing-bugs`
  discipline: verify root cause before editing).
- Fix without changing intended geometry elsewhere; the degraded flag must
  clear and the non-fatal error count must go to zero on base.stl supports-on
  runs.
- Correctness acceptance with the human (this is route work only because the
  full 8-cell matrix decides and the fairness contract disqualifies cells whose
  work is skipped — a degraded slice is not Orca's job). Performance
  non-regression is checked at stage level per the recipe, not by whole-slice
  single runs.

Until this lands, every scoreboard revision must disclose the degraded
supports-on condition (Q5's disqualify rule marks those four cells tainted
rather than won or lost).

Measured context (ticket 11 scoreboard, 2026-09-22, matched job): every
base.stl supports-on run in both PNP modes reports `degraded=true` with
**172,181** non-fatal errors (29,108 at the old 3-wall/0.5 mm config), and PNP
emits only 280 `Support` / 55 `Support interface` sections against Orca's
638–640 / 224–225 — the dropped-layer work-skipping is visible in the output
section counts themselves.

## Findings — 2026-09-23 (resolution: kept on human acceptance)

Evidence pack: [`../evidence/dev174-repair/`](../evidence/dev174-repair/)
(probe log, matched-pair rows, stage table, visual bundles). Raw heavy
artifacts stay regenerable under `target/` per its reproduction commands.

**Root cause (verified by probe on real data before any edit).** All 172,181
code-1200 diagnostics carry the reason `body rejected: max-body-extent
violation` from `in_routing_cell` (`crates/slicer-wasm-host/src/support_aggregation.rs`).
A tagged probe of the rejection path showed 151 rejected entries covering
layers 108–258 of `tmp/base.stl` — one entry per `(layer, region 0, object)`
carrying 812–1,438 demands each (the DEV-174 contiguous band). Two
measurement mechanisms fire against `MAX_BODY_EXTENT_UNITS = 1 << 20`
(104.86 mm): multi-body identity aggregates whose combined envelope reaches
1,113,301 units while no single region exceeds 901,608, and single fused body
cross-sections up to 1,111,286 units (111.13 mm) — legitimately printable
geometry marginally over a cap that is the **deleted routing cell's size**
(ADR-0059 Ruling 2 kept the constant after deleting the grid). The producer
contract (`SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`),
enforced by `Blackboard::commit_support_plan` (`crates/slicer-runtime/src/blackboard.rs`))
admits exactly one entry per `(global_layer_index, object_id, region_id)`, so
an entry is an identity aggregate of many bodies; the gate measured that
aggregate as one body ("a single body's own envelope" per its own doc), which
no monotone per-entry bound can satisfy on wide layers. Canonical imposes no
body-size cap at all (the host's own `union_same_family_entries` comment
concedes it), so this was PnP-only work-skipping.

**Fix (landed).** ADR-0059 Ruling 3 / ledger row
D-287-ADR-0059-AMENDED: `in_routing_cell` now measures **each body
cross-section** (role region) against `MAX_BODY_EXTENT_UNITS`, and the
constant is recalibrated `1 << 20` → `1 << 22` units (419.43 mm — larger than
the BBL X1C 256 mm bed diagonal, i.e. "a single body must fit the build
plate"). Both are required: per-body measurement alone still rejects the
111.13 mm fused cross-sections, and no cap calibration alone can keep the
pinned oversized-body fixtures rejecting while accepting 111.13 mm bodies.
Five size-contract fixtures recalibrate their cap-relative literals
(`spans_cell` in `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`,
`OVERSIZE` in `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
and `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`,
and the `same_family_union` / `invalid_body_degraded` oversized fixtures in
`crates/slicer-runtime/tests/integration/support_family_routing.rs` — the
latter two are absolute-literal oversize bodies, one of which rejects via
message-id assertions and is easy to miss); every assertion is unchanged.
`body_bounds` is deleted; the exact-Z occupancy semantics are untouched (zero
occupancy rejections in the failure data).

**Regression tests (watched RED, then GREEN).**
`identity_aggregate_spread_across_the_plate_is_measured_per_body` and
`support_body_wider_than_the_deleted_routing_cell_is_retained`
(`crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`) encode
the measured base.stl shapes; both failed pre-fix with `body rejected:
max-body-extent violation` and pass post-fix.

**Before/after evidence (base.stl, supports-on).**

| Signal | Before (dev174-before / ticket 11) | After (dev174-after) |
| --- | --- | --- |
| `slice_complete` | degraded=true, non_fatal=172,181 | degraded=false, non_fatal=0 (all 4 PNP rows) |
| Support sections (classic/arachne) | 280 | 431 (both modes; Orca 638–639) |
| Support interface sections | 55 | 70 (both modes; Orca 223–225) |
| Layers 108–258 with zero Support sections | 151 | 0 |
| All layers with zero Support sections | 215 / 495 | 64 / 495 |
| Every other TYPE counter | — | bit-identical (Inner wall 525/527 → 523/526, the documented jitter class) |

**Stage-level performance (PERF-HANDOFF §3 discipline).** Instrumented
before/after pair (extracted in [`../evidence/dev174-repair/EVIDENCE.md`](../evidence/dev174-repair/EVIDENCE.md);
both sides instrumented, never mixed into wall claims): prepass 349.5 s → 455.7 s,
per_layer 107.9 s → 152.8 s, postpass 25.1 s → 52.2 s. The growth concentrates
in support-emitting work (`com.core.tree-support` summed-worker 66.4 s →
143.8 s rendering 151 restored layers) while unrelated stages moved in the
same direction by the same order (`com.core.classic-perimeters` 1,105 s →
1,513 s summed-worker, +37% — walls cannot be affected by a support gate), the
§10.3 external-load signature. The fix adds no cost class anywhere (the
per-region bbox scan is the same O(regions) as the old union scan); the
restored work is the ticket's intended output, not a regression. No whole-slice
wall claims are made: the after rows' walls (792/806 s classic-on, 467/474 s
arachne-on) are single runs under the observed session drift and the ±13%
base.stl wall spread (§3.6) — a medians-of-N study is the right instrument if
a tight wall verdict is wanted.

**Gates.** `cargo xtask build-guests --check` and `--accelerated --check` exit 0;
slicer-wasm-host 216 tests, slicer-runtime `support_family*` 12 tests,
tree-support-planner guest 137 tests, traditional-support-planner guest 45
tests all green (narrow runs tee'd to `target/test-output.log`);
`cargo clippy --workspace --all-targets -- -D warnings` clean; `check-literals`
0 violations; `check-test-quality --report` 12 findings, none in touched code;
`check-deviations` regenerated + green (doc 07 map, 70 open); touched regions
rustfmt-clean (pre-existing diffs elsewhere left untouched).

**Visual verification (visual-debug, `final_gcode` tap, `filled_areas`).**
Before/after G-code rendered at the same band layers (z = 30.2 mm / 46.2 mm;
bundles [`../evidence/dev174-repair/visual/`](../evidence/dev174-repair/visual/)
before/after,
entries `final_gcode_filled_areas_l150`/`l230` with `layer_z` 30.2/46.2 and
warnings limited to `M73` time lines): the after renders add exactly the
support classes — a packed column field hugging the flared-hull perimeter and
the deck-overhang footprint, isolated tree columns in the interior pockets,
and interface bands under the deck — standing entirely in cross-section space
the before render shows empty (no overlap with model walls/infill, which are
pixel-identical between the pairs, matching the bit-identical model TYPE
counters). Palette caveat: `gcode_role_color` (`crates/slicer-runtime/src/visual_debug_style.rs`)
FNV-hashes roles into 6 colors, and `Support` collides with `Outer wall`
(rgb 220,50,47) — wall/support disambiguation rests on the before/after
differential, not on color. G-code-source silhouette is documented in
`docs/19_visual_debug.md` but this build rejects it (`silhouette is not
supported for tap 'final_gcode'`) — vertical continuity is covered by the
per-layer section scan instead.

**Not claimed.** No Orca section-count parity
(431/70 vs 638–639/223–225 remains a disclosure item under the fairness
contract — section counts are tool-format-dependent); no wall verdict — the
whole-slice numbers are single runs under session drift and the ±13% base.stl
spread (§3.6), so a medians-of-N study is the follow-up if one is wanted.
