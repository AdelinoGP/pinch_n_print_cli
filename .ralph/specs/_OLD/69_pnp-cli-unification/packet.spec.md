---
status: implemented
packet: pnp-cli-unification
task_ids:
  - TASK-213
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: pnp-cli-unification

## Goal

Replace the `slicer-cli` and `slicer-host` binaries with a single `pnp_cli` binary by renaming `crates/slicer-host` → `crates/slicer-runtime` (library only, no binary target), extracting a `slicer_runtime::run::run_slice()` library entry point, externalising the 8 synthetic host built-ins onto a `Producer` trait that flows through both the DAG validator and `dag_cli`, consolidating manifest-validation constants into `slicer-schema`, and creating a new `crates/pnp-cli/` binary crate that owns the noun-namespaced verb tree (`slice`, `module new|diagnose|config-schema`, `mesh repair|decimate|import`, `dag stages|stage|depends|claims`).

## Scope Boundaries

This packet collapses every CLI-shaped concern in the workspace into one `pnp_cli` binary and one `slicer-runtime` library. It does not touch guest WASM contents, IR schemas, scheduler semantics, manifest TOML schema, the `wasm-tools component new` invocation shape, or the `modules/core-modules/build-core-modules.sh` and `test-guests/build-test-guests.sh` bash scripts (those retire in the follow-up packet `workspace-aware-guest-builder`). The full module-author build-flow rewrite in `docs/05_module_sdk.md` also waits for that packet; only binary-name renames land here.

## Prerequisites and Blockers

- Depends on: none (this packet is self-contained — verified by `grep` that no external crate imports `slicer_host`).
- Unblocks: `workspace-aware-guest-builder` (Packet 2 — that packet rewrites `docs/05_module_sdk.md`'s build section assuming the `pnp_cli` name and the deleted `slicer build` verb already exist).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a clean checkout post-merge, **when** the implementer runs `cargo build --workspace --release --bin pnp_cli`, **then** the build succeeds and produces `target/release/pnp_cli` (or `pnp_cli.exe` on Windows). | `cargo build --workspace --release --bin pnp_cli`
- **AC-2. Given** the workspace built per AC-1 and core-module guest WASMs built via `./modules/core-modules/build-core-modules.sh` (still the canonical builder until Packet 2), **when** the implementer runs `target/release/pnp_cli slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/pnp_smoke.gcode`, **then** the command exits 0 and `/tmp/pnp_smoke.gcode` is non-empty. | `cargo run --release --bin pnp_cli -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/pnp_smoke.gcode && test -s /tmp/pnp_smoke.gcode`
- **AC-3. Given** the extracted library API, **when** the test `slicer_runtime::run::run_slice` is invoked from `crates/slicer-runtime/tests/run_slice_api_tdd.rs` against an in-memory `SliceRunOptions` pointed at `resources/benchy.stl` + `modules/core-modules`, **then** the call returns `Ok(SliceOutcome)` with a non-empty `gcode_text` and `main.rs` contains no `_stale_build_plan` mod. | `cargo test -p slicer-runtime --test run_slice_api_tdd`
- **AC-4. Given** the externalised host built-ins, **when** the test `crates/slicer-runtime/tests/builtin_producers_tdd.rs` enumerates every `BuiltinProducer` constant exported from `mesh_analysis`, `region_mapping`, `prepass_slice`, `support_geometry`, `paint_segmentation`, and `gcode_emit`, **then** it finds exactly these 8 `(id, stage, ir_writes)` triples: `(host:mesh, PrePass::MeshAnalysis, [MeshIR])`, `(host:mesh_analysis, PrePass::MeshAnalysis, [SurfaceClassificationIR])`, `(host:region_mapping, PrePass::RegionMapping, [RegionMapIR])`, `(host:slice, PrePass::Slice, [SliceIR])`, `(host:shell_classification, PrePass::ShellClassification, [SliceIR])`, `(host:support_geometry, PrePass::SupportGeometry, [SupportGeometryIR])`, `(host:paint_segmentation, PrePass::PaintSegmentation, [PaintRegionIR])`, `(host:gcode_emit, PostPass::GCodeEmit, [GCodeIR])`. | `cargo test -p slicer-runtime --test builtin_producers_tdd`
- **AC-5. Given** the broadened `Producer` seam reaching `dag_cli`, **when** the implementer runs `pnp_cli dag claims --module-dir modules/core-modules --no-default-module-paths`, **then** the JSON output contains at least one entry whose `holders` array includes the literal string `"host:slice"` and at least one whose holders include `"host:gcode_emit"`. | `cargo run --release --bin pnp_cli -- dag claims --module-dir modules/core-modules --no-default-module-paths | grep -q '"host:slice"' && cargo run --release --bin pnp_cli -- dag claims --module-dir modules/core-modules --no-default-module-paths | grep -q '"host:gcode_emit"'`
- **AC-6. Given** the consolidated manifest validator, **when** the implementer greps for the duplicated validator constants across the workspace, **then** matches are returned only from files under `crates/slicer-schema/src/`. | `! grep -rln 'VALID_STAGES\|SUPPORTED_WIT_WORLDS\|RECOGNIZED_CLAIMS' cli/ crates/slicer-runtime/ crates/pnp-cli/ 2>/dev/null && grep -rln 'VALID_STAGES\|SUPPORTED_WIT_WORLDS\|RECOGNIZED_CLAIMS' crates/slicer-schema/`
- **AC-7. Given** the slicer-cli deletion, **when** the implementer checks the workspace, **then** `cli/slicer-cli/` does not exist and the workspace `Cargo.toml` `members` list does not include any path starting with `cli/`. | `test ! -d cli/slicer-cli && ! grep -E '^\s*"cli/' Cargo.toml`
- **AC-8. Given** the crate rename, **when** the implementer checks the workspace, **then** `crates/slicer-host/` does not exist, `crates/slicer-runtime/` does exist, no `.rs` file under `crates/slicer-runtime/` contains `use slicer_host::` or `slicer_host::` references, and the workspace `Cargo.toml` `members` list contains `crates/slicer-runtime` and `crates/pnp-cli` but not `crates/slicer-host`. | `test ! -d crates/slicer-host && test -d crates/slicer-runtime && ! grep -rln 'slicer_host::' crates/slicer-runtime/ && grep -q 'crates/slicer-runtime' Cargo.toml && grep -q 'crates/pnp-cli' Cargo.toml && ! grep -q 'crates/slicer-host' Cargo.toml`
- **AC-9. Given** the new verb tree, **when** the implementer runs `--help` against the four noun-namespaced parents and the top-level `slice` verb, **then** each invocation exits 0 and clap prints a subcommand list. | `cargo run --release --bin pnp_cli -- slice --help >/dev/null && cargo run --release --bin pnp_cli -- module --help >/dev/null && cargo run --release --bin pnp_cli -- mesh --help >/dev/null && cargo run --release --bin pnp_cli -- dag --help >/dev/null`
- **AC-10. Given** the extended scaffolding, **when** the test `crates/pnp-cli/tests/module_new_tdd.rs::emits_cargo_config_alias` runs `module_new::execute_in` against a tempdir, **then** the tempdir contains a `.cargo/config.toml` whose contents include the substring `build-wasm = "build --target wasm32-unknown-unknown --release"` and a `README.md` whose contents include the substring `wasm-tools component new`. | `cargo test -p pnp-cli --test module_new_tdd emits_cargo_config_alias`
- **AC-11. Given** the CI sweep, **when** the implementer inspects `.github/workflows/ci.yml`, **then** the file references `slicer-runtime` and `pnp-cli` as cargo package names and does not reference `slicer-host` or `slicer-cli` as cargo package names. | `grep -q 'slicer-runtime' .github/workflows/ci.yml && grep -q 'pnp-cli' .github/workflows/ci.yml && ! grep -E 'cargo test -p (slicer-host|slicer-cli)' .github/workflows/ci.yml`

## Negative Test Cases

- **AC-N1. Given** the hard-break of the `slicer-host` binary target, **when** the implementer runs `cargo build --workspace --release --bin slicer-host`, **then** the build fails because no such bin target exists. | `! cargo build --workspace --release --bin slicer-host 2>&1`
- **AC-N2. Given** the hard-break of the `slicer` binary, **when** the implementer runs `cargo build --workspace --release --bin slicer`, **then** the build fails because the `slicer-cli` crate (which owned that bin) has been deleted. | `! cargo build --workspace --release --bin slicer 2>&1`
- **AC-N3. Given** the deliberately-removed `build` verb (module authors use `cargo build --target wasm32-unknown-unknown --release` directly), **when** the implementer runs `pnp_cli build`, **then** clap exits non-zero with an "unrecognized subcommand" diagnostic. | `! cargo run --release --bin pnp_cli -- build 2>&1 | grep -E 'unrecognized subcommand|invalid value'`

## Verification

- `cargo build --workspace --release`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-runtime && cargo test -p slicer-schema && cargo test -p pnp-cli`

## Authoritative Docs

- `docs/00_project_overview.md` — read only the binary/crate map section (lines ~120–135) directly; the rest is delegated context if needed.
- `docs/05_module_sdk.md` — read only the binary-name section being renamed; delegate any cross-reference outside that section.
- `docs/13_slicer_helpers_crate.md` — read only the §"Integration with Host CLI" block (lines ~504–540) directly; the rest is delegated.
- `docs/16_slicer_report.md` — small (~180 lines); load directly.
- `docs/17_agent_debugging.md` — small (~130 lines); load directly.
- `CLAUDE.md` — load directly; the Guest WASM Staleness block is a small region the doc-sweep must touch.

## Doc Impact Statement (Required)

A list of specific doc sections this packet modifies, with verification greps:

- `docs/00_project_overview.md` §"Source tree" — binary/crate map renamed — `rg -q 'crates/slicer-runtime' docs/00_project_overview.md && rg -q 'crates/pnp-cli' docs/00_project_overview.md && ! rg -q 'crates/slicer-host' docs/00_project_overview.md`
- `docs/05_module_sdk.md` §"Developer CLI" — binary name → `pnp_cli` — `rg -q 'pnp_cli' docs/05_module_sdk.md && ! rg -E '^\s*slicer-host ' docs/05_module_sdk.md`
- `docs/13_slicer_helpers_crate.md` §"Integration with Host CLI" — verb names → `pnp_cli mesh …` — `rg -q 'pnp_cli mesh' docs/13_slicer_helpers_crate.md`
- `docs/16_slicer_report.md` §"CLI" — invocation renamed — `rg -q 'pnp_cli slice' docs/16_slicer_report.md`
- `docs/17_agent_debugging.md` (all CLI invocations) — `pnp_cli` substituted — `rg -q 'pnp_cli' docs/17_agent_debugging.md && ! rg -q 'slicer-host run' docs/17_agent_debugging.md`
- `CLAUDE.md` §"Build & Test Commands" + §"Guest WASM Staleness" — translation note + binary rename — `rg -q 'post-merge naming' CLAUDE.md && rg -q 'slicer-runtime' CLAUDE.md`
- `docs/07_implementation_status.md` — TASK-213 entry added with closure note — `rg -q 'TASK-213' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
