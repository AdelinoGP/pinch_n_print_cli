# Task Map: 254-prime-tower-keys-wipe-tower

## OrcaSlicer feature-gap queue packet P02 (wayfinder ticket 09)

Backlog ticket `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` owns the P02 key set (13 prime-tower keys, Tier A, owner `wipe-tower`; membership from `05-asset-packet-list.md`). This packet has **no** `docs/07_implementation_status.md` task ID: the feature-gap queue's established pattern (packets 234a and 253 — re-derive at completion time, this paragraph is a ledger statement frozen at authoring time) records implementation against the wayfinder ticket, and the docs/07 crosswalk is therefore N-A. Rows summarize; authoritative step contracts live in `implementation-plan.md`.

## Crosswalk

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (queue packet P02) | Step 1 | `05-asset-packet-list.md` P02 row | `modules/core-modules/wipe-tower/wipe-tower.toml` (+13 entries) | `PrintConfig.cpp::PrintConfigDef` | S | Orca-parity defaults/bounds; guest freshness gate fires |
| N-A (queue packet P02) | Step 2 | ticket 02 evidence standard | new `tests/wipe_tower_config_schema_tdd.rs` + `Cargo.toml` dev-dep | `PrintConfig.cpp::PrintConfigDef` | S | 21-key contract pinned |
| N-A (queue packet P02) | Step 3 | ticket 09 | `modules/core-modules/wipe-tower/src/lib.rs` (`from_config` + `generate_purge_paths`), `tests/wipe_tower_tdd.rs` fallout | `WipeTower.cpp` ctor + `align_perimeter` | M | the one live wiring; pitch change at defaults owned here |
| N-A (queue packet P02) | Step 4 | ticket 02 plumbing standard | `crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` (new), one `slicer-runtime` leakage arm | — (scheduler behavior is in-tree fact) | M | zero scheduler production edits |
| N-A (queue packet P02) | Step 5 | AC-4 greps | docs regen + workspace gates | — | S | no deviation rows expected |

Copy costs from `implementation-plan.md`. Aggregate `M`; no row is `L`.

## S7 wiring notes (aggregators)

- `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` — per-crate `tests/` file: compiles as its own `--test wipe_tower_config_schema_tdd` binary; wipe-tower's tests dir holds four sibling binaries today, none aggregated (same verified shape as packet 253's part-cooling note).
- `crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` — flat file at `crates/slicer-scheduler/tests/` root: auto-discovered as its own binary. The crate's explicit `[[test]]` entries exist for the bucket binaries and the two region-split files **to give unique binary names** in shared `target/debug/deps/`; a flat file with a unique name needs no registration.
- `crates/slicer-runtime/tests/integration/` — aggregated by `tests/integration/main.rs`; AC-N2's arm is appended to the existing leakage file found by grep (`from_declared`) and is **already registered** — the `--test integration -- <filter>` form targets the real binary.