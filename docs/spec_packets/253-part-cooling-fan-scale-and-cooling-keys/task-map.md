# Task Map: 253-part-cooling-fan-scale-and-cooling-keys

## OrcaSlicer feature-gap queue packet P01 (wayfinder ticket 08)

Backlog ticket `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` owns the P01 key set (19 cooling keys; amended by ticket 99 to add the `fan_max_speed`/`fan_min_speed` reclassification). This packet has **no** `docs/07_implementation_status.md` task ID: the feature-gap queue's established pattern (packet 234a, re-affirmed by the docs/07 survey at authoring time — no TASK row exists for any P0x packet) records implementation against the wayfinder ticket, and the docs/07 crosswalk is therefore N-A. Re-derive that fact at completion time — this paragraph is a ledger statement frozen at authoring time. Rows summarize; authoritative step contracts live in `implementation-plan.md`.

## Crosswalk

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (queue packet P01) | Step 1 | ticket 02 evidence standard | `part-cooling/src/lib.rs` + `machine-gcode-emit/src/lib.rs` (the three canonical converters), new `tests/cooling_curve_parity_tdd.rs` | `GCodeWriter.cpp::set_fan`, `set_additional_fan`, `set_exhaust_fan` | S | baseline byte contract pinned; three exhaustive conversion tests |
| N-A (queue packet P01) | Step 2 | `05-asset-packet-list.md` P01 row | `part-cooling.toml`, `machine-gcode-emit.toml`, `tests/cooling_config_schema_tdd.rs` | `PrintConfig.cpp::PrintConfigDef` | S | 15 keys in part-cooling + 5 in machine-gcode-emit; no co-declaration; guest freshness gate |
| N-A (queue packet P01) | Step 3 | — | `part-cooling/src/lib.rs`, `tests/part_cooling_tdd.rs` | `GCodeWriter.cpp::set_fan` | S | percent normalization, output-neutral at defaults |
| N-A (queue packet P01) | Step 4 | ticket 08 (99 amendment) | `part-cooling/src/lib.rs` (curve port), `tests/cooling_curve_parity_tdd.rs` | `CoolingBuffer.cpp::apply_layer_cooldown` | M | role-fan precedence + -1 fallbacks; `fan_min_speed` first live read; the shared layer-time estimator lands here |
| N-A (queue packet P01) | Step 5 | — | `part-cooling/src/lib.rs` (threshold classifier, re-timer), curve test file | `GCode.cpp::check_overhang_fan`; `FanMover.cpp` | M | quartile-band mapping; re-timing gated |
| N-A (queue packet P01) | Step 6 | — | P2 channel; scheduler bounds test; `gcode_part_cooling_emission_tdd.rs` leakage arm | `GCodeWriter.cpp::set_additional_fan` | M | AC-7, AC-N1, AC-N2 |
| N-A (queue packet P01) | Step 7 | ADR-0052 | `part-cooling/src/lib.rs` (slowdown stage), new `tests/layer_slowdown_parity_tdd.rs` | `CoolingBuffer.cpp::calculate_layer_slowdown`, `CoolingBuffer.cpp::parse_layer_gcode` | M | AC-10, AC-11; first live reads of the three `slow_down_*` keys and `dont_slow_down_outer_wall` |
| N-A (queue packet P01) | Step 8 | — | `machine-gcode-emit/src/lib.rs`, new `tests/exhaust_and_chamber_emission_tdd.rs` | `GCode.cpp::_do_export`, `GCodeWriter.cpp::set_exhaust_fan`, `set_chamber_temperature`, `GCode.cpp::custom_gcode_sets_temperature` | S | AC-1b, AC-8, AC-8b, AC-8c; independent of Steps 3-7 |
| N-A (queue packet P01) | Step 9 | `docs/15_config_keys_reference.md` (regen) | docs regeneration + workspace gates | — | S | AC-9 greps; deviation sign-off rule if findings surface |

Copy costs from `implementation-plan.md`. Aggregate `M`; no row is `L`.

## Test-binary wiring notes (aggregators)

- `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs` — per-crate `tests/` file: compiles as its own `--test cooling_curve_parity_tdd` binary; no aggregator registration exists or is needed in a guest module crate (verified: `part-cooling/tests/` holds three sibling binaries today, none aggregated).
- `modules/core-modules/machine-gcode-emit/tests/exhaust_and_chamber_emission_tdd.rs` — same per-crate style; own binary.
- `modules/core-modules/part-cooling/tests/layer_slowdown_parity_tdd.rs` — same per-crate style; own binary.
- `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` — **verified at authoring time**: `crates/slicer-scheduler/tests/` is flat at its root (three sibling `*_tdd.rs` binaries; aggregation exists only in the `unit/` and `contract/` subdirs via `main.rs`), so the new file is its own `--test config_bounds_enforcement_tdd` binary — exactly what AC-N1 names; no registration step needed or permitted.
- `crates/slicer-runtime/tests/integration/` — **verified at authoring time**: aggregated by `tests/integration/main.rs` (which declares `mod gcode_part_cooling_emission_tdd;`), and `tests/contract/main.rs` declares `mod integrated_parity_part_cooling_tdd;`. The AC-N2 arm is appended to the existing file (already registered — no new registration needed), and the AC's `--test integration -- <filter>` / `--test contract -- <filter>` forms target the real binaries.