# Task Map: scope-eligibility

This explicit queue crosswalk is retained because TASK-568 consumes packet-05 forward exports and unblocks two dependent packets.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-568` | `Steps 1–7` | `docs/specs/config-scope-resolution-plan.md`, ADR-0069 | host/module declarations, registry admission, unified resolver, manifest model/parser, 24 manifests, tests/docs | None | `M` | Proves exact authored denials, independent drift derivation, loud rejection, and legacy allow-list retirement. |
