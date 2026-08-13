# Implementation Plan: 225-dragon-curve-feasibility-gate

## Execution Rules

- Execute steps serially. Delegate all cargo/build/test commands and return bounded output.
- Before each candidate probe, enforce the tooling stop protocol in `design.md`; absence never becomes a candidate failure.
- Generated bindings/components stay in scratch space. Commit only source fixtures, scripts, and evidence records.

## Steps

### Step 1: Inventory pins, host seam, and external generator provenance

- Task IDs: `TASK-336`
- Objective: Bound the toolchain bump and freeze the common host/WIT contract plus local wit-bindgen provenance.
- Precondition: workspace and `D:\wit-bindgen` are readable.
- Postcondition: scratch inventory lists all stale pins, wasmtime fallout symbols, exact WIT world/function, host test seam, and notes that external HEAD is intentionally deferred until the concurrent fork work is committed.
- Files allowed to read: manifest lines via grep; `postpass-text-postprocess.wit:1-19`; `host.rs:1048-1070`; `dispatch.rs:1691-1802`; `wasm_instance_tdd.rs:181` vicinity; `D:\wit-bindgen` references named in `requirements.md`.
- Files allowed to edit: `$COMMANDCODE_SCRATCHPAD/225_inventory.md`.
- Files out of bounds: generated files, lockfiles, `target/`, OrcaSlicer.
- Dispatches: `LOCATIONS` for stale pins/host symbols; `FACT` for current fork branch/status only to determine whether the later user-confirmation gate is required. Do not freeze the current in-progress HEAD.
- Context cost: `S`.
- Authorities: `requirements.md` references.
- Verification: `test -s "$COMMANDCODE_SCRATCHPAD/225_inventory.md" && echo PASS || echo FAIL`.
- Exit condition: all five inventory categories have concrete entries and external provenance matches or explicitly supersedes the authoring snapshot.

### Step 2: Bump pins and absorb API fallout

- Task IDs: `TASK-336`
- Objective: Land exact wasmtime/wit-bindgen pins, remove stale inline pins, and restore green compile/lint/guest freshness.
- Precondition: Step 1 pin/symbol inventory complete.
- Postcondition: AC-1 through AC-3 pass.
- Files allowed to read: only compile-error ranges in `Cargo.toml`, stale manifests, `crates/slicer-wasm-host/src/{host,dispatch,instance}.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-sdk/src/host.rs`, and `crates/slicer-macros/src/lib.rs`.
- Files allowed to edit, root batch: `Cargo.toml` only.
- Files allowed to edit, stale-pin batches: at most three stale manifests per batch.
- Files allowed to edit, fallout batches: at most three compile-error files per batch.
- Files out of bounds: `Cargo.lock`, generated guest bindings, unrelated source.
- Dispatches: delegated `cargo check` returns error file/line plus at most 20 lines; signature lookups return at most three 30-line snippets.
- Context cost: `M`.
- Authorities: root workspace comments and exact pins in packet ACs.
- Verification: AC-1, AC-2, AC-3; targeted `cargo test -p slicer-wasm-host --test contract host_services_tdd`; targeted `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd`.
- Exit condition: all commands exit zero and `build-guests --check` reports no stale artifact.

### Step 2a: Reconcile current-toolchain version documentation

- Task IDs: `TASK-336`
- Objective: Update every authoritative current-toolchain statement affected by Step 2 without altering historical probe records.
- Precondition: Step 2 exact pins and compile gate are green.
- Postcondition: AC-9 passes.
- Files allowed to read: grep-selected version lines only in `docs/00_project_overview.md`, `docs/05_module_sdk.md`, and `docs/14_submodule_programming_languages.md`.
- Files allowed to edit: those three documentation files.
- Files out of bounds: historical `docs/feasibility-probes/*.md`, unrelated sections, implementation source.
- Dispatches: `LOCATIONS` for `43.0.0`, `47.0.3`, `0.57.1`, and `0.60.0` in the three files, classifying each occurrence as current or historical.
- Context cost: `S`.
- Authorities: landed root `Cargo.toml` pins.
- Verification: AC-9 and the matching Doc Impact command.
- Exit condition: current statements match landed pins and historical statements remain labeled by their original probe context.

### Step 3: Author the shared fixture contract and tooling instructions

- Task IDs: `TASK-336`
- Objective: Create a self-contained same-world fixture with exact installation/verification instructions and candidate scripts, without generated output.
- Precondition: Step 2 establishes the target workspace toolchain.
- Postcondition: README defines exact world/input/output, per-candidate prerequisite checks, official installation instructions, stop-and-ask behavior, and scratch build commands; WIT snapshot resolves.
- Files allowed to read: exact imported WIT files reached from `postpass-text-postprocess.wit`; official tool docs via delegated web research; bounded local wit-bindgen README ranges.
- Files allowed to edit, batch A: `foreign-language-text-postprocess/README.md`; `check-prerequisites.sh`; `check-fork-readiness.sh`.
- Files allowed to edit, later WIT batches: at most three copied WIT files per batch, unchanged from the production WIT dependency closure.
- Files allowed to edit, fixture-control batch: fixture ignore file and at most two shared build helpers.
- Files allowed to edit, later batches: at most three source/build files per language directory under `foreign-language-text-postprocess/{moonbit,assemblyscript,cpp,go}/`.
- Files out of bounds: production WIT, generated bindings/components, `D:\wit-bindgen` edits.
- Dispatches: `SNIPPETS` for exact WIT dependency closure; `FACT` from official docs for each install and version check; `LOCATIONS` for local generator commands.
- Context cost: `M`.
- Authorities: existing WIT plus official tool docs and local generator docs as resolved at execution time.
- Verification: AC-8, AC-N2, and AC-N3; run each script's syntax/help mode where supported, without treating unavailable tools as candidate failure.
- Exit condition: a fresh worker can identify prerequisites, install-request text, exact build commands, and oracle values without inference.

### Step 4: Add the production-linker integration oracle

- Task IDs: `TASK-336`
- Objective: Add one ignored test that loads an arbitrary foreign component and asserts the exact text-postprocess result through production bindings.
- Precondition: Step 3 contract fixes the environment variable, input, and output.
- Postcondition: test is registered in the existing `integration` binary and demonstrably rejects a wrong-output or unresolved-import component.
- Files allowed to read: `tests/integration/main.rs`; `wasm_instance_tdd.rs:160-210`; `tests/common/wasm_cache.rs:20-65`; host/dispatch ranges in design.
- Files allowed to edit: `tests/integration/foreign_language_feasibility_tdd.rs`; `tests/integration/main.rs`; optionally one test fixture helper under `tests/common/` only if existing public construction cannot load an arbitrary path.
- Files out of bounds: production linker changes, WIT changes, generated components.
- Dispatches: `FACT` verifying test binary aggregation and exact pre-existing construction/call APIs.
- Context cost: `S`.
- Authorities: existing integration-test conventions and production text-postprocess bindings.
- Verification: compile with `cargo test -p slicer-wasm-host --test integration foreign_language_feasibility_tdd::foreign_language_text_postprocess_component --no-run`; run AC-N1 against the existing SDK text guest after guest freshness is green; then AC-4 when a candidate component exists.
- Exit condition: the named ignored test compiles, is listed exactly once, and fails rather than skips when `PNP_FOREIGN_COMPONENT` points to an invalid component.

### Step 5: Run MoonBit probe

- Task IDs: `TASK-336`
- Objective: Build and measure MoonBit first using the shared fixture and host oracle.
- Precondition: Steps 3-4 complete.
- Postcondition: `moonbit-text-postprocess.md` contains the evidence contract and terminal result.
- Files allowed to read: shared README and MoonBit fixture only; historical MoonBit record via delegated summary.
- Files allowed to edit, fixture batches: at most three MoonBit fixture files per batch.
- Files allowed to edit, evidence batch: `docs/feasibility-probes/moonbit-text-postprocess.md` only, after the fixture build/test completes.
- Files out of bounds: host/WIT behavior, other candidate records.
- Dispatches: prerequisite `FACT`; build/test `SNIPPETS` limited to terminal output.
- Context cost: `M`.
- Authorities: shared README and existing MoonBit generator/tool docs.
- Verification: candidate record fields from AC-5 plus AC-4 with its component.
- Exit condition: terminal measured result exists; if tooling is missing, stop and ask the user instead, leaving no result.

### Step 6: Run AssemblyScript probe

- Task IDs: `TASK-336`
- Objective: Generate with the experimental local backend, compile/embed as UTF-16, and measure through the same oracle.
- Precondition: MoonBit has a terminal result; the user confirms concurrent async-support work is committed; the fork is clean on `feat/assemblyscript-backend`; its latest HEAD is captured immediately before generation.
- Postcondition: `assemblyscript-text-postprocess.md` satisfies AC-5 and AC-6.
- Files allowed to read: shared README/AssemblyScript fixture; local generator ranges at the captured HEAD.
- Files allowed to edit, fixture batches: at most three AssemblyScript fixture files per batch.
- Files allowed to edit, evidence batch: `assemblyscript-text-postprocess.md` only, after the fixture build/test completes.
- Files out of bounds: every file under `D:\wit-bindgen`, host/WIT behavior, other records.
- Dispatches: prerequisite/provenance `FACT`; generation/build/test `SNIPPETS`.
- Context cost: `M`.
- Authorities: captured clean `D:\wit-bindgen` HEAD evidence required by AC-6.
- Verification: AC-4 with AssemblyScript component; AC-5 candidate fields; AC-6 provenance.
- Exit condition: terminal measured result exists; missing `node`/`asc`/`wasm-tools` stops for user installation; dirty/wrong-branch/unconfirmed fork stops for user action without generation.

### Step 7: Run C++ probe

- Task IDs: `TASK-336`
- Objective: Generate C++-17+ bindings, compile with WASI SDK clang++, componentize, and measure without host WASI.
- Precondition: AssemblyScript has a terminal result; the same captured clean fork HEAD remains checked out and clean.
- Postcondition: `cpp-text-postprocess.md` records exact unresolved imports/output or success.
- Files allowed to read: shared README/C++ fixture; local C++ generator/test ranges.
- Files allowed to edit, fixture batches: at most three C++ fixture files per batch.
- Files allowed to edit, evidence batch: `cpp-text-postprocess.md` only, after the fixture build/test completes.
- Files out of bounds: `D:\wit-bindgen` edits, host WASI/linker changes, other records.
- Dispatches: prerequisite/provenance `FACT`; generation/build/test `SNIPPETS`.
- Context cost: `M`.
- Authorities: the same captured local generator HEAD used by AssemblyScript and WASI SDK official instructions.
- Verification: AC-4 with C++ component; AC-5 candidate fields; inspect imports with bounded `wasm-tools component wit`/`print` output.
- Exit condition: terminal measured result exists; missing `clang++`/WASI SDK/`wasm-tools` stops for user installation.

### Step 8: Run Go probe

- Task IDs: `TASK-336`
- Objective: Re-run Go against the same text world/oracle, last in candidate priority.
- Precondition: C++ has a terminal result.
- Postcondition: `go-text-postprocess.md` contains terminal measured evidence.
- Files allowed to read: shared README/Go fixture; historical Go record via delegated summary.
- Files allowed to edit, fixture batches: at most three Go fixture files per batch.
- Files allowed to edit, evidence batch: `go-text-postprocess.md` only, after the fixture build/test completes.
- Files out of bounds: host WASI/linker changes, other records.
- Dispatches: prerequisite `FACT`; build/test `SNIPPETS`.
- Context cost: `M`.
- Authorities: shared README and historical Go command evidence.
- Verification: AC-4 with Go component; AC-5 candidate fields.
- Exit condition: terminal measured result exists; missing Go/wasm-tools stops for user installation.

### Step 9: Compute and publish the language verdict

- Task IDs: `TASK-336`
- Objective: Compute the fixed-priority selection from four records and update the living language guide.
- Precondition: all four records have terminal non-blocked results.
- Postcondition: docs/14 reports each result and exactly one Dragon Curve authoring language.
- Files allowed to read: four candidate records; relevant `docs/14_submodule_programming_languages.md` section only.
- Files allowed to edit: `docs/14_submodule_programming_languages.md`.
- Files out of bounds: historical probe docs, plan/spec, packet 226/227 files.
- Dispatches: `FACT` independently computes first passing candidate from records.
- Context cost: `S`.
- Authorities: locked priority in packet requirements.
- Verification: AC-5 and AC-7.
- Exit condition: computed and recorded language agree; Rust appears only if all four fail.

## Per-Step Budget Roll-Up

| Step | Cost | Note |
| --- | --- | --- |
| 1 | S | bounded inventory |
| 2 | M | toolchain fallout |
| 2a | S | current-toolchain doc reconciliation |
| 3 | M | shared fixture/tool docs |
| 4 | S | one ignored integration test |
| 5-8 | M each | serialized scratch probe; stop on missing tool |
| 9 | S | deterministic transcription |

Aggregate remains M because each M step is independently bounded and serialized. Split before activation if any step expands to L or requires production linker/WIT redesign.

## Packet Completion Gate

- AC-1 through AC-9 and AC-N1 through AC-N3 pass.
- All four candidate records contain terminal measured results and no tooling blocker.
- Workspace check, clippy, targeted tests, guest rebuild, and freshness check pass.
- Update `docs/07_implementation_status.md` only when packet 228 creates/reconciles `TASK-336`; do not invent a row here.
- Packet remains `draft` until implementation and closure review.
