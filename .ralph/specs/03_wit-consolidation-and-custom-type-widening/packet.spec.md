---
status: pending
packet: wit-consolidation-and-custom-type-widening
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
  - TASK-149
  - TASK-150
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: wit-consolidation-and-custom-type-widening

## Goal

Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/` (DEV-014), normalize WIT package/version identifiers, restore missing members, add drift-detection regression, add `wit_world` allowlist validation (DEV-026), and widen WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` cross the boundary losslessly (DEV-016).

## Scope Boundaries

- In scope:
  - TASK-144: Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/`. Covers DEV-014.
  - TASK-145: Normalize WIT package/version identifiers and restore missing members across canonical WIT surface, generated bindings, schema constants, and test guests; add drift-detection regression. Continues DEV-014.
  - TASK-146: Add host-side `wit_world` allowlist validation using canonical identifiers and reject mismatched manifests at startup. Covers DEV-014 and DEV-026.
  - TASK-149: Widen WIT types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the boundary losslessly. Covers DEV-016.
  - TASK-150: Update host, macro, and guest converters to preserve the widened custom payloads and add round-trip WIT regression tests. Continues DEV-016.

- Out of scope:
  - TASK-121/TASK-122 (manifest population — separate packet)
  - TASK-123/124/125/126 (runtime access audit — separate packet)
  - OrcaSlicer geometry parity (Workstream 3)

## Acceptance Criteria

- **Given** the host, macro, and guest build systems, **when** each is built, **then** all three consume the same canonical WIT files rooted at `wit/` with no duplicate copies.
- **Given** a WIT package or version identifier discrepancy between the canonical WIT and generated bindings, **when** the discrepancy is detected, **then** a build-time or startup error is raised with the specific mismatch named.
- **Given** a manifest with a `wit_world` value, **when** the host starts, **then** it validates the value against the canonical identifier allowlist and rejects mismatched manifests with precise diagnostics.
- **Given** a module that emits an `ExtrusionRole::Custom("foo")`, **when** the role crosses the WIT boundary, **then** the string `"foo"` is preserved exactly and round-trips correctly through the host converter.
- **Given** a module that emits a `PaintSemantic::Custom("com.example/semantic@1")`, **when** the semantic crosses the WIT boundary, **then** the string is preserved exactly and round-trips correctly.
- **Given** a wall segment with `WallFeatureFlags.custom` entries, **when** they cross the WIT boundary, **then** all custom key/value pairs are preserved exactly.
- **Given** the WIT custom-type widening, **when** round-trip regression tests run, **then** `ExtrusionRole::Custom`, `PaintSemantic::Custom`, and `WallFeatureFlags.custom` all pass with exact payload preservation.

## Verification

- `cargo build --package slicer-host` (verifies canonical WIT is used)
- `cargo test --package slicer-host --test wit_drift_detection -- --nocapture` (once added)
- `cargo test --package slicer-host --test custom_type_roundtrip -- --nocapture` (once added)
- Manual: `find wit/ -name "*.wit" | xargs md5sum` — all consuming crates use identical files
- Startup test: manifest with wrong `wit_world` is rejected with precise error

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT file organization, WIT ↔ IR Compatibility Matrix, Module Manifest Schema
- `docs/02_ir_schemas.md` — `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom`
- `docs/01_system_architecture.md` — Host-Boundary Access Enforcement (for custom type handling)
- `docs/04_host_scheduler.md` — manifest validation at startup

## OrcaSlicer Reference Obligations

None. This is an infrastructure/task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`