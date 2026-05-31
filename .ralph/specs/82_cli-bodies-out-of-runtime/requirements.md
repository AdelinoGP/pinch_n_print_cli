# Packet 82 — Requirements

## Problem Statement

`slicer-runtime` hosts three things that have no business in a host library crate:

1. **`helpers_cmd.rs` (744 LOC)** — the bodies of the `mesh convert`, `mesh repair`, `mesh decimate`, `mesh import` CLI subcommands. They consume `slicer_model_io::load_model` (after P81) and `slicer_helpers` and emit files. They are CLI presentation, not a runtime contract. Today they sit in the library crate so `pnp-cli` can call them as library functions, inverting the natural direction (binary depends on lib).
2. **`cli.rs` (271 LOC)** — a clap-Parser definition (`HostCli`, `HostCommands`) that the source comment at `cli.rs:34` already labels "no longer the program entry point; retained as a library surface for parser-shape tests." It is dead weight. Two utility items embedded in the same file — `OutputFormat` (a `ValueEnum` used by helpers commands) and `write_with_parents` — are still live and belong with the helpers in `pnp-cli`.
3. **`report/` (1 597 LOC)** — opt-in HTML rendering for the `--report` flag. Mandatory in all builds today even when reporting is unwanted; offers a clean feature-gate opportunity that costs almost nothing because `report/` already depends only on `crate::instrumentation` types (no extra external deps).

The fix is three structural moves: helpers_cmd into the binary, cli.rs deleted with its live items rehomed, report/ behind a default-enabled feature. `dag_cli.rs` (633 LOC) stays in `slicer-runtime` for this packet — it is planning-time introspection and moves to `slicer-scheduler` in packet 85.

## Grouped Task IDs

- **TASK-232** (new) — Move CLI bodies out of `slicer-runtime`; feature-gate the report subsystem. Recorded under "Architecture Deepening Phase I" alongside TASK-231 (packet 81) and TASK-233 (packet 83).

## In Scope

- Move `crates/slicer-runtime/src/helpers_cmd.rs` (744 LOC) into `crates/pnp-cli/src/`. File name and submodule layout flexible (e.g., `crates/pnp-cli/src/commands/mod.rs` + `convert.rs` + `repair.rs` + `decimate.rs` + `import.rs`, OR a single `helpers_cmd.rs`). The four entry functions (`run_repair`, `run_decimate`, `run_import`, `run_convert`) must be reachable from the subcommand dispatcher.
- Delete `crates/slicer-runtime/src/cli.rs`. Move its still-used items into `crates/pnp-cli/src/`:
  - `OutputFormat` enum → `crates/pnp-cli/src/commands/mod.rs` (or wherever the helper commands live).
  - `write_with_parents` fn → `crates/pnp-cli/src/io.rs` (or alongside `OutputFormat`).
  - DELETE `HostCli`, `HostCommands` entirely. They are dead per the source comment; pnp-cli already has its own parser.
  - DELETE `SliceRunOptions` if it is only referenced by `HostCli`/`HostCommands` and pnp-cli's parser builds an equivalent struct internally. (Confirm via dispatch #1.)
- Add `report` feature to `crates/slicer-runtime/Cargo.toml`:
  - `[features] default = ["report"] report = []`
  - The `report` feature has no associated optional deps because `report/` reuses crate-internal deps.
- Gate `crates/slicer-runtime/src/lib.rs`'s `pub mod report;` and its `pub use report::...` re-exports with `#[cfg(feature = "report")]`.
- Gate `crates/slicer-runtime/src/run.rs`'s `report::Collector` / `report::allocator` usages with `#[cfg(feature = "report")]`. The slice path must compile cleanly with the feature off and on.
- Gate `crates/pnp-cli`'s `--report` argparse flag and its handling logic with `#[cfg(feature = "slicer-runtime/report")]` OR equivalent feature propagation. When the feature is off, `--report` either is absent from the CLI or fails clearly at runtime with a "report support not compiled" message.
- Update `crates/slicer-runtime/src/lib.rs`: drop `pub mod cli;` and `pub mod helpers_cmd;` and their `pub use ...::...;` re-exports. KEEP `pub mod dag_cli;` and its re-exports (P85 moves that file).
- Migrate or delete tests in `crates/slicer-runtime/tests/` that import `slicer_runtime::{HostCli, HostCommands, OutputFormat, SliceRunOptions}` or `slicer_runtime::helpers_cmd::*`. Tests of CLI parser shape that referenced `HostCli` are deleted (those types are gone); helper-command integration tests move to `crates/pnp-cli/tests/`.

## Out of Scope

- `crates/slicer-test/` or `crates/slicer-sdk/` — concurrent work (packet 78).
- WIT contract changes. None are needed.
- Moving `dag_cli.rs` — that is packet 85's territory. `slicer-runtime` continues to expose `run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims` and the matching output types through this packet.
- Extracting `report/` into a separate crate (`slicer-report`). The feature gate inside `slicer-runtime` is the chosen approach (see `design.md` §Selected Approach).
- Adding new `pnp_cli` subcommands or changing subcommand surface. The four helper commands keep their existing CLI shape; only their bodies move.
- Adding any new dep to `slicer-runtime` or removing any except via the `[features]` gate. Specifically: `clap` stays in `slicer-runtime`'s `[dependencies]` because `dag_cli.rs` still uses it.
- Renaming `pnp-cli` or restructuring its binary entry. The binary remains `pnp_cli` per CLAUDE.md §Post-Merge Naming.

## Authoritative Docs

- `docs/17_agent_debugging.md` — confirms that `pnp_cli dag <subcommand>` and `pnp_cli module diagnose` exist; helps disambiguate `dag_cli` (stays) from `helpers_cmd` (moves).
- `docs/16_slicer_report.md` — describes the HTML report format and the `--report <PATH>` opt-in flag. Confirms the gate must preserve current default behavior. ≤ 200 LOC typically — load directly only if needed; otherwise delegate SUMMARY.
- `CLAUDE.md` §"Post-Merge Naming" — confirms `slicer-cli` was deleted and `pnp_cli` is the canonical binary. Deletion of `slicer-runtime::cli` is the trailing edge of that history.
- `CLAUDE.md` §"Build & Test Commands" — confirms the slice invocation.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-9, AC-N1, AC-N2). Measurable refinements that did not fit Given/When/Then:

- **AC-7 — Per-subcommand SHA baselines**: capture pre-packet SHAs for `pnp_cli mesh convert/repair/decimate/import` outputs against canonical fixtures BEFORE moving. After the move, the same invocations produce SHA-identical outputs. Implementation log records both sets.
- **AC-4/AC-5 — Feature-gate compilation discipline**: the gate's correctness is asserted both by build success on `--no-default-features` (positive: AC-5) and by absence of stray references on default (AC-4 inspection). Both required.
- **AC-N2 — Manual ceremony**: a working-tree-only experiment that tries to call `slicer_runtime::report::render_html(...)` under `--no-default-features` and observes a compile error confirms the gate is load-bearing.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test ! -f crates/slicer-runtime/src/helpers_cmd.rs && find crates/pnp-cli/src -name '*.rs' \| xargs grep -lE 'pub fn (run_repair\|run_decimate\|run_import\|run_convert)' \| head -1 \| grep -q .` | FACT pass/fail |
| AC-2 | `test ! -f crates/slicer-runtime/src/cli.rs && ! grep -rqE 'struct HostCli\b\|enum HostCommands\b' crates/` | FACT pass/fail |
| AC-3 | `! grep -qE '^pub mod (cli\|helpers_cmd);' crates/slicer-runtime/src/lib.rs && grep -qE '^pub mod dag_cli;' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-4 | `grep -qE '^default *= *\["report"\]' crates/slicer-runtime/Cargo.toml && grep -qE '^#\[cfg\(feature = "report"\)\]$' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-5 | `cargo build --no-default-features -p slicer-runtime` | FACT pass/fail |
| AC-6 | `cargo build --workspace` | FACT pass/fail |
| AC-7 | `cargo run --bin pnp_cli --release -- mesh convert --input resources/benchy.stl --output /tmp/benchy-p82.obj --format obj && sha256sum /tmp/benchy-p82.obj` | SNIPPET (last line — SHA) |
| AC-8 | `cargo run --bin pnp_cli --release -- slice ... --report /tmp/p82-report.html && test -s /tmp/p82-report.html && head -5 /tmp/p82-report.html \| grep -qE '<!DOCTYPE html'` | FACT pass/fail |
| AC-9 | `cargo test -p slicer-runtime && cargo test -p pnp-cli` | FACT pass/fail + counts |
| AC-N1 | `rg -uu "use slicer_runtime::\{?[^}]*\b(HostCli\|HostCommands)\b" crates/` (success = empty) | FACT empty/non-empty |
| AC-N2 | Manual ceremony — `implementation-plan.md`. | (not CI) |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo build --no-default-features -p slicer-runtime` | FACT pass/fail |
| gate-3 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-4 | `cargo xtask build-guests --check` | FACT clean/STALE |

## Step Completion Expectations

- Pre-packet SHAs for the four `mesh *` subcommand outputs (AC-7) MUST be captured before any source edit. Without baselines, AC-7 cannot be falsified.
- The `report` feature gate edits MUST land together (Cargo.toml, lib.rs, run.rs, pnp-cli's argparse) so no intermediate commit leaves a half-gated state where `--no-default-features` build is broken.
- `helpers_cmd.rs` must use the P81 imports (`slicer_model_io::load_model`, `slicer_model_io::assemble_object`) when it moves; if the file still references `slicer_runtime::model_loader::...`, P81 was not closed properly and this packet must not start.

## Packet-Specific Context Discipline

- `helpers_cmd.rs` is 744 LOC. Do not load in full. Use grep + line-range hints to identify the four entry-point function signatures and their dispatcher wiring.
- `report/` consists of 5 files totalling 1 597 LOC. Do not load any of them in full. The feature gate is added at the `pub mod report;` declaration site in `lib.rs` and at every `crate::report::...` call site — those are the only edits needed.
- `OrcaSlicerDocumented/` is irrelevant. Do not consult it.
