# Task Map: 58_gcode-toolchange-purge-integration

This file is the explicit bridge from packet steps to `docs/07_implementation_status.md` task IDs. It also names the prior packets whose integration gap this packet closes (no supersession — each prior packet stays `implemented`).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-143` (WipeTower live finalization) | Steps 1, 2, 4, 5, 6 | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/src/lib.rs`; new `#[cfg(test)] mod tests` block in same file | `WipeTower2.cpp:1557-1640`, `WipeTower2.cpp:2069-2205` | M | Wipe-tower module previously emitted geometry that never surfaced in the live G-code for multi-material fixtures; this packet wires the entities through `LayerCollectionIR` with `ExtrusionRole::WipeTower` and a `;TYPE:Wipe tower` marker. |
| `TASK-152b` (Mixed-tool ordering / push-tool-change) | Steps 2, 3, 5, 6 | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/gcode_emit.rs` (T<n> emission block at ~line 1155); new `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` | `Print.cpp:3180-3268`, `GCode.cpp:7624` | M | `push-tool-change` deposits `ToolChange` entries into `LayerCollectionIR` (packet 19); this packet ensures the emitter wraps each one rather than writing a bare `T<n>`. |
| `TASK-120d2` (Retract/unretract pair emission) | Steps 2, 3, 5, 6 | `docs/02_ir_schemas.md` | `crates/slicer-host/src/gcode_emit.rs` (missing-purge guard); `crates/slicer-host/src/postpass.rs::PostpassError::MissingToolchangePurge` additive variant | `WipeTower2.cpp:1619-1640` (Unload retract + Wipe rows) | S–M | Existing retract policy from packet 15 emits retracts for travel; this packet adds a toolchange-specific retract requirement enforced at emission via a defensive `MissingToolchangePurge` variant added additively to `PostpassError`. |

Aggregate context cost (sum of per-step costs): **M**. No step is L.

## Prior-packet relationships

This packet does **not** supersede any prior packet. The five prior packets each closed their declared scope:

- `17_wipe-tower-finalization-live-path` — wipe-tower module ported to `PostPass::LayerFinalization`. ✓ Implemented.
- `19_path-optimization-tool-order-and-cooling-policy` — mixed-tool ordering closed; `push-tool-change` WIT shipped. ✓ Implemented.
- `11_orca-gcode-emission-contract` — `;TYPE:` role labeling contract defined. ✓ Implemented.
- `15_live-travel-retraction-policy` — travel retract/no-retract decision in `path-optimization-default`. ✓ Implemented.
- `34_retraction-mode-firmware-vs-gcode` — `retract_mode` toggle (gcode/firmware). ✓ Implemented.

The integration between these closed packets is what packet 58 wires together. Per the cross-packet mutation rule, this packet does NOT modify files inside any of the prior packet directories; it only touches workspace source under `crates/`, `modules/`, `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`, and its own packet directory.

## Step → Acceptance Criterion coverage

| Step | Lands ACs | Lands NCs |
| --- | --- | --- |
| Step 1 | — (read-only; unblocks AC1, AC4, AC6 by confirming role variant) | — |
| Step 2 | (compile + failing scaffolding for AC1, AC3, NC1) | NC1 (scaffolded, failing) |
| Step 3 | — (clippy/check gate; AC4 role mapping ready) | NC1 (passes after guard added) |
| Step 4 | AC1, AC3, AC4, AC6 (all green after module emission) | — |
| Step 5 | AC2a, AC2b, AC5 (file-level awk/python) | NC2, NC3 |
| Step 6 | — (docs only; finalizes packet status) | — |

Every AC and NC traces to at least one step. No AC is implied; every assertion has a step that lands it.
