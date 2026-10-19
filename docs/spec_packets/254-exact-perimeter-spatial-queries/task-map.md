# Task Map: exact-perimeter-spatial-queries

This packet uses an explicit crosswalk because TASK-561 spans core geometry, both generators, runtime/WASM capture, xtask build policy, and acceptance evidence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-561` | `Steps 1-2` | `docs/08_coordinate_system.md`; `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-core/src/perimeter_spatial.rs`, `crates/slicer-core/build.rs` (check-cfg), `crates/slicer-core/tests/perimeter_spatial_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` | `M` | Establishes exact independent oracle, four domains, fallbacks, adversarial evidence, and the reserved-cfg declaration. |
| `TASK-561` | `Steps 3-6` | `docs/17_agent_debugging.md`; `docs/19_visual_debug.md` | Classic/Arachne module paths; `slicer-runtime` forwarding feature; native in-process capture test; `slicer-wasm-host` dispatch hook; runtime harness; self-baseline fixtures | `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` | `M` | Proves real per-region reuse, multi-pass behavior, native capture (AC-2), and separate native/WASM self-baselines (AC-5). |
| `TASK-561` | `Steps 7-9` | root `AGENTS.md`; `docs/07_implementation_status.md` bounded row; `docs/23_controlled_perimeter_builds.md` (new, Step 10) | `xtask/src/rustc_driver.rs`, `xtask/src/rustc_driver_tests.rs`, CLI/test/dist/build-guests private plumbing | None | `M` | Enforces exact compiler policy, cfg scope, profiles, cache identity, and freshness modes without public GuestSpec churn. |
| `TASK-561` | `Steps 10-12` | `docs/21_data_defaults_and_fixtures.md`; `docs/23_controlled_perimeter_builds.md`; confirmed design record | `resources/perimeter-acceptance/run-acceptance.ps1` (new, tracked), `resources/perimeter-acceptance/run_bench.ps1` (tracked verbatim copy), acceptance validator integration test, doc edits, focused gates | None | `M`/`S` | Provides reproducible ABBA/BAAB evidence and measured KEEP/DROP/inconclusive reporting; no auto-commit. |

All rows map to the single current backlog row `TASK-561`; no other task or packet is absorbed.
