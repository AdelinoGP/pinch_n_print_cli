# Implementation Plan: support-family-orca-closure

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-335`.
- **Sequencing for the two steps added 2026-08-18:** Steps 3a and 3b are written beside the Step 3 diagnosis they descend from, but they **execute after Step 5 and before Step 6**, and **3a precedes 3b** (RC-A discards plan entries downstream of contact generation, so the RC-15 port cannot be measured until it is fixed).
- Use TDD, then implementation, then narrow falsifying validation.
- Never claim parity from uninspected or self-captured goldens.
- **No test may read `tmp/SupportTest_Tree_Orca.gcode` or `tmp/SupportTest_Normal_Orca.gcode`**, and no Orca-derived constant may be hardcoded into a test. Parity gating is structural invariants plus the written inspection checklist.
- **Extruding-move counts are not a parity metric** (Orca tree segments are ~15x shorter). Do not gate on them or quote them as evidence.
- Run `cargo xtask build-guests --check` after editing any `modules/core-modules/*/src/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`, or `crates/slicer-core/**` path, and rebuild before trusting any measurement. Stale-guest artifacts already caused one recorded false diagnosis (see `design.md` §Root Causes RC-11).
- `eprintln!` from guest code does not reach the test harness; use `push_diagnostic`.
- **Never filter the closure suite with the bare token `support_family_closure`.** The closure tests are bare `#[test] fn` wrappers in `crates/slicer-runtime/tests/integration/main.rs` with no module prefix, so that filter matches **zero** tests and reports a green run with everything filtered out. Always name the tests explicitly with `-- <name> ... --exact`.
- Every canonical feature gap discovered mid-flight is **registered and routed** (`docs/specs/support-parity-gap-register.md`, unnumbered stubs under `docs/spec_packets/stubs/`), never implemented here.

## Steps

### Step 0: Protect and re-baseline
- Task IDs: `TASK-335`
- Objective: get the existing uncommitted work onto a commit, prove guest freshness, derive a matched Orca config fixture, re-measure the baseline against the regenerated Orca references, and void the stale handoff numbers.
- Precondition: the working tree carries the uncommitted packet-224 work described in `HANDOFF-224.md`; branch `parity/support-planners`.
- Postcondition: work is committed; `cargo xtask build-guests --check` is clean; a matched PnP config fixture reproducing the recorded Orca `normal` profile exists; the dated baseline table in `design.md` §Measured Baseline is confirmed or corrected in place; `HANDOFF-224.md` carries a header stating its numbers are void.
- Files allowed to read: `HANDOFF-224.md`; `git status` / `git diff --stat` output; `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files allowed to edit: `HANDOFF-224.md` (void annotation only); `docs/spec_packets/224-support-family-orca-closure/design.md` (§Measured Baseline only); one matched-config fixture file under `crates/slicer-runtime/tests/fixtures/support-family/`.
- Files explicitly out of bounds: all `modules/core-modules/**/src/**`; all `crates/**/src/**`; `packet.spec.md`.
- Expected sub-agent dispatches: Question: re-measure the five baseline metrics for both families from freshly emitted PnP G-code and the two Orca references; scope: `target/**`, `tmp/*.gcode`; return: `FACT` table only.
- Context cost: `S`
- Verification: `cargo xtask build-guests --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`.
- Exit condition: a commit exists containing the prior verified work, the guest check is clean, and the baseline table is dated and reproducible.

**Status 2026-08-18 — DONE (`4d245486`).** Gates were run, the prior uncommitted work was committed, and a matched Orca config fixture was derived under `crates/slicer-runtime/tests/fixtures/support-family/`. Carry-forward: that fixture is now wired into `run_slice_for_family` (`crates/slicer-runtime/tests/integration/support_family_closure.rs`) by `4c67ccd9`, so both families slice against the matched profile rather than defaults.

### Step 1: Tree top-Z gap (RC-11)
- Task IDs: `TASK-335`
- Objective: make `tree-support-planner` read `support_top_z_distance_mm` and honour it by shifting the contact layer along actual layer Z.
- Precondition: Step 0 committed and guest check clean. `from_config` currently reads 17 keys and not this one; the key is absent from `crates/slicer-schema/wit/`.
- Postcondition: a red-first test pins the gap between the overhang underside and the topmost tree support layer for at least two distinct config values; the contact shift is derived by **walking actual layer Z** (the technique `traditional-support-planner::plan_for_object` uses); `effective_layer_height` is never used as a divisor in the new code.
- Files allowed to read: `modules/core-modules/tree-support-planner/src/**`; `modules/core-modules/traditional-support-planner/src/lib.rs` (`plan_for_object` only); `modules/core-modules/tree-support-planner/tree-support-planner.toml`.
- Files allowed to edit (3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; the tree-support-planner manifest if the declared key needs correcting.
- Files explicitly out of bounds: `modules/core-modules/traditional-support-planner/src/**` (read-only); `crates/slicer-schema/wit/**`; both renderers.
- Expected sub-agent dispatches: Question: describe canonical tree contact-gap placement; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: `cargo test -p tree-support-planner`; then `cargo xtask build-guests --check` (rebuild if stale) and `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`.
- Exit condition: the red-first test is green, the gap changes with the config value, and every measurement was taken after a clean guest check.

**Status 2026-08-18 — DONE (`d97fb2b8`).** `tree-support-planner` now reads and honours `support_top_z_distance_mm`, shifting the contact layer by walking actual layer Z.

### Step 2: Interface layer counts in both families
- Task IDs: `TASK-335`
- Objective: diagnose and fix why `support_interface_top_layers` / `support_interface_bottom_layers` are not honoured, so both families emit the configured number of interface layers.
- Precondition: Step 1 landed. Measured blocker: PnP normal emits 1 `;TYPE:Support interface` block against Orca's 3 at `top_layers=2` / `bottom_layers=2` (see `design.md` §Measured Baseline).
- Postcondition: the counts are honoured by both families; a permanent test asserts the interface layer count as a function of the two config keys (not against a hardcoded Orca number); interface remains carved out of the body and is never duplicated with it.
- Files allowed to read: both planners' `src/**`; both renderers' `src/**`; `crates/slicer-wasm-host/src/marshal/out.rs`.
- Files allowed to edit (3): `modules/core-modules/tree-support/src/lib.rs`; `modules/core-modules/traditional-support/src/lib.rs`; `crates/slicer-runtime/tests/integration/support_family_closure.rs`. If the defect proves planner-side, substitute the owning planner `src/lib.rs` for one renderer file and record the substitution.
- Files explicitly out of bounds: `crates/slicer-schema/wit/**`; interface **pattern** generation (packet 226); `support_bottom_z_distance` (packet 226).
- Expected sub-agent dispatches: Question: canonical roof/floor layer-count semantics, including the negative-`bottom_interface_layers` fallback to `support_interface_top_layers`; scope: `TreeSupport.cpp`, `SupportMaterial.cpp`, `SupportCommon.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`; `cargo xtask build-guests --check`.
- Exit condition: configured interface layer counts are produced by both families and pinned by a test that fails when the keys are ignored.

**Status 2026-08-20 — DONE (`ee27ac94`).** Interface counts are exact 1/2/3 in both families, pinned by `interface_layer_count_follows_config` through a real `run_slice` plus the `bottom = -1` fallback (session-3 audit: sound, strengthening). The count follows the configured top band; the remaining 2-vs-3 difference against Orca at `top=2`/`bottom=2` is canonical roof/floor band structure, registered as gap G-18.

### Step 3: Tree density diagnosis (read-only)
- Task IDs: `TASK-335`
- Objective: produce a written, evidence-backed root cause for the tree coverage deficit measured on **deposited** material: PnP tree **388.73 mm** against Orca tree **683.96 mm** = **56.8%**, a **1.76x** deficit, over an identical Z range and layer count. (The superseded figures 31.6% / 486.33 mm / 1538.36 mm are **void** — they summed de-retraction prime `E`, which deposits nothing; see `design.md` §Measured Baseline and `tree-density-diagnosis.md`. Do not requote them.)
- Precondition: Steps 1-2 landed and re-measured. This step is **read-only** and changes no production code.
- Postcondition: a root-cause write-up in `design.md` that explicitly eliminates or convicts each of: fill pitch versus `support_base_pattern_spacing`; wall count; branch radius; branch count. Only after the cause is known is the finding classified **bug** (fixed in a follow-up step of this packet) or **gap** (registered in Step 7). Classifying before diagnosis is prohibited.
- **Outcome (2026-08-18): diagnosis COMPLETE.** Root cause is **RC-15** — `tree-support-planner` derives contact points from **mesh overhang-triangle centroids, one per triangle**, so a ~400 mm² overhang made of two triangles yields **2** contact points and branch density is bounded by the input file's tessellation. Canonical `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) never touches triangles: it samples the per-layer overhang `ExPolygon` three independent ways (contour corners, arc walk along contour and holes, rotated interior grid) and unions them. Classified **GAP**, agreed to be implemented in this packet (Step 3b), not routed to the gap register. See `design.md` §Root Causes RC-15 and `tree-density-diagnosis.md`.
- Files allowed to read: `modules/core-modules/tree-support/src/**`; `modules/core-modules/tree-support-planner/src/**`; emitted PnP G-code and the two Orca references under `tmp/` (inspection only, via sub-agent).
- Files allowed to edit (1): `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files explicitly out of bounds: every `src/**` path (this step edits no code); `packet.spec.md`.
- Expected sub-agent dispatches: Question: per-layer support extrusion length and line spacing for PnP tree versus Orca tree at three matched heights; scope: emitted G-code plus `tmp/*.gcode`; return: `FACT`. Question: canonical tree base-area density derivation; scope: `TreeSupport.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: no cargo change required; re-measure only with `cargo xtask build-guests --check` clean before quoting any number.
- Exit condition: each of the four candidate causes is eliminated or convicted with a measurement, and a bug-versus-gap classification is recorded with its basis.

**Status 2026-08-18 — DONE (`4c67ccd9`), diagnosis only.** The diagnosis is complete; the root cause is **RC-15** (tree contacts derived from mesh overhang-triangle centroids), classified **GAP** and **agreed to be implemented in packet 224** (Step 3b). The port itself is **not yet implemented**.

### Step 3a: RC-17 tree-regression punch list (added 2026-08-18)
- Task IDs: `TASK-335`
- Objective: clear the eight tree-family regressions introduced by commit `9f4540bd`, in the order established by the read-only audit in `tree-regression-punch-list.md`: RC-A (family-assignment gate, 5 tests, one fix) first, then RC-B (renderer fill/density/direction, 2 tests), then RC-C (self-captured golden drift, 1 test), then the two inherited failures recorded there as RC-D/RC-E.
- Precondition: Steps 0-5 landed. `tree-support-planner` and `tree-support` went from **3 failures at `5a38fdce`** to **11 at `9f4540bd`** and **10 at HEAD**; seven of the eight introduced failures sit in test files that are byte-identical across that window. Reproduce with `--no-fail-fast` - without it Cargo stops after `orca_parity_tdd` and only 2 of the 10 are visible.
- Postcondition: RC-A's silent drop is fixed **in production** (`SupportPlanner::plan_for_object` falls back to the module's own configured `support_family` when no assignment matches, and emits a diagnostic when the fallback fires) - the fixtures are **not** migrated, because they are the only coverage of the no-assignment path; RC-B's percent-versus-fraction bug is fixed (`support_density` is read without a `/100.0` and then clamped by `.min(1.0)`, saturating every percent value to a solid fill); each remaining item carries the audit's verdict (TEST-IS-RIGHT / TEST-IS-STALE / TEST-IS-WRONG) and either a code fix or a recorded retarget. **No assertion is weakened** - the audit found zero tests requiring it.
- Files allowed to read: `tree-regression-punch-list.md`; `tree-failure-attribution.md`; `modules/core-modules/tree-support-planner/**`; `modules/core-modules/tree-support/**`.
- Files allowed to edit (3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support/src/lib.rs`; the owning tree test file for each retargeted assertion.
- Files explicitly out of bounds: `resources/golden/**` (regenerated only in Step 8, after Step 3b lands - regenerating now bakes in a second wrong algorithm); `crates/slicer-schema/wit/**`; the traditional family.
- Expected sub-agent dispatches: Question: re-run both tree crates with `--no-fail-fast` and report per-binary pass/fail counts before and after the RC-A fix; scope: `modules/core-modules/tree-support*`; return: `FACT` counts only.
- Context cost: `M`
- Verification: `cargo test -p tree-support-planner --tests --no-fail-fast`; `cargo test -p tree-support --tests --no-fail-fast`; `cargo xtask build-guests --check`; `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition support_never_intersects_model_at_exact_z accepted_demands_terminate_on_plate_or_model interface_is_topmost_and_carved_out no_overhang_mesh_produces_zero_support --exact`.
- Exit condition: RC-A is fixed once and the five tests it masked are re-run **before** any of them is separately "fixed"; every remaining punch-list item is green or carries a written verdict; binary counts are compared across runs, so no failure is hidden by an early abort.

**Status 2026-08-20 — DONE (`39507cff`, `0629a9b5`, `868508ba`, `ed62090d`; session-3 audit).** RC-A fixed in production with a diagnostic on fallback; RC-B percent/fraction fixed; RC-C left red (golden regenerated in Step 8); RC-D/RC-E carried the audit's verdicts. `tree-support-planner` 8 binaries with only RC-C red; `tree-support` 27/27 (now 26/26 after the Step 7 deletion of the vacuous `enforcer_overrides_needs_support_false`).

### Step 3b: RC-15 contact-point-sampling port (added 2026-08-18)
- Task IDs: `TASK-335`
- Objective: replace `tree-support-planner`'s mesh-overhang-triangle-centroid contact derivation with the 2D, slice-based sampling recorded in `design.md` Root Causes RC-15, so branch density is a function of the support settings rather than of the input file's tessellation.
- Precondition: Step 3 recorded RC-15, and Step 3a's RC-A fix has landed (RC-A discards plan entries downstream of contact generation, so the port is unmeasurable until it is fixed). Measured today: PnP emits **2** closed loops at every Z within an **8.2 mm** footprint, while Orca fans out **2 -> 3 -> 4 -> 14 -> 58** loops to a **19.1 x 20.3 mm** footprint at `z = 24`.
- Postcondition: contacts are derived from the per-layer overhang `ExPolygon` by the three canonical streams of `TreeSupport::generate_contact_points` (`TreeSupport.cpp`), unioned and deduped through a hash-bucket grid of cell size `base_radius`: (1) contour corners, taken where the two incident edge directions satisfy `v1.dot(v2) > -0.7`; (2) an `EdgeCache` arc walk emitting points at `point_spread = tree_support_branch_distance` along the contour **and along every hole**; (3) an interior grid rotated 22 degrees at `sample_step = max(point_spread, max_bridge_length / 2)`, kept where the point falls inside the overhang eroded by `base_radius`. The global (not per-island) grid origin and the rotation are load-bearing - they are what keep the sampling from aliasing with axis-aligned model features. A red-first test pins contact count growing with overhang area rather than with triangle count.
- **Blocking hazard (from `tree-regression-punch-list.md`; read before writing code).** All six planner fixtures are **flat coplanar horizontal plates at `z = 1.8`** with an unreferenced `[0,0,0]` vertex present only to set `bmin[2]`; they have an **empty cross-section at every Z**. They work today only because the planner never slices (`detect_overhang_facets` reads `MeshObjectView.triangles` directly). If the port derives its `ExPolygon` by slicing (canonical `curr_layer - offset(prev_layer)`), every fixture yields an empty polygon, the port produces zero contacts, and five green tests go red for a reason that looks like the port is wrong. Either project the downward-facing triangles onto the layer plane, or rebuild the fixtures as closed solids first - decide this before writing code.
- Files allowed to read: `modules/core-modules/tree-support-planner/src/**`; `design.md` RC-15; `tree-density-diagnosis.md`; `tree-regression-punch-list.md`.
- Files allowed to edit (3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; one further tree-planner test file if the red-first test belongs there.
- Files explicitly out of bounds: both renderers' `src/**`; `resources/golden/**` (the port raises contacts from 2 to tens on the same fixture and will move the branch count - regenerate once, in Step 8); `crates/slicer-schema/wit/**`.
- Expected sub-agent dispatches: Question: describe `generate_contact_points`' three sampling streams, their dedup grid, and the grid rotation and origin; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY`.
- Context cost: `M`
- Verification: `cargo test -p tree-support-planner --tests --no-fail-fast`; `cargo xtask build-guests --check`; `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition support_never_intersects_model_at_exact_z accepted_demands_terminate_on_plate_or_model interface_is_topmost_and_carved_out no_overhang_mesh_produces_zero_support --exact`; then re-measure the deposited-material and support XY-path-length rows in `design.md` Measured Baseline.
- Exit condition: contact count is driven by overhang area and `tree_support_branch_distance` rather than by triangle count, the loop-count and footprint fan-out move measurably toward the Orca figures above, and every number was taken after a clean guest check.

**Status 2026-08-20 — DONE (`ad9019ee`).** The three canonical sampling streams (contour corners, arc walk, 22°-rotated interior grid over the whole-object bbox) landed with the collision-gate narrowing reverted (bisect-confirmed: narrowing alone = closure 9/12, sampling alone = 12/12). Closure 12/12; planner crate 8 binaries with only RC-C red after the fixture/config migrations (no assertion weakened; the `radius_aware_collision` floor retarget to 0.3 was human-approved). Re-measured: tree deficit 1.76x → 1.58x deposited (432.85 vs 683.96 mm), 1.949x → 1.75x XY path (13,013.9 vs 22,774.9 mm); `design.md` §Measured Baseline updated.

### Step 4: Config-key reconciliation (four support modules)
- Task IDs: `TASK-335`
- Objective: reconcile declared-versus-read config keys across `tree-support-planner`, `traditional-support-planner`, `tree-support`, and `traditional-support` only.
- Precondition: Steps 1-3 landed. Known dead keys: raft keys and `support_base_pattern` — these **stay as-is**, recorded rather than wired or removed.
- Postcondition: for each of the four modules, every manifest-declared key is either read by that module or listed as a dead key with its routing packet; the list lands in the gap register (create the stub here if Step 7 has not run).
- Files allowed to read: the four modules' `src/**` and `*.toml`; `docs/15_config_keys_reference.md`.
- Files allowed to edit (3): the four modules' `*.toml` manifests (only where a key must be corrected) and `docs/specs/support-parity-gap-register.md`.
- Files explicitly out of bounds: `xtask/**` — **no new xtask gate is introduced by this packet**; any module outside the four; `crates/slicer-schema/wit/**`.
- Expected sub-agent dispatches: Question: enumerate declared-versus-read config keys for the four support modules; scope: those four module directories; return: `LOCATIONS`.
- Context cost: `S`
- Verification: `cargo xtask build-guests --check`; `cargo test -p tree-support-planner`; `cargo test -p traditional-support-planner`.
- Exit condition: the declared/read/dead classification is complete for all four modules and every dead key names its routing packet.

**Status 2026-08-18 — DONE (`4d1848eb`).** Config keys reconciled across the four support modules (`tree-support-planner`, `traditional-support-planner`, `tree-support`, `traditional-support`).

### Step 5: Honest tests
- Task IDs: `TASK-335`
- Objective: delete the tests that assert nothing and replace them with real invariants on tracked models.
- Precondition: Steps 1-4 landed.
- Postcondition: deleted — the empty `if` blocks in `differential_evidence` and `task_163b_disposition`, the `missing_fixture_is_blocking` test, and the three `#[allow(dead_code)]` manifest helpers (`read_manifest`, `manifest_images`, `layer_indices`). Added — four invariants, each exercised across `resources/cube_with_concave_hole_enlarged_standing.obj`, `resources/two_hollow_squares.obj`, `resources/V_standing.obj`, and `resources/A_upsidedown.obj`:
  1. **Exact-Z non-intersection** — no support body or nozzle sweep intersects model occupancy at exact Z.
  2. **Termination** — every accepted demand terminates on the build plate or on the model.
  3. **Interface placement** — interface is the topmost support and is carved out of the body, never duplicated with it.
  4. **No-overhang null result** — a mesh with no overhang produces zero support entries.
- Files allowed to read: `crates/slicer-runtime/tests/integration/**`; the `resources/` model listing.
- Files allowed to edit (3): `crates/slicer-runtime/tests/integration/support_family_closure.rs`; `crates/slicer-runtime/tests/integration/main.rs`; one family integration test file if an invariant belongs there.
- Files explicitly out of bounds: all `src/**`; `tmp/*.gcode` (no test may read them).
- Expected sub-agent dispatches: Question: confirm the four `resources/` models are tracked and load through the real slice path; scope: `resources/**`; return: `FACT`.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition support_never_intersects_model_at_exact_z accepted_demands_terminate_on_plate_or_model interface_is_topmost_and_carved_out no_overhang_mesh_produces_zero_support --exact`; `cargo xtask check-literals`.
- Exit condition: no closure test contains an assertion-free branch or a dead helper, and each of the four invariants fails when its guard is inverted.

**Status 2026-08-18 — PARTIAL (`4c67ccd9`, `4d1848eb`).** The assertion-free theatre was deleted and the four invariants were added. `interface_is_topmost_and_carved_out` is **RED**, on a genuine tree-planner defect: an interface region is planned at layer 119 above a column whose geometry ends at layer 79. The red is a real defect, not a fixture problem — do not weaken the assertion to clear it.

### Step 6: Inspection gate
- Task IDs: `TASK-335`
- Objective: replace the broken AC-2 command with two genuine per-family visual-debug requests, render Orca G-code side by side at matched heights, and write the inspection checklist.
- Precondition: Steps 1-5 landed; both families emit support G-code. The current AC-2 command renders **one family twice**.
- Postcondition: two distinct request fixtures (`tmp/visual-debug-support-family-tree.json`, `tmp/visual-debug-support-family-normal.json`) replace the single request rendered twice; each family and its Orca counterpart are rendered at the same physical heights; `design.md` §Orca Inspection Checklist is written with per-axis inspected verdicts (termination, coverage, collision freedom, interface placement/count, independent heights), each naming its layer and tap.
- Files allowed to read: `crates/pnp-cli/src/visual_debug.rs` (bounded ranges); existing visual-debug tests; generated manifests via delegated inspection only.
- Files allowed to edit (3): `tmp/visual-debug-support-family-tree.json`; `tmp/visual-debug-support-family-normal.json`; `docs/spec_packets/224-support-family-orca-closure/design.md`.
- Files explicitly out of bounds: `packet.spec.md` (AC text is amended in Step 7); generated PNGs as source edits; any test reading the Orca G-code.
- Expected sub-agent dispatches: Question: inspect matched-height PNGs for both families against the Orca renders and report per-axis verdicts; scope: `target/vd-support-family-*`; return: `FACT` plus manifest paths.
- Context cost: `M`
- Verification: `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family-tree.json --output target/vd-support-family-tree --overwrite`; the same for the `-normal` request into `target/vd-support-family-normal`; `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition support_never_intersects_model_at_exact_z accepted_demands_terminate_on_plate_or_model interface_is_topmost_and_carved_out no_overhang_mesh_produces_zero_support --exact`.
- Exit condition: two per-family requests exist and render distinct output, side-by-side Orca renders were inspected, and the checklist records a verdict per axis.

**Status 2026-08-20 — DONE (`8cb60b91`).** Both per-family requests and both Orca G-code requests rendered at matched heights (layers 10/30/79/119/123 = z 2.0..24.8 mm); `matched_height_evidence` passes; `design.md` §Orca Inspection Checklist written with a verdict per axis, each naming its layer and tap. One honest DIVERGENT verdict (traditional interface count 2 vs Orca 3) registered as gap G-18. Request-fixture notes: layer 124 dropped (no support there in three of four files), tree request `filament_lines`-only (skeleton paths carry no width; renderer fails closed), Orca G-code requests carry `gcode_line_width_mm: 0.4`.

### Step 7: Paperwork — ACs, gap register, packet stubs, docs/07
- Task IDs: `TASK-335`
- Objective: amend the acceptance criteria to the delivered gate, create the gap register, stub the follow-on packets, and update implementation status.
- Precondition: Steps 0-6 landed with their evidence recorded.
- Postcondition: `packet.spec.md` AC-2/AC-3/AC-6 amended with the amendment recorded verbatim in place (the style used for AC-N2), pointing at the two per-family commands and the inspection-checklist gate, AC-2's fixture path corrected from `tmp/SupportTest.stl` to the authoritative `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`, and the stale claim about `missing_fixture_is_blocking` corrected to match Step 5's actual deletion; `docs/specs/support-parity-gap-register.md` exists listing every routed gap with its owning packet; stubs exist for the AGG rasterizer / `support_area_algorithm`, independent support-layer Z, base/interface patterns + `support_expansion` + `support_bottom_z_distance`, raft, and `needs_support` eligibility classification (unnumbered, under `docs/spec_packets/stubs/` — the previously named 224a/225/226/227 are taken by unrelated packets); `docs/07_implementation_status.md` carries the `TASK-335` row and the follow-on rows.
- Files allowed to read: `packet.spec.md`; `docs/07_implementation_status.md`; the gap notes produced in Steps 3-4.
- Files allowed to edit: `docs/spec_packets/224-support-family-orca-closure/packet.spec.md`; `docs/specs/support-parity-gap-register.md`; the four packet stubs; `docs/07_implementation_status.md` (via delegated status worker).
- Files explicitly out of bounds: all `src/**` and all test files.
- Expected sub-agent dispatches: Question: update `docs/07_implementation_status.md` rows for `TASK-335` and the follow-on packets; scope: that file; return: `SUMMARY`.
- Context cost: `S`
- Verification: `rg -q 'TASK-335' docs/07_implementation_status.md`; `rg -q 'support-agg-rasterizer' docs/specs/support-parity-gap-register.md`; `cargo xtask check-deviations` if any deviation row was filed.
- Exit condition: no gap named in this packet is unrouted, and every AC reads as the gate actually delivered.

**Status 2026-08-20 — DONE.** `needs_support` eligibility gap filed (G-17, decision 2) and the vacuous `enforcer_overrides_needs_support_false` deleted from `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` (tree-support 26/26). Gap register updated: G-01 marked implemented, G-17/G-18 added, destinations renamed to the five unnumbered stubs under `docs/spec_packets/stubs/` (human decision: no numbers). Stale status lines corrected in `packet.spec.md` and `implementation-plan.md` (Steps 2/3a/3b/6). `docs/07_implementation_status.md` carries the TASK-335 state note and the five stub rows.

### Step 8: Close
- Task IDs: `TASK-335`
- Objective: regenerate the benchy golden last and run the full acceptance gates.
- Precondition: Steps 0-7 complete and **no further production change is pending**. The golden is regenerated only now, after all fixes.
- Postcondition: the benchy golden is regenerated via the sanctioned path (`SUPPORT_PLANNER_REGEN_GOLDEN=1`) with human approval, **renamed off `orca_parity`** to a regression-tripwire name, and carries a provenance header stating it is a PnP self-capture and **not** parity evidence; the workspace gates pass.
- Files allowed to read: the golden fixture and its owning test.
- Files allowed to edit (3): the benchy golden fixture; its owning test file (rename plus provenance); `docs/spec_packets/224-support-family-orca-closure/design.md` (closure note).
- Files explicitly out of bounds: everything else; no production `src/**` change may land in this step.
- Expected sub-agent dispatches: Question: run `cargo xtask test --summary --workspace`; scope: workspace; return: `FACT pass/fail` plus failing-test names only. Never absorb the full output.
- Context cost: `M`
- Verification: `cargo xtask check-literals`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask test --summary --workspace` (sub-agent, FACT return).
- Exit condition: all three gates pass and the renamed golden's provenance header disclaims parity.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | protect, guest check, re-baseline |
| Step 1 | M | tree `support_top_z_distance_mm` (RC-11) |
| Step 2 | M | interface layer counts, both families |
| Step 3 | M | tree density diagnosis (read-only) |
| Step 3a | M | RC-17 tree-regression punch list (8 regressions from `9f4540bd`) |
| Step 3b | M | RC-15 contact-point-sampling port |
| Step 4 | S | config-key reconciliation, four modules |
| Step 5 | M | honest tests on tracked `resources/` models |
| Step 6 | M | inspection gate plus checklist |
| Step 7 | S | ACs, gap register, packet stubs, docs/07 |
| Step 8 | M | golden regeneration plus workspace gates |

No step is rated `L`.

## Packet Completion Gate
- All eleven steps complete with their exit conditions met (Steps 0-8 plus Steps 3a and 3b, added 2026-08-18).
- **Correctness closure, not canonical completeness.** The packet closes when tree honours `support_top_z_distance_mm`, both families honour the interface layer-count keys, the tree-density root cause is written down and classified, the four support modules' config keys are reconciled, and the closure suite contains no assertion-free test or dead helper.
- **Parity gate:** structural invariants plus the written `/visual-debug` inspection checklist with side-by-side Orca renders, recorded in `design.md` §Orca Inspection Checklist. No test reads the Orca G-code. Extruding-move counts are not evidence.
- Every routed gap (base/interface patterns, `support_expansion`, `support_bottom_z_distance`, raft, independent support-layer Z, the AGG rasterizer, the dead raft/`support_base_pattern` keys, `needs_support` eligibility, and the roof/floor layer-count semantics) appears in `docs/specs/support-parity-gap-register.md` against an owning stub under `docs/spec_packets/stubs/`.
- `packet.spec.md` AC-2/AC-3/AC-6 amended in place with the amendment recorded verbatim; every remaining AC command returns PASS as written.
- Benchy golden regenerated **last**, renamed off `orca_parity`, provenance header disclaiming parity.
- `cargo xtask check-literals`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask test --summary --workspace` (sub-agent, FACT return) all pass.
- Assert packet 213 / `TASK-329` supersession with closure evidence, and either close `TASK-163b-orca-ref` using the existing references or record its precise external blocker.
- `docs/07_implementation_status.md` updated through a worker dispatch.

## Acceptance Ceremony
- Re-dispatch every AC and packet-level gate command.
- Inspect matched-height PNGs and manifest provenance side by side with the Orca renders — not just file existence.
- Confirm every number quoted in `design.md` was measured after a clean `cargo xtask build-guests --check`.
- Record remaining external and inherited blockers, and confirm each routed gap has a named owning packet.
