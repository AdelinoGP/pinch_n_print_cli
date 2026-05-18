---
status: implemented
packet: 03_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 03_wit-canonical-source-and-validation

## Goal

Consolidate WIT compatibility onto one canonical shared source rooted in `wit/` by replacing the three diverged inline WIT copies in `slicer-macros` (macro inline WIT in `lib.rs`), `slicer-host` (host inline WIT in `wit_host.rs`), and test guests, with `include_str!`-backed references to the on-disk canonical WIT files. Normalize all package/version identifiers to match the on-disk canonical. Add host-side `wit_world` allowlist validation using the canonical identifiers. Add drift-detection regression coverage.

## Scope Boundaries

- In scope:
  - TASK-144: Replace macro inline WIT (`lib.rs` `build_*_world_glue` functions) with `include_str!` references to canonical `wit/` files
  - TASK-144: Replace host inline WIT (`wit_host.rs` inline `bindgen!` blocks) with `include_str!` references to canonical `wit/` files
  - TASK-145: Normalize `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` in host inline WIT (line 179 of `wit_host.rs`)
  - TASK-145: Normalize `slicer:prepass-world@1.0.0` → `slicer:world-prepass@1.0.0` in host inline WIT (line 379 of `wit_host.rs`)
  - TASK-145: Normalize `slicer:ir-types@1.0.0` → `slicer:ir-types@1.1.0` wherever it diverges (on-disk canonical is `1.1.0`)
  - TASK-145: Restore missing `needs-support` interface in `deps/ir-types.wit`
  - TASK-145: Restore missing `push-z-hop` in postpass `gcode-output-builder` in on-disk WIT
  - TASK-145: Add drift-detection regression test proving disk WIT matches the embedded copies used by macro and host
  - TASK-146: Add host-side `wit_world` allowlist validation at startup using the four canonical world identifiers (`slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`)
  - TASK-146: Reject manifests with mismatched `wit_world` at module-load time with a fatal diagnostic

- Out of scope:
  - TASK-149/TASK-150 (custom payload widening — separate packet `04_custom-payload-widening`)
  - Changes to WIT type shapes (no changes to `extrusion-role`, `paint-semantic`, or `wall-feature-flag` in this packet)
  - Changes to converter/marshaling logic (only WIT source consolidation here)
  - Changes to IR schema versions or IR compatibility checking

## Prerequisites and Blockers

- Depends on:
  - None (this is the first WIT consolidation packet)
- Unblocks:
  - `04_custom-payload-widening` (TASK-149/150) — that packet modifies the canonical WIT types, which must exist in a consolidated form first
- Activation blockers:
  - Confirm `include_str!` paths resolve correctly from `crates/slicer-macros/src/lib.rs` to `wit/` (proc-macro path resolution)
  - Confirm the canonical `ir-types@1.1.0` version number is accepted by all existing bindings (schema constants may need updating)

## Acceptance Criteria

- **Given** `wit/deps/types.wit` is the canonical source, **when** `slicer-macros` compiles and calls `build_layer_world_glue`, **then** the WIT string used in `bindgen!` matches the on-disk `wit/deps/types.wit`, `wit/deps/config.wit`, `wit/deps/ir-types.wit`, and `wit/world-layer.wit` byte-for-byte. | `cargo build --package slicer-macros 2>&1 | grep -i "mismatch\\|drift\\|witinconsistent" || echo "build OK"`

- **Given** `wit_host.rs` is updated to use `include_str!` for all four worlds, **when** `slicer-host` compiles, **then** the inline WIT strings match the on-disk canonical files byte-for-byte. | `cargo build --package slicer-host 2>&1 | grep -i "mismatch\\|drift\\|wit" || echo "build OK"`

- **Given** a module manifest declares `wit-world = "slicer:world-layer@1.0.0"` and that world identifier is in the host's allowlist, **when** the module is loaded at startup, **then** the module is accepted and no `wit_world mismatch` diagnostic is emitted. | `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`

- **Given** a module manifest declares `wit-world = "slicer:layer-world@1.0.0"` (wrong package name) or any non-allowlisted identifier, **when** the module is loaded at startup, **then** the host emits a fatal diagnostic `"Unknown wit_world 'slicer:layer-world@1.0.0' — expected one of: slicer:world-layer@1.0.0, slicer:world-prepass@1.0.0, slicer:world-postpass@1.0.0, slicer:world-finalization@1.0.0"` and aborts module load. | `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world_mismatch --nocapture`

- **Given** the drift-detection test (`wit_drift_detection_tdd.rs`) runs, **when** it compares disk WIT files against embedded strings in macro and host, **then** all four worlds (`world-layer`, `world-prepass`, `world-postpass`, `world-finalization`) and all three dependency interfaces (`types`, `config`, `ir-types`) report zero drift. | `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`

- **Given** `ir-types.wit` on disk carries `package slicer:ir-types@1.1.0`, **when** the host validates a manifest with `min-ir-schema = "1.1.0"` and the host's IR schema version is `1.1.0`, **then** the version check passes. | `cargo test --package slicer-host --test dag_construction_tdd -- ir_schema_version_compatibility --nocapture`

## Negative Test Cases

- **Given** a `wit/` file is modified (e.g., `world-layer.wit` package renamed to `slicer:wrong-world@1.0.0`) but macro/host still embed the old string, **when** `wit_drift_detection_tdd` runs, **then** the test fails and reports the specific file and line of drift. | Manual: `echo 'package slicer:wrong-world@1.0.0;' > wit/world-layer.wit && cargo test --package slicer-host --test wit_drift_detection_tdd && git checkout wit/world-layer.wit`

- **Given** a manifest with `wit-world = "slicer:world-layer@2.0.0"` (future major version) is loaded, **when** the host checks the allowlist, **then** it rejects with a fatal diagnostic noting the major version mismatch. | `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world_major_version_mismatch --nocapture`

- **Given** the host inline WIT in `wit_host.rs` still uses `slicer:layer-world@1.0.0` (pre-consolidation), **when** the host tries to load a module with `wit-world = "slicer:world-layer@1.0.0"`, **then** the allowlist check rejects it because `slicer:layer-world@1.0.0` is the only entry in the allowlist. | Inspection of `wit_host.rs` line 179 confirms canonical name used

## Verification

- `cargo build --package slicer-macros`
- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
- `cargo clippy --package slicer-macros --package slicer-host -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT File Organization, Host-Boundary Access Enforcement, `deps/types.wit`, `deps/ir-types.wit`, `world-layer.wit`, `world-prepass.wit`, `world-postpass.wit`, `world-finalization.wit`
- `docs/04_host_scheduler.md` — module load validation, DAG construction

## OrcaSlicer Reference Obligations

None. This is an internal WIT source consolidation and validation task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
