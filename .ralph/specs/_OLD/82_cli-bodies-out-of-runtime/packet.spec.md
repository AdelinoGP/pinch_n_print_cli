---
status: implemented
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

- **Requires packet 81 Step 3 complete** (`slicer-model-io` carved out as a leaf crate and `crates/slicer-runtime/src/helpers_cmd.rs` rewired to `slicer_model_io::{assemble_object, load_model}`). P81 need not have flipped to `status: superseded` — the deepening batch (P81–P88) is allowed to overlap. The file move in this packet preserves the P81 imports unchanged. Step 0 verifies the prerequisite point via a single grep; if `helpers_cmd.rs` still references `slicer_runtime::model_loader::...`, P81 has not reached the prerequisite point and this packet must not start.
- Closure requires `cargo xtask build-guests --check` clean. This packet edits no guest-feeding paths.

## Acceptance Criteria

### AC-1 — `helpers_cmd.rs` no longer exists under `slicer-runtime/src/`; equivalent commands compile inside `pnp-cli`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/helpers_cmd.rs` is true; `crates/pnp-cli/src/helpers_cmd.rs` exists (or its content is split across files under `crates/pnp-cli/src/commands/`) and exposes `pub fn` entries for `run_repair`, `run_decimate`, `run_import`, `run_convert` (or whatever the pnp-cli subcommand dispatcher calls). The relocated file imports from `slicer_model_io::` and from `slicer_runtime::` only — NOT from `slicer_runtime::helpers_cmd::*` (which no longer exists).

| `test ! -f crates/slicer-runtime/src/helpers_cmd.rs && find crates/pnp-cli/src -name '*.rs' | xargs grep -lE 'pub fn (run_repair|run_decimate|run_import|run_convert)' | head -1 | grep -q . && ! find crates/pnp-cli/src -name '*.rs' | xargs grep -qE 'use slicer_runtime::helpers_cmd'`

### AC-2 — `cli.rs` no longer exists under `slicer-runtime/src/`; its dead types are deleted, its live items are in `pnp-cli`

**Given** the deletion,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/cli.rs` is true. `HostCli` and `HostCommands` (the dead clap-Parser types) no longer appear anywhere in the workspace. `OutputFormat` (the value enum used by `mesh repair/decimate/import`) is defined inside `crates/pnp-cli/src/` and reachable from the pnp-cli subcommand bodies. `write_with_parents` is defined inside `crates/pnp-cli/src/` and used by the slice and report output writers.

| `test ! -f crates/slicer-runtime/src/cli.rs && ! grep -rqE 'struct HostCli\b|enum HostCommands\b' crates/ && find crates/pnp-cli/src -name '*.rs' | xargs grep -lE 'pub enum OutputFormat\b|pub fn write_with_parents\b' | head -1 | grep -q .`

### AC-3 — `slicer-runtime/src/lib.rs` no longer declares `pub mod cli;` or `pub mod helpers_cmd;`; their re-exports are gone

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** none of these lines exist: `pub mod cli;`, `pub mod helpers_cmd;`, `pub use cli::{...};`, `pub use helpers_cmd::...`. The `pub mod dag_cli;` declaration and its `pub use dag_cli::...` block ARE preserved (dag_cli stays in slicer-runtime through P82; it moves in P85). Other `pub mod` declarations in the file are unchanged.

| `! grep -qE '^pub mod (cli|helpers_cmd);' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use cli::' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use helpers_cmd::' crates/slicer-runtime/src/lib.rs && grep -qE '^pub mod dag_cli;' crates/slicer-runtime/src/lib.rs`

### AC-4 — `report/` is feature-gated behind `report` (default-enabled), and `lib.rs` + `run.rs` use `#[cfg(feature = "report")]` consistently

**Given** the feature-gate,
**When** `crates/slicer-runtime/Cargo.toml` and `crates/slicer-runtime/src/lib.rs` are inspected,
**Then** `Cargo.toml` has a `[features]` section with `default = ["report"]` and `report = []`. `lib.rs` declares the report module as `#[cfg(feature = "report")] pub mod report;` and its `pub use report::...` re-exports are also gated. `run.rs`'s `report::Collector` / `report::allocator` usages are wrapped in `#[cfg(feature = "report")]` blocks; the `--report` argparse flag in pnp-cli's slice subcommand is similarly gated OR the absence-of-report branch handles "report flag passed but feature off" with a clear error.

| `grep -qE '^default *= *\["report"\]' crates/slicer-runtime/Cargo.toml && grep -qE '^report *= *\[\]' crates/slicer-runtime/Cargo.toml && grep -qE '#\[cfg\(feature = "report"\)\]' crates/slicer-runtime/src/lib.rs && grep -qE '#\[cfg\(feature = "report"\)\]' crates/slicer-runtime/src/run.rs`

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
**When** each of the four subcommands runs against its canonical fixture (enumerated in `implementation-plan.md` Step 0),
**Then** each completes with exit 0 and produces SHA-identical output to the pre-packet baseline captured in Step 0. The implementation log records, per subcommand: exit code, output file size, and SHA-256. All four SHAs match Step 0 baselines.

The inline smoke check below exercises one subcommand (`mesh convert`) end-to-end; the full four-subcommand SHA-parity matrix is run as `implementation-plan.md` Step 5, which is a closure gate.

| `cargo run --bin pnp_cli --release -- mesh convert --input resources/benchy.stl --output /tmp/benchy-p82.obj --output-format obj && test -s /tmp/benchy-p82.obj` (smoke — full parity matrix in Step 5)

### AC-8 — `pnp_cli slice --report <PATH>` still produces a valid HTML report on default features

**Given** `report` in default features and the gating in place,
**When** `pnp_cli slice ... --report /tmp/p82-report.html` runs,
**Then** the report file is created, is non-empty, contains the documented sentinel strings (e.g., the HTML doctype and the title bar containing the slicer banner — exact pattern in `docs/16_slicer_report.md`). The implementation log captures the file size and the first 5 lines.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p82.gcode --report /tmp/p82-report.html && test -s /tmp/p82-report.html && head -5 /tmp/p82-report.html | grep -qiE '<!doctype html'`

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

| `! rg -uu "\b(HostCli|HostCommands)\b" crates/ 2>/dev/null`

### AC-N2 — Under `--no-default-features`, the `slicer_runtime::report` module is unreachable from downstream code

**Given** the feature gate,
**When** a probe test file containing `use slicer_runtime::report::Collector;` is added to `crates/slicer-runtime/tests/` and `cargo build --no-default-features -p slicer-runtime --tests` runs,
**Then** the build MUST fail with an `unresolved import \`slicer_runtime::report\`` error (or equivalent E0432). This proves the gate excludes the subtree from compilation entirely rather than merely hiding the re-export. The probe file is removed after the check; the experiment is git-stash-friendly.

| Ceremony — see `implementation-plan.md` Step 7. Success = (a) `cargo build --no-default-features -p slicer-runtime --tests` exits non-zero with the probe file present AND its stderr contains `unresolved import` referencing `slicer_runtime::report`, AND (b) after probe removal, `cargo build --no-default-features -p slicer-runtime` is green again. (not CI)

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

One doc edit. The new `report` Cargo feature is a user-observable build seam that future agents and operators may discover and need to reason about. Although the gate is invisible on default builds, an undocumented feature flag is the kind of thing that gets "discovered" through grep-spelunking and then mis-described in subsequent packets.

- `docs/16_slicer_report.md` — add a one-line note in the introduction or a new "Build configuration" sub-section: "The report subsystem is feature-gated behind the default-enabled `report` Cargo feature on `slicer-runtime`. Build with `cargo build --no-default-features -p slicer-runtime` to omit it; the `--report` flag is then absent from `pnp_cli slice`."
  - Verification grep: `grep -qE 'no-default-features.*slicer-runtime|report.*Cargo feature' docs/16_slicer_report.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- **D-1 (AC-7 SHA parity, partial)** — Specified: all 4 `pnp_cli mesh *` subcommand outputs produce SHA-identical bytes vs the Step 0 baseline. Implemented: 3 of 4 match exactly (convert, decimate, import). `mesh repair` produced a different SHA on every run (observed `616c97b7…` and `fdea6888…` across two consecutive post-packet runs; baseline `a128e80b…`). Reason: pre-existing non-determinism inside the repair pipeline (`slicer-helpers` / `slicer-model-io`), code untouched by P82. AC-7's byte-determinism assumption does not hold for `mesh repair`; filed for separate investigation against the repair pipeline owner.
- **D-2 (AC-8 grep casing)** — Specified: `head -5 /tmp/p82-report.html | grep -qE '<!DOCTYPE html'` (case-sensitive uppercase). Implemented: rendered output is `<!doctype html>…` (lowercase HTML5 form, unchanged by P82). Verified via case-insensitive grep; file is 118 KB, valid HTML, contains the documented `<title>Slicer Report</title>` sentinel. AC-8 grep predicate amended inline (`-qE` → `-qiE`, pattern lowercased) to match the actual renderer output.
