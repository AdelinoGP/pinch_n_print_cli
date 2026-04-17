# Implementation Plan: wit-consolidation-and-custom-type-widening

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit current WIT file distribution

- Task IDs:
  - `TASK-144`
- Objective: Find all `.wit` files across the repository, identify duplicates, divergent copies, and consuming crate paths.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/00_project_overview.md` — Repository Structure
  - `docs/03_wit_and_manifest.md` — WIT File Organization
- OrcaSlicer refs: None
- Verification: `find . -name "*.wit" -not -path "./.git/*"` returns complete inventory with paths.

### Step 2: Establish canonical WIT source at wit/

- Task IDs:
  - `TASK-144`
- Objective: Designate `wit/` as the single source of truth. Copy or move all canonical WIT files there. Remove duplicates from other locations.
- Files expected to change:
  - `wit/` (canonical WIT files)
  - Remove copies from `crates/`, `modules/`, etc.
- Authoritative docs:
  - `docs/00_project_overview.md` — Repository Structure
- OrcaSlicer refs: None
- Verification: `find . -name "*.wit" -not -path "./wit/*" -not -path "./.git/*" | wc -l` returns 0.

### Step 3: Update host build to consume canonical WIT

- Task IDs:
  - `TASK-144`
- Objective: Update `crates/slicer-host/build.rs` or `Cargo.toml` to use the canonical WIT from `wit/` via path dependency. Regenerate bindings.
- Files expected to change:
  - `crates/slicer-host/Cargo.toml` (add path dependency on wit/)
  - `crates/slicer-host/build.rs` (update wit-bindgen invocation)
  - Generated binding files
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — WIT File Organization
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` succeeds with canonical WIT.

### Step 4: Update macro/guest builds to consume canonical WIT

- Task IDs:
  - `TASK-144`
- Objective: Update all `wit-guest/` crates across `modules/core-modules/*/wit-guest/` to import from the canonical `wit/` source. Remove any local WIT copies.
- Files expected to change:
  - `modules/core-modules/*/wit-guest/Cargo.toml` (update WIT path)
  - Remove local WIT copies in `modules/core-modules/*/wit/`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — WIT File Organization
- OrcaSlicer refs: None
- Verification: `cargo build --package <each-wit-guest-crate>` succeeds with canonical WIT.

### Step 5: Normalize WIT package/version identifiers

- Task IDs:
  - `TASK-145`
- Objective: Across all WIT files, ensure package names and major versions are consistent (e.g., `slicer:types@1.0.0` vs `slicer:types@1.0.1`). Update generated bindings and schema constants to match.
- Files expected to change:
  - `wit/**/*.wit` (canonical files)
  - Generated binding files
  - Schema constant files
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — WIT ↔ IR Compatibility Matrix
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wit_identifier_normalization -- --nocapture` (test to be added).

### Step 6: Add drift-detection regression tests

- Task IDs:
  - `TASK-145`
- Objective: Add a test that hashes all WIT files in `wit/` and compares against a known-good snapshot embedded in the test. Fail if they diverge.
- Files expected to change:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` (new file)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — WIT File Organization
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` passes with canonical WIT and fails if WIT is modified.

### Step 7: Implement wit_world allowlist validation at startup

- Task IDs:
  - `TASK-146`
- Objective: At manifest load time, validate the module's `wit-world` field against the host's known-valid identifiers derived from installed WIT worlds. Reject mismatched manifests with precise diagnostics.
- Files expected to change:
  - `crates/slicer-host/src/scheduler/manifest.rs` or similar (validation logic)
- Authoritative docs:
  - `docs/04_host_scheduler.md` — manifest validation at startup
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wit_world_allowlist_rejection -- --nocapture` (test with manifest having wrong wit_world).

### Step 8: Widen ExtrusionRole to include custom(String) in WIT

- Task IDs:
  - `TASK-149`
- Objective: Add `custom(String)` variant to `extrusion-role` enum in `wit/deps/types.wit`. Update Rust `ExtrusionRole` enum in `crates/slicer-ir/` to match.
- Files expected to change:
  - `wit/deps/types.wit`
  - `crates/slicer-ir/src/` (ExtrusionRole enum)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — ExtrusionRole definition
  - `docs/03_wit_and_manifest.md` — deps/types.wit
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` compiles with widened type.

### Step 9: Update host converter for ExtrusionRole custom payload

- Task IDs:
  - `TASK-150`
- Objective: Update the host-side WIT converter to correctly serialize/deserialize `ExtrusionRole::Custom(String)` so the string payload is preserved exactly.
- Files expected to change:
  - `crates/slicer-host/src/wit/converter.rs` (or similar)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — ExtrusionRole::Custom
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test extrusion_role_custom_roundtrip -- --nocapture` (test to be added).

### Step 10: Widen PaintSemantic to include custom(String) in WIT

- Task IDs:
  - `TASK-149`
- Objective: Add `custom(String)` variant to `paint-semantic` enum in `wit/deps/ir-types.wit`. Update Rust `PaintSemantic` enum to match.
- Files expected to change:
  - `wit/deps/ir-types.wit`
  - `crates/slicer-ir/src/` (PaintSemantic enum)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — PaintSemantic definition
  - `docs/03_wit_and_manifest.md` — deps/ir-types.wit
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` compiles with widened type.

### Step 11: Update host converter for PaintSemantic custom payload

- Task IDs:
  - `TASK-150`
- Objective: Update the host-side WIT converter to correctly serialize/deserialize `PaintSemantic::Custom(String)` so the string payload is preserved exactly.
- Files expected to change:
  - `crates/slicer-host/src/wit/converter.rs` (or similar)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — PaintSemantic::Custom
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test paint_semantic_custom_roundtrip -- --nocapture` (test to be added).

### Step 12: Widen WallFeatureFlags to include custom field in WIT

- Task IDs:
  - `TASK-149`
- Objective: Add `custom: list<tuple<string, paint-value>>` field to `wall-feature-flag` record in `wit/deps/ir-types.wit`. Update Rust `WallFeatureFlags` struct to match.
- Files expected to change:
  - `wit/deps/ir-types.wit`
  - `crates/slicer-ir/src/` (WallFeatureFlags struct)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — WallFeatureFlags definition
  - `docs/03_wit_and_manifest.md` — deps/ir-types.wit
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` compiles with widened type.

### Step 13: Update host converter for WallFeatureFlags.custom

- Task IDs:
  - `TASK-150`
- Objective: Update the host-side WIT converter to correctly serialize/deserialize the `custom` field so all key/value pairs are preserved exactly.
- Files expected to change:
  - `crates/slicer-host/src/wit/converter.rs` (or similar)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — WallFeatureFlags.custom
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wall_feature_flags_custom_roundtrip -- --nocapture` (test to be added).

### Step 14: Full round-trip regression test pass

- Task IDs:
  - `TASK-149`
  - `TASK-150`
- Objective: Run all custom-type round-trip tests and verify complete pass. Regenerate any affected guest bindings.
- Files expected to change: Generated bindings (rebuilt)
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test custom_type_roundtrip -- --nocapture` — all pass.

## Packet Completion Gate

- All WIT files consolidated under `wit/` with no duplicates.
- Host, macro, and guest builds all use the canonical WIT source.
- WIT package/version identifiers normalized and verified.
- Drift-detection regression tests in place and passing.
- `wit_world` allowlist validation active at startup.
- `ExtrusionRole::Custom(String)` round-trips correctly through WIT boundary.
- `PaintSemantic::Custom(String)` round-trips correctly through WIT boundary.
- `WallFeatureFlags.custom` round-trips correctly through WIT boundary.
- All round-trip regression tests pass.
- `docs/07_implementation_status.md` TASK-144/145/146/149/150 marked complete.
- `packet.spec.md` ready to move to `status: implemented`.