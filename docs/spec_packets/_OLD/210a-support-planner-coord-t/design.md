# Design: 210a-support-planner-coord-t

## Controlling Code Paths

- Primary code path: `modules/core-modules/support-planner/src/lib.rs` — `PlannedSupportNode`, `SupportPlanner::plan_for_object`'s contact-creation / MST / merge / move / emit loop and its post-smoothing tail, and the free-function helper block (`group_branches_into_columns`, `first_point_xyw`, `smooth_branches`, `point_in_any_expoly`, `prim_mst`, `euclidean_distance`, `aggregate_neighbour_targets`, `clamp_to_avoidance`, `closest_point_on_polygon`, `closest_point_on_segment`, `push_interface_scan_lines`).
- Tests edited: `tests/multi_neighbour_mst_tdd.rs` (rewritten) and the in-file `#[cfg(test)] mod tests` block.
- Tests read-only and used as oracles: `tests/smooth_nodes_tdd.rs` (guards both the `smooth_branches` retype and the `split_column_into_chains` extraction), `tests/to_buildplate_tdd.rs` (fixture shape), `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`.
- Frozen-golden fixtures owned for deliberate regeneration: `resources/golden/benchy_tree_support_orca_{endpoints,branch_count}.txt` and `resources/golden/support_regression_wedge_{endpoints,branch_count}.txt`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

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

## Code Change Surface

Selected approach: **retype the node in place keeping the wire format, then rewrite `smooth_branches` once to be both integer and chain-split-aware.**

- `PlannedSupportNode` — `x: f32, y: f32` → `x: i64, y: i64`.
- **Struct-literal blast radius (complete, grep-verified): six literal sites — four in `plan_for_object`** (the overhang-facet contact push, the paint-enforcer contact push, the no-MST-neighbour propagate-unchanged branch, and the moved-node branch) **plus two inside the in-file `#[cfg(test)] mod tests` case `prim_mst_on_two_nodes_returns_one_edge`.** The struct is private (`struct PlannedSupportNode`, no `pub`), so there are **no** sites outside `src/lib.rs`; checked, not assumed. Zero sites under `tests/`. All six land in Step 2.
- Test-assertion fallout in the same step: `prim_mst_on_two_nodes_returns_one_edge` currently asserts `(edges[0].2 - 5.0).abs() < 1e-4` (an `f32` millimetre distance). It becomes the exact `assert_eq!(edges[0].2, 50_000)` — 3 mm/4 mm legs give a 5 mm hypotenuse = 50 000 units, and `(30_000² + 40_000²).isqrt()` is exactly `50_000`. Never weakened to a tolerance.
- `prim_mst(nodes: &[PlannedSupportNode]) -> Vec<(usize, usize, i64)>`; `min_dist`/`best_dist` sentinels `i64::MAX`; the final `edges.sort_by` tie-break becomes a plain `cmp` chain.
- `euclidean_distance(a, b) -> i64` via `.isqrt()`.
- The per-node neighbour table binding **carries an explicit type annotation**: `let mut neighbours_of: Vec<Vec<(usize, i64)>> = …`. This is mandated, not incidental — AC-2 greps for it, and an inferred binding would leave the retype unverifiable by static check.
- `aggregate_neighbour_targets(neighbour_positions: &[Point2], distances_units: &[i64]) -> Option<Point2>`; `EPS_MM` deleted; degenerate branch keyed on `== 0`.
- `point_in_polygon_units(poly: &[Point2], p: Point2) -> bool` — **net-new**, `i128` cross-product ray cast over a **single ring**. Its **only in-packet call site is `point_in_any_expoly`**, which calls it once for the contour ring and once per hole ring; nothing else in this module has a bare `&[Point2]` ring to test. It is a private primitive of the `ExPolygon` test, not a standalone membership API, and it is deliberately *not* what `210b` consumes — `210b`'s `LayerCollisionCache.collision_polys` is `Vec<ExPolygon>`, so `210b` calls `point_in_any_expoly`.
- `point_in_any_expoly(polygons: &[ExPolygon], p: Point2) -> bool` — drops both `* SCALING_FACTOR as f32` scalings and the `p.x as f32` collection maps; compares `Point2` against `ExPolygon`'s existing `i64` points directly. **The retype is signature-and-arithmetic only; the hole semantics are unchanged and must be written down rather than left implied.** Today's body is `point_in_polygon(outer) && !ex.holes.iter().any(|h| point_in_polygon(h, …))` — a point inside a hole is NOT inside the `ExPolygon`. The migrated composition is exactly:

  ```text
  polygons.iter().any(|ex|
      point_in_polygon_units(&ex.contour.points, p)
      && !ex.holes.iter().any(|h| point_in_polygon_units(&h.points, p)))
  ```

  Dropping the hole term would let the avoidance clamp and the collision drop treat a model hole as solid, and would let `210b`'s floor band land inside one. Nothing in AC-7 catches that — AC-7 pins the two signatures and bans `SCALING_FACTOR as f32`, all of which a contour-only body satisfies — so it is pinned behaviourally by **AC-N8** (`point_in_any_expoly_excludes_points_inside_holes`). **Exported to `210b`** at exactly this signature, for its model-landing test. `ExPolygon`'s field names (`contour: Polygon`, `holes: Vec<Polygon>`, each `Polygon` carrying `points: Vec<Point2>`) are verified against `slicer_ir::ExPolygon` / `slicer_ir::Polygon` (`crates/slicer-ir/src/slice_ir.rs`); do not assume a flattened ring list.
- `clamp_to_avoidance(p: Point2, avoidance_polys: &[ExPolygon]) -> Point2`; `closest_point_on_polygon(poly: &[Point2], p: Point2) -> (Point2, i64)`; `closest_point_on_segment(p0: Point2, p1: Point2, t: Point2) -> Point2`.
- `push_interface_scan_lines(out, centre: Point2, z: f32, half_units: i64, width_mm: f32, spacing_units: i64, parity: i32, avoidance_polys, collision_polys)` — `z` and `width` stay `f32` mm; endpoints emitted through `units_to_mm`. **Exported to `210b`** at exactly this signature.
- `first_point_xyw(entry) -> Option<(Point2, f32)>` — position in units (via `mm_to_units` of the stored mm), width in mm, **no `z`**. `210b` reads `z` separately from `entry.branch_segments.first()?.first()?.z`; that is a consequence of this signature and is recorded here so `210b`'s plan is not surprised by it.
- `merge_distance_mm` stays an `f32` config field; only the comparison moves to units (`mm_to_units(self.merge_distance_mm)`).
- `max_move_xy` → `let max_move_xy: i64 = mm_to_units((tan_angle * effective_height * wall_count_factor).max(0.0));`.
- New in-file `#[cfg(test)]` cases: `smooth_branches_uses_truncating_integer_average`, `point_in_polygon_units_is_exact_on_contour_vertex`, `point_in_any_expoly_excludes_points_inside_holes` (AC-N8 — an `ExPolygon` with one hole; a point inside the hole is `false`, a point inside the contour but outside the hole is `true`; this is the only check that fails if the hole term is dropped during the retype), `node_position_roundtrips_beyond_f32_integer_ceiling` (which must construct and read the `i64` field directly and must **not** route through `units_to_mm`/`mm_to_units` — it pins the field, not the boundary), and `mm_unit_round_trip_envelope_is_5_120_003_units` (AC-N7).
- Rewritten: `tests/multi_neighbour_mst_tdd.rs` (all four cases, in units, exact assertions).
- `//!` module header: add a sentence stating node positions are `i64` internal units and naming the two conversion boundaries.

### The shared seam (Step 3, one rewrite)

- `split_column_into_chains(entries: &[SupportPlanEntry], column: &[usize]) -> Vec<(usize, usize)>` — **net-new, private.** Half-open `(start, end)` ranges into `column`. Declares the fn-local `const CHAIN_BREAK_THRESHOLD_UNITS: i64 = 50_000` (5.0 mm × 10 000) replacing the fn-local `const CHAIN_BREAK_THRESHOLD_MM: f32 = 5.0` that lives inside `smooth_branches` today. The gap test compares squared units (`dx*dx + dy*dy > CHAIN_BREAK_THRESHOLD_UNITS.pow(2)`) rather than taking a square root. Returns every range, including short ones. Preserves the `None ⇒ break` malformed-entry behaviour exactly. **Exported to `210b`** at exactly this signature.
- `smooth_branches` — public signature `(&mut Vec<SupportPlanEntry>, usize)` unchanged. Body becomes: `group_branches_into_columns` → `column.len() < 3` continue → `split_column_into_chains` → per range, `e - s < 3` continue → one scratch `Vec<Point2>` of the range's positions → `iterations` passes of `(prev + cur + next) / 3` in `i64` (truncating, matching canonical `TreeSupport::smooth_nodes`) → one `units_to_mm` write-back per point after the final pass. Widths keep their `f32` averaging and `MAX_BRANCH_RADIUS_MM` clamp; `z`, `role`, `speed_factor`, layer index, ids and counts are preserved as today.

### Rejected alternatives

- **Keeping the merged 210 (210 + 211 in one packet).** Rejected by user decision 2026-08-07 after the reviewer ruled `SIZE: must decompose`. The `smooth_branches` collision that motivated the merge is confined to Step 3, which lives entirely in this half; `210b` consumes the extracted helper and never reopens the function. See `requirements.md` §Provenance.
- **`pos: Point2` as the node field instead of two `i64`s.** Marginally tidier and free `Ord`, but it changes every field access in `plan_for_object` from `node.x` to `node.pos.x`, enlarging the diff without changing behaviour, and AC-1 pins `x: i64, y: i64`. `Point2` is still used for helper signatures where it is the natural parameter type.
- **Store squared distances in the MST tuple** (avoiding `isqrt`, matching canonical's `max_move_distance2`). Order-equivalent and cheaper, but it changes the *meaning* of the third tuple element and of `aggregate_neighbour_targets`' second slice — a silent-wrong-argument hazard for any future caller. `isqrt` is exact and integer, so nothing is lost.
- **Widen `Point3WithWidth` / `point3-with-width` to `f64` or integer.** Out of scope, cross-crate, cross-WIT. The *old* justification ("inside the 838 mm exact envelope") is retired as false; the decision now rests on the bounded-error argument in §Architecture Constraints — the round trip degrades to a 1-unit (100 nm) quantisation above 512.0003 mm, three to four orders below every tolerance downstream. Pinned out by AC-N3.
- **Convert the mesh-space detection helpers too.** `MeshObjectView` delivers `f32` millimetre vertices; converting them adds a boundary rather than removing one, and churns `orca_parity_tdd.rs`'s `point_in_polygon` call sites for no correctness gain.
- **Duplicate the gap walk in `210b` instead of extracting it here.** Recreates `DEV-127`'s "two drifted copies" failure mode. AC-19 forbids it in this packet and `210b`'s own AC-19 raises the call count to three.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` — the node pipeline, the smoother, and the in-file unit tests.
- `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` — rewrite all four cases in units with exact integer assertions.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — **conditionally, Step 4 only**: widening `overhang_plate_fixture`'s geometric margin if a sub-unit rounding shift flips one of its cases. Assertions and tolerance constants may not be loosened.
- `resources/golden/benchy_tree_support_orca_{endpoints,branch_count}.txt` — regenerated via `SUPPORT_PLANNER_REGEN_GOLDEN=1` only in Step 4, with written justification.
- `resources/golden/support_regression_wedge_{endpoints,branch_count}.txt` — regenerated via `SUPPORT_WEDGE_REGEN_GOLDEN=1` only in Step 4, with written justification.
- `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` — Step 5b ledger edits only, no design content.

The per-step cap of 3 edits is respected by the step decomposition in `implementation-plan.md`, not by this list.

## Read-Only Context

Cited by symbol; line numbers are deliberately omitted because these files are edited by this packet and by parallel work.

- `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — the four `#[test]` fns `smoothing_reduces_curvature`, `endpoints_held_fixed`, `columns_below_three_points_unchanged`, `empty_entries_no_panic`, plus the helper fns they call (`pt`, `entry`, `build_column`, `read_column`, `max_turn_angle`). Purpose: confirm they are shape assertions, not hard float equalities, so Step 3 need not touch them. They are the guard for both the integer retype and the chain-split extraction. The four tests sit *after* the helper block; locate by name.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — the fns `radius_tapers_with_distance_to_top`, `avoidance_keeps_branches_inside_support_outline`, `raft_and_interface_layers_emit_expected_entry_count`, `wall_count_scales_max_move_distance`, `benchy_orca_parity_within_tolerance`, `node_dropped_when_avoidance_rejects_all_moves`, and the shared `overhang_plate_fixture`. Purpose: Step 4's reconciliation. This file is **not** benign for this packet; see §Risks.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers. Purpose: confirming the collision fixture's expectations are membership-based, not coordinate-based. Read-only.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — delegated FACT only, no direct read. Purpose: AC-17.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — the fns `current_wedge_output_stays_within_self_capture_tolerance` and `detects_intentional_branch_count_drift`, and the `SUPPORT_WEDGE_REGEN_GOLDEN` env gate. Purpose: AC-8's second half. Read-only; the comparator self-test must not be touched.
- `crates/slicer-schema/wit/deps/types.wit` — the `interface geometry` block's `record point3-with-width` only. Purpose: AC-N3. Do not edit.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, `modules/core-modules/support-planner/support-planner.wasm`, `modules/core-modules/support-planner/wit-guest/target/` — never load.
- `crates/slicer-wasm-host/`, `crates/slicer-ir/`, `crates/slicer-sdk/`, `crates/slicer-schema/` — no edit; this packet changes no cross-crate type. Editing `slicer-ir`, `slicer-sdk`, `slicer-schema` or `slicer-core` stales all 34 guests.
- `crates/slicer-runtime/tests/**` — the invariant suite is this packet's oracle; editing it to accommodate the retype is prohibited. The *golden data files* under `resources/golden/` are the one deliberate exception, and only under Step 4's regeneration contract.
- `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — the behaviour guard; if it needs editing, the migration or the extraction went wrong.
- **Everything owned by `210b`**: `modules/core-modules/support-planner/support-planner.toml`, `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`, `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs`, the `SupportPlanner.support_interface_bottom_layers` field, `resolve_interface_bottom_layers`, `densify_bottom_interface`, the code-1003 block in `run_support_geometry`, `docs/15_config_keys_reference.md`, `docs/adr/0010-typed-diagnostic-channel.md`. This packet leaves the code-1003 stub firing exactly as today.
- `docs/spec_packets/211-support-interface-bottom-layers/` — superseded; retained for provenance. Never edit, never implement.
- `docs/spec_packets/210b-support-interface-bottom-layers/` — the paired packet; read its §Prerequisites if you need to know what you are exporting, never edit it.
- Every other `modules/core-modules/*` directory — unrelated.

## Expected Sub-Agent Dispatches

- Question: what exact type is `SupportNode::position` and is there any `float`/`double` position member?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`; return: `FACT` (≤5 lines); purpose: Step 2's module-header justification.
- Question: does `TreeSupport::smooth_nodes` average `SupportNode::position` with integer `Point` arithmetic and a truncating `/3`, is the radius average `double`, and what is `max_move` derived from?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY` (≤200 words, no code); purpose: Step 3's averaging shape.
- Question: do `benchy_orca_parity_within_tolerance` and `current_wedge_output_stays_within_self_capture_tolerance` pass, and if not what are the reported Hausdorff distance and branch counts?; scope: the two golden suites; return: `FACT` (≤5 lines); purpose: Step 4.
- Question: do `branch_endpoints_are_outside_support_collision_outlines`, `branch_points_match_entry_layer_z`, `branch_radii_stay_within_current_bounds` and `branch_curvature_below_threshold` pass?; scope: `cargo test -p slicer-runtime --test integration support_invariants_wedge`; return: `FACT` pass/fail plus ≤20 lines of the first failure; purpose: AC-17. Never absorb the full `--test integration` output.
- Question: does `cargo clippy --workspace --all-targets -- -D warnings` pass, and which lints fire in `support-planner`?; scope: workspace; return: `FACT` pass/fail + lint names; purpose: Step 5. Expect `clippy::cast_possible_truncation` / `cast_precision_loss` at the conversion boundaries.
- Question: what is the current status cell of `DEV-128`, and is `TASK-326` still absent from `docs/07_implementation_status.md`?; scope: both files; return: `FACT` (≤5 lines); purpose: Step 5b, re-derived at point of use.

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

## Context Cost Estimate

- Aggregate: `M`.
- Largest steps: `M` (Step 2, the atomic retype; Step 3, the single `smooth_branches` rewrite). No step is `L`.
- Highest-risk dispatches and required return formats: the `TreeSupport::smooth_nodes` `SUMMARY` (≤200 words, no code) and the AC-17 wedge-invariant run (`FACT` pass/fail plus at most 20 lines of the first failure — never the full `--test integration` output, which aggregates dozens of modules).

## Open Questions

- `[FWD-1]` Canonical's `TreeSupport::smooth_nodes` writes into `pts1` while its iteration loop keeps reading `pts`, making its 100 iterations idempotent (one Jacobi pass repeated), whereas PnP's `smooth_branches` is genuinely iterative Gauss-Seidel over 100 passes. A real behavioural divergence, but a *semantic* one rather than a representation one, and changing it here would confound AC-4 and AC-17. If the Step 3 dispatch confirms it, file a new `DEV-###` row (re-derive the next free ID at that moment) and leave the behaviour as-is.
- `[FWD-2]` `push_interface_scan_lines`' `half` and `spacing` derive from `radius + tree_support_branch_distance * 0.5` and `tree_support_interface_spacing_mm`, both `f32` mm. Convert at the call site (`mm_to_units(bbox_half)`) so the helper body is unit-clean — this is the preference, and `210b`'s `densify_bottom_interface` is written assuming it. Converting inside the helper is acceptable but must then be done consistently for every call site, and `210b` must be told.
- `[FWD-3]` `active_nodes.sort_by(...)` currently sorts by `x` then `y`. After the retype it becomes `sort_by_key(|n| (n.x, n.y))`, and two nodes at an identical integer position now compare equal where `f32` might have separated them. Ties are already possible today and the downstream MST is order-stable, so no behaviour change is expected; if a determinism test flakes, extend the key to `(n.x, n.y, n.dist_to_top)` rather than reintroducing a float tiebreak.
- `[FWD-4]` The top-interface band is emitted inside the layer loop, *before* `smooth_branches`, so it is centred on pre-smoothing positions while its own structural point moves. Pre-existing; this packet does not touch it and `210b` mirrors the bottom band *after* smoothing deliberately. If confirmed, file a new `DEV-###` row (re-derive the ID) and leave the top band alone.

No `[BLOCK]` items.
