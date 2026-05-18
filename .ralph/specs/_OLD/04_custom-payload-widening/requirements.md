# Requirements: 04_custom-payload-widening

## Packet Metadata

- Grouped task IDs:
  - `TASK-149` — Widen the WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the boundary losslessly. Covers DEV-016.
  - `TASK-150` — Update host, macro, and guest converters to preserve the widened custom payloads and add round-trip WIT regression tests. Continues DEV-016.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Three distinct custom payload types are silently dropped at the WIT boundary (DEV-016):

1. **`ExtrusionRole::Custom(String)`**: The WIT `enum extrusion-role { ..., custom }` is arity-0 (no fields). The macro converter at `__slicer_ir_role_to_wit` (~line 1774 in `lib.rs`) synthesizes `String::new()` instead of carrying the actual string. Similarly, host converters in `wit_host.rs` at 5 known sites synthesize `String::new()`. Module authors who use `ExtrusionRole::Custom(...)` lose the string payload.

2. **`PaintSemantic::Custom(String)`**: The WIT `enum paint-semantic { ..., custom }` is arity-0. The macro converter at `__slicer_wit_semantic_to_ir` (~line 1619) synthesizes `String::new()`. The paint layer routing (`ir_to_wit_paint_semantic` ~line 1406) intentionally skips `Custom`. The string payload is lost.

3. **`WallFeatureFlags.custom: HashMap<String, PaintValue>`**: The WIT `record wall-feature-flag` has no `custom` field. The converter at `__slicer_wit_feature_to_ir` (~line 1589) has no `custom` field to populate. Any custom paint values on wall segments are silently discarded because the field doesn't exist in WIT yet.

No test in `macros/tests/` or `host/tests/` verifies payload survival across the WIT boundary. Existing tests using `PaintSemantic::Custom(...)` operate at the host-IR level only.

This packet widens the WIT types and updates all converters to preserve payloads.

## In Scope

- Change `wit/deps/types.wit` `enum extrusion-role` → `variant extrusion-role` with `custom(string)` variant
- Change `wit/deps/ir-types.wit` `enum paint-semantic` → `variant paint-semantic` with `custom(string)` variant
- Change `wit/deps/ir-types.wit` `record wall-feature-flag` to add `custom: list<tuple<string, paint-value>>` field (WIT-compatible representation of `HashMap<String, PaintValue>`; wasmtime supports tuples in lists; this field does not yet exist and must be added)
- Update macro converters in `crates/slicer-macros/src/lib.rs` (`__slicer_ir_role_to_wit`, `__slicer_wit_semantic_to_ir`, `__slicer_ir_feature_to_wit`, `__slicer_wit_feature_to_ir`, `ir_to_wit_paint_semantic`)
- Update host converters in `crates/slicer-host/src/wit_host.rs` (5 `String::new()` synthesis sites plus the `convert_wall_feature_flag` function for the new custom field)
- Add round-trip WIT regression tests for all three custom payloads
- Verify the changes compile with the canonical WIT source (after Packet A consolidation)

## Out of Scope

- Changes to `slicer:world-prepass.wit`, `slicer:world-postpass.wit`, or `slicer:world-finalization.wit` beyond propagating the widened type imports
- Changes to IR serialization (IR already has `Custom(String)` and `HashMap` — no IR change needed)
- Changes to module authors' code — modules that use `ExtrusionRole::Custom` will automatically get the widened boundary once the types and converters are updated
- Changes to downstream GCode consumers that interpret custom roles/semantics (those are future work)
- Changes to `min_host_version` or other manifest fields

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `deps/types.wit`, `deps/ir-types.wit` sections
- `docs/02_ir_schemas.md` — `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom` IR definitions
- `crates/slicer-macros/src/lib.rs` — converter function locations (actual lines: `__slicer_ir_role_to_wit` ~1774, `__slicer_wit_semantic_to_ir` ~1619, `__slicer_ir_feature_to_wit` ~1822, `__slicer_wit_feature_to_ir` ~1589, `ir_to_wit_paint_semantic` ~1406)
- `crates/slicer-host/src/wit_host.rs` — host-side converter locations (5 `String::new()` synthesis sites for custom variants, and `convert_wall_feature_flag` for the new custom field)

## OrcaSlicer Reference Obligations

None. This is an internal WIT boundary type-widening task.

## Acceptance Summary

- Positive cases:
  - `wit/deps/types.wit` defines `variant extrusion-role { ..., custom(string) }`
  - `wit/deps/ir-types.wit` defines `variant paint-semantic { ..., custom(string) }`
  - `wit/deps/ir-types.wit` defines `record wall-feature-flag` with `custom: list<tuple<string, paint-value>>`
  - `__slicer_ir_role_to_wit` correctly encodes `Custom("...")` with string payload and decodes WIT `custom(string)` back to `Custom("...")`
  - `__slicer_wit_semantic_to_ir` correctly encodes `Custom("...")` and decodes WIT variant back
  - `__slicer_ir_feature_to_wit` / `__slicer_wit_feature_to_ir` correctly encode/decode the custom map and WIT list back to `HashMap`
  - All three round-trip tests pass with actual payload assertions
  - Full workspace build and clippy pass
- Negative cases:
  - Arity-0 `custom` enum before the fix causes converter to synthesize `String::new()` (old broken behavior)
  - Empty string `Custom("")` preserved as `Custom("")`, not dropped
  - Multiple custom entries in `WallFeatureFlags.custom` all survive round-trip
- Measurable outcomes:
  - Zero `String::new()` synthesis for custom variants in converters after fix
  - All three custom payload round-trip tests assert non-empty payload (or expected empty string for that case)
  - Clippy zero warnings

## Cross-Packet Dependencies

- This packet modifies `wit/deps/types.wit` and `wit/deps/ir-types.wit` — the same canonical files consolidated in `03_wit-canonical-source-and-validation`. Run Packet A first or run both together. Packet A's drift-detection test (`wit_drift_detection_tdd`) must be updated to reflect the new widened types.

## Verification Commands

- `cargo build --package slicer-macros`
- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- --nocapture`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` (Packet A test — must pass after Packet B's WIT changes)
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

Each step in `implementation-plan.md` must produce:
- Step 1 (WIT type changes): Disk `wit/deps/types.wit` and `wit/deps/ir-types.wit` updated with widened types; diff shows `enum` → `variant` for extrusion-role and paint-semantic; `wall-feature-flag` has new `custom` field
- Step 2 (Macro converter): `__slicer_ir_role_to_wit`, `__slicer_wit_semantic_to_ir`, `__slicer_ir_feature_to_wit`, `__slicer_wit_feature_to_wit`, `ir_to_wit_paint_semantic` in `lib.rs` updated; macro crate builds
- Step 3 (Host converter): Host `wit_host.rs` converters updated for all three types; host crate builds
- Step 4 (Round-trip tests): Three new test cases in `macro_all_worlds_roundtrip_tdd.rs` (or new file) that assert payload survival for each custom type
- Step 5 (Workspace gate): Full build and clippy pass
