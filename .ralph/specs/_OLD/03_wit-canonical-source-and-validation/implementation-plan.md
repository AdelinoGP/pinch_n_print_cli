# Implementation Plan: 03_wit-canonical-source-and-validation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-144, TASK-145, TASK-146).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit inline WIT copies and establish canonical inventory

- Task IDs: `TASK-144`
- Objective: Produce an exact inventory of all inline WIT string locations in macro `lib.rs` and host `wit_host.rs`, their content, and which on-disk file each should reference. This is a read-only discovery step.
- Precondition: None
- Postcondition: A list (file:line → canonical path → content summary) for all 7 inline WIT copies: macro layer-world, macro prepass-world, macro postpass-world, macro finalization-world, host layer-world, host prepass-world, host postpass/finalization-world.
- Files expected to change: None (read-only)
- Authoritative docs: `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs`, `wit/` directory
- OrcaSlicer refs: None
- Verification: `grep -n "include_str\|inline: r#" crates/slicer-macros/src/lib.rs | head -20` and `grep -n "inline: r#" crates/slicer-host/src/wit_host.rs | head -20` (verify no `include_str!` yet exists)
- Exit condition: Complete inventory written to `design.md` notes section with exact file:line references for every inline WIT string

---

### Step 2: Verify `include_str!` path resolution from proc-macro

- Task IDs: `TASK-144`
- Objective: Confirm that `include_str!("../../wit/deps/types.wit")` resolves correctly from `crates/slicer-macros/src/lib.rs` at proc-macro compile time.
- Precondition: Step 1 complete; inventory shows all inline WIT locations
- Postcondition: Either (a) the relative path `../../wit/` works from `slicer-macros`, or (b) an alternative path strategy is chosen and documented
- Files expected to change: `crates/slicer-macros/src/lib.rs` (add one test `include_str!` to verify path)
- Authoritative docs: `crates/slicer-macros/Cargo.toml` (crate root), `Cargo.toml` workspace root
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-macros 2>&1 | grep -i "could not find\|file not found\|include_str" || echo "path resolution OK"`
- Exit condition: A single `include_str!` test in `lib.rs` compiles successfully, proving the path works. If it fails, document the alternative (e.g., copy key WIT files into `slicer-macros/src/` or use a workspace `build.rs`).

---

### Step 3: Consolidate macro inline WIT onto canonical `wit/` files

- Task IDs: `TASK-144`
- Objective: Replace all four `build_*_world_glue` functions' inline WIT strings in `crates/slicer-macros/src/lib.rs` with `include_str!` references to canonical on-disk files. Update `WIT_WORLD_MAP` if needed.
- Precondition: Step 2 confirmed working path resolution
- Postcondition: `lib.rs` contains no inline WIT string literals for types/config/ir-types/world files; all four `build_*_world_glue` functions use `include_str!` pointing to `wit/`; `cargo build --package slicer-macros` succeeds
- Files expected to change:
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs: `docs/03_wit_and_manifest.md` (WIT file organization)
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-macros && grep -c 'inline: r#"' crates/slicer-macros/src/lib.rs` → 0 (no inline WIT strings remain in macro)
- Exit condition: All inline WIT replaced with `include_str!`; macro crate builds with zero errors or warnings

---

### Step 4: Consolidate host inline WIT onto canonical `wit/` files

- Task IDs: `TASK-144`
- Objective: Replace all inline `wasmtime::component::bindgen!({ inline: r#"..."# })` blocks in `crates/slicer-host/src/wit_host.rs` with `include_str!` references. Fix package names `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` and `slicer:prepass-world@1.0.0` → `slicer:world-prepass@1.0.0`.
- Precondition: Step 3 complete; macro consolidated
- Postcondition: `wit_host.rs` uses `include_str!` for all four worlds; package names are canonical; `cargo build --package slicer-host` succeeds
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
  - Possibly `crates/slicer-host/src/dag.rs:158` and `crates/slicer-host/src/execution_plan.rs:858` (verify hardcoded values are canonical)
- Authoritative docs: `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host && grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` → 0
- Exit condition: All inline WIT replaced; host crate builds with zero errors or warnings

---

### Step 5: Restore missing WIT members in canonical disk files

- Task IDs: `TASK-145`
- Objective: Add missing `needs-support` to `wit/deps/ir-types.wit` and `push-z-hop` to `wit/world-postpass.wit` in the canonical disk source.
- Precondition: Steps 3 and 4 complete; canonical disk is the source
- Postcondition: `wit/deps/ir-types.wit` contains the `needs-support` interface; `wit/world-postpass.wit` contains `push-z-hop` in `gcode-output-builder`
- Files expected to change:
  - `wit/deps/ir-types.wit`
  - `wit/world-postpass.wit`
- Authoritative docs: `docs/03_wit_and_manifest.md` (ir-types.wit section, world-postpass.wit section)
- OrcaSlicer refs: None
- Verification: `grep "needs-support" wit/deps/ir-types.wit` returns the interface definition; `grep "push-z-hop" wit/world-postpass.wit` returns the method
- Exit condition: Both missing members are present in disk canonical

---

### Step 6: Add `wit_world` allowlist validation at module load

- Task IDs: `TASK-146`
- Objective: Add host-side `wit_world` allowlist check that rejects manifests with non-allowlisted world identifiers at startup. Use the four canonical identifiers: `slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`.
- Precondition: Steps 3-5 complete; host builds with consolidated WIT
- Postcondition: `validate_wit_world` function exists in `manifest.rs` (or new `module_load.rs`); it is called during module load; non-allowlisted `wit_world` values produce a fatal startup diagnostic with the expected/actual values
- Files expected to change:
  - `crates/slicer-host/src/manifest.rs` (add `validate_wit_world`)
  - `crates/slicer-host/src/dag.rs` or `crates/slicer-host/src/module_load.rs` (add call to `validate_wit_world` in load path)
- Authoritative docs: `docs/04_host_scheduler.md` (module load validation), `docs/03_wit_and_manifest.md` (Module Manifest Schema)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture` (new tests for valid and invalid cases)
- Exit condition: Allowlist validation exists, compiles, and correctly rejects non-allowlisted `wit_world` values

---

### Step 7: Add drift-detection regression test

- Task IDs: `TASK-145`
- Objective: Create `wit_drift_detection_tdd.rs` that proves the disk WIT files match the embedded strings used by macro and host. This prevents future drift.
- Precondition: Steps 3 and 4 complete; all WIT sources use `include_str!`
- Postcondition: `crates/slicer-host/tests/wit_drift_detection_tdd.rs` exists; it reads disk WIT files and compares them against the `include_str!` results extracted from macro and host source; the test passes when disk matches embedded
- Files expected to change:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` (new file)
- Authoritative docs: `crates/slicer-macros/src/lib.rs` (for `include_str!` paths), `crates/slicer-host/src/wit_host.rs`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` → all worlds and interfaces report zero drift
- Exit condition: Drift detection test exists and passes with zero drift

---

### Step 8: Verify workspace build and clippy

- Task IDs: `TASK-144`, `TASK-145`, `TASK-146`
- Objective: Run the full workspace build and clippy to confirm no regressions from the consolidation.
- Precondition: Steps 1-7 complete
- Postcondition: `cargo build --workspace` succeeds; `cargo clippy --workspace -- -D warnings` passes
- Files expected to change: None (verification only)
- Authoritative docs: `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo build --workspace && cargo clippy --workspace -- -D warnings`
- Exit condition: Full workspace build and clippy pass with zero warnings

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- `cargo build --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes with zero warnings.
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` passes.
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture` passes.
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture` passes.
- All acceptance criteria from `packet.spec.md` are verified by the pipe-suffixed commands.
- `docs/07_implementation_status.md` updated: TASK-144, TASK-145, TASK-146 marked complete.
- `packet.spec.md` status updated to `implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm full workspace build and clippy are green.
- Confirm drift detection test reports zero drift for all four worlds and three dependency interfaces.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
