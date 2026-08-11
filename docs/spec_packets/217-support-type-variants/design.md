# Design: support-type-variants

## Controlling Code Paths

- Primary code path: `crates/slicer-scheduler/src/execution_plan.rs:219-252` preserves claim selection; `crates/slicer-wasm-host/src/execution_plan_live.rs:261-280` forwards raw `support_type`; `modules/core-modules/support-planner/src/lib.rs:133-220,390-427` parses planner config and collects contacts.
- Neighboring tests/fixtures: `crates/slicer-scheduler/src/execution_plan.rs:1215-1386`; `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs`; `tmp/visual-debug-tree.json`; `tmp/visual-debug-support-manual.json`; and `tmp/support-config-manual.json`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Keep `support_generator_preferred_module_id` private and unchanged as the two-way claim resolver; mode selection is a separate planner concern.
- The planner's existing `ConfigView::get` path is the configuration boundary; do not infer mode from the selected module ID.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a private or module-local `SupportGenerationMode` with `Auto` and `Manual`, parse recognized `support_type` strings in `SupportPlanner::from_config`, and branch contact collection before `plan_for_object` builds `contacts_by_layer`.
- Exact functions, traits, manifests, tests, and fixtures: `SupportPlanner::from_config`; `SupportPlanner::plan_for_object`; `detect_overhang_facets`; `collect_paint_enforcer_contacts`; `support-planner.toml` `[config.schema.support_type]`; scheduler tests that prove the unchanged resolver; planner tests and model visual-debug request/manifest assertion.
- Rejected alternatives and reasons: changing scheduler claim resolution would violate the approved two-way split; selecting mode from module ID would conflate module implementation with auto/manual policy; adding a new support module is unnecessary.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` - role: planner mode and contact source; expected change: parse `support_type` and skip overhang detection in manual mode.
- `modules/core-modules/support-planner/support-planner.toml` - role: config forwarding/schema; expected change: declare the `support_type` string field if required by config scoping.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` - role: focused planner regression; expected change: prove auto and manual contact sources.
- `tmp/visual-debug-tree.json` - role: auto model-mode request; expected change: request `PrePass::SupportGeometry` alongside `Layer::Support`.
- `tmp/support-config-manual.json` - role: manual model-mode settings; expected change: set `support_type` to `tree(manual)` for the painted-enforcer fixture.
- `tmp/visual-debug-support-manual.json` - role: manual model-mode request; expected change: request both support taps against the enforcer fixture.

## Read-Only Context

- `crates/slicer-scheduler/src/execution_plan.rs` lines `219-252,1215-1386` only - unchanged resolver and tests.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` lines `261-280` only - raw config forwarding.
- `modules/core-modules/support-planner/src/lib.rs` lines `60-101,133-220,389-470,1009-1078` only - planner state/config/contact collection.

## Out-of-Bounds Files

- `crates/slicer-scheduler/src/execution_plan.rs` - read-only; do not edit resolver.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` - read-only; no forwarding redesign.
- `target/`, generated WASM, lockfiles, and `OrcaSlicerDocumented/` - never load directly.
- Fallback, raft, interface, marshalling, and G-code crates - outside this packet.

## Expected Sub-Agent Dispatches

- Question: enumerate all `SupportPlanner` config schema/fixture consumers; scope: `modules/core-modules/support-planner/**`; return: `LOCATIONS`; purpose: prevent a missing schema or struct-literal site.
- Question: confirm Orca auto/manual support policy; scope: `OrcaSlicerDocumented/src/libslic3r/Support/{SupportMaterial.cpp,TreeSupport.cpp}`; return: `SUMMARY`; purpose: parity adjudication.
- Question: verify visual-debug model request call sites and manifest assertion shape; scope: `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`; return: `LOCATIONS`; purpose: choose a real e2e test binary.
- Question: verify the painted-enforcer fixture resolves at the selected layer and produces `SupportGeometryIR`/`SupportPlanIR` captures; scope: `resources/bridge_support_enforcers.3mf`, `tmp/visual-debug-support-manual.json`; return: `FACT`; purpose: keep the manual visual-debug gate runnable.

## Data and Contract Notes

- IR/manifest contracts: no IR or manifest identifier changes; `support_type` remains a raw config string and `SupportPlanIR` shape is unchanged.
- WIT boundary: no WIT identifier changes; `ConfigView` already supplies planner config.
- Determinism/scheduler constraints: preserve scheduler claim dedup and deterministic contact ordering; manual mode only removes automatically detected contacts.

## Locked Assumptions and Invariants

- `tree(auto)` and `tree(manual)` select tree-support; `classic(auto)`, `classic(manual)`, `normal(auto)`, and unrecognized/absent values retain existing scheduler outcomes.
- Manual mode never calls `detect_overhang_facets`; auto mode continues to call it and also collects enforcers.
- Painted enforcers are identified by `support_enforcer` or `SupportEnforcer`, as implemented by `collect_paint_enforcer_contacts`.

## Risks and Tradeoffs

- If config scoping excludes `support_type`, adding only planner Rust code will silently leave every run in auto mode; the manifest schema and a config-forwarding assertion must catch this.
- Existing generated guest artifacts can mask source changes; the required staleness check is part of the step contract.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: config-consumer inventory, `LOCATIONS`.

## Open Questions

None.
