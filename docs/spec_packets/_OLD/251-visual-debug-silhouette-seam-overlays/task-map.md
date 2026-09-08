# Task Map: 251-visual-debug-silhouette-seam-overlays

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-455` | `Step 1`, `Step 3` | `docs/specs/visual-debug-silhouette-side-views-plan.md` (facts 5/11, D18, §7) | `crates/slicer-runtime/src/visual_debug_style.rs`, `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/slicer-runtime/src/lib.rs`, `crates/pnp-cli/src/visual_debug_gcode.rs` | none (no parity) | M | Proves the additive `z` (1.0/1.1 byte-stable) and the isolated/composited seam render forms with layer filtering and determinism |
| `TASK-456` | `Step 2`, `Step 4` | `docs/specs/visual-debug-silhouette-side-views-plan.md` (§5, §6 R9/R10) | `crates/pnp-cli/src/visual_debug.rs`, `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`, `crates/pnp-cli/tests/visual_debug_seam_overlay_tdd.rs` | none | M | Proves the full R9 fail-closed matrix, the 247/248 pin retirements, and the bundle/manifest wiring for both forms |
| `TASK-457` | `Step 5` | `docs/19_visual_debug.md` | `docs/19_visual_debug.md` | none | S | Proves the user-facing contract records both seam-overlay forms, the `z` mirror, and the exclusions |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
