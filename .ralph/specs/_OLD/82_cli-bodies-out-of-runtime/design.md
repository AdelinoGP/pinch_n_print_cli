# Packet 82 — Design

## Controlling Code Paths

Three orthogonal moves in one packet. Each has a small change surface and they do not interact.

```
move 1: helpers_cmd.rs (744 LOC)  slicer-runtime/src/  →  pnp-cli/src/
move 2: cli.rs         (271 LOC)  slicer-runtime/src/  →  DELETE
                                                          (OutputFormat + write_with_parents → pnp-cli)
                                                          (HostCli + HostCommands → /dev/null)
gate 3: report/        (1 597 LOC) feature-gated within slicer-runtime
```

After this packet, `slicer-runtime/src/lib.rs` has 3 fewer `pub mod` declarations (`cli`, `helpers_cmd`, plus the feature-gated `report`) and roughly 8 fewer `pub use` lines. `pnp-cli` gains a `commands/` (or single-file) submodule and the two `cli.rs` utilities. The runtime binary path is unchanged.

OrcaSlicer comparison surface: none. None of the moved code ports OrcaSlicer behavior.

## Architecture Constraints

- ADR-0001/0002/0003 are unaffected — no WIT, no bindgen, no built-in producer touched.
- `slicer-runtime` MUST keep `clap` in `[dependencies]` because `dag_cli.rs` still uses it (clap parses subcommand args for the `dag` introspection tools). Removing `clap` is a P85 concern.
- `slicer-runtime`'s test bucket aggregators (`crates/slicer-runtime/tests/integration/main.rs`, `tests/executor/main.rs`) lose any `mod` declarations for tests that get migrated or deleted.
- The `report` feature MUST be in `default` so existing slice invocations work without `--features` flags. The gate is opt-out, not opt-in.

## Selected Approach

Direct move + delete + feature-gate. No abstraction, no shim.

Rejected alternatives:

- **Move `report/` into a separate `slicer-report` crate**. Rejected: report has no external deps that runtime doesn't already use (it depends only on `crate::instrumentation` types). Extracting a crate adds workspace bookkeeping for zero dep-tree reduction and zero depth gain. The feature gate inside `slicer-runtime` achieves the same opt-out cleanly.
- **Promote `HostCli`/`HostCommands` to `pnp-cli`** instead of deleting them. Rejected: the source comment (`cli.rs:34`) already says they are dead. `pnp-cli` has its own parser (noun-namespaced verb tree) that does not match `HostCommands`. Promoting would be code resurrection.
- **Move `dag_cli.rs` to `pnp-cli` in this packet**. Rejected: `dag_cli.rs` introspects the static DAG and belongs in the planning crate. Moving it to `pnp-cli` now would require moving it again to `slicer-scheduler` in P85. P85 owns the relocation.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-runtime/src/helpers_cmd.rs` | **DELETE** | Content moves to pnp-cli. |
| `crates/slicer-runtime/src/cli.rs` | **DELETE** | `HostCli`/`HostCommands` discarded; live items relocated. |
| `crates/pnp-cli/src/commands/mod.rs` | **CREATE** (or extend existing) | Hosts `OutputFormat` + the 4 helper command functions OR re-exports a single `helpers_cmd.rs` sibling. |
| `crates/pnp-cli/src/commands/helpers_cmd.rs` | **CREATE (from move)** | Verbatim content of `slicer-runtime/src/helpers_cmd.rs` with `use crate::commands::OutputFormat;` (after move) and `use slicer_model_io::...;` (unchanged from P81). |
| `crates/pnp-cli/src/io.rs` | **CREATE** | Hosts `write_with_parents` lifted from `cli.rs`. |
| `crates/pnp-cli/src/main.rs` | **EDIT** | Wire the new `commands::` module; dispatch `mesh convert/repair/decimate/import` subcommand arms to the moved functions. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Remove `pub mod cli;` + `pub mod helpers_cmd;` + matching `pub use` re-exports. Gate `pub mod report;` and its `pub use report::...` block with `#[cfg(feature = "report")]`. Keep `pub mod dag_cli;`. |
| `crates/slicer-runtime/src/run.rs` | **EDIT** | Gate `use crate::report::...` and all `report::Collector` / `report::allocator` references with `#[cfg(feature = "report")]`. Default behavior unchanged. |
| `crates/slicer-runtime/Cargo.toml` | **EDIT** | Add `[features] default = ["report"] report = []` section. No dep additions or deletions. |
| `crates/pnp-cli/Cargo.toml` | **EDIT** | Add `[features] default = ["report"] report = ["slicer-runtime/report"]` (forward the feature so the binary's `--report` handling compiles in step with the lib). |
| `crates/pnp-cli/src/main.rs` (slice subcommand argparse) | **EDIT** | Gate the `--report` flag definition AND its handler with `#[cfg(feature = "report")]`. When the feature is off, the flag is absent; users passing `--report ...` get the standard clap "unknown argument" error. |
| `crates/slicer-runtime/tests/**` | **EDIT or DELETE** | Tests importing `HostCli`/`HostCommands` are deleted. Tests of `helpers_cmd::*` integration move to `crates/pnp-cli/tests/`. Aggregators (`tests/integration/main.rs`, `tests/executor/main.rs`) lose corresponding `mod` declarations. |

Primary edit target ≤ 3 files: `crates/pnp-cli/src/` (counted as one — the new commands subtree), `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/run.rs`. All other edits are mechanical follow-on.

## Files in Scope (read+edit)

- The 12 files in the table above plus the conditional test file set surfaced by dispatch #2 below.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-runtime/src/helpers_cmd.rs` | Confirm the four `pub fn` signatures and what they import. | Read lines 1–80 (imports + first signatures) and the bottom 40 lines. Do not load in full. |
| `crates/slicer-runtime/src/cli.rs` | Confirm which items are live (used by `pnp-cli` or runtime) vs dead. | 271 LOC total — OK to load in full this time. |
| `crates/slicer-runtime/src/lib.rs` | Confirm exact `pub mod` and `pub use` lines that need to change. | Lines 7–48 (mod block) and 54 (`cli::*` re-exports), plus wherever `report::*` is re-exported. |
| `crates/slicer-runtime/src/run.rs` | Identify the `report` usage sites that need `#[cfg]` guards. | Grep for `report::` and `Collector` and `report_alloc::`. |
| `crates/pnp-cli/src/main.rs` | Identify the slice subcommand argparse to know where `--report` lives. | Search `report` and the slice subcommand match arm. |
| `crates/slicer-runtime/Cargo.toml` | Confirm current `[dependencies]` shape; check `clap` is workspace-inherited. | Full file (~80 lines after P81). |
| `docs/16_slicer_report.md` | Confirm the report's documented sentinel strings for AC-8. | Delegate SUMMARY if > 200 LOC. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work.
- `crates/slicer-runtime/src/report/{allocator.rs,collector.rs,model.rs,render.rs,mod.rs}` — DO NOT LOAD. The feature gate works at the import site; the content is irrelevant.
- `crates/slicer-runtime/src/dag_cli.rs` — out of scope (P85 territory).
- `crates/slicer-runtime/src/wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` — P83 territory.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | Is `SliceRunOptions` referenced anywhere outside `cli.rs` (e.g., from `run.rs`, `pnp-cli`, or tests)? If yes, where? | `crates/slicer-runtime/src/`, `crates/pnp-cli/src/`, `crates/slicer-runtime/tests/` | LOCATIONS (≤ 10 entries) |
| 2 | Which tests under `crates/slicer-runtime/tests/` import `HostCli`, `HostCommands`, or `slicer_runtime::helpers_cmd::*`? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 20 entries) |
| 3 | Which lines in `crates/slicer-runtime/src/run.rs` reference `report::*` or `report_alloc::*` (the gate sites)? | `crates/slicer-runtime/src/run.rs` | LOCATIONS (file:line, ≤ 10 entries) |
| 4 | Baseline SHAs: capture `sha256sum` of `pnp_cli mesh convert/repair/decimate/import` outputs against canonical fixtures. | repo root | SNIPPETS (4 SHAs, one per subcommand) |
| 5 | After move: confirm `cargo build --workspace` and `cargo build --no-default-features -p slicer-runtime` both green. | repo root | FACT pass/fail × 2 |
| 6 | After move: confirm `cargo test -p slicer-runtime -p pnp-cli` green; counts ± delta. | repo root | FACT pass/fail + counts |
| 7 | After move: confirm post-packet SHAs for the four `mesh *` outputs match the baselines. | repo root | FACT pass/fail per subcommand |
| 8 | Documented sentinel strings for AC-8 report file. | `docs/16_slicer_report.md` | FACT (1–2 lines, e.g., "`<!DOCTYPE html>` on line 1; `<title>` contains `Slicer Report`") |

## Data and Contract Notes

- `OutputFormat` enum value names (`stl`, `obj`, `3mf`) are CLI-facing. They must be preserved exactly during the move because users type them on the command line.
- `write_with_parents` is filesystem-side; no contract change.
- The `--report` flag's semantics on default builds (a `--report <PATH>` arg, optional, writes HTML to PATH) are preserved exactly. On `--no-default-features`, the flag is absent.
- `Collector::new_with_verbose` and `report_alloc::{enable, disable}` are the only `report::*` symbols `run.rs` calls today (verify via dispatch #3). All references must be `#[cfg]`-gated.

## Locked Assumptions and Invariants

- `dag_cli.rs` stays in `slicer-runtime` through this packet. Its `pub fn`s remain re-exported at `lib.rs`. P85 moves the file; this packet leaves it untouched.
- The `report` feature is in `default`. Slice invocations without explicit `--features` continue to support `--report` exactly as today.
- No g-code output change: byte-identical g-code for `pnp_cli slice ... resources/benchy.stl` between this packet and P81's closure SHA (the P81 baseline SHA carries forward).
- `clap` stays in `slicer-runtime/Cargo.toml` because `dag_cli.rs` uses it.

## Risks and Tradeoffs

- **Risk: report-feature gate misses a call site.** If `run.rs` (or another file) references `report::*` without `#[cfg(feature = "report")]`, `--no-default-features` will fail to build. Mitigation: dispatch #3 enumerates the sites; the implementer guards each.
- **Risk: `SliceRunOptions` is referenced from `run.rs`.** If `SliceRunOptions` is the public configuration shape that `run_slice` takes, deleting it cascades. Mitigation: dispatch #1 enumerates references; if `SliceRunOptions` is used by `run.rs`, it stays in `slicer-runtime` (move it from `cli.rs` to `run.rs` or `lib.rs`) and is unaffected by the `cli.rs` deletion.
- **Tradeoff: `pnp-cli` grows.** ~750 LOC added. Acceptable — the binary is the rightful home for CLI bodies.
- **Tradeoff: feature gate adds `#[cfg]` blocks across 4 files.** Acceptable — the alternative (always-compile `report/`) has no dep-tree benefit and removes the user's opt-out.

## Context Cost Estimate

- Aggregate: **M** (5 S-steps, 2 M-steps).
- Largest single step: step 3 (the actual move + feature gate, M). It edits ~10 files; the implementer must be careful with `#[cfg]` propagation.
- Highest-risk dispatch: dispatch #5 (the dual `cargo build` — default and `--no-default-features`). Both must be green.

## Open Questions

None. `None — change is reversible via reverting moves; the feature gate is the only behavior introduction and it preserves default behavior.`
