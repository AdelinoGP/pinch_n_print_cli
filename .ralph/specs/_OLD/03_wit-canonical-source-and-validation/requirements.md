# Requirements: 03_wit-canonical-source-and-validation

## Packet Metadata

- Grouped task IDs:
  - `TASK-144` — Consolidate host, macro, and guest codegen onto one canonical shared WIT source rooted in `wit/`. Covers DEV-014.
  - `TASK-145` — Normalize WIT package/version identifiers and restore missing members across the canonical WIT surface, generated bindings, schema constants, and test guests; add drift-detection regression coverage. Continues DEV-014.
  - `TASK-146` — Add host-side `wit_world` allowlist validation using the canonical identifiers and reject mismatched manifests at startup. Covers the validation slice of DEV-014 and DEV-026.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

WIT compatibility is split across three sources: (1) on-disk `wit/` directory, (2) macro inline WIT in `crates/slicer-macros/src/lib.rs` (the `build_*_world_glue` functions), and (3) host inline WIT in `crates/slicer-host/src/wit_host.rs`. This duplication has caused real drift:

1. **Package name drift**: Host inline WIT uses `slicer:layer-world@1.0.0` and `slicer:prepass-world@1.0.0`; canonical on-disk files use `slicer:world-layer@1.0.0` and `slicer:world-prepass@1.0.0`. The macro uses the canonical names. DAG construction code (`dag.rs:158`, `execution_plan.rs:858`) hardcodes the canonical names.
2. **ir-types version drift**: On-disk canonical is `slicer:ir-types@1.1.0`; some inline copies may reference `1.0.0`.
3. **Missing members**: `needs-support` is absent from inline WIT copies; `push-z-hop` is absent from postpass inline `gcode-output-builder`.
4. **No drift detection**: No test verifies the three copies stay in sync.

This packet consolidates onto one canonical source (`wit/`) and adds startup `wit_world` validation.

If this packet reopens or narrows a prior packet: this is the first WIT consolidation packet for Workstream 1. There is no prior WIT consolidation packet.

## In Scope

- Replace macro inline WIT (`lib.rs` `build_*_world_glue`) with WIT-level `include` directives inside const string literals (pointing to canonical `wit/deps/` files). Note: `include_str!` was not used — WIT-level `include` directives inside `const` strings work because `wit_bindgen::generate!` processes the WIT string with its own parser.
- Replace host inline WIT (`wit_host.rs` inline `bindgen!` blocks) — **not feasible with `include_str!`**: wasmtime's `bindgen!` requires fully-expanded inline WIT; disk files use `import slicer:...` package references that cannot be resolved at `bindgen!` compile time. Host inline WIT is retained with expanded interfaces. This is documented as a deviation from the "eliminate all inline copies" goal.
- Normalize `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` and `slicer:prepass-world@1.0.0` → `slicer:world-prepass@1.0.0` in host inline WIT
- Normalize `slicer:ir-types@1.0.0` → `slicer:ir-types@1.1.0` wherever it diverges
- Restore missing `needs-support` interface in `deps/ir-types.wit`
- Restore missing `push-z-hop` in postpass `gcode-output-builder` in on-disk WIT
- Add `wit_world` allowlist validation at module load using four canonical identifiers
- Add `wit_drift_detection_tdd.rs` regression test
- Update schema/CLI constants that reference WIT identifiers to use canonical names

## Out of Scope

- Custom payload widening (`ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom`) — those are in `04_custom-payload-widening`
- Changes to IR schema versions or IR major-version compatibility checking (already partially done in existing code)
- Changes to `min_host_version` enforcement (belongs to TASK-154)

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — canonical WIT structure, package naming conventions, module manifest schema
- `docs/04_host_scheduler.md` — module load validation, manifest ingestion
- `crates/slicer-macros/src/lib.rs` — `WIT_WORLD_MAP`, `build_*_world_glue` functions
- `crates/slicer-host/src/wit_host.rs` — host inline WIT `bindgen!` blocks
- `crates/slicer-host/src/manifest.rs` — `Manifest` struct, `wit_world` field
- `crates/slicer-host/src/dag.rs:158` — hardcoded `wit_world` in test helper
- `crates/slicer-host/src/execution_plan.rs:858` — hardcoded `wit_world` in test helper
- `wit/` directory — canonical on-disk WIT files

## OrcaSlicer Reference Obligations

None. This is an internal WIT infrastructure task.

## Acceptance Summary

- Positive cases:
  - Macro and host both compile using `include_str!` references to canonical `wit/` files
  - All four worlds (`world-layer`, `world-prepass`, `world-postpass`, `world-finalization`) load correctly with canonical package names
  - `wit_world` allowlist accepts `slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`
  - `needs-support` interface is present in `deps/ir-types.wit`
  - `push-z-hop` is present in postpass `gcode-output-builder`
  - Drift detection test passes (zero drift between disk and embedded)
- Negative cases:
  - Manifest with `slicer:layer-world@1.0.0` (wrong package name) is rejected with fatal diagnostic
  - Manifest with future major version (`slicer:world-layer@2.0.0`) is rejected with fatal diagnostic
  - Post-consolidation disk modification is caught by drift detection test
- Measurable outcomes:
  - Zero drift between on-disk `wit/` and embedded WIT strings in macro and host
  - Exactly four allowlisted `wit_world` identifiers
  - `wit_drift_detection_tdd.rs` passes with 100% coverage of all four worlds and three dependency interfaces

## Verification Commands

- `cargo build --package slicer-macros`
- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo clippy --package slicer-macros --package slicer-host -- -D warnings`

## Step Completion Expectations

Each step in `implementation-plan.md` must produce:
- Step 1 (WIT audit): An inventory of all inline WIT strings in macro and host with their file:line locations and the canonical file they should reference
- Step 2 (Macro consolidation): `lib.rs` uses `include_str!` for all WIT imports; `build_*_world_glue` compiles
- Step 3 (Host consolidation): `wit_host.rs` uses `include_str!` for all WIT imports; `wit_host` module compiles
- Step 4 (Identifier normalization): Host inline WIT uses `slicer:world-layer@1.0.0` not `slicer:layer-world@1.0.0`; same for prepass
- Step 5 (Missing members): `needs-support` in `ir-types.wit`; `push-z-hop` in postpass; disk files updated
- Step 6 (Allowlist validation): Host startup rejects non-allowlisted `wit_world` with fatal diagnostic; existing modules with canonical names load successfully
- Step 7 (Drift detection): `wit_drift_detection_tdd.rs` created and passes; any disk modification causes test failure
- Step 8 (Schema constants): Schema/CLI constants updated to canonical names
