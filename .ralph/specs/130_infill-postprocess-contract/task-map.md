# Task Map: 130_infill-postprocess-contract

Single-task-ID packet (`TASK-255`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-255` in `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-255` | `Step 1` | `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` §Amendment, `CLAUDE.md` §WIT/Type Changes Checklist | `crates/slicer-schema/wit/deps/{ir-types,world-layer/world-layer}.wit`, `crates/slicer-sdk/src/{views,traits,test_support/fixtures}.rs`, `crates/slicer-macros/src/lib.rs` | none | M | Lands the six fields + `prior-infill` param + 1.1.0 bump the backlog row names. |
| `TASK-255` | `Step 2` | ADR-0028 §Amendment (derivation rules) | `crates/slicer-wasm-host/src/{dispatch.rs,marshal/out.rs}`, `crates/slicer-runtime/src/region_partition.rs` (predicate hoist) | none | M | Host population: four polygons, `tool-index` precedence, `wall-source-region-id`. |
| `TASK-255` | `Step 3` | none new | ~30 constructors/matches on `PerimeterRegionView` (compiler-driven) | none | M | Blast-radius sweep; gates prove the contract change is workspace-complete. |
| `TASK-255` | `Step 4` | ADR-0028 §Amendment (AC semantics) | `crates/slicer-wasm-host/test-guests/<echo guest>/` (new), `crates/slicer-runtime/tests/contract/` (new tests + drift update) | none | M | The five contract tests are the row's closure evidence (AC-1…AC-5, AC-N1, AC-N2). |
| `TASK-255` | `Step 5` | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | the two docs (Doc Impact) | none | S | Doc Impact greps + gate ceremony; docs/07 row checked off via worker dispatch. |

Aggregate context cost: `M` (M + M + M + M + S). No step rated `L`.
