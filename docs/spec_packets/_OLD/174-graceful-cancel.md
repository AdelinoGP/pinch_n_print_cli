---
status: implemented
packet: 174-graceful-cancel
task_ids:
  - TASK-278
---

# 174-graceful-cancel

## Goal

Give `pnp_cli slice` a graceful-cancel contract: CTRL_BREAK_EVENT/CTRL_C (Windows) and SIGINT (unix) — plus stdin EOF behind an opt-in `--cancel-on-stdin-eof` flag for the fork's close-the-child's-stdin cancel path — set a shared `AtomicBool` that the per-layer execution loop checks, producing a `cancelled` JSONL progress event, a guaranteed-absent output file, and the distinct documented exit code 130.

## Problem Statement

`crates/pnp-cli` has zero signal handling: killing a slice mid-run (Ctrl+C, Ctrl+Break, or the fork closing the child's stdin pipe) hard-kills the process with no protocol-level acknowledgement. The fork (handoff item 11) needs a deterministic cancel contract: it closes the child's stdin and expects the slicer to stop promptly, say so on the JSONL progress stream, leave no output artifact, and exit with a code the fork can distinguish from failure. One grounding correction against the plan: the plan located the checkpoint as "the module-execution loop over `global_layers` in `run.rs`" — the real loop is the rayon `par_iter` over `plan.global_layers` in `execute_per_layer_with_instrumentation` (`crates/slicer-runtime/src/layer_executor.rs:189-215`); `run.rs:369` is an unrelated paint-scan loop. The checkpoint lands in the real loop.

## Architecture Constraints

- Host-side only: no file under `modules/`, `crates/slicer-schema/wit/`, `slicer-sdk`, `slicer-macros`, or `slicer-ir` is touched, so the guest-WASM staleness gate is not triggered by this packet's edits (the pnp-cli e2e test still needs previously built guests on disk).
- No geometry or mm/unit conversion anywhere in this packet; the coordinate-system checklist does not apply.
- Progress-event schema rules (`docs/09_progress_events.md` §Compatibility): additive event ⇒ minor bump only; `slice_complete`/`module_error` must never be dropped — on cancel no `slice_complete` is emitted at all (mirroring the existing fatal-abort sequence, which also ends without `slice_complete`).
- Event-type serialization is `snake_case` (`progress_events.rs:66` `#[serde(rename_all = "snake_case")]`): variant `Cancelled` ⇒ wire string `cancelled`.
- Config keys snake_case: this packet adds no config key (the cancel switch is the CLI flag `--cancel-on-stdin-eof`, kebab-case like every existing flag).

## Data and Contract Notes

- IR/manifest contracts: none touched.
- WIT boundary: none.
- Progress-event contract: `cancelled` is additive; required fields `schema_version`, `event`, `timestamp_ms`, `slice_id`. Emitted at most once, only on the cancel path, never followed by `slice_complete` (parallel to the documented fatal-abort sequence). Version target is computed from the live constant at implementation time; the live constant is already `1.2.0` (`progress_events.rs:35`), consumed by the `slice_stats` row — a row `docs/09_progress_events.md` attributes to `pinch_n_print_studio` T-096, with packet 169 implementing it. This packet takes the next free minor above the live constant (`1.3.0` as of this grounding; re-verify via the Step-1 FACT dispatch) and never takes the `slice_stats` row's version. ACs deliberately assert the event, not a version literal.
- Process contract (fork-facing): cancel triggers = OS signal, or stdin EOF iff `--cancel-on-stdin-eof`; acknowledgement = `cancelled` JSONL on stderr (when progress events are enabled; with `--no-progress-events` the only signals are the exit code and absent output); exit code 130; `--output` path guaranteed absent.
- Determinism/scheduler: the checkpoint reads the flag with `Ordering::Relaxed`; layers already scheduled may finish (cancel latency ≤ in-flight layer batch). No change to layer ordering or module scheduling when the flag is unset.

## Locked Assumptions and Invariants

- The CLI writes the output file only after `run_slice` returns `Ok` (main.rs:425-433); therefore no partial G-code file can exist on cancel — AC-3 asserts absence, and the `remove_file` is defensive only. Any future streaming writer must revisit this packet's contract.
- Exit code 130 is the sole cancellation exit code and is documented in `slice --help`.
- Stdin-EOF cancel is opt-in forever (flag-gated); flagless behaviour with closed stdin is locked by AC-4.
- `cancel_flag: None` reproduces today's behaviour bit-for-bit (AC-N1 guards the `Some`-but-unset case).

## Risks and Tradeoffs

- `ctrlc` CTRL_BREAK coverage is asserted by the crate's Windows implementation but verified by dispatch before wiring ([FWD] below); the automated e2e test drives the stdin-EOF path (deterministic cross-platform), not real console events — signal delivery itself is manually verified once per platform at the acceptance ceremony and recorded in closure notes.
- Signature change to `execute_per_layer_with_instrumentation` touches its callers; blast radius bounded by the Step-2 LOCATIONS dispatch.
- Shared-file contention with draft packet 169 (`progress_events.rs`, docs/09 table): additive edits in different regions; second-lander rebases the version row.
- A layer stuck inside a WASM module cannot be interrupted; accepted (out of scope) and stated in the fork-facing contract.
