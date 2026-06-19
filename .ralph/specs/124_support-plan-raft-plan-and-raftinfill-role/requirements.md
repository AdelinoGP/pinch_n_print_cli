# Requirements: support-plan-raft-plan-and-raftinfill-role

## Packet Metadata

- Grouped task IDs:
  - `TASK-265` — `SupportPlanIR.raft_plan` + `ExtrusionRole::RaftInfill` + `claim:raft-fill` (C6)
  - `TASK-266` — `traditional-support` ↔ `SupportPlanIR` contract documented as "does not consume" (C7)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner`'s current raft handling is a placeholder: a single point at `(0.0, 0.0, raft_z)` emitted per region per raft layer (lines 381-423). The IR `SupportPlanEntry.global_layer_index: u32` is violated by the placeholder using negative `i32` values for raft (cast through `as i32`). Consumers see `SupportPlanIR.entries` carry zero-length "branch" entries that the renderer tries to extrude as zero-width paths.

ADR-0009 commits the design: `SupportPlanIR` gains an additive `raft_plan: Vec<RaftPlan>` field; raft rendering is handled by `Layer::Infill` modules via a new `claim:raft-fill` (mirroring existing `top-fill`, `bottom-fill`, `sparse-fill`, `bridge-fill` per the dispatch in `crates/slicer-sdk/src/views.rs::should_emit`). The renderer is a *future* `raft-default` module — out of scope here.

This packet does the IR side of ADR-0009 and the doc-only C7 (clarifying that `traditional-support` does not consume `SupportPlanIR` by design — the rectilinear scan-line filler is per-layer-independent and gains nothing from the planner's organic branches).

Commit `74710fa` ("host-side fill partition + multi-module infill dispatch end-to-end") landed since the original spec was written. The implementer's Step 1 dispatch confirms whether the per-role-per-claim arm pattern in `should_emit` (or the equivalent dispatcher) is still at `views.rs:347-359` — if it moved, the packet adapts to the new location.

## In Scope

- Add `RaftPlan`, `RaftLayerSpec` structs to `crates/slicer-ir/src/slice_ir.rs` (or wherever `SupportPlanIR` lives). Fields per ADR-0009 / `docs/specs/support-modules-orca-port.md` §C6.
- Add `raft_plan: Vec<RaftPlan>` field to `SupportPlanIR`.
- Bump `SupportPlanIR.schema_version` minor (additive change).
- Add `ExtrusionRole::RaftInfill` variant.
- Add `ExtrusionRole::RaftInfill => "claim:raft-fill"` arm in `should_emit` dispatch (or wherever the arm pattern lives post-74710fa).
- Update `crates/slicer-sdk` `Diagnostic`-style consumers / tests as needed for the new variant.
- Replace `support-planner/src/lib.rs` lines 381-423 (the placeholder raft block) with `RaftPlan` emission:
  - Compute the expanded raft footprint per object that requires raft (object has at least one branch contact).
  - Use `raft_first_layer_density`, `raft_layer_height_mm`, `raft_z_gap_mm` from config (already present per packet 1's doc-honesty cleanup; confirm via Step 1 dispatch).
  - Compute `z_bed` and per-raft-layer `z` + `height` populating `RaftLayerSpec`.
  - Emit one `RaftPlan` per object that needs raft.
- Add `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` with AC-4, AC-5, AC-N1 (running the planner end-to-end via the integration harness).
- Add `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` (or extend existing test) for AC-N2.
- Add `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` for AC-7.
- Update `traditional-support/src/lib.rs` doc-comment with the explicit non-consumption sentence.
- Confirm `traditional-support.toml [ir-access].reads` does NOT contain `SupportPlanIR` (it shouldn't, per the original spec; this packet asserts).
- Update `docs/02_ir_schemas.md` §"SupportPlanIR" with the new field + struct definitions.

## Out of Scope

- Implementing the raft renderer (any `Layer::Infill` module). Covered by future `raft-default-module` packet.
- Generating raft fill paths anywhere in this packet. The planner emits `RaftPlan` (the *plan*); no renderer consumes it yet.
- Removing `support_raft_layers` from `support-planner.toml`. The user-facing config key stays.
- `tree-support`'s raft handling. `tree-support` currently reads `SupportPlanEntry.branch_segments` only; the new `raft_plan` is a sibling field and `tree-support` does not need to change.
- Validation harness invariant for non-zero raft (the existing AC-6 in packet 4 covers `support_raft_layers = 0`; updating it for non-zero is part of this packet's Step 5).

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C6, §C7, §D5, §D6 — directly.
- `docs/adr/0009-raft-as-layer-infill-role.md` — directly.
- `docs/specs/raft-default-module.md` — directly (the sibling spec is the consumer of this packet's IR seam).
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read lines 862-921 directly.
- `crates/slicer-sdk/src/views.rs` — locate `should_emit` (range-read); confirm post-74710fa location.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — confirm raft footprint shape, expansion factor, gap-layer Z, per-layer height assignment.

## Acceptance Summary

- Positive cases: AC-1 through AC-10.
- Negative cases: AC-N1, AC-N2.
- Cross-packet impact: future `raft-default-module` packet consumes `SupportPlanIR.raft_plan`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current after IR + SDK changes. | FACT pass/fail |
| `cargo build --workspace` | End-to-end compile. | FACT pass/fail |
| `cargo test -p slicer-ir --test support_plan_ir_schema_version_bumped 2>&1 \| tee target/test-output.log` | AC-7. | FACT pass/fail |
| `cargo test -p slicer-sdk --test should_emit_raft_fill_claim 2>&1 \| tee target/test-output.log` | AC-N2. | FACT pass/fail |
| `cargo test -p support-planner --test raft_plan_emission_tdd 2>&1 \| tee target/test-output.log` | AC-4, AC-5, AC-N1. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `rg -q 'pub raft_plan: Vec<RaftPlan>' crates/slicer-ir/src/slice_ir.rs` | AC-1 IR field. | FACT pass/fail |
| `rg -A20 'pub enum ExtrusionRole' crates/slicer-ir/src/slice_ir.rs \| rg -q 'RaftInfill'` | AC-2 role variant. | FACT pass/fail |
| `rg -q 'ExtrusionRole::RaftInfill => "claim:raft-fill"' crates/slicer-sdk/src/views.rs` | AC-3 claim arm. | FACT pass/fail |
| `! rg -q 'Point3WithWidth.*x: 0\.0.*y: 0\.0.*z: raft_z' modules/core-modules/support-planner/src/lib.rs` | AC-6 placeholder gone. | FACT pass/fail |
| `rg -q 'Does NOT consume SupportPlanIR by design' modules/core-modules/traditional-support/src/lib.rs` | AC-8 doc. | FACT pass/fail |
| `! rg -A5 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml \| rg -q 'SupportPlanIR'` | AC-9 manifest. | FACT pass/fail |
| `rg -q 'raft_plan: Vec<RaftPlan>' docs/02_ir_schemas.md && rg -q 'pub struct RaftLayerSpec' docs/02_ir_schemas.md` | AC-10 docs. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint. | FACT pass/fail |

## Step Completion Expectations

- The IR field addition (Step 2) is additive. Every existing consumer of `SupportPlanIR` continues to work without changes — `tree-support` reads `entries` only and is unaffected. Step 2 MUST NOT modify the `SupportPlanEntry` shape; raft_plan is a sibling collection.
- The schema_version bump (Step 2) is minor (additive). The implementer must NOT bump to a new major; doing so breaks existing host compatibility checks per `docs/02_ir_schemas.md` semver convention.
- The placeholder raft block removal (Step 4) is atomic with the new `RaftPlan` emission — the implementer does not leave the placeholder code commented out or behind a feature flag. Either the new emission is present and passes ACs, or the placeholder stays — no in-between state.
- The traditional-support doc-comment edit (Step 6) extends the existing `//!` block from packet `116_support-modules-doc-honesty-cleanup`. The implementer adds one sentence; existing content is preserved.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-ir/src/slice_ir.rs` — likely 1,000+ lines; range-read `SupportPlanIR`, `ExtrusionRole`, `Point3WithWidth` definitions only.
  - `crates/slicer-sdk/src/views.rs` — range-read `should_emit` (was 347-359 in the original audit; commit 74710fa may have moved it).
  - `modules/core-modules/support-planner/src/lib.rs` — range-read around `plan_for_object`'s raft block (was 381-423).
  - `docs/02_ir_schemas.md` — read only §"SupportPlanIR" section (≤ 80 lines).
- Likely temptation reads (skip these):
  - The post-74710fa fill-partition test files (`infill_partition_e2e_tdd.rs` etc.) — out of scope; the new dispatch they exercise is separate from this packet's role/claim addition.
  - Other crates' tests for unrelated dispatch patterns.
  - The future raft-default-module — has its own spec; do NOT pre-design its renderer code.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo build --workspace` post-IR-change — FACT pass/fail; on fail SNIPPETS ≤ 30 lines FIRST error.
  - `cargo xtask build-guests --check` — FACT clean / STALE; never paste rebuild log.
  - LOCATIONS for `should_emit` post-74710fa — file:line + 1-line context, ≤ 5 entries.
