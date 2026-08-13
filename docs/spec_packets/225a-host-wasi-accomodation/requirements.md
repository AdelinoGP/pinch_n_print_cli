# Requirements: 225a-host-wasi-accomodation

## Packet Metadata

- Grouped task IDs: `TASK-336`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 225 measured production fit against a slicer-only linker. Go and C++ therefore failed because their toolchains link WASI, which measures a host constraint rather than whether the language can produce a working component. ADR-0060 requires a default-deny preview2 accommodation followed by a new language-feasibility measurement; packet 225 remains open and PARTIAL as the production-fit record.

## In Scope

- Add `wasmtime-wasi` preview2 linking to every production `HostExecutionContext` component linker with a default-deny `WasiCtx` in both host store-state types.
- Add equivalent default-deny WASI wiring to the independent foreign-language text-postprocess oracle.
- Re-run MoonBit, C++, Go with released `wit-bindgen-cli 0.60.0`; record real component hashes and terminal results.
- Run AssemblyScript only after user confirmation and the clean fork gate; capture fork HEAD immediately before generation and use UTF-16 embedding.
- Write the four-result summary and one fixed-priority Dragon Curve verdict in `docs/14_submodule_programming_languages.md`.

## Out of Scope

- Altering packet 225 evidence, the SDK guest build, WIT definitions, guest artifacts, or the production module contract.
- Granting WASI preopens, environment, arguments, network, inherited stdio, or any other capability.
- OrcaSlicer comparison or geometry-coordinate work.

## Authoritative Docs

- `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - direct read; decision and consequences.
- `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - direct read; fixture contract.
- `docs/14_submodule_programming_languages.md` - direct relevant-section read; selection destination.

## Acceptance Summary

- Positive: `AC-1` through `AC-9` in `packet.spec.md`; the host accepts the full preview2 import surface but exposes zero ambient capabilities.
- Negative: `AC-N1` preserves wrong-output rejection; `AC-N2` preserves the dirty-fork stop protocol.
- Cross-packet impact: packet 225 supplies the baseline toolchain/fixture/oracle and remains the production-fit evidence; packet 228 creates the `TASK-336` backlog row.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings` | AC-3 broad compilation and lint gate | FACT pass/fail; first failure <=20 lines |
| `cargo test -p slicer-wasm-host --test contract host_services_tdd --all-targets` | host services still satisfy their contract after state/linker changes | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd --all-targets` | WIT boundary remains synchronized | FACT pass/fail |
| `PNP_FOREIGN_COMPONENT=<component> cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact` | candidate-specific deterministic host oracle | FACT pass/fail; failure text retained in its evidence record |
| `docs/feasibility-probes/foreign-language-text-postprocess/check-prerequisites.sh` | toolchain gate before every probe | FACT ready or `BLOCKED: TOOLCHAIN` |
| `docs/feasibility-probes/foreign-language-text-postprocess/check-fork-readiness.sh` | AssemblyScript fork gate before generation | FACT ready or `BLOCKED: FORK_NOT_READY` |

## Step Completion Expectations

- Run `check-prerequisites.sh` before each probe. A missing tool is `BLOCKED: TOOLCHAIN`: stop and ask the user; it is never a candidate failure.
- Before AssemblyScript generation, require user confirmation, then run the fork gate and capture clean `feat/assemblyscript-backend` HEAD immediately before generation. A fork failure is `BLOCKED: FORK_NOT_READY` and stops the probe.
- The fixed verdict order is MoonBit, AssemblyScript, C++, Go; choose Rust only if all four records are `NOT_LOADABLE_OR_CORRECT`.

## Context Discipline Notes

`dispatch.rs` exceeds 300 lines: read only listed linker ranges and delegate global linker/struct-literal inventories. Probe tool output can be large; evidence records retain full diagnostics while dispatches return FACT plus bounded failure snippets.
