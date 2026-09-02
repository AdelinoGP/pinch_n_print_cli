# Task Map: top-surface-ironing-keys

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes earlier packet work. This queue packet emits the explicit no-task clause: `task_ids: []`; implementation is recorded against [21 - Author packet P14 - Quality / Ironing - top-surface-ironing](../specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md), and the `docs/07_implementation_status.md` crosswalk is N-A.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| - (queue packet, `task_ids: []`) | Steps 1-5 | `docs/15_config_keys_reference.md` (generated) | `modules/core-modules/top-surface-ironing/{top-surface-ironing.toml,Cargo.toml,src/lib.rs,tests/*}` + top-owned runtime fixtures + scheduler/runtime integration arms | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` / `PrintConfig.hpp` (`PrintRegionConfig` declarations) + `Fill/Fill.cpp` (`Layer::make_ironing`, `Fill::fill_surface`) + `tests/fff_print/test_fill.cpp` | S/M | Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; canonical owner correction keeps support-surface-ironing for P15; no `docs/07` task row exists. |
