# Task Map: 250-visual-debug-silhouette-gcode-emit

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-452` | `Step 1`, `Step 2`, `Step 4` | `docs/specs/visual-debug-silhouette-side-views-plan.md` (fact 9, D11, D16) | `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/slicer-runtime/src/lib.rs`, `crates/pnp-cli/src/visual_debug_gcode.rs` | none (no parity) | M | Proves the corrected `Move.e` position-differencing inversion, shared width formula, Z-containment bucketing, and W4 at the renderer level |
| `TASK-453` | `Step 3`, `Step 5` | `docs/specs/visual-debug-silhouette-side-views-plan.md` (D10, §5, §6) | `crates/pnp-cli/src/visual_debug.rs`, `crates/pnp-cli/tests/visual_debug_gcode_emit_silhouette_tdd.rs`, `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` | none | M | Proves the emitter-config fidelity fix, schedule plumbing, tap lift, bundle shape, and the 247/249 pin retirements |
| `TASK-454` | `Step 6` | `docs/19_visual_debug.md` | `docs/19_visual_debug.md` | none | S | Proves the user-facing contract records Z-containment, W4, and the self-testability caveat |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
