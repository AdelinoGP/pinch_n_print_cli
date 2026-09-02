# Preflight Report: top-surface-ironing-keys

Reviewed: 2026-09-01 · Mode: `--preflight`

| Check | Result | Evidence |
| --- | --- | --- |
| S0 Packet structure | PASS | All five contract files are non-empty. |
| S1 Prerequisite-status truth | PASS | Map issues 05, 06, 07, and 106 are resolved; no implemented packet dependency is claimed. |
| S2 Deviation-ID conformance | PASS | No deviation ID is created, closed, superseded, or grepped. |
| S3 Schema-version computed | PASS | The packet excludes IR/WIT/schema-version changes and pins no future version. |
| S4 ADR slot allocation | PASS | No ADR is authored or forward-referenced. |
| S5 Shipped-symbol existence/shape | PASS | Existing top-module, `SliceRegionView`, offset, output-builder, bounds, serializer, and padding symbols resolve with the stated shapes. |
| S6 WIT/IR identifier drift | PASS | No new WIT/IR identifier is assumed; existing `SliceRegionView` accessors and `ExtrusionRole::Ironing` resolve. |
| S7 Test-target wiring | PASS | The new module guard is auto-discovered; scheduler uses `scheduler_integration`; runtime `contract`, `executor`, `e2e`, and `integration` aggregators register the named tests. |
| S8 ADR conformance | PASS | The offset design uses the sanctioned `slicer_sdk::host::offset_polygons` seam and does not contradict an existing ADR. |
| AC runnable commands | PASS | All package/test targets, filters, xtask commands, and tee/grep forms resolve. |
| Doc Impact Statement | PASS | Generated `docs/15_config_keys_reference.md` is identified with regeneration and verification probes. |

**Verdict:** `PREFLIGHT PASS` (0 blockers, 0 highs)
