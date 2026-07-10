---
status: implemented
packet: 26_live-support-module-evidence
task_ids:
  - TASK-120b
  - TASK-120
---

# 26_live-support-module-evidence

## Goal

Replace the synthetic `HostExecutionContext` commit-helper tests with real live-dispatch tests that load checked-in `tree-support.wasm` and `traditional-support.wasm` modules, run `Layer::Support` through the production `WasmRuntimeDispatcher`/`LayerStageRunner::run_stage` path, and prove deterministic non-empty `SupportIR` output. Then add a true support-enabled Benchy acceptance harness that uses a filtered module directory so `tree-support` is the active support holder, and asserts support-specific output markers in the emitted `.gcode`.

## Problem Statement

The current TASK-120b evidence in `live_support_generation_tdd.rs` consists of synthetic `HostExecutionContext` commit-helper tests that prove the commit path works but do not prove that real `tree-support.wasm` or `traditional-support.wasm` modules actually run on the production host dispatch path, produce non-empty `SupportIR` output, and remain deterministic across repeated runs. Additionally, the Benchy acceptance harness in `benchy_end_to_end_tdd.rs` does not run with support enabled and does not assert support-specific output markers.

## Architecture Constraints

- Real support module loading requires `WasmInstancePool::get` + `WasmRuntimeDispatcher::dispatch_layer_call` on the production path (not a synthetic fixture).
- The filtered module-dir builder must produce a directory that passes the claim resolution system (tree-support wins over traditional-support).
- JSON config fixture must use keys that match the live `tree-support.toml` manifest.
- Benchy acceptance assertions must prove actual support on the live path, not just printable output.

## Data and Contract Notes

- `SupportIR.support_paths` is the canonical output surface for `Layer::Support`.
- `ExtrusionRole::SupportMaterial` is the correct role for support paths.
- Claim resolution: `tree-support.wasm` holds `support-generator`; `traditional-support.wasm` also holds `support-generator`; with `traditional-support` excluded, `tree-support` wins.
- `;TYPE:Support` and `;TYPE:Support interface` are the OrcaSlicer-compatible comment markers.

## Risks and Tradeoffs

- **Risk**: Stale checked-in `.wasm` binaries cause the live dispatch tests to fail or behave non-deterministically.
  - Mitigation: Packet 27 runs `build-core-modules.sh` before the focused test matrix; rebuild as part of Packet 26 if tests reveal stale binaries.
- **Risk**: The JSON config keys don't match the live `tree-support.toml` schema.
  - Mitigation: Read `tree-support.toml` to confirm accepted keys before writing the fixture.
