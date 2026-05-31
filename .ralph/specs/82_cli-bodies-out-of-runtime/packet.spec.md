---
status: draft
packet: 82
task_ids: [TASK-232]
requires: [81]
backlog_source: docs/07_implementation_status.md
---

# Packet 82 — Move CLI Bodies Out of `slicer-runtime`; Feature-Gate `report/`

## Goal

Move `helpers_cmd.rs` (744 LOC; the `mesh convert/repair/decimate/import` command bodies) out of `slicer-runtime/src/` into `crates/pnp-cli/src/`; delete `cli.rs` (271 LOC; the dead `HostCli`/`HostCommands` clap-Parser types, retained "for parser-shape tests" per the source comment but no longer the entry point) while migrating its still-used `OutputFormat` enum and `write_with_parents` helper into `pnp-cli`; and put `slicer-runtime/src/report/` (1 597 LOC HTML rendering subsystem) behind a default-enabled `report` Cargo feature so that `cargo build --no-default-features -p slicer-runtime` builds the runtime without the reporting subsystem compiled.

## Scope Boundaries

This packet is hygiene + an opt-out seam. `helpers_cmd.rs` and `cli.rs` are CLI presentation code that does not belong in a host library crate; their move into `pnp-cli` closes that inversion. `report/` stays in `slicer-runtime` (not extracted to a separate crate — that was considered and rejected as not depth-deepening) but becomes optional. `dag_cli.rs` stays in `slicer-runtime` until packet 85 (it moves to `slicer-scheduler`, not `pnp-cli`). The 8 pub items that `lib.rs` re-exports from these three modules (`OutputFormat`, `write_with_parents`, `HostCli`, `HostCommands`, `SliceRunOptions`, plus the `report` re-exports) get rewired or deleted per `requirements.md` §In Scope.

## Prerequisites and Blockers

- **Requires packet 81 closed**. `helpers_cmd.rs` imports `slicer_model_io::{assemble_object, load_model}` (rewired in P81 step 3); the file move in this packet preserves those imports.
- Closure requires `cargo xtask build-guests --check` clean. This packet edits no guest-feeding paths.

## Acceptance Criteria

### AC-1 — `helpers_cmd.rs` no longer exists under `slicer-runtime/src/`; equivalent commands compile inside `pnp-cli`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/helpers_cmd.rs` is true; `crates/pnp-cli/src/helpers_cmd.rs` exists (or its content is split across files under `crates/pnp-cli/src/commands/`) and exposes `pub fn` entries for `run_repair`, `run_decimate`, `run_import`, `run_convert` (or whatever the pnp-cli subcommand dispatcher calls). The relocated file imports from `slicer_model_io::` and from `slicer_runtime::` only — NOT from `slicer_runtime::helpers_cmd::*` (which no longer exists).

| `test ! -f crates/slicer-runtime/src/helpers_cmd.rs && find crates/pnp-cli/src -name '*.rs' | xargs grep -lE 'pub fn (run_repair\|run_decimate\|run_import\|run_convert)' | head -1 | grep -q . && ! find crates/pnp-cli/src -name '*.rs' | xargs grep -qE 'use slicer_runtime::helpers_cmd'`

### AC-2 — `cli.rs` no longer exists under `slicer-runtime/src/`; its dead types are deleted, its live items are in `pnp-cli`

**Given** the deletion,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/cli.rs` is true. `HostCli` and `HostCommands` (the dead clap-Parser types) no longer appear anywhere in the workspace. `OutputFormat` (the value enum used by `mesh repair/decimate/import`) is defined inside `crates/pnp-cli/src/` and reachable from the pnp-cli subcommand bodies. `write_with_parents` is defined inside `crates/pnp-cli/src/` and used by the slice and report output writers.

| `test ! -f crates/slicer-runtime/src/cli.rs && ! grep -rqE 'struct HostCli\b\|enum HostCommands\b' crates/ && find crates/pnp-cli/src -name '*.rs' | xargs grep -lE 'pub enum OutputFormat\b\|pub fn write_with_parents\b' | head -1 | grep -q .`

### AC-3 — `slicer-runtime/src/lib.rs` no longer declares `pub mod cli;` or `pub mod helpers_cmd;`; their re-exports are gone

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** none of these lines exist: `pub mod cli;`, `pub mod helpers_cmd;`, `pub use cli::{...};`, `pub use helpers_cmd::...`. The `pub mod dag_cli;` declaration and its `pub use dag_cli::...` block ARE preserved (dag_cli stays in slicer-runtime through P82; it moves in P85). Other `pub mod` declarations in the file are unchanged.

| `! grep -qE '^pub mod (cli\|helpers_cmd);' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use cli::' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use helpers_cmd::' crates/slicer-runtime/src/lib.rs && grep -qE '^pub mod dag_cli;' crates/slicer-runtime/src/lib.rs`

### AC-4 — `report/` is feature-gated behind `report` (default-enabled), and `lib.rs` + `run.rs` use `#[cfg(feature = "report")]` consistently

**Given** the feature-gate,
**When** `crates/slicer-runtime/Cargo.toml` and `crates/slicer-runtime/src/lib.rs` are inspected,
**Then** `Cargo.toml` has a `[features]` section with `default = ["report"]` and `report = []`. `lib.rs` declares the report module as `#[cfg(feature = "report")] pub mod report;` and its `pub use report::...` re-exports are also gated. `run.rs`'s `report::Collector` / `report::allocator` usages are wrapped in `#[cfg(feature = "report")]` blocks; the `--report` argparse flag in pnp-cli's slice subcommand is similarly gated OR the absence-of-report branch handles "report flag passed but feature off" with a clear error.

| `grep -qE '^default *= *\["report"\]' crates/slicer-runtime/Cargo.toml && grep -qE '^report *= *\[\]' crates/slicer-runtime/Cargo.toml && grep -qE '^#\[cfg\(feature = "report"\)\]$' crates/slicer-runtime/src/lib.rs && grep -qE '#\[cfg\(feature = "report"\)\]' crates/slicer-runtime/src/run.rs`

### AC-5 — `cargo build --no-default-features -p slicer-runtime` succeeds (the feature gate compiles cleanly with `report` off)

**Given** the gate,
**When** `cargo build --no-default-features -p slicer-runtime` runs from a clean target,
**Then** the build succeeds (exit 0). No errors about missing `report::*` symbols, no errors about `Collector` not being in scope. The resulting `slicer-runtime` artifact compiles without the `report/` subtree.

| `cargo build --no-default-features -p slicer-runtime`

### AC-6 — `cargo build --workspace` succeeds with default features intact (report enabled, behavior identical)

**Given** the default build,
**When** `cargo build --workspace` runs,
**Then** the build succeeds. With `report` in `default`, the `pnp_cli slice --report <PATH>` flow produces the same HTML output it did before the packet (byte-identical or render-identical — assertion below).

| `cargo build --workspace`

### AC-7 — `pnp_cli mesh repair/decimate/import/convert` subcommands still work end-to-end

**Given** the move,
**When** each subcommand runs against a fixture under `resources/` (or a `tests/` fixture),
**Then** each completes successfully and produces the same output it did before the move. The implementation log records: per-subcommand exit code, output file size, output SHA. SHAs match the pre-packet baseline captured before the move.

| `cargo run --bin pnp_cli --release -- mesh convert --input resources/benchy.stl --output /tmp/benchy-p82.obj --format obj`

### AC-8 — `pnp_cli slice --report <PATH>` still produces a valid HTML report on default features

**Given** `report` in default features and the gating in place,
**When** `pnp_cli slice ... --report /tmp/p82-report.html` runs,
**Then** the report file is created, is non-empty, contains the documented sentinel strings (e.g., the HTML doctype and the title bar containing the slicer banner — exact pattern in `docs/16_slicer_report.md`). The implementation log captures the file size and the first 5 lines.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p82.gcode --report /tmp/p82-report.html && test -s /tmp/p82-report.html && head -5 /tmp/p82-report.html | grep -qE '<!DOCTYPE html'`

### AC-9 — `cargo test -p slicer-runtime -p pnp-cli` pass

**Given** the moves + the feature gate,
**When** the narrow per-crate tests run,
**Then** all tests pass with zero regressions vs the pre-packet count. Any test in `crates/slicer-runtime/tests/` that imported `slicer_runtime::{HostCli, HostCommands}` either fails to find them (and is deleted because it tested dead types) or rewires to `pnp-cli`'s parser via a new dev-dep.

| `cargo test -p slicer-runtime && cargo test -p pnp-cli`

## Negative Test Cases

### AC-N1 — No `use slicer_runtime::{HostCli, HostCommands};` import remains in the workspace

**Given** the deletion,
**When** the workspace is grepped,
**Then** the result is empty. This is the structural signal that the dead types are gone for good.

| `! rg -uu "use slicer_runtime::\{?[^}]*\b(HostCli\|HostCommands)\b" crates/ 2>/dev/null`

### AC-N2 — `cargo build --no-default-features -p slicer-runtime` does NOT compile `report/`

**Given** the feature gate,
**When** the no-default-features build runs with `RUSTFLAGS='--cfg report_dump'` (or by inspecting the artifact),
**Then** no symbol from `slicer_runtime::report` is reachable. A test binary trying to call `slicer_runtime::report::render_html(...)` under `--no-default-features` fails to compile. Documented in `implementation-plan.md` step "Verify the feature gate excludes report symbols".

| `! cargo build --no-default-features -p slicer-runtime 2>&1 | grep -qE 'unresolved import.*slicer_runtime::report'` (success = build green with NO unresolved-import errors — the gate keeps the references out of compilation entirely)

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo build --no-default-features -p slicer-runtime`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p slicer-runtime -p pnp-cli`
5. `cargo xtask build-guests --check` (must stay clean — this packet edits no guest-feeding path)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/17_agent_debugging.md` — describes the `pnp_cli dag <subcommand>` and `pnp_cli module diagnose` debugging commands. Helps frame what stays in `slicer-runtime::dag_cli` (everything) vs what leaves (`helpers_cmd.rs`). No change.
- `docs/16_slicer_report.md` — describes the HTML report format and allocator contract. Read to confirm the report-feature gate preserves the documented behavior on default builds. No change.
- `CLAUDE.md` §"Post-Merge Naming" — confirms `slicer-cli` is gone and `pnp_cli` is the only binary name; the deletion of `slicer-runtime::cli` aligns with that history.
- `CLAUDE.md` §"Build & Test Commands" — confirms `cargo run --bin pnp_cli --release -- slice ...` still matches after the move.

## Doc Impact Statement

No doc files are edited by this packet. `docs/16_slicer_report.md` does NOT need a note about the feature gate because the gate is invisible to users on default builds (which is the only documented configuration). If the user ever adopts `--no-default-features` for binary distribution, a one-line note about the absent `--report` flag would land in that doc; that is out of scope.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
