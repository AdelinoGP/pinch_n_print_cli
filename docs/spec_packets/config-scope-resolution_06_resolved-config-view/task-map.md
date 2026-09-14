# Task Map: resolved-config-view

This explicit queue crosswalk is retained because TASK-567 consumes forward exports from packets 03 and 05 and owns a staged warn-to-drop gate.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-567` | `Steps 1–7` | `docs/specs/config-scope-resolution-plan.md`, ADR-0067, ADR-0068 | registry projection, binding, 13 guests, ingestion, G-code emission, focused tests/docs | None | `M` | Proves always-resolved views, exact 89-site cleanup, no-drop gate, and registry-driven emission. |
