---
status: implemented
packet: 80
task_ids: [TASK-229, TASK-230]
---

# 80_relocate-misplaced-runtime-module-tests

## Goal

Move the two `slicer-runtime/tests/executor/` files whose system-under-test is a core-module (`wipe_tower_bed_bounds.rs` → `wipe-tower/tests/`, `prepass_support_generation_orca_parity_tdd.rs` → `support-planner/tests/`) into their respective module crates, switching the support-planner test from manual `host::test_support::install_log_capture` setup to `#[module_test]` so the relocation directly exercises the seam packet 77 created. Annotate the three other runtime-located tests that import a module by name (`slicing_promotion_e2e_regression_tdd`, `gcode_part_cooling_emission_tdd`, `gcode_skirt_brim_emission_tdd`) with `// NOT RELOCATABLE — SUT is <runtime symbol>, module is fixture input` comments so future agents do not re-litigate.

## Problem Statement

`crates/slicer-runtime/tests/{executor,integration,unit}/` contains 80+ files spread across five buckets. Five of them import a core-module by name in their `use` declarations: `wipe_tower_bed_bounds.rs` (uses `wipe_tower::WipeTower`), `prepass_support_generation_orca_parity_tdd.rs` (uses `support_planner::*`), `slicing_promotion_e2e_regression_tdd.rs` (uses `top_surface_ironing::TopSurfaceIroning`), `gcode_part_cooling_emission_tdd.rs` (uses `part_cooling::PartCooling`), and `gcode_skirt_brim_emission_tdd.rs` (uses `skirt_brim::SkirtBrim`). Recon (run during the original architectural review) categorized them: **two are misplaced module unit tests** that landed in runtime because module-side testing had no easy host-state seam; **three are legitimately runtime tests** whose system-under-test is a runtime symbol (`commit_*_builtin`, `Blackboard`, `DefaultGCodeEmitter`, `DefaultGCodeSerializer`) and the module is just fixture input.

The two misplaced files import only `slicer_ir` + `slicer_sdk` + the module — no `slicer_runtime::*` symbols. After packets 77-79, they belong in their respective module crates: `wipe-tower/tests/bed_bounds_tdd.rs` and `support-planner/tests/orca_parity_tdd.rs`. The support-planner relocation is particularly meaningful because that test already calls `host::test_support::install_log_capture()` manually — the `#[module_test]` macro wired up in packet 77 was designed exactly for this case. After relocation, the test uses `#[module_test]` and the manual install_log_capture call disappears.

The three legitimately-runtime tests stay where they are. To prevent future agents from re-litigating the relocation question on every architectural pass, each gets a top-of-file `// NOT RELOCATABLE — SUT is <runtime symbol>, module <name> is fixture input` comment naming the actual runtime symbol under test. This is documentation-as-test-decision: a future agent grepping for `NOT RELOCATABLE` sees the rationale without needing to re-derive it.

The user flagged during grilling that `GCodeEmitter` may move to its own crate in a future packet, which would unlock relocating the two `gcode_*_emission` tests. P80 documents the current state but does not pre-empt that future work; the `NOT RELOCATABLE` comments name the specific symbols, so when those symbols move, the comments become stale and trigger the next relocation packet.

This packet is the smallest of the 77–80 sequence by design — packets 77/78/79 did the architectural work; P80 is the cleanup that completes the "every test whose SUT is a module lives in that module's crate" invariant.

## Architecture Constraints

- **Test imports must shift cleanly from runtime to module-local visibility.** The wipe-tower relocation: `use wipe_tower::WipeTower;` (cross-crate, runtime) → `use wipe_tower::WipeTower;` (same — but now self-crate because the test lives in `wipe-tower/tests/`, which gets crate-local access). The support-planner relocation has the same pattern: `use support_planner::{point_in_polygon, tapered_radius, SupportPlanner};` works in both locations.
- **Aggregator removal MUST happen AFTER the file is deleted from its old location.** Order: (a) write the new file at the destination, (b) verify it compiles + passes via `cargo test -p <module> --test <new_name>`, (c) delete the source file, (d) update the aggregator. Reversing (c) and (d) leaves a transient broken `mod` reference. The implementation-plan step ordering enforces this.
- **`#[module_test]` requires `slicer-sdk = { features = ["test"] }` in `[dev-dependencies]`.** The support-planner relocation introduces both — the macro and the dev-dep — in the same commit. Without the feature, the macro expansion's `::slicer_sdk::test_support::*` paths fail to resolve.
- **`NOT RELOCATABLE` comments are placed deliberately between the existing module doc-comment and the `#![allow(missing_docs)]` attribute.** This is the most-read part of a Rust test file (top 20 lines); future agents scanning for relocation decisions will find it.
- **Assertion preservation is non-negotiable.** Same discipline as packet 79: every original `assert!`, `assert_eq!`, etc. survives verbatim. The implementer captures pre-relocation snapshots for both moved files (just two snapshots — this packet's scope is small) for the implementation log.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

### Pre-relocation file structure (per recon, line 1-30 of each)

Both moved files start with:
```
//! <existing module doc-comment block>
//!
//! ... description of what the test exercises ...
#![allow(missing_docs)]

use ...;
```

Post-relocation, both retain the doc-comment + `#![allow(missing_docs)]` and just change the imports. The wipe-tower file additionally swaps the `config_from_pairs` / `layer_with_tool_change` helpers' bodies.

### Three legitimately-runtime tests (full set, for traceability)

| File | Runtime SUT | Module fixture |
|---|---|---|
| `crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs` | `commit_shell_classification_builtin`, `commit_slice_builtin`, `Blackboard` | `top-surface-ironing` |
| `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` | `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `load_module_from_paths`, `Blackboard` | `part-cooling` |
| `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` | `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `Blackboard` | `skirt-brim` |

The `NOT RELOCATABLE` comments name the specific runtime symbols so future agents (especially the planned future packet that moves the GCodeEmitter to its own crate) can detect the comment's staleness when those symbols move.

### Suggested `NOT RELOCATABLE` comment block format

```rust
// NOT RELOCATABLE — SUT is <symbol-list>; module <name> is fixture input.
// If <symbol> moves out of slicer-runtime in a future packet, this comment becomes stale
// and the test should be re-evaluated for relocation to the module's crate.
```

## Locked Assumptions and Invariants

- **Invariant A**: The relocated `wipe-tower` test uses `LayerCollectionFixtureBuilder` + `tool_change(...)` (from packet 79) and `ConfigViewBuilder` (from packet 78). Both must already exist; this packet does not extend them.
- **Invariant B**: The relocated `support-planner` test uses `#[module_test]` (packet 77) + `slicer_sdk::test_prelude::*` (packet 78). The `mock_host_setup` hook from packet 77 handles `install_log_capture` — the explicit call is removed.
- **Invariant C**: Both relocated files preserve every original assertion verbatim. The implementer's pre/post snapshot ceremony confirms this.
- **Invariant D**: The three `NOT RELOCATABLE` annotations are factual — they name actual runtime symbols imported by each file. The implementer verifies the symbol names against the file's `use slicer_runtime::{...}` line before writing the comment.
- **Invariant E**: `slicer-runtime`'s test count decreases by the count of test functions in the two moved files (typically 2-5 each); `wipe-tower` and `support-planner` test counts increase by the matching amount. The implementation log records both sides.

## Risks and Tradeoffs

- **Risk: removing aggregator lines before deleting source files leaves a broken `mod` reference.** Mitigation: ordering rule in `implementation-plan.md` — write destination, verify destination, delete source, update aggregator. Each step's exit condition gates the next.
- **Risk: the relocated `support-planner` test uses `#[module_test]` for the first time; if packet 77's hook implementation has a bug not caught by packet 77's tests, this is where it surfaces.** Mitigation: AC-N2's manual ceremony verifies the hook is load-bearing; if the test passes only because of accidental ordering, the manual probe catches it.
- **Risk: the `support-planner` crate gains its first dev-dep section, which may regenerate `Cargo.lock` in a noisy way.** Mitigation: commit the lockfile change with the Cargo.toml addition; review the diff briefly; expect only the path-dep edge addition.
- **Risk: relocating into a module crate's `tests/` directory triggers `cargo xtask build-guests --check` to report stale.** Mitigation: closure gate includes the rebuild; the implementer expects STALE and rebuilds.
- **Tradeoff: relocating one test but leaving three "stays in runtime" with comments creates an asymmetric file population in `slicer-runtime/tests/executor/`.** Accepted: the asymmetry is documented (3 tests with NOT RELOCATABLE; ~30 others without). Future agents reading the directory can distinguish "explicitly evaluated, decided to stay" from "never evaluated."
