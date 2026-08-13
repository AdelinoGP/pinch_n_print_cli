---
status: draft
packet: 225a-host-wasi-accomodation
task_ids:
  - TASK-336
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 225a-host-wasi-accomodation

## Goal

Extend `slicer-wasm-host` with default-deny WASI preview2 support, re-measure MoonBit, AssemblyScript, C++, and Go through the deterministic text-postprocess oracle, and publish the single Dragon Curve authoring-language verdict.

## Scope Boundaries

This is the accommodating-host continuation of packet 225's language-feasibility gate, not a revision of its production-fit evidence. It wires the full preview2 import surface without granting host capabilities, updates the independent oracle, records four honest probe outcomes, and writes the selection result. It does not change guest SDK builds, WIT, geometry, or Orca parity.

## Prerequisites and Blockers

- Forward-dep: this packet depends on packet 225's work (wasmtime 47.0.3 / wit-bindgen 0.60.0 bump, the shared probe fixture, and the ignored integration oracle). Packet 225 remains draft/open, with its records treated as production-fit evidence here. 225a executes only after 225's prerequisite work is present in the tree — the bump and fixture are already in the working tree; the AssemblyScript probe record is not. ADR-0060.
- Unblocks: final Dragon Curve authoring-language selection.
- Activation blockers: each candidate's prerequisite gate; AssemblyScript additionally requires user confirmation of clean `D:\wit-bindgen` at `feat/assemblyscript-backend` immediately before generation.

## Acceptance Criteria

- **AC-1. Given** the workspace dependency manifest, **when** its TOML is parsed, **then** `workspace.dependencies.wasmtime-wasi` exists and its version starts with `47.0.` (accepting published `47.0.2` or `47.0.3`). | `python -c "import tomllib; d=tomllib.load(open('Cargo.toml','rb')); v=d['workspace']['dependencies']['wasmtime-wasi']['version']; assert v.startswith('47.0.'), v; print('PASS',v)"`
- **AC-2. Given** `crates/slicer-wasm-host/Cargo.toml`, **when** its dependencies are parsed, **then** `wasmtime-wasi` is a workspace dependency. | `python -c "import tomllib; d=tomllib.load(open('crates/slicer-wasm-host/Cargo.toml','rb')); assert d['dependencies']['wasmtime-wasi']=={'workspace':True}; print('PASS')"`
- **AC-3. Given** the completed workspace, **when** all targets are checked and linted, **then** both commands exit zero with warnings denied. | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings`
- **AC-4. Given** every production component linker, including text postprocess at `dispatch.rs` near line 1721, **when** production linker construction is inspected, **then** every production linker construction site calls the shared helper `add_wasi_to_linker(&mut linker)` (count equality with `Linker::<HostExecutionContext>::new`, both greater than zero), and the helper registers WASI preview2 via `add_to_linker_sync`. | `python3 -c "import re; s=open('crates/slicer-wasm-host/src/dispatch.rs').read(); sites=len(re.findall(r'Linker::<HostExecutionContext>::new', s)); calls=len(re.findall(r'add_wasi_to_linker\(&mut linker\)', s)); assert sites==calls and sites>0, (sites, calls); assert 'add_to_linker_sync' in s; print('PASS')"`
- **AC-5. Given** production WASI state construction in `crates/slicer-wasm-host/src`, **when** its default-deny context builder and `WasiView` wiring are inspected, **then** the builder grants no stdio, environment, arguments, preopened directories, or network capability. | `! rg -n 'inherit_stdio|inherit_env|inherit_args|preopened_dir|inherit_network' crates/slicer-wasm-host/src && (rg -q 'WasiCtx::builder' crates/slicer-wasm-host/src/host.rs || rg -q 'WasiCtxBuilder' crates/slicer-wasm-host/src/host.rs) && (rg -q 'impl WasiView for HostExecutionContext' crates/slicer-wasm-host/src/host.rs || rg -q 'impl wasmtime_wasi::WasiView for HostExecutionContext' crates/slicer-wasm-host/src/host.rs || rg -q 'impl p2::WasiView for HostExecutionContext' crates/slicer-wasm-host/src/host.rs) && (rg -q 'impl WasiView for HostState' crates/slicer-wasm-host/src/instance.rs || rg -q 'impl wasmtime_wasi::WasiView for HostState' crates/slicer-wasm-host/src/instance.rs || rg -q 'impl p2::WasiView for HostState' crates/slicer-wasm-host/src/instance.rs) && echo PASS`
- **AC-6. Given** the independent foreign-language oracle linker and store, **when** the ignored oracle test is inspected, **then** it registers preview2 WASI and stores the `HostExecutionContext` that implements `WasiView`, while the wrong-output SDK guest remains rejected. | `rg -q 'add_to_linker_sync' crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs && rg -q 'Store::new.*HostExecutionContext\|HostExecutionContextBuilder' crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs && sh -c 'PNP_FOREIGN_COMPONENT=crates/slicer-wasm-host/test-guests/sdk-postpass-text-guest.component.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact >/tmp/pnp-225a-n1.out 2>&1; rc=$?; test "$rc" -ne 0 && rg -q "foreign component returned wrong output" /tmp/pnp-225a-n1.out && echo PASS'`
- **AC-7. Given** the four candidate re-measurements, **when** their evidence records are parsed, **then** `moonbit-text-postprocess.md`, `assemblyscript-text-postprocess.md`, `cpp-text-postprocess.md`, and `go-text-postprocess.md` each contain all required non-empty fields, a real 64-hex `COMPONENT_SHA256`, a terminal `RESULT`, and no `BLOCKED:` marker. | `python -c "from pathlib import Path; import re; names=('moonbit','assemblyscript','cpp','go'); req=('WIT_WORLD:','TOOL_VERSIONS:','GENERATION_COMMANDS:','BUILD_COMMANDS:','COMPONENT_SHA256:','HOST_COMMAND:','HOST_OUTPUT:','FAILURE_DETAIL:','RESULT:'); [(_ for _ in ()).throw(AssertionError(n)) if (not (s:=Path(f'docs/feasibility-probes/{n}-text-postprocess.md').read_text()) or 'BLOCKED:' in s or any(not re.search(rf'^{re.escape(k)}\\s*\\S',s,re.M) for k in req) or not re.search(r'^COMPONENT_SHA256: [0-9a-f]{64}$',s,re.M) or not re.search(r'^RESULT: (?:LOADABLE_AND_CORRECT|NOT_LOADABLE_OR_CORRECT)$',s,re.M)) else None for n in names]; print('PASS')"`
- **AC-8. Given** the AssemblyScript re-measurement record, **when** its provenance and ABI fields are parsed, **then** it records a 40-hex `WIT_BINDGEN_HEAD`, branch `feat/assemblyscript-backend`, status `clean`, UTF-16 embedding, and world `slicer:postpass-text-postprocess/text-postprocess-module`. | `python -c "from pathlib import Path; import re; s=Path('docs/feasibility-probes/assemblyscript-text-postprocess.md').read_text(); checks=(r'^WIT_BINDGEN_HEAD: [0-9a-f]{40}$',r'^WIT_BINDGEN_BRANCH: feat/assemblyscript-backend$',r'^WIT_BINDGEN_STATUS: clean$',r'UTF-16',r'slicer:postpass-text-postprocess/text-postprocess-module'); assert all(re.search(x,s,re.M) for x in checks); print('PASS')"`
- **AC-9. Given** the four terminal records in locked MoonBit, AssemblyScript, C++, Go priority, **when** `docs/14_submodule_programming_languages.md` is parsed, **then** it has exactly one `**Dragon Curve authoring language: ...**` line whose language is the first `LOADABLE_AND_CORRECT` candidate in that order, or `Rust` if none qualifies. | `python -c "from pathlib import Path; import re; names=(('MoonBit','moonbit'),('AssemblyScript','assemblyscript'),('C++','cpp'),('Go','go')); winner=next((label for label,file in names if re.search(r'^RESULT: LOADABLE_AND_CORRECT$',Path(f'docs/feasibility-probes/{file}-text-postprocess.md').read_text(),re.M)),'Rust'); lines=re.findall(r'^\\*\\*Dragon Curve authoring language: (.+?)\\*\\*$',Path('docs/14_submodule_programming_languages.md').read_text(),re.M); assert lines==[winner],(lines,winner); print('PASS',winner)"`

## Negative Test Cases

- **AC-N1. Given** the SDK postpass guest, which returns output other than the foreign-language oracle contract, **when** it is supplied as `PNP_FOREIGN_COMPONENT` after WASI registration, **then** the ignored oracle exits nonzero and reports `foreign component returned wrong output`. | `sh -c 'PNP_FOREIGN_COMPONENT=crates/slicer-wasm-host/test-guests/sdk-postpass-text-guest.component.wasm cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component -- --ignored --exact >/tmp/pnp-225a-n1.out 2>&1; rc=$?; test "$rc" -ne 0 && rg -q "foreign component returned wrong output" /tmp/pnp-225a-n1.out && echo PASS'`
- **AC-N2. Given** the AssemblyScript fork readiness gate is deliberately dirty, **when** `check-fork-readiness.sh --simulate-dirty` runs, **then** it exits `43`, writes `BLOCKED: FORK_NOT_READY dirty`, and writes no `GENERATION_COMMANDS:` field. | `sh -c 'out=$(docs/feasibility-probes/foreign-language-text-postprocess/check-fork-readiness.sh --simulate-dirty 2>&1); rc=$?; test "$rc" = 43 && printf "%s" "$out" | rg -q "BLOCKED: FORK_NOT_READY dirty" && ! printf "%s" "$out" | rg -q "GENERATION_COMMANDS:" && echo PASS'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-wasm-host --test contract host_services_tdd --all-targets && cargo test -p slicer-runtime --test contract wit_drift_detection_tdd --all-targets`

## Authoritative Docs

- `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - direct read; accepted host-accommodation decision.
- `docs/14_submodule_programming_languages.md` - direct relevant-section read; final verdict destination.
- `docs/feasibility-probes/foreign-language-text-postprocess/README.md` - direct read; oracle and evidence contract.

## Doc Impact Statement (Required)

- `docs/14_submodule_programming_languages.md` Dragon Curve verdict summary - `rg -q '^\*\*Dragon Curve authoring language: ' docs/14_submodule_programming_languages.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
