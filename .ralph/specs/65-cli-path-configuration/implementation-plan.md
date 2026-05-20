# Implementation Plan: 65-cli-path-configuration

## Execution Rules

- One atomic step at a time. TDD: read the existing test, understand assertion, make the smallest change needed to pass, then validate.
- Each step must map back to one or more task IDs.
- After every step, run `cargo check --workspace` (via sub-agent, FACT return) before moving on. Fix any type errors before verifying tests.
- After all steps, run `cargo clippy --workspace -- -D warnings` and the narrowest test suite.
- Context discipline: read only the files listed in each step's "Files allowed to read" section. Delegate verification commands via sub-agent; do not absorb command output into context.

## Steps

### Step 1: Normalize CLI arg types from `String` to `PathBuf`

- Task IDs:
  - `TASK-204`
- Objective: Change `model`, `config`, `output` CLI args from `String`/`Option<String>` to `PathBuf`/`Option<PathBuf>` in the `Run` variant of `HostCommands`.
- Precondition: The current types are `model: String`, `config: Option<String>`, `output: Option<String>`.
- Postcondition: All three use `PathBuf` types. The `module` arg is not touched in this step (removed in Step 3).
- Files allowed to read:
  - `crates/slicer-host/src/cli.rs` — lines 20-56 (Run variant fields) and 71-84 (HostRunOptions)
  - `crates/slicer-host/src/main.rs` — lines 121-168 (Run arm destructure and config handling)
  - `crates/slicer-host/tests/cli_tdd.rs` — lines 24-61 (run_parses_all_flags test), 64-93 (run_optional_config_and_output), 133-204 (validate_run_options tests)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/cli.rs` — change types of `model`, `config`, `output` fields in the `Run` variant
  - `crates/slicer-host/src/main.rs` — update references to these fields: remove `Path::new(&model)` wrappers, remove `PathBuf::from(config)` / `PathBuf::from(output)` calls, use the PathBuf values directly
  - `crates/slicer-host/tests/cli_tdd.rs` — update test arg assertions to use `PathBuf::from(...)` instead of string equality
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/report/collector.rs` — not yet
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail" — validate type changes compile
- Context cost: **S**
- Authoritative docs:
  - Read `cli.rs:20-56` and `cli.rs:71-84` directly (both ≤ 40 lines each)
- OrcaSlicer refs: None.
- Verification:
  - `rg -q 'model(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'config(:.*)? PathBuf' crates/slicer-host/src/cli.rs && rg -q 'output(:.*)? PathBuf' crates/slicer-host/src/cli.rs && echo PASS || echo FAIL`
  - `cargo check --workspace` (delegated)
- Exit condition: `cargo check --workspace` passes; the rg check returns PASS.

### Step 2: Complete `HostRunOptions`, delete dead `validate_run_options` and `CliError`

- Task IDs:
  - `TASK-205`
- Objective: Add missing fields to `HostRunOptions`, delete `validate_run_options` function and `CliError` enum, remove re-exports from `lib.rs`, construct `HostRunOptions` directly in `main.rs`.
- Precondition: Step 1 complete (PathBuf types in place).
- Postcondition: `HostRunOptions` has `thumbnail`, `report`, `report_verbose`. `validate_run_options` and `CliError` do not exist in `src/`. `main.rs` constructs `HostRunOptions` directly from CLI args with inline existence checks. `lib.rs` re-exports updated.
- Files allowed to read:
  - `crates/slicer-host/src/cli.rs` — lines 71-84 (HostRunOptions current fields), 86-110 (CliError), 114-176 (validate_run_options)
  - `crates/slicer-host/src/main.rs` — lines 121-163 (Run arm), 200-210 (search_roots + load), 228-268 (config resolution), 315-335 (report handling)
  - `crates/slicer-host/src/lib.rs` — line 47 (current re-export)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/cli.rs` — add 3 fields to HostRunOptions; delete validate_run_options; delete CliError and Display impl
  - `crates/slicer-host/src/main.rs` — construct HostRunOptions from CLI args; refactor Run arm to use HostRunOptions fields; remove all references to validate_run_options
  - `crates/slicer-host/src/lib.rs` — remove `validate_run_options` and `CliError` from pub use
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/tests/cli_tdd.rs` — not yet (dead-code tests deleted in separate step)
  - `crates/slicer-host/src/report/collector.rs` — not yet
- Expected sub-agent dispatches:
  - "Run `rg -c 'validate_run_options|CliError' crates/slicer-host/src/`; return FACT" — confirm dead code removed
  - "Run `cargo check --workspace`; return FACT pass/fail" — validate compile
- Context cost: **S**
- Authoritative docs:
  - Read `cli.rs:71-84` directly; read `cli.rs:86-176` for deletion surface
  - Read `main.rs:121-165` directly (Run arm destructure)
  - Read `main.rs:200-268` directly (search_roots + config resolution)
- OrcaSlicer refs: None.
- Verification:
  - `rg -q 'thumbnail: Option<PathBuf>' crates/slicer-host/src/cli.rs && echo PASS || echo FAIL`
  - `rg -c 'validate_run_options|CliError' crates/slicer-host/src/ | rg '^0$' -q && echo PASS || echo FAIL`
  - `cargo check --workspace` (delegated)
- Exit condition: `cargo check --workspace` passes; both rg checks return PASS.

### Step 3: Remove `--module` flag and update tests

- Task IDs:
  - `TASK-206`
- Objective: Remove `--module` flag from the `Run` variant, remove `module_path` from `HostRunOptions`, delete dead-code tests from `cli_tdd.rs`, update remaining tests.
- Precondition: Step 2 complete.
- Postcondition: `module` field does not exist in `Run` variant or `HostRunOptions`. `module: _` removed from `main.rs` destructure. Dead-code tests deleted from `cli_tdd.rs`. Remaining tests updated.
- Files allowed to read:
  - `crates/slicer-host/src/cli.rs` — lines 20-28 (Run variant, module arg), 71-84 (HostRunOptions, module_path field)
  - `crates/slicer-host/src/main.rs` — line 122 (module: _ binding)
  - `crates/slicer-host/tests/cli_tdd.rs` — lines 8-61, 64-93, 133-204
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/cli.rs` — remove `module: Option<String>` from Run variant; remove `module_path: Option<PathBuf>` from HostRunOptions
  - `crates/slicer-host/src/main.rs` — remove `module: _,` from match destructure
  - `crates/slicer-host/tests/cli_tdd.rs` — delete 3 tests (lines 133-204); remove `--module` from remaining 4 tests; update assertions for PathBuf types
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/report/collector.rs` — not yet
- Expected sub-agent dispatches:
  - "Run `cargo run --bin slicer-host -- run --help 2>&1 | rg -q -- '--module' && echo FAIL || echo PASS`; return FACT" — confirm flag removed from help
  - "Run `cargo run --bin slicer-host -- run --module /tmp/mod.wasm --model /tmp/model.stl 2>&1; echo EXIT:$LASTEXITCODE`; return FACT" — confirm clap rejects removed flag (exit != 0)
  - "Run `cargo check --workspace`; return FACT pass/fail" — validate compile
- Context cost: **S**
- Authoritative docs: None beyond the files listed above.
- OrcaSlicer refs: None.
- Verification:
  - AC-3 and AC-N1 rg + cargo-run checks (see above)
  - `cargo check --workspace` (delegated)
- Exit condition: `cargo check --workspace` passes; `--help` shows no `--module`; passing `--module` exits non-zero.

### Step 4: Add parent-directory creation for output and report paths

- Task IDs:
  - `TASK-207`
- Objective: Add `fs::create_dir_all` before output and report file writes. Add a test for the output path behavior.
- Precondition: Steps 1-3 complete.
- Postcondition: `main.rs:329` creates parent dirs before `finish_and_render_to`. `main.rs:340` creates parent dirs before `std::fs::write`. `collector.rs:232-236` creates parent dirs before `std::fs::write`. New test `output_path_creates_parent_dir` passes.
- Files allowed to read:
  - `crates/slicer-host/src/main.rs` — lines 315-342 (report/output write blocks)
  - `crates/slicer-host/src/report/collector.rs` — lines 232-236 (finish_and_render_to)
  - `crates/slicer-host/tests/cli_tdd.rs` — entire file (to add new test)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/main.rs` — add `fs::create_dir_all(parent)` before output write at ~line 339; add same before report write at ~line 329
  - `crates/slicer-host/src/report/collector.rs` — add `fs::create_dir_all(parent)` before `std::fs::write(path, html)` at line 235
  - `crates/slicer-host/tests/cli_tdd.rs` — add `output_path_creates_parent_dir` test
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/pipeline.rs` — not touched
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test cli_tdd -- output_path_creates_parent_dir --nocapture`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure"
  - "Run `cargo test -p slicer-host --test cli_tdd`; return FACT pass/fail; SNIPPETS on failure with first failing assertion ≤ 20 lines" — full cli_tdd suite
- Context cost: **S**
- Authoritative docs:
  - Read `main.rs:329` and `main.rs:340` directly
  - Read `collector.rs:232-236` directly
- OrcaSlicer refs: None.
- Verification:
  - `cargo test -p slicer-host --test cli_tdd -- output_path_creates_parent_dir --nocapture` (delegated)
  - `cargo test -p slicer-host --test cli_tdd` (delegated, full suite)
- Exit condition: New test passes; full `cli_tdd` suite passes.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1: Normalize String→PathBuf | S | 3 files edited, types-only |
| Step 2: HostRunOptions + delete dead code | S | 3 files edited, deletion + struct completion |
| Step 3: Remove --module + update tests | S | 3 files edited, flag removal + test cleanup |
| Step 4: Parent-dir creation | S | 3 files edited, I/O guard + new test |
| **Total** | **M** | No step exceeds S; aggregate M |

## Packet Completion Gate

- All 4 steps complete.
- Every step exit condition met.
- All acceptance criteria green (each pipe-suffixed verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated with entries for TASK-204 through TASK-207 (via worker dispatch — never loaded into the implementer's context).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC from `packet.spec.md`.
- Confirm `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, and `cargo test -p slicer-host --test cli_tdd` all PASS.
- Confirm `cargo test -p slicer-host --test module_search_path_tdd` still PASS (regression guard).
- Confirm implementer's peak context usage stayed under 70%.
