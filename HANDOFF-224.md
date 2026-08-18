# Handoff — packet 224 `support-family-orca-closure` (2026-08-18)

Branch `parity/support-planners`, last commit `5a38fdce`. **Nothing is committed** — all work
below is uncommitted in the working tree (20 modified files; see `git status`).

## What this session was

A grilling review of packet 224 followed by execution of an agreed 6-step re-plan. Steps 1 and 2
are complete and verified; Step 3 is complete except one recorded defect. Steps 4–6 are not started.

## Decisions locked with the human (do not relitigate)

| Topic | Decision |
|---|---|
| 224 scope | Closes on correctness + honest tests. Feature gaps route to existing packets/issues |
| RC-4 fix site | Backfill `ActiveRegion.resolved_config` from `RegionMapIR` at plan promotion |
| RC-5 | Consolidate in 224: delete planner self-defaults, de-duplicate the alias table |
| Interface role | Fix in the renderers + restore `is_top_interface` through marshal |
| Tree defects in 224 | double-extrusion, contact tips, swept capsules, and *wire* `smooth_branches` |
| AC-2 | Amend to inspection-only; delete the three dead manifest helpers |
| AC-3 / AC-6 | Amend to invariant test + recorded `/visual-debug` inspection write-up |
| Fixtures | Reuse the OrcaSlicer models already tracked in `resources/`; prefer invariants |
| Orca G-code | Stays inspection-only in gitignored `tmp/`, read by no test |
| AGG rasterizer | Packet **224a**, after 224 closes. Key `support_area_algorithm = grid \| direct`, default `grid` |
| Gap routing | Register `TASK-322`..`TASK-328`; annotate existing drafts/issues; new packets only for the uncovered |
| Packet structure | One packet; re-author `implementation-plan.md` into ~6 S/M steps |

## Completed and verified

### Step 1 — family routing (RC-4 + RC-5)

- `promote_global_layers` + `backfill_active_region_configs`
  (`crates/slicer-runtime/src/layer_executor.rs`) backfill each `ActiveRegion.resolved_config`
  from `RegionMapIR` at the three plan-promotion sites (`pipeline.rs` x2, `run.rs` x1).
- The layer-stage gate's local clone-patch is deleted.
- `canonical_support_family` now lives once in `slicer-ir`; `slicer-scheduler`'s
  `select_support_family` and both planners delegate to it.
- Both planners' self-defaulting family fallbacks are deleted: a planner plans nothing for a
  region the host did not assign it.
- New test `family_reaches_region_routing` (in `support_family_closure.rs`), red first with the
  diagnosed cause, green after.

### Step 2 — interface role end-to-end (RC-6 + RC-7)

- Both renderers stamp `ExtrusionRole::SupportInterface` on interface paths.
- `convert_support_output_with_plan` (`crates/slicer-wasm-host/src/marshal/out.rs`) carries
  `is_top_interface` through the drain, so `SupportRole::BottomInterface` is produced in
  production for the first time.

### Step 3 — geometry (RC-8, RC-9, RC-11 traditional, RC-12, plus two found in flight)

- `tree-support` `render_polygon` rewritten: walls inset half a line width each, fill inset clear
  of them, pitch honours `support_density`, holes respected. The duplicate `fill_expolygon_tree`
  overlay is gone, and the now-dead grid-MST code deleted.
- `structural_body_regions` builds **swept capsules** (convex hull of endpoint circles, unioned)
  instead of detached 16-gon discs; zero-width contact tips now carry a real radius.
- Tree interface is now the node's own area classified as roof/floor (`InterfaceRole`), carved out
  of the body — replacing the bounding-box scan-line hack. `push_interface_scan_lines` deleted.
- `is_roof` band now includes the contact layer (`dist_to_top < top_n`), so the topmost support
  layer is interface rather than bare body.
- `smooth_branches` now translates role regions, not just `skeleton.points[0]`.
- Traditional: interface is carved out of the body instead of duplicating it; `BottomInterface`
  only where the column terminates on the **model** (never the plate); the termination layer
  always prints; top-Z gap derived by walking layer Z.

### Verified end to end through `pnp_cli` (not only tests)

Traditional top interface lands at **Z = 24.8**, exactly OrcaSlicer's contact height in
`tmp/SupportTest_Normal_Orca.gcode`. Interface is now the topmost support with body beneath.
At the Step-2 measurement the `;TYPE:Support interface` count matched Orca exactly for both
families (tree 2 vs 2, normal 3 vs 3).

All 13 support closure + family integration tests pass:

```
cargo test -p slicer-runtime --test integration -- family_reaches_region_routing fixture_invariants \
  final_gcode_roles invalid_geometry_fails matched_height_evidence tree_support_family \
  traditional_support_family support_disabled differential_evidence task_163b supersedes
```

`cargo clippy` clean on all edited crates. **Re-run `cargo xtask build-guests --check` first thing** —
a guest + release rebuild was in flight when this session paused.

## Defects found and fixed that were NOT in the packet

- **RC-6** no production code constructed `ExtrusionRole::SupportInterface`; the marker was
  unreachable and `support_interface_speed` was never applied.
- **RC-7** `is_top_interface` was discarded in marshal; `SupportRole::BottomInterface` never existed.
- **RC-8** tree renderer emitted `wall_count` *coincident* walls + 100% fill + a second grid-MST fill.
- **RC-9** contact tips were created with `width = 0.0` and filtered out — the layer meeting the
  overhang printed nothing.
- **RC-12** traditional emitted bottom interface on the build plate.
- **`body_overlaps_occupancy`** ended with `point_in_polygon(closest_boundary_point)`, decided by
  floating-point accident, reporting "overlapping" for a body 8 mm clear. Pinned by the new
  `body_clear_of_occupancy_does_not_overlap`.
- **Termination layer was droppable** when it failed the support-layer-height modulo, so columns
  stopped short of the plate.

## Open — pick up here

1. **RC-11 tree top-Z gap (OPEN, precisely characterised).** Tree still ignores
   `support_top_z_distance_mm`; its top interface lands at Z = 25.0 with the overhang underside also
   at 25.0 — zero gap. Traditional is fixed and Orca-matching; tree is not.
   I implemented a shift in `push_contact_with_demand` and **reverted it**, because it demonstrably
   had no effect (identical output with and without) while the config *value* still changed the
   result by 35 layers. That contradiction is unexplained and I would not ship it.
   Measured, deterministic across repeat runs: with `support_top_z_distance_mm = 0` tree yields 125
   entries, top layers 124/123 = `TopInterface` (correct shape); with 0.2 it yields 90 entries
   topping at 89, all `SupportBody`; with 0.4, 89 entries topping at 88. An unrelated key does not
   perturb it. **Find the real consumer of that key in the tree path before changing anything.**
   Note `eprintln!` from guest code does not reach the test harness — use `push_diagnostic`, which does.

2. **`LayerPlanViewEntry.effective_layer_height` is unreliable in the guest view.** Dividing by it
   produced a zero-layer gap in traditional and a 35-layer gap in tree. Both planners now avoid it;
   the field itself should be investigated or documented as untrustworthy.

3. **`benchy_orca_parity_within_tolerance` is RED on purpose** (Hausdorff 1.2998 mm vs 0.5 tolerance).
   It compares against a golden whose own header says "Source: Pinch 'n Print self-capture ... Replace
   with real OrcaSlicer reference data". Per `CLAUDE.md` I left it red rather than regenerating.
   `SUPPORT_PLANNER_REGEN_GOLDEN=1` is the sanctioned regeneration path **if the human approves**.

4. **RC-3 is unverified.** No host code filters `SupportPlanIR` *entries* by family — region routing
   is the only guard. `design.md` is corrected; the `n:<family>` qualifier it described never existed.

5. **Step 4 not started** — delete the theatre tests (`differential_evidence` and
   `task_163b_disposition` still contain empty `if` blocks; `missing_fixture_is_blocking` still tests
   `std::fs`; the three `#[allow(dead_code)]` manifest helpers still have zero callers) and add
   invariant tests on `resources/` models: `cube_with_concave_hole_enlarged_standing.obj` (wall
   leakage), `two_hollow_squares.obj` (multi-island), `V_standing.obj` (branch merging),
   `A_upsidedown.obj` (sharp tail).

6. **Step 5 not started** — amend AC-2/AC-3/AC-6 in `packet.spec.md` with the amendment recorded
   verbatim (as AC-N2 was), record RC-6..RC-12 in `design.md`, and correct the false claim that
   `missing_fixture_is_blocking` was deleted. `design.md` sections RC-3 and RC-4 are already corrected.

7. **Step 6 not started** — `cargo xtask check-literals`, then
   `cargo xtask test --summary --workspace` dispatched to a sub-agent for a FACT pass/fail.

8. **Ledger** — `TASK-322`..`TASK-328` are declared in packets 213–218 front matter but do not exist
   in `docs/07_implementation_status.md`. `TASK-335` stays unchecked.

## Known-unrelated, do not attribute to this work

`ERR_MALFORMED_LAYER_MARKER` (code 12) from `machine-gcode-emit` fires **107 times with support
disabled** on the decisive fixture, 118 with tree support. Pre-existing, outside 224's scope, untouched.

`tree_support_family` also trips a `boostvoronoi` assertion (`rhs.fpv_.is_finite()`) in a worker
thread; the test passes regardless. Not investigated.

## Still true after all this work

224 will close as *correct*, not as *canonically complete*. `support_bottom_z_distance`,
`support_expansion`, base patterns, interface patterns and raft geometry remain unimplemented.
The Orca `normal` reference emits **205 distinct print Z** for a 150-layer print (55 support-only
layers at independent Z); PnP emits 150. That gap belongs to the follow-up packets.
