# Design — Packet 80

## Controlling Code Paths

The packet has three small surfaces: two file relocations + one aggregator edit + three header-comment additions.

**Relocation 1 — `wipe_tower_bed_bounds.rs`**:

- Source: `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` (≈ 200 lines per recon).
- Destination: `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`.
- Pre-relocation imports: `slicer_ir::{ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey, SemVer, ToolChange}`, `slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView}`, `wipe_tower::WipeTower`. No `slicer_runtime::*`.
- Post-relocation imports: `slicer_sdk::test_prelude::*` (or the specific helpers used: `ConfigViewBuilder`, `LayerCollectionFixtureBuilder`, `tool_change`, `print_entity`), `slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView}` (preserved verbatim), `wipe_tower::WipeTower` (preserved).
- Helper rewrites: `config_from_pairs` body → `ConfigViewBuilder::new()....build()`; `layer_with_tool_change` body → `LayerCollectionFixtureBuilder::new()....add_tool_change(tool_change(...)).build()`. The function signatures may stay (call-site readability) or disappear (single-use shorthand — implementer choice).
- Assertions preserved verbatim. Recon confirmed the file has `#![allow(missing_docs)]` at line 6 — preserve.

**Relocation 2 — `prepass_support_generation_orca_parity_tdd.rs`**:

- Source: `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs`.
- Destination: `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`.
- Pre-relocation imports: `slicer_ir::*`, `slicer_sdk::host::{test_support as log_test_support, LogLevel}`, `slicer_sdk::prepass_builders::SupportGeometryOutput`, `slicer_sdk::prepass_types::*`, `slicer_sdk::traits::PrepassModule`, `support_planner::{point_in_polygon, tapered_radius, SupportPlanner}`. No `slicer_runtime::*`.
- Post-relocation: same imports MINUS the manual `log_test_support` import (because `#[module_test]` handles install/drain via packet 77's `mock_host_setup`/`mock_host_teardown`); ADD `slicer_sdk::test_prelude::*` if the test uses any builders, OR keep just `slicer_sdk::prelude::*` if it only constructs IR shapes via `..Default::default()`.
- Behavioral rewrite: the test function(s) that previously did `#[test] fn t() { log_test_support::install_log_capture(); let _ = log_test_support::take_log_messages(); log_test_support::install_log_capture(); ... assertions ... }` (per recon, this is the pattern at lines 459-518 of the original file) become `#[module_test] fn t() { ... assertions ... }`. The `install_log_capture` calls disappear because `mock_host_setup` runs first; the `take_log_messages()` drain inside the test body stays (it's the actual assertion mechanism).
- Assertions preserved verbatim. `#![allow(missing_docs)]` at line 6 preserved.

**Aggregator update — `executor/main.rs`**:

- Remove `mod wipe_tower_bed_bounds;` (recon: line 42).
- Remove `mod prepass_support_generation_orca_parity_tdd;` (recon: line 36).
- Preserve all other `mod` declarations in the file.

**Three `NOT RELOCATABLE` comments**:

- `crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs` — header gains a `// NOT RELOCATABLE — SUT is commit_shell_classification_builtin / commit_slice_builtin / Blackboard; module top-surface-ironing is fixture input.` comment after the existing doc-comment.
- `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` — similar comment naming `DefaultGCodeEmitter`, `DefaultGCodeSerializer`.
- `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` — similar comment naming `DefaultGCodeEmitter`, `Blackboard`.

`Cargo.toml` updates:

- `modules/core-modules/wipe-tower/Cargo.toml` — adds `slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` if not already present from packet 79. (Packet 79 migrates wipe-tower's existing in-module tests, so this dev-dep is already there.)
- `modules/core-modules/support-planner/Cargo.toml` — populates the existing empty `[dev-dependencies]` section with its first entry: `slicer-sdk = { ..., features = ["test"] }`. (Recon: section is present but empty pre-packet-80.)

## Architecture Constraints

- **Test imports must shift cleanly from runtime to module-local visibility.** The wipe-tower relocation: `use wipe_tower::WipeTower;` (cross-crate, runtime) → `use wipe_tower::WipeTower;` (same — but now self-crate because the test lives in `wipe-tower/tests/`, which gets crate-local access). The support-planner relocation has the same pattern: `use support_planner::{point_in_polygon, tapered_radius, SupportPlanner};` works in both locations.
- **Aggregator removal MUST happen AFTER the file is deleted from its old location.** Order: (a) write the new file at the destination, (b) verify it compiles + passes via `cargo test -p <module> --test <new_name>`, (c) delete the source file, (d) update the aggregator. Reversing (c) and (d) leaves a transient broken `mod` reference. The implementation-plan step ordering enforces this.
- **`#[module_test]` requires `slicer-sdk = { features = ["test"] }` in `[dev-dependencies]`.** The support-planner relocation introduces both — the macro and the dev-dep — in the same commit. Without the feature, the macro expansion's `::slicer_sdk::test_support::*` paths fail to resolve.
- **`NOT RELOCATABLE` comments are placed deliberately between the existing module doc-comment and the `#![allow(missing_docs)]` attribute.** This is the most-read part of a Rust test file (top 20 lines); future agents scanning for relocation decisions will find it.
- **Assertion preservation is non-negotiable.** Same discipline as packet 79: every original `assert!`, `assert_eq!`, etc. survives verbatim. The implementer captures pre-relocation snapshots for both moved files (just two snapshots — this packet's scope is small) for the implementation log.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

Three small surfaces:

1. **`crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs`** (delete) + **`modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`** (new) — relocation 1. ~200 LoC moves; helper bodies (~40 LoC) get rewritten.
2. **`crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs`** (delete) + **`modules/core-modules/support-planner/tests/orca_parity_tdd.rs`** (new) — relocation 2. Larger file (~550 LoC per recon); preserves all assertions, drops install_log_capture pair, switches `#[test]` → `#[module_test]` on the affected function(s).
3. **`crates/slicer-runtime/tests/executor/main.rs`** + 3 `NOT RELOCATABLE` annotations + 2 `Cargo.toml` dev-dep updates — mechanical edits.

## Files in Scope (read+edit)

Edit-allowed:

- `crates/slicer-runtime/tests/executor/main.rs` (remove 2 `mod` lines)
- `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` (delete after relocation)
- `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` (delete after relocation)
- `crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs` (add header comment)
- `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (add header comment)
- `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` (add header comment)
- `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (new)
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (new)
- `modules/core-modules/support-planner/Cargo.toml` (add `[dev-dependencies]` section)
- `modules/core-modules/wipe-tower/Cargo.toml` (verify dev-dep present from packet 79; no-op if so)

## Read-Only Context

- `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` — full read to capture original helpers and assertions before relocation.
- `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` — full read for the same reason. ≈ 550 lines per recon; safely under the 600-line cap.
- `crates/slicer-runtime/tests/executor/main.rs` — recon-confirmed lines 36 and 42 carry the two `mod` declarations.
- `modules/core-modules/wipe-tower/Cargo.toml` — confirm post-packet-79 state.
- `modules/core-modules/support-planner/Cargo.toml` — pre-packet-80 state (`[dev-dependencies]` section exists but is empty; this packet adds its first entry).
- `crates/slicer-sdk/src/test_prelude.rs` (post-packet-78) — confirm import paths.

## Out-of-Bounds Files

- All other `crates/slicer-runtime/tests/` files except the six named above.
- All other modules in `modules/core-modules/` except `wipe-tower` and `support-planner` (which are touched per scope).
- `crates/slicer-sdk/**` — frozen since packet 79.
- `crates/slicer-macros/**` — frozen since packet 77.
- `crates/slicer-ir/**`, `crates/slicer-core/**`, `crates/slicer-schema/**`, `crates/slicer-helpers/**`, `crates/pnp-cli/**`, `xtask/**`.
- `OrcaSlicerDocumented/**` — never load.
- All `target/`, all `*.wasm` artifacts, `Cargo.lock` (will regenerate mechanically).
- `docs/**` — no doc files are edited.

## Expected Sub-Agent Dispatches

1. **Pre-relocation assertion snapshot (wipe_tower_bed_bounds)** — `Question: list every assert! / assert_eq! / assert_ne! / panic! line in crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs. Scope: that file. Return: LOCATIONS (line:full-line, ≤ 30 entries).`
2. **Pre-relocation assertion snapshot (prepass_support_generation)** — same for the second file.
3. **Pre-relocation imports + helper bodies (both files)** — `Question: for each of the 2 files, list (a) the verbatim use statements, (b) the verbatim bodies of fn config_from_pairs / fn layer_with_tool_change / any other make_* helpers. Scope: the 2 files. Return: SNIPPETS (≤ 2 snippets per file, ≤ 30 lines each).`
4. **Test count baseline (pre)** — `Question: how many test functions exist in (a) crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs, (b) crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs, (c) crates/slicer-runtime/tests/ overall, (d) modules/core-modules/wipe-tower/tests/, (e) modules/core-modules/support-planner/tests/? Counting method: rg -c '^#\[(tokio::)?test\]' <path> | awk -F: '{s+=$2} END{print s+0}' (count #[test] / #[tokio::test] attributes at start-of-line; do NOT run cargo test). Scope: those paths. Return: FACT (≤ 5 lines).`
5. **Wipe-tower regression test** — `Question: does cargo test -p wipe-tower --test bed_bounds_tdd pass after the relocation? Scope: workspace. Return: FACT: pass count / first failure.`
6. **Support-planner regression test** — `Question: does cargo test -p support-planner pass after the relocation? Scope: workspace. Return: FACT: pass count.`
7. **Slicer-runtime regression** — `Question: does cargo test -p slicer-runtime pass after the moves + aggregator update? Scope: workspace. Return: FACT: total count + delta vs pre-baseline.`
8. **Guest staleness recheck** — `Question: does cargo xtask build-guests --check pass after the support-planner Cargo.toml addition? Scope: xtask. Return: FACT: clean / list of STALE guests.`

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

## Context Cost Estimate

- **Aggregate**: M. Six surfaces, all small; the heaviest individual step is the `slicer-runtime` test sweep at closure.
- **Largest single step**: the support-planner relocation (step 3) — M because the source file is ≈ 550 lines and the `#[module_test]` switch + import cleanup is the most behavioral change in the packet.
- **Highest-risk dispatch**: dispatch 7 (slicer-runtime regression test) — large test suite; return FACT only.

## Open Questions

None. Recon at packet generation time confirmed (a) the relocation targets' imports are clean of `slicer_runtime::*`, (b) the aggregator line numbers, (c) the three legitimately-runtime tests' SUT symbols. Every design decision was resolved during the grilling session.
