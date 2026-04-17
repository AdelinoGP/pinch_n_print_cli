# Requirements: wit-consolidation-and-custom-type-widening

## Packet Metadata

- Grouped task IDs:
  - `TASK-144` — Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/`. Covers DEV-014.
  - `TASK-145` — Normalize WIT package/version identifiers and restore missing members across canonical WIT surface, generated bindings, schema constants, and test guests; add drift-detection regression. Continues DEV-014.
  - `TASK-146` — Add host-side `wit_world` allowlist validation using canonical identifiers and reject mismatched manifests at startup. Covers DEV-014 and DEV-026.
  - `TASK-149` — Widen WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the boundary losslessly. Covers DEV-016.
  - `TASK-150` — Update host, macro, and guest converters to preserve the widened custom payloads and add round-trip WIT regression tests. Continues DEV-016.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-014: WIT compatibility is split across multiple sources and still drifts. Host, macro (wit-guest), and guest builds each have their own copy or view of WIT files, leading to version mismatches at both build time and runtime.

DEV-016: Custom string payloads (`ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom`) are dropped at the WIT boundary because the current WIT types do not preserve the string content. Community modules that use custom extrusion roles or custom paint semantics lose data when crossing the boundary.

DEV-026: Manifest validation does not check `wit_world` against the host's known-valid identifiers, so a module built against a different WIT world can be loaded and cause confusing failures at call time.

## In Scope

- Audit all current WIT file locations across host, macro (wit-guest), and guest crates.
- Establish `wit/` at the repo root as the single canonical source for all WIT files.
- Update all consuming crates (host, macro, guest) to import from `wit/` with no local copies.
- Normalize all package names and version identifiers across WIT files, generated bindings, and schema constants.
- Add drift-detection regression tests that detect if WIT files diverge from the canonical source.
- Implement `wit_world` allowlist validation at manifest load time.
- Widen `extrusion-role` in WIT to include a `custom(String)` variant; update corresponding Rust type and converters.
- Widen `paint-semantic` in WIT to include a `custom(String)` variant; update converters.
- Add `custom: HashMap<String, PaintValue>` field to `WallFeatureFlags` in WIT and converters.
- Add round-trip WIT regression tests proving custom payloads cross the boundary losslessly.

## Out of Scope

- Manifest population (TASK-121/TASK-122 — separate packet).
- Runtime access audit (TASK-123/124/125/126 — separate packet).
- OrcaSlicer geometry parity (Workstream 3).
- Python postpass decision (Workstream 4).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT File Organization, WIT ↔ IR Compatibility Matrix, Module Manifest Schema
- `docs/02_ir_schemas.md` — `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom`
- `docs/04_host_scheduler.md` — manifest validation at startup
- `docs/00_project_overview.md` — Repository Structure (wit/ directory location)

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- Host, macro, and guest all consume the same canonical WIT files from `wit/` with no duplicates.
- All package name and major version identifiers are normalized across WIT, generated bindings, and schema constants.
- Drift-detection tests run on every build and fail if WIT files diverge.
- `wit_world` allowlist validation rejects mismatched manifests at startup.
- `ExtrusionRole::Custom(String)` round-trips correctly through WIT boundary.
- `PaintSemantic::Custom(String)` round-trips correctly through WIT boundary.
- `WallFeatureFlags.custom` round-trips correctly through WIT boundary.
- Round-trip WIT regression tests pass.

## Verification Commands

- `cargo build --package slicer-host --package slicer-sdk` (verifies canonical WIT consumption)
- `cargo test --package slicer-host --test wit_drift_detection -- --nocapture`
- `cargo test --package slicer-host --test custom_type_roundtrip -- --nocapture`
- `cargo run --package slicer-host -- manifest-validate --module-path modules/core-modules` (verifies wit_world validation)