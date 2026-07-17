---
status: draft
packet: 174-graceful-cancel
task_ids:
  - TASK-278
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 174-graceful-cancel

## Goal

Give `pnp_cli slice` a graceful-cancel contract: CTRL_BREAK_EVENT/CTRL_C (Windows) and SIGINT (unix) — plus stdin EOF behind an opt-in `--cancel-on-stdin-eof` flag for the fork's close-the-child's-stdin cancel path — set a shared `AtomicBool` that the per-layer execution loop checks, producing a `cancelled` JSONL progress event, a guaranteed-absent output file, and the distinct documented exit code 130.

## Scope Boundaries

Host-side only: `crates/pnp-cli` (signal handlers, stdin watcher thread, exit code, `ctrlc` dependency), `crates/slicer-runtime` (`SliceRunOptions.cancel_flag`, `PipelineConfig.cancel_flag`, checkpoint in `execute_per_layer_with_instrumentation`'s loop over `plan.global_layers`, new `cancelled` progress event), and `docs/09_progress_events.md`. No WIT, IR, module, guest-WASM, or scheduler change; no cancellation inside a running WASM module dispatch (cancel takes effect at the next layer/phase boundary).

## Prerequisites and Blockers

- Depends on: nothing. The 1.2.0 `slice_stats` schema row (attributed in docs/09 to `pinch_n_print_studio` T-096; implemented by packet 169) is already consumed — the live constant is 1.2.0 at `progress_events.rs:35`; this packet adds a NEW additive row above it (see Doc Impact).
- Unblocks: fork handoff item 11 (frontend cancel button / process teardown).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a `SliceRunOptions` with `cancel_flag` pre-set to `true`, **when** `run_slice` executes, **then** it returns `Err` without completing the pipeline, and the recorded progress stream contains exactly one event with `"event":"cancelled"` carrying `schema_version`, `timestamp_ms`, and `slice_id` fields, and no `slice_complete` event. | `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** the per-layer phase with a cancel flag that flips to `true`, **when** `execute_per_layer_with_instrumentation` runs, **then** it stops scheduling further layers and returns `LayerExecutionError::Cancelled` (new variant), which maps to `PipelineError::LayerExecution`. | `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `pnp_cli slice --cancel-on-stdin-eof` spawned with a piped stdin that is closed immediately, **when** the process runs, **then** it exits with code 130, stderr contains a `"event":"cancelled"` JSONL line, and the `--output` path does not exist afterwards. | `mkdir -p target && cargo test -p pnp-cli --test slice_cancel_tdd stdin_eof 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `pnp_cli slice` WITHOUT `--cancel-on-stdin-eof` spawned with a piped, immediately-closed stdin, **when** the process runs, **then** the slice completes normally: exit code 0, the `--output` file exists, and stderr contains no `"event":"cancelled"` line. | `mkdir -p target && cargo test -p pnp-cli --test slice_cancel_tdd no_flag_stdin_eof_completes 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** the docs edits land, **when** grepping `docs/09_progress_events.md`, **then** a new additive schema row documents the `cancelled` event and the 1.2.0 `slice_stats` row is intact. | `rg -q 'cancelled' docs/09_progress_events.md && rg -q 'slice_stats' docs/09_progress_events.md && echo PASS`
- **AC-6. Given** the CLI help text, **when** running `pnp_cli slice --help`, **then** it documents `--cancel-on-stdin-eof` and exit code 130 for cancellation. | `cargo run -p pnp-cli --bin pnp_cli -- slice --help 2>&1 | grep -c 'cancel-on-stdin-eof\|130' | grep -vqx 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a normal slice with `cancel_flag` present but never set, **when** `run_slice` completes, **then** the outcome is `Ok`, no `cancelled` event is recorded, and `slice_complete` is emitted (cancellation plumbing is inert when unsignalled). | `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag_unset_inert 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p pnp-cli --test slice_cancel_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/09_progress_events.md` — direct ranged read: §Compatibility (~lines 118-121) and §Schema Version Cadence (~lines 156-168) only.
- `docs/07_implementation_status.md` — delegated; TASK-278 minted at closure via `task-map.md`.

## Doc Impact Statement (Required)

- `docs/09_progress_events.md` section "Schema Version Cadence" — add a new additive row for the `cancelled` event (next free minor version above the live `PROGRESS_EVENT_SCHEMA_VERSION` — `1.2.0` at `progress_events.rs:35`, already consumed by the `slice_stats` row, attributed to `pinch_n_print_studio` T-096 and implemented by packet 169; MUST NOT take that row's version) and a cancellation excerpt under "Canonical Event Sequences" - `rg -q 'cancelled' docs/09_progress_events.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
