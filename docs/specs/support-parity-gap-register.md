# Support parity gap register

**Purpose.** One row per known OrcaSlicer support-parity gap, its evidence, and the packet that owns
it. Packet 224 closes on **correctness plus honest tests**, not on canonical feature completeness
(`docs/spec_packets/224-support-family-orca-closure/requirements.md` §Problem Statement). A gap that
is registered and routed here is **not** a 224 blocker; an incorrect behaviour, or a test that
asserts nothing, is.

**Reading rules.**

- OrcaSlicer is cited by **file + function**, never by line number (`CLAUDE.md` §OrcaSlicer Citation
  Style). PnP is cited by **crate-qualified path + symbol**.
- Every figure below was measured; none is estimated. Where a figure is contaminated or unmeasured
  the row says so rather than substituting a plausible number.
- `224` in the destination column means the gap is **not** routed out — it is owned by packet 224
  itself, despite being a gap, because the packet's other parity claims depend on it.

**Destination packets.** `224` support-family Orca closure · `224a` `SupportGridPattern` AGG
rasterizer (needs-research first) · `225` independent support-layer Z · `226` base/interface pattern
generators, `support_expansion`, `support_bottom_z_distance` · `227` raft geometry.

---

## Register

| # | Gap | Evidence | Destination |
| --- | --- | --- | --- |
| G-01 | **Tree contact points from mesh triangle centroids.** PnP derives tree contacts from overhang-triangle centroids, one per triangle, so branch density is bounded by mesh tessellation. Canonical `TreeSupport::generate_contact_points` samples the per-layer overhang `ExPolygon` three ways — contour corners at `v1.dot(v2) > -0.7`, an `EdgeCache` arc walk at `point_spread = tree_support_branch_distance` over contour **and** holes, and a rotated interior grid at `sample_step = max(point_spread, max_bridge_length / 2)` tested inside the overhang eroded by `base_radius` — deduped by a hash-bucket grid of cell size `base_radius`. | PnP: 2 closed loops at every Z, footprint ≤ 8.2 mm. Orca: 2 → 3 → 4 → 14 → 58 loops, footprint 19.1 × 20.3 mm at `z = 24`. Dominant cause of the 1.949x support XY-path-length deficit. `design.md` §RC-15. | **224** (not routed) |
| G-02 | **Independent support-layer Z.** PnP has no support-layer Z independent of object-layer Z. *Partly reachable already:* the blockers are `is_same_z_entity`'s on-grid filter (`crates/slicer-runtime/src/layer_executor.rs`), which excludes off-grid entities, and `crates/slicer-runtime/src/pipeline.rs` never calling `execute_per_layer_with_anchored_events`. **Unverified risk:** `height_delta` (`crates/slicer-gcode/src/emit.rs`) is computed per layer, so it may mis-scale flow for an off-grid entity. That risk is stated, not measured. | Both regenerated references emit **150** distinct print Z for a 150-layer print, because `independent_support_layer_height` was disabled when they were regenerated (2026-08-18). The gap is therefore a **missing canonical feature**, not a divergence measurable against the current references. The previously quoted "Orca 205 vs PnP 150" figure is **void** — do not requote it. | **225** |
| G-03 | **Support base-pattern and interface-pattern generators**, including `support_base_pattern` and `support_base_pattern_spacing` behaviour. | Keys are declared and dead; the reference profile uses `support_base_pattern = rectilinear`, `support_base_pattern_spacing = 2`. `requirements.md` §Out of Scope. | **226** |
| G-04 | **`support_expansion`** unimplemented. | Reference profile sets `support_expansion = 0`, so the current references cannot exercise it. | **226** |
| G-05 | **`support_bottom_z_distance`** unimplemented. | Reference profile sets `support_bottom_z_distance = 0.2`; PnP honours only the top-Z distance (`design.md` §RC-11). | **226** |
| G-06 | **Raft geometry.** `RaftPlan` (`crates/slicer-ir/src/slice_ir.rs`) is **built and rendered by nothing** — the IR exists, the consumer does not. All raft config keys are dead in the four support modules. | Structural: no renderer consumes `RaftPlan`. Kept as-is (dead) rather than removed or wired, per `requirements.md` §Out of Scope. | **227** |
| G-07 | **`SupportGridPattern` AGG rasterizer.** Canonical (`SupportMaterial.cpp`) is an AGG antialiased scanline rasterizer over a byte grid, plus a 4-direction seed fill and marching-squares contour extraction; the `EdgeGrid`/`calculate_sdf` branch is compiled out by `SUPPORT_USE_AGG_RASTERIZER`. PnP implements the **semantic** (propagate without growth, trim per layer at `support_object_xy_distance`), not the rasterizer. | **Needs-research first**, not a queued port: the open question is whether grid-snapping and contour simplification affect anything this project needs. They change support outline shape but not termination, coverage, collision freedom, interfaces, or independent heights. `design.md` §Deviation to file. | **224a** |
| G-08 | **No `support_line_width`.** PnP's `line_width` is **global**, shared by perimeters, infill, and skirt/brim, so a support-specific width is inexpressible. Orca's reference profile splits support 0.4 against perimeter 0.525. `support_line_width` does appear in `crates/slicer-gcode/src/serialize.rs`, but only as a G-code **header config-block field** — it feeds no extrusion geometry. | Reference profile: `support_line_width = 80%`. The split is unrepresentable in PnP's config today. | **226** |
| G-09 | **`effective_layer_height` disagrees across transports.** `project_layer_plan_view` (`crates/slicer-wasm-host/src/marshal/in_.rs`) takes a **max**; `build_native_prepass_request` (`crates/slicer-wasm-host/src/marshal/native.rs`) takes a **first match**. The same run can therefore hand a guest two different layer heights depending on transport. | This is why `design.md` §RC-11 prohibits dividing by `LayerPlanViewEntry.effective_layer_height` and mandates walking actual layer Z instead. | **226** |
| G-10 | **Tree branch bodies are filled, and `support_density` is mis-scaled.** PnP renders tree branches as **filled** areas; canonical renders **hollow concentric walls**. Compounding it, `support_density` arrives as `20.0` (a percent) and is consumed as a **fraction**, so the `.min(1.0)` clamp makes the fill **100% solid** at any configured density above 1. | Contributes to PnP's higher flow per mm (see G-11). `modules/core-modules/tree-support/src/` `render_polygon`. | **226** |
| G-11 | **PnP over-extrudes support 1.107x versus Orca** — flow per mm of path is 1.107x higher. This is why the deposited-material deficit (1.76x) is smaller than the path-length deficit (1.949x): 1.949 / 1.107 = 1.76. | Deposited support + interface filament: PnP 388.73 mm vs Orca 683.96 mm (56.8%). XY path length: PnP 11,687.5 mm vs Orca 22,774.9 mm. `design.md` §Measured Baseline; `tree-density-diagnosis.md`. | **226** |
| G-12 | **`MAX_BRANCH_RADIUS_MM = 6.0`** in `modules/core-modules/tree-support-planner/src/lib.rs`; canonical caps branch radius at **10.0**. | Constant mismatch, structural. | **226** |
| G-13 | **Missing canonical "raise radius to `base_radius` when `support_interface_top_layers > 0`".** PnP's tree planner has no equivalent of this rule, so branch tips stay narrower than canonical wherever roofs are enabled. | Reference profile sets `support_interface_top_layers = 2`, so the rule is active in every current reference. | **226** |
| G-14 | **`ERR_MALFORMED_LAYER_MARKER` fires ~110 times per run** from `modules/core-modules/machine-gcode-emit/src/lib.rs`. | **Pre-existing and unrelated to support** — it is present with support disabled. Recorded here only so it is not re-diagnosed as a support defect. | unrouted (pre-existing; not a support gap) |
| G-15 | **`cargo xtask check-literals` reports 61 inherited violations across 34 files.** | Pre-dates packet 224. Recorded so a 224 closure run is neither blocked on, nor credited with, inherited debt. | unrouted (inherited debt) |
| G-16 | **Undeclared config keys in `tree-support-planner`.** It reads `support_branch_merge_distance_mm` and `support_max_branches_per_layer`; **neither is declared in its manifest** (`modules/core-modules/tree-support-planner/tree-support-planner.toml`). A module's config view is filtered to its declared `config.schema`, so an undeclared key silently resolves to its in-code default. | Found during packet 224's config-key reconciliation (limited to the four support modules). Same failure mode as `design.md` §RC-4's `layer-planner-default` finding. | **226** |

---

## Rows deliberately **not** in this register

- **RC-15 tree contact-point derivation** is listed as G-01 for traceability but is **owned by packet
  224**. Routing it out would make every other tree parity claim in 224 unmeasurable.
- Anything classified as an incorrect behaviour rather than a missing feature. Those are root causes
  in `docs/spec_packets/224-support-family-orca-closure/design.md` §Root Causes (RC-0..RC-17), not
  register rows.
