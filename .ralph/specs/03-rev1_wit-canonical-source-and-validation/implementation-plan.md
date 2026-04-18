# Implementation Plan: 03-rev1_wit-canonical-source-and-validation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-144, TASK-145, TASK-146).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Add `push-z-hop` to canonical `wit/world-postpass.wit`

- Task IDs: `TASK-145`
- Objective: Add the missing `push-z-hop` method to the `gcode-output-builder` resource in the canonical disk source so the disk file matches what the macro and host inline already have.
- Precondition: None
- Postcondition: `wit/world-postpass.wit` contains `push-z-hop: func(after-entity-index: u32, hop-height: f32) -> result<_, string>;` in the `gcode-output-builder` resource. The method comment referencing Layer::PathOptimization context is present.
- Files expected to change:
  - `wit/world-postpass.wit`
- Authoritative docs: `docs/03_wit_and_manifest.md` (world-postpass.wit section)
- OrcaSlicer refs: None
- Verification: `grep "push-z-hop" wit/world-postpass.wit` returns the full method signature
- Exit condition: Disk canonical has `push-z-hop` matching the macro's inline copy at `lib.rs:571`

---

### Step 2: Verify host postpass inline WIT has `push-z-hop`

- Task IDs: `TASK-144`
- Objective: Confirm the postpass `bindgen!` block in `wit_host.rs` contains `push-z-hop` in its inline WIT. This was already present in the original block (host inline defines types inline, unlike the macro which uses `include` directives).
- Precondition: Step 1 complete; disk canonical has `push-z-hop`
- Postcondition: The postpass inline block in `wit_host.rs` has `push-z-hop` method. Host blocks define all interface types inline — they do not use `include` directives (this is the correct pattern for wasmtime component bindgen host-side bindings).
- Files expected to change: None (verification only — already correct)
- Authoritative docs: `docs/03_wit_and_manifest.md` (WIT file organization)
- OrcaSlicer refs: None
- Verification: `grep "push-z-hop" crates/slicer-host/src/wit_host.rs` returns the method; `grep 'include.*wit/deps' crates/slicer-host/src/wit_host.rs` returns empty (host uses inline definitions)
- Exit condition: Host postpass inline has `push-z-hop`; no `include` directives present (by design)

---

### Step 3: Verify remaining host world blocks are correctly inline

- Task IDs: `TASK-144`
- Objective: Confirm that all four host world blocks (layer, prepass, postpass, finalization) define their interfaces inline and do not use `include` directives. This is the correct pattern for host-side wasmtime component bindgen — the macro (at `slicer-macros/src/lib.rs`) uses `include` directives because it generates WIT at compile time, but the host binds against inline WIT directly.
- Precondition: Step 2 verified
- Postcondition: All four host inline blocks define geometry, config-types, host-services, and ir-handles interfaces inline. No `include` directives present in host blocks.
- Files expected to change: None (verification only — already correct)
- Authoritative docs: `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` returns 4; `grep -c 'include.*wit/deps' crates/slicer-host/src/wit_host.rs` returns 0
- Exit condition: Host correctly uses inline WIT definitions (not includes)

---

### Step 4: Fix `slicer-core` clippy errors

- Task IDs: `TASK-146`
- Objective: Fix the three clippy errors blocking `cargo clippy --package slicer-core -- -D warnings`:
  1. Remove `find_unused_line` function (never used, dead code at `triangle_mesh_slicer.rs:344`)
  2. Replace `|lines| chain_lines_to_expolygons(lines)` with `chain_lines_to_expolygons` (redundant closure at `triangle_mesh_slicer.rs:56`)
  3. Change `value.clone()` to `*value` (clone_on_copy at `paint_region.rs:54`)
- Precondition: None
- Postcondition: `cargo clippy --package slicer-core -- -D warnings` exits with code 0
- Files expected to change:
  - `crates/slicer-core/src/triangle_mesh_slicer.rs`
  - `crates/slicer-core/src/paint_region.rs`
- Authoritative docs: None
- OrcaSlicer refs: None
- Verification: `cargo clippy --package slicer-core -- -D warnings 2>&1 | tail -5`
- Exit condition: Clippy passes with zero errors on `slicer-core`

---

### Step 5: Re-verify all acceptance criteria

- Task IDs: `TASK-144`, `TASK-145`, `TASK-146`
- Objective: Run the full verification suite to confirm all original acceptance criteria still hold after the consolidation fixes.
- Precondition: Steps 1-4 complete
- Postcondition: All verification commands pass; `wit_drift_detection_tdd` reports zero drift across all four worlds
- Files expected to change: None (verification only)
- Authoritative docs: `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo build --package slicer-host`
  - `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
  - `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
  - `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
  - `cargo clippy --package slicer-core -- -D warnings`
- Exit condition: All commands pass; zero drift reported; no slicer-core clippy errors

---

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- `cargo build --package slicer-host` succeeds.
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` passes with 9/9.
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture` passes with 2/2.
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture` passes with 13/13.
- `cargo clippy --package slicer-core -- -D warnings` exits with code 0.
- `grep "push-z-hop" wit/world-postpass.wit` returns the method signature.
- `grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` → 4 (four host inline blocks, not zero)
- `docs/07_implementation_status.md` confirms TASK-144, TASK-145, TASK-146 are `[x]`.
- `packet.spec.md` status updated to `implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm full workspace build and clippy are green.
- Confirm drift detection test reports zero drift for all four worlds.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.