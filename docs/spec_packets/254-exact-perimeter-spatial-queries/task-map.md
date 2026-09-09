# Task Map: exact-perimeter-spatial-queries

This packet uses an explicit crosswalk because TASK-561 spans core geometry, both generators, runtime/WASM capture, xtask build policy, and acceptance evidence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-561` | `Steps 1-2` | `docs/08_coordinate_system.md`; `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-core/src/perimeter_spatial.rs`; core tests | `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` | `M` | Establishes exact independent oracle, four domains, fallbacks, and adversarial evidence. |
| `TASK-561` | `Steps 3-4` | `docs/17_agent_debugging.md`; `docs/19_visual_debug.md` | Classic/Arachne module paths; runtime harness; WASM dispatch; parity integration | `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` | `M` | Proves real per-region reuse, multi-pass behavior, capture, and separate native/WASM baselines. |
| `TASK-561` | `Steps 5-6` | root `AGENTS.md`; `docs/07_implementation_status.md` bounded row; `docs/22_controlled_perimeter_builds.md` (new, Step 7) | `xtask/src/rustc_driver.rs`, CLI/test/dist/build-guests private plumbing | None | `M` | Enforces exact compiler policy, cfg scope, profiles, cache identity, and freshness modes without public GuestSpec churn. |
| `TASK-561` | `Steps 7-8` | `docs/21_data_defaults_and_fixtures.md`; `docs/22_controlled_perimeter_builds.md`; confirmed design record | `tmp/perimeter-acceptance/run-acceptance.ps1` (new), acceptance validator integration test, committed provenance fixtures, focused gates | None | `M`/`S` | Provides reproducible ABBA/BAAB evidence and measured KEEP/DROP/inconclusive reporting; no auto-commit. |

All rows map to the single current backlog row `TASK-561`; no other task or packet is absorbed.
