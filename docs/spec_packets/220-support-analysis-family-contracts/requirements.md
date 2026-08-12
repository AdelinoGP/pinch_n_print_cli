# Requirements: support-analysis-family-contracts

## Packet Metadata

- Grouped task IDs: `TASK-331`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The live tree has only `PrePass::SupportGeometry` in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs:15-46`), a tree-specific `SupportPlanIR` carrying `ExtrusionPath3D` branch segments (`crates/slicer-ir/src/slice_ir.rs:1144-1207`), and flat path-only `SupportIR` (`crates/slicer-ir/src/slice_ir.rs:2172-2187`). Both support modules claim the same global `support-generator` and are selected by raw `support_type` (`crates/slicer-scheduler/src/execution_plan.rs:219-251, 393-412`). This packet establishes the strategy-neutral and family-atomic contracts needed by downstream planners.

## In Scope

- Add host-owned `PrePass::SupportAnalysis` and `SupportAnalysisIR` with strategy-neutral candidates, evidence, annotations, occupancy/termination surfaces, shared settings, feasible envelope, and family assignments.
- Add normalized exact-Z host query service with immutable caching.
- Replace branch-path semantics of `SupportPlanIR` with universal structural roles and metadata; migrate WIT, SDK, macro, marshal, blackboard, visual-debug, and tests in one schema decision.
- Replace flat `SupportIR` with attributed body/role entries while preserving printable extrusion paths only at render output.
- Add per-region `support_family` selection, compatibility aliases for `support_type`, paired planner/renderer manifest claims, startup pairing validation, and host multi-writer aggregation.
- Validate complete bodies against exact-Z occupancy/routing cells, drop invalid bodies atomically, and represent degraded unmet-demand diagnostics.
- Remove missing-plan fallback filler semantics; disabled support emits no support output.

## Out of Scope

- Tree or traditional family algorithm implementation, which belong to TASK-332/TASK-333.
- Mixed-family routing conflict policy beyond the base attribution/routing-cell contract, which belongs to TASK-334.
- Final Orca closure evidence, which belongs to TASK-335.
- Changing raft scheduling or `claim:raft-fill`.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - 430 lines; direct full read.
- `docs/adr/0059-support-families-and-anchored-entities.md` - delegated SUMMARY; approved architecture decision.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` - delegate bounded summaries.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - documented traditional support planning boundaries.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - documented tree support planning boundaries.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md`.
- Negative: `AC-N1` and `AC-N2` cover pairing enforcement and support-disabled suppression.
- Cross-packet impact: consumes TASK-330's `AnchoredEntity`, `AnchoredGeometryContract`, `CapabilityDerivedEventClosure`, `OrderedEventCollection`, and `AnchoredEventRuntimeHooks` exactly as exported.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-wasm-host --test contract exact_z_support_query -- --exact` | Exact-Z normalization/cache/query shape | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract support_plan_structural_contract -- --exact` | Universal plan fields and no toolpaths | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact` | Atomic family selection and alias mapping | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration support_family_pairing_rejected -- --exact` | Fatal mismatch enforcement | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration structured_support_identity -- --exact` | Identity through anchored execution and G-code handoff; the target is registered by TASK-330 | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract support_plan_validation -- --exact` | Complete-body validation and degraded unmet diagnostics | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract support_decline_contract -- --exact` | Decline reasons and no fallback filler | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration support_disabled_no_output -- --exact` | Disabled-support suppression; the target is registered by TASK-330 | FACT pass/fail |
| `cargo check --workspace --all-targets` | IR/WIT/macro blast radius | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint | FACT pass/fail |

## Step Completion Expectations

- Host aggregation is the sole multi-writer merge point.
- Family planners never read or transform one another's plans.
- A family may decline candidates without fatal slice failure; malformed schemas, crashes, and mismatched pairs remain fatal.
- TASK-330 event collections are used for anchored support rendering rather than synthetic model layers.

## Context Discipline Notes

This packet crosses IR, WIT, macro, scheduler, host marshal, and module manifests. Keep each worker to bounded ranges and delegate generated binding/cargo checks. Do not load `target/` or Orca source.
