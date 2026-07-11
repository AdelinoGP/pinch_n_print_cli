# Task Map: 131_per-region-config-delivery

Single-task-ID packet (`TASK-256`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `docs/07_implementation_status.md:229`.
FORWARD-DEP: packet 130 must be `status: implemented` before activation (serial WIT-churn
ordering; see `packet.spec.md` §Prerequisites and Blockers).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-256` | `Step 1` | `docs/specs/modifier-region-infill.md` §Phase M2 | `.ralph/specs/131_per-region-config-delivery/carve-list.md` (new) | none | S | Baseline capture (incl. wedge SHA-256 digest) + golden survey, before any code edit. |
| `TASK-256` | `Step 2` | `CLAUDE.md` §WIT/Type Changes Checklist | `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-sdk/src/views.rs`, `crates/slicer-macros/src/lib.rs` | none | M | Locked `config: func() -> config-view` accessor on both region views. |
| `TASK-256` | `Step 3` | `docs/adr/0030-modifier-splits-fill-not-perimeters.md` Decision point 3 | `crates/slicer-wasm-host/src/dispatch.rs` (first-match derivation retired) | none | M | The bug fix the backlog row names: `RegionKey`-matched resolution replaces first-match. |
| `TASK-256` | `Step 4` | none new | `crates/slicer-runtime/tests/contract/per_region_config_tdd.rs` (new), `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` (new SHA-256 test), carved test files | none | M | AC-1/AC-N1/AC-N2 close; carve markers applied per carve-list.md. |
| `TASK-256` | `Step 5` | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | the two docs (Doc Impact) | none | S | Doc Impact greps + gate ceremony; docs/07 row checked off via worker dispatch. |

Aggregate context cost: `M` (S + M + M + M + S). No step rated `L`.
