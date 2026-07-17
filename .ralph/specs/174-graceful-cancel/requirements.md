# Requirements: 174-graceful-cancel

## Packet Metadata

- Grouped task IDs: `TASK-278` (new; minted at closure via `task-map.md` — not yet a row in `docs/07_implementation_status.md`)
- Backlog source: `docs/07_implementation_status.md` (wave-2 plan `docs/specs/fork-gaps-wave2-plan.md` §Packet 174, handoff item 11)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`crates/pnp-cli` has zero signal handling: killing a slice mid-run (Ctrl+C, Ctrl+Break, or the fork closing the child's stdin pipe) hard-kills the process with no protocol-level acknowledgement. The fork (handoff item 11) needs a deterministic cancel contract: it closes the child's stdin and expects the slicer to stop promptly, say so on the JSONL progress stream, leave no output artifact, and exit with a code the fork can distinguish from failure. One grounding correction against the plan: the plan located the checkpoint as "the module-execution loop over `global_layers` in `run.rs`" — the real loop is the rayon `par_iter` over `plan.global_layers` in `execute_per_layer_with_instrumentation` (`crates/slicer-runtime/src/layer_executor.rs:189-215`); `run.rs:369` is an unrelated paint-scan loop. The checkpoint lands in the real loop.

## In Scope

- New `pnp-cli` dependency `ctrlc` (choice + coverage evaluation recorded in `design.md`): handler for SIGINT (unix) and CTRL_C_EVENT/CTRL_BREAK_EVENT (Windows) that sets a shared `Arc<AtomicBool>`.
- Stdin-EOF cancel, opt-in via new CLI flag `--cancel-on-stdin-eof` (slice verb only): a watcher thread reads stdin to EOF, then sets the same flag. Opt-in because non-interactive shells (`pnp_cli slice < /dev/null`, CI pipes) present instant EOF; unconditional EOF-cancel would cancel every scripted run. The fork passes the flag and closes the pipe to cancel. AC-4 locks the without-flag behaviour.
- Plumbing: `SliceRunOptions.cancel_flag: Option<Arc<AtomicBool>>` (`crates/slicer-runtime/src/run.rs:46-80`) → `PipelineConfig.cancel_flag` (`crates/slicer-runtime/src/pipeline.rs:51`, populated at the `PipelineConfig` literal in `run.rs:624`) → checkpoint parameter of `execute_per_layer_with_instrumentation` (`layer_executor.rs:189`).
- Checkpoints: (a) inside the per-layer `par_iter` closure before `execute_single_layer` (`layer_executor.rs:203-204`) — flag set ⇒ return new `LayerExecutionError::Cancelled`; (b) cheap pre-phase checks in `run_pipeline_core` (`pipeline.rs:299`) before prepass, per-layer, and postpass so cancellation between phases doesn't wait a whole phase.
- New progress event: `ProgressEventType::Cancelled` (serialized `cancelled`, snake_case like all event types) + constructor `ProgressEvent::cancelled(slice_id, timestamp_ms)`; emitted by `run_slice` when it observes the set flag on the error path, before returning. Additive schema-minor bump of `PROGRESS_EVENT_SCHEMA_VERSION` (live value `1.2.0` at `progress_events.rs:35`, both plain and `_INSTRUMENTED` — `1.2.0` is already consumed in the working tree by the `slice_stats` row, which `docs/09_progress_events.md` attributes to `pinch_n_print_studio` T-096 and packet 169 implements). The exact target version is computed at implementation time as the next free minor above the live constant and MUST NOT take the `slice_stats` row's version.
- CLI behaviour on cancel: `run_slice` returns `Err`; `main.rs` checks the flag — when set, ensure the `--output` path does not exist (defensive `remove_file`; today the CLI only writes output after a successful `run_slice`, so no partial file is ever produced — the guarantee is asserted, not merely the removal), and `std::process::exit(130)` (POSIX SIGINT-convention code, documented in help text).
- Tests: `crates/slicer-runtime/tests/unit/cancel_flag_tdd.rs` (registered in `tests/unit/main.rs`) for AC-1/AC-2/AC-N1; new standalone `crates/pnp-cli/tests/slice_cancel_tdd.rs` (assert_cmd, `resources/regression_wedge.stl` + `modules/core-modules`, patterned on `slice_instrumentation_fork_tdd.rs`) for AC-3/AC-4.
- Docs: `docs/09_progress_events.md` new version row + cancellation canonical-sequence excerpt.

## Out of Scope

- Interrupting a WASM module mid-dispatch (epoch/fuel-based interruption); cancel latency is bounded by the slowest single layer/stage.
- Cancel support in other verbs (`visual-debug`, `mesh`, `dag`, `module`) and in `execute_captured_stages` (visual-debug tap path).
- SIGTERM/CTRL_CLOSE/logoff handling beyond what the chosen `ctrlc` configuration provides.
- Progress-event emission of partial statistics on cancel (`slice_stats` belongs to packet 169).
- Any streaming G-code writer (output remains written whole after success).

## Authoritative Docs

- `docs/09_progress_events.md` — ~170 lines; direct ranged read of §Compatibility and §Schema Version Cadence only.
- `docs/specs/fork-gaps-wave2-plan.md` §Packet 174 — direct read; contract source (checkpoint location corrected, see Problem Statement).

## Acceptance Summary

- Positive: `AC-1` through `AC-6`. Refinement absent from AC text: AC-3's test must poll-wait for process exit with a generous timeout (slice of `regression_wedge.stl` cancels at the first checkpoint, so runtime is short) rather than sleeping a fixed interval.
- Negative: `AC-N1` (unsignalled flag is inert; `slice_complete` still emitted), plus AC-4 doubles as the no-flag stdin-EOF rejection case.
- Cross-packet impact: shares `progress_events.rs` and `docs/09_progress_events.md` with draft packet 169 (which amends the reserved `1.2.0` row). Version-row ordering is reconciled at implementation time: whichever packet lands second takes the next free minor above the then-live constant.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag 2>&1 | tee target/test-output.log | grep -E "^test result"` | Flag plumbing + Cancelled variant + cancelled event + inert case (AC-1/2/N1) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p pnp-cli --test slice_cancel_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | End-to-end process contract: exit 130, cancelled JSONL, no output file, no-flag completion (AC-3/4) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo run -p pnp-cli --bin pnp_cli -- slice --help 2>&1 | grep -c 'cancel-on-stdin-eof\|130' | grep -vqx 0 && echo PASS` | Help-text contract (AC-6) | FACT PASS/absent |
| `rg -q 'cancelled' docs/09_progress_events.md && rg -q 'slice_stats' docs/09_progress_events.md && echo PASS` | Doc-impact grep (AC-5) | FACT PASS/absent |
| `cargo check --workspace --all-targets` | Whole-tree type gate incl. tests | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate (required before commit) | FACT pass/fail |

## Step Completion Expectations

Step 1 adds `cancel_flag` to `SliceRunOptions`, which breaks every in-tree struct-literal constructor until they gain the field in the same step (known sites: `main.rs:409`, `visual_debug_agent_overhead_tdd.rs:188`; the step runs `cargo check --workspace --all-targets` to flush the rest). Steps 2-3 depend on the field existing. No other cross-step state.

## Context Discipline Notes

- `crates/slicer-runtime/src/run.rs` is >900 lines — read only the `SliceRunOptions` struct (~46-91) and the `PipelineConfig` construction window (~600-650); never the full file.
- `crates/slicer-runtime/src/layer_executor.rs` is >1300 lines — read only `LayerExecutionError` (~58-140) and `execute_per_layer_with_instrumentation` (~189-240).
- The pnp-cli e2e test slices a real model through WASM modules; the run requires fresh guest artifacts on disk (`cargo xtask build-guests --check` before attributing its failure to this packet), even though this packet edits no guest-feeding path.
