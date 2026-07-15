# Task Map: 159-visual-debug-intermediate-renderer

This explicit crosswalk is retained because the user requires a task-map artifact even though the packet contains one task ID.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-269` | `Steps 1-4` | `docs/07_implementation_status.md`; `docs/specs/visual-pipeline-debug.md`; `docs/19_visual_debug.md`; `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md`; `docs/01_system_architecture.md`; `docs/11_operational_governance_and_acceptance_gate.md` | Packet-158 renderer-owned capture handoff; `crates/slicer-runtime/src/` intermediate renderer and PNG path; `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs` | `none` | `M` | Consumes packet 158's draft/forward capture contract and owns typed geometry, swept widths, overlays, shared viewport/fixed palette, and deterministic PNG output only. CLI parsing/lifecycle, scheduler capture, final G-code renderer, agent skill, and ordinary-slice overhead remain out of bounds. |
