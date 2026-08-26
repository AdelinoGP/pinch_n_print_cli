# Spec Review: 238b-tree-planner-canonical-fidelity

**Packet Path**: `docs/spec_packets/238b-tree-planner-canonical-fidelity/`
**Status**: `draft` (correct terminal state — human sign-off separately pending per plan §8)
**Review Mode**: `full` (packet scope, final closure review)
**Reviewed**: 2026-08-25
**Reviewer**: final packet-close reviewer (cold, adversarial)

---

## Summary

All 12 implementation steps landed uncommitted in the working tree and the acceptance
ceremony (`tmp/p238b-ceremony.md`) reports AC-1..AC-14, AC-N1..N3, clippy, and
check-literals all PASS. I independently traced every AC to file:symbol evidence and
confirmed the transport, keying, and style changes are real (not placeholder tests, not
dead helpers). The one substantive gap is a test-coverage hole on the AC-14 nonzero
`wall_counts` path (residual #4): the carrier is structurally complete and length-verified
end-to-end, but no test asserts a `need_extra_wall` node actually produces `wall_counts ≥ 1`.
This is a MEDIUM finding, not a correctness defect — the field is additive with
`#[serde(default)]` + `Default`, so a missing value degrades safely. No Critical findings,
no `[unverified]` load-bearing rows.

---

## Acceptance Criteria Check

| AC | Status | Evidence (file:symbol) |
|----|--------|------------------------|
| AC-1 top-Z gap | PASS | `canonical_top_gap_uses_nominal_layer_count_with_variable_heights` (`crates/slicer-runtime/tests/executor/tree_support_top_gap_variable_height_tdd.rs:261`) asserts `ceil(gap/nominal)+1`, virtual-gap-node non-extrusion, and discriminates the deleted accumulated-Z walk; registered `mod tree_support_top_gap_variable_height_tdd;` (`executor/main.rs:109`). Ceremony: executor 206 passed. |
| AC-2 smoothing | PASS | Option A recorded in `design.md` §The Smoothing Decision Point; `smooth_nodes` called at `lib.rs:2766` before emit pass (`lib.rs:2791`); f64 kernel + 100 iters + `max_move = line_width/2` (`lib.rs:534-606`). Ceremony: smooth_nodes_tdd 6 passed. |
| AC-3 role coexistence | PASS | `build_roles` (`lib.rs:800-857`) builds body/roof/floor separately, subtracts roof/floor from body via `clip_polygons` Difference; whole-layer `carved.clear()` gone. Ceremony: tree_family_tdd 8 passed. |
| AC-4 circle fidelity | PASS | `BRANCH_CIRCLE_SEGMENTS=16` now only in `swept_region` (`lib.rs:964-991`, the documented port addition); emitted role contours not truncated; simplify gated by `role_simplify_tolerance`. Ceremony: multi_neighbour_mst_tdd 6 passed. |
| AC-5 collision/avoidance/largest-part | PASS | `body_overlaps_occupancy` is `#[doc(hidden)]` test-only (`lib.rs:4205-4209`); emit gate uses `get_collision(radius, cache_idx)` + `point_inside_collision_volume` (`lib.rs:2876-2878`); largest-part carve test `collision_carve_keeps_only_the_largest_surviving_part` (`wall_clearance_tdd.rs:270`). Ceremony: wall_clearance_tdd 4 passed. |
| AC-6 miter limit 3.0 | PASS | `miter_limit: Some(3.0)` at `lib.rs:1364`/`1409`/`3713`; `offset_with_miter_limit` (`crates/slicer-core/src/polygon_ops.rs`), `offset_polygons_with_miter_limit` (`crates/slicer-sdk/src/host.rs`), `OffsetRequest.miter_limit` (`host_batch.rs`), WIT `miter-limit: option<f32>` on `offset-polygons` + `offset-request` (`common.wit`). Default `offset` keeps 2.0. |
| AC-7 TreeVolumes ctor | PASS | `TreeVolumes::new` simplifies at `RADIUS_SAMPLE_RESOLUTION_MM` via `expolygons_simplify_union` (`lib.rs:1263-1265`) before `layer_outlines_below`. Ceremony: expolygons_simplify 5 passed. |
| AC-8 to_buildplate split | PASS | `contact_seed_to_buildplate`/`branch_a_to_buildplate` use `!is_inside_ex(get_collision(0,l))` (`lib.rs:4170-4178`); `move_pass_to_buildplate` keeps raw outlines (F-14 exception, `lib.rs:4182`). Ceremony: to_buildplate_tdd 7 passed. |
| AC-9 move_out_expolys | PASS | Dilated-ring projection + `pt_max` clamp + bool return (`lib.rs:4629-4656`); false `from0` comment corrected (`lib.rs:4625`); `branch_a_move_out_args` vs `studio_4252_move_out_args` distinct (`lib.rs:1053-1065`). Ceremony: move_out 3 passed. |
| AC-10 mesh-path shim | PASS | `has_analysis_contacts` gate (`lib.rs:1802`) + `filter(|_| !has_analysis_contacts)` (`lib.rs:1809`); test `analysis_contact_makes_legacy_mesh_projection_unreachable` (`diagnostics_tdd.rs:420`) asserts a blocked host candidate suppresses `mesh-demand-*`. Boundary recorded in `design.md` §Mesh-path boundary. Ceremony: diagnostics_tdd 7 passed. |
| AC-11 branch-A roof counter | PASS | `branch_a_roof_counter = parent_counter - (parent_distance_to_top >= 0)` (`lib.rs:4188-4190`); `insert_dropped_node_roof_counter = max` (`lib.rs:4194-4196`). Test `branch_a_two_leaf_collapse_inherits_parent_roof_counter` (`multi_neighbour_mst_tdd.rs:28`). Ceremony: 1 passed. |
| AC-12 tree styles | PASS | `TreeSupportStyle` enum + `from_config` (`lib.rs:195-212`); `style_neighbour_direction_for` unweighted sum (`lib.rs:4356-4370`); `style_movement_for` dot-product gate (`lib.rs:4372-4395`); hybrid `TreeNodeType::Polygon` minting (`lib.rs:3830-3834`); helpers wired to production call sites (`lib.rs:2561,2608,3833`). Ceremony: tree_style_styles_tdd 4 passed. |
| AC-13 simplify gating | PASS | `role_simplify_tolerance` gates to `is_base_area && avg_node_per_layer > COARSE_CIRCLE_NODE_THRESHOLD` at `line_width*0.5` (`lib.rs:859-866`). Ceremony: build_roles 4 passed. |
| AC-14 extra-wall transport | PASS (gap noted) | `wall-counts: list<u32>` in `record support-plan-skeleton` (`prepass-support-geometry.wit`) + `support-plan-view-skeleton` (`ir-types.wit`); `wall_counts: Vec<u32>` on `SupportPlanSkeleton` (`slice_ir.rs:1322`); both marshal legs map + assert length (`marshal/in_.rs:884-930`, `marshal/native.rs:640-647`); emit fill from `need_extra_wall` (`lib.rs:3338-3352`); schema 2.0.0→2.1.0. **Gap:** no nonzero-value e2e assertion (see MED-1). |
| AC-N1 final-geometry validation | PASS | Emit gate reads radius-baked collision on FINAL (post-smoothing) geometry (`lib.rs:2876-2878`); `emit_gate_uses_radius_baked_collision_point_in_semantics` (`wall_clearance_tdd.rs:302`). Ceremony: same run as AC-5. |
| AC-N2 unknown style rejected | PASS | `enum_values` in `ConfigBoundsIndex` + `check` rejects undeclared enum (`crates/slicer-scheduler/src/config_resolution.rs`); test `rejects_unknown_support_style_value` loads real manifest (`config_bounds_enforcement_tdd.rs`); manifest declares `support_style` enum with 7 values (`tree-support-planner.toml:213-216`). Ceremony: 1 passed. |
| AC-N3 guest freshness | PASS | Ceremony `cargo xtask build-guests --check` exit 0. |

---

## Requirements Traceability

All 14 divergence rows in `requirements.md` §In Scope map to an AC with traced evidence
(see table above). DEV-128 sizing recorded as waiver (`design.md` §DEV-128 Sizing, sized
M ⇒ not implemented). No orphaned requirements; no unrequested implementations beyond the
justified cross-crate surfaces named in `design.md` §Files in Scope.

---

## Design Fidelity

- **Selected approach followed**: divergence-by-divergence edits inside the planner module,
  red-first pinned by owning test files. Confirmed.
- **Files in scope**: all declared surfaces touched; two justified extras — `crates/slicer-macros/src/lib.rs`
  (macro derive fallout, anticipated in plan Step 11) and `crates/slicer-scheduler/src/config_resolution.rs`
  (bounds table, named in `design.md` §Files in Scope as "scheduler bounds table (AC-N2)").
- **Out-of-bounds respected**: no edits to renderer modules, `support-planner` legacy module,
  `DEVIATION_LOG.md`, or `OrcaSlicerDocumented/`.
- **F-14 exception preserved**: `move_pass_to_buildplate` keeps raw outlines (`lib.rs:4182`).
- **Schema bump derived-at-activation**: `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 2.0.0→2.1.0
  (`slice_ir.rs:256-260`), no hardcoded future literal.

---

## Implementation Completeness

All 12 steps executed; each AC command green per ceremony. TASK-369..380 registered in
`docs/07_implementation_status.md` with `[~]` (pending human sign-off) — correct terminal
state. Doc Impact Statement greps all pass (`SupportPlanIR`, `schema` in 02_ir_schemas.md;
`support_style` in 15_config_keys_reference.md; `TASK-380` in 07_implementation_status.md).

---

## Findings

### Medium

1. **[MED-1]** AC-14 nonzero `wall_counts` path is untested end-to-end. Every fixture emits
   `wall_counts: vec![0, …]` or `vec![]` (grep across `crates/**` and `modules/**` tests);
   the only assertion is the length invariant (`tree_family_tdd.rs:856-860`). The
   `need_extra_wall → wall_counts ≥ 1` producer branch (`lib.rs:3338-3348`) has no test that
   a merge-point/fast-moving node actually yields a nonzero count. The seam-identity test
   cited in `design.md` §Risks (`view_seam_identity_tdd.rs:14`) uses `SupportPlanIR::default()`
   (empty entries) and does **not** exercise `wall_counts` — that citation is overstated.
   **Impact:** transport is structurally complete and additive-safe (`#[serde(default)]` +
   `Default`), so no crash risk; but 238c (renderer) will consume this field with no proof the
   `≥1` semantic is produced. **Fix (before 238c relies on it):** add one test that drives a
   fixture with a merge point and asserts a nonzero `wall_counts` entry survives both marshal
   legs.

### Low

2. **[LOW-1]** Step 10 exceeded its 3-file edit cap: `crates/slicer-scheduler/src/config_resolution.rs`
   is a 4th file beyond the plan's allowlist (`lib.rs`, `tree_style_styles_tdd.rs`,
   `config_bounds_enforcement_tdd.rs`). Justified (the bounds table lives there and
   `design.md` §Files in Scope names it), but the plan cap was not reconciled. Process note only.

3. **[LOW-2]** Scope hygiene: clippy-driven cosmetic edits in files outside the declared
   surface — import reordering in `finalization_builder_{insert,permute,readback}.rs` and
   `layer_executor_tdd.rs`, indentation fixes in `layer_world_deep_copy_tdd.rs` and
   `live_seam_path_tdd.rs`. These are pre-existing `-D warnings` cleanups, harmless, but
   unrelated to the packet's functional surface.

4. **[LOW-3]** Emit fill matches `need_extra_wall` nodes to skeleton points by exact x/y
   coordinate equality (`lib.rs:3341-3346`). Correct for integer `Point2`, but relies on
   segment endpoints coinciding with node positions; a future refactor that decouples them
   would silently zero the carrier. Consider asserting the match count at emit.

### Note (not a finding)

5. **[NOTE-1]** DEV-142's `DEVIATION_LOG.md` row still describes the pre-fix behavior
   (unconditional 0.0125 mm simplify) while AC-13 changed it to gated simplify. Intentional
   per the packet's doc-hygiene rule (closure edits are implementation-time work, residual #7),
   but the row is now stale and must be updated at closure.

---

## Residuals Confirmation (7 items)

1. **Smoothing = Option A, DEV-143 f64 round-on-commit** — CONFIRMED. `design.md` §The
   Smoothing Decision Point records Option A; `smooth_nodes` uses `Vec<(f64,f64)>`/`Vec<f64>`
   (`lib.rs:584-588`), 100 iterations, round-on-commit. DEV-143 row (`DEVIATION_LOG.md:68`)
   matches.
2. **AC-10 satisfied-by-recorded-boundary** — CONFIRMED. Boundary in `design.md` §Mesh-path
   boundary; `requirements.md` Step-9 outcome note; pinned by
   `analysis_contact_makes_legacy_mesh_projection_unreachable` (`diagnostics_tdd.rs:420`).
3. **Step 10 added a 4th file** — CONFIRMED (see LOW-1). `config_resolution.rs` is the bounds
   table, named in `design.md` §Files in Scope.
4. **No nonzero-wall_counts e2e assertion** — CONFIRMED as a real gap (see MED-1). Transport
   evidence is *partially* sufficient: length invariants (emit + both marshal legs +
   `tree_family_tdd.rs:856`) and emit fill prove the carrier exists and is length-consistent,
   but the seam-identity test does NOT cover `wall_counts` and no test proves the `≥1` value.
5. **Four packet-doc verification-command fixes** — CONFIRMED in place. AC-11 exact test name
   `branch_a_two_leaf_collapse_inherits_parent_roof_counter` (`multi_neighbour_mst_tdd.rs:28`);
   AC-N2 module-qualified filter `config_bounds_enforcement_tdd::rejects_unknown_support_style_value`;
   seam test crate is `slicer-wasm-host` (`view_seam_identity_tdd.rs`); `--no-fail-fast`
   positioned before `--` in all AC commands.
6. **Pre-existing unrelated failures (gcode header width, painted-3MF, modifier infill,
   precision golden)** — CONFIRMED outside surface. `git diff --stat` shows no edits to any
   gcode-header, 3MF, modifier-infill, or precision-golden test file; the only slicer-runtime
   test edits are `wall_counts` struct-literal fallout + the new executor pinning test. These
   failures are not attributable to this packet.
7. **DEV-141..144 rows not edited** — CONFIRMED. `DEVIATION_LOG.md:66-69` rows still `Open`,
   unchanged; consistent with the packet's out-of-scope (closure-time work).

---

## Verification Results

| Check | Result | Details |
|-------|--------|---------|
| AC-1..AC-14, AC-N1..N3 | PASS | `tmp/p238b-ceremony.md` (narrow suites, all green) |
| clippy --workspace --all-targets -D warnings | PASS | ceremony |
| check-literals | PASS | 0 violations |
| build-guests --check | PASS | exit 0 |
| Doc Impact greps | PASS | 4/4 hits |
| Human-gate artifacts | PRESENT | `tmp/p238b-tree-fixture.gcode` (124 Support blocks, delta 0 vs Orca ref), `tmp/p238b-wedge.gcode` (149+7 interface), `tmp/p238b-vd/` (8 PNGs + manifest) |

---

## Verdict

| Level | Decision |
|-------|----------|
| **Critical Issues** | 0 |
| **High Priority Items** | 0 |
| **Medium Priority Items** | 1 (MED-1, nonzero-wall_counts test gap) |
| **Overall Verdict** | **SHIP** |

VERDICT: SHIP

Rationale: every AC traces to file:symbol evidence and passes; no `[unverified]` load-bearing
rows; no Critical/High findings. The single MEDIUM finding (MED-1) is a test-coverage gap on
the AC-14 `≥1` path, explicitly flagged as residual #4 and safe by construction (additive
field with serde default). It should be closed before 238c consumes the transport, but does
not block this packet's closure. Human-gate sign-off remains separately pending by design
(packet stays `draft` until signed).
