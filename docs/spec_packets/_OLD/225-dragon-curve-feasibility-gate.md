---
status: implemented
packet: 225-dragon-curve-feasibility-gate
task_ids:
  - TASK-336
---

# 225-dragon-curve-feasibility-gate

## Goal

Upgrade the workspace to wasmtime 47.0.3 and wit-bindgen 0.60.0, then run MoonBit, AssemblyScript, C++, and Go components through one deterministic `slicer:postpass-text-postprocess/text-postprocess-module` host oracle and select the first loadable-and-correct authoring language in the locked priority order MoonBit, AssemblyScript, C++, Go, Rust.

## Problem Statement

The existing gate considered only historical Go and MoonBit probes. That is insufficient after discovery of an experimental AssemblyScript backend in `D:\wit-bindgen`, and it leaves C++'s documented component support unmeasured. Candidate comparisons also used different worlds and host harnesses. This packet creates one fair, deterministic feasibility oracle: every candidate must satisfy the existing `text-postprocess-module` component contract, instantiate in the production slicer-only linker, and return one exact string. Toolchain absence blocks measurement and requires user action; it is not evidence against a language.

## Architecture Constraints

- wasmtime is exactly 47.0.3; workspace wit-bindgen is exactly 0.60.0; `call-hook` remains enabled.
- The host oracle links slicer interfaces only. It must not add WASI merely to make C++ or Go pass.
- Candidate success requires both instantiation and exact invocation output. Component validity alone is insufficient.
- Candidate priority is immutable: MoonBit, AssemblyScript, C++, Go, Rust fallback.
- Missing tooling is not a result. The worker stops the current step, asks the user to install it, includes exact install and version-verification instructions, and resumes only after confirmation.
- `D:\wit-bindgen` is read-only. Its latest committed clean HEAD on `feat/assemblyscript-no-async` is resolved after explicit user confirmation immediately before generation, then held constant across the AssemblyScript and C++ probes. No workspace path dependency is introduced.
- AssemblyScript uses UTF-16 canonical embedding; the backend's unsupported async/future/stream/error-context surfaces are irrelevant to the synchronous text world but remain recorded limitations.
- No WIT/schema/IR version changes and no new ADR.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Risks and Tradeoffs

- The imported `config-view` resource makes the text world more representative than a scalar-only toy, but foreign generators may expose resource-lifetime friction. That is valid feasibility evidence.
- Concurrent work may leave the local fork dirty or advance its HEAD. The clean-tree/user-confirmation gate prevents measuring partial work; the captured SHA makes the eventual result attributable.
- C++ may import WASI due to its target/toolchain. The slicer-only linker intentionally exposes that incompatibility rather than adapting around it.
- A tool installation pause can interrupt packet implementation. This is required to avoid silently biasing selection toward an already-installed language.
