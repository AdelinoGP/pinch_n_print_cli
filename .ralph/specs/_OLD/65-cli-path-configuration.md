---
status: implemented
packet: 65-cli-path-configuration
task_ids:
  - TASK-204
  - TASK-205
  - TASK-206
  - TASK-207
---

# 65-cli-path-configuration

## Goal

Clean up stale CLI path configuration in `slicer-host` by completing `HostRunOptions`, deleting dead `validate_run_options` / `CliError`, removing the ignored `--module` flag, creating parent directories for output and report files before write, and normalizing `String` CLI arg types to `PathBuf`.

## Problem Statement

The `slicer-host` CLI has accumulated several path-configuration inconsistencies during its evolution from a single-module runner to a multi-root module discovery system:

1. **Dead validation code.** `validate_run_options` (`cli.rs:125-176`) performs file-existence checks for `--module`, `--model`, `--config`, and `--module-dir` but is **never called from `main.rs`**. The main binary validates paths inline (via `load_model`, `read_to_string`, and `load_live_modules_for_plan`). The function and `CliError` enum exist only as exported library API surface consumed by test code.

2. **Incomplete `HostRunOptions`.** The struct is supposed to be the validated runtime options object but lacks `thumbnail`, `report`, and `report_verbose` — all of which are path-bearing CLI args handled ad-hoc in `main.rs`.

3. **Silently ignored `--module` flag.** The `--module` flag is parsed by clap but bound to `_` in `main.rs:122`, making it a no-op that misleads users.

4. **Missing parent-directory creation.** `--output` and `--report` file writes fail cryptically when the parent directory does not exist.

5. **Inconsistent arg types.** Four CLI args use `String` (`module`, `model`, `config`, `output`) while three use `PathBuf` (`module_dir`, `thumbnail`, `report`).

These are not individually severe bugs, but collectively they represent API surface drift that wastes developer attention and erodes trust in the CLI contract. This packet is the smallest coherent remediation slice that closes all five gaps.

## Architecture Constraints

- **Host-only change.** This packet touches no WASM compilation, no WIT boundary, no module manifest, and no guest code. The entire change surface is within `slicer-host`'s CLI and main entry point.
- **No geometry or coordinate-system conversion.** All changes are in the argument-parsing and file-I/O layer. The `coord-system` snippet does not apply.
- **No OrcaSlicer parity.** The `--module` flag was never an OrcaSlicer-visible feature; parent-dir creation is a standard OS pattern with no OrcaSlicer equivalent to compare against. The `orca-delegation` snippet does not apply.
- **Backward compatibility.** The `--module` flag is removed without a deprecation period (per user decision). Since the flag was never connected to any downstream behavior, no pipeline functionality breaks.
- **`HostRunOptions` must remain exported** from `lib.rs` because `docs/specs/default-builder-migration.md` (§Bucket D, lines 1405-1410) expects it as a clap-derived validated options struct.

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
