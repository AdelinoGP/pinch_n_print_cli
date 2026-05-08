# Task Map: paint-region-transport-widening

This file bridges the packet's implementation steps back to `docs/07_implementation_status.md`. The packet registers a new task ID (`TASK-130c`) at Step 1; the table below assumes that registration has happened.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-130c` (new) | Step 0 | none | none | none | S | Pure FACT dispatch — locks the four open Step-0 questions (paint-value-input variant existence, paint_order readers, ExPolygonView strategy, PaintValue::Custom necessity, docs/07 insertion line, wasm32 toolchain availability). Sufficient evidence: a Step-0 Notes addendum with one binary answer per question. |
| `TASK-130c` (new) | Step 1 | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | doc edits only | none | S | Registers TASK-130c row + adds DEV-025 mismatches 4 + 5 (open) + audit row update. Sufficient evidence: worker dispatch confirms inserted line snippets verbatim. |
| `TASK-130c` (new) | Step 2 | `docs/02_ir_schemas.md`, `docs/05_module_sdk.md` | `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` (NEW), `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` (NEW) | none | M | TDD anchor — every named AC test exists in RED state; predictable compile errors recorded. |
| `TASK-130c` (new) | Step 3 | `docs/05_module_sdk.md` | `crates/slicer-sdk/src/prepass_builders.rs` | none | S/M | SDK widening — `ExPolygonView` + `PaintRegionEntry.polygons` + typed `push_paint_region`. Three named SDK tests GREEN. |
| `TASK-130c` (new) | Step 4 | `docs/03_wit_and_manifest.md` | `wit/world-prepass.wit`, `wit/deps/ir-types.wit`, `crates/slicer-macros/src/lib.rs:1283-1314` | `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` (delegate; ExPolygon parity) | S | WIT widening — typed `paint-value-input` variant; canonical and inline byte-match. |
| `TASK-130c` (new) | Step 5 | `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs:1954-2045`, `crates/slicer-host/tests/dispatch_tdd.rs:5349-5441` | `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` (delegate; paint-region shape) | M | Host widening — drop `parse_value`; `harvest_paint_segmentation_ir` becomes 1:1 typed mapping; one direct-wiring test migrates. Four named host tests + dispatch test GREEN. |
| `TASK-130c` (new) | Step 6 | none | `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` | none | S/M | Canonical guest typed-emit migration. `cargo test -p paint-segmentation` GREEN. |
| `TASK-130c` (new) | Step 7 | none | `test-guests/prepass-guest.component.wasm` (rebuilt artifact) | none | S | Pre-built guest rebuild; IR-level paint-region regression sweep GREEN. |
| `TASK-130c` (new) | Step 8 | `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` (status flip), DEV log closures | `OrcaSlicerDocumented/generated_documentation/01_system_architecture.md:73-98` (delegate; cite as parity in DEV-025 closure note) | S | Acceptance ceremony — full AC sweep, DEV-025 mismatches 4 + 5 closed; status: implemented. |

## Aggregate

- Aggregate context cost: **M** (sum of step costs; no single step at L).
- TASK-130c is registered by Step 1; the docs/07 cell shows `(new)` to flag that it does not exist at packet authoring time.
- TASK-130, TASK-130a, and TASK-130b are **not** addressed by this packet — they belong to the follow-on Packet 43 (`43_macro-prepass-segmentation-output-drain`).
- DEV-025 closure is **partial** at the end of this packet: mismatches 4 + 5 close; mismatch 3 stays open until Packet 43.

## Why this packet exists as a separate slice

Packet 06 (`06_macro-prepass-segmentation-bridge`) closed mismatches 1 + 2 of DEV-025 and explicitly DEFERRED mismatch 3 (the macro-arm drain). When the deferred work was scoped for the present session, an architectural review surfaced two additional mismatches the original DEV-025 audit did not catch: the lossy `value: string` channel and the hole-blind SDK polygons. Closing mismatch 3 against the existing transport would have entrenched silent geometric corruption (hole-blind SupportEnforcer/Material/FuzzySkin) and silent value corruption (Custom semantics → `ToolIndex(0)`). This packet therefore widens the transport first; Packet 43 then closes mismatch 3 against the corrected transport. The split is "Plan A" from the design discussion that preceded packet generation; Plan B (single packet) was rejected on aggregate-cost grounds (would have hit L) and on review-clarity grounds (mixing transport refactor with macro-arm drain).
