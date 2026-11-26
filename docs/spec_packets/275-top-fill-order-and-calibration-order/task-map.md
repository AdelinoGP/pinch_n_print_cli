# Task Map: top-fill-order-and-calibration-order

This packet carries `task_ids: []`. Its backlog is the wayfinder map **Close the OrcaSlicer FFF feature gap** (`docs/specs/orca-feature-gap/map.md`), not a `docs/07_implementation_status.md` slice — the same arrangement as its sibling packet 264. The crosswalk below therefore maps **wayfinder ticket rows** to packet steps. It is emitted despite the single-source rule because the packet spans two backlog rows (ticket 33 and the two uninventoried fill-order keys) and carries a forward dependency on another packet, both of which the template names as explicit mapping needs.

| Backlog row | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Ticket 33 — `calib_flowrate_topinfill_special_order`, the calibration toolpath order | `Step 1`, `Step 5` | `docs/08_coordinate_system.md` | `crates/slicer-sdk/src/surface_fill_order.rs`, `modules/core-modules/archimedean-chords-infill/` | `OrcaSlicerDocumented/src/libslic3r/Fill/FillPlanePath.cpp` (`FillPlanePath::fill_surface`) | `S` + `M` | Step 1 ports the ordering kernel; Step 5 makes it reachable from the key. Neither alone covers the key under map Authoring rule 1. |
| Ticket 33 — the atomic-block half (`no_sort` + `can_reverse = false`) | `Step 2`, `Step 3`, `Step 4` | `docs/02_ir_schemas.md`, `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` | `crates/slicer-runtime/src/layer_executor.rs` (`remap_infill_order_locks_from`, `next_global_infill_tag`, `validate_infill_order_locks`), `crates/slicer-sdk/src/surface_fill_order.rs` (`lock_block`) | `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` (`Fill::fill_surface_extruded`), `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.hpp` (`set_reverse`) | `M` + `S` + `S` | Proves the port's `order_lock` reaches `solid_infill` at all. Steps 2–3 are ADR-0062 conformance, not an amendment. |
| Not previously inventoried — `top_surface_fill_order` / `bottom_surface_fill_order` | `Step 4`, `Step 5` | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `archimedean-chords-infill.toml`, `concentric-infill.toml`, `octagram-spiral-infill.toml` + their `src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp`, `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` (`restore_source_path_order`) | `S` + `M` | These keys are absent from `docs/ORCA_CONFIG_REFERENCE.md` and so from the map's 407-key queue. Filing them into that reference is wayfinder ticket 123's job, not this packet's. |
| ADR-0063 obligations incurred by emitting a lock | `Step 5` | `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` | `crates/slicer-runtime/tests/contract/solid_infill_order_lock_tdd.rs` (new, registered in `tests/contract/main.rs`) | none | included in `Step 5` `M` | AC-14. Locked paths become self-clipping and trigger the linker's swept-footprint carve; the carve must be a no-op against sparse infill or the packet stops for re-scoping. |
| Recorded divergence + docs + gates | `Step 6` | `docs/02_ir_schemas.md`, `docs/DEVIATION_LOG.md` | `docs/15_config_keys_reference.md` (generated) | `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` | `S` | The `DEV-###` is re-derived at that step, never frozen here (map rule on ledger facts). |

Costs are copied from `implementation-plan.md`'s roll-up. Aggregate is `M` with no `L` step, so no split is required before activation. Activation remains blocked on packet 264 reaching `status: implemented`.

## Forward dependency

| Symbol / artifact this packet consumes | Producer | Producer status at authoring | Name and shape reconciled? |
| --- | --- | --- | --- |
| `archimedean-chords-infill` module (crate, manifest, `claim:top-fill`) | packet 264 | `draft` — re-derive from `docs/spec_packets/264-top-bottom-surface-keys/packet.spec.md` at point of use | Yes — 264's AC-9 and AC-10 name `archimedean-chords-infill` with `holds = ["claim:top-fill", "claim:bottom-fill"]` |
| `concentric-infill` module | packet 264 | `draft` — re-derive | Yes — 264's AC-8 and AC-10 name it with the same claims |
| `octagram-spiral-infill` module | packet 264 | `draft` — re-derive | Yes — 264's AC-9 and AC-10 name it with the same claims |

No symbol here is treated as already shipped. Steps 1–3 have no dependency on 264 and may land first.
