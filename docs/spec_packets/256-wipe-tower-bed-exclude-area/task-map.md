# Task Map: wipe-tower-bed-exclude-area

**This packet emits the template's skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253–264), so the `docs/07_implementation_status.md` crosswalk is N-A. Implementation is recorded against wayfinder ticket 11 (`docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md`). Re-derive the absence of a TASK row at completion time rather than trusting this sentence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A | Step 1 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `modules/core-modules/print-validator/**` (new crate, manifest, `wit-guest/`, `src/`, tests) | `Print.cpp::Print::validate`, `PrintConfig.cpp::get_bed_excluded_area` | `M` | no WIT edit — `mesh-analysis` `run` + `slicer:common/host-services` used as they stand |
| N-A | Step 2 | `docs/04_host_scheduler.md` | root `Cargo.toml`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/Cargo.toml`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` | — | `M` | core-module count is a ledger fact — re-derive; `254b` may have moved it |
| N-A | Step 3 | — | `crates/slicer-runtime/tests/integration/{bed_exclusion_abort_tdd.rs, main.rs}` | `Print.cpp::Print::validate` (fatal semantics) | `M` | aggregator `mod` registration is mandatory — unregistered = false green |
| N-A | Step 4 | `docs/03_wit_and_manifest.md` | `modules/core-modules/wipe-tower/{wipe-tower.toml, src/lib.rs, tests/bed_bounds_tdd.rs}`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` | — | `S` | DIV-2: canonical never validates the tower; this port does |
| N-A | Step 5 | `docs/04_host_scheduler.md`, `docs/15_config_keys_reference.md` | those two docs | — | `S` | doc 15 is generated, never hand-edited |

Aggregate: `L` (re-derive from `implementation-plan.md` at review time). No single step is L; split Step 2's registration before escalating any context band.

## Supersession

This packet directory replaces its own prior revision in place (same number, same slug), authored before the map's Authoring rules 1–6 and before the ⚠ correction on the map's ticket-11 entry. No other packet directory is modified. It shares the `wipe-tower` manifest with `254a` / `254b` / `255` and the core-module count with `254b`; none of those is superseded.
