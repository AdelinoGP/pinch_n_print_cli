# Design: support-interface-layers

## Controlling Code Paths

- Primary code path: `support-planner::run_support_geometry`/`plan_for_object`, typed prepass output, host harvest, then `tree-support::run_support` and `traditional-support::run_support`.
- Neighboring tests/fixtures: support-planner diagnostics; new `interface_layers_tdd.rs` in both support modules; IR schema tests; runtime `SupportPlanIR` literal tests; `tmp/support-config-interface.json` and `tmp/visual-debug-support-interface.json`.
- OrcaSlicer comparison: delegated `TreeSupport.cpp` structural behavior only.

## Architecture Constraints

- Canonical new symbol is `SupportInterfacePlanEntry` everywhere in Rust and WIT-facing plan vocabulary; do not introduce `SupportInterfacePlan`.
- Planner emits plan records; Layer::Support modules generate all interface paths. `SupportPlanEntry.branch_segments` stays branch-only.
- `SupportInterfaceKind::Top` maps to `is_top_interface = true`; `Bottom` maps to `false`; output role is `ExtrusionRole::SupportInterface`.
- `traditional-support` must read `SupportPlanIR` in its manifest because it is a support-generator winner.
- Additive `SupportPlanIR` field requires `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` `1.3.0` -> `1.4.0`; all explicit literals and old-value assertions belong to the schema step.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: one typed `interface_plan` list, one canonical entry shape, one WIT push path, and module-local scan-line generation over the planned region.
- Exact owners: IR `crates/slicer-ir/src/slice_ir.rs`; SDK types/output builder `crates/slicer-sdk/src/prepass_types.rs` and `crates/slicer-sdk/src/prepass_builders.rs`; WIT `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`; macro conversion `crates/slicer-macros/src/lib.rs`; host collection `crates/slicer-wasm-host/src/host.rs`; host marshal/harvest `crates/slicer-wasm-host/src/marshal/in_.rs` and dispatch; planner `modules/core-modules/support-planner/src/lib.rs`; consumers/manifests `modules/core-modules/tree-support/` and `modules/core-modules/traditional-support/`.
- Rejected alternatives: keep scan lines in branch segments (wrong ownership); use a second symbol spelling (ambiguous contract); let only tree-support read the plan (traditional-support is a declared winner).

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prepass_builders.rs` - canonical IR/SDK shape and builder storage.
- `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`, `crates/slicer-macros/src/lib.rs` - typed WIT method and conversion/drain owners.
- `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-wasm-host/src/marshal/in_.rs`, `crates/slicer-wasm-host/src/dispatch.rs` - collection and harvest/marshalling.
- `crates/slicer-ir/tests/ir_tests.rs`, `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`, `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` - literal/version fallout.
- `modules/core-modules/support-planner/src/lib.rs`, its diagnostics tests - planner behavior.
- `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/traditional-support/src/lib.rs`, both manifests, and focused tests - consumers and manifest declaration.
- `tmp/support-config-interface.json`, `tmp/visual-debug-support-interface.json`, and three named docs - fixture and closure artifacts.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` lines 253-259, 1144-1217 - version and plan types.
- `crates/slicer-sdk/src/prepass_types.rs` lines 260-287 and `prepass_builders.rs` lines 294-360 - SDK types/builder.
- WIT lines 1-61; macro lines 2300-2343; host lines 1203-1218 and 4160-4195; marshal lines 709-768.
- `traditional-support/traditional-support.toml` lines 9-15 and tree manifest lines 9-15 - IR access declarations.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate only; never load.
- `target/`, generated guest WASM, lockfiles, and vendored dependencies - never load.
- Raft, scheduler selection, fallback clipping, and final G-code paths - no edits.

## Expected Sub-Agent Dispatches

- Question: enumerate all `SupportPlanIR` literals and old `1.3.0` assertions; scope `crates/**/*.rs,modules/**/*.rs`; return: `LOCATIONS`; purpose: schema blast radius.
- Question: map WIT generated-binding, SDK builder/type, macro, host collection, and marshal owners; scope the paths listed above; return: `LOCATIONS`; purpose: contract implementation.
- Question: adjudicate top/bottom structural convention; scope `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY`; purpose: parity framing.

## Data and Contract Notes

- `SupportInterfacePlanEntry` fields: `global_layer_index: i32`, `object_id`, `region_id`, `kind`, `density: f32`, `spacing_mm: f32`.
- `SupportPlanIR.interface_plan` is `Vec<SupportInterfacePlanEntry>` with serde default; WIT carries records, not scan-line points.
- Host collection stores raw WIT entries alongside branch entries and raft plan; marshal converts kind and IDs into IR types.
- Stable ordering is planner emission order; no path exists without a plan record.

## Locked Assumptions and Invariants

- Top planned layer is `125`; bottom planned layer is `0` in the authored visual request.
- Bottom count `-1` and absent key are disabled and produce no code `1003`.
- Exact configured counts are tested: top `2`, bottom `3`.

## Risks and Tradeoffs

- The additive field has a broad literal and schema-assertion blast radius; Steps 2-3 explicitly own every current site before transport work.
- Small duplicated scan-line helpers are preferred over a new cross-module dependency.
- Visual capture shape is asserted by `jq` on both boolean interface flags, not by a serialized field-name grep.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: `SupportPlanIR`/schema inventory, `LOCATIONS`.

## Open Questions

None.
