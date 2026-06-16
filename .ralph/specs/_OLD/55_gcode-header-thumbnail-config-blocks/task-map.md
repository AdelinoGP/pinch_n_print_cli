# Task Map: 55_gcode-header-thumbnail-config-blocks

This packet introduces TWO new task IDs that are not yet in `docs/07_implementation_status.md`. The backlog mapping is therefore prospective — Step 7 of `implementation-plan.md` inserts these rows during the packet completion gate, via worker dispatch (never by loading the full backlog into the implementer's context).

This file is required because:

1. The packet spans more than one task ID (`TASK-184` and `TASK-185`).
2. Different OrcaSlicer references govern different steps.
3. The packet sits alongside (does NOT reopen or supersede) the closed `TASK-119` series that already established the live in-body comment contract; the bridge between "in-body comments done" and "envelope blocks missing" needs to be explicit so reviewers do not confuse this packet with a TASK-119 follow-up.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-184` | Step 1 | `docs/02_ir_schemas.md` (PrintMetadata/LayerCollectionIR sections) | `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (new) | none | `S` | TDD scaffolding for HEADER + width + CONFIG ACs and negatives. |
| `TASK-184` | Step 2 | `docs/03_wit_and_manifest.md` (config-schema section) | `crates/slicer-host/src/config_schema.rs` | none | `S` | Register `filament_diameter`, `filament_density`, `max_z_height` plus any missing width keys; no semantic change. |
| `TASK-184` | Step 3 | `docs/02_ir_schemas.md`, `docs/08_coordinate_system.md` | `crates/slicer-host/src/gcode_emit.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:2640-2710` (delegate FACT ×2) | `S` | HEADER_BLOCK emission with four required fields + filament order. |
| `TASK-184` | Step 4 | — | `crates/slicer-host/src/gcode_emit.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:2750-2765` (delegate FACT ×1) | `S` | Extrusion-width comments after HEADER_BLOCK_END; canonical key list grounded by FACT. |
| `TASK-185` | Step 5 | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/cli.rs`, `main.rs`, `gcode_emit.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp:100-135` (delegate FACT ×1) | `M` | `--thumbnail` flag + PNG-magic validation + Base64 chunking + THUMBNAIL_BLOCK emission. Largest single step. |
| `TASK-184` | Step 6 | `docs/02_ir_schemas.md` (ConfigView section) | `crates/slicer-host/src/gcode_emit.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:5590-5620` (delegate FACT ×1) | `S` | CONFIG_BLOCK at file tail; deterministic key-sorted iteration. |
| `TASK-184`, `TASK-185` | Step 7 | — | `docs/07_implementation_status.md` (via dispatch) | none | `S` | Pure-dispatch regression + backlog row insertion. |

Aggregate: `M`. Largest single step: Step 5 (`M`). No `L` cells. The packet is within budget for activation.

## Relationship to closed TASK-119 series

`TASK-119` / `TASK-119a-c` (all `[x]`) established the live, byte-correct OrcaSlicer comment contract for in-body comments (`;LAYER_CHANGE`, `;Z:`, `;HEIGHT:`, `;TYPE:`). They are unaffected by this packet — the new envelope blocks are emitted strictly outside the in-body region (HEADER + width + optional THUMBNAIL before the first in-body comment, CONFIG after the last motion line). The regression command `cargo test -p slicer-host --test orca_comment_contract_tdd` in Step 7 codifies that non-interference.

## Why no docs/07 update happens BEFORE Step 7

Spec-packet-generator deliberately does NOT pre-edit `docs/07_implementation_status.md`. Adding rows requires loading the backlog into context, and the file is large. The implementer inserts the rows once via worker dispatch in Step 7, immediately before the packet completion gate, when the work is actually `[~]` and there is no risk of stale or speculative rows.
