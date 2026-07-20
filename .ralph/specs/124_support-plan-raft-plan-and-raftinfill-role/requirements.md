# Requirements: support-plan-raft-plan-and-raftinfill-role

## Packet Metadata

- Grouped task IDs:
  - `TASK-289` (renumbered; replaces source-plan `TASK-265` and `TASK-266`. `TASK-265` is now lightning-infill per `docs/07_implementation_status.md:230`; `TASK-266` is absent from the ledger). The renumber also covers C7's "does not consume" doc-only work that was originally on a separate `TASK-266`.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner`'s current raft handling is degenerate: a single point at `(0.0, 0.0, raft_z)` emitted per `(raft_layer, region)` (the real implementation at `modules/core-modules/support-planner/src/lib.rs:475-487`, not a placeholder as the original spec claimed). The `SupportPlanEntry.global_layer_index: i32` carries the negative raft indices, but the `(0, 0)` position is meaningless — consumers see zero-width paths at the build plate origin.

ADR-0009 commits the design: `SupportPlanIR` gains an additive `raft_plan: Vec<RaftPlan>` field; raft rendering is handled by `Layer::Infill` modules via a new `claim:raft-fill` (mirroring existing `top-fill`, `bottom-fill`, `sparse-fill`, `bridge-fill` per the dispatch in `crates/slicer-sdk/src/views.rs::should_emit` at lines 497-513). The renderer is a *future* `raft-default` module — out of scope here.

This packet does the IR side of ADR-0009 and the doc-only C7 (clarifying that `traditional-support` does not consume `SupportPlanIR` by design — the rectilinear scan-line filler is per-layer-independent and gains nothing from the planner's organic branches).

## In Scope

- Add `RaftPlan`, `RaftLayerSpec` structs to `crates/slicer-ir/src/slice_ir.rs` (the file where `SupportPlanIR` lives at line 1138). Fields per ADR-0009 / `docs/specs/support-modules-orca-port.md` §C6:
  ```rust
  pub struct RaftPlan {
      pub object_id: ObjectId,
      pub footprint: Vec<ExPolygon>,
      pub layers: Vec<RaftLayerSpec>,
      pub z_bed: f32,
      pub gap_z: f32,
      pub first_layer_density: f32,
  }
  pub struct RaftLayerSpec {
      pub z: f32,
      pub height: f32,
  }
  ```
- Add `raft_plan: Vec<RaftPlan>` field to `SupportPlanIR` (line 1138).
- Bump `SupportPlanIR.schema_version` minor (additive change). Confirm current value via Step 1 dispatch; bump to the next minor.
- Add `ExtrusionRole::RaftInfill` variant to the enum at `crates/slicer-ir/src/slice_ir.rs:1639`.
- Add `raft-infill` to the WIT mirror at `crates/slicer-schema/wit/deps/types.wit:12-17` (interface `geometry`).
- Add `ExtrusionRole::RaftInfill => "claim:raft-fill"` arm in `should_emit` dispatch (`crates/slicer-sdk/src/views.rs:497-513`).
- Replace `support-planner/src/lib.rs:442-491` (the current degenerate raft block) with `RaftPlan` emission:
  - Compute the expanded raft footprint per object that requires raft (object has at least one branch contact).
  - Use `raft_first_layer_density`, `raft_layer_height_mm`, `raft_z_gap_mm` from config (already present per packet 1's doc-honesty cleanup; confirm via Step 1 dispatch).
  - Compute `z_bed` and per-raft-layer `z` + `height` populating `RaftLayerSpec`.
  - Emit one `RaftPlan` per object that needs raft.
- Add `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` with AC-4, AC-5, AC-N1.
- Add `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` (or extend existing test) for AC-N2.
- Add `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` for AC-7.
- Update `traditional-support/src/lib.rs` doc-comment with the explicit non-consumption sentence (AC-8).
- Confirm `traditional-support.toml [ir-access].reads` does NOT contain `SupportPlanIR` (it doesn't per the current state at line 14; this packet asserts).
- Update `docs/02_ir_schemas.md` §"SupportPlanIR" with the new field + struct definitions (AC-10).
- Run `cargo xtask build-guests` after the WIT enum extension (triggers 20-guest rebuild).

## Out of Scope

- Implementing the raft renderer (any `Layer::Infill` module). Covered by future `raft-default-module` packet per `docs/specs/raft-default-module.md`.
- Generating raft fill paths anywhere in this packet. The planner emits `RaftPlan` (the *plan*); no renderer consumes it yet.
- Removing `support_raft_layers` from `support-planner.toml`. The user-facing config key stays (it's the input to the new `RaftPlan` emission).
- `tree-support`'s raft handling. `tree-support` currently reads `SupportPlanEntry.branch_segments` only; the new `raft_plan` is a sibling field and `tree-support` does not need to change.
- Validation harness invariant for non-zero raft (the existing `disabled_raft_has_no_negative_entries` test in `support_invariants_wedge_tdd.rs:235` covers `support_raft_layers = 0`; updating it for non-zero is part of this packet's Step 5).

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C6, §C7, §D5, §D6 — directly.
- `docs/adr/0009-raft-as-layer-infill-role.md` — directly.
- `docs/specs/raft-default-module.md` — directly (the sibling spec is the consumer of this packet's IR seam).
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read the existing section.
- `crates/slicer-sdk/src/views.rs` — locate `should_emit` (range-read at lines 497-513).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — confirm raft footprint shape, expansion factor, gap-layer Z, per-layer height assignment.

## Acceptance Summary

- Positive cases: AC-1 through AC-11.
- Negative cases: AC-N1, AC-N2, AC-N3.
- Cross-packet impact: future `raft-default-module` packet consumes `SupportPlanIR.raft_plan`; the WIT enum addition triggers 20-guest rebuild.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current after IR + SDK + WIT changes. | FACT pass/fail |
| `cargo build --workspace` | End-to-end compile. | FACT pass/fail |
| `cargo test -p slicer-ir --test support_plan_ir_schema_version_bumped 2>&1 \| tee target/test-output.log` | AC-7. | FACT pass/fail |
| `cargo test -p slicer-sdk --test should_emit_raft_fill_claim 2>&1 \| tee target/test-output.log` | AC-N2. | FACT pass/fail |
| `cargo test -p support-planner --test raft_plan_emission_tdd 2>&1 \| tee target/test-output.log` | AC-4, AC-5, AC-N1. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo test -p slicer-wasm-host --test wit_boundary_tdd 2>&1 \| tee target/test-output.log` | AC-N3 WIT round-trip. | FACT pass/fail |
| `rg -q 'pub raft_plan: Vec<RaftPlan>' crates/slicer-ir/src/slice_ir.rs` | AC-1 IR field. | FACT pass/fail |
| `rg -A20 'pub enum ExtrusionRole' crates/slicer-ir/src/slice_ir.rs \| rg -q 'RaftInfill'` | AC-2 role variant. | FACT pass/fail |
| `rg -q 'ExtrusionRole::RaftInfill => "claim:raft-fill"' crates/slicer-sdk/src/views.rs` | AC-3 claim arm. | FACT pass/fail |
| `! rg -U --multiline-dotall -q 'Point3WithWidth \{[\s\S]{0,80}x: 0\.0,[\s\S]{0,80}y: 0\.0,[\s\S]{0,80}z: raft_z' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'z: raft_z,' modules/core-modules/support-planner/src/lib.rs` | AC-6 degenerate emission gone. | FACT pass/fail |
| `rg -q 'raft-infill' crates/slicer-schema/wit/deps/types.wit` | AC-11 WIT mirror. | FACT pass/fail |
| `rg -q 'Does NOT consume SupportPlanIR by design' modules/core-modules/traditional-support/src/lib.rs` | AC-8 doc. | FACT pass/fail |
| `! rg -A5 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml \| rg -q 'SupportPlanIR'` | AC-9 manifest. | FACT pass/fail |
| `rg -q 'raft_plan: Vec<RaftPlan>' docs/02_ir_schemas.md && rg -q 'pub struct RaftLayerSpec' docs/02_ir_schemas.md` | AC-10 docs. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint. | FACT pass/fail |

## Step Completion Expectations

- The IR field addition (Step 2) is additive. Every existing consumer of `SupportPlanIR` continues to work without changes — `tree-support` reads `entries` only and is unaffected. Step 2 MUST NOT modify the `SupportPlanEntry` shape; raft_plan is a sibling collection.
- The schema_version bump (Step 2) is minor (additive). The implementer must NOT bump to a new major; doing so breaks existing host compatibility checks per `docs/02_ir_schemas.md` semver convention.
- The WIT enum addition (Step 3) requires `cargo xtask build-guests` (no `--check`) — every guest uses bindgen on the WIT file. Step 3 must rebuild guests BEFORE the integration tests in Step 5, otherwise AC-N3's WIT round-trip will fail with a stale-guest attribution.
- The degenerate raft block removal (Step 4) is atomic with the new `RaftPlan` emission — the implementer does not leave the degenerate code commented out or behind a feature flag. Either the new emission is present and passes ACs, or the degenerate code stays — no in-between state.
- The traditional-support doc-comment edit (Step 6) extends the existing `//!` block from packet `116_support-modules-doc-honesty-cleanup`. The implementer adds one sentence; existing content is preserved.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-ir/src/slice_ir.rs` — 2431 lines; range-read `SupportPlanIR` (line 1137-1145), `ExtrusionRole` (line 1639-1679), and `RaftPlan` (the new struct, after Step 2).
  - `crates/slicer-sdk/src/views.rs` — range-read `should_emit` (lines 481-513).
  - `modules/core-modules/support-planner/src/lib.rs` — 1590 lines; range-read the raft block (lines 442-491) + the `plan_for_object` function header (line 313).
  - `docs/02_ir_schemas.md` — read only §"SupportPlanIR" section.
  - `crates/slicer-schema/wit/deps/types.wit` — file is small; full read.
- Likely temptation reads (skip these):
  - The post-74710fa fill-partition test files (`infill_partition_e2e_tdd.rs` etc.) — out of scope; the new dispatch they exercise is separate from this packet's role/claim addition.
  - Other crates' tests for unrelated dispatch patterns.
  - The future raft-default-module — has its own spec; do NOT pre-design its renderer code.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo build --workspace` post-IR-change — FACT pass/fail; on fail SNIPPETS ≤ 30 lines FIRST error.
  - `cargo xtask build-guests --check` — FACT clean / STALE; never paste rebuild log.
  - LOCATIONS for `should_emit` — file:line + 1-line context, ≤ 5 entries.
