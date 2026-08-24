# Task Map: 237-support-analysis-parity

Crosswalk for `docs/07_implementation_status.md`. Registration is **packet-owned closure
work**: Step 10 (TASK-362) appends these rows verbatim through a worker dispatch; nothing is
registered at authoring time. Re-derive the tail of docs/07 before appending — ledger facts
are re-derived at the point of use.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-353` | Step 1 | `docs/specs/support-families-anchored-entities-plan.md` §7 | `crates/slicer-core/tests/support_overhang_detection_tdd.rs` | `SupportMaterial.cpp` `detect_overhangs` | S | red-first sharp-tail + enforce-layer tests |
| `TASK-354` | Step 2 | `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` 5.3 | `crates/slicer-core/src/algos/overhang_annotation.rs` | `SupportMaterialInternal::remove_bridges_from_contacts` | S | bridge stage port; consuming behavior of 238a's key |
| `TASK-355` | Step 3 | `docs/02_ir_schemas.md` SupportAnalysisIR | `crates/slicer-ir/src/slice_ir.rs` + `overhang_annotation.rs` | `detect_overhangs` cantilever tail | M | additive schema minor bump (value derived from live constant) + blast radius |
| `TASK-356` | Step 4 | plan §12 div 5.2 | `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` | `detect_contacts` enforcer branch | M | composes with 236 per-region minting |
| `TASK-357` | Step 5 | `docs/02_ir_schemas.md` §IR 2 | `crates/slicer-core/src/algos/mesh_analysis.rs` | none | S | G-17 producer derivation |
| `TASK-358` | Step 6 | plan §12 G-17 bullet | `crates/slicer-sdk/src/views.rs` + wasm-host marshal legs | none | M | T9 both-legs guard; freshness gate |
| `TASK-359` | Step 7 | plan §6 invariant 15 | `support_analysis_producer.rs` + new integration test | none | M | consumer gating; E1 replacement asserts real signal |
| `TASK-360` | Step 8 | `docs/02_ir_schemas.md` | `docs/02_ir_schemas.md` only | none | S | Doc Impact greps |
| `TASK-361` | Step 9 | `AGENTS.md` Test Discipline | gates + `tmp/237-human-validation.md` | none | M | human-gate artifacts; sign-off stays pending |
| `TASK-362` | Step 10 | `docs/07_implementation_status.md` | registration + status flip post-sign-off | none | S | this crosswalk is the verbatim source |

Split before activation if any row is L or aggregate exceeds M. Aggregate is M; no row is L.
