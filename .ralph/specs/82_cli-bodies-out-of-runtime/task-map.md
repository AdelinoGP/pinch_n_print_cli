# Task Map — Packet 82

This packet spans **1 task ID** in `docs/07_implementation_status.md`: **TASK-232**.

> Numbering note: TASK-232 was reserved for this packet during packet 81's docs/07 sync (see `.ralph/specs/81_slicer-model-io/task-map.md`). TASK-233 is held by packet 81; TASK-234..236 will be claimed by packets 83..85 as they activate.

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-232 | Steps 0, 1, 2, 3, 4, 5, 6, 7 | Move `crates/slicer-runtime/src/helpers_cmd.rs` (744 LOC) into `crates/pnp-cli/src/`; delete `crates/slicer-runtime/src/cli.rs` (271 LOC), relocating its still-used `OutputFormat` enum and `write_with_parents` helper into `pnp-cli` while discarding the dead `HostCli`/`HostCommands` clap-Parser types; put `crates/slicer-runtime/src/report/` (1 597 LOC) behind a default-enabled `report` Cargo feature so `cargo build --no-default-features -p slicer-runtime` compiles the runtime without the report subsystem; preserve SHA-identical output for the four `pnp_cli mesh *` subcommands against pre-packet baselines; preserve byte-identical `--report` HTML output on default builds; add a one-line note to `docs/16_slicer_report.md` documenting the new `report` Cargo feature. |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-232 | `crates/slicer-runtime/src/helpers_cmd.rs` lines 1–80 (imports + first signature) and 363+ (the four `pub fn run_*` entry points). `crates/slicer-runtime/src/cli.rs` (271 LOC — load in full at Step 3) for `OutputFormat`, `write_with_parents`, `HostCli`, `HostCommands`, `SliceRunOptions`. `crates/slicer-runtime/src/lib.rs` for `pub mod` and `pub use` edits. `crates/slicer-runtime/src/run.rs` for the `report::Collector` / `report_alloc::*` gate sites (enumerated by Step 1 dispatch #3). `crates/pnp-cli/src/main.rs` for the slice subcommand argparse and the four `mesh *` subcommand dispatcher arms. `crates/slicer-runtime/Cargo.toml` and `crates/pnp-cli/Cargo.toml` for the `[features]` blocks. `docs/16_slicer_report.md` — read only at Step 6 to confirm sentinel strings AND to land the one-line `report`-feature note (DIS deliverable). `docs/17_agent_debugging.md` — read only; confirms `pnp_cli dag <subcommand>` stays in `slicer-runtime::dag_cli` (out of scope, P85). `CLAUDE.md` §"Post-Merge Naming" — confirms the `slicer-cli` → `pnp_cli` history; this packet is the trailing edge of that consolidation. `CLAUDE.md` §"Build & Test Commands" — confirms `cargo run --bin pnp_cli --release -- slice ...` remains the canonical invocation after the move. |

## OrcaSlicer references

None. None of the moved code (`helpers_cmd.rs`, `cli.rs`) or the gated code (`report/`) ports OrcaSlicer behavior. `OrcaSlicerDocumented/` is explicitly out of scope per `design.md` §"Out-of-Bounds Files" and `requirements.md` §"Packet-Specific Context Discipline".

## Predecessor / successor relationships

- **Predecessors**:
  - **Packet 81** (`.ralph/specs/81_slicer-model-io/`, TASK-233) — hard dependency at the Step-3 boundary. P81 carves `slicer-model-io` out of `slicer-runtime` and rewires `helpers_cmd.rs` to import `slicer_model_io::{assemble_object, load_model}` in place. P82 preserves those imports unchanged when it moves the file. P82 does NOT require P81 to be fully `status: superseded` — only that P81's Step 3 has landed (the deepening batch P81–P88 is allowed to overlap). Step 0 dispatch A verifies the prerequisite point: `slicer-model-io` crate exists AND `helpers_cmd.rs` imports from `slicer_model_io::` AND no `slicer_runtime::model_loader::*` paths remain.
- **Successors**:
  - **Packet 83** (planned, TASK-234) — restructures `slicer-runtime`'s wasm-instance and dispatch internals. Reads the post-P82 `lib.rs` (with `cli`, `helpers_cmd` gone and `report` gated) as its starting point. Soft dependency only — P83's surface is `wit_host.rs`/`dispatch.rs`/`wasm_instance.rs`/`instance_pool.rs`, explicitly listed as out-of-bounds in P82's `design.md`.
  - **Packet 85** (planned, TASK-236) — moves `dag_cli.rs` from `slicer-runtime` to `slicer-scheduler`. P82 explicitly preserves `dag_cli.rs` and its `pub use` re-exports (`run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims`) so P85 has a clean source-of-truth to relocate. Soft dependency.

## Backlog sync status

TASK-232 is listed in `docs/07_implementation_status.md` under "Architecture Deepening Phase I" alongside TASK-231 (= P-no-longer-this-one, reassigned during P81 activation) and TASK-233 (P81). Its `[ ]` → `[x]` flip with the `Closed <date> — packet 82` suffix lands at this packet's Acceptance Ceremony, after the closure gate's eight checks all pass.

## End-state of packet 82

At packet 82's closure:

- `crates/slicer-runtime/src/helpers_cmd.rs` no longer exists. Its content (the four `run_repair`, `run_decimate`, `run_import`, `run_convert` entry-point functions plus their helpers) lives under `crates/pnp-cli/src/` (file layout flexible — single `helpers_cmd.rs` or a `commands/` subtree). The functions import from `slicer_model_io::` and `slicer_runtime::` only, never from `slicer_runtime::helpers_cmd::*`.
- `crates/slicer-runtime/src/cli.rs` no longer exists. `OutputFormat` and `write_with_parents` are reachable from `pnp-cli`'s subcommand dispatcher. `HostCli`, `HostCommands` are deleted from the workspace (no `use slicer_runtime::{HostCli, HostCommands}` import remains anywhere — AC-N1). `SliceRunOptions` is either deleted (if no consumer outside `cli.rs`) or relocated to `slicer-runtime/src/run.rs` (if `run.rs` consumes it — Step 1 dispatch #1 decides).
- `crates/slicer-runtime/src/lib.rs` no longer declares `pub mod cli;` or `pub mod helpers_cmd;` and their `pub use` re-exports are gone. `pub mod dag_cli;` and its re-exports are preserved (P85 territory). `pub mod report;` and its `pub use report::...` block are gated with `#[cfg(feature = "report")]`.
- `crates/slicer-runtime/Cargo.toml` has `[features] default = ["report"] report = []`. No deps added or removed; `clap` stays (used by `dag_cli.rs`).
- `crates/pnp-cli/Cargo.toml` has `[features] default = ["report"] report = ["slicer-runtime/report"]` so the binary's `--report` handler compiles in lockstep with the lib.
- `crates/slicer-runtime/src/run.rs`'s `report::Collector` / `report_alloc::*` usages are wrapped in `#[cfg(feature = "report")]` blocks (sites enumerated by Step 1 dispatch #3).
- `pnp_cli mesh convert/repair/decimate/import` produce SHA-identical output to the Step 0 baselines against the canonical fixtures listed in Step 0.
- `pnp_cli slice --report <PATH>` on default builds produces an HTML file structurally identical to pre-packet (passes the `<!DOCTYPE html` and file-size sentinels — AC-8).
- `cargo build --no-default-features -p slicer-runtime` is green (AC-5). A probe `use slicer_runtime::report::Collector;` under `--no-default-features` fails to compile with `unresolved import` (AC-N2, Step 7).
- `docs/16_slicer_report.md` gains one sentence describing the `report` Cargo feature and the opt-out path via `cargo build --no-default-features -p slicer-runtime`.
- `cargo test -p slicer-runtime -p pnp-cli` is green; any test-count delta against the Step 0 baseline matches the documented migrate/delete log (Step 4).
- `docs/07_implementation_status.md` flips TASK-232 from `[ ]` to `[x]` with the `Closed <date> — packet 82` suffix.
