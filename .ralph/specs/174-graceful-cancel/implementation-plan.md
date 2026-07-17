# Implementation Plan: 174-graceful-cancel

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: `cancelled` progress event + `SliceRunOptions.cancel_flag` (TDD)

- Task IDs: `TASK-278`
- Objective: add `ProgressEventType::Cancelled` and `ProgressEvent::cancelled(slice_id, timestamp_ms)` (required fields `schema_version`, `event`, `timestamp_ms`, `slice_id`) to `crates/slicer-runtime/src/progress_events.rs`; bump `PROGRESS_EVENT_SCHEMA_VERSION`/`_INSTRUMENTED` one minor above the live value (live `1.2.0` at `progress_events.rs:35`, already consumed by the `slice_stats` row — attributed in docs/09 to `pinch_n_print_studio` T-096, implemented by packet 169; never take that row's version — dispatch below re-verifies the literal); add `cancel_flag: Option<Arc<AtomicBool>>` to `SliceRunOptions` (`run.rs:46-80`) and patch every in-tree struct literal (known: `crates/pnp-cli/src/main.rs:409`, `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs:188`; flush the rest via check). New test file `crates/slicer-runtime/tests/unit/cancel_flag_tdd.rs` (plus `mod cancel_flag_tdd;` in `tests/unit/main.rs`) starts with a serialization test: `ProgressEvent::cancelled(...)` JSON contains `"event":"cancelled"` and the four required fields.
- Precondition: tree builds clean; `PROGRESS_EVENT_SCHEMA_VERSION` live value re-confirmed by dispatch.
- Postcondition: event constructible and serialized correctly; options field exists; workspace type-checks.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/progress_events.rs` - lines `20-70`, `180-230`
  - `crates/slicer-runtime/src/run.rs` - lines `46-91`
  - `crates/slicer-runtime/tests/unit/main.rs` - full (short)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/progress_events.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/tests/unit/cancel_flag_tdd.rs` (new; its `tests/unit/main.rs` mod-line registration is part of this file's creation)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/pipeline.rs`, `layer_executor.rs` (Step 2), `.ralph/specs/169-*/**`
- Expected sub-agent dispatches:
  - Question: live `PROGRESS_EVENT_SCHEMA_VERSION` value and docs/09 table rows above 1.1.0; scope: `crates/slicer-runtime/src/progress_events.rs`, `docs/09_progress_events.md`; return: `FACT`
  - Question: every `SliceRunOptions {` struct literal in `crates/` (file:line); scope: `crates/`; return: `LOCATIONS`
  - Question: run `cargo check --workspace --all-targets`; return: `FACT` pass/fail + first 20 error lines
- Context cost: `M`
- Authoritative docs:
  - `docs/09_progress_events.md` - lines `118-121`, `156-168`
- OrcaSlicer refs: none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail (delegated)
- Exit condition: serialization test green and workspace type-checks with the new field everywhere; falsified if the constant was bumped to `1.2.0`.

### Step 2: Runtime plumbing — `PipelineConfig.cancel_flag`, per-layer checkpoint, `Cancelled` variant, event emission (TDD)

- Task IDs: `TASK-278`
- Objective: add `cancel_flag: Option<Arc<AtomicBool>>` to `PipelineConfig` (`pipeline.rs:51`), populate it at the `PipelineConfig` literal (`run.rs:624`), destructure in `run_pipeline_core` (`pipeline.rs:305`) with Relaxed-load checks before the PrePass/PerLayer/PostPass phase starts; add `LayerExecutionError::Cancelled` (`layer_executor.rs:58` + Display arm); add `cancel_flag: Option<&AtomicBool>` parameter to `execute_per_layer_with_instrumentation` (`layer_executor.rs:189`) guarded inside the `par_iter` closure (`:203`) before `execute_single_layer`; update all call sites (LOCATIONS dispatch); in `run_slice`'s error path (`run.rs`), record `ProgressEvent::cancelled(channel.slice_id, now_unix_ms())` on `channel.sink` when `opts.cancel_flag` is set. Extend `cancel_flag_tdd.rs`: pre-set flag ⇒ `run_slice` returns `Err`, collector stream contains exactly one `cancelled` and no `slice_complete` (AC-1); direct `execute_per_layer_with_instrumentation` call with set flag ⇒ `LayerExecutionError::Cancelled` (AC-2); `Some`-but-unset flag ⇒ `Ok` + `slice_complete` + no `cancelled` (AC-N1, test name `cancel_flag_unset_inert`).
- Precondition: Step 1 merged.
- Postcondition: AC-1/AC-2/AC-N1 unit tests green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - lines `40-140`, `291-410`
  - `crates/slicer-runtime/src/layer_executor.rs` - lines `58-140`, `189-240`
  - `crates/slicer-runtime/src/run.rs` - lines `194-300`, `560-660`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/pipeline.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/run.rs`
- Files explicitly out of bounds:
  - `execute_captured_stages` region (`layer_executor.rs:938+`), `crates/pnp-cli/**` (Step 3), `modules/**`
- Expected sub-agent dispatches:
  - Question: every call site of `execute_per_layer_with_instrumentation` (file:line); scope: `crates/`; return: `LOCATIONS`
  - Question: how do unit tests build a runnable `SliceRunOptions`/minimal pipeline today (name one existing test fn to pattern-match, e.g. in `tests/integration/progress_events_tdd.rs`); scope: `crates/slicer-runtime/tests/`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/09_progress_events.md` - lines `95-155` (ordering + canonical sequences)
- OrcaSlicer refs: none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test unit cancel_flag 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test integration progress_events_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (existing event contract unbroken)
- Exit condition: AC-1/2/N1 tests green and `progress_events_tdd` still green; falsified if a cancelled run also emits `slice_complete`.

### Step 3: CLI — `ctrlc` handler, `--cancel-on-stdin-eof` watcher, exit 130, output-absence guarantee (TDD)

- Task IDs: `TASK-278`
- Objective: add `ctrlc = "3"` to `crates/pnp-cli/Cargo.toml`; in `main.rs` slice arm: new `--cancel-on-stdin-eof` bool arg (help text names cancellation and exit code 130), create `Arc<AtomicBool>`, `ctrlc::set_handler` setting it (CTRL_BREAK coverage per FACT dispatch; add shim/feature if needed), spawn stdin-EOF watcher thread only when the arg is set, pass `Some(flag)` in `SliceRunOptions`; on `Err` from `run_slice` with flag set: best-effort `remove_file(output)`, notice on stderr, `std::process::exit(EXIT_CODE_CANCELLED)` with `pub const EXIT_CODE_CANCELLED: i32 = 130;`. New standalone test `crates/pnp-cli/tests/slice_cancel_tdd.rs` (assert_cmd, `resources/regression_wedge.stl`, `--module-dir modules/core-modules`, patterned on `slice_instrumentation_fork_tdd.rs:26-110`): `stdin_eof_cancels` (piped stdin closed at spawn + flag ⇒ exit 130, stderr has `"event":"cancelled"`, output path absent — AC-3) and `no_flag_stdin_eof_completes` (same spawn without flag ⇒ exit 0, output exists, no `cancelled` line — AC-4).
- Precondition: Steps 1-2 merged; `cargo xtask build-guests --check` clean before running the e2e test.
- Postcondition: AC-3/AC-4/AC-6 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/main.rs` - lines `44-130`, `346-440`
  - `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` - lines `1-110`
  - `crates/pnp-cli/Cargo.toml` - full
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/main.rs`
  - `crates/pnp-cli/Cargo.toml`
  - `crates/pnp-cli/tests/slice_cancel_tdd.rs` (new; standalone — no aggregator exists for pnp-cli tests)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (frozen after Step 2), `modules/**`
- Expected sub-agent dispatches:
  - Question: does `ctrlc` 3.x (default features) fire for `CTRL_BREAK_EVENT` on Windows?; scope: docs.rs / crate source; return: `FACT`
  - Question: run `cargo xtask build-guests --check`; return: `FACT` clean/STALE
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/fork-gaps-wave2-plan.md` §Packet 174 - direct read
- OrcaSlicer refs: none
- Verification:
  - `mkdir -p target && cargo test -p pnp-cli --test slice_cancel_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo run -p pnp-cli --bin pnp_cli -- slice --help 2>&1 | grep -c 'cancel-on-stdin-eof\|130' | grep -vqx 0 && echo PASS` - FACT PASS/absent
- Exit condition: both e2e tests green and help grep PASS; falsified if the no-flag run cancels or the flag run leaves an output file.

### Step 4: docs/09 schema row + cancellation sequence

- Task IDs: `TASK-278`
- Objective: in `docs/09_progress_events.md`, add the new additive version row for `cancelled` (the literal chosen in Step 1; the `1.2.0` `slice_stats` reservation row is left intact) to §Schema Version Cadence, and a cancellation excerpt under §Canonical Event Sequences (`layer_start` … `cancelled`, stream ends, exit 130, no `slice_complete`), plus the trigger contract (signals always; stdin EOF iff `--cancel-on-stdin-eof`).
- Precondition: Steps 1-3 merged (literal version known).
- Postcondition: AC-5 grep passes; table remains internally consistent.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/09_progress_events.md` - lines `95-168`
- Files allowed to edit (at most 3):
  - `docs/09_progress_events.md`
- Files explicitly out of bounds:
  - all source crates; `.ralph/specs/169-*/**`
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/09_progress_events.md` - ranged as above
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'cancelled' docs/09_progress_events.md && rg -q 'slice_stats' docs/09_progress_events.md && echo PASS` - FACT PASS/absent
- Exit condition: grep PASS with the `slice_stats` reservation untouched; falsified if the new row claims `1.2.0`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | event + options field + blast-radius flush |
| Step 2 | M | three-file runtime plumbing |
| Step 3 | M | CLI handlers + e2e process tests |
| Step 4 | S | docs table + sequence |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: add the `TASK-278` row (graceful cancel contract for pnp_cli, packet 174) and tick it.
- Reconcile reopened/superseded status transitions: none (no packet superseded).
- One-time manual signal check per platform (Ctrl+C / Ctrl+Break on Windows console; SIGINT on unix) recorded in closure notes — the automated suite covers only the stdin-EOF trigger deterministically.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
