# Task Map: top-surface-ironing-rev1

## Backlog Mapping

| Task ID | docs/07 row status | Packet step coverage |
| --- | --- | --- |
| `TASK-169` (NEW; will be inserted by Step 5) | not yet present | Steps 0, 0a (conditional), 1, 2, 2a, 3, 4, 5 |

## Predecessor Reconciliation

- This packet supersedes `.ralph/specs/38_top-surface-ironing/`.
- Predecessor used `TASK-168`, but `TASK-168` was already taken at `docs/07_implementation_status.md:81` for the closed packet 36-rev1 bridge-detector remediation. The predecessor's `task_ids: [TASK-168]` was a packet-authoring defect; this rev1 packet adopts a fresh `TASK-169` instead of editing the existing TASK-168 row.
- The existing `TASK-168` row at `docs/07_implementation_status.md:81` is **not** edited by this packet. It belongs to packet 36-rev1's closed bridge-detector work.
- Step 5 inserts a fresh `TASK-169` row referencing this packet (`38-rev1_top-surface-ironing`) and noting that the rev1 implementation supersedes the failed packet 38 attempt.
- Predecessor packet's `packet.spec.md` frontmatter is updated by the planner during packet authoring (planner action, not implementer action) to:
  - `status: superseded`
  - `superseded_by: 38-rev1_top-surface-ironing`
- Predecessor's source files in `modules/core-modules/top-surface-ironing/` are NOT reverted before this packet's implementation pass. The implementer rewrites them in place per Steps 1, 2, 2a, 3.

## docs/07 Edit Plan

- Add ONE new row for `TASK-169` describing: "Implement packet 38-rev1 top-surface-ironing: object-scope ironing module at PostPass::LayerFinalization. Closes the predecessor packet 38_top-surface-ironing (which placed the module at the wrong stage Layer::InfillPostProcess and failed acceptance)."
- Status flag set when Step 5 acceptance ceremony PASSES.
- DO NOT edit, modify, or annotate the existing `TASK-168` row at line 81 — it belongs to a different (closed) packet.

## Authoritative Docs Per Step

| Step | Primary docs | Notes |
| --- | --- | --- |
| Step 0 | none directly | Discovery dispatches only. |
| Step 0a | `docs/05_module_sdk.md` | FinalizationOutputBuilder API extension. Conditional. |
| Step 1 | `docs/05_module_sdk.md`, `docs/02_ir_schemas.md` | Test fixture pattern. |
| Step 2 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | Manifest schema, trait pattern. |
| Step 2a | `docs/03_wit_and_manifest.md` | wit-guest cdylib pattern. |
| Step 3 | `docs/05_module_sdk.md`, `docs/02_ir_schemas.md`, `docs/08_coordinate_system.md` | Implementation. Coordinate system reminder: 1 unit = 100 nm. |
| Step 4 | `docs/03_wit_and_manifest.md` | Only if `placeholder_wasm` is a real schema field per Step 0 (d). |
| Step 5 | `docs/07_implementation_status.md` | Backlog row insertion. |

## OrcaSlicer Refs Per Step

| Step | Refs | Notes |
| --- | --- | --- |
| Step 0 | (none) | Step 0 dispatches are codebase-internal. |
| Step 3 | `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` | Algorithm parity (already SUMMARY'd by predecessor). |
| Step 5 | `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp::ironing()` | FACT only — confirms phase-order claim made by this packet. |

All OrcaSlicer reads MUST be delegated.
