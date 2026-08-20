# Design: support-family-orca-closure

## Controlling Code Paths
- Primary code path: `slicer-runtime` real-slice closure test, `pnp_cli visual-debug` model/G-code requests, manifest evidence index, and final G-code role parser.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/main.rs`; existing visual-debug tests under `crates/pnp-cli/tests/`; existing SupportTest model and Orca references; comparison bundles `target/vd-orca-tree-compare` and `target/vd-orca-normal-compare`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints
- Closure proves behavioral parity only: coverage, termination, collision freedom, interfaces, independent heights, and printable construction; exact path identity is out of scope.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` are separate evidence boundaries and must all be captured. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- The existing decisive fixtures are the primary closure path. A deliberately missing copied path is reserved for the negative gate.
- Final evidence must not treat PNG existence, byte size, manifest greps, or self-captured goldens as proof.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface
- Selected approach: fixture-driven runtime assertions plus visual-debug request/evidence generation and final-G-code role inspection.
- Exact functions, tests, and fixtures: existing typed capture and G-code visual-debug paths; new closure integration test and manifest/evidence fixture requests; fixture production script/process if approved.
- Rejected alternative: accepting stale self-captured goldens, because the plan explicitly requires regenerated inspected differential evidence.

## Files in Scope (read + edit)
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - real fixture invariants and role tests.
- `crates/slicer-runtime/tests/integration/main.rs` and `crates/slicer-runtime/Cargo.toml` - planned closure module and single `integration` Cargo test target registration.
- `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` - decisive fixture, tracked in-repo (authoritative path; `tmp/` copies are not authoritative).
- `tmp/visual-debug-support-family-tree.json` and `tmp/visual-debug-support-family-normal.json` - per-family visual-debug request fixtures.
- `modules/core-modules/traditional-support-planner/src/` - contact/base-layer split (see §Root Causes, RC-1).
- `modules/core-modules/tree-support-planner/src/` - distinct interface roles (see §Root Causes, RC-2).
- Host support-view marshalling in `crates/slicer-wasm-host/` - per-family plan-entry routing (see §Root Causes, RC-3).
- `modules/core-modules/tree-support/src/` - **renderer, in scope.** Already modified in this packet (RC-8, RC-9, and the interface carve-out); further interface-layer-count work under Step 2 touches it.
- `modules/core-modules/traditional-support/src/` - **renderer, in scope.** Already modified in this packet (RC-12 and the interface carve-out); further interface-layer-count work under Step 2 touches it.
- The four support modules' manifests (`tree-support-planner.toml`, `traditional-support-planner.toml`, `tree-support.toml`, `traditional-support.toml`) - config-key reconciliation only (Step 4).

`tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode` are **inspection aids, not fixtures.** They exist to let a human or agent close this family of packets against a proven OrcaSlicer comparison. No test may read them, and no Orca-derived constant may be hardcoded into a test. The differential is performed by inspection and recorded in §Orca Differential Evidence below.

## Measured Baseline (2026-08-18)

Measured on 2026-08-18 against **regenerated** OrcaSlicer references, with `cargo xtask build-guests --check` reporting clean. Decisive fixture: `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.

| metric | PnP tree | Orca tree | PnP normal | Orca normal |
| --- | --- | --- | --- | --- |
| distinct `;Z:` | 150 | 150 | 150 | 150 |
| `;TYPE:Support` blocks | 123 | 122 | 122 | 121 |
| `;TYPE:Support interface` blocks | 2 | 2 | 1 | 3 |
| distinct Z carrying support | 125 | 124 | 123 | 124 |
| deposited support + interface filament (mm) | 432.85 | 683.96 | not re-measured | not re-measured |
| support + interface XY path length (mm) | 13,013.9 | 22,774.9 | not re-measured | not re-measured |

**Support filament must be measured as DEPOSITED material (corrected 2026-08-18).** The earlier row read PnP tree 486.33 / Orca tree 1538.36 / PnP normal 852.02 / Orca normal 1158.87, and the "31.6%" derived from it. Those sums are the total of every positive `E` delta inside a support block, **including de-retraction prime `E`**, which deposits no material and travels zero distance. Orca carries ~853 mm of prime against PnP's ~96 mm precisely *because* it prints ~58 short separate loops per top layer and retracts between them — so the naive metric penalises PnP twice for the same defect (once for printing less, once for the granularity that is itself the finding). **Do not quote 486.33 / 1538.36 / 852.02 / 1158.87 or the 31.6% derived from them.** The normal-family cells are left `not re-measured` rather than restated, because only the tree pair was re-derived on deposited material; see `tree-density-diagnosis.md` for the parser and its anchoring against each file's own `; filament used [mm]` footer.

The corrected tree deficit is **63.3%** of Orca's deposited material — a **1.58x** deficit, **not 3.2x**. It decomposes cleanly: PnP lays down 13,013.9 mm of support XY path against Orca's 22,774.9 mm (**1.75x short**), while PnP's flow per mm of path is **1.107x higher** than Orca's, and 1.75 / 1.107 = 1.58. The path-length row is the honest coverage metric; the filament row is path length scaled by a flow discrepancy that is a separate defect (see the gap register).

**Re-measured 2026-08-20 after the RC-15 contact-sampling port (commit `ad9019ee`).** The PnP tree rows above were re-derived from a fresh slice (`target/pnp_support_tree.gcode`, config `tmp/support-family-config-tree-matched.json`, `--module-dir modules/core-modules`) with the same deposited-material parser as the 2026-08-18 measurement, anchored against each file's own `; filament used [mm]` footer (PnP 1908.02 vs 1908.03; Orca 2872.81 vs 2872.80). The port moved the tree deficit from 1.76x to 1.58x on deposited material (1.949x → 1.75x on XY path length); the structural rows (distinct Z, block counts, Z carrying support) are unchanged. The remaining deficit decomposes into the routed causes recorded in `tree-density-diagnosis.md` (wall/fill divergence W1/W2/W3, the uniform flow model, and top-interface area downstream of contact density) — none of which this packet implements.

**Extruding-move counts are NOT a parity metric.** Orca's tree segments are roughly 15x shorter than PnP's, so a move count measures polygon granularity, not deposited material. Never gate on it, and never quote it as evidence of coverage.

Two rows are 224 blockers: the normal family's 1-versus-3 interface blocks (locked decision 4), and the tree coverage deficit — 1.75x short on support XY path length, 1.58x short on deposited material (post-port, 2026-08-20) — over an identical Z range and layer count (locked decision 5). Root-cause diagnosis was blocking and is now recorded as RC-15; the RC-15 port landed in `ad9019ee` and the remaining deficit is attributable to the routed causes in `tree-density-diagnosis.md`.

### Orca reference profile (normal), regenerated 2026-08-18

`support_threshold_angle=30`, `support_object_xy_distance=0.35`, `support_top_z_distance=0.2`, `support_bottom_z_distance=0.2`, `support_interface_top_layers=2`, `support_interface_bottom_layers=2`, `support_interface_spacing=0.4`, `support_base_pattern=rectilinear`, `support_base_pattern_spacing=2`, `support_line_width=80%`, `support_on_build_plate_only=1`, `support_expansion=0`, `support_style=default`, `layer_height=0.2`, `initial_layer_print_height=0.2`.

### HANDOFF-224.md numbers are void

`HANDOFF-224.md`'s **STRUCTURAL** findings (RC-6..RC-12, the `body_overlaps_occupancy` floating-point defect, the droppable termination layer) **stand** and are recorded below. Every **measured number** in that document is **void**: it was captured against the previous Orca references, which were regenerated on 2026-08-18 with many settings disabled. This includes its interface-count match claim ("tree 2 vs 2, normal 3 vs 3"), its 125/90/89 tree entry counts, its 205-distinct-print-Z figure (the regenerated references disable `independent_support_layer_height`; both now emit 150 distinct Z — see `requirements.md` §Out of Scope), and any filament or move-count figure. Requote nothing from it; the table above is the only current baseline.

## Root Causes (recorded 2026-08-17)

### RC-0 — the support analysis stage performs no overhang detection

`commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) pushes a `SupportCandidate` for **every non-empty region on every layer**, with `geometry: region.polygons.clone()` — the full model cross-section — and `enforced: false, blocked: false` unconditionally. There is no overhang detection anywhere in the stage. What the IR calls "candidates" are simply the model's slice regions.

This is the upstream cause of RC-1. Both planners receive a candidate stream that carries no support signal, so each is forced to invent its own contact detection, and any planner that trusts the stream necessarily produces support at every layer of the model.

**Canonical contact derivation is 2D and slice-based, not mesh-facet-based.** `detect_overhangs` (`SupportMaterial.cpp`) computes `diff(layerm_polygons, expand(lower_layer_polygons, lower_layer_offset, SUPPORT_SURFACES_OFFSET_PARAMETERS))`, where `lower_layer_offset` derives from `support_threshold_angle` as `scale_(lower_layer.height / tan(threshold_rad))` (an offset of 0 degenerates to a plain `diff`). Results feed `detect_contacts`, which allocates one `TopContact` layer per object layer. A contact layer is produced **once, at the overhang's own Z**.

**Chosen fix.** Add a pure sibling function to `annotate_overhangs` in `crates/slicer-core/src/algos/overhang_annotation.rs` implementing the canonical angle-thresholded slice difference, and call it from `commit_support_analysis_builtin` over the `SliceIR` regions that function already iterates. This preserves per-`(object_id, region_id)` attribution and needs no IR or WIT change.

Two upstream fields were considered and rejected as the primary source:

- `SurfaceClassificationIR.overhang_quartile_polygons` mirrors canonical `detect_overhangs_for_lift` (`PrintObject.cpp`), a different function from support's `detect_overhangs`. It partitions into four bands at `line_width_mm × {0.5, 1.0, 1.5, 2.0}` — fixed-distance quantization with an unbounded top band — whereas support needs a continuous `layer_height / tan(threshold)` offset. Wrong threshold semantics, and the unbounded band loses resolution exactly where support decisions are made.
- `SurfaceClassificationIR.prev_layer_boundaries` (packet 193) is the right *kind* of input and makes the diff exactly computable for any angle, but it is keyed by global layer index alone, so it aggregates across objects and regions. Support analysis is per `(object_id, region_id)`; deriving the previous layer's own region polygons from `SliceIR` keeps that attribution.

### RC-1 — traditional support declines every candidate

`traditional-support-planner` re-derives contact geometry for every candidate layer from **mesh facet normals** (`overhang_facets`), filtering by `max_z >= slab_bottom && min_z <= layer.z`. The decisive fixture's overhang facets are coplanar at z=25 (fixture is 20 triangles, bbox z 0→30, distinct vertex z ∈ {0, 25, 30}, 4 downward-facing facets), so they intersect exactly one layer slab. Every other candidate yields an empty contact set and is declined `NoRoute`.

Two divergences, not one: the detection *mechanism* is wrong (facets, not slices — see RC-0), and the *structure* is wrong (re-derived per layer instead of derived once and propagated down).

**Canonical downward propagation does not expand.** This corrects an earlier reading recorded during this packet's remediation. `generate_base_layers` performs only `diff(polygons_new, polygons_trimming, ApplySafetyOffset::Yes)` per intermediate layer — no union with the layer above, no XY expansion, no closing (the fillet block is `#if 0`). The actual downward carry lives in `bottom_contact_layers_and_layer_support_areas`, which walks object layers top→bottom carrying `overhangs_projection` through `project_support_to_grid`, and deliberately propagates a **slightly smaller** area than it prints (`extract_support(expansion_to_propagate)` vs `extract_support(expansion_to_slice)`) so that base areas do not grow with depth.

**XY clearance is a separate pass.** `SupportParameters::gap_xy = support_object_xy_distance` is applied in `trim_support_layers_by_object`, not during propagation.

**A mesh with no overhang must still produce zero entries.** Any fix that makes support appear where canonical produces none is a regression, not a fix.

### RC-2 — tree support emits no interface roles

`tree-support-planner` emits only `SupportPlanRole::SupportBody`; `traditional-support-planner` emits `SupportBody`, `TopInterface`, and `BottomInterface`. The tree planner nonetheless declares `support_interface_top_layers` and `support_interface_bottom_layers` config keys (see `docs/15_config_keys_reference.md`), so it advertises interface behaviour it does not implement. AC-4 requires the `;TYPE:Support interface` marker for both families.

A prior attempt to split `push_interface_scan_lines` into a distinct `TopInterface` role collapsed the branch shaft from 30 entries to 16, leaving only the contact. That collapse must be diagnosed before any further change; reverting to body-only and relaxing the gate is not an acceptable resolution.

**Canonical confirms both roof and floor exist for tree.** Roofs are built in `TreeSupport::generate_toolpaths`'s area pass, which fills `ts_layer->roof_areas`, `roof_base_areas`, `roof_1st_layer`, and `roof_gap_areas`. Per node: `distance_to_top < 0` → `roof_gap_areas`; `support_roof_layers_below == 1` → `roof_1st_layer`; `> 1` → `roof_areas` (or `roof_base_areas` when at or below `top_base_interface_layers`). Roofs are produced only when `num_top_interface_layers > 0`, `obj_layer_nr > 0`, and the node is not a sharp tail; otherwise the tip goes to `base_areas` as a bare branch tip. `support_roof_layers_below` is seeded at tip creation and **decremented per `create_node` while descending** — it is a per-node counter, not a global layer-distance band.

Roof geometry is a **distinct** set of `ExPolygons`, closed with `closing_ex(.., line_width_scaled)`, clipped by collision and the machine border, and **subtracted out of `base_areas`**. This is the most likely explanation for the prior shaft collapse: the earlier attempt appears to have made the interface role *replace* the body rather than *subtract from* it, so gating on `branch_segments.is_empty()` discarded the shaft. Interface must be carved out of the body, leaving the remainder as body.

Floors exist as `ts_layer->floor_areas`, gated on `!support_on_build_plate_only && (bottom_gap_height > EPSILON || bottom_interface_layers > 0)`, where `bottom_gap_height = gap_object_support` and `bottom_interface_layers = support_interface_bottom_layers`, **falling back to `support_interface_top_layers` when negative**. PnP's default for `support_interface_bottom_layers` is `-1`, so under default config floors are active via that fallback. Floors are anchored to the true support-to-model contact surface by searching downward for object layers whose top/bottom surfaces intersect the base component, clearing the gap band, and intersecting the component with the margin-expanded band; the result is removed from base.

### RC-3 — foreign family entries reach the wrong renderer

Both family renderers are always in the DAG, so each sees the other family's plan entries and returns `ModuleError::non_fatal(332)` / `(333)`. **Correction (2026-08-18).** The `n:<family>` qualifier described here does not exist and never did. `rg 'n:tree|strip_prefix\("n:"\)|qualifier'` over `crates/` returns nothing. The real mechanism is a separate `support-family:<id>` claim in each module manifest (`tree-support.toml` and `traditional-support.toml` both list it under `[claims] holds`), consumed by `module_claims_match_active_region` (`crates/slicer-scheduler/src/execution_plan.rs`). No host code filters `SupportPlanIR` *entries* by family — **region routing is the only guard**, which is why RC-4 alone was enough to hand a whole tree plan to `traditional-support`. The renderers keep their hard errors, which become genuinely unreachable invariants. No manifest schema change and no WIT change.

Renderer-side relaxation (`continue` on a foreign family) was attempted previously and is explicitly rejected: it silences genuine mis-routing.

### RC-4 — the support family never reaches region routing (FIXED 2026-08-18)

**Fixed 2026-08-18 by the backfill described below, and covered by `family_reaches_region_routing`.** RC-11 (tree top-Z) is likewise fixed (`d97fb2b8`). The open work in this packet is now the interface layer counts and RC-15 (tree contact-point derivation) — see §Measured Baseline.

`tree(auto)` publishes a full 126-entry tree `SupportPlanIR` and `run_slice` emits no `;TYPE:Support` at all.

**Measured, not inferred.** A temporary probe returning `ModuleError::non_fatal` from the top of `tree-support`'s `run_support` reported `enabled=true density=20 regions=0` at layer 24 of the decisive fixture. `Layer::Support` runs on all 150 layers and `com.core.tree-support` is dispatched on every one (confirmed via `--instrument-stderr` `module_start` records), so the renderer executes and is handed an empty region slice. A renderer with nothing to render is not an error, which is why this failed in complete silence — `non_fatal_error_count: 0`, `degraded: false`.

**Chain.** `module_receives_slice_region` (`crates/slicer-wasm-host/src/dispatch.rs`) filters regions for any module holding a `support-family:<id>` claim, delegating to `module_claims_match_active_region` (`crates/slicer-scheduler/src/execution_plan.rs`), which resolves the family from `ActiveRegion.resolved_config` — its `support_family` extension, its `support_type` extension, or the `support_type` enum. `RegionLayerProposal` carries no config across the WIT boundary, so `restore_layer_plan_configs` (`crates/slicer-wasm-host/src/marshal/in_.rs`) reconstructs it. That function injects the family keys **only in its final `.or_else` fallback**, so as soon as any earlier source supplies a config — the normal path — the family is absent and `select_support_family(None, None)` resolves `"traditional"` for every region. The equivalent native-transport path in `crates/slicer-wasm-host/src/marshal/native.rs` has the same shape.

**Why the obvious fix does not work.** Both sites read the family from `module_config`, which is the *layer planner's* config view, and module config views are filtered to the module's declared `config.schema`. `modules/core-modules/layer-planner-default/layer-planner-default.toml` declares only `layer_height` and `first_layer_height`, so `module_config.get("support_type")` returns `None`. The existing fallback has therefore never worked either. Stamping the keys in the earlier branches was attempted during this session and reverted, because it is inert for the same reason.

**Superseded fix (2026-08-18).** The recorded plan — thread the run's effective config into `restore_layer_plan_configs` — cannot be correct. `PrePass::LayerPlanning` runs *before* `PrePass::RegionMapping` (the latter's slot dependency requires `LayerPlan`), so at that point no region map exists and only the **global default** config could be stamped. Per-object `support_type` would still be wrong, which is exactly the mixed-family case packet 223 exists for.

**Implemented fix.** `promote_global_layers` (`crates/slicer-runtime/src/layer_executor.rs`) backfills each `ActiveRegion.resolved_config` from the committed `RegionMapIR` at plan promotion — after prepass, so the authoritative *per-region* config exists. Applied at all three promotion sites (`pipeline.rs` ×2, `run.rs` ×1). The layer-stage gate's local clone-patch is deleted, since every consumer now reads a backfilled region. Regions with no region-map entry are left untouched. The family is a run-level routing fact; `layer-planner-default` should not have to declare a key it never uses in order to smuggle it across the boundary. Apply the same change to the native transport. 

**Traditional was never correct here either.** It appeared to work only because `"traditional"` is the family the failed resolution happens to fall back to.

### RC-5 — family resolution is decided independently in three places

Found while diagnosing RC-4, and the reason RC-4 stayed invisible. Three sites decide a region's support family and can disagree:

1. `PrePass::SupportAnalysis` — from the region map's resolved config, into `SupportAnalysisIR.family_assignments`.
2. Each planner — from `family_assignments`, **defaulting to its own family** when no assignment is present (`tree-support-planner` falls back to `"tree"`, so it plans regardless).
3. Region routing — from `ActiveRegion.resolved_config`, per RC-4.

On the decisive fixture with `tree(auto)`, (2) said tree and planned 126 entries while (3) said traditional and starved the tree renderer. The RC-4 fix makes (3) agree with the others; consolidating the three onto a single source of truth was considered and deferred as a larger change to the scheduler's routing contract.

### RC-6 — no production code ever constructed `ExtrusionRole::SupportInterface`

Recorded from `HANDOFF-224.md` §"Defects found and fixed". The `;TYPE:Support interface` marker was unreachable in production: no production code path constructed the role, so `support_interface_speed` was never applied either. Both renderers now stamp the role on interface paths. Structural finding; the handoff's accompanying counts are void (see §Measured Baseline).

### RC-7 — `is_top_interface` was discarded in marshal

`convert_support_output_with_plan` (`crates/slicer-wasm-host/src/marshal/out.rs`) dropped `is_top_interface` during the drain, so `SupportRole::BottomInterface` had never been produced in production. The flag is now carried through.

### RC-8 — tree renderer emitted coincident walls plus 100% fill plus a duplicate grid-MST fill

`tree-support`'s `render_polygon` emitted `wall_count` **coincident** walls (all at the same offset), a fill at 100% density regardless of `support_density`, and a second overlaid fill from the grid-MST path. Rewritten: walls inset half a line width each, fill inset clear of the walls, pitch honours `support_density`, holes respected; the duplicate overlay and the now-dead grid-MST code are deleted.

### RC-9 — contact tips were created with `width = 0.0` and filtered out

`structural_body_regions` built detached 16-gon discs and gave contact tips `width = 0.0`, so the downstream width filter removed them and the layer meeting the overhang printed nothing. Bodies are now **swept capsules** (convex hull of endpoint circles, unioned) and tips carry a real radius.

### RC-10 — tree interface was a bounding-box scan-line hack

The tree interface was synthesised from bounding-box scan lines (`push_interface_scan_lines`) rather than from the node's own area. It is now the node's own area classified as roof/floor (`InterfaceRole`) and **carved out of** the body, matching the canonical rule that roof geometry is subtracted from `base_areas` (§RC-2). `push_interface_scan_lines` is deleted. The `is_roof` band includes the contact layer (`dist_to_top < top_n`), so the topmost support layer is interface rather than bare body.

### RC-11 — tree ignores `support_top_z_distance_mm` (FIXED 2026-08-18, commit `d97fb2b8`)

**FIXED in commit `d97fb2b8`** (`fix(tree-support): honour support_top_z_distance_mm (RC-11)`). Recorded below because the root cause was misdiagnosed twice before it was found.

**True root cause: the key was never read.** `tree-support-planner`'s `from_config` reads 17 other keys and does not read `support_top_z_distance_mm` at all, while the module declares the key in two manifests. The key is also absent from `crates/slicer-schema/wit/` entirely. Tree's top interface therefore lands at the overhang underside with a **zero** gap; traditional is fixed and Orca-matching.

**The earlier "unexplained contradiction" is closed, not open.** A prior session recorded that a shift implemented in `push_contact_with_demand` had no effect while the config *value* still moved the result by 35 layers (125 entries at `0`, 90 at `0.2`, 89 at `0.4`), and declined to ship on that basis. Those measurements were **stale-guest artifacts** — the guest `.wasm` did not contain the edited planner. Do not carry the contradiction framing forward; there is nothing anomalous to explain. Run `cargo xtask build-guests --check` before trusting any tree-planner measurement.

**Fix as landed.** The key is read in `from_config`, and the contact layer is shifted the contact layer by **walking actual layer Z**, the technique `traditional-support-planner::plan_for_object` already uses. Dividing by `LayerPlanViewEntry.effective_layer_height` remains prohibited — the field is unreliable in the guest view (it produced a zero-layer gap in traditional and a 35-layer gap in tree) and should be separately investigated or documented as untrustworthy.

Note: `eprintln!` from guest code does not reach the test harness. Use `push_diagnostic`.

### RC-12 — traditional emitted bottom interface on the build plate

`traditional-support` produced `BottomInterface` for columns terminating on the **plate**. Bottom interface now appears only where a column terminates on the **model**. Related and fixed in the same pass: the termination layer was **droppable** when it failed the support-layer-height modulo, so columns stopped short of the plate; it now always prints.

### RC-13 — `body_overlaps_occupancy` decided overlap by floating-point accident

The helper ended with `point_in_polygon(closest_boundary_point)`, so the verdict depended on which side of the boundary a floating-point rounding landed; it reported "overlapping" for a body 8 mm clear of occupancy. Pinned by `body_clear_of_occupancy_does_not_overlap`.

### RC-14 — `in_routing_cell` rejected any body straddling an absolute grid line (FIXED 2026-08-18, commit `2afa4cf9`)

`in_routing_cell` (`crates/slicer-wasm-host/src/support_aggregation.rs`) required a support body's bounding box to fall inside a single cell of an **absolute** grid keyed by `ROUTING_CELL_SIZE = 1 << 20` units — 104.8576 mm at this repo's 100 nm unit. A body was therefore accepted or rejected by *where it sat on the world grid*, not by how large it was.

The decisive fixture's bbox edge sits exactly on `y = 0`, which is a grid line. A 0.4 mm tip disc at a model corner reaches `y = -0.4 mm`, crossing it, and was dropped. Measured: **528 rejections at `support_top_z_distance = 0.2`, zero at `0.0`** — the gap moves the tip across the line. The rejections took layers 90..124 with them and destroyed **both** `TopInterface` layers, which is why the interface counts and the top-of-branch coverage looked like planner defects.

**Fix.** `2afa4cf9` bounds a body by its **extent** (`maxx - minx` and `maxy - miny` against `ROUTING_CELL_SIZE`) rather than by absolute cell containment. Size is the property the guard was ever meant to check.

**Canonical hits the same case and does not guard against it.** `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) emits overhang **contour vertices directly** as contact points, so contacts land on the model's outer boundary as a matter of course — including on axis-aligned bbox edges. Any containment test keyed to an absolute grid is wrong for that input by construction.

### RC-15 — tree contact points come from mesh overhang-triangle centroids (GAP; in scope for 224)

**This is the dominant cause of the tree coverage deficit in §Measured Baseline.**

`tree-support-planner` derives contact points from **mesh overhang-triangle centroids, one per triangle**. A ~400 mm² overhang made of two triangles therefore yields **two** contact points, and the branch set can never be denser than the mesh tessellation — a property of the input file, not of the support settings.

Canonical `TreeSupport::generate_contact_points` (`TreeSupport.cpp`) never touches triangles. It samples the **per-layer overhang `ExPolygon`** three independent ways and unions the results:

1. **Contour corners** — a vertex is taken when its two incident edge directions satisfy `v1.dot(v2) > -0.7`, i.e. the contour turns sharply enough to need its own branch.
2. **Arc walk** — an `EdgeCache` walk emits points at `point_spread = tree_support_branch_distance` along the contour **and along every hole**.
3. **Interior grid** — points on a global grid rotated 22 degrees, at `sample_step = max(point_spread, max_bridge_length / 2)`, kept when they fall inside the overhang eroded by `base_radius`.

All three streams are deduped through a hash-bucket grid of cell size `base_radius`. The rotation and the global (not per-island) grid origin are what keep the sampling from aliasing with axis-aligned model features.

**Measured consequence.** PnP produces **2 closed loops at every Z**, with a footprint no larger than **8.2 mm**. Orca fans out with height: **2 → 3 → 4 → 14 → 58 loops**, reaching a **19.1 × 20.3 mm** footprint at `z = 24`. That is the same shape as the 1.949x XY-path-length deficit, and it explains the interface deficit too — with two tips there is almost nothing to roof.

**Classification: GAP** (a canonical sampling algorithm PnP has never had, not a regression). **Agreed to be implemented in 224**, because every other tree parity claim in this packet — coverage, interface placement, footprint — is unmeasurable until contact derivation is 2D and slice-based. It is not routed to the gap register.

### RC-16 — three tests passed only because RC-14 was broken (REPAIRED, commit `4d1848eb`)

`invalid_body_degraded` and `invalid_body_rejected` (the latter in **both** the tree-family and traditional-family planner test files) asserted rejection, and got it — but from `in_routing_cell`'s absolute-grid bug (RC-14), not from the invariant each test names. Fixing RC-14 correctly turned them red. They were wrong-reason passes, not regressions.

`invalid_body_degraded`'s occupancy path was dead for two further, independent reasons, both in the fixture rather than the assertion:

- the mesh was a **single coplanar triangle at `z = 100`**, which produces no occupancy at any layer the test inspects; and
- it was built with `..ObjectMesh::default()`, whose `Transform3d::default()` is an **all-zeros matrix** — so every vertex collapses to the origin regardless of the coordinates written above it.

An all-zeros default transform is a silent geometry eraser: it never errors, it just yields a degenerate mesh. Any fixture using `..ObjectMesh::default()` must set the transform explicitly. All three tests were repaired in `4d1848eb` to fail for the reason they name.

### RC-17 — commit `9f4540bd` introduced eight tree-family regressions

The tree renderer rewrite (`9f4540bd`, RC-8/RC-9/RC-10) took the two tree modules from **3 failures at `5a38fdce`** to **11 at `9f4540bd`**, and **10 at HEAD**. Seven of the eight introduced failures are in test files that are **byte-identical** across the window, so they cannot be new-test-against-old-source artefacts. Full per-test attribution, method, and its stated limitation: `tree-failure-attribution.md` in this packet directory.

**Process finding, recorded so it is not repeated.** `HANDOFF-224.md` called this work "Completed and verified". That verification rested on narrow `cargo test` runs that **stopped at the first failing binary** and consequently never built three of the eight relevant test binaries. A run that aborts early is not a green run; a binary that was never built cannot have passed. Compare binary counts before trusting any narrow run (see `CLAUDE.md` §Test Discipline).

### Recorded deviation — top-Z gap structure differs from canonical

Both implementations produce a one-layer gap at `support_top_z_distance = 0.2`, so the printed outcome matches; the **structures** do not, and the difference will matter to anyone porting further tree behaviour.

- **Canonical.** `TreeSupport::generate_contact_points` seeds `distance_to_top = -gap_layers`, creating a **virtual gap node** at `obj_layer_nr = layer_nr - 1`. That node is not printed as body: negative `distance_to_top` routes it to `roof_gap_areas` in the `generate_toolpaths` area pass (see §RC-2). The gap is a first-class node in the tree.
- **PnP.** `tree-support-planner` instead moves the **contact layer itself** down, by walking actual layer Z (RC-11). There is no gap node; the layers simply start lower.

Consequence: PnP has no object on which to hang gap-band behaviour, so anything canonical does with `roof_gap_areas` is currently inexpressible. Recorded as a deviation, not a defect.

## Read-Only Context
- `crates/pnp-cli/src/visual_debug.rs` lines 743-761, 1500-1680 - manifest and typed capture fields.
- `crates/slicer-runtime/src/visual_debug_render.rs` lines 1082-1142 - current support geometry tap renderer.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - standalone G-code role parsing pattern.

## Out-of-Bounds Files
- Orca source (delegated sub-agent reads only), target bundles, generated bindings, and packet 213 files.
- Base/interface **pattern generators**, `support_expansion`, `support_bottom_z_distance`, raft geometry, independent support-layer Z, and the `SupportGridPattern` AGG rasterizer — all routed to follow-on packets via `docs/specs/support-parity-gap-register.md` (224a/225/226/227). The renderers themselves are in scope (see §Files in Scope); their family-mismatch hard errors stay as they are, since RC-3 is fixed host-side.
- `docs/07_implementation_status.md` is updated only through delegated status work.

## Orca Differential Evidence

Recorded by inspection against `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode`. Parity claims are limited to termination, coverage, collision freedom, interfaces, and independent heights. Exact path identity is never claimed.

**Gate shape (locked 2026-08-18).** Parity is gated by (a) structural invariants in the test suite and (b) a written human/LLM `/visual-debug` inspection checklist with **side-by-side Orca renders at matched physical heights**. No test may read the Orca G-code; no Orca-derived constant may be hardcoded into a test. Extruding-move counts are excluded as a metric.

**Current state.** RC-4 has landed, so both families now emit support G-code and the differential is unblocked. The quantitative side is recorded in §Measured Baseline (2026-08-18). The inspection checklist itself is written into §Orca Inspection Checklist by implementation Step 6.

## Orca Inspection Checklist

*Written by Step 6, 2026-08-20, after the RC-15 port (`ad9019ee`) and the Step 2 interface-count fix (`ee27ac94`). All four bundles were rendered with `cargo xtask build-guests --check` clean; `matched_height_evidence` passed.*

**Requests and bundles.** Per family, the PnP render is a model-source request and the Orca render is a standalone-G-code request over the regenerated reference, both at the same five layer indices:

| family | PnP request | PnP bundle | Orca request | Orca bundle |
| --- | --- | --- | --- | --- |
| tree | `tmp/visual-debug-support-family-tree.json` | `target/vd-support-family-tree` | `tmp/visual-debug-orca-tree.json` (`tmp/SupportTest_Tree_Orca.gcode`) | `target/vd-orca-tree-compare` |
| traditional | `tmp/visual-debug-support-family-normal.json` | `target/vd-support-family-normal` | `tmp/visual-debug-orca-normal.json` (`tmp/SupportTest_Normal_Orca.gcode`) | `target/vd-orca-normal-compare` |

**Matched physical heights.** Layers 10, 30, 79, 119, 123 on the shared 150-layer / 0.2 mm schedule = z 2.0, 6.0, 15.8, 23.8, 24.8 mm. Layer 123 (z 24.8) is the topmost support-carrying layer in all four files; the originally authored layer 124 (z 25.0) was dropped because PnP traditional and both Orca references carry no support there (only PnP tree prints a small interface remnant at z 25.0 — see the independent-heights verdict).

**Request-fixture notes (recorded so the evidence is not misread).** The tree request uses `filament_lines` only: the `PrePass::SupportGeometry` tap's skeleton paths carry no `Point3WithWidth.width`, and the renderer fails closed on `filled_areas` for width-less paths (`MissingWidth`, `crates/slicer-runtime/src/visual_debug_render.rs` — read-only context for this packet). The traditional request keeps both visualizations. Both Orca G-code requests carry `gcode_line_width_mm: 0.4` (the reference profile's support line width), which the G-code `filled_areas` renderer requires.

**Verdicts.** Each verdict names the layer and tap it was read from. Exact path identity is never claimed.

| family | axis | verdict | layer | tap | what was seen |
| --- | --- | --- | --- | --- | --- |
| tree | termination | PASS | 123 | `Layer::Support` | Support is visible from the low layers through the top interface layer; no floating islands. |
| tree | coverage | PASS | 79 | `Layer::Support` | PnP support spans the positive-x overhang footprint seen at the matched Orca height. |
| tree | collision freedom | PASS | 79 | `Layer::Support` | The support footprint stays on the positive-x side and does not enter the model wall (x ∈ [-10, 0]). |
| tree | interface placement/count | PASS | 123 | `Layer::Support` | Interface is the topmost support and is carved out of the body; 2 `;TYPE:Support interface` blocks, matching Orca's 2. |
| tree | independent heights | PASS | 123 | `Layer::Support` | Both emit the same 150-layer schedule; support appears at the same matched heights. PnP tree additionally prints a small interface remnant at z 25.0 that Orca does not — the recorded top-Z gap-structure deviation (`design.md` §Recorded deviation). |
| traditional | termination | PASS | 123 | `Layer::Support` | Support is present from the low layers through the top interface layer; no floating islands. |
| traditional | coverage | PASS | 79 | `Layer::Support` | PnP support spans the positive-x cantilever footprint corresponding to the matched Orca layer. |
| traditional | collision freedom | PASS | 79 | `Layer::Support` | Support remains offset from the model wall and does not visibly intersect it. |
| traditional | interface placement/count | DIVERGENT (count) | 123 | `Layer::Support` | Placement is correct (topmost, carved out of the body). Count: PnP emits **2** `;TYPE:Support interface` blocks at `top_layers=2`/`bottom_layers=2` (the count follows the configured top band, pinned by `interface_layer_count_follows_config`), Orca emits **3** at the same config. The difference is canonical roof/floor layer-count semantics, registered as a gap (see the gap register). |
| traditional | independent heights | PASS | 123 | `Layer::Support` | Both emit the same 150-layer schedule; support appears at the same matched heights. |

## Session Handoff (2026-08-17) — superseded

**Superseded by §Measured Baseline (2026-08-18) and §Root Causes RC-6..RC-13.** Retained for provenance only. Its state claims (notably RC-4 open and `final_gcode_roles` red) and every number in it are stale; do not requote them.

### Decisive fixture geometry (measured from the tracked STL)

20 triangles; bbox x ∈ [-10, 20], y ∈ [0, 20], z ∈ [0, 30]; distinct vertex z ∈ {0, 25, 30}; 4 downward-facing facets. It is an L-shape: a full-height wall at x ∈ [-10, 0] spanning z ∈ [0, 30], and a cantilever arm at x ∈ [0, 20] spanning z ∈ [25, 30]. The overhang is the arm's underside at z = 25; support must descend from there to the plate at x ∈ [0, 20] without touching the wall. Contact detection landing at anchor z ≈ 25.2 is the correct result.

### State

Seven of the eight closure tests pass. `final_gcode_roles` is **red on purpose**: it was hardened to AC-4 as written (both `;TYPE:Support` and `;TYPE:Support interface`, for both families, through the real `run_slice`) and now correctly fails on RC-4. Do not relax it.

Verified green: `cargo test -p tree-support-planner` (9 binaries), `cargo test -p traditional-support-planner`, `cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd`, `cargo test -p slicer-runtime --lib builtins::support_analysis`, and the seven passing closure tests. `cargo clippy` was run per-crate on both planners, clean.

### Not yet done

- RC-4 fix (above), then `final_gcode_roles` green for both families.
- Orca differential write-up (blocked on RC-4).
- Visual-debug regeneration against the two per-family requests with the `Layer::Support` tap added; `matched_height_evidence` still does not read the manifests, and `read_manifest` / `manifest_images` / `layer_indices` remain `#[allow(dead_code)]`.
- Full-workspace acceptance run via `cargo xtask test --summary --workspace`, dispatched to a sub-agent.
- `cargo xtask check-literals`.
- The needs-research deviation for `SupportGridPattern` (below).

### Deviation to file: SupportGridPattern — NEEDS RESEARCH

File this as **needs-research**, not as a queued port. A future session must first validate that the rasterizer is actually required before implementing it.

Canonical `SupportGridPattern` (`SupportMaterial.cpp`) is an AGG antialiased scanline rasterizer over a byte grid plus a 4-direction seed fill and marching-squares contour extraction — not a signed-distance field (the `EdgeGrid`/`calculate_sdf` branch is compiled out by `SUPPORT_USE_AGG_RASTERIZER`, which that file defines). It has five call sites, four of them in `fill_contact_layer`.

What packet 224 implemented instead is its *semantic*: propagate the contact area without growth, trim per layer against the object with `support_object_xy_distance` clearance. The justification is that the two canonical expansion values differ only by `flow_spacing/2` — `expansion_to_slice = scaled_spacing()/2 + 5` versus `expansion_to_propagate = -3`, and in Orca's 1 nm units (PnP's unit is 100 nm) the `+5` and `-3` are 5 nm and 3 nm, i.e. rounding epsilons. The `+flow_spacing/2` was deliberately **not** applied, because it exists to snap the zig-zag onto grid lines and without the grid it would only fatten support by an arbitrary amount.

The open research question is therefore: **does grid-snapping and contour simplification affect anything this project needs?** It changes support outline shape; it does not affect termination, coverage, collision freedom, interfaces, or independent heights — the five axes this packet's parity claims are limited to. Answer that before committing to an AGG rasterizer port.

## Expected Sub-Agent Dispatches
- Question: verify fixture existence and gitignore/production path; scope: `tmp/**`, `docs/specs/**`; return: `LOCATIONS`.
- Question: locate visual-debug tap/request and manifest differential seams; scope: `crates/pnp-cli/src/**`, `crates/slicer-runtime/src/**`, existing tests; return: `LOCATIONS`.
- Question: inspect Orca documented behavior at the listed locations; scope: `OrcaSlicerDocumented/**`; return: `LOCATIONS`.
- Question: delegate docs/07 closure and TASK-163b status; scope: `docs/07_implementation_status.md`; return: `SUMMARY`.

## Data and Contract Notes
- IR/manifest contracts: assert typed captures at `PrePass::SupportAnalysis`, `PrePass::SupportGeometry`, and `Layer::Support`; preserve structured family/body/demand roles from TASK-334.
- WIT boundary: no new WIT contract; inherited TASK-331 migration blocker is resolved. Packet 220 performed a breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0` (package stays 1.0.0). Schema versions: `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0.
- WASM boundary: packet 220's live WASM dispatch hands guests an EMPTY structural plan (the paint-view boundary does not carry plan entries); plan-consuming tests drive the renderer natively. For closure evidence, visual-debug taps and G-code role parsing must capture the host-side aggregated plan/`SupportIR` (which carry the identity), not guest-side plan reads.
- Determinism/scheduler constraints: compare forced serial/parallel fixture results and preserve anchored event order.

## Locked Assumptions and Invariants
- Exact-Z body and rendered sweep collision checks are authoritative over skeleton-only checks.
- Missing Orca references cannot be silently replaced by PNP output.

## Risks and Tradeoffs
- `TASK-163b-orca-ref` may remain externally blocked only if provenance/authority cannot be established; the existing references must still be used for primary differential review.
- Visual evidence is human-inspected and therefore cannot be reduced to a grep-only AC.

## Context Cost Estimate
- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: fixture production/availability, `FACT` or `LOCATIONS`.

## Open Questions
- [RESOLVED] TASK-331 exact-Z seam and WIT migration decisions. Exact-Z seam = `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs` (injected into `HostExecutionContext`; normalized to repo units, immutable per-(object,region,Z) caching). WIT migration = breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0. New contracts live: `PrePass::SupportAnalysis` + `SupportAnalysisIR` (candidates, occupancy/termination surfaces, baseline envelope, deterministic family assignments); structural `SupportPlanIR` v2.0.0 (family_id, demand IDs, body IDs, anchor layer index + Z, semantic ExPolygon roles, optional skeleton metadata, capabilities/provenance, decline reasons); attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths); `support_family` canonical + `support_type` aliases; `support-family:<id>` claims; startup pairing validation; host aggregation in `crates/slicer-wasm-host/src/support_aggregation.rs` with internal deterministic routing cells; no fallback filler.
- [BLOCK] Who will provide authoritative Orca tree/normal G-code references if the documented checkout cannot regenerate them?
- [FWD] TASK-334 must export final diagnostic fields and unmet-demand disposition consumed by closure tests.
