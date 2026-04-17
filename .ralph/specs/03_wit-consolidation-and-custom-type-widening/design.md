# Design: wit-consolidation-and-custom-type-widening

## Controlling Code Paths

- Primary code path: `wit/` (canonical WIT source), `crates/slicer-host/src/wit/`, `crates/slicer-sdk/wit-guest/`, `modules/core-modules/*/wit-guest/`
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` (to be added)
  - `crates/slicer-host/tests/custom_type_roundtrip_tdd.rs` (to be added)
- OrcaSlicer comparison surface: None

## Architecture Constraints

- All WIT files must live under `wit/` at the repo root. No crate may have its own copy.
- All consuming crates must import WIT via workspace dependencies or path dependency pointing at the canonical `wit/` source.
- The `wit-world` allowlist must be derived from the installed WIT worlds, not hardcoded.
- Custom string payloads must round-trip through the WASM boundary without loss of data.

## Proposed Changes

### TASK-144 — WIT Source Consolidation

1. **Audit current WIT file locations**: Find all WIT files across `crates/`, `modules/`, and any other locations. Identify duplicates and divergent copies.
2. **Establish canonical source**: Designate the `wit/` directory as the single source of truth. Ensure all WIT files (types.wit, config.wit, ir-types.wit, world-*.wit, host-api.wit) are present.
3. **Update host build**: Point `crates/slicer-host/witgen/` (or similar) at the canonical `wit/` source. Remove any local WIT copies.
4. **Update macro/guest build**: Point all `wit-guest/` crates at the canonical `wit/` source via workspace or path dependencies.
5. **Verify no duplicates remain**: `find . -name "*.wit" -not -path "./wit/*"` should return nothing.

### TASK-145 — Identifier Normalization and Drift Detection

6. **Normalize package names and versions**: Across all WIT files, ensure `package slicer:types@1.0.0;` etc. are consistent. Verify generated bindings match.
7. **Add schema constants**: If there are Rust constants derived from WIT (e.g., `WIT_WORLD_LAYER_V100`), ensure they match the WIT package declarations.
8. **Add drift-detection regression tests**: A test that hashes all WIT files in `wit/` and compares against a known-good snapshot. Fail if they diverge.

### TASK-146 — wit_world Allowlist Validation

9. **Implement allowlist at startup**: At manifest load, read the module's `wit-world` field and validate it against the host's known-valid identifiers (e.g., `slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, etc.).
10. **Emit precise diagnostics on rejection**: Include expected worlds, received value, and module id.

### TASK-149/150 — Custom Type Widening

11. **Widen ExtrusionRole in WIT**: Add `custom(String)` variant to the `extrusion-role` enum in `deps/types.wit`. Update Rust `ExtrusionRole` enum to match.
12. **Update host converter**: Ensure `ExtrusionRole::Custom(s)` is serialized/deserialized correctly across the WIT boundary without truncation or lossy conversion.
13. **Widen PaintSemantic in WIT**: Add `custom(String)` variant to the `paint-semantic` enum in `deps/ir-types.wit`. Update Rust `PaintSemantic` enum to match.
14. **Update host converter**: Ensure `PaintSemantic::Custom(s)` round-trips correctly.
15. **Widen WallFeatureFlags in WIT**: Add `custom: list<(string, paint-value)>` field to the `wall-feature-flag` record in `deps/ir-types.wit`. Update Rust `WallFeatureFlags` struct to match.
16. **Update host converter**: Ensure `custom` map round-trips correctly with exact key/value preservation.
17. **Add round-trip WIT regression tests**: Tests that create `ExtrusionRole::Custom`, `PaintSemantic::Custom`, and `WallFeatureFlags.custom` values, cross the WIT boundary, and assert the exact same values come back.

## Data and Contract Notes

- WIT custom variants must use the same string format as the IR types (e.g., `"com.example/role@1"` for `PaintSemantic::Custom`).
- Custom payloads must not be truncated, lowercased, or otherwise transformed at the boundary.
- The allowlist for `wit_world` is derived from the installed WIT worlds in `wit/world-*.wit`.

## Risks and Tradeoffs

- Widening WIT types is a breaking change for generated bindings. All guest modules must be rebuilt if their WIT changes. However, this is a necessary change for DEV-016 closure.
- Consolidation of WIT sources may surface build-time dependency cycles if not done carefully. Ensure WIT is a leaf dependency.
- Drift-detection tests must be robust to false positives (e.g., timestamps in generated files). Use content hashing, not metadata.

## Open Questions

- Are there existing generated bindings that would need regeneration after WIT changes? Check `crates/slicer-host/src/generated/` or similar.
- Does the macro (wit-guest) build already import from a shared `wit/` source, or does each module have its own copy?
- Is there an existing `wit_world` validation in the host scheduler, or does it need to be built from scratch?
- What is the current state of `ExtrusionRole::Custom`, `PaintSemantic::Custom`, and `WallFeatureFlags.custom` in the WIT files? Do they already exist but just not round-trip correctly?