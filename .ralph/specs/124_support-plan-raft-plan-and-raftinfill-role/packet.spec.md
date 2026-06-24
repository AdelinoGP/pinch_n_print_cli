---
status: draft
packet: 124
task_ids:
  - TASK-265
  - TASK-266
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-plan-raft-plan-and-raftinfill-role

## Goal

Land the seam between `support-modules-orca-port` and the future `raft-default-module`: add `SupportPlanIR.raft_plan: Vec<RaftPlan>` (additive minor schema bump), introduce `ExtrusionRole::RaftInfill` + `claim:raft-fill` in the per-role-per-claim dispatch pattern, populate `raft_plan` from `support-planner` (replacing today's placeholder single-point-at-(0,0) code), and document `traditional-support`'s explicit non-consumption of `SupportPlanIR` per ADR-0009 + C7.

## Scope Boundaries

Touches `slicer-ir::SupportPlanIR` (additive field), `slicer-sdk::views.rs::should_emit` (one new role/claim arm), `support-planner/src/lib.rs` (raft_plan emission), `traditional-support` doc-comment (C7 only). Adds `ExtrusionRole::RaftInfill` enum variant. No new module is introduced — `raft-default` is owned by `docs/specs/raft-default-module.md` and ships in a separate packet block.

## Prerequisites and Blockers

- Depends on: `116_support-modules-doc-honesty-cleanup` (the traditional-support doc-comment already exists; this packet extends it with the explicit non-consumption statement); `120_support-modules-paint-segment-annotations-migration` implemented; `119_support-validation-wedge-harness` (the raft-count invariant evolves here).
- Unblocks: `raft-default-module` packet block (separate spec); future raft-renderer packets.
- Activation blockers: commit `74710fa` ("host-side fill partition + multi-module infill dispatch end-to-end") landed since the original spec; the implementer's Step 1 dispatch confirms whether the per-role-per-claim arm pattern in `should_emit` is still at `crates/slicer-sdk/src/views.rs:347-359` or has moved. The packet adapts.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** parsed, **then** `SupportPlanIR` has a field `raft_plan: Vec<RaftPlan>` and a struct `RaftPlan { object_id: ObjectId, footprint: Vec<ExPolygon>, layers: Vec<RaftLayerSpec>, z_bed: f32, gap_z: f32, first_layer_density: f32 }` plus `struct RaftLayerSpec { z: f32, height: f32 }` are defined and `pub`. | `rg -q 'pub raft_plan: Vec<RaftPlan>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub struct RaftPlan' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub struct RaftLayerSpec' crates/slicer-ir/src/slice_ir.rs`
- **AC-2. Given** `crates/slicer-ir/src/slice_ir.rs::ExtrusionRole`, **when** parsed, **then** the enum has a `RaftInfill` variant. | `rg -A20 'pub enum ExtrusionRole' crates/slicer-ir/src/slice_ir.rs | rg -q 'RaftInfill'`
- **AC-3. Given** `crates/slicer-sdk/src/views.rs::should_emit`, **when** searched, **then** the `match role` block contains the arm `ExtrusionRole::RaftInfill => "claim:raft-fill"`. | `rg -q 'ExtrusionRole::RaftInfill => "claim:raft-fill"' crates/slicer-sdk/src/views.rs`
- **AC-4. Given** `support-planner` running on the wedge fixture with `support_raft_layers = 3` and a `support_geometry` first-layer outline available, **when** the planner emits `SupportPlanIR`, **then** `raft_plan.len() == 1` (one object needs raft), `raft_plan[0].object_id` matches the wedge's object ID, `raft_plan[0].layers.len() == 3`, `raft_plan[0].footprint.len() >= 1`, and `raft_plan[0].z_bed > 0.0`. | `cargo test -p support-planner --test raft_plan_emission_tdd -- raft_plan_populated_for_three_layer_raft --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support-planner` running on the wedge fixture with `support_raft_layers = 0`, **when** the planner emits, **then** `raft_plan.is_empty()`. | `cargo test -p support-planner --test raft_plan_emission_tdd -- raft_plan_empty_when_disabled --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the placeholder raft block previously at `modules/core-modules/support-planner/src/lib.rs:381-423`, **when** searched, **then** the per-region duplicated single-point at `(0.0, 0.0, raft_z)` emission is GONE — no longer present in any code path. The verification uses ripgrep's multiline mode (`-U --multiline-dotall`) so the regex actually spans the multi-line `Point3WithWidth { ... }` literal; without `-U`, line-based search would never match a multi-line struct literal and the negation would always succeed (false-positive). | `! rg -U --multiline-dotall -q 'Point3WithWidth \{[\s\S]{0,80}x: 0\.0,[\s\S]{0,80}y: 0\.0,[\s\S]{0,80}z: raft_z' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'z: raft_z,' modules/core-modules/support-planner/src/lib.rs`
- **AC-7. Given** `crates/slicer-ir/src/slice_ir.rs::SupportPlanIR::schema_version`, **when** the IR is constructed in this packet's branch, **then** the version is bumped to the next minor (e.g., from `1.x.y` to `1.(x+1).0`) to reflect the additive `raft_plan` field. | `cargo test -p slicer-ir --test support_plan_ir_schema_version_bumped --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** `modules/core-modules/traditional-support/src/lib.rs`, **when** searched, **then** the lead `//!` block contains the sentence `Does NOT consume SupportPlanIR by design — see docs/specs/support-modules-orca-port.md §C7`. | `rg -q 'Does NOT consume SupportPlanIR by design' modules/core-modules/traditional-support/src/lib.rs`
- **AC-9. Given** `modules/core-modules/traditional-support/traditional-support.toml`, **when** searched, **then** `[ir-access].reads` does NOT contain `SupportPlanIR`. | `! rg -A5 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml | rg -q 'SupportPlanIR'`
- **AC-10. Given** `docs/02_ir_schemas.md` §"SupportPlanIR" (IR 9b), **when** read, **then** the section documents the new `raft_plan: Vec<RaftPlan>` field with the `RaftPlan` + `RaftLayerSpec` struct definitions inline. | `rg -q 'raft_plan: Vec<RaftPlan>' docs/02_ir_schemas.md && rg -q 'pub struct RaftLayerSpec' docs/02_ir_schemas.md`

## Negative Test Cases

- **AC-N1. Given** a multi-object fixture where one object needs raft and another does not, **when** the planner runs with `support_raft_layers = 3`, **then** `raft_plan.len() == 1` (only the object that needs support has a RaftPlan). | `cargo test -p support-planner --test raft_plan_emission_tdd -- raft_plan_per_object_needing_raft --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a guest module that does NOT hold `claim:raft-fill`, **when** it tries `region.should_emit(ExtrusionRole::RaftInfill)`, **then** the return is `false`. | `cargo test -p slicer-sdk --test should_emit_raft_fill_claim --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo build --workspace`
- `cargo test -p support-planner --test raft_plan_emission_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-sdk --test should_emit_raft_fill_claim 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-ir --test support_plan_ir_schema_version_bumped 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C6, §C7, §D5, §D6 — directly.
- `docs/adr/0009-raft-as-layer-infill-role.md` — directly (≤ 100 lines).
- `docs/specs/raft-default-module.md` — read directly. The sibling spec consumes the IR seam this packet defines.
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read lines 862-921 directly.
- `crates/slicer-sdk/src/views.rs` `should_emit` block — range-read (`should_emit` may have moved if commit 74710fa restructured the dispatch).

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` §"SupportPlanIR" — extend with `raft_plan` field documentation per AC-10. Verification: `rg -q 'raft_plan: Vec<RaftPlan>' docs/02_ir_schemas.md`.
- `docs/specs/support-modules-orca-port.md` §C7 — confirm the documented "traditional-support does not consume SupportPlanIR" decision is present (it is per the source spec; the packet only verifies it). Verification: `rg -q 'traditional-support does NOT consume SupportPlanIR' docs/specs/support-modules-orca-port.md` OR equivalent phrasing.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — confirm raft footprint shape, expansion factor (`raft_expansion`), gap-layer Z (`raft_z_gap`), per-layer height assignment. The packet doesn't render raft; it confirms the *plan data* the renderer (a future packet) will consume covers these properties.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
