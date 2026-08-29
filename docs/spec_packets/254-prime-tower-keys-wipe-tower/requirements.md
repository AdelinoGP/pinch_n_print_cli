# Requirements: 254-prime-tower-keys-wipe-tower

## Packet Metadata

- Grouped task IDs: none — the feature-gap queue's established pattern is `task_ids: []` (packets 234a and 253 precedent); `docs/07_implementation_status.md` holds no TASK row for this queue.
- Backlog source: `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P02).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower is 13 keys that OrcaSlicer reads inside `WipeTower.cpp`/`WipeTower2.cpp`/`GCode.cpp`/`Print.cpp`, and Pinch 'n Print implements none of them:

- The `wipe-tower` module declares 8 keys today and none of the 13 P02 keys exists anywhere in code — not in a manifest, not in `ResolvedConfig`, not read by any Rust symbol (survey at authoring time; `prime_tower_brim_width` appears once as a static padding literal in `crates/slicer-gcode/src/serialize.rs`'s `ORCA_CONFIG_PADDING`, mentioned but never consumed).
- Canonically, one of the 13 drives tower geometry arithmetic that this port already performs differently: `prime_tower_infill_gap` (percent, default 150) sets the tower infill scan-line pitch to `(value/100) × m_perimeter_width` (canonical `WipeTower` ctor + `align_perimeter` + the wipe-path `dy` sites). This port's purge generator advances a hardcoded `y += line_width` (`generate_purge_paths`, `modules/core-modules/wipe-tower/src/lib.rs`), so the port emits denser tower lines than Orca at Orca's own default.
- The remaining 12 keys gate canonical behaviours this port's simplified rectangular scan-line tower does not have: the per-filament interface-feature cluster (`WipeTower2::set_extruder` + `toolchange_ChangeExtruder`), the ramming unload sequence (`toolchange_Unload`), the full-height framework walls (ctor `m_tower_framework` → `generate_wipe_tower_blocks`), the first-layer brim with its −1 "Auto" height resolution (`get_auto_brim_by_height`, first-layer loop emission in `WipeTower2.cpp`), the flat-ironing surface passes (effective only with gap-wall mode), and the travel-avoid skip points (`GCode.cpp::_do_export`, `compute_wall_skip_points` — note the config key is a **bool** despite its name; the points themselves are internal `Vec2f` state, not user data).
- Six of the 13 are canonically per-filament vectors (`coFloats`/`coInts`). This tree has no per-filament config model (the map's Tier-D fog, `map.md` §Not yet specified); the queue's established ruling (ticket 04: 11 filament keys are global, not per-filament) makes scalar-globals the honest interim representation, not a new subsystem.

The slice is coherent because all 13 keys share one owner (`wipe-tower`), one manifest, and one wiring decision each: wire the key whose decision point exists, declare + emit the rest with recorded gaps.

## In Scope

All 13 P02 keys (membership from `05-asset-packet-list.md`; per-key Tier A rows in `04-asset-tier-assignment.md`):

1. **Manifest declarations:** the 13 keys in `modules/core-modules/wipe-tower/wipe-tower.toml` with Orca defaults, types, bounds, and display/group metadata — six bools (`enable_filament_ramming` true, `enable_tower_interface_cooldown_during_tower` false, `enable_tower_interface_features` false, `prime_tower_enable_framework` false, `prime_tower_flat_ironing` false, `prime_tower_skip_points` true), four floats with bounds (`prime_tower_brim_width` 3.0 min −1 keeping the Auto sentinel; `filament_tower_interface_pre_extrusion_dist` 10.0 and `filament_tower_interface_purge_volume` 20.0 and `filament_tower_ironing_area` 4.0 min 0; `filament_tower_interface_pre_extrusion_length` 0.0 min 0), one int (`filament_tower_interface_print_temp` −1, min −1: "use max nozzle temp" sentinel), one percent (`prime_tower_infill_gap` `"150%"`, min 100).
2. **The one live wiring — `prime_tower_infill_gap`:** read in `WipeTower::from_config` as `ConfigValue::Percent`, scan-line advance becomes `(value/100) × line_width` — the port's `line_width` standing in for canonical `m_perimeter_width` (a divergence recorded in `design.md`: canonical pitches off nozzle-derived perimeter width, the port off its existing `line_width` field; the port has no nozzle-diameter-perimeter-width pipeline at this stage). This changes default-path output (pitch 0.4 → 0.6 mm at the default 0.4 line width) and owns its test fallout (AC-2).
3. **Emission-surface reachability for user-set values:** any of the 13 keys supplied by a profile rides the existing extensions bucket (`resolve_global_config`'s `apply_cli_key`/`extensions` fallback in `crates/slicer-scheduler/src/config_resolution.rs`) into the G-code CONFIG_BLOCK — so Orca 3MF/preset values for all 13 keys round-trip visibly. The percent key's schema default additionally threads via the packet-185 percent transport (AC-3).
4. **Recorded decision-point gaps:** the 12 keys without a live consumer carry per-key gap notes in `design.md` (what the canonical decision point is, file + function; why this port lacks it; what building it would entail at packet-queue level). None is silently dropped.
5. **Scalar-globals disposition (per-filament keys):** the six `coFloats`/`coInts` keys are declared as scalar globals with a per-key note; the per-filament model question stays with the map's Tier-D fog (P03/P40's filament-for-features tickets inherit it).
6. **Docs + guest freshness:** `docs/15_config_keys_reference.md` regenerated; guest WASM rebuilt (`cargo xtask build-guests`).

## Out of Scope

- **Building the absent decision points** — the interface-feature tower (`WipeTower2`-style toolchange with per-filament interface parameters), the ramming unload sequence, framework walls, first-layer brim, flat-ironing passes, and travel-avoid skip-point logic are Tier C "new granular geometry" work (04's rubric), not Tier A plumbing. Future packets.
- **`prime_volume` migration** — the port's existing purge-volume key (default 45.0 after ticket 100's realignment) stays; canonical's interface-purge key `filament_tower_interface_purge_volume` (default 20.0) is a *different* parameter consumed inside the absent interface-feature cluster. Collapsing them is future work if the interface tower ever lands.
- **Per-filament config model** — no new subsystem; the six vector keys go scalar-global (see In Scope 5), deferring to the map's Tier-D fog.
- **`ORCA_CONFIG_PADDING` changes** — the host-side padding table in `crates/slicer-gcode/src/serialize.rs` is untouched; it is not manifest-derived, and once the keys are declared and user-set values ride the extensions bucket, the padding entry for `prime_tower_brim_width` remains harmless (padding only fills absent keys).
- Baseline byte-identicality for the wipe-tower geometry at defaults — **not preserved** by this packet (AC-2 changes the pitch at defaults; that is the point of the key). Self-captured-baseline suites pinning old pitch values are updated in the same step, never weakened elsewhere.

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` — 23 lines; direct read.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P02 row at `### P02 — Multimaterial / Prime tower (1/2) — wipe-tower`; ranged read ~10 lines.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the 13 Tier A rows (`enable_filament_ramming` … `prime_tower_skip_points`); ranged read ~15 lines. Over 300 lines total: delegate beyond these rows.
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — ~80 lines; direct read.
- `docs/15_config_keys_reference.md` — large; regeneration + grep verification only, never read in full.

## Verified Grounding

All claims below were verified against the tree and the canonical checkout at authoring time (2026-08-28):

- **Owner manifest:** `modules/core-modules/wipe-tower/wipe-tower.toml` declares exactly 8 keys today (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`), each read by `WipeTower::from_config` (`config.get(...)` match arms, `modules/core-modules/wipe-tower/src/lib.rs`).
- **Scan-line advance:** `generate_purge_paths` computes `y += line_width` (module `src/lib.rs`, line ~385 region; `E = length × line_width × flow; length = purge_volume / (line_width × layer_height)` at ~392). The only pitch input today is `line_width`.
- **Read/bounds/transport machinery:** module reads via `ConfigView` (`ConfigValue::Percent` is a live variant — `serialize.rs` matches it at `crates/slicer-gcode/src/serialize.rs`); percent-typed manifest defaults are parsed into `parsed_default` (`crates/slicer-scheduler/src/manifest.rs`, `read_config_schema` full-table path) and threaded into `ResolvedConfig.extensions` by `resolve_global_config` via `ConfigBoundsIndex::schema_defaults()` (`crates/slicer-scheduler/src/config_resolution.rs`: `for (key, default) in bounds.schema_defaults()`); *non-percent* declared defaults (bool/float/int) are **not** threaded — the module-side read fallback is their runtime home, mirroring `retract_length`'s pattern. User-supplied values of any declared key reach `extensions` via `apply_cli_key`'s `Ok(false)` arm → `cfg.extensions.insert` (three sites in `config_resolution.rs`).
- **Bounds enforcement + enum check live in** `ConfigBoundsIndex::check` (`config_resolution.rs`: enum-membership for `String` values, numeric min/max with per-element list reporting). Numeric bounds declarations are collected only for numeric field types with at least one bound (`is_numeric_field_type` + the `from_modules` filter that skips `min.is_none() && max.is_none()`).
- **Mirroring fixtures:** `percent_schema_bounds` + `percent_round_trip` + `percent_profile_value_overrides_schema_default` (`crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`) build `ConfigFieldEntry` fixtures in memory via `LoadedModuleBuilder::new(...).config_schema(ConfigSchema{entries}).build()` + `ConfigBoundsIndex::from_modules([&module])`; the percent bounds arms (`percent_bounds_rejects_…`) live in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`. New scheduler test files are separate `--test` binaries (crates/slicer-scheduler `[[test]]` entries are explicit, but flat `tests/*.rs` files auto-discover — packet 253 verified the same for its own new binary).
- **Modules-see-only-their-keys:** `ConfigView::from_declared` (read via `config.get` in `from_config`); the leakage arm pattern mirrors packet 253's AC-N2 (`slicer-runtime --test integration`).
- **No pre-existing collisions:** none of the 13 keys is declared in any other manifest or read anywhere in Rust; the single stray literal is the `ORCA_CONFIG_PADDING` entry in `crates/slicer-gcode/src/serialize.rs` (padding-only, key-deduped, untouched by this packet).
- **`m_perimeter_width` basis (canonical):** `WipeTower.cpp` ctor — `m_perimeter_width = nozzle_diameter * Width_To_Nozzle_Ratio`; scan-line pitch sites `align_perimeter` (`float spacing = m_extra_spacing * m_perimeter_width;`) and the wipe-path `dy` sites. `m_extra_spacing` is initialized from `config.prime_tower_infill_gap.value/100` in the ctor and *re-fitted* at runtime to meet tower depth — the port takes the initial value only; depth-refitting is part of the absent canonical tower planner and is out of scope (noted in the AC-2 gap row).

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

### Per-key parity evidence (ticket 02 standard)

All 13 keys were read in canonical `PrintConfig.cpp::PrintConfigDef` (type, default, bounds) and their consumers located. None is dead-in-canonical — every key has at least one reader (`Print.cpp::validate` additionally lists all 13 as re-slice-invalidating).

| Key | Canonical def | Canonical consumer (file + function) | Behaviour in canonical | Disposition here |
| --- | --- | --- | --- | --- |
| `enable_filament_ramming` | coBool, true | `WipeTower2.cpp::toolchange_Unload` | master switch for the staged ramming unload/load sequence ("Ramming start", 4-step retraction, cooling moves; MMU-gated) | declared + emitted; **decision-point gap**: no unload sequence in the scan-line tower |
| `enable_tower_interface_cooldown_during_tower` | coBool, false | `WipeTower2.cpp::set_extruder` (two read sites) | temp-boost cooldown during tower printing vs at toolchange | declared + emitted; **gap**: no interface-temp machinery |
| `enable_tower_interface_features` | coBool, false | `WipeTower.cpp` ctor (`m_use_gap_wall` cluster); `toolchange_ChangeExtruder` | enables the interface-feature tower (interface body between layers) | declared + emitted; **gap**: interface tower absent |
| `filament_tower_interface_pre_extrusion_dist` | coFloats, 10, min 0 | `WipeTower.cpp` ctor → per-filament `m_filpar` | pre-extrusion travel distance before interface extrusion | declared (scalar-global) + emitted; **gap** |
| `filament_tower_interface_pre_extrusion_length` | coFloats, 0, min 0 | same path | pre-extrusion length at interface | declared (scalar-global) + emitted; **gap** |
| `filament_tower_interface_print_temp` | coInts, −1, min −1 | same path; `set_extruder` temp boost | −1 = use max nozzle temp | declared (scalar-global) + emitted; **gap** |
| `filament_tower_interface_purge_volume` | coFloats, 20, min 0 | same path | purge volume for interface extrusion (distinct from `prime_volume`) | declared (scalar-global) + emitted; **gap** |
| `filament_tower_ironing_area` | coFloats, 4, min 0 | same path | mm² ironing area fraction on interface layers | declared (scalar-global) + emitted; **gap** |
| `prime_tower_brim_width` | coFloat, 3, min −1 (f_enum_open "Auto") | `WipeTower2.cpp` first-layer block: `loops_num = (brim_width + spacing/2)/spacing`, `offset(poly, scale_(spacing))`, `writer.extrude`; `Print.cpp::plan_tower_new` resolves Auto via `get_auto_brim_by_height` (min(max_height/100×8, 8) mm) | first-layer brim rings around tower; −1 = auto by tower height | declared + emitted; **gap**: no brim geometry, no height planner |
| `prime_tower_enable_framework` | coBool, false | `WipeTower.cpp` ctor (`m_tower_framework`) → `generate_wipe_tower_blocks` forces every layer's depth to the first layer's | uniform full-height framework wall vs per-layer depth | declared + emitted; **gap**: no per-layer depth model at all |
| `prime_tower_flat_ironing` | coBool, false | ctor (`m_flat_ironing = m_flat_ironing && m_use_gap_wall`); `toolchange_ChangeExtruder` (`should_flat_ironging`) | flat-ironing passes over tower surface, only when gap-wall mode on | declared + emitted; **gap** (also conditional-on-gap-wall in canonical) |
| `prime_tower_infill_gap` | coPercent, 150, min 100 | ctor: `m_extra_spacing = value/100`; `align_perimeter`: `spacing = m_extra_spacing × m_perimeter_width`; wipe-path `dy` sites | infill scan-line pitch = (gap/100) × perimeter width; value refitted at runtime for depth fit | **WIRED** to scan-line advance `(value/100) × line_width`; depth-refitting explicitly out of scope |
| `prime_tower_skip_points` | **coBool**, true (name is misleading — verified against the def line) | ctor (`m_enable_wall_skip_points`-style flag); `compute_wall_skip_points` builds internal `Vec<Vec2f>`; `GCode.cpp::_do_export` gates travel-avoid-perimeter | enables internal wall-skip-point computation for nozzle travel avoidance around the tower | declared + emitted; **gap**: no travel-avoid machinery |

**Default-parity claim:** all 13 declared defaults equal canonical's (verified from `PrintConfigDef`). The **only** known divergences are behavioral: (a) pitch basis `line_width` vs `nozzle_diameter × Width_To_Nozzle_Ratio` (recorded in design; not a default deviation, no `DEVIATION_LOG.md` row required); (b) per-filament vectors as scalar globals (queue-established, Tier-D fog). Neither is unverifiable-behaviour — no human sign-off is consumed by this packet. If implementation discovers either claim false, the discovery is reported to the human before any deviation row is filed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`.
- Negative: `AC-N1` (bounds reject out-of-range), `AC-N2` (cross-module leakage blocked).
- Cross-packet impact: P03 (wayfinder ticket 10) shares the `wipe-tower` owner and will see the manifest grown by this packet's 13 keys; its authoring must treat the Tier-D per-filament fog note as inherited. Packet 253's ACs are unaffected (different owner).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` §Verification lists the 2–3 gate commands and defers the rest here (it intentionally does not repeat this table).

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1: 21-key manifest contract | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p wipe-tower 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2: wired pitch + updated pitch-pinned baselines | FACT pass/fail |
| `cargo test -p slicer-scheduler --test wipe_tower_config_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 + AC-N1: percent threading, bounds acceptance/rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- undeclared_prime_tower_keys_stay_hidden_from_other_modules 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2: leakage blocked | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tail -3` | AC-4: docs regenerated | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness after manifest+src edits | FACT exit=0 |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, or shared scratch state:

- The manifest growth (Step 1) must land before the module reads (Step 3), because `ConfigView::from_declared` filters by declared keys — reading an undeclared key is a silent-None bug, not an error.
- The scheduler bounds fixture (Step 2) mirrors the in-memory `LoadedModuleBuilder` shape, not a manifest TOML fixture; do not invent a `fixtures/` manifest subdir for it.
- The guest WASM freshness gate (`cargo xtask build-guests --check`) runs at Step 3 exit and at the acceptance ceremony — manifest + src both feed the guest fingerprint.

## Context Discipline Notes

Packet-specific hazards:

- `docs/ORCA_CONFIG_REFERENCE.md` and `docs/15_config_keys_reference.md` are large: never read in full; the reference table rows for these 13 keys live around the `Wipe Tower` section if a cross-check is ever needed (ranged read only).
- `OrcaSlicerDocumented/` is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — all reads delegated per the orca-delegation snippet (ticket 08 pinned this ledger fact; re-verify the checkout exists on first use).
- Do not read `crates/slicer-ir/src/resolved_config.rs` in full (1700+ lines); the only facts needed are in `requirements.md` §Verified Grounding.