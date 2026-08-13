# Design: 225-dragon-curve-feasibility-gate

## Selected Approach

Use the smallest existing string-bearing world as a common language oracle: `slicer:postpass-text-postprocess/text-postprocess-module`. A new ignored integration test accepts a component path through `PNP_FOREIGN_COMPONENT`, instantiates it through the existing production `TextPostprocessModule` slicer-only linker, invokes `run-text-postprocess`, and asserts one exact output. Candidate-specific scripts generate/build/componentize into scratch directories; checked-in records retain the reproducible commands and measured evidence, not generated output.

## Architecture Constraints

- wasmtime is exactly 47.0.3; workspace wit-bindgen is exactly 0.60.0; `call-hook` remains enabled.
- The host oracle links slicer interfaces only. It must not add WASI merely to make C++ or Go pass.
- Candidate success requires both instantiation and exact invocation output. Component validity alone is insufficient.
- Candidate priority is immutable: MoonBit, AssemblyScript, C++, Go, Rust fallback.
- Missing tooling is not a result. The worker stops the current step, asks the user to install it, includes exact install and version-verification instructions, and resumes only after confirmation.
- `D:\wit-bindgen` is read-only. Its latest committed clean HEAD on `feat/assemblyscript-backend` is resolved after explicit user confirmation immediately before generation, then held constant across the AssemblyScript and C++ probes. No workspace path dependency is introduced.
- AssemblyScript uses UTF-16 canonical embedding; the backend's unsupported async/future/stream/error-context surfaces are irrelevant to the synchronous text world but remain recorded limitations.
- No WIT/schema/IR version changes and no new ADR.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Exact Contract

- Existing WIT package: `slicer:postpass-text-postprocess@1.0.0`.
- Existing world: `text-postprocess-module`.
- Existing exported interface/function: `text-postprocess.run(gcode-text: string, config: config-view) -> result<string, module-error>`.
- Input: `; probe input\n`.
- Expected success output: `;; foreign-language-probe\n; probe input\n`.
- Host path: `host::postpass_text::TextPostprocessModule`, existing `add_to_linker` registrations, then generated `call_run`.
- Result vocabulary: `LOADABLE_AND_CORRECT`, `NOT_LOADABLE_OR_CORRECT`, or transient `BLOCKED: TOOLCHAIN`. Only the first two may appear in finalized candidate records.

## Code Change Surface

- `Cargo.toml` and stale inline `crates/**/Cargo.toml`, `modules/**/Cargo.toml`: version bump.
- Compile-error-selected ranges in `crates/slicer-wasm-host/src/{host,dispatch,instance}.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-sdk/src/host.rs`, and `crates/slicer-macros/src/lib.rs`: mechanical API fallout only.
- `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs`: new ignored environment-driven host oracle.
- `crates/slicer-wasm-host/tests/integration/main.rs`: register `mod foreign_language_feasibility_tdd;`.
- `docs/feasibility-probes/foreign-language-text-postprocess/README.md`: shared world, exact input/output, install/request protocol, and per-language build/run commands.
- `docs/feasibility-probes/foreign-language-text-postprocess/check-prerequisites.sh`: real command checks plus `--simulate-missing <command>`; exit 42 and print candidate-specific `INSTALL:`/`VERIFY:` instructions.
- `docs/feasibility-probes/foreign-language-text-postprocess/check-fork-readiness.sh`: user-confirmation, branch, and clean-tree checks plus `--simulate-dirty`; exit 43 before writing `.generation-started`.
- `docs/feasibility-probes/foreign-language-text-postprocess/wit/**`: checked-in snapshot of the existing package and imported dependencies needed by the world; no WIT identifiers are changed.
- `docs/feasibility-probes/foreign-language-text-postprocess/{moonbit,assemblyscript,cpp,go}/**`: minimal handwritten source/build scripts. Generated bindings and binaries are excluded.
- `docs/feasibility-probes/{moonbit,assemblyscript,cpp,go}-text-postprocess.md`: measured evidence.
- `docs/14_submodule_programming_languages.md`: four-result summary and final language line.
- `docs/00_project_overview.md`, `docs/05_module_sdk.md`, and current-toolchain statements in `docs/14_submodule_programming_languages.md`: reconcile wasmtime/wit-bindgen versions; do not rewrite historical probe evidence.

## Read-Only Context

- `crates/slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit:1-19` and only imported type definitions required to copy a self-contained WIT tree.
- `crates/slicer-wasm-host/src/host.rs:1048-1070` and `dispatch.rs:1691-1802` for production linker/call shape.
- `crates/slicer-wasm-host/tests/integration/wasm_instance_tdd.rs:181` and `tests/common/wasm_cache.rs:20-65` for test conventions.
- Historical `docs/feasibility-probes/{moonbit,go}-wasm.md` for previous commands and failure hypotheses only.
- `D:\wit-bindgen\README.md:267-439`, generator CLI wiring, and AssemblyScript/C++ test harness ranges cited by requirements.

## Out-of-Bounds Files

- `target/`, `Cargo.lock`, generated bindings, component binaries, vendored dependencies.
- All edits and git mutations under `D:\wit-bindgen`; do not stash, clean, switch, pull, reset, or commit its working tree.
- Production WIT files under `crates/slicer-schema/wit/**`; fixture copies are edited only if needed to remain self-contained.
- Production linker behavior beyond mechanical wasmtime API fallout.
- `OrcaSlicerDocumented/`; no geometry parity obligation.
- Dragon module files, authored-coloring files, ADRs, and deviation log.

## Tooling Stop Protocol

The shared README owns concrete installation instructions and version checks for `moon`, `cargo`, local `wit-bindgen`, `node`/`npm`/`npx`, `asc`, WASI SDK `clang++`, `go`, `python3`, and `wasm-tools`; `check-prerequisites.sh` emits those instructions. Each probe step first runs its listed checks. On any miss:

1. Write no candidate `RESULT:` and do not modify the final selection.
2. Report `BLOCKED: TOOLCHAIN <candidate> <missing-command>` to the user.
3. Quote the README's platform-appropriate installation steps and version-verification command.
4. Wait for the user to install/confirm; do not install it agentically and do not continue to a lower-priority candidate.

## Probe Evidence Contract

Each candidate record contains these exact headings/fields: `WIT_WORLD:`, `TOOL_VERSIONS:`, `GENERATION_COMMANDS:`, `BUILD_COMMANDS:`, `COMPONENT_SHA256:`, `HOST_COMMAND:`, `HOST_OUTPUT:`, `FAILURE_DETAIL:` (use `none` on success), and terminal `RESULT:`. AssemblyScript and C++ additionally include the same `WIT_BINDGEN_HEAD:`, `WIT_BINDGEN_BRANCH:`, and `WIT_BINDGEN_STATUS: clean` captured immediately before AssemblyScript generation.

## Risks and Tradeoffs

- The imported `config-view` resource makes the text world more representative than a scalar-only toy, but foreign generators may expose resource-lifetime friction. That is valid feasibility evidence.
- Concurrent work may leave the local fork dirty or advance its HEAD. The clean-tree/user-confirmation gate prevents measuring partial work; the captured SHA makes the eventual result attributable.
- C++ may import WASI due to its target/toolchain. The slicer-only linker intentionally exposes that incompatibility rather than adapting around it.
- A tool installation pause can interrupt packet implementation. This is required to avoid silently biasing selection toward an already-installed language.

## Context Cost Estimate

- Aggregate: `M`.
- Largest steps: candidate fixture authoring and candidate execution, each `M` but serialized and bounded.
- If any candidate requires redesigning production WIT or linker behavior, stop and split; that is outside this packet.

## Open Questions

- [FWD] Exact current installation URLs/commands for missing toolchains must be verified against each tool's official documentation when the README is authored; do not guess stale commands.
- [FWD] The local AssemblyScript backend may require source-layout adjustments after generation. These remain inside its fixture directory if the WIT shape is supported.

There are no unresolved activation blockers in the design. Tool availability is an implementation-time user-action gate.
