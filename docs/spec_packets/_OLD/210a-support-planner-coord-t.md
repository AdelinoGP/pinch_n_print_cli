---
status: superseded
packet: 210a-support-planner-coord-t
task_ids:
  - TASK-326
superseded_by: 221-tree-support-family (DEV-128 integer-coordinate work absorbed into the tree planner rewrite) + 220-support-analysis-family-contracts (structural SupportPlanIR migration)
---

# 210a-support-planner-coord-t

## Goal

Migrate `support-planner`'s branch-node geometry from `f32` millimetres to scaled-integer `i64` internal units (DEV-128): `PlannedSupportNode` position, the Prim MST edge weights, the move-pass step cap, the avoidance clamp, the exact polygon-membership test, and the 100-iteration Laplacian smoother — converting to `f32` mm only at the `Point3WithWidth` emission boundary. As part of the one and only rewrite of `smooth_branches`, extract its inlined sub-chain gap walk into `split_column_into_chains` so that packet `210b`'s bottom-interface pass consumes a chain definition that already exists rather than re-deriving one.

**Why the extraction lives here.** `smooth_branches` is the seam where the 2026-08-07 merge of packets 210 and 211 was originally motivated: half A retypes the gap walk to an integer squared-unit comparison, half B needs the same walk as a reusable helper. Both are edits to the same twenty lines. This packet performs that rewrite exactly once (Step 3) and ships the helper; `210b` adds only a second *caller*. Under this ordering the collision cannot recur, and `210b` never reopens `smooth_branches`.

# Requirements: 210a-support-planner-coord-t

## Packet Metadata

- Grouped task IDs: `TASK-326` (net-new; re-derive that the slot is still free at the moment you register it — the highest `TASK-###` in `docs/07_implementation_status.md` moves)
- Paired packet: `210b-support-interface-bottom-layers` carries `TASK-327` and `DEV-129`. It is **not** absorbed here; it depends on this packet being **implemented and merged**.
- Deviations owned: `DEV-128` (closed here). `DEV-129` belongs to `210b`.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Provenance

The history of this slice is non-linear and is recorded here so that a future reader does not mistake the directory names for duplication:

1. Packets `210-support-planner-coord-t` (DEV-128) and `211-support-interface-bottom-layers` (DEV-129) were authored separately.
2. **2026-08-07, user decision:** they were **merged** into one packet, because both rewrote `smooth_branches` (`modules/core-modules/support-planner/src/lib.rs`) and neither planned for the other's edit — 210 retyped its inlined sub-chain gap walk to an integer squared-unit comparison, 211 extracted that same walk into `split_column_into_chains`. `211`'s directory was marked `status: superseded` at that point.
3. **The merged packet was then reviewed and ruled `SIZE: must decompose`.**
4. **2026-08-07, user decision:** the merged packet was **re-split** into `210a` (this packet, DEV-128, the migration plus the extraction) and `210b` (DEV-129, the bottom-interface bands).

`docs/spec_packets/211-support-interface-bottom-layers/` remains `status: superseded` and is neither revived nor deleted — its `superseded_by` still points at the merged `210`, which is the truthful record of what happened to it. Its work now lives in `210b`. Do not implement that directory, do not edit it, do not delete it.

**Why the re-split is safe where the original two-packet arrangement was not.** The collision was always confined to one function. This packet performs the *only* rewrite of `smooth_branches` and ships `split_column_into_chains` as a finished, exported helper; `210b` adds a second **caller** and nothing else. The ordering constraint that makes this work — `210b` starts only after `210a` is merged, and is written against post-migration signatures rather than "whichever signature is on disk" — is the specific defect that made the original packet 211 unmergeable, and it is now an explicit prerequisite in `210b`'s `packet.spec.md`.

**Corrections carried forward from 211's preflight** (they constrain this packet's Step 3, which is why they are repeated here rather than left in the superseded directory):

1. The gap-walk constant is the **fn-local** `const CHAIN_BREAK_THRESHOLD_MM: f32 = 5.0` declared inside `smooth_branches`, not a module-level `CHAIN_BREAK_THRESHOLD`. It becomes `CHAIN_BREAK_THRESHOLD_UNITS: i64 = 50_000`, declared inside `split_column_into_chains`.
2. `smooth_nodes_tdd.rs`'s guard assertions are the four `#[test]` fns `smoothing_reduces_curvature`, `endpoints_held_fixed`, `columns_below_three_points_unchanged` and `empty_entries_no_panic`. 211 pinned them to a line range that in fact contains only the helper fns. Cite the test names.
3. The `e - s < 3` short-chain filter **stays in `smooth_branches`**, and so does the `column.len() < 3` outer guard. `split_column_into_chains` returns *all* sub-chain ranges including short ones, and is callable on a column of any length. Anything else makes the extraction non-behaviour-preserving, and short chains must still receive floor bands in `210b`.

## Problem Statement — DEV-128, `f32` millimetres where canonical carries `coord_t`

`PlannedSupportNode` declares `x: f32, y: f32`; the Prim MST edge weights (`prim_mst`, `euclidean_distance`, `neighbours_of`) are `f32`; the move-pass step cap, `clamp_to_avoidance` and `point_in_any_expoly` all round-trip through `f32` by multiplying a millimetre value by `SCALING_FACTOR as f32` and dividing back; and `smooth_branches` runs a 100-iteration three-point Laplacian in `f32` millimetres, reading and re-writing the emitted `Point3WithWidth` on every iteration.

Canonical's `SupportNode::position` is an Eigen `Point` of `coord_t` and `TreeSupport::smooth_nodes` averages those integers directly with a truncating `/3`. The exposure is accumulated rounding, worst exactly where it matters: `f32`'s precision is *relative*, so a node at 250 mm already quantises to ≈3 × 10⁻⁵ mm per operation. Across 100 smoothing iterations plus the per-layer clamp/move cycle, a branch endpoint can land on the wrong side of a collision outline — the invariant-2 (`branch_endpoints_are_outside_support_collision_outlines`, `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`) failure mode `DEV-128` names as its trigger.

**Measuring the blast radius without contradicting yourself.** `docs/specs/deviation-remediation-206-212-plan.md` quotes "~113 f32 sites". That is a **matching-line** count (`rg -c`). Measured on the current tree: 113 matching lines / **153 total occurrences** file-wide, and 83 matching lines / **114 occurrences** outside the in-file `#[cfg(test)]` module. Both pairs are correct; they measure different things. State which measure you used when you re-derive, or the next reader will conclude the packet is stale.

## In Scope

- Retype `PlannedSupportNode` to `x: i64, y: i64` (internal units, 1 unit = 100 nm). `dist_to_top: u32` and `to_buildplate: bool` unchanged.
- Move the mm→unit boundary to contact creation: overhang-facet centroids and paint-enforcer contacts arrive from `MeshObjectView` in millimetres and are converted once with `mm_to_units` / `Point2::from_mm`.
- Move the unit→mm boundary to emission: every `Point3WithWidth { x, y, .. }` written in `plan_for_object`, `push_interface_scan_lines` and `smooth_branches` is produced by `units_to_mm`.
- Retype `prim_mst` to `Vec<(usize, usize, i64)>` and `euclidean_distance` to `-> i64`, computed as `(dx*dx + dy*dy).isqrt()` (`i64::isqrt`; exact, no float on the path).
- Retype the per-node neighbour table to `neighbours_of: Vec<Vec<(usize, i64)>>`, **with an explicit type annotation on the binding** (mandated so AC-2's static check is meaningful), and the merge-threshold comparison to integer units.
- Retype `aggregate_neighbour_targets` to `pub fn aggregate_neighbour_targets(neighbour_positions: &[Point2], distances_units: &[i64]) -> Option<Point2>`, with the degenerate collapse triggered by `distances_units[j] == 0` instead of an `EPS_MM` float epsilon.
- Retype the move-pass step cap `max_move_xy` to `i64` units derived through `mm_to_units`, and the displacement/cap comparison to integer arithmetic.
- Add `point_in_polygon_units(poly: &[Point2], p: Point2) -> bool` — exact ray cast using `i128` cross products over a **single ring**. Its only in-packet call site is `point_in_any_expoly`, which calls it for the contour ring and then for each hole ring; no other code in this module holds a bare `&[Point2]` ring.
- Retype `point_in_any_expoly` to `(polygons: &[ExPolygon], p: Point2) -> bool` so node units compare against `ExPolygon`'s already-integer points with no cast. **This is a retype, not a rewrite: the hole semantics are preserved exactly.** Today's body is `point_in_polygon(outer) && !ex.holes.iter().any(…)`, i.e. a point inside a hole is *not* inside the `ExPolygon`; the migrated form is `point_in_polygon_units(&ex.contour.points, p) && !ex.holes.iter().any(|h| point_in_polygon_units(&h.points, p))`. Dropping the hole term would let a branch (and, in `210b`, a floor band) be placed inside a model hole. AC-7 cannot catch that loss — it pins signatures and bans `SCALING_FACTOR as f32`, all of which a contour-only body satisfies — so AC-N8 pins it behaviourally. **`210b` consumes this helper**, not `point_in_polygon_units`, because `LayerCollisionCache.collision_polys` is `Vec<ExPolygon>`.
- Retype `clamp_to_avoidance`, `closest_point_on_polygon` and `closest_point_on_segment` to `Point2` / `i64`, using `i128` intermediates for the segment projection ratio.
- Retype `push_interface_scan_lines`' centre, half-extent and spacing parameters to `Point2` / `i64`; `z` and `width` stay `f32` millimetres.
- Retype `first_point_xyw` to `Option<(Point2, f32)>` (position in units via `mm_to_units` of the stored mm; width in mm; no `z`).
- Rewrite the four tests in `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` against the new `aggregate_neighbour_targets` signature, in units, with exact integer assertions replacing the `1e-3` / `1e-9` millimetre tolerances.
- Rewrite the in-file `#[cfg(test)]` case `prim_mst_on_two_nodes_returns_one_edge` against the integer edge weight (`assert_eq!(edges[0].2, 50_000)`).
- Add the in-file `#[cfg(test)]` cases named by AC-4, AC-N1, AC-N2, AC-N7 and AC-N8: `smooth_branches_uses_truncating_integer_average`, `point_in_polygon_units_is_exact_on_contour_vertex`, `node_position_roundtrips_beyond_f32_integer_ceiling`, `mm_unit_round_trip_envelope_is_5_120_003_units`, `point_in_any_expoly_excludes_points_inside_holes`.

### The shared seam — `smooth_branches`, rewritten once (Step 3)

- Extract the sub-chain gap walk into `split_column_into_chains(entries: &[SupportPlanEntry], column: &[usize]) -> Vec<(usize, usize)>` returning **half-open ranges into `column`**, private, with the fn-local `const CHAIN_BREAK_THRESHOLD_UNITS: i64 = 50_000` (5.0 mm × 10 000) and a squared-unit comparison (`dx*dx + dy*dy > CHAIN_BREAK_THRESHOLD_UNITS.pow(2)`), no square root.
- `split_column_into_chains` returns **every** sub-chain range, including ranges shorter than 3. The `e - s < 3` skip and the `column.len() < 3` outer guard stay in `smooth_branches`; `210b`'s bottom pass deliberately does not apply them.
- The `None`-on-malformed-entry `break` in the current walk is preserved verbatim: it terminates the split loop, leaving the remaining indices in the final chain.
- Migrate `smooth_branches`' averaging to integers: read each sub-chain's points once into a scratch `Vec<Point2>`, run `iterations` passes of `(prev + cur + next) / 3` in `i64` (truncating, matching canonical `TreeSupport::smooth_nodes`), and write back through `units_to_mm` exactly once per point after the final pass. Widths keep their `f32` averaging and `MAX_BRANCH_RADIUS_MM` clamp. Public signature `(&mut Vec<SupportPlanEntry>, usize)` is unchanged, so `tests/smooth_nodes_tdd.rs` needs no edit and remains the behaviour guard.

### Frozen-golden fixtures (owned, not incidental)

The migration changes emitted branch geometry, and **two** self-captured golden pairs compare it. Both are in scope for deliberate regeneration:

- `resources/golden/benchy_tree_support_orca_endpoints.txt` + `..._branch_count.txt`, compared by `benchy_orca_parity_within_tolerance` (`modules/core-modules/support-planner/tests/orca_parity_tdd.rs`), regenerated with `SUPPORT_PLANNER_REGEN_GOLDEN=1`.
- `resources/golden/support_regression_wedge_endpoints.txt` + `..._branch_count.txt`, compared by `current_wedge_output_stays_within_self_capture_tolerance` (`crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`), regenerated with `SUPPORT_WEDGE_REGEN_GOLDEN=1`.

Per `CLAUDE.md` §Test Discipline, canonical-correct output wins and fixtures may be re-recorded to match — but **only as the explicit, owned act of Step 4**, with a written justification naming which mechanism moved the output, and that justification must also land in the `DEV-128` closure row, because **AC-8 clause (c) greps the deviation log for the regenerated file's basename**. A silent regeneration is now a failing criterion, not an invisible one. The tolerance constants (`let tolerance_mm = 0.5_f32;`, `let tolerance_fraction = 0.10_f32;`, and the `0.10, 0.5` argument pair twice in the wedge comparator) are frozen by AC-8; widening any of them is prohibited. `detects_intentional_branch_count_drift` in the wedge golden file is a self-test of the comparator and must not be touched.

Also owned: `overhang_plate_fixture` (`modules/core-modules/support-planner/tests/orca_parity_tdd.rs`) — the shared mesh fixture behind `avoidance_keeps_branches_inside_support_outline`, `benchy_orca_parity_within_tolerance` and `node_dropped_when_avoidance_rejects_all_moves`. If a sub-unit rounding shift flips one of those, Step 4 may widen the **fixture's geometric margin**; it may never loosen an assertion.

## Out of Scope

- **Everything `210b` owns**: `support_interface_bottom_layers` parsing, `resolve_interface_bottom_layers`, `densify_bottom_interface`, the code-1003 stub (which keeps firing exactly as today when this packet closes), `support-planner.toml`, `tests/diagnostics_tdd.rs`, `tests/interface_bottom_layers_tdd.rs`, `docs/15_config_keys_reference.md`, `docs/adr/0010-typed-diagnostic-channel.md`, and `DEV-129`.
- The WIT/IR wire format. `record point3-with-width` (`crates/slicer-schema/wit/deps/types.wit`) and `slicer_ir::Point3WithWidth` keep `x: f32, y: f32` — pinned by AC-N3. No host marshal, no `crates/slicer-wasm-host/` change. The supporting argument is the **bounded-error** one in `design.md` §Architecture Constraints, not the discredited "838 mm exact envelope" claim.
- Millimetre-valued non-position quantities: `width`, `flow_factor`, `dist_to_top_mm`, `z`, `effective_layer_height`, `MAX_BRANCH_RADIUS_MM`, `tapered_radius`, `branch_radius`, all angles, and `raft_first_layer_density`. Radii and widths stay `f32` mm because they cross the wire as `f32` mm; canonical smooths radii in `double` for the same reason.
- Mesh-vertex-space helpers that consume `MeshObjectView`'s `f32` millimetre vertices before any node exists: `detect_overhang_facets`, `compute_bounds`, `collect_paint_enforcer_contacts`, `collect_paint_blocker_polygons`, `point_in_any_polygon`, and the `pub fn point_in_polygon(poly: &[[f32; 2]], x: f32, y: f32)` they share. `point_in_polygon` stays `f32` and keeps its `orca_parity_tdd.rs` call sites unchanged; the integer test is the *new* `point_in_polygon_units`, not a replacement.
- Any change to which nodes are *created* — overhang detection, contact admission and `support_on_build_plate_only` semantics are untouched. Which nodes are merged/moved/dropped may shift by sub-unit rounding; that is the measured exposure AC-8 exists to bound, not an intended change.
- Adding any field to `PlannedSupportNode`. `210b`'s bottom band deliberately does not need one.
- The **top**-interface band, and the pre-existing asymmetry that it is emitted before smoothing — see `design.md` `[FWD-4]`.
- Canonical's `smooth_nodes` quirk of reading `pts` (never `pts1`) inside its iteration loop, which makes its 100 iterations idempotent while PnP's is genuinely iterative. A semantic divergence, not a representation one — see `design.md` `[FWD-1]`.

## Deviation Ledger Obligations

- `DEV-128` → `Closed`, referencing this packet and the invariant-2 evidence, and — if Step 4 regenerated either golden pair — naming the mechanism and the regenerated file basenames (AC-8 clause (c) greps for them).
- `DEV-129` is **not** touched here. It stays `Open` until `210b` closes it.
- `design.md` `[FWD-1]` and `[FWD-4]` may each add a further `Open` row if confirmed during implementation. **Do not pre-allocate the ID.** Re-derive it at the moment of writing: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next. Nothing in this packet may quote a `DEV-###` for a new row.

## Authoritative Docs

- `docs/08_coordinate_system.md` — 285 lines; direct ranged read of §"Conversion & Determinism (Normative)", §"Conversion When Porting OrcaSlicer Code", §"Constant Conversion Table", §"SDK Helpers", §"Point2 Wrapper", §"Porting Checklist". Do not read in full.
- `docs/05_module_sdk.md` — 1571 lines; delegate a SUMMARY confirming `Point2`, `mm_to_units`, `units_to_mm`, `SCALING_FACTOR` reach guests through `slicer_sdk::prelude`. Never read directly.
- `docs/DEVIATION_LOG.md` — large; grep `DEV-128` and read that row alone.
- `docs/07_implementation_status.md` — 412 lines; delegate a `LOCATIONS` dispatch for the §"Workstream 3 — Benchy parity and missing OrcaSlicer behavior" insertion point. Never read in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode::position` is declared `Point position;` (Eigen `Vec2crd`, `coord_t` = `int64_t`); the type shape mirrored onto `PlannedSupportNode`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::smooth_nodes` (integer `Point` averaging, truncating `/3`, `double` radii, `max_move = scale_(support_line_width / 2)`).
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — `SCALING_FACTOR_INTERNAL` and `scale_`/`unscale_`; establishes canonical's 1 nm unit, 100× finer than PnP's.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` … `AC-8`.
  - `AC-1`/`AC-2`/`AC-5`/`AC-6`/`AC-7` are static type-shape checks — the cheapest proof the retype is complete rather than partial, and a partial retype is the likeliest silent outcome (Rust compiles a version that converts back to `f32` mid-pipeline perfectly happily). `AC-1`'s negative clause matches a *field declaration*, not the bare token `f32`, so a field doc-comment cannot break it. `AC-2` is checkable only because `design.md` mandates the explicit `neighbours_of` annotation.
  - `AC-3`'s exact-integer assertion (`Point2 { x: 10_000, y: 10_000 }`, no tolerance) is the measurable refinement: a float-tolerant assertion would pass both before and after and prove nothing.
  - `AC-4` is the flagship discriminator — the one criterion that separates a truncating **integer** average from an `f32` millimetre average. Its chain sits at ~5 mm, where `units_to_mm(50_001)` and the `f32` average `5.000133…` are ~70 ULPs apart. **Its command asserts a non-zero passed count and zero failures**; the earlier `… 2>&1 | rg '^test result'` form could not fail, because `test result: FAILED. …` also matches `^test result` and the pipeline's exit status is `rg`'s.
  - `AC-8` is the frozen-golden gate, and it is now genuinely red-able on three independent clauses: the suites must report passes with zero failures, the tolerance constants must be byte-identical, and a golden that changed relative to the merge base must be named in `docs/DEVIATION_LOG.md`. As previously written it was green on an unimplemented tree and could not go red.
- Whole-packet: `AC-17` (invariant 2 — DEV-128's stated trigger), `AC-18` (every test binary in the crate, with a `>= 7` binary count clause so silently-skipped binaries cannot pass), `AC-19` (the gap walk exists exactly once with one caller, so `210b` can add the second without re-deriving it — the `DEV-127` failure mode).
- Negative: `AC-N1` (exactness on a contour vertex), `AC-N2` (`i64` field round-trip above `f32`'s 2^24 consecutive-integer ceiling — the **field**, not the mm boundary), `AC-N3` (wire format explicitly NOT widened, record name and field types bound in one pattern), `AC-N6` (guest freshness, gated on the command's exit code so a broken xtask cannot pass vacuously — its earlier `if …; then … else echo ACN6-FAIL; fi` wrapper exited 0 on the failing branch and defeated its own stated intent), `AC-N7` (the measured mm round-trip envelope pinned in-tree so it cannot rot a fourth time, with every literal clause load-bearing rather than satisfiable by the test's own name), `AC-N8` (`point_in_any_expoly` still excludes points inside holes after the retype — the one thing AC-7's signature-and-cast clauses cannot fail on).
- Cross-packet impact: `210b` consumes `split_column_into_chains`, `point_in_any_expoly`, `first_point_xyw` and `push_interface_scan_lines` at the signatures listed in `packet.spec.md` §Exports Consumed by 210b. `point_in_polygon_units` is a private ring-level primitive of `point_in_any_expoly` and is **not** part of `210b`'s consumed surface — `210b` tests a point against `LayerCollisionCache.collision_polys`, a `Vec<ExPolygon>`. Nothing outside `modules/core-modules/support-planner/` depends on any symbol changed here.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check -p support-planner --all-targets` | Retype compiles including rewritten test files | FACT pass/fail; SNIPPETS ≤20 lines of the first error on failure |
| `cargo test -p support-planner --lib` | In-file unit tests: `prim_mst_on_two_nodes_returns_one_edge`, `smooth_branches_uses_truncating_integer_average`, `point_in_polygon_units_is_exact_on_contour_vertex`, `node_position_roundtrips_beyond_f32_integer_ceiling`, `mm_unit_round_trip_envelope_is_5_120_003_units`, `point_in_any_expoly_excludes_points_inside_holes`, `tapered_radius_*`, `offset_*` | FACT pass/fail + failing test names |
| `cargo test -p support-planner --test multi_neighbour_mst_tdd` | AC-3 exact-integer aggregate | FACT pass/fail |
| `cargo test -p support-planner --test smooth_nodes_tdd` | AC-4/AC-19 guard: smoothing behaviour unchanged through the unchanged public signature, across both the retype and the extraction | FACT pass/fail |
| `cargo test -p support-planner --test orca_parity_tdd` | AC-8 half; also `point_in_polygon` / `tapered_radius` call sites untouched, and the four float-tolerance assertions listed in `design.md` §Risks | FACT pass/fail + failing case names |
| `cargo test -p slicer-runtime --test integration support_golden_regression_wedge` | AC-8 half: the second frozen golden pair | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p support-planner --test to_buildplate_tdd` | Contact admission + code-1002 drop behaviour unchanged | FACT pass/fail |
| `cargo test -p support-planner --test diagnostics_tdd` | Read-only regression check: code 1003 must still fire unchanged — `210b` retires it, not this packet | FACT pass/fail |
| `cargo test -p support-planner` | AC-18 whole-crate sweep; `support-planner`'s `Cargo.toml` has no `[features]` table and no `required-features` targets, so this compiles every test binary (the `CLAUDE.md` silent-zero-test hazard does not apply). Expect **7** binaries: 6 files under `tests/` plus `--lib` | FACT pass/fail + count of `test result: ok` lines |
| `cargo test -p slicer-runtime --test integration support_invariants_wedge` | AC-17 on the real wedge fixture | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | AC-N6; `src/**` is a guest input | FACT: exit code + reports `STALE:` yes/no |
| `SUPPORT_PLANNER_REGEN_GOLDEN=1 cargo test -p support-planner --test orca_parity_tdd benchy_orca_parity_within_tolerance` | Deliberate regeneration, Step 4 only, with justification | FACT: regenerated counts |
| `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test integration support_golden_regression_wedge` | Deliberate regeneration, Step 4 only, with justification | FACT: regenerated counts |
| `cargo check --workspace --all-targets` | Closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure gate; integer casts are the likeliest new lint source | FACT pass/fail + lint names |

## Step Completion Expectations

- The retype is **not** separable into "helpers first, callers later": `prim_mst`, `euclidean_distance`, `aggregate_neighbour_targets`, `point_in_any_expoly`, `clamp_to_avoidance` and `plan_for_object` all reference `PlannedSupportNode`'s field types, so Step 2 must land them together or the crate does not compile between steps. Do not attempt an `f32`↔`i64` shim; a shim is exactly the partial retype AC-1/AC-7 exist to catch.
- **`smooth_branches` is rewritten exactly once, in Step 3.** That rewrite performs both the extraction of `split_column_into_chains` and the integer retype of the averaging. No later step in this packet, and no step in `210b`, may reopen it.
- Step 4 is the mandatory golden reconciliation and must complete — including any regeneration and its written justification — before Step 5.
- `cargo xtask build-guests --check` must be run after the last `src/lib.rs` edit and before AC-17. It must exit 0, not merely print nothing.
- `TASK-326`'s availability and the `DEV-128` row text are ledger facts. Re-derive both at the moment of the Step 5b edit; do not trust any value quoted in this packet.
- **Do not start `210b` from this session.** It is a separate packet with its own preflight, and it must be authored against the signatures this packet actually shipped.

## Context Discipline Notes

- `modules/core-modules/support-planner/src/lib.rs` is 2 058 lines. Read it in ranges: the config/struct header + `from_config`, `plan_for_object`, the free-function helper block (`group_branches_into_columns` / `first_point_xyw` / `smooth_branches` / `push_interface_scan_lines`), and the `#[cfg(test)] mod tests` block are four separate reads. Never open it in full.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` is read-only and only through a delegated FACT on the four named tests. Do not open it to "understand" invariant 2 — `DEV-128` already states it.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` is read-only except for regeneration runs; its tolerance constants are frozen by AC-8.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` is 570 lines and read-only. Read only `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers.
- `docs/DEVIATION_LOG.md` rows are single-line and very long. Grep for the row and read it alone.
- Resist reading `crates/slicer-ir/src/slice_ir.rs` for `Point2`: its shape is `{ x: i64, y: i64 }` with `from_mm` / `to_mm`, and `mm_to_units(mm: f32) -> i64` / `units_to_mm(units: i64) -> f32` / `SCALING_FACTOR: i64` come from `slicer_sdk::coords` via the prelude. The exact bodies of `mm_to_units` / `units_to_mm` are quoted in `design.md` §Architecture Constraints, which is the only place you need them.

## Architecture Constraints

- The mm↔unit boundary is exactly two places and must stay that way: **in** at contact creation (mesh centroids and paint-enforcer contacts, `mm_to_units` / `Point2::from_mm`) and **out** at `Point3WithWidth` construction (`units_to_mm`). Any third conversion is a bug — precisely the `x * SCALING_FACTOR as f32` … `/ SCALING_FACTOR as f32` round trip this packet removes. AC-7 and AC-N1 exist to catch a reintroduction.
- No floating point may appear on the node-position path. `euclidean_distance` uses `(dx * dx + dy * dy).isqrt()` (`i64::isqrt`, stable since 1.84; workspace `rust-version = "1.91.0"`). `closest_point_on_segment` computes its projection ratio with `i128` intermediates (`t_num = dx*tdx + dy*tdy`, `len_sq = dx*dx + dy*dy`, clamp `t_num` to `[0, len_sq]`, then `p0 + d * t_num / len_sq`). `point_in_polygon_units` replaces the `f32` x-intercept division in the ray cast with an `i128` cross-product sign test, removing the division entirely. `aggregate_neighbour_targets` is the one deliberate exception: its `1/d²` weighting accumulates in `f64` (as canonical does in `double`) and rounds once to `i64` at the end — `f64`'s 53-bit mantissa represents every unit value in the build volume exactly, so this is not the lossy path.
- Overflow envelope, stated because integer math hides it: coordinates are bounded by the build volume, so `|dx| ≤ ~4 × 10⁶` units (400 mm) and `dx*dx + dy*dy ≤ ~3.2 × 10¹³`, four orders of magnitude below `i64::MAX`. `i64` is safe for distances and squared distances; `i128` is used only where a product of two coordinate *differences* meets a third coordinate (`closest_point_on_segment`, `point_in_polygon_units` cross products), where the value can reach ~10¹⁹.

### The `f32` mm round-trip envelope — MEASURED, and previously wrong three times

**Do not restate this section from memory. It has been wrong in three successive drafts** (2^24, then "corrected" to 2^23, and neither is the answer). Everything below was measured in-session by compiling the real helper bodies with `rustc -O` and scanning `u` exhaustively.

The helpers, verbatim from `crates/slicer-ir/src/slice_ir.rs`:

```rust
pub const UNITS_PER_MM: f64 = 10_000.0;
pub fn mm_to_units(mm: f32) -> i64 { (mm * UNITS_PER_MM as f32).round() as i64 }
pub fn units_to_mm(units: i64) -> f32 { (units as f32) / UNITS_PER_MM as f32 }
```

There are **two** `f32` roundings on the path `u → mm → u`, not one: `units_to_mm` rounds the quotient, and `mm_to_units` rounds the product again before `.round()` sees it. Each contributes up to a half-ULP, i.e. `2⁻²⁴` relative, so the analytic worst case is `|u| · 2⁻²³ < 0.5`, giving a *provable* bound of `2²² = 4 194 304 units = 419.4304 mm`.

The *measured* behaviour is better than the provable bound but far worse than either previously-claimed one:

| quantity | value |
| --- | --- |
| first `u > 0` with `mm_to_units(units_to_mm(u)) != u` | **5 120 004** (512.0004 mm), which maps to 5 120 005 |
| first `u < 0` that fails | −5 120 004 (symmetric) |
| **largest contiguous exact envelope** | **\|u\| ≤ 5 120 003 = 512.0003 mm** |
| magnitude of the error at the first failure | exactly **1 unit = 100 nm = 10⁻⁴ mm** |
| analytic worst-case bound (safe to quote without measuring) | 2²² = 4 194 304 units = 419.4304 mm |
| previously claimed, both **false** | 2²⁴ = 1677.72 mm; 2²³ = 838.86 mm |

Failures above 5 120 003 are **sparse, not monotone** — 5 120 007 still round-trips exactly. That is why the envelope must be quoted as the largest *contiguous* value, and why spot-checking a single large `u` (as the 2²³ "correction" evidently did) produces a false pass.

**The round trip is load-bearing here, so this is not academic.** `first_point_xyw` reads a stored millimetre back as units via `mm_to_units`, and `smooth_branches` writes back via `units_to_mm`; every re-read of an emitted entry is exactly one `u → mm → u` cycle. AC-N7 pins the measured envelope in-tree so it cannot rot a fourth time.

**Consequence for the wire format — the supporting claim changes, the decision survives.** The rejected alternative "widen `Point3WithWidth`" was previously justified by "emitted values are inside the 838 mm exact envelope for any real build volume." **That justification is false.** 512.0003 mm does not cover every real build volume: 500 mm-class beds sit right at the edge and 600 mm-class machines (e.g. Modix BIG-60) exceed it outright on a single axis. Keeping the `f32` mm wire format is nevertheless still correct, on three different grounds:

1. **The failure is bounded and graceful, not catastrophic.** At the first failure the discrepancy is exactly 1 unit = 100 nm. It stays ≤ 1 unit well past 1 600 mm, because `f32`'s ULP at 1 000 mm is ≈ 6.1 × 10⁻⁵ mm ≈ 0.6 unit. There is no cliff, only a 100 nm quantisation that switches on above 512 mm.
2. **100 nm is below every tolerance in the system.** G-code emits 3–4 decimal millimetres (1 µm = 10 units); `orca_parity_tdd`'s Hausdorff bound is 0.5 mm (5 000 units); the packet already accepts a 0.5-unit rounding grid at contact creation. A 1-unit boundary error is 3–4 orders below the coarsest of these.
3. **Widening is cross-crate and cross-WIT** (`crates/slicer-schema/wit/deps/types.wit`, `slicer_ir::Point3WithWidth`, every host/guest marshal), and it would buy a precision nobody consumes. If a 600 mm-class machine ever needs sub-100 nm emitted positions, that is a separate packet with a schema bump — pinned out of this one by AC-N3.

Do **not** re-derive this as "f32 holds integers exactly to 2²⁴". That statement is true and *irrelevant*: it describes integer representability, which is what AC-N2 pins on the `i64` field, and it is not the round-trip bound.

### Remaining constraints

- `prim_mst`'s `f32::INFINITY` sentinel becomes `i64::MAX`, and `active_nodes.sort_by(|a, b| a.x.partial_cmp(&b.x) …)` becomes a total `sort_by_key(|n| (n.x, n.y))`. The `partial_cmp` fallbacks (`Some(Equal) | None`) exist only because `f32` is not `Ord`; deleting them is part of the win and removes a real (if unreached) NaN-ordering hazard from the deterministic MST input order.
- **The `smooth_branches` seam.** This packet performs the *only* rewrite of `smooth_branches` across `210a` and `210b`: the integer Laplacian and the `split_column_into_chains` extraction land together in Step 3, and `210b` adds nothing but a second caller. The extraction is behaviour-preserving only if all three of these hold: `split_column_into_chains` returns every sub-chain range including those shorter than 3; the `e - s < 3` skip and the `column.len() < 3` early-continue stay in `smooth_branches`; and the current walk's `None ⇒ break` on a malformed entry is preserved (it terminates the split loop, leaving the remaining indices inside the final chain — *not* `continue`, which would change chain boundaries). `smooth_nodes_tdd.rs`'s four cases (`smoothing_reduces_curvature`, `endpoints_held_fixed`, `columns_below_three_points_unchanged`, `empty_entries_no_panic`) are the guard and must not be edited.
- **The `f32` site counts are matching-line counts, not occurrence counts, and the plan file quotes the former.** `docs/specs/deviation-remediation-206-212-plan.md` says "~113 f32 sites". Measured on the current tree: `rg -c 'f32' modules/core-modules/support-planner/src/lib.rs` = **113 matching lines**, while `rg -o 'f32' … | wc -l` = **153 total occurrences**. Restricted to the region outside the `#[cfg(test)]` module (which begins at the file's single `#[cfg(test)]` marker): **83 matching lines / 114 occurrences**. Anyone re-deriving the count with a different flag will get a different number and think the packet is stale — it is not; state which measure you used.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Concretely: canonical's `max_move = scale_(support_line_width / 2)` is `support_line_width / 2 × 10⁶`. Never transcribe that literal. Compute PnP's cap from PnP's own configured value through `mm_to_units`, which is `× 10⁴`. Any hard-coded scaled constant lifted from `TreeSupportCommon.hpp` is 100× too large and will silently disable the cap.
- No schema or public version constant is bumped. `record point3-with-width` is unchanged; no manifest key moves, so the generated key table in `docs/15_config_keys_reference.md` does not move.

## Data and Contract Notes

- IR/manifest contracts: none change. `SupportPlanEntry`, `RaftPlan`, `Diagnostic` and `support-planner.toml`'s `[config.schema.*]` values are all untouched. No config key is added, removed or retyped.
- WIT boundary: unchanged. `record point3-with-width` keeps `x: f32, y: f32`; the guest still marshals millimetres. Because no WIT file is edited, the `CLAUDE.md` WIT/Type-Changes checklist does not fire — but the guest-staleness rule does, for `src/**`.
- Determinism: this packet *increases* determinism (total integer node sort, integer MST tie-breaks, exact integer averaging). No claim, stage, or dependency edge changes. No diagnostic code changes — code 1003 keeps firing until `210b` retires it.

## Locked Assumptions and Invariants

- **Locked:** node positions are internal units end-to-end between contact creation and emission; exactly two conversion boundaries exist. Reversible only by re-introducing `f32` fields, which AC-1 forbids.
- **Locked:** the emitted wire format stays `f32` millimetres (AC-N3). Any future integer wire format is a separate packet with its own schema bump.
- **Locked:** `smooth_branches`' integer average uses truncating `/ 3`, matching canonical's `Point` division, not a rounding division. A rounding variant drifts from canonical by up to one unit per point per iteration.
- **Locked:** `split_column_into_chains` returns all ranges; the `< 3` filters stay in `smooth_branches`. Moving them into the helper would silently deny floor bands to short chains in `210b` and make the extraction non-behaviour-preserving.
- **Invariant (bounded, MEASURED — not "exact"):** `mm_to_units(units_to_mm(u)) == u` holds for every `|u| ≤ 5 120 003` (512.0003 mm), and above that the result differs from `u` by at most 1 unit (100 nm) throughout the representable build-volume range. The provable analytic bound is `2²² = 4 194 304` units (419.4304 mm); the exhaustively measured contiguous bound is 5 120 003. **2²³ and 2²⁴ are both wrong** — see §Architecture Constraints for the measurement and for why the wire-format decision survives on different grounds. AC-N7 pins this in-tree.
- **Not locked:** whether `merge_distance_mm`, `line_width_mm` and the other `SupportPlanner` config fields remain `f32` mm. They may stay as parsed; only their *comparisons* against node distances must be in units.

## Risks and Tradeoffs

- **The frozen goldens are the packet's real regression surface, and there are two of them.** `benchy_orca_parity_within_tolerance` (`orca_parity_tdd.rs`) compares branch endpoints against `resources/golden/benchy_tree_support_orca_endpoints.txt` (symmetric Hausdorff ≤ 0.5 mm, `let tolerance_mm = 0.5_f32;`) and `..._branch_count.txt` (±10%, `let tolerance_fraction = 0.10_f32;`); `current_wedge_output_stays_within_self_capture_tolerance` applies the same bounds (`0.10, 0.5` positional arguments, twice each in that file). `isqrt` truncation in MST weights, exact-integer `point_in_polygon_units`, and the integer `max_move_xy` cap can each flip a merge/drop/densify decision. Step 4 owns the reconciliation, including regeneration via `SUPPORT_PLANNER_REGEN_GOLDEN=1` / `SUPPORT_WEDGE_REGEN_GOLDEN=1` with a written canonical-correctness justification that must also land in the `DEV-128` closure text — AC-8 clause (c) checks for it, so a silent regeneration now fails a criterion instead of passing unnoticed.
- **`orca_parity_tdd.rs` carries float-tolerance assertions.** The complete list, verified against the file: `radius_tapers_with_distance_to_top` (two — `< 1e-6` and `< 1e-4`), `raft_and_interface_layers_emit_expected_entry_count` (`raft_first_layer_density`, `< f32::EPSILON`), `wall_count_scales_max_move_distance` (`< 1e-6`), plus the golden comparison. Only `raft_first_layer_density` is genuinely insensitive to the retype. Step 4 owns any fallout: widen `overhang_plate_fixture`'s geometric margin or regenerate a golden, never loosen a tolerance.
- **Behaviour shifts by sub-unit amounts.** Rounding contacts to the 100 nm grid can flip a node that sat within 10⁻⁵ mm of a collision boundary. `to_buildplate_tdd.rs` and `diagnostics_tdd.rs` assert on *whether* diagnostics fire rather than on coordinates, so their exposure is low.
- **The `smooth_branches` rewrite is the single highest-risk edit in the packet.** It is shipped, tested code; it is retyped and restructured in one step; and its guard (`smooth_nodes_tdd.rs`) must stay byte-identical. If that suite goes red, the correct response is to revert Step 3 and redo it, never to adjust the guard.
- **Clippy cast lints.** `mm_to_units` / `units_to_mm` encapsulate most casts, but `i128 → i64` narrowing in `closest_point_on_segment` and `f64 → i64` in `aggregate_neighbour_targets` may trip `-D warnings`. Resolve with explicit bounded conversions, not `#[allow]`.
- **`isqrt` truncates.** `euclidean_distance` now returns `floor(√(dx²+dy²))`, so 0.99999 units reads as 0. This matters only for the merge threshold (smallest meaningful value 0.8 mm = 8 000 units) and the `1/d²` weighting (whose degenerate branch already handles 0). Strictly better than `f32`'s relative error at large coordinates.
- **The wedge suites are the slow gate and need a fresh guest.** Running AC-17 or AC-8's wedge half before `build-guests` is the single most likely way to waste a cycle on a false failure.
- **`210b` is blocked on this packet being merged, not authored.** If this packet's exported signatures change during implementation, `210b`'s `design.md` §Prerequisites must be updated in the same session, or `210b` will be implemented against a shape that does not exist.
