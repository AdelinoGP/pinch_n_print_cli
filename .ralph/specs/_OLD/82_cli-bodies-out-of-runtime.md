---
status: implemented
packet: 82
task_ids: [TASK-232]
---

# 82_cli-bodies-out-of-runtime

## Goal

Move `helpers_cmd.rs` (744 LOC; the `mesh convert/repair/decimate/import` command bodies) out of `slicer-runtime/src/` into `crates/pnp-cli/src/`; delete `cli.rs` (271 LOC; the dead `HostCli`/`HostCommands` clap-Parser types, retained "for parser-shape tests" per the source comment but no longer the entry point) while migrating its still-used `OutputFormat` enum and `write_with_parents` helper into `pnp-cli`; and put `slicer-runtime/src/report/` (1 597 LOC HTML rendering subsystem) behind a default-enabled `report` Cargo feature so that `cargo build --no-default-features -p slicer-runtime` builds the runtime without the reporting subsystem compiled.

## Problem Statement

`slicer-runtime` hosts three things that have no business in a host library crate:

1. **`helpers_cmd.rs` (744 LOC)** — the bodies of the `mesh convert`, `mesh repair`, `mesh decimate`, `mesh import` CLI subcommands. They consume `slicer_model_io::load_model` (after P81) and `slicer_helpers` and emit files. They are CLI presentation, not a runtime contract. Today they sit in the library crate so `pnp-cli` can call them as library functions, inverting the natural direction (binary depends on lib).
2. **`cli.rs` (271 LOC)** — a clap-Parser definition (`HostCli`, `HostCommands`) that the source comment at `cli.rs:34` already labels "no longer the program entry point; retained as a library surface for parser-shape tests." It is dead weight. Two utility items embedded in the same file — `OutputFormat` (a `ValueEnum` used by helpers commands) and `write_with_parents` — are still live and belong with the helpers in `pnp-cli`.
3. **`report/` (1 597 LOC)** — opt-in HTML rendering for the `--report` flag. Mandatory in all builds today even when reporting is unwanted; offers a clean feature-gate opportunity that costs almost nothing because `report/` already depends only on `crate::instrumentation` types (no extra external deps).

The fix is three structural moves: helpers_cmd into the binary, cli.rs deleted with its live items rehomed, report/ behind a default-enabled feature. `dag_cli.rs` (633 LOC) stays in `slicer-runtime` for this packet — it is planning-time introspection and moves to `slicer-scheduler` in packet 85.

## Architecture Constraints

- ADR-0001/0002/0003 are unaffected — no WIT, no bindgen, no built-in producer touched.
- `slicer-runtime` MUST keep `clap` in `[dependencies]` because `dag_cli.rs` still uses it (clap parses subcommand args for the `dag` introspection tools). Removing `clap` is a P85 concern.
- `slicer-runtime`'s test bucket aggregators (`crates/slicer-runtime/tests/integration/main.rs`, `tests/executor/main.rs`) lose any `mod` declarations for tests that get migrated or deleted.
- The `report` feature MUST be in `default` so existing slice invocations work without `--features` flags. The gate is opt-out, not opt-in.

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
