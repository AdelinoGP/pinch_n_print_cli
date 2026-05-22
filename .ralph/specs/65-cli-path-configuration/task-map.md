# Task Map: 65-cli-path-configuration

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Context cost | Notes |
| --- | --- | --- | --- | --- | --- |
| TASK-204 (closed) | Step 1 | `docs/07_implementation_status.md` | `cli.rs:20-56`, `main.rs:121-168`, `cli_tdd.rs:24-93` | S | Normalize String CLI arg types to PathBuf |
| TASK-205 (closed) | Step 2 | `docs/07_implementation_status.md` | `cli.rs:66-110`, `main.rs:121-268`, `lib.rs:46` | S | Complete HostRunOptions, delete validate_run_options and CliError |
| TASK-206 (closed) | Step 3 | `docs/07_implementation_status.md` | `cli.rs:20-28`, `main.rs:122`, `cli_tdd.rs:8-130`, `slicer_cache.rs:243-272`, `dispatch_tdd.rs:5552-5577`, `gcode_part_cooling_emission_tdd.rs:548-573` | S | Remove --module flag; update test helpers across all test suites |
| TASK-207 (closed) | Step 4 | `docs/07_implementation_status.md` | `main.rs:303-342`, `collector.rs:232-236`, `cli_tdd.rs:84-98` | S | Create parent directories for --output and --report paths |

TASK-204 through TASK-207 already exist in `docs/07_implementation_status.md` (lines 151-154) and are marked `[x]`. The TASK-206 row reflects expanded scope: in addition to `cli.rs` and `cli_tdd.rs`, the `--module` flag was removed from test helper code in `tests/common/slicer_cache.rs`, `tests/dispatch_tdd.rs`, and `tests/gcode_part_cooling_emission_tdd.rs` after spec review exposed the gap.
