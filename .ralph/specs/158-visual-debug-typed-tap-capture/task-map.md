# Task Map: 158-visual-debug-typed-tap-capture

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-268` | `Steps 1-4` | `docs/07_implementation_status.md`; `docs/specs/visual-pipeline-debug.md`; `docs/19_visual_debug.md`; `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md`; `docs/01_system_architecture.md`; `docs/09_progress_events.md` | `crates/pnp-cli/` minimal command-to-runtime dispatch seam; `crates/slicer-runtime/src/`; `crates/slicer-runtime/tests/visual_debug_typed_tap_capture_tdd.rs`; packet-157 request/manifest integration seam | `none` | `M` | Adds request-gated typed post-stage capture and dependency-closure execution plus the minimal CLI dispatch wiring; packet 157 retains parsing/validation/bundle-model ownership, while packet 158 does not change WIT, IR, module, rendering, G-code, or agent contracts. |
