# Task Map: 253-part-cooling-fan-scale-and-cooling-keys

## OrcaSlicer feature-gap queue packet P01 (wayfinder ticket 08)

Backlog ticket `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` owns the P01 key set (19 cooling keys; amended by ticket 99 to add the `fan_max_speed`/`fan_min_speed` reclassification). This packet has **no** `docs/07_implementation_status.md` task ID: the feature-gap queue's established pattern (packet 234a, re-affirmed by the docs/07 survey at authoring time — no TASK row exists for any P0x packet) records implementation against the wayfinder ticket, and the docs/07 crosswalk is therefore N-A. Re-derive that fact at completion time — this paragraph is a ledger statement frozen at authoring time. Rows summarize; authoritative step contracts live in `implementation-plan.md`.

## Crosswalk

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (queue packet P01) | Step 1 | ticket 02 evidence standard | `modules/core-modules/part-cooling/src/lib.rs` (`percent_to_fan_s` helper), new `tests/cooling_curve_parity_tdd.rs` | `GCodeWriter.cpp::set_fan` | S | baseline byte contract pinned; conversion exhaustive test |
| N-A (queue packet P01) | Step 2 | `05-asset-packet-list.md` P01 row | `part-cooling.toml`, `machine-gcode-emit.toml`, `tests/cooling_config_schema_tdd.rs` | `PrintConfig.cpp::PrintConfigDef` | S | 19 keys + co-declaration; guest freshness gate |
| N-A (queue packet P01) | Step 3 | — | `part-cooling/src/lib.rs`, `tests/part_cooling_tdd.rs` | `GCodeWriter.cpp::set_fan` | S | percent normalization, output-neutral at defaults |
| N-A (queue packet P01) | Step 4 | ticket 08 (99 amendment) | `part-cooling/src/lib.rs` (curve port), `tests/cooling_curve_parity_tdd.rs` | `CoolingBuffer.cpp::apply_layer_cooldown` | M | role-fan precedence + −1 fallbacks; `fan_min_speed` first live read |
| N-A (queue packet P01) | Step 5 | — | `part-cooling/src/lib.rs` (threshold classifier, re-timer), curve test file | `GCode.cpp::check_overhang_fan`; `FanMover.cpp` | M | quartile-band mapping; re-timing gated |
| N-A (queue packet P01) | Step 6 | — | P2 channel; `machine-gcode-emit/tests/cooling_placeholder_reachability_tdd.rs`; scheduler bounds test; `gcode_part_cooling_emission_tdd.rs` leakage arm | `_do_export` header/footer structure | M | two sequenced edit rounds per the step's cap |
| N-A (queue packet P01) | Step 7 | `docs/15_config_keys_reference.md` (regen) | docs regeneration + workspace gates | — | S | AC-9 greps; deviation sign-off rule if findings surface |

Copy costs from `implementation-plan.md`. Aggregate `M`; no row is `L`.

## S7 wiring notes (aggregators)

- `modules/core-modules/part-cooling/tests/cooling_curve_parity_tdd.rs` — per-crate `tests/` file: compiles as its own `--test cooling_curve_parity_tdd` binary; no aggregator registration exists or is needed in a guest module crate (verified: `part-cooling/tests/` holds three sibling binaries today, none aggregated).
- `modules/core-modules/machine-gcode-emit/tests/cooling_placeholder_reachability_tdd.rs` — same per-crate style; own binary.
- `crates/slicer-scheduler/tests/config_bounds_enforcement_tdd.rs` — **verified at authoring time**: `crates/slicer-scheduler/tests/` is flat at its root (three sibling `*_tdd.rs` binaries; aggregation exists only in the `unit/` and `contract/` subdirs via `main.rs`), so the new file is its own `--test config_bounds_enforcement_tdd` binary — exactly what AC-N1 names; no registration step needed or permitted.
- `crates/slicer-runtime/tests/integration/` — **verified at authoring time**: aggregated by `tests/integration/main.rs` (which declares `mod gcode_part_cooling_emission_tdd;`), and `tests/contract/main.rs` declares `mod integrated_parity_part_cooling_tdd;`. The AC-N2 arm is appended to the existing file (already registered — no new registration needed), and the AC's `--test integration -- <filter>` / `--test contract -- <filter>` forms target the real binaries.