# Task Map: [spec-slug]

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This file is required when the packet spans more than one task ID, reopens prior packet work, or supersedes an earlier packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-000` | `Step 1` | `docs/01_system_architecture.md` | `crates/...` | `OrcaSlicerDocumented/...` | `S | M` | Include why this step is sufficient evidence for the task ID. |

The `Context cost` column copies the per-step estimate from `implementation-plan.md`. If any cell is `L`, the packet must be split before activation. The aggregate (sum across rows) must be `S` or `M`.
