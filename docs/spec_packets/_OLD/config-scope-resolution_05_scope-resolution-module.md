---
status: implemented
packet: config-scope-resolution_05_scope-resolution-module
task_ids:
  - TASK-566
---

# config-scope-resolution_05_scope-resolution-module

## Goal

Replace prefix-driven and default-difference config merging with one `slicer-config` resolution module that resolves typed scope deltas for both layer-planning Z-grid input and region scope stacks, then carries typed per-object planning inputs across `prepass-layer-planning@2.0.0`.

## Problem Statement

The current host resolves global, object, paint-semantic, and tool values through `resolve_global_config`, `resolve_per_object_configs`, `resolve_per_paint_semantic_configs`, `resolve_per_tool_configs`, and private `apply_overlay` in `crates/slicer-scheduler/src/config_resolution.rs`, then applies modifiers/paint/tool through `overlay_resolved` in `crates/slicer-core/src/algos/region_mapping.rs`. The former reparses wire prefixes and the latter infers presence by comparing full records with defaults, so explicit default-valued overrides and fields omitted from the hand-written merge are lost. `run_slice_with_collector` and `prepare_prepass_context` also independently format host-only object namespaces, and the layer-planner guest repeats that protocol across WIT.

## Architecture Constraints

- Scope values are deltas. Presence is `ScopeDelta.values.contains_key`, never inequality with `ResolvedConfig::default()`.
- One ordered engine owns both queries. `query_z_grid` uses global/object data today; `resolve_scope_stack` additionally applies modifier, lexical paint-semantic, and tool deltas. Row 9 inserts layer-range values between object and modifier without introducing a second resolver.
- The settled future overlap contract is not optional: later-starting `layer_height` wins while re-deriving the Z-grid; conflicting values of the same non-`layer_height` key are a load error. This packet documents and preserves the insertion seam but does not fabricate a current Rust layer-range implementation.
- Phase-B expansion runs after the complete applicable merge and before a `ResolvedConfig` is interned or a planning record crosses WIT.
- `prepass-layer-planning.run` parameter addition is one major package bump to `2.0.0`; update every hard-coded package/export assertion in the same coordinated step.
- The five-field host struct and WIT record must remain field-for-field adapters: `object_id/object-id`, `object_height/object-height`, `layer_height/layer-height`, `first_layer_height/first-layer-height`, and `support_raft_layers/support-raft-layers`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: no IR schema or manifest vocabulary change. `ResolvedConfig` remains the final typed host record and extension carrier.
- WIT boundary: package identity changes from `slicer:prepass-layer-planning@1.0.0` to `@2.0.0`; the new parameter is intentionally incompatible and no v1 shim is retained.
- Determinism/scheduler constraints: object results are object-id sorted; modifiers are priority ascending with existing stable tie behavior; paint semantics are lexical; tool applies last.
- Layer range: precedence position and conflict policy are locked now; runtime interval and overlap logic remains absent until row 9.

## Locked Assumptions and Invariants

- Packet-03 and packet-04 exports are FORWARD-DEPs until landed; no implementation may guess around a mismatch.
- Absence remains absence. Explicitly stating a registry default at a narrower scope overrides a broader non-default.
- Both production entry points call the same two public resolver queries.
- `ConfigView` consumers receive no packet-04 placeholder from these paths.
- `layer_height_profile_from_ranges` and `LayerRanges::assign` are canonical-plan references, not pre-existing Rust symbols.
- Future layer-range overlap semantics are exactly: later-starting wins for `layer_height`; different values of the same non-`layer_height` key overlap are a load error.

## Risks and Tradeoffs

- The WIT bump has a broad compile blast radius; splitting declaration, adapters, dispatch, and guests into explicit steps limits context while they still land together.
- Removing scheduler APIs can break tests that were testing validation through those wrappers; preserve behavior by moving those tests to the unified resolver rather than deleting coverage.
- Converting `support_raft_layers` from a registry value must reject negative/out-of-range data rather than silently cast.
- A tempting premature layer-range enum would create an untested geometry contract; the public target deliberately leaves that field for row 9.
