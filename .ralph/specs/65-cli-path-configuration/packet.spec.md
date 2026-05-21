---
status: implemented
packet: 65-cli-path-configuration
task_ids:
  - TASK-204
  - TASK-205
  - TASK-206
  - TASK-207
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 65-cli-path-configuration

## Goal

Clean up stale CLI path configuration in `slicer-host` by completing `HostRunOptions`, deleting dead `validate_run_options` / `CliError`, removing the ignored `--module` flag, creating parent directories for output and report files before write, and normalizing `String` CLI arg types to `PathBuf`.

## Scope Boundaries

This packet refactors the `crates/slicer-host/src/cli.rs` and `main.rs` entry point to eliminate dead code and bring CLI arg types into consistency. `HostRunOptions` gains the missing `thumbnail`, `report`, `report_verbose` fields and becomes the single validated options struct constructed directly from CLI args in `main.rs`. The `validate_run_options` function and `CliError` enum (dead code, never called from `main.rs`) are removed. The legacy `--module` flag (parsed but silently discarded) is removed entirely. Output and report file writes gain `create_dir_all` for parent directories. All changes are host-only: no WASM, no IR schema, no WIT boundary, no OrcaSlicer parity.

## Prerequisites and Blockers

- Depends on: None.
- Unblocks: Any future packet that wants to inspect or extend `HostRunOptions` without inheriting dead-code drift.
- Activation blockers: None.

## Acceptance Criteria

- **AC-1. Given** the `HostRunOptions` struct, **when** inspected, **then** it contains fields `thumbnail: Option<PathBuf>`, `report: Option<PathBuf>`, and `report_verbose: bool` alongside the existing fields. | `rg -q 'thumbnail: Option<PathBuf>' crates/slicer-host/src/cli.rs && echo PASS || echo FAIL`
- **AC-2. Given** the source tree after refactoring, **when** searching for `validate_run_options` and `CliError` in production source, **then** no occurrence exists outside of git history. | `rg -c 'validate_run_options|CliError' crates/slicer-host/src/ | rg '^$' -q && echo PASS || echo FAIL`
- **AC-3. Given** the CLI definition, **when** running `slicer-host run --help`, **then** no `--module` flag is listed. | `cargo run --bin slicer-host -- run --help 2>&1 | rg -q -- '--module' && echo FAIL || echo PASS`
- **AC-4. Given** the `model`, `config`, and `output` CLI args, **when** inspecting their types, **then** all three are `PathBuf` or `Option<PathBuf>` rather than `String`. | `rg -q 'model(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'config(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'output(:.*)? PathBuf' crates/slicer-host/src/cli.rs && echo PASS || echo FAIL`
- **AC-5. Given** a `--output` path whose parent directory does not exist, **when** the pipeline completes, **then** the file is created successfully at the given path. | `cargo test -p slicer-host --test cli_tdd -- output_path_creates_parent_dir --nocapture`
- **AC-6. Given** the `main.rs` entry point, **when** the `Run` command is parsed, **then** `HostRunOptions` is constructed directly from CLI args and used for the remainder of the pipeline setup. | `rg -c 'validate_run_options' crates/slicer-host/src/main.rs | rg '^0$' -q && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** a CLI invocation that passes `--module` (a now-removed flag), **when** clap parses the arguments, **then** parsing fails with a non-zero exit code. | `cargo run --bin slicer-host -- run --module ./tmp/mod.wasm --model ./tmp/model.stl 2>&1; if ($LASTEXITCODE -ne 0) { echo PASS } else { echo FAIL }`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test cli_tdd`

## Authoritative Docs

- `crates/slicer-host/src/cli.rs` (entire file, 176 lines — read directly)
- `crates/slicer-host/src/main.rs` (entire file, 483 lines — read directly; skip `_stale_build_plan` module at lines 372-483)
- `crates/slicer-host/tests/cli_tdd.rs` (entire file, 204 lines — read directly)
- `crates/slicer-host/src/report/collector.rs` — lines 232-236 only (`finish_and_render_to` function)

## Doc Impact Statement

**`none`** — internal host-only refactor with no public surface change. No IR fields, WIT types, scheduler rules, claim IDs, manifest schemas, host services, or module SDK contracts are modified. The only external behavior change is `--module` flag removal and parent-dir creation for output files, both of which are invisible to modules and downstream consumers of the G-code output.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
