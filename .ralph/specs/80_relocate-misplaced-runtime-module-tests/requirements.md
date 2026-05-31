# Requirements — Packet 80

## Packet Metadata

- **Packet**: 80
- **Slug**: `80_relocate-misplaced-runtime-module-tests`
- **Status**: draft
- **Task IDs**: TASK-229, TASK-230
- **Requires**: 79
- **Backlog source**: `docs/07_implementation_status.md`

## Problem Statement

`crates/slicer-runtime/tests/{executor,integration,unit}/` contains 80+ files spread across five buckets. Five of them import a core-module by name in their `use` declarations: `wipe_tower_bed_bounds.rs` (uses `wipe_tower::WipeTower`), `prepass_support_generation_orca_parity_tdd.rs` (uses `support_planner::*`), `slicing_promotion_e2e_regression_tdd.rs` (uses `top_surface_ironing::TopSurfaceIroning`), `gcode_part_cooling_emission_tdd.rs` (uses `part_cooling::PartCooling`), and `gcode_skirt_brim_emission_tdd.rs` (uses `skirt_brim::SkirtBrim`). Recon (run during the original architectural review) categorized them: **two are misplaced module unit tests** that landed in runtime because module-side testing had no easy host-state seam; **three are legitimately runtime tests** whose system-under-test is a runtime symbol (`commit_*_builtin`, `Blackboard`, `DefaultGCodeEmitter`, `DefaultGCodeSerializer`) and the module is just fixture input.

The two misplaced files import only `slicer_ir` + `slicer_sdk` + the module — no `slicer_runtime::*` symbols. After packets 77-79, they belong in their respective module crates: `wipe-tower/tests/bed_bounds_tdd.rs` and `support-planner/tests/orca_parity_tdd.rs`. The support-planner relocation is particularly meaningful because that test already calls `host::test_support::install_log_capture()` manually — the `#[module_test]` macro wired up in packet 77 was designed exactly for this case. After relocation, the test uses `#[module_test]` and the manual install_log_capture call disappears.

The three legitimately-runtime tests stay where they are. To prevent future agents from re-litigating the relocation question on every architectural pass, each gets a top-of-file `// NOT RELOCATABLE — SUT is <runtime symbol>, module <name> is fixture input` comment naming the actual runtime symbol under test. This is documentation-as-test-decision: a future agent grepping for `NOT RELOCATABLE` sees the rationale without needing to re-derive it.

The user flagged during grilling that `GCodeEmitter` may move to its own crate in a future packet, which would unlock relocating the two `gcode_*_emission` tests. P80 documents the current state but does not pre-empt that future work; the `NOT RELOCATABLE` comments name the specific symbols, so when those symbols move, the comments become stale and trigger the next relocation packet.

This packet is the smallest of the 77–80 sequence by design — packets 77/78/79 did the architectural work; P80 is the cleanup that completes the "every test whose SUT is a module lives in that module's crate" invariant.

## In Scope

- **Two relocations**:
  - `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` → `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`. During relocation: replace the hand-rolled `config_from_pairs` + `layer_with_tool_change` helpers with `ConfigViewBuilder` + the packet-79 `LayerCollectionFixtureBuilder` + `tool_change(...)`. Preserve every original assertion. Add `slicer-sdk = { ..., features = ["test"] }` to `modules/core-modules/wipe-tower/Cargo.toml` `[dev-dependencies]` if not already present from packet 79.
  - `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` → `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`. During relocation: switch the `#[test]` + manual `log_test_support::install_log_capture()` pattern to `#[module_test]` (the macro's `mock_host_setup` hook from packet 77 handles install/drain automatically). Use `slicer_sdk::test_prelude::*` for builder imports. Add `slicer-sdk = { ..., features = ["test"] }` to `modules/core-modules/support-planner/Cargo.toml` `[dev-dependencies]` — this is the first test the crate has had, so the section may need to be created.
- **Two aggregator updates**:
  - Remove `mod wipe_tower_bed_bounds;` from `crates/slicer-runtime/tests/executor/main.rs:42` (per recon-confirmed line).
  - Remove `mod prepass_support_generation_orca_parity_tdd;` from `crates/slicer-runtime/tests/executor/main.rs:36` (per recon-confirmed line).
- **Three `NOT RELOCATABLE` annotations** (one comment per file, near the top after the existing module doc-comment and `#![allow(missing_docs)]`):
  - `crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs` — comment naming `commit_shell_classification_builtin` + `commit_slice_builtin` + `Blackboard` as the runtime SUT.
  - `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` — comment naming `DefaultGCodeEmitter` + `DefaultGCodeSerializer` as the runtime SUT.
  - `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` — comment naming `DefaultGCodeEmitter` + `Blackboard` as the runtime SUT.
- **Preserve crate-level attributes during relocation**: both moved files have `#![allow(missing_docs)]` at line 6 per recon; this MUST be preserved in the relocated files because the destination module crates may not have a workspace-wide `missing_docs = "warn"` lint override. (Reading: the workspace `[workspace.lints.rust]` block in `Cargo.toml:70-73` does have `missing_docs = "warn"`, so the attribute is needed.)

## Out of Scope

- Relocating any of the three `NOT RELOCATABLE` tests. Their SUT is runtime; they stay.
- Moving `GCodeEmitter` / `DefaultGCodeEmitter` to its own crate — flagged by the user as future work; out of P80.
- Auditing the rest of `slicer-runtime/tests/` for further potential misplacements (the 5-file survey from the original architectural review was deliberate; broader audit deferred).
- Adding new test coverage. The relocated tests carry their original assertions; the migration to `#[module_test]` for the support-planner test is structural, not behavioral.
- Touching `crates/slicer-runtime/tests/integration/main.rs` (this aggregator is for the three gcode tests that stay; no changes needed).
- Touching any test file in `crates/slicer-runtime/tests/unit/` or `crates/slicer-runtime/tests/contract/` or `crates/slicer-runtime/tests/e2e/`. None of those buckets contained the misplaced files.
- Extending or modifying any builder or test-support helper from packets 77-79.
- Updating `docs/05_module_sdk.md` or `docs/00_project_overview.md` or any other doc file. The relocation is structural; no doc describes the layout being changed.
- Touching `OrcaSlicerDocumented/`. No parity concerns.

## Authoritative Docs

- `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` (current location; ≤ 200 lines per recon — readable directly) — the source of the relocation; read once to capture the assertion snapshot before rewriting helpers.
- `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` (current location) — same.
- `crates/slicer-runtime/tests/executor/main.rs` (aggregator) — confirmed lines 36 and 42 carry the two `mod` declarations to remove.
- `docs/02_ir_schemas.md` — IR-12 LayerCollectionIR + ToolChange field references for the wipe-tower relocation's fixture rewrite. Read only the relevant lines.
- `crates/slicer-sdk/src/test_prelude.rs` (post-packet-78) — the source of `MockHost`, `ConfigViewBuilder`, `LayerCollectionFixtureBuilder`, etc. Read once to confirm import paths.
- `CLAUDE.md` (project root) — §Test Discipline (narrow tests; the closure gate is per-crate, NOT `cargo test --workspace`).

## OrcaSlicer Reference Obligations

None. This packet does not borrow or check parity against any OrcaSlicer code. (The relocated `prepass_support_generation_orca_parity_tdd.rs` *file* mentions OrcaSlicer parity in its content — that's the support-planner module's responsibility, not this packet's. We relocate the test verbatim except for the helper-rewrite + `#[module_test]` switch.)

## Acceptance Summary

ACs are defined in `packet.spec.md` and referenced by ID. Measurable refinements:

- **AC-1 refinement**: the relocated `bed_bounds_tdd.rs` filename drops the `_bed_bounds` redundancy because the file is now inside `wipe-tower/tests/` (the `_bed_bounds_` part of the name was carrying that disambiguation). The new name is `bed_bounds_tdd.rs` per the destination convention. If the implementer prefers `bed_bounds.rs` (matching the original's lack of `_tdd` suffix), that's acceptable — the AC tolerates either; only the file's presence at SOME path under `wipe-tower/tests/` matters.
- **AC-2 refinement**: the destination filename `orca_parity_tdd.rs` drops the `prepass_support_generation_` prefix because the file is now inside `support-planner/tests/`. The OrcaSlicer-parity scope of the test is what the name should preserve.
- **AC-3 refinement**: the exact line numbers in `executor/main.rs` (36 and 42 per recon) may shift if intervening packets edit `main.rs`. The AC is line-number-agnostic; it asserts the absence of the two `mod` declarations regardless of position.
- **AC-5 refinement**: `support-planner` had zero tests pre-packet-80. This relocation is the FIRST test the crate has. The implementation log records this as a meaningful milestone — `support-planner` was the only core-module with no tests yet still had src-level reach into `host::log` (per packet 77's grilling). The relocation closes that gap.
- **AC-6 refinement**: the `NOT RELOCATABLE` comments are placed in the file header alongside any existing module-level doc-comment and `#![allow(missing_docs)]` attribute. Suggested format (one comment per file):
  ```
  //! <existing doc-comment>
  
  // NOT RELOCATABLE — SUT is <runtime symbol>, module <name> is fixture input.
  // If <runtime symbol> moves to its own crate in a future packet, revisit this comment.
  
  #![allow(missing_docs)]
  ```
- **AC-7 refinement**: `cargo test -p slicer-runtime` is one of the heavier per-crate test runs in the workspace (bundles 5 buckets). The implementation log captures the pre-relocation count and the post-relocation count; the difference equals the count of tests that moved out (typically 5-10 test functions across the two moved files).
- **AC-N2 refinement**: the manual ceremony probes whether `reset_global_state` from packet 77 is genuinely doing the work the `#[module_test]` macro requires. The implementer temporarily replaces `reset_global_state`'s body with `// no-op for probe` (in a working-tree-only edit, never committed), runs the relocated support-planner test twice — first with a `#[module_test]` that installs a log message, then a second `#[module_test]` that asserts no leftover logs. With the probe in place, the second test sees the first's log; with the probe reverted, the second test sees an empty buffer. This documents that packet 77's hook is load-bearing for the relocated test.

## Verification Commands

| AC | Command | Delegation hint |
|---|---|---|
| AC-1 | `test ! -f crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs && test -f modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs && grep -qE 'use slicer_sdk' modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs && ! grep -qE 'use slicer_runtime::' modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` | Direct file + grep checks. |
| AC-2 | `test ! -f crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs && test -f modules/core-modules/support-planner/tests/orca_parity_tdd.rs && [ $(grep -c '#\[module_test\]' modules/core-modules/support-planner/tests/orca_parity_tdd.rs) -ge 1 ] && ! grep -qE 'install_log_capture' modules/core-modules/support-planner/tests/orca_parity_tdd.rs` | Direct. |
| AC-3 | `! grep -qE '^mod (wipe_tower_bed_bounds|prepass_support_generation_orca_parity_tdd);' crates/slicer-runtime/tests/executor/main.rs` | Direct. |
| AC-4 | `cargo test -p wipe-tower --test bed_bounds_tdd` | Delegate; expect FACT pass + test count. |
| AC-5 | `cargo test -p support-planner && grep -A5 '^\[dev-dependencies\]' modules/core-modules/support-planner/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]'` | Delegate cargo test. |
| AC-6 | (compound, see `packet.spec.md`) | Direct head + grep loop. |
| AC-7 | `cargo test -p slicer-runtime` | Delegate; expect FACT total count vs pre-baseline. |
| AC-N1 | `! rg "use (wipe_tower|support_planner)::" crates/slicer-runtime/tests/` | Direct. |
| AC-N2 | Manual implementer ceremony — documented in `implementation-plan.md` step "Verify packet 77 hook is load-bearing". | Not CI-gated. |
| Closure: workspace check | `cargo check --workspace --all-targets` | Delegate. |
| Closure: clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Delegate. |
| Closure: targeted test sweep | `cargo test -p wipe-tower -p support-planner -p slicer-runtime` | Delegate; three packages; the slicer-runtime invocation is the heaviest. |
| Closure: guest staleness | `cargo xtask build-guests --check` then rebuild if STALE | Delegate; expect STALE because `support-planner/Cargo.toml` gains a `[dev-dependencies]` section (new section added; cargo recomputes inputs). |

## Step Completion Expectations

This packet has a load-bearing **ordering rule**: relocations (steps 2-3) MUST land green individually before the aggregator update (step 4) removes the `mod` declarations. Reordering would compile-fail because runtime/tests/executor would import nothing for the still-existing files (or vice versa). The per-step preconditions in `implementation-plan.md` encode this.

## Context Discipline Notes

- Both source files being relocated are ≤ 600 lines per recon. The implementer can read them directly during relocation; no offset-loads needed.
- The relocation discipline is verbatim except for the documented helper rewrites and `#[test]` → `#[module_test]` switch on the support-planner test. Every other line — `use` statements (rewritten to point at `slicer_sdk` / module crate), assertion statements, comments, doc-comments — preserves the original wording verbatim.
- Guest WASM is likely to be reported STALE because `support-planner/Cargo.toml` gains a `[dev-dependencies]` section (new section, not just a new line). The closure gate's `--check` must be followed by a rebuild.
- The packet is small enough that running ACs in parallel (after the relocations land) is feasible. The `slicer-runtime` test sweep (AC-7) is the longest single operation; dispatch with `run_in_background: true` if context is tight.
