---
status: implemented
packet: 225a-host-wasi-accomodation
task_ids:
  - TASK-336
---

# 225a-host-wasi-accomodation

## Goal

Extend `slicer-wasm-host` with default-deny WASI preview2 support, re-measure MoonBit, AssemblyScript, C++, and Go through the deterministic text-postprocess oracle, and publish the single Dragon Curve authoring-language verdict.

## Problem Statement

Packet 225 measured production fit against a slicer-only linker. Go and C++ therefore failed because their toolchains link WASI, which measures a host constraint rather than whether the language can produce a working component. ADR-0060 requires a default-deny preview2 accommodation followed by a new language-feasibility measurement; packet 225 remains open and PARTIAL as the production-fit record.

## Architecture Constraints

- Register the full wasmtime-wasi preview2 surface using `wasmtime_wasi::p2::add_to_linker_sync::<T: WasiView>`; default-deny `WasiCtx::builder()` must not call capability-granting methods.
- `HostExecutionContext` and `HostState` must each own a `WasiCtx` and implement `WasiView`, so runtime and direct instantiation use equivalent store capabilities.
- Apply the shared linker helper `add_wasi_to_linker(&mut linker)` at all 15 production linker sites; text postprocess is the acceptance anchor, not the only affected path.
- WASM-staleness snippet intentionally omitted: this host-only linker/state change does not feed guest WASM builds, and the probes build foreign components in scratch.
- No Orca delegation and no coordinate-system constraints apply.

## Data and Contract Notes

- IR/manifest contracts: unchanged.
- WIT boundary: world remains `slicer:postpass-text-postprocess/text-postprocess-module`; WASI only satisfies foreign component imports alongside existing generated bindings.
- Determinism/scheduler constraints: no ambient host input reaches guests; no preopens, env, args, network, or inherited stdio; the oracle input/output remains exact.

## Locked Assumptions and Invariants

- Use released `wit-bindgen-cli 0.60.0` for MoonBit, C++, and Go. Use only the confirmed, clean AssemblyScript fork and capture its HEAD immediately before generation.
- Every record has a real component SHA-256 and terminal result; a tooling blocker stops the candidate rather than producing a candidate result. Deviation (user-authorized, 2026-08-13): Go's released-toolchain incapability (wit-bindgen-go v0.7.0 generated imports rejected by `go:wasmimport` on every viable Go toolchain — tool present but incapable, not absent) was recorded as terminal `NOT_LOADABLE_OR_CORRECT` per explicit user decision; the measured artifact is the only buildable (unwired) component.
- The selection is the first `LOADABLE_AND_CORRECT` record in MoonBit, AssemblyScript, C++, Go order, otherwise Rust, and docs/14 contains exactly one formatted verdict line.

## Risks and Tradeoffs

- Full preview2 linking broadens imports accepted but default-deny state preserves the host capability boundary.
- MoonBit may trap or corrupt strings due to UTF-16/UTF-8 behavior; retain full diagnostics rather than normalize failure.
- Toolchain/fork availability can block the packet; blockers are explicit and must not be misreported as a negative language verdict.
