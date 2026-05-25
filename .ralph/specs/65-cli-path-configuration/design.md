# Design: 65-cli-path-configuration

## Controlling Code Paths

- **Primary code path:** `slicer-host` binary entry (`main.rs:118-369`), CLI arg definitions (`cli.rs:7-176`), and the `Run` command pipeline setup.
- **Neighboring tests:** `cli_tdd.rs` (9 existing tests covering CLI parsing), `module_search_path_tdd.rs` (unrelated but runs the same binary path).
- **New test surface:** One new test in `cli_tdd.rs` (`output_path_creates_parent_dir`) to verify parent-directory creation.

## Architecture Constraints

- **Host-only change.** This packet touches no WASM compilation, no WIT boundary, no module manifest, and no guest code. The entire change surface is within `slicer-host`'s CLI and main entry point.
- **No geometry or coordinate-system conversion.** All changes are in the argument-parsing and file-I/O layer. The `coord-system` snippet does not apply.
- **No OrcaSlicer parity.** The `--module` flag was never an OrcaSlicer-visible feature; parent-dir creation is a standard OS pattern with no OrcaSlicer equivalent to compare against. The `orca-delegation` snippet does not apply.
- **Backward compatibility.** The `--module` flag is removed without a deprecation period (per user decision). Since the flag was never connected to any downstream behavior, no pipeline functionality breaks.
- **`HostRunOptions` must remain exported** from `lib.rs` because `docs/specs/default-builder-migration.md` (§Bucket D, lines 1405-1410) expects it as a clap-derived validated options struct.

## Code Change Surface

- **Selected approach:** Keep `HostRunOptions` and complete it, delete `validate_run_options` and `CliError`, construct `HostRunOptions` directly in `main.rs`. Alternative rejected: deleting the struct entirely would contradict the builder-migration spec which lists it as a Bucket D (no-builder) struct.
- **Exact functions, types, and tests expected to change:**

  | Symbol | File | Change |
  |--------|------|--------|
  | `Run` variant fields | `cli.rs:20-56` | Remove `module: Option<String>`; change `model: String` → `model: PathBuf`; `config: Option<String>` → `Option<PathBuf>`; `output: Option<String>` → `Option<PathBuf>` |
  | `HostRunOptions` struct | `cli.rs:71-84` | Add `thumbnail: Option<PathBuf>`, `report: Option<PathBuf>`, `report_verbose: bool`; remove `module_path: Option<PathBuf>` |
  | `validate_run_options` fn | `cli.rs:125-176` | Delete entirely |
  | `CliError` enum + `Display` | `cli.rs:86-110` | Delete entirely |
  | `lib.rs:47` re-exports | `lib.rs:47` | Remove `validate_run_options` and `CliError` from `pub use` |
  | `main.rs:121-163` run arm | `main.rs:121-163` | Remove `module: _,` from destructure; construct `HostRunOptions` via inline existence checks; use its fields for pipeline setup |
  | `main.rs` report write | `main.rs:299-307` | Add `create_dir_all(parent)` before `finish_and_render_to`; report the I/O error as a `warning:` line instead of swallowing it |
  | `main.rs` output write | `main.rs:325-330` | Replace inline `create_dir_all` + `std::fs::write` with a call to the new `write_with_parents` helper; error+exit on failure |
  | `write_with_parents` helper | `cli.rs` | New public free function: `pub fn write_with_parents(path: &Path, contents: &[u8]) -> std::io::Result<()>`. Centralises the "create parent dir, then write" pattern. Re-exported from `lib.rs`. |
  | `finish_and_render_to` | `collector.rs:232-236` | Add `create_dir_all(parent)` before `std::fs::write` (propagated via `?`) |
  | `cli_tdd.rs` tests | `cli_tdd.rs:8-204` | Delete 3 dead-code tests; remove `--module` from 4 remaining tests; change String literals to PathBuf; add `report_path_creates_parent_dir`, `output_path_creates_parent_dir`, and `write_with_parents_handles_bare_filename` tests |

## Files in Scope (read + edit)

- `crates/slicer-host/src/cli.rs` — primary: arg types, struct, dead code deletion
- `crates/slicer-host/src/main.rs` — primary: wire HostRunOptions, add create_dir_all
- `crates/slicer-host/tests/cli_tdd.rs` — primary: update/delete tests, add parent-dir test
- `crates/slicer-host/src/lib.rs` — secondary: one line (re-exports)
- `crates/slicer-host/src/report/collector.rs` — secondary: one function (3-4 lines)

## Read-Only Context

- `crates/slicer-host/src/cli.rs:71-84` — current `HostRunOptions` field list (to know what to add)
- `crates/slicer-host/src/main.rs:121-163` — current `Run` match arm destructure and config-path handling (to know what to replace)
- `crates/slicer-host/tests/cli_tdd.rs:133-204` — the 3 dead-code tests to delete, plus lines 8-93 for the 4 tests to update
- `crates/slicer-host/src/lib.rs:47` — current re-export line

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — no parity needed
- `target/`, `Cargo.lock`, generated code — never load
- Any file under `modules/` — no change surface in guest code
- `docs/01_system_architecture.md` through `docs/16_slicer_report.md` — no doc changes needed
- `crates/slicer-host/src/pipeline.rs` — not touched
- `crates/slicer-host/src/module_search_path.rs` — not touched
- Any `wit/` file — not touched

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace`; return FACT pass/fail" — validate Step 1-4
- "Run `rg -c 'validate_run_options|CliError' crates/slicer-host/src/`; return FACT" — validate dead code deletion
- "Run `cargo test -p slicer-host --test cli_tdd`; return FACT pass/fail; SNIPPETS on failure with first failing assertion ≤ 20 lines" — validate Step 4
- "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail; SNIPPETS on failure with first lint ≤ 20 lines" — validate all steps
- "Run `rg -c 'validate_run_options\|CliError\|--module' crates/slicer-host/tests/`; return FACT" — confirm no orphan test references

## Data and Contract Notes

- No IR or manifest contracts touched.
- No WIT boundary considerations.
- No determinism or scheduler constraints.

## Locked Assumptions and Invariants

- `HostRunOptions` remains publicly exported from `slicer-host` (matches the builder-migration spec's Bucket D classification).
- The `--module` flag's removal is not a breaking change for any known caller because the flag's value was always silently discarded.
- All verification commands produce the same exit code and output format on Linux, macOS, and Windows. Path-related tests use `tempfile` for platform-independent temp directories.

## Risks and Tradeoffs

- **No deprecation period for `--module`.** Users who scripted `--module` in CI or local workflows will get a clap parse error instead of a warning. Mitigation: the flag was never documented in architecture docs, never wired to behavior, and was described as "legacy" in the CLI help text.
- **`output_path_creates_parent_dir` test cannot run a full pipeline** (too heavy). It must test the `create_dir_all` logic in isolation, e.g., by testing a helper extracted from main.rs or by exercising the bare write path. The test is scoped to proving the directory is created, not that the pipeline produces correct G-code on the other side of the write.

## Context Cost Estimate

- Aggregate (sum across all steps): **M** (4 × S = M)
- Largest single step: **S** (Step 2: HostRunOptions + dead code deletion; 3 files edited, 2 sub-agent dispatches)
- Highest-risk dispatch: **`cargo test -p slicer-host --test cli_tdd`** — returns FACT pass/fail; SNIPPETS ≤ 20 lines on failure. Risk: if a step changes the wrong symbol, multiple tests fail simultaneously. Mitigation: run after every step, not just at the end.

## Open Questions

None.
