# Task Map: wipe-tower-bed-exclude-area

**This packet emits the template's skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253–264), so the `docs/07_implementation_status.md` crosswalk is N-A. Implementation is recorded against wayfinder tickets 11 (`docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md`) and 40 (`docs/specs/orca-feature-gap/issues/40-author-packet-p33-extruder-nozzle-mmu-hardware-emitter.md` — the `start_end_points` return-to-queue ruling this revision folds in). Re-derive the absence of a TASK row at completion time rather than trusting this sentence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A | Step 1 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `modules/core-modules/print-validator/**` (new crate, manifest, `wit-guest/`, `src/`, tests) | `Print.cpp::Print::validate`, `PrintConfig.cpp::get_bed_excluded_area` | `M` | no WIT edit — `mesh-analysis` `run` + `slicer:common/host-services` used as they stand |
| N-A | Step 2 | `docs/04_host_scheduler.md` | root `Cargo.toml`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/Cargo.toml`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` | — | `M` | core-module count is a ledger fact — re-derive; `254b` may have moved it |
| N-A | Step 3 | — | `crates/slicer-runtime/tests/integration/{bed_exclusion_abort_tdd.rs, main.rs}` | `Print.cpp::Print::validate` (fatal semantics) | `M` | aggregator `mod` registration is mandatory — unregistered = false green |
| N-A | Step 4 | `docs/03_wit_and_manifest.md` | `modules/core-modules/wipe-tower/{wipe-tower.toml, src/lib.rs, tests/bed_bounds_tdd.rs}`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` | — | `S` | DIV-2: canonical never validates the tower; this port does |
| N-A | Step 5 | `docs/04_host_scheduler.md`, `docs/15_config_keys_reference.md` | those two docs | — | `S` | doc 15 is generated, never hand-edited |
| N-A | Step 6 | `docs/03_wit_and_manifest.md`, `docs/adr/0050-custom-gcode-architecture.md` | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (8 new schema keys, manifest-only) | `GCode.cpp::get_path_of_change_filament` (verbatim port), `PrintConfig.cpp` (`start_end_points` def) | `S` | ADR-0050 §2 conformance: the six `travel_point_*` values are manifest-declared keys, no computed-value insertion |
| N-A | Step 7 | — | `crates/slicer-runtime/src/travel_path.rs` (new), `crates/slicer-runtime/src/run.rs` | `GCode.cpp::get_path_of_change_filament` (verbatim port) | `M` | host computes + injects the six `travel_point_*` values, `slice_has_paint` pattern; DIV-3, DIV-4 |
| N-A | Step 8 | — | `crates/slicer-runtime/tests/integration/{travel_path_injection_tdd.rs, main.rs}` | `GCode.cpp::get_path_of_change_filament` (expected values) | `M` | AC-11 + AC-9 substitution arms; aggregator `mod` registration is mandatory — unregistered = false green |
| N-A | Step 9 | `docs/15_config_keys_reference.md` | that doc | — | `S` | doc 15 is generated, never hand-edited |

Aggregate: `L` (re-derive from `implementation-plan.md` at review time). No single step is L; split Step 2's registration before escalating any context band.

## Supersession

This packet directory replaces its own prior revisions in place (same number, same slug), authored before the map's Authoring rules 1–6 and before the ⚠ correction on the map's ticket-11 entry; the second revision folds in `start_end_points` per the map's ticket-40 ruling. No other packet directory is modified. It shares the `wipe-tower` manifest with `254a` / `254b` / `255`, the `machine-gcode-emit` manifest with packets 253 / 267 / P18-P20, and the core-module count with `254b`; none of those is superseded.
