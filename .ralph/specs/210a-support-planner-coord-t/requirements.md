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

`.ralph/specs/211-support-interface-bottom-layers/` remains `status: superseded` and is neither revived nor deleted — its `superseded_by` still points at the merged `210`, which is the truthful record of what happened to it. Its work now lives in `210b`. Do not implement that directory, do not edit it, do not delete it.

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
