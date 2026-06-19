---
status: draft
packet: 116
task_ids:
  - TASK-250
  - TASK-251
  - TASK-252
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: support-modules-doc-honesty-cleanup

## Goal

Rewrite the lead doc-comments of `tree-support`, `traditional-support`, `support-planner` to describe what their code actually does (not the Orca-parity aspiration earlier scaffolding claimed), delete the dead `support_interface_bottom_layers` field from `SupportPlanner` and surface a `not_implemented` warning when a user sets it, and document the project-wide `BASE_SPEED = 50.0` speed-normalization convention in the four modules that use it.

## Scope Boundaries

Touches lead doc-comment blocks of three support modules + `rectilinear-infill`, the `SupportPlanner` struct fields + `on_print_start` parser, and one comment line in `support-planner.toml`. No algorithm change, no IR change, no WIT change. The only behavior change is the `LogLevel::Warn` diagnostic emitted at `on_print_start` when a user explicitly sets `support_interface_bottom_layers` to a value other than `-1`.

## Prerequisites and Blockers

- Depends on: none (P95 already implemented; P97 dead-WASM-mesh-seg already removed)
- Unblocks: packet `117_support-planner-geometric-correctness` (next Block B item, shares review surface)
- Activation blockers: none

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/tree-support/src/lib.rs`, **when** read, **then** the file's leading `//!` block (lines 1-12 or current equivalent) opens with `Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption` and contains the phrase `not a port of OrcaSlicer's TreeSupport`. | `rg -q 'Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption' modules/core-modules/tree-support/src/lib.rs && rg -q 'not a port of OrcaSlicer' modules/core-modules/tree-support/src/lib.rs`
- **AC-2. Given** `modules/core-modules/traditional-support/src/lib.rs`, **when** read, **then** the leading `//!` block opens with `Per-layer rectilinear scan-line filler for Layer::Support` and contains the phrase `Depends entirely on upstream SurfaceClassificationIR.needs_support`. | `rg -q 'Per-layer rectilinear scan-line filler for Layer::Support' modules/core-modules/traditional-support/src/lib.rs && rg -q 'Depends entirely on upstream SurfaceClassificationIR.needs_support' modules/core-modules/traditional-support/src/lib.rs`
- **AC-3. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** read, **then** the leading `//!` block contains `Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes. Implements the algorithmic shape (detect → contact → top-down MST propagation → emit) but not numerical parity`. | `rg -q 'Multi-layer support planner inspired by OrcaSlicer' modules/core-modules/support-planner/src/lib.rs && rg -q 'algorithmic shape \(detect → contact → top-down MST propagation → emit\) but not numerical parity' modules/core-modules/support-planner/src/lib.rs`
- **AC-4. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** parsed as Rust, **then** the `SupportPlanner` struct has no field named `support_interface_bottom_layers` and `on_print_start` has no `support_interface_bottom_layers = match config.get(...)` assignment. | `! rg -q 'support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs`
- **AC-5. Given** a fresh `SupportPlanner::on_print_start` call with `ConfigView` containing `support_interface_bottom_layers = Int(3)`, **when** invoked, **then** exactly one `LogLevel::Warn` diagnostic is emitted whose message contains the literal substring `support_interface_bottom_layers is not yet implemented`. | `cargo test -p support-planner --test on_print_start_not_implemented_warning -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the four modules `tree-support`, `traditional-support`, `support-planner`, `rectilinear-infill`, **when** their lead doc-comment blocks are read, **then** each contains a `# Speed normalization` heading followed by an explanation that `speed_factor = configured_speed / BASE_SPEED` with `BASE_SPEED = 50.0`. | `for m in tree-support traditional-support support-planner rectilinear-infill; do rg -q '# Speed normalization' modules/core-modules/$m/src/lib.rs && rg -q 'speed_factor = configured_speed / BASE_SPEED' modules/core-modules/$m/src/lib.rs || { echo "MISSING: $m"; exit 1; }; done`

## Negative Test Cases

- **AC-N1. Given** a fresh `SupportPlanner::on_print_start` call with `ConfigView` containing `support_interface_bottom_layers = Int(-1)` (the default), **when** invoked, **then** zero `LogLevel::Warn` diagnostics are emitted matching the `not yet implemented` message. | `cargo test -p support-planner --test on_print_start_no_warning_at_default -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a fresh `SupportPlanner::on_print_start` call with `ConfigView` that does NOT contain the `support_interface_bottom_layers` key, **when** invoked, **then** zero `LogLevel::Warn` diagnostics are emitted matching the `not yet implemented` message. | `cargo test -p support-planner --test on_print_start_no_warning_when_absent -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo build -p tree-support -p traditional-support -p support-planner -p rectilinear-infill`
- `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings`
- `cargo test -p support-planner 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — §B1, §B2, §B3, §D8, §D9. Source of the rewritten doc-comment text and the deletion+warn approach for `support_interface_bottom_layers`.
- `docs/01_system_architecture.md` — `Layer::Support` and `PrePass::SupportGeometry` stage descriptions; referenced by the new doc-comments. Read only the §`Layer::Support` and §`PrePass::SupportGeometry` sub-sections (~50 lines combined).

## Doc Impact Statement (Required)

`none` — this packet edits only Rust source doc-comments and one TOML comment. The authoritative spec at `docs/specs/support-modules-orca-port.md` already documents the rewritten language (D8 / D9). No public surface, IR, WIT, claim, or manifest schema changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
