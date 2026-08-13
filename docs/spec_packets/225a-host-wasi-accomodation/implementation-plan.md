# Implementation Plan: 225a-host-wasi-accomodation

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-336`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Before each probe run the prerequisite gate; only AssemblyScript also needs user confirmation and fork readiness.

## Steps

### Step 1: Add default-deny production WASI

- Task IDs: `TASK-336`
- Objective: add `wasmtime-wasi`, default-deny `WasiCtx`/`WasiView` state, and shared preview2 registration at all production linkers.
- Precondition: packet 225's wasmtime `47.0.3` and wit-bindgen `0.60.0` changes are present; a LOCATIONS inventory of affected struct literals exists.
- Postcondition: all production `HostExecutionContext` linkers register WASI; runtime and direct state implement `WasiView` without capability grants.
- Files allowed to read, with ranges when over 300 lines:
  - `Cargo.toml` - lines `52-80`
  - `crates/slicer-wasm-host/Cargo.toml` - lines `8-40`
  - `crates/slicer-wasm-host/src/host.rs` - lines `1134-1400`
  - `crates/slicer-wasm-host/src/instance.rs` - lines `96-220`
  - `crates/slicer-wasm-host/src/dispatch.rs` - listed linker ranges in `design.md`
- Files allowed to edit (at most 3 per batch):
  - Batch A: `Cargo.toml`, `crates/slicer-wasm-host/Cargo.toml`, `crates/slicer-wasm-host/src/host.rs`
  - Batch B: `crates/slicer-wasm-host/src/instance.rs`, `crates/slicer-wasm-host/src/dispatch.rs`
- Files explicitly out of bounds: guest source/build paths, `Cargo.lock`, generated bindings, all docs, `OrcaSlicerDocumented/`.
- Blast-radius discipline: adding `WasiCtx` fields requires the dispatched `LOCATIONS` inventory of every `HostExecutionContext`/`HostState` literal or constructor; edit each compiling site only in the owning source batch, never discover omissions through broad check.
- Expected sub-agent dispatches:
  - Question: enumerate struct literals/constructors and 15 production linker sites; scope: `crates/slicer-wasm-host/**/*.rs`; return: `LOCATIONS` <=80 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - lines `1-16`
- OrcaSlicer refs: none.
- Verification:
  - AC-1 through AC-5 commands - FACT pass/fail.
  - `cargo test -p slicer-wasm-host --test contract host_services_tdd --all-targets` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd --all-targets` - FACT pass/fail.
- Exit condition: FALSIFIED if any linker omits shared WASI registration, either state lacks `WasiView`, a prohibited capability method appears in production wiring, or a targeted test fails.

### Step 2: Accommodate the independent oracle

- Task IDs: `TASK-336`
- Objective: register default-deny preview2 WASI in the foreign-language oracle's own linker and ensure its store state supplies `WasiCtx` through `WasiView`.
- Precondition: Step 1 host WASI API compiles.
- Postcondition: the oracle can instantiate WASI-importing foreign components and continues to reject wrong output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs` - lines `1-58`
  - `crates/slicer-wasm-host/src/host.rs` - lines `1134-1400`
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs`
- Files explicitly out of bounds: production dispatch/state sources, guest artifact/source, all records, `Cargo.lock`.
- Blast-radius discipline: no production struct field is added in this step; keep oracle-only state local to the test.
- Expected sub-agent dispatches:
  - Question: run the SDK wrong-output oracle command; scope: test binary only; return: `FACT` plus <=20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - contract section.
- OrcaSlicer refs: none.
- Verification:
  - AC-6 and AC-N1 commands - FACT pass/fail.
- Exit condition: FALSIFIED if the test uses production dispatch rather than its own linker, cannot instantiate WASI imports, or accepts the wrong-output SDK guest.

### Step 3: Re-measure MoonBit

- Task IDs: `TASK-336`
- Objective: generate/build/componentize MoonBit with released `wit-bindgen-cli 0.60.0`, run the oracle, and record its terminal result with full trap diagnostics if it fails.
- Precondition: Steps 1-2 are green and `check-prerequisites.sh` reports ready.
- Postcondition: `moonbit-text-postprocess.md` has truthful required fields, real hash, and terminal result.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - contract section.
  - `docs/feasibility-probes/foreign-language-text-postprocess/moonbit/` - generation/build scripts only.
- Files allowed to edit (at most 3):
  - `docs/feasibility-probes/moonbit-text-postprocess.md`
- Files explicitly out of bounds: production Rust sources, other candidate directories/records, `D:\wit-bindgen`, guest SDK sources.
- Blast-radius discipline: no Rust struct/schema change.
- Expected sub-agent dispatches:
  - Question: execute prerequisite gate and MoonBit oracle; scope: MoonBit fixture; return: `FACT`, commands, SHA-256, and <=20 diagnostic lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/14_submodule_programming_languages.md` - MoonBit ABI caveat lines `63-71`.
- OrcaSlicer refs: none.
- Verification:
  - AC-7 MoonBit portion and oracle command - FACT pass/fail.
- Exit condition: FALSIFIED if a missing tool is recorded as a language result, hash is sentinel/non-hex, released generator provenance is absent, or a trap lacks full diagnostic capture.

### Step 4: Re-measure C++

- Task IDs: `TASK-336`
- Objective: use released CLI, `C:/wasi-sdk/bin/clang++.exe`, and the preview1 reactor adapter under `$HOME/wasi-adapters/` to record C++'s terminal oracle result.
- Precondition: Steps 1-2 are green and prerequisite gate reports ready.
- Postcondition: `cpp-text-postprocess.md` records released `wit-bindgen-cli 0.60.0`, build/componentization evidence, real hash, and terminal result.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - contract section.
  - `docs/feasibility-probes/foreign-language-text-postprocess/cpp/` - generation/build scripts only.
- Files allowed to edit (at most 3):
  - `docs/feasibility-probes/cpp-text-postprocess.md`
- Files explicitly out of bounds: Rust host sources, other candidate directories/records, fork checkout, guest SDK sources.
- Blast-radius discipline: no Rust struct/schema change.
- Expected sub-agent dispatches:
  - Question: execute prerequisite gate and C++ oracle; scope: C++ fixture/toolchain; return: `FACT`, commands, SHA-256, and <=20 diagnostic lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - consequences lines `12-16`.
- OrcaSlicer refs: none.
- Verification:
  - AC-7 C++ portion and oracle command - FACT pass/fail.
- Exit condition: FALSIFIED if the adapter/toolchain is missing and result is still terminal, released generator source is not recorded, or evidence fields are incomplete.

### Step 5: Re-measure Go

- Task IDs: `TASK-336`
- Objective: use released CLI, `wit-bindgen-go v0.7.0` at `$(go env GOPATH)/bin`, and the adapter to record Go's terminal oracle result.
- Precondition: Steps 1-2 are green and prerequisite gate reports ready.
- Postcondition: `go-text-postprocess.md` records released provenance, commands, real hash, and terminal result.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - contract section.
  - `docs/feasibility-probes/foreign-language-text-postprocess/go/` - generation/build scripts only.
- Files allowed to edit (at most 3):
  - `docs/feasibility-probes/go-text-postprocess.md`
- Files explicitly out of bounds: Rust host sources, other candidate directories/records, fork checkout, guest SDK sources.
- Blast-radius discipline: no Rust struct/schema change.
- Expected sub-agent dispatches:
  - Question: execute prerequisite gate and Go oracle; scope: Go fixture/toolchain; return: `FACT`, commands, SHA-256, and <=20 diagnostic lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - lines `1-16`.
- OrcaSlicer refs: none.
- Verification:
  - AC-7 Go portion and oracle command - FACT pass/fail.
- Exit condition: FALSIFIED if tool absence becomes a language failure, generator provenance is absent, or the record lacks a real hash/terminal result.

### Step 6: Re-measure AssemblyScript

- Task IDs: `TASK-336`
- Objective: after user confirmation, gate the clean fork, capture its HEAD immediately before generation, generate with its CLI and UTF-16 embedding, then record the terminal oracle result.
- Precondition: Steps 1-2 are green; prerequisite gate is ready; user confirms; `check-fork-readiness.sh` verifies clean `feat/assemblyscript-backend`.
- Postcondition: AssemblyScript evidence contains clean 40-hex fork provenance, UTF-16/world evidence, real hash, and terminal result.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - contract section.
  - `docs/feasibility-probes/foreign-language-text-postprocess/assemblyscript/` - generation/build scripts only.
- Files allowed to edit (at most 3):
  - `docs/feasibility-probes/assemblyscript-text-postprocess.md`
- Files explicitly out of bounds: Rust host sources, other candidate directories/records, any fork source beyond gate/provenance commands, guest SDK sources.
- Blast-radius discipline: no Rust struct/schema change.
- Expected sub-agent dispatches:
  - Question: run fork gate immediately before generation and capture branch/status/HEAD; scope: `D:\wit-bindgen`; return: `FACT` with exactly branch, status, HEAD.
  - Question: execute AssemblyScript oracle; scope: AssemblyScript fixture; return: `FACT`, commands, SHA-256, and <=20 diagnostic lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/feasibility-probes/foreign-language-text-postprocess/check-fork-readiness.sh` - gate behavior.
- OrcaSlicer refs: none.
- Verification:
  - AC-7 AssemblyScript portion, AC-8, and oracle command - FACT pass/fail.
- Exit condition: FALSIFIED if user confirmation/fork readiness is bypassed, HEAD is not captured immediately before generation, fork status is not clean, UTF-16/world evidence is absent, or a gate failure produces a terminal candidate result.

### Step 7: Publish fixed-priority verdict

- Task IDs: `TASK-336`
- Objective: derive the locked-priority winner from the four terminal records and add their summary plus exactly one verdict line to docs/14.
- Precondition: all four records are terminal, valid, and no tooling blocker remains.
- Postcondition: docs/14 has one `**Dragon Curve authoring language: ...**` line naming first qualifying candidate or Rust.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/feasibility-probes/moonbit-text-postprocess.md` - full record.
  - `docs/feasibility-probes/assemblyscript-text-postprocess.md` - full record.
  - `docs/feasibility-probes/cpp-text-postprocess.md` - full record.
  - `docs/feasibility-probes/go-text-postprocess.md` - full record.
  - `docs/14_submodule_programming_languages.md` - relevant community-module section only.
- Files allowed to edit (at most 3):
  - `docs/14_submodule_programming_languages.md`
- Files explicitly out of bounds: all code, all fixtures, `docs/07_implementation_status.md`, Orca references.
- Blast-radius discipline: no Rust struct/schema change.
- Expected sub-agent dispatches:
  - Question: evaluate AC-9 against four records and docs/14; scope: those five documents; return: `FACT` winner and matching line.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - lines `12-16`.
- OrcaSlicer refs: none.
- Verification:
  - AC-9 command - FACT pass/fail.
- Exit condition: FALSIFIED if records are non-terminal/blocked, more or fewer than one verdict line exists, or the line differs from the fixed-priority result.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | shared state and all production linkers |
| Step 2 | S | isolated oracle wiring |
| Step 3 | M | MoonBit diagnostics/evidence |
| Step 4 | M | C++ toolchain/evidence |
| Step 5 | M | Go toolchain/evidence |
| Step 6 | M | confirmed fork provenance/evidence |
| Step 7 | S | deterministic documentation selection |

## Packet Completion Gate

- All steps and exits complete; all AC commands pass; four records are terminal; no tooling blocker remains.
- Workspace check, clippy, and targeted tests are green.
- `docs/07_implementation_status.md` has no delta: packet 228 creates its `TASK-336` row, and this packet extends that task's scope.
- Packet remains draft until implementation and closure review.

## Acceptance Ceremony

- Re-dispatch AC-1 through AC-9 and AC-N1 through AC-N2, plus the three packet-level gates, with FACT pass/fail returns.
- Record any packet-local toolchain availability risk in closure review.
- Confirm context stayed within the shared standard band or record the required hand-off/escalation.
