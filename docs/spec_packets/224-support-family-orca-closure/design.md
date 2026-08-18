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

`tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode` are **inspection aids, not fixtures.** They exist to let a human or agent close this family of packets against a proven OrcaSlicer comparison. No test may read them, and no Orca-derived constant may be hardcoded into a test. The differential is performed by inspection and recorded in §Orca Differential Evidence below.

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

Both family renderers are always in the DAG, so each sees the other family's plan entries and returns `ModuleError::non_fatal(332)` / `(333)`. The fix is host-side: each support module declares its family via the `n:<family>` qualifier on its `support-generator` claim (`tree-support.toml` → `n:tree`, `traditional-support.toml` → `n:traditional`), and the host filters `SupportPlanIR` entries by that qualifier when building the per-module support view. The renderers keep their hard errors, which become genuinely unreachable invariants. No manifest schema change and no WIT change.

Renderer-side relaxation (`continue` on a foreign family) was attempted previously and is explicitly rejected: it silences genuine mis-routing.

### RC-4 — the support family never reaches region routing (OPEN)

**This is the one open defect. Everything else in this section is fixed and covered by tests.**

`tree(auto)` publishes a full 126-entry tree `SupportPlanIR` and `run_slice` emits no `;TYPE:Support` at all.

**Measured, not inferred.** A temporary probe returning `ModuleError::non_fatal` from the top of `tree-support`'s `run_support` reported `enabled=true density=20 regions=0` at layer 24 of the decisive fixture. `Layer::Support` runs on all 150 layers and `com.core.tree-support` is dispatched on every one (confirmed via `--instrument-stderr` `module_start` records), so the renderer executes and is handed an empty region slice. A renderer with nothing to render is not an error, which is why this failed in complete silence — `non_fatal_error_count: 0`, `degraded: false`.

**Chain.** `module_receives_slice_region` (`crates/slicer-wasm-host/src/dispatch.rs`) filters regions for any module holding a `support-family:<id>` claim, delegating to `module_claims_match_active_region` (`crates/slicer-scheduler/src/execution_plan.rs`), which resolves the family from `ActiveRegion.resolved_config` — its `support_family` extension, its `support_type` extension, or the `support_type` enum. `RegionLayerProposal` carries no config across the WIT boundary, so `restore_layer_plan_configs` (`crates/slicer-wasm-host/src/marshal/in_.rs`) reconstructs it. That function injects the family keys **only in its final `.or_else` fallback**, so as soon as any earlier source supplies a config — the normal path — the family is absent and `select_support_family(None, None)` resolves `"traditional"` for every region. The equivalent native-transport path in `crates/slicer-wasm-host/src/marshal/native.rs` has the same shape.

**Why the obvious fix does not work.** Both sites read the family from `module_config`, which is the *layer planner's* config view, and module config views are filtered to the module's declared `config.schema`. `modules/core-modules/layer-planner-default/layer-planner-default.toml` declares only `layer_height` and `first_layer_height`, so `module_config.get("support_type")` returns `None`. The existing fallback has therefore never worked either. Stamping the keys in the earlier branches was attempted during this session and reverted, because it is inert for the same reason.

**Chosen fix (agreed, not yet implemented).** Pass the run's effective/global resolved config into `restore_layer_plan_configs` rather than the layer planner's schema-filtered view, and stamp the support family onto every `ActiveRegion` from there. The family is a run-level routing fact; `layer-planner-default` should not have to declare a key it never uses in order to smuggle it across the boundary. Apply the same change to the native transport. Do **not** overwrite an `ActiveRegion`'s config wholesale — the current code leaves it untouched when no source matches, and preserving that is required.

**Traditional was never correct here either.** It appeared to work only because `"traditional"` is the family the failed resolution happens to fall back to.

### RC-5 — family resolution is decided independently in three places

Found while diagnosing RC-4, and the reason RC-4 stayed invisible. Three sites decide a region's support family and can disagree:

1. `PrePass::SupportAnalysis` — from the region map's resolved config, into `SupportAnalysisIR.family_assignments`.
2. Each planner — from `family_assignments`, **defaulting to its own family** when no assignment is present (`tree-support-planner` falls back to `"tree"`, so it plans regardless).
3. Region routing — from `ActiveRegion.resolved_config`, per RC-4.

On the decisive fixture with `tree(auto)`, (2) said tree and planned 126 entries while (3) said traditional and starved the tree renderer. The RC-4 fix makes (3) agree with the others; consolidating the three onto a single source of truth was considered and deferred as a larger change to the scheduler's routing contract.

## Read-Only Context
- `crates/pnp-cli/src/visual_debug.rs` lines 743-761, 1500-1680 - manifest and typed capture fields.
- `crates/slicer-runtime/src/visual_debug_render.rs` lines 1082-1142 - current support geometry tap renderer.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - standalone G-code role parsing pattern.

## Out-of-Bounds Files
- Orca source (delegated sub-agent reads only), target bundles, generated bindings, and packet 213 files.
- Support renderer implementations (`modules/core-modules/tree-support/src/`, `modules/core-modules/traditional-support/src/`) — RC-3 is fixed host-side; the renderers' family-mismatch errors stay as they are.
- `docs/07_implementation_status.md` is updated only through delegated status work.

## Orca Differential Evidence

Recorded by inspection against `tmp/SupportTest_Tree_Orca.gcode` and `tmp/SupportTest_Normal_Orca.gcode`. Parity claims are limited to termination, coverage, collision freedom, interfaces, and independent heights. Exact path identity is never claimed.

**Not yet performed.** It is blocked on RC-4: tree currently emits no support G-code, so there is nothing to compare on the tree side. Do this after RC-4 lands.

## Session Handoff (2026-08-17)

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
