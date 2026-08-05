---
status: implemented
packet: 116_support-modules-doc-honesty-cleanup
task_ids: []
---

# 116_support-modules-doc-honesty-cleanup

## Goal

Make the support-module documentation match the current implementations, remove unused bottom-interface state while preserving its config key and explicit deferred status, and document the existing speed-factor normalization only in the current support/infill consumers that define `BASE_SPEED`.

## Problem Statement

The support modules' lead `//!` doc blocks misdescribed their algorithms (tree-support claimed to be a TreeSupport port; support-planner's doc was stale), `SupportPlanner` carried dead `support_interface_bottom_layers` state (field + parse-and-store), and the `speed_factor = configured_speed / BASE_SPEED` normalization convention (`BASE_SPEED = 50.0`) was documented nowhere. Packet 116 fixes the docs and dead state WITHOUT emitting the D8 not-implemented warning — packet 118 owns the typed D11 warning channel (code 1003) and reads the preserved config key.

## Architecture Constraints

- Doc/state only: edits the lead `//!` blocks of `tree-support`, `traditional-support`, `support-planner`, `rectilinear-infill`; deletes the dead bottom-interface field/parse from `SupportPlanner`; adds a `# Not yet implemented — see docs/specs/support-modules-orca-port.md` comment adjacent to the preserved `[config.schema.support_interface_bottom_layers]` entry in `support-planner.toml`.
- No warning is emitted here: packet 118 owns the typed D11 diagnostic; AC-5/AC-N1 pin `! rg -q 'support_interface_bottom_layers is not yet implemented' modules/core-modules/support-planner/src/lib.rs`.
- `support-planner` is NOT a `BASE_SPEED` consumer in the current tree — its speed section is not fabricated (only tree-support, traditional-support, rectilinear-infill get `# Speed normalization` doc sections).
- No IR, WIT, scheduler, manifest, or real bottom-interface implementation changes.

## Data and Contract Notes

- B1: tree-support `//!` opens with "Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption" and states it is NOT a port of OrcaSlicer's TreeSupport.
- B2: traditional-support `//!` opens with "Per-layer rectilinear scan-line filler for Layer::Support" and "Depends entirely on upstream SliceRegionView::needs_support()".
- B3: support-planner `//!` opens with "Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes" and states its detect/contact/top-down-MST/emit shape is not numerical parity.
- D9: `speed_factor = configured_speed / BASE_SPEED`; `BASE_SPEED = 50.0` is the project-wide normalization base; downstream gcode-emit multiplies speed_factor through to feed rate.
- D11 boundary: packet 118 creates the typed `support_interface_bottom_layers` diagnostic itself; no packet-116 string-warning prerequisite.

## Locked Assumptions and Invariants

- The dead-field cleanup removes the struct field, struct-literal assignments, and the `config.get("support_interface_bottom_layers")` parse-and-store branch — but the config key stays in the TOML schema (user-facing surface unchanged).
- Source-plan TASK-250/251/252 collisions are NOT adopted; packet 116 intentionally assigns no replacement TASK-###.
- Unblocks shared-file review for packet 117; no semantic dependency on 118.

## Risks and Tradeoffs

- Doc drift risk only; all acceptance criteria are rg/awk greps over the lead doc blocks plus `cargo check`/`clippy`/`test` on the four modules.
- The preserved config key with deferred status is the honest surface: users can see the key, the toml comment says not-yet-implemented, and (per packet 118) a typed code-1003 warning fires when the value is not `-1`.

## Implementation Deviations (recorded at close)

None beyond scope. Doc Impact: `none` — only Rust/TOML comments and dead-state cleanup change.
