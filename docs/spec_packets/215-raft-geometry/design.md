# Design: raft-geometry

## Controlling Code Paths

- Primary code path: `com.core.raft-default` Layer::Infill synthesizes `SlicedRegion.raft_fill`; `com.core.rectilinear-infill` claims `claim:raft-fill` and emits `ExtrusionRole::RaftInfill` through the existing renderer.
- Signed schedule path: `GlobalLayer.index` in `crates/slicer-ir/src/slice_ir.rs:1015-1019` is projected into visual-debug `ScheduledLayer` at `crates/pnp-cli/src/visual_debug.rs:1377-1390`, resolved fail-closed at `crates/pnp-cli/src/visual_debug.rs:933-975`, and converted back to the runtime capture request at `crates/pnp-cli/src/visual_debug.rs:1390-1394`.
- Infill contract path: `LayerModule::run_infill` is `u32` at `crates/slicer-sdk/src/traits.rs:358-365`; WIT glue receives `i32` and currently casts at `crates/slicer-macros/src/lib.rs:3080-3095`; the SDK guest implementation is `u32` at `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs:28-35`.
- Neighboring tests/fixtures: `modules/core-modules/raft-default/tests/raft_geometry_tdd.rs`, `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`, `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`, `tmp/visual-debug-raft.json`, and `tmp/visual-debug-raft-typed.json`.
- ADR conformance: follows ADR-0009; no Layer::Support raft renderer exists.

## Architecture Constraints

- `SupportPlanIR.raft_plan` remains configuration input; `SlicedRegion.raft_fill` is the carrier; `SupportIR.raft_paths` remains downstream output.
- `GlobalLayer.index` and the IR fields named in `requirements.md` are signed. `SupportGeometryKey.global_support_layer_index` and its `u32::MAX` intermediate-layer sentinel remain unsigned and are not part of the scheduled-layer migration.
- WIT `layer-idx` is already `s32`; the macro must pass negative values unchanged and no host/runtime conversion may use `as u32` for the infill layer argument.
- `claim:raft-fill` maps to `ExtrusionRole::RaftInfill`; existing rectilinear scan-line code is reused.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: first migrate scheduled IR indices and the Layer::Infill contract; then add the synthesizer and claim holder; finally wire negative-prefix scheduling, typed capture, fixture fallback, and docs.
- Exact symbols: `GlobalLayer.index`, `ObjectLayerRef.local_layer_index`, `ObjectLayerRef.global_layer_index`, `SliceIR.global_layer_index`, `SupportIR.global_layer_index`, `LayerModule::run_infill`, generated `run(layer_index: i32)`, `RaftDefault`, `SlicedRegion.raft_fill`, `ExtrusionRole::RaftInfill`, `claim:raft-fill`, and `SupportIR.raft_paths`.
- Migration inventory command: `rg -l 'GlobalLayer\s*\{|ObjectLayerRef\s*\{|LayerPlanIR\s*\{|SliceIR\s*\{|SupportIR\s*\{|global_layer_index:|global_support_layer_index:|u32::MAX' crates modules`; inspect returned lines and classify each hit as migrated scheduled field, affected conversion/literal/assertion, or preserved sentinel/unrelated field before editing.
- Infill inventory command: `rg -n 'run_infill|call_run_infill|run-infill|layer_index as u32|layer_index: u32' crates/slicer-sdk crates/slicer-macros crates/slicer-wasm-host crates/slicer-runtime modules`; include SDK traits, macro glue, host/runtime dispatch, guests, fixtures, and assertions in the owned migration.
- Rejected: Layer::Support `raft-generator` (contradicts ADR-0009); separate raft stage/claim (architecturally rejected by ADR-0009); unsigned schedule conversion (makes negative selectors fail closed); pattern duplication (violates the ADR).

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` and every returned scheduled-field literal/assertion site - signed definitions and fallout.
- `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, and affected SDK/macro/guest tests - infill contract.
- `crates/slicer-runtime/`, `crates/slicer-wasm-host/`, and `crates/pnp-cli/src/visual_debug.rs` same dispatch/capture path - signed scheduling and selector resolution.
- `modules/core-modules/raft-default/` and `modules/core-modules/rectilinear-infill/` - synthesizer and claim holder.
- `Cargo.toml`, `tmp/visual-debug-raft.json`, `tmp/visual-debug-raft-typed.json`, and named documentation sections - integration and docs.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` lines 1013-1048, 1223-1234, 1529-1550, 2172-2199 - schedule and migrated IR definitions plus preserved sentinel.
- `crates/slicer-sdk/src/traits.rs` lines 345-365 - WIT-documented infill trait.
- `crates/slicer-macros/src/lib.rs` lines 3073-3115 - generated infill glue and current cast.
- `crates/pnp-cli/src/visual_debug.rs` lines 925-979, 1371-1394, 1620-1629 - selector resolution and schedule use.
- `crates/slicer-runtime/src/layer_executor.rs` lines 1129-1171 and 623-650 - capture filtering and schedule-indexed slice hydration.
- `crates/slicer-wasm-host/src/marshal/in_.rs` lines 68-95 and 566-593 - layer-plan projection and literals.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...`, `target/`, generated bindings/WASM, lockfiles, and vendored dependencies.
- Support planner, tree-support, traditional-support, and unrelated finalization/scheduler fields, except bounded symbol lookup for the migration inventory.

## Expected Sub-Agent Dispatches

- Question: enumerate every scheduled IR struct literal, conversion, hard assertion, and preserved sentinel hit; scope `crates/**/*.rs,modules/**/*.rs`; return `LOCATIONS`.
- Question: enumerate every Layer::Infill `run_infill` implementation, macro/host/runtime conversion, guest, test, and call site; scope `crates/slicer-sdk,crates/slicer-macros,crates/slicer-wasm-host,crates/slicer-runtime,modules`; return `LOCATIONS`.
- Question: determine whether negative selectors resolve and whether typed captures expose non-empty `raft_paths`; scope `crates/pnp-cli`, `crates/slicer-runtime`, `tmp/visual-debug-raft*.json`; return `FACT`.
- Question: verify polygon offset/clip helpers and existing `RaftInfill` dispatch; scope `crates/slicer-core/src,crates/slicer-sdk/src,modules/core-modules/rectilinear-infill`; return `LOCATIONS`.

## Data and Contract Notes

- IR: the five scheduled fields are `i32`; support-geometry sentinel fields remain `u32`; `SupportIR.raft_paths` shape is unchanged.
- WIT: `type layer-idx = s32` remains unchanged; SDK and generated glue must agree on `i32`.
- Determinism: stable footprint union, polygon order, scan order, prefix order, and capture order are required.
- Visual-debug: selectors resolve against the signed schedule and fail closed for a genuinely absent selector; fallback typed capture must report PNG unsupported rather than silently passing.

## Locked Assumptions and Invariants

- Empty plan, zero layers, or empty footprint is a successful no-op.
- Raft carrier geometry is clipped to the object footprint expanded by the fixed test margin.
- Raft rendering uses `RaftInfill`, never `SupportMaterial`, and does not populate ordinary support/interface paths.
- Negative prefix indices are `-1..=-raft_layers`; model layers are non-negative.

## Risks and Tradeoffs

- The signed migration has broad compile fallout; its implementation step owns the complete inventory and all assertion updates rather than deferring discovery to workspace compilation.
- Runtime vectors historically use layer indices as `usize`; negative prefix layers require an explicit signed-key lookup or schedule mapping, never a direct negative-to-`usize` cast.
- Visual-debug PNG rendering may remain model-layer-only; typed `raft_paths` capture is the decisive fallback gate and its limitation is recorded.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch: scheduled-field and infill-contract inventories, both required return `LOCATIONS`.

## Open Questions

- [FWD] If the renderer rejects negative selectors after signed schedule migration, retain the explicit negative selectors in `tmp/visual-debug-raft.json`, print `PNG layer selection unsupported`, and use `tmp/visual-debug-raft-typed.json` for the non-empty typed-capture gate.
