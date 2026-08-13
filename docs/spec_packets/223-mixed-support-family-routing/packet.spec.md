---
status: draft
packet: 223-mixed-support-family-routing
task_ids:
  - TASK-334
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on draft tree-support-family and traditional-support-family; TASK-331 blockers resolved (packet 220 implemented).
---

# Packet Contract: mixed-support-family-routing

## Goal
Implement deterministic host-owned routing cells and validation so mixed tree/traditional support plans merge only within family, reject cross-family collisions, and report degraded unmet demands without fallback geometry.

## Scope Boundaries
This packet owns host routing-cell construction, multi-family aggregation validation, rendered swept-path conflict handling, diagnostics, and mixed-family tests. It consumes the draft analysis, structural plan, family pairing, tree, traditional, and anchored-event contracts; it does not implement either family algorithm or closure evidence.

## Prerequisites and Blockers
- Depends on: draft `tree-support-family` (TASK-332), draft `traditional-support-family` (TASK-333), and their dependency `support-analysis-family-contracts` (TASK-331) — now IMPLEMENTED (2026-08-13, status: implemented, TASK-331 closed in docs/07).
- Unblocks: `support-family-orca-closure` (TASK-335).
- Activation blockers: [RESOLVED] TASK-331 exact-Z seam ownership (`ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs`); [RESOLVED] TASK-331 breaking-versus-additive WIT migration (breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`). The inherited blockers that kept this packet draft are resolved; the packet activates once TASK-332/333 land.

## Acceptance Criteria
- **AC-1. Given** deterministic candidates with mixed `family_id` assignments, **when** host routing runs, **then** each routing cell has stable object/region/demand ownership, same-family cells may union, and no cell has positive-area overlap with another family's cell. | `cargo test -p slicer-runtime --test support_family_routing -- routing_cells -- --exact`
- **AC-2. Given** tree and traditional planners receive one object, **when** host aggregation completes, **then** every `SupportPlanIR` entry has a family matching its assigned demand and preserves `support_demand_id`, `support_body_id`, object/region attribution, and semantic role. | `cargo test -p slicer-runtime --test support_family_routing -- family_attribution -- --exact`
- **AC-3. Given** same-family bodies serving multiple regions, **when** aggregation validates them, **then** the unioned body is retained with every source demand ID exactly once; a body crossing another family's routing cell is rejected. | `cargo test -p slicer-runtime --test support_family_routing -- same_family_union -- --exact`
- **AC-4. Given** a body with positive-area overlap against another family's complete body polygon, **when** plan validation runs, **then** both bodies are dropped and their demands are marked unmet with a structured diagnostic; boundary touching within tolerance is retained. | `cargo test -p slicer-runtime --test support_family_routing -- cross_family_body_overlap -- --exact`
- **AC-5. Given** validated family plans rendered for one anchored event, **when** the host commit hook checks swept paths, **then** conflicting cross-family support paths are dropped as complete bodies and diagnostics retain family, body, demand, and reason fields. | `cargo test -p slicer-runtime --test support_family_routing -- swept_path_overlap -- --exact`
- **AC-6. Given** a candidate declined for `declined-policy`, `no-route`, `blocked`, or `unsupported-mode`, **when** diagnostics are committed, **then** no fallback support path or anchored support event is emitted and the exact decline reason is serialized. | `cargo test -p slicer-runtime --test support_family_routing -- degraded_diagnostics -- --exact`

## Negative Test Cases
- **AC-N1. Given** a plan entry whose family is not selected for its source region or whose planner-renderer pair is mismatched, **when** host validation runs, **then** it returns a fatal pairing/attribution error before rendering and emits no `SupportIR` path. | `cargo test -p slicer-runtime --test support_family_routing -- mismatched_family_fatal -- --exact`
- **AC-N2. Given** a body outside its assigned routing cell or intersecting exact-Z model occupancy, **when** validation runs, **then** the complete cross-layer body is dropped, attached demands become unmet, and no clipped or fallback geometry is emitted. | `cargo test -p slicer-runtime --test support_family_routing -- invalid_body_degraded -- --exact`

## Verification
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test support_family_routing -- --exact`

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§7-9 and invariants 1-7, 13-14.
- `docs/adr/0059-support-families-and-anchored-entities.md` - delegated bounded summary for routing and ownership constraints.
- `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` - delegated bounded summaries.
- `docs/08_coordinate_system.md` - delegated summary for geometry units.

## Doc Impact Statement (Required)
- `docs/02_ir_schemas.md` mixed-family routing, diagnostics, and structured `SupportIR` sections - `rg -q 'support_demand_id' docs/02_ir_schemas.md`.
- `docs/04_host_scheduler.md` host aggregation and degraded commit sections - `rg -q 'routing cell' docs/04_host_scheduler.md`.
- `docs/19_visual_debug.md` family routing diagnostics section - `rg -q 'cross-family' docs/19_visual_debug.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` and `:1839` - distributed tree contacts and radius-aware collision behavior.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374`, `:2953`, `:3106` - traditional orchestration, downward base propagation, and obstacle trimming.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` - shared interface generation behavior.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
