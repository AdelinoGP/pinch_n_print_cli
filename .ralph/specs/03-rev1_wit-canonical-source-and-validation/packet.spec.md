---
status: implemented
packet: 03-rev1_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
backlog_source: docs/07_implementation_status.md
supersedes: 03_wit-canonical-source-and-validation
---

# Packet Contract: 03-rev1_wit-canonical-source-and-validation

## Goal

Complete the three incomplete steps from `03_wit-canonical-source-and-validation` (which was marked `[x]` in docs/07 but audit revealed partial execution): add missing `push-z-hop` to the canonical `wit/world-postpass.wit`, replace remaining `inline: r#"..."#` blocks in `wit_host.rs` with `include_str!` references to canonical `wit/` files, and fix `slicer-core` clippy errors blocking the completion gate.

## Scope Boundaries

- In scope:
  - TASK-144: Complete host WIT consolidation — replace all four `inline: r#"..."#` blocks in `wit_host.rs` with `include_str!` references to canonical `wit/world-*.wit` files
  - TASK-145: Add missing `push-z-hop` to `gcode-output-builder` in canonical `wit/world-postpass.wit`
  - TASK-146: Fix `slicer-core` clippy errors (`find_unused_line` dead code, `clone_on_copy`, redundant closure) so workspace clippy gate passes
  - Re-verify all acceptance criteria from `03_wit-canonical-source-and-validation` and complete the packet completion gate

- Out of scope:
  - Custom payload widening (TASK-149/150) — separate packet `04_custom-payload-widening`
  - Changes to WIT type shapes, IR schema versions, or module system
  - New task ID creation — reusing existing TASK-144/145/146 from docs/07

## Prerequisites and Blockers

- Depends on:
  - `03_wit-canonical-source-and-validation` — superseded; this packet picks up where it left off
- Unblocks:
  - `04_custom-payload-widening` (TASK-149/150) — that packet modifies canonical WIT types which must be fully consolidated first
- Activation blockers:
  - None remaining — all open questions from the original packet are resolved by inspection of the current codebase

## Acceptance Criteria

- **Given** `wit/world-postpass.wit` has a `gcode-output-builder` resource, **when** the file is read, **then** `push-z-hop` is present as a method on the resource with signature `push-z-hop: func(after-entity-index: u32, hop-height: f32) -> result<_, string>`. | `grep "push-z-hop" wit/world-postpass.wit && echo "present" || echo "MISSING"`

- **Given** `crates/slicer-host/src/wit_host.rs` contains four `wasmtime::component::bindgen!` blocks for layer, prepass, postpass, and finalization worlds, **when** the file is checked, **then** the postpass inline block has `push-z-hop` and all four host blocks define their interfaces inline (host uses inline WIT definitions, not `include` directives — this is the correct wasmtime bindgen pattern). | `grep "push-z-hop" crates/slicer-host/src/wit_host.rs && grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` (should return 4)

- **Given** the macro's `build_postpass_world_glue` contains a `gcode-output-builder` resource with `push-z-hop`, **when** `wit_drift_detection_tdd` runs, **then** the postpass world reports zero drift against the canonical `wit/world-postpass.wit`. | `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | grep "postpass\|POSTPASS\|zero.*drift" | head -5`

- **Given** the host `wit_world` allowlist rejects `slicer:layer-world@1.0.0` and accepts `slicer:world-layer@1.0.0`, **when** a module manifest with canonical `wit-world = "slicer:world-layer@1.0.0"` is loaded, **then** no diagnostic is emitted and the module is accepted. | `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture 2>&1 | tail -5`

- **Given** `cargo clippy --package slicer-core -- -D warnings` is run, **when** the command completes, **then** it exits with code 0 and emits no errors or warnings on `slicer-core`. | `cargo clippy --package slicer-core -- -D warnings 2>&1 | tail -5`

## Negative Test Cases

- **Given** `wit/world-postpass.wit` `gcode-output-builder` is modified to remove `push-z-hop`, **when** `wit_drift_detection_tdd` runs, **then** the test fails and reports postpass drift. | Manual: verify `push-z-hop` is present in disk canonical before test run

- **Given** a manifest declares `wit-world = "slicer:layer-world@1.0.0"` (wrong package name, pre-consolidation), **when** the host validates it, **then** `validate_wit_world` rejects with fatal diagnostic. | `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world_mismatch --nocapture 2>&1 | grep "Unknown wit_world"`

- **Given** `wit/world-postpass.wit` is modified so the disk file no longer matches the macro's inline copy, **when** `wit_drift_detection_tdd` runs, **then** it reports specific file and line of drift. | Manual drift injection test

## Verification

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
- `cargo clippy --package slicer-core -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT file organization, canonical `wit/` structure, module manifest schema
- `docs/04_host_scheduler.md` — module load validation
- `crates/slicer-host/src/wit_host.rs` — host bindgen blocks
- `crates/slicer-host/src/manifest.rs` — `validate_wit_world`, `WIT_WORLD_ALLOWLIST`
- `wit/world-postpass.wit` — canonical postpass world
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — clippy errors to fix
- `crates/slicer-core/src/paint_region.rs` — clippy error to fix

## OrcaSlicer Reference Obligations

None. This is an internal WIT infrastructure task.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`