---
status: implemented
packet: 124_support-plan-raft-plan-and-raftinfill-role
task_ids: [TASK-289]
---

# 124_support-plan-raft-plan-and-raftinfill-role

## Goal

Land the `RaftInfill` role/claim extension that ADR-0009 commits: add `ExtrusionRole::RaftInfill` to the Rust enum and its WIT mirror, add the `claim:raft-fill` arm to `should_emit`, bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` minor to reflect the additive `ExtrusionRole` variant, and audit every workspace `match role` site to guard against the silent-true-fallback at `views.rs:504`. The packet does NOT introduce geometry — `SupportPlanIR.raft_plan` is the config-only record §C6 mandates and the renderer is `raft-default-module` (separate spec).

## Problem Statement

ADR-0009 commits raft rendered through the existing `Layer::Infill` role/claim dispatch (`ExtrusionRole::RaftInfill` + `claim:raft-fill`), but the IR-side half was unimplemented: no enum variant, no WIT mirror, no `should_emit` arm, no schema bump. The `_ => return true` fallback in `should_emit` (`views.rs:504`) means a missing arm would silently emit `true` — the audit exists to catch that. Source-plan `TASK-265`/`TASK-266` both collide with unrelated ledger work and collapse into a single `TASK-289` (the original draft's `Vec<RaftPlan>` geometry claim was found already-implemented-and-§C6-forbidden).

## Architecture Constraints

- IR-side only: `ExtrusionRole::RaftInfill` variant in `crates/slicer-ir/src/slice_ir.rs` (with `default_priority() = 50`), WIT mirror `raft-infill` in `crates/slicer-schema/wit/deps/types.wit` (snake_case per WIT convention), `should_emit` arm `ExtrusionRole::RaftInfill => "claim:raft-fill"` in `crates/slicer-sdk/src/views.rs`.
- Schema bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.2.0 → 1.3.0 (semver-minor per ADR-0009 §Consequences — additive variant).
- The `match role` audit MUST run before the variant is added; the audit's `[explicit]` list determines which sites gain arms. 21 match sites audited; 3 explicit non-wildcard sites (`crates/slicer-macros/src/lib.rs:922/2067`, `crates/slicer-wasm-host/src/marshal/leaf.rs:369`) plus the IR→WIT macro and 3 g-code sites (`tolerance_for_role`, `resolve_feedrate`, `orca_type_label`) gained explicit arms per spec-review fix iteration to prevent silent fallback.
- `SupportPlanIR` / `RaftPlan` / `support-planner` NOT modified (landed in packet 119 per §C6). `traditional-support` NOT touched — C7 non-consumption verified, not edited.
- WIT enum addition triggers the 20-guest rebuild ceremony.

## Data and Contract Notes

- `should_emit(ExtrusionRole::RaftInfill)` is `true` only when the module holds `claim:raft-fill`; empty-claims suppression branch fires before the role lookup (AC-N3); modules without the claim return `false` (AC-N1).
- New behavioral test `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` (AC-4/AC-N1/AC-N3); new round-trip test `extrusion_role_raft_infill_roundtrip` in `crates/slicer-runtime/tests/contract/macro_all_worlds_roundtrip_tdd.rs` (AC-7).
- Geometry ownership: packet 124 owns the IR-side role/claim extension; raft geometry generation, the `SliceRegionView.raft_fill` carrier, and downstream rendering are owned by the sibling `raft-default-module.md` spec (ADR-0009). `SupportPlanIR.raft_plan: Option<RaftPlan>` remains configuration-only.

## Locked Assumptions and Invariants

- Sites with a `_ =>` wildcard are exempt from new arms (the variant falls into the wildcard); non-wildcard sites MUST have the explicit arm or `cargo build --workspace` fails non-exhaustively (AC-N2).
- The empty-claims branch at `views.rs:507-509` fires before the role lookup, matching existing `TopSolidInfill` behavior.

## Risks and Tradeoffs

- The silent-true-fallback is the load-bearing risk — the audit + AC-2 structural grep + behavioral tests gate it.
- Every guest rebuilds (WIT change); `wit_drift_detection_tdd` covers the new types.

## Implementation Deviations (recorded at close)

None. Doc Impact: `none` per packet — the docs already document the role/claim extension pattern; §C6 already names the packet. (2026-08-05 doc audit subsequently added the `claim:raft-fill` catalog row, `RaftInfill` priority-50 row, and schema-1.3.0 updates to `docs/02_ir_schemas.md` / `docs/03_wit_and_manifest.md` / `docs/01_system_architecture.md`.)
