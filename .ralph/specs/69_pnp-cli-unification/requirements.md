# Requirements: pnp-cli-unification

## Packet Metadata

- Grouped task IDs:
  - `TASK-213`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The workspace currently ships two binaries with overlapping surface area: `slicer-cli` (binary `slicer`, in `cli/slicer-cli/`) and `slicer-host` (binary `slicer-host`, in `crates/slicer-host/`). `slicer-cli run` shells out to `slicer-host run` after a partial duplicate of `slicer-host diagnose`'s manifest validation; `slicer-cli validate` ships its own copy of `VALID_STAGES`/`SUPPORTED_WIT_WORLDS`/`VALID_CONFIG_TYPES`/`RECOGNIZED_CLAIMS`/`VALID_SEVERITIES` parallel to `slicer-host::execution_plan::STAGE_ORDER`; `slicer-host::main()`'s `HostCommands::Run` arm is a 456-LOC god-function that inlines config parsing, mesh loading, an 8-row synthetic `LoadedModule` block to humour the DAG validator about host built-ins, and a 4-way instrumentation fork. The `crates/slicer-host` library has zero external importers (verified by `grep` across the workspace), so its name is a misnomer — it's the runtime library, not specifically "the host". The `dag_cli::run_dag_*` functions accept `&[LoadedModule]` and never see the synthetic host built-ins, so `slicer-host dag claims` lies by omission about who writes `MeshIR`/`SliceIR`/etc. This packet collapses both binaries into one (`pnp_cli`), renames the runtime crate honestly, externalises host built-ins behind a `Producer` trait that flows through both the DAG validator AND `dag_cli`, and removes the duplicate validator without leaving a transitional alias.

## In Scope

- Rename directory `crates/slicer-host/` → `crates/slicer-runtime/`; update the crate's own `Cargo.toml` `name = "slicer-runtime"`; mass-replace every `use slicer_host::` / `slicer_host::` reference in the crate's ~38 source files and `tests/` to `slicer_runtime`.
- Extract `slicer_runtime::run::run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError>` from the `HostCommands::Run` match arm. The library exposes `run_slice()` as its primary entry point; `main.rs` becomes a thin shim until step 9 removes it entirely.
- Delete the dead `_stale_build_plan` mod (originally `main.rs:813-924`).
- Define `Producer` trait + `BuiltinProducer` adapter in `crates/slicer-runtime/src/dag.rs` mirroring the projection the validator currently reads from `LoadedModule` (`id`, `stage`, `ir_writes`, `ir_reads`, `claims`, `requires_modules`, plus the IR-schema compat fields used in `validate_startup_dag`). Provide a blanket impl for `LoadedModule`.
- Move the 8 synthetic-row constructors (originally `main.rs:432-499`) into the modules that own each writer, exporting `const`/`fn` `BuiltinProducer` values: `mesh_analysis.rs::MESH_PRODUCER` + `MESH_ANALYSIS_PRODUCER`, `region_mapping.rs::REGION_MAPPING_PRODUCER`, `prepass_slice.rs::SLICE_PRODUCER` + `SHELL_CLASSIFICATION_PRODUCER`, `support_geometry.rs::SUPPORT_GEOMETRY_PRODUCER`, `paint_segmentation.rs::PAINT_SEGMENTATION_PRODUCER`, `gcode_emit.rs::GCODE_EMIT_PRODUCER`. Add a central `runtime_builtins()` registry in `crates/slicer-runtime/src/lib.rs` that returns all 8.
- Update `DagValidationRequest`, `build_intra_stage_dag`, AND `dag_cli::run_dag_stages` / `run_dag_stage` / `run_dag_depends` / `run_dag_claims` signatures to accept `&[&dyn Producer]` (or an equivalent projection). Update existing `dag_cli` tests to construct producers via the new API.
- Move `VALID_STAGES`, `SUPPORTED_WIT_WORLDS`, `VALID_CONFIG_TYPES`, `RECOGNIZED_CLAIMS`, `VALID_SEVERITIES` from `cli/slicer-cli/src/cmd_validate.rs:11-66` into `crates/slicer-schema/src/lib.rs` as public consts. Update `crates/slicer-runtime/src/manifest.rs` to import these from `slicer-schema`. (The transitional `slicer-cli::cmd_validate` import is moot because step 8 deletes the crate entirely.)
- Create `crates/pnp-cli/` workspace member with `[[bin]] name = "pnp_cli"`. Deps: `slicer-runtime`, `slicer-schema`, `clap` (workspace dep, derive feature), plus the read deps the dispatchers need. Dev-deps: `assert_cmd`, `predicates`, `tempfile`. Author `crates/pnp-cli/src/main.rs` as a thin clap dispatcher over the noun-namespaced verb tree: `slice`, `module new|diagnose|config-schema`, `mesh repair|decimate|import`, `dag stages|stage|depends|claims`. All verbs delegate to existing `slicer-runtime` functions; `module new` dispatches to a new local `module_new.rs`.
- Migrate the 4 `cargo_bin`-using CLI tests from `crates/slicer-runtime/tests/` to `crates/pnp-cli/tests/`: `e2e_integration_tdd.rs`, `helpers_cli.rs`, `cli_tdd.rs`, `module_search_path_tdd.rs`. Update each `Command::cargo_bin("slicer-host")` → `Command::cargo_bin("pnp_cli")` and rewrite verb invocations to the noun-namespaced form (e.g. `["run", ...]` → `["slice", ...]`, `["repair", ...]` → `["mesh", "repair", ...]`, `["dag", "stages"]` unchanged).
- Port `cli/slicer-cli/src/cmd_new.rs` into `crates/pnp-cli/src/module_new.rs`. Port its tests into `crates/pnp-cli/tests/module_new_tdd.rs`. Extend the template output to emit `.cargo/config.toml` containing `[alias]\nbuild-wasm = "build --target wasm32-unknown-unknown --release"` and `README.md` documenting the post-build `wasm-tools component new <input>.wasm -o target/slicer/<name>.wasm` step. Add a TDD test (`emits_cargo_config_alias`) asserting both files exist with the expected substrings.
- Delete `cli/slicer-cli/` entirely: remove from workspace `Cargo.toml:10`, delete the directory and its `tests/` subtree.
- Remove the `[[bin]] name = "slicer-host"` target from `crates/slicer-runtime/Cargo.toml`; delete `crates/slicer-runtime/src/main.rs` (after step 4 it is already a thin shim; this step removes the file outright). The runtime crate has no `[[bin]]` targets after this packet.
- Doc + CI + skill sweep (Q6 scope B in the plan):
  - Canonical docs: `CLAUDE.md`, `docs/00_project_overview.md`, `docs/05_module_sdk.md` (binary-name rename only — the build-flow rewrite waits for Packet 2), `docs/13_slicer_helpers_crate.md`, `docs/16_slicer_report.md`, `docs/17_agent_debugging.md`.
  - CI: `.github/workflows/ci.yml` — `cargo test -p slicer-host` → `cargo test -p slicer-runtime`; remove `cargo test -p slicer-cli` term; add `cargo test -p pnp-cli`.
  - Living agent surface: `.claude/skills/**/*.md`, `.agents/skills/**/*.md`, `.claude/agents/**/*.md` — any file that emits CLI invocations for `slicer-host` or `slicer`.
  - Active packets: any `.ralph/specs/<NN>_*/packet.spec.md` whose front-matter is `status: active` at landing time.
  - Translation note added to `CLAUDE.md`: one-line "post-merge naming: `slicer-host` library → `slicer-runtime`, `slicer-cli` crate deleted, `slicer`/`slicer-host` binaries → `pnp_cli`".
- Mark `.ralph/specs/_OLD/29_slicer-cli-cmd-run-cross-platform/` as superseded in this packet's `requirements.md` Problem Statement reference (NOT by editing that packet's files — cross-packet mutation rule).
- Append a TASK-213 entry to `docs/07_implementation_status.md` (via worker dispatch — do not load the full backlog).

## Out of Scope

- `modules/core-modules/build-core-modules.sh` and `test-guests/build-test-guests.sh` retirement → `workspace-aware-guest-builder` (Packet 2).
- Full module-author build-flow rewrite in `docs/05_module_sdk.md` (i.e., documenting the two-step `cargo build` + `wasm-tools component new` incantation as the canonical path) → Packet 2.
- A `cargo-generate` template repo for external module authors (intentionally deferred per plan Q8).
- Any change to WASM module ABI, manifest TOML schema, IR schemas, scheduler claim semantics, or the `wasm-tools component new` invocation shape.
- The `MeshHelper` trait abstraction over `repair`/`decimate`/`import` (C5 in the architecture review; parked).
- Editing closed `.ralph/specs/_OLD/` or historical packet text (treated as snapshots).
- `OrcaSlicerDocumented/` — foreign tree; no parity work in this packet.

## Authoritative Docs

- `docs/00_project_overview.md` (~150 lines) — load directly; the binary/crate map at ~lines 120–135 is the section that changes.
- `docs/05_module_sdk.md` (~700 lines) — delegate a SUMMARY of the "Developer CLI" section; the rest is unchanged in this packet.
- `docs/13_slicer_helpers_crate.md` (~600 lines) — delegate; only §"Integration with Host CLI" (lines ~504–540) needs editing.
- `docs/16_slicer_report.md` (~180 lines) — load directly.
- `docs/17_agent_debugging.md` (~130 lines) — load directly.
- `CLAUDE.md` (~150 lines) — load directly.
- `docs/07_implementation_status.md` (~200 lines) — delegate the TASK-213 append (worker dispatch); never load the full file.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11` from `packet.spec.md`. Refinements that didn't fit Given/When/Then:
  - The `Producer` trait's exact field surface (`id`, `stage`, `ir_writes`, `ir_reads`, `claims`, `requires_modules`, plus the IR-schema compat fields needed by `validate_startup_dag::IrVersionCompatibility`) is determined by reading the existing `DagValidationRequest` consumers — the trait is the smallest projection that satisfies them. A blanket impl for `&LoadedModule` is required so existing call sites that pass `&LoadedModule` slices continue to work via `as &dyn Producer`.
  - `SliceRunOptions` is the renamed `HostRunOptions` (current `crates/slicer-host/src/cli.rs:251-271`). The struct shape stays identical; only the name changes.
  - The 4-way instrumentation fork (currently `crates/slicer-host/src/main.rs:673-722`) moves inside `run_slice()` as a private helper. `run_slice` does NOT take instrumentation as a parameter — the four cases (`report`+`progress`, `report`-only, `progress`-only, none) are derived inside the function from `opts.report` and `opts.instrument_stderr`. This keeps the library API uncluttered and pushes composition policy into the function that owns the runtime semantics.
- Negative cases: `AC-N1`, `AC-N2`, `AC-N3` from `packet.spec.md`.
- Cross-packet impact: unblocks `workspace-aware-guest-builder` (Packet 2) by establishing the `pnp_cli` binary name that Packet 2's `docs/05_module_sdk.md` rewrite assumes. Supersedes `.ralph/specs/_OLD/29_slicer-cli-cmd-run-cross-platform/` (whose `cmd_run` workflow is deleted with the slicer-cli crate).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo build --workspace --release` | Workspace builds with new crate layout | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo build --workspace --release --bin pnp_cli` | The new binary builds (AC-1) | FACT pass/fail |
| `! cargo build --workspace --release --bin slicer-host 2>&1` | The old binary does NOT build (AC-N1) | FACT pass/fail |
| `! cargo build --workspace --release --bin slicer 2>&1` | The old binary does NOT build (AC-N2) | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo test -p slicer-runtime` | Runtime library tests | FACT pass/fail |
| `cargo test -p slicer-runtime --test run_slice_api_tdd` | run_slice library API (AC-3) | FACT pass/fail |
| `cargo test -p slicer-runtime --test builtin_producers_tdd` | Producer registry (AC-4) | FACT pass/fail |
| `cargo test -p slicer-schema` | Validator constants live in slicer-schema | FACT pass/fail |
| `cargo test -p pnp-cli` | pnp-cli CLI-flow tests + scaffold tests | FACT pass/fail |
| `cargo test -p pnp-cli --test module_new_tdd emits_cargo_config_alias` | Scaffold ergonomics (AC-10) | FACT pass/fail |
| `cargo run --release --bin pnp_cli -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/pnp_smoke.gcode` | E2E slice smoke (AC-2) | FACT exit-0 + file non-empty; SNIPPETS ≤ 20 lines on failure |
| `cargo run --release --bin pnp_cli -- dag claims --module-dir modules/core-modules --no-default-module-paths \| grep -q '"host:slice"'` | dag_cli sees built-ins (AC-5) | FACT pass/fail |
| `! grep -rln 'VALID_STAGES\|SUPPORTED_WIT_WORLDS\|RECOGNIZED_CLAIMS' cli/ crates/slicer-runtime/ crates/pnp-cli/ 2>/dev/null` | Validator consolidation (AC-6) | FACT empty = pass |
| `test ! -d cli/slicer-cli` | slicer-cli deletion (AC-7) | FACT pass/fail |
| `test ! -d crates/slicer-host && test -d crates/slicer-runtime` | Crate rename (AC-8) | FACT pass/fail |
| `! grep -rln 'slicer_host::' crates/slicer-runtime/` | No stale namespace refs (AC-8) | FACT empty = pass |
| `cargo run --release --bin pnp_cli -- slice --help` (and `module`, `mesh`, `dag` and `build` variants) | Verb tree wiring (AC-9, AC-N3) | FACT pass/fail per invocation |
| `grep -q 'slicer-runtime' .github/workflows/ci.yml && grep -q 'pnp-cli' .github/workflows/ci.yml && ! grep -E 'cargo test -p (slicer-host\|slicer-cli)' .github/workflows/ci.yml` | CI sweep (AC-11) | FACT pass/fail |

All commands are delegation-friendly. The slice smoke command (AC-2) requires core-module guest WASMs to be present; the implementer must run `./modules/core-modules/build-core-modules.sh` once at the start of the packet (or rely on already-built artifacts) — this is NOT redone per acceptance run.

## Step Completion Expectations

- Step 1 (the rename) must land as one atomic working-tree change. Partial renames leave the workspace uncompilable. The implementer must verify `cargo check --workspace` returns green before progressing past step 1.
- Steps 2–4 (Producer trait + synthetic-row move + run_slice extract) preserve the existing `cargo test -p slicer-runtime` result set — no test added or removed except the new TDD tests for AC-3 and AC-4. If a pre-existing test breaks during steps 2–4, the implementer must restore the test's pre-rename behaviour before adding the new ones.
- Step 6 (create `pnp-cli` crate) and step 7 (port `cmd_new`) can land before step 8 (delete `cli/slicer-cli/`), but `cli/slicer-cli/` MUST NOT be deleted before the validator constants are moved in step 5 — otherwise the validator surface vanishes.
- Doc sweep (step 10) MUST NOT edit `.ralph/specs/_OLD/` or any `status: implemented` packet (per cross-packet mutation rule); the translation note in `CLAUDE.md` is the documented bridge.

## Context Discipline Notes

- The biggest single read in this packet is `crates/slicer-host/src/main.rs` (~925 lines including the dead mod). The implementer must NOT load it in full. The structural anchors used in `implementation-plan.md` (`HostCommands::Run` match arm, `_stale_build_plan` mod boundary, synthetic-row block `host_builtin(…)`) localise edits to ranges of at most ~150 lines; range-read or delegate when the anchor is unclear.
- Tempting curiosity reads to skip: `docs/04_host_scheduler.md` (large; the scheduler semantics don't change in this packet); `OrcaSlicerDocumented/**` (no parity); the other ~30 `.rs` files under `crates/slicer-runtime/src/` (only the 8 listed in §"In Scope" need edits — for the rest, the rename sweep is mechanical and the implementer should rely on `cargo check` failure output to find missed sites rather than browsing them).
- Heaviest dispatch: the `cargo test -p slicer-runtime` after step 1 — could surface many compile errors from missed `slicer_host::` references. Required return format: SNIPPETS ≤ 20 lines of the first 5 distinct compile errors, NOT the full output. The implementer fixes them, re-dispatches, repeats until FACT pass.
