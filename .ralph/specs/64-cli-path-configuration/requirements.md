# Requirements: 64-cli-path-configuration

## Packet Metadata

- Grouped task IDs:
  - `TASK-204` — Normalize `String` CLI arg types to `PathBuf`
  - `TASK-205` — Complete `HostRunOptions`, delete `validate_run_options` and `CliError`
  - `TASK-206` — Remove dead `--module` flag
  - `TASK-207` — Create parent directories for `--output` and `--report` paths before write
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The `slicer-host` CLI has accumulated several path-configuration inconsistencies during its evolution from a single-module runner to a multi-root module discovery system:

1. **Dead validation code.** `validate_run_options` (`cli.rs:125-176`) performs file-existence checks for `--module`, `--model`, `--config`, and `--module-dir` but is **never called from `main.rs`**. The main binary validates paths inline (via `load_model`, `read_to_string`, and `load_live_modules_for_plan`). The function and `CliError` enum exist only as exported library API surface consumed by test code.

2. **Incomplete `HostRunOptions`.** The struct is supposed to be the validated runtime options object but lacks `thumbnail`, `report`, and `report_verbose` — all of which are path-bearing CLI args handled ad-hoc in `main.rs`.

3. **Silently ignored `--module` flag.** The `--module` flag is parsed by clap but bound to `_` in `main.rs:122`, making it a no-op that misleads users.

4. **Missing parent-directory creation.** `--output` and `--report` file writes fail cryptically when the parent directory does not exist.

5. **Inconsistent arg types.** Four CLI args use `String` (`module`, `model`, `config`, `output`) while three use `PathBuf` (`module_dir`, `thumbnail`, `report`).

These are not individually severe bugs, but collectively they represent API surface drift that wastes developer attention and erodes trust in the CLI contract. This packet is the smallest coherent remediation slice that closes all five gaps.

## In Scope

- Add `thumbnail: Option<PathBuf>`, `report: Option<PathBuf>`, `report_verbose: bool` fields to `HostRunOptions`
- Delete `validate_run_options` function and `CliError` enum from `cli.rs`
- Remove `validate_run_options` and `CliError` from `lib.rs` public re-exports
- Delete 3 dead-code tests (`validate_run_options_missing_model`, `validate_run_options_missing_module`, `validate_run_options_valid`) from `cli_tdd.rs`
- Construct `HostRunOptions` directly from CLI args in `main.rs` with inline existence checks
- Refactor `main.rs` pipeline setup to read from `HostRunOptions` fields instead of raw CLI bindings
- Remove `--module` flag from `Run` CLI arg in `cli.rs`
- Remove `module_path` field from `HostRunOptions`
- Remove `module: _` from match destructure in `main.rs`
- Update CLI tests in `cli_tdd.rs` to remove `--module` from test invocations and use `PathBuf` literals
- Add `fs::create_dir_all` call before `std::fs::write` for `--output` path in `main.rs`
- Add `fs::create_dir_all` call before `std::fs::write` in `collector.rs::finish_and_render_to`
- Change `model: String` → `model: PathBuf` in `Run` variant
- Change `config: Option<String>` → `config: Option<PathBuf>` in `Run` variant
- Change `output: Option<String>` → `output: Option<PathBuf>` in `Run` variant
- Update `main.rs` to use `PathBuf`-typed args directly (remove `Path::new()` wrappers)

## Out of Scope

- No IR schema changes (`docs/02_ir_schemas.md` untouched)
- No WIT boundary changes (`wit/` directory untouched)
- No module manifest changes (`modules/core-modules/` untouched)
- No WASM compilation or guest code changes
- No OrcaSlicer parity work
- No documentation changes to `docs/01`-`docs/16` (all behavior is internal; `--module` removal warrants no doc update because it was never documented as a supported flag in architecture docs)
- No changes to module search path logic (`module_search_path.rs`)
- No changes to pipeline orchestration beyond the entry-point wiring
- No addition of `--module` deprecation warning (removed immediately per user decision)

## Authoritative Docs

- `crates/slicer-host/src/cli.rs` — 176 lines; read directly
- `crates/slicer-host/src/main.rs` — 483 lines; read directly (skip lines 372-483 `_stale_build_plan`)
- `crates/slicer-host/tests/cli_tdd.rs` — 204 lines; read directly
- `crates/slicer-host/src/lib.rs` — 134 lines; read directly (line 47 only: the public re-export line)
- `crates/slicer-host/src/report/collector.rs` — lines 232-236 only; range-read
- `docs/specs/default-builder-migration.md` — lines 990-995 and 1405-1410 only; delegate FACT for `HostRunOptions` intent

## Acceptance Summary

Reference criteria by ID from `packet.spec.md`; do not copy them.

- **AC-1** (HostRunOptions completeness): verifies the three missing fields are present. `rg` against `cli.rs`.
- **AC-2** (dead code removal): verifies `validate_run_options` and `CliError` are gone from production source. `rg` against `src/`.
- **AC-3** (--module removal): verifies the flag no longer appears in help text.
- **AC-4** (String→PathBuf): verifies the type change compiled correctly. `rg` for `PathBuf` on the three args.
- **AC-5** (parent-dir creation): verifies `--output` works when parent dir is absent. New dedicated test.
- **AC-6** (HostRunOptions wired in): verifies `validate_run_options` is not referenced in `main.rs`.
- **AC-N1** (--module rejection): verifies clap rejects the removed flag.

Refinements:
- AC-5 parent-directory test must create a temp directory, derive a nested path one level deeper (e.g., `{tmp}/newdir/out.gcode`), run the pipeline (or a minimal harness), and assert the file exists after.
- AC-6 must additionally verify that the 4 test files in `cli_tdd.rs` do not reference `validate_run_options`, `CliError`, or `--module`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'thumbnail: Option<PathBuf>' crates/slicer-host/src/cli.rs && echo PASS \|\| echo FAIL` | AC-1: HostRunOptions has thumbnail | FACT |
| `rg -c 'validate_run_options\|CliError' crates/slicer-host/src/ \| rg '^\$' -q && echo PASS \|\| echo FAIL` | AC-2: dead code removed from src/ | FACT |
| `cargo run --bin slicer-host -- run --help 2>&1 \| rg -q -- '--module' && echo FAIL \|\| echo PASS` | AC-3: --module removed from help | FACT |
| `rg -q 'model(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'config(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'output(:.*)? PathBuf' crates/slicer-host/src/cli.rs && echo PASS \|\| echo FAIL` | AC-4: String→PathBuf | FACT |
| `cargo test -p slicer-host --test cli_tdd -- output_path_creates_parent_dir --nocapture` | AC-5: parent-dir creation | FACT pass/fail; SNIPPETS assertion on failure |
| `rg -c 'validate_run_options' crates/slicer-host/src/main.rs \| rg '^0$' -q && echo PASS \|\| echo FAIL` | AC-6: main.rs doesn't call validate_run_options | FACT |
| `cargo run --bin slicer-host -- run --module /tmp/mod.wasm --model /tmp/model.stl 2>&1; if ($LASTEXITCODE -ne 0) { echo PASS } else { echo FAIL }` | AC-N1: --module rejected | FACT |
| `rg -c 'validate_run_options\|CliError\|--module' crates/slicer-host/tests/ \| rg '^0$' -q && echo PASS \|\| echo FAIL` | Cross-step: no test references to deleted symbols | FACT |
| `cargo check --workspace` | Workspace build | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint | FACT pass/fail |
| `cargo test -p slicer-host --test cli_tdd` | CLI test suite | FACT pass/fail; SNIPPETS on failure |
| `cargo test -p slicer-host --test module_search_path_tdd` | Unrelated regression guard | FACT pass/fail |

## Step Completion Expectations

None. All per-step preconditions and postconditions are captured in `implementation-plan.md`. Steps are strictly sequential (each depends on the previous step's type changes).

## Context Discipline Notes

**Large files in the read-only path:**
- `crates/slicer-host/src/main.rs` (483 lines) — skip the dead `_stale_build_plan` block at lines 372-483 entirely. Only the `Run` match arm (lines 121-352) and `ConfigSchema` arm (lines 354-369) matter.

**Likely temptation reads:**
- `crates/slicer-host/src/module_search_path.rs` — not touched by this packet. Skip.
- Any pipeline or dispatch file — not touched. Skip.
- `docs/` — no doc changes needed. Skip.

**Sub-agent return-format hints:**
- All verification commands return `FACT pass/fail`. The heaviest is the parent-dir test (`cli_tdd -- output_path_creates_parent_dir`), which is a single test — its output is ≤ 10 lines on failure.
