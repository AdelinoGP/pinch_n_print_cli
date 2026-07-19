# Design: 178-seam-region-aware-planning

## Controlling Code Paths

- Primary code path: `execute_prepass_with_builtins_configured_instr` / `required_slots` (`crates/slicer-runtime/src/prepass.rs`) -> typed `run-seam-planning` (`crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`) -> `SeamPlannerDefault::run_seam_planning` and `run_aligned_planning` (`modules/core-modules/seam-planner-default/src/lib.rs`) -> `harvest_seam_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`) -> variant-aware `push_perimeter_regions` and `backfill_resolved_seam`.
- Neighboring tests/fixtures: `dispatch_prepass_harvest_tdd`, `seam_region_aware_planning_tdd` (new), `seam_placer_dispatch_tdd`, and `seam_placer_tdd`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

- Cross-layer alignment remains in `PrePass::SeamPlanning`; per-layer `seam-placer` remains a consumer and final-geometry adapter.
- The new prepass input is a read-only projection of committed `SliceIR`/region data; it must not create a second mutable blackboard channel.
- `RegionKey` identity is `(global_layer_index, object_id, region_id, variant_chain)` everywhere. A numeric region ID is not sufficient when variants coexist.
- The existing `SeamPlanIR` duplicate-key validation remains authoritative; malformed identity is a contract error, not a best-effort drop.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a `seam-planning-view` composed of per-layer active-region records. Each record carries full region identity, `ex-polygon` contours in canonical integer units, existing segment annotations, layer Z/height, and the resolved prepass scoring width in millimetres. Run the guest after the required host products exist; retain mesh input for visibility in packet 2.
- Exact functions, traits, manifests, tests, and fixtures: `run-seam-planning` WIT export; `PrepassModule::run_seam_planning`; generated guest shim; `dispatch_prepass_call`; `required_slots`/phase routing; `project_region_segmentation_view` plus a new slice-region projection; `harvest_seam_plan_ir_from`; `SeamPlanEntry` SDK/WIT fields; `RegionKey` matching; `SeamPlannerDefault::run_aligned_planning`; `PerimeterRegion`/`perimeter-region-view` variant identity; new multi-region planner and injection tests.
- Rejected alternatives and reasons:
  - Keep mesh contours and contour ordinals: silently misses multi-region plans and contradicts active-region identity.
  - Move alignment to `seam-placer`: layer calls are parallel and have no cross-layer state.
  - Add a host-native alignment pass: moves slicing policy outside the module seam.
  - Share one object-wide target: can cross painted variants with disjoint geometry.

## Files in Scope (read + edit)

- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` and `crates/slicer-schema/wit/deps/ir-types.wit` - role: versioned prepass and region identity contracts; expected change: add seam-planning region input and variant-aware seam/perimeter records.
- `crates/slicer-sdk/src/traits.rs`, `crates/slicer-sdk/src/prepass_types.rs`, and guest macro shims - role: SDK/WIT shape propagation; expected change: carry the new view and variant chain.
- `crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-wasm-host/src/marshal/in_.rs`, and host projection modules - role: project committed IR and harvest/inject identity; expected change: late-stage input and full-key lookup.
- `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/src/layer_executor.rs` - role: stage prerequisites and commit-time injection; expected change: schedule and match full identity.
- `crates/slicer-ir/src/slice_ir.rs` - role: perimeter-region identity/schema; expected change: additive variant chain and any required schema bump.
- `modules/core-modules/seam-planner-default/src/lib.rs` and new tests - role: consume per-region polygons; expected change: remove contour ordinal source from aligned path.
- `modules/core-modules/seam-placer/src/lib.rs` and WIT view plumbing - role: expose variant-aware region identity; expected change: preserve lookup context without changing placement semantics.

## Read-Only Context

- `crates/slicer-runtime/src/prepass.rs` - lines 385-679 and 714-783 only - phase routing and prerequisites.
- `crates/slicer-wasm-host/src/dispatch.rs` - lines 656-849 and 1382-1420 only - typed prepass dispatch and perimeter injection.
- `crates/slicer-wasm-host/src/marshal/in_.rs` - lines 180-324 and 491-568 only - slice projection and seam harvest.
- `crates/slicer-ir/src/slice_ir.rs` - lines 971-1027, 1202-1234, 1360-1372, and 1924-1959 only - active regions, keys, sliced regions, and perimeter regions.
- `crates/slicer-sdk/src/traits.rs` - lines 584-640 only - prepass trait boundary.
- `modules/core-modules/seam-planner-default/src/lib.rs` - lines 68-199 only - aligned driver.
- `modules/core-modules/seam-placer/src/lib.rs` - lines 121-183 and 265-353 only - consumer lookup and wall preservation.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load directly.
- `target/`, `Cargo.lock`, generated code, and vendored dependencies - never load.
- Final canonical scoring and spline implementation - packet 2.
- Continuous wall insertion and default/fallback behavior - packet 3.

## Expected Sub-Agent Dispatches

- Question: identify every WIT guest shim and struct literal affected by the new prepass parameter; scope: `crates/slicer-macros/**`, `modules/core-modules/*/wit-guest/**`, and `crates/slicer-wasm-host/test-guests/**`; return: `LOCATIONS`; purpose: Step 1 blast-radius inventory.
- Question: verify the exact RegionMap/SliceIR-to-WIT projection shape and variant-chain ordering; scope: `crates/slicer-wasm-host/src/marshal/**` and `crates/slicer-ir/src/slice_ir.rs`; return: `LOCATIONS`; purpose: Step 2 projection contract.
- Question: verify canonical perimeter candidate ownership; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/{SeamPlacer.cpp,SeamPlacer.hpp}`; return: `LOCATIONS`; purpose: preserve the source distinction without direct reads.

## Data and Contract Notes

- IR/manifest contracts: `SeamPlanIR` already owns full `RegionKey` in Rust but current WIT harvest reconstructs an empty `variant_chain`; this packet closes that loss. Perimeter regions must expose the same identity before host injection can be exact.
- WIT boundary: adding required fields/parameters to `slicer:world-prepass` and shared `ir-handles` is a major world change and rebuilds all prepass guests.
- Determinism/scheduler constraints: projection ordering is ascending `(layer, object, region, variant_chain)`; phase routing must ensure SliceIR and region data exist before dispatch; no map iteration may choose plan order.

## Locked Assumptions and Invariants

- No aligned plan is keyed by contour ordinal.
- No active-region plan is broadcast to another variant.
- Inactive regions produce no plan entry.
- `variant_chain` order and values survive guest -> host -> IR -> layer lookup unchanged.
- Existing wall-preservation behavior remains unchanged in this packet.

## Risks and Tradeoffs

- Adding variant identity to perimeter IR may require a minor schema bump and broad struct-literal fallout.
- Moving seam planning to the late prepass phase changes timing but not the module claim; the scheduler must still run it before any layer stage.
- Per-region SliceIR polygons are closer to canonical source than mesh contours but remain upstream of final inset walls; packet 3 owns the final projection mitigation.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: WIT/IR struct-literal inventory, `LOCATIONS`.

## Open Questions

- `[FWD]` The implementer may choose whether the new region view is a dedicated record or a reuse of imported `slice-region-view` resources, provided the exact fields and ordering in AC-1 remain stable.