# Requirements: mixed-support-family-routing

## Packet Metadata
- Grouped task IDs: `TASK-334`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement
The host currently has one generic support-plan ownership path while the approved family packets produce independent tree and traditional entries. Without deterministic ownership and conflict rejection, mixed regions can silently cross-write, collide, or fall back to an unrelated filler. This packet supplies the host boundary needed before closure evidence.

## In Scope
- Deterministic routing cells from candidate assignment and proximity, with stable tie-breaking.
- Host aggregation of tree and traditional entries and family attribution validation.
- Same-family union preserving all demand IDs.
- Complete-body exact-Z, routing-cell, and cross-family overlap rejection.
- Rendered cross-family swept-path conflict rejection and structured degraded diagnostics.
- Registration of the planned `support_family_routing` integration test target and aggregator module.

## Out of Scope
- Tree or traditional planning/rendering algorithms.
- Planner negotiation or cross-family structural sharing.
- Exact-Z service naming and WIT migration decision (owned by TASK-331) — RESOLVED in packet 220: `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs` (injected into `HostExecutionContext`, normalized to repo units, immutable per-(object,region,Z) caching); breaking in-place WIT replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`.
- Final Orca closure, fixture regeneration, or packet 213/TASK-329 disposition.

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§7-9 and invariants 1-7.
- `docs/02_ir_schemas.md` - delegated bounded summary.
- `docs/04_host_scheduler.md` - delegated bounded summary.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388`, `:1839`, `:1969` - tree distributed contacts, collision, and complete body emission.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374`, `:2953`, `:3106` - traditional support ownership, base propagation, and collision trimming.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` - interface role behavior.

## Acceptance Summary
- Positive: `AC-1` through `AC-6`.
- Negative: `AC-N1` through `AC-N2`.
- Cross-packet impact: consumes draft family planner/renderer exports and exports routing ownership and degraded conflict handling to TASK-335.

## Verification Commands
| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test support_family_routing -- --exact` | Run all mixed-family routing tests after the planned target is registered. | FACT pass/fail; failure SNIPPETS <=20 lines |
| `cargo check --workspace --all-targets` | Compile host, test target, and planned family contract changes. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce diagnostics and ownership code quality. | FACT pass/fail |

## Step Completion Expectations
Routing, validation, and rendered conflict checks use the same deterministic tolerance and diagnostic identity fields. No invalid body may be clipped or replaced by fallback geometry.

## Context Discipline Notes
The exact-Z service is resolved (`ExactZQueryService`, packet 220) and consumed from its implemented seam. Delegate broad IR and scheduler documents and never inspect target artifacts.
