# Requirements: 255-wipe-tower-geometry-keys

## Packet Metadata

- Grouped task IDs: none — the feature-gap queue's established pattern is `task_ids: []` (packets 234a, 253, and 254 precedent); `docs/07_implementation_status.md` holds no TASK row for this queue.
- Backlog source: `docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P03).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P03 — Multimaterial / Prime tower (2/2) — wipe-tower is 13 keys that OrcaSlicer reads inside `WipeTower.cpp`/`WipeTower2.cpp`/`GCode.cpp`/`Print.cpp`, and Pinch 'n Print implements none of them under their Orca names:

- The `wipe-tower` module declares 21 keys after packet 254 (8 pre-existing + 13 P02) and none of the P03 keys exists in any manifest. Three P03 keys appear **host-side only** as static `ORCA_CONFIG_PADDING` literals in `crates/slicer-gcode/src/serialize.rs` (`single_extruder_multi_material` = "1", `wipe_tower_rotation_angle` = "0", `wipe_tower_no_sparse_layers` = "0") — padding for Orca's viewer, probed at authoring time to match Orca's defaults, never consumed as config.
- Canonically, `wipe_tower_extra_flow` multiplies the toolchange wipe extrusion volume (`WipeTower2.cpp::toolchange_Wipe` reads `m_extra_flow`; `set_toolchange`/`save_on_last_wipe` consume it). This port's purge generator hardcodes `flow_factor: 1.0` on its scan-line entities (`generate_purge_paths`, `modules/core-modules/wipe-tower/src/lib.rs`), and the emitter multiplies extrusion E by each point's `flow_factor` (`crates/slicer-gcode/src/emit.rs` E computation) — a live, output-visible decision point at the same semantic site.
- The other declared keys gate canonical behaviours this port's simplified rectangular scan-line tower does not have: cone-surface walls (`wipe_tower_cone_angle` → `generate_support_cone_wall`), rib walls (`wipe_tower_rib_width`, `wipe_tower_extra_rib_length` → `generate`), rib/cone/rectangle wall-type selection (`wipe_tower_wall_type` → `use_gap_wall`/`finish_layer`/`compute_wall_skip_points`), fillet walls (`wipe_tower_fillet_wall` → ctor), the sparse-layer bridge pass (`wipe_tower_bridging` → `finish_layer`), tower rotation (`wipe_tower_rotation_angle` → ctor), flush-matrix routing (`purge_in_prime_tower` + `single_extruder_multi_material` → `extract_wipe_volumes`), and the ramming/space arithmetic (`wipe_tower_extra_spacing` → `toolchange_Unload`/`set_toolchange`).
- `wipe_tower_max_purge_speed` needs no new key in this tree: the host feedrate key `wipe_tower_speed` already names the identical decision (declared in `crates/slicer-ir/src/feedrate.rs` with default 90.0 = Orca's `wipe_tower_max_purge_speed` default 90; consumed by `ExtrusionRole::WipeTower` in `crates/slicer-gcode/src/emit.rs::resolve_feedrate`). It is a same-owner **alias finding**, handed to the rename workstream (wayfinder ticket 108) rather than declared twice.
- One declared key (none of the six P02-style vectors here) had a per-filament question; P03's 13 are all scalar-typed canonically, so the Tier-D fog is not engaged by this packet.

The slice is coherent because all keys share one owner, one manifest, and — for the single wired key — one arithmetic site consumed by both module execution paths.

## In Scope

12 of the 13 P03 keys (membership from the `05-asset-packet-list.md` P03 row; per-key Tier A rows in `04-asset-tier-assignment.md`):

1. **Manifest declarations:** the 12 keys in `modules/core-modules/wipe-tower/wipe-tower.toml` with Orca defaults, types, bounds, and display/group metadata — four bools (`purge_in_prime_tower` true, `single_extruder_multi_material` true, `wipe_tower_fillet_wall` true, `wipe_tower_no_sparse_layers` false), five floats (`wipe_tower_bridging` 10.0 unbounded; `wipe_tower_cone_angle` 30.0 [0, 90]; `wipe_tower_extra_rib_length` 0.0 max 300, no min — canonical has no min; `wipe_tower_rib_width` 8.0 [0, 300]; `wipe_tower_rotation_angle` 0.0 unbounded), two percents (`wipe_tower_extra_flow` and `wipe_tower_extra_spacing`, both `"100%"` with [100, 300]), one enum (`wipe_tower_wall_type` `["rectangle", "cone", "rib"]` default `"rib"`).
2. **The one live wiring — `wipe_tower_extra_flow`:** read in `WipeTower::from_config` (percent → factor), stored on `WipeTower`, and applied in `generate_purge_paths` as the scan-line points' `flow_factor` (value/100, replacing the hardcoded `1.0`). Both execution paths (`process()`, `run_finalization()`) route through `generate_purge_paths`, so one site covers both. **No output change at defaults** (factor 1.0 is identity); a `"200%"` config doubles purge extrusion on scan lines. Travel (0.0) and prime (1.0) flows are untouched.
3. **Emission-surface reachability for user-set values:** any of the 12 keys supplied by a profile rides the existing extensions bucket into the G-code CONFIG_BLOCK, so Orca 3MF/preset values round-trip visibly. The two percent-typed schema defaults additionally thread via the packet-185 percent transport, adding exactly 2 CONFIG_BLOCK lines at defaults (both spell `100%`, matching Orca's default spelling).
4. **Recorded decision-point gaps:** the 10 declared-but-unwired keys carry per-key gap notes in this file's parity table (what the canonical decision point is — file + function; why this port lacks it; what building it would entail). None is silently dropped.
5. **`wipe_tower_max_purge_speed` exclusion (alias finding):** documented per-key below and surfaced to the map as a new rename-workstream ticket (108) asking whether `wipe_tower_speed` should take Orca's name. This packet neither declares it nor touches `crates/slicer-ir/src/feedrate.rs`.

## Out of Scope

- **Building the absent decision points** — cone/rib/fillet wall geometry, the bridging sparse pass, tower rotation, flush-matrix routing, and ramming spacing are Tier C "new granular geometry" work (04's rubric), not Tier A plumbing. Future packets.
- **`wipe_tower_speed` renaming** — owned by wayfinder ticket 108 (created by the ticket-10 session per the alias-finding rule); this packet only documents the equivalence evidence.
- **`ORCA_CONFIG_PADDING` changes** — the host-side padding table is untouched (packet 254's ruling); padding only fills absent keys and dedups against user values via `emit_config_kv`.
- **The legacy `process()` retirement** — TODO(packet-41) stays; the wired flow factor reaches both paths through the shared `generate_purge_paths`.
- **Baseline byte-identicality for CONFIG_BLOCK** — not preserved: the two threaded percent defaults add 2 lines at defaults (intended, mirrors Orca's own viewer output). Geometry at defaults *is* byte-identical (identity flow factor).

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md` — the packet ticket; direct read.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P03 row at `### P03 — Multimaterial / Prime tower (2/2) — wipe-tower`; ranged read ~10 lines.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the 13 Tier A rows (`purge_in_prime_tower` … `wipe_tower_wall_type`); ranged read ~15 lines. Over 300 lines total: delegate beyond these rows.
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — ~80 lines; direct read.
- `docs/15_config_keys_reference.md` — large; regeneration + grep verification only, never read in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: the 13 declaration facts (types, defaults, bounds, enum options) quoted in §Per-key parity evidence.
- `src/libslic3r/GCode/WipeTower2.cpp` — the `WipeTower2` constructor and its consumers (`toolchange_Wipe`, `set_toolchange`, `save_on_last_wipe`, `toolchange_Unload`, `finish_layer`, `generate`, `extract_wipe_volumes`, `generate_support_cone_wall`, `use_gap_wall`, `compute_wall_skip_points`): the behaviour described per key in §Per-key parity evidence.
- `src/libslic3r/GCode/WipeTower.cpp` — the legacy `WipeTower` constructor (confirms the member cluster is not Type2-only).
- `src/libslic3r/GCode.cpp` — `WipeTowerIntegration::tool_change` (`wipe_tower_no_sparse_layers` consumption outside the tower class) and `_do_export` (`single_extruder_multi_material` export-path reads).

## Verified Grounding

All claims below were verified against the tree and the canonical checkout at authoring time (2026-08-28):

- **Manifest state:** `modules/core-modules/wipe-tower/wipe-tower.toml` declares exactly 21 keys after packet 254 (8 pre-existing + the 13 P02 keys; the packet is `draft` and authored — the union assertion in AC-1 is a Step-2 precondition check that re-derives the count from disk at implementation time and FAILS the step if 254 has not landed).
- **Purge-path flow:** `generate_purge_paths` builds three entity kinds — travel (both points `flow_factor: 0.0`), scan lines (both points `flow_factor: 1.0`), prime (first point 0.0, second 1.0); scan-line sites are the only `1.0` flow sites in the purge entity set. The emitter's E computation multiplies distance × width × height × `flow_factor` (`crates/slicer-gcode/src/emit.rs`); none of the wipe-tower geometry tests pins `flow_factor` today (the module test fixture copies the literal).
- **Both paths share the site:** `process()` and `run_finalization()` each call `generate_purge_paths` (`modules/core-modules/wipe-tower/src/lib.rs`); neither post-adjusts flow afterwards.
- **Config-read machinery:** `WipeTower::from_config` reads declared keys via `ConfigView::get`; percent values arrive as `ConfigValue::Percent` (and `ConfigValue::FloatOrPercent { is_percent: true }` for float_or_percent field types). Percent-typed manifest defaults are parsed into `parsed_default` (`crates/slicer-scheduler/src/manifest.rs::read_config_schema`) and threaded into `ResolvedConfig.extensions` (packet-185 transport); non-percent defaults stay manifest-side, applied by the module's read fallback.
- **Enum machinery:** manifest `type = "enum"` with `values = [...]` is established (five core modules use it, e.g. `seam-placer.toml` `seam_mode`); `ConfigBoundsIndex::from_modules` collects enum domains, and `ConfigBoundsIndex::check` enforces membership for `String` values (`crates/slicer-scheduler/src/config_resolution.rs`). The scheduler rejects unknown enum values; bounds declarations are numeric-only (`is_numeric_field_type`), so the enum key needs no min/max.
- **Bounds enforcement shape:** numeric min/max with per-element list reporting in `ConfigBoundsIndex::check`; percent bounds fixtures mirror `percent_bounds_rejects_…` arms in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and the `LoadedModuleBuilder::new(...).config_schema(...)` fixture shape in `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`.
- **Padding-table collisions:** three P03 keys exist as `ORCA_CONFIG_PADDING` literals (`single_extruder_multi_material`, `wipe_tower_rotation_angle`, `wipe_tower_no_sparse_layers`) whose probed values equal Orca's defaults. Padding only emits when the key is absent from the raw config map; once declared and user-suppliable, user values dedup via `emit_config_kv`'s `emitted` set. Manifest-side defaults (bool/float/enum) do not enter `raw_config` at defaults, so padding remains the sole emitter until a user supplies a value — after which the padding literal is silently skipped. Both paths produce Orca-default-matching bytes.
- **CONFIG_BLOCK fallout surface:** no golden test pins CONFIG_BLOCK line counts; the nearest pins are `≥ 80 key-value lines` (`crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`) and `CONFIG_BLOCK_START`/`END` occurrence counts (`crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs`, `gcode_header_thumbnail_config_blocks_tdd.rs`) — all satisfied by +2 lines.
- **Host feedrate key (the alias finding):** `wipe_tower_speed` is declared in `crates/slicer-ir/src/feedrate.rs` `FeedrateConfig` (default 90.0; the `("wipe_tower_speed", |fc| &mut fc.wipe_tower_speed)` arm), consumed by `ExtrusionRole::WipeTower` in `crates/slicer-gcode/src/emit.rs::resolve_feedrate`, documented in `docs/15_config_keys_reference.md` (host-key table), and locked against drift by `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`. It is not in the ticket 99–107 rename set.
- **Module tests at authoring time:** `modules/core-modules/wipe-tower/tests/` carries `bed_bounds_tdd.rs`, `finalization_live_tdd.rs`, `slicer_module_binding_tdd.rs`, `wipe_tower_tdd.rs`; struct-literal churn gate applies to any `WipeTower {` literal in new tests (waiver format in `docs/21_data_defaults_and_fixtures.md`).
- **Guest staleness baseline:** `cargo xtask build-guests --check` at authoring time exited 1 with exactly one stale guest — `tree-support-planner-guest` (fingerprint mismatch), a pre-existing condition unrelated to this packet's surface (only the wipe-tower manifest feeds this packet; the wipe-tower guest is fresh). Per the map's Notes, source-rewriting operations can trip mtime-based staleness; the freshness gate re-runs inside the packet at Step 3 exit.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

### Per-key parity evidence (ticket 02 standard)

All 13 keys were read in canonical `PrintConfig.cpp::PrintConfigDef` (type, default, bounds) and their consumers located. None is dead-in-canonical — every key has at least one reader. `wipe_tower_max_purge_speed` is excluded (alias finding, row 13).

| Key | Canonical def | Canonical consumer (file + function) | Behaviour in canonical | Disposition here |
| --- | --- | --- | --- | --- |
| `purge_in_prime_tower` | coBool, true | `WipeTower2.cpp::extract_wipe_volumes` | master switch for tower purging; when false (or `!single_extruder_multi_material`) the flush matrix is zeroed — no tower purge printed | declared + emitted; **decision-point gap**: the port has no flush-matrix routing to gate |
| `single_extruder_multi_material` | coBool, true | `WipeTower2.cpp::WipeTower2` ctor (`m_semm`) + `extract_wipe_volumes`; `GCode.cpp::_do_export` (filament-end, temperatures, reset-E, ooze-prevention); `post_process_wipe_tower_moves` | enables SEMM mode across the tower and export paths | declared + emitted; **gap**: port's toolchange machinery has no SEMM branch to gate |
| `wipe_tower_bridging` | coFloat, 10.0, no bounds | `WipeTower2.cpp::WipeTower2` ctor (`m_bridging`) → `finish_layer` | sparse-structure bridge spacing over unsupported tower spans: `n = 1 + int(span/m_bridging)` | declared + emitted; **gap**: the port's tower is fully supported per-layer; no bridge pass |
| `wipe_tower_cone_angle` | coFloat, 30.0, [0, 90] | `WipeTower2.cpp::WipeTower2` ctor (`m_wipe_tower_cone_angle`) → `generate_support_cone_wall` / `get_wipe_tower_cone_base` | cone-surface taper angle in degrees | declared + emitted; **gap**: no cone-wall geometry in the scan-line tower |
| `wipe_tower_extra_flow` | coPercent, 100.0, [100, 300] | `WipeTower2.cpp::WipeTower2` ctor (`m_extra_flow`) → `toolchange_Wipe`, `set_toolchange`, `save_on_last_wipe` | multiplies the toolchange wipe extrusion volume; 100% = identity | **WIRED** to the scan-line purge paths' `flow_factor` (`value/100`, replacing the hardcoded 1.0); identity at defaults; canonical's [100, 300] bounds enforced |
| `wipe_tower_extra_rib_length` | coFloat, 0.0, max 300, no min | `WipeTower2.cpp::generate` (`m_rib_length += m_extra_rib_length`) | lengthens rib-wall segments; tooltip notes negative values shrink ribs | declared + emitted; **gap**: no rib-wall geometry |
| `wipe_tower_extra_spacing` | coPercent, 100.0, [100, 300] | `WipeTower2.cpp::WipeTower2` ctor (`m_extra_spacing_wipe`, `m_extra_spacing_ramming`) → `toolchange_Unload` (ramming `y_step`), `set_toolchange`, `save_on_last_wipe` | widens wipe/ramming line spacing as a percentage | declared + emitted; **gap**: no ramming sequence or wipe-spacing arithmetic (the module's *infill* spacing is `prime_tower_infill_gap`'s, packet 254) |
| `wipe_tower_fillet_wall` | coBool, true | `WipeTower2.cpp::WipeTower2` ctor; `WipeTower.cpp` ctor | fillets the tower wall corners | declared + emitted; **gap**: no wall-corner geometry |
| `wipe_tower_max_purge_speed` | coFloat, 90.0, min 10, no max | `WipeTower2.cpp::WipeTower2` ctor (`m_max_speed`) → `toolchange_Wipe` (feedrate = min(max, wipe speed)), `finish_layer` | caps toolchange purge feedrate | **EXCLUDED — alias finding**: host key `wipe_tower_speed` (`FeedrateConfig`, default 90.0) already drives `ExtrusionRole::WipeTower` feedrate in `resolve_feedrate`. Same consumer, same default, same decision (per-path feedrate cap vs base speed). Declaring it would mint a duplicate spelling (ticket 107's class). Rename question → wayfinder ticket 108 |
| `wipe_tower_no_sparse_layers` | coBool, false | `WipeTower2.cpp::WipeTower2` ctor (`m_no_sparse_layers`) → `toolchange_Wipe`/`finish_layer`/`set_toolchange`/`generate_support_cone_wall`; `GCode.cpp::WipeTowerIntegration::tool_change` | forces the tower to print on every layer (no sparse/empty layers when unused) | declared + emitted; **gap**: the port already prints the tower only at tool-change layers and never skips — the canonical no-sparse *toggle* has no opposite behaviour to select |
| `wipe_tower_rib_width` | coFloat, 8.0, [0, 300] | `WipeTower2.cpp::WipeTower2` ctor → `finish_layer` (via rib-wall selection) | mm width of rib-wall structures | declared + emitted; **gap**: no rib-wall geometry |
| `wipe_tower_rotation_angle` | coFloat, 0.0, no bounds | `WipeTower2.cpp::WipeTower2` ctor; `WipeTower.cpp` ctor | rotates the tower footprint by degrees | declared + emitted; **gap**: the port's tower is axis-aligned only |
| `wipe_tower_wall_type` | coEnum `WipeTowerWallType`, default `wtwRib` ("rib"), values rectangle/cone/rib | `WipeTower2.cpp::use_gap_wall` + ctor (`m_wall_type`) → `finish_layer` (cone vs rib selection), `generate` (rib forces square tower), `compute_wall_skip_points` | selects wall structure type | declared (enum, domain ["rectangle","cone","rib"], default "rib") + emitted; **gap**: all three types route to the same scan-line wall in this port |

**Default-parity claim:** every declared default equals canonical's (verified from `PrintConfigDef`; the three padding-table probes agree). The only divergences are **behavioral** and recorded: (a) sparse-layer handling is fixed ON in the port (tower prints only at tool-change layers) with `wipe_tower_no_sparse_layers` declared-but-unwired; (b) the max-purge-speed consumer exists under a different name (`wipe_tower_speed`) — an alias finding, not a deviation; (c) per-key decision-point gaps as tabulated. None is unverifiable-behaviour — no human sign-off is consumed by this packet; no `DEVIATION_LOG.md` row is filed. If implementation discovers any claim false, the discovery is reported to the human before any deviation row is filed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`.
- Negative: `AC-N1` (percent bounds reject), `AC-N2` (cross-module leakage blocked).
- Cross-packet impact: shares the `wipe-tower` owner with packet 254 (P02, `draft` at authoring time); its Step-2 precondition re-derives the 21-key base union from disk at implementation time. The Tier-D per-filament fog note is **not** inherited by this packet (all 13 P03 keys are scalar-typed canonically).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` §Verification lists the 2–3 gate commands and defers the rest here (it intentionally does not repeat this table).

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1: the 21+12-key manifest contract | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p wipe-tower 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2: wired flow factor + module suite green | FACT pass/fail |
| `cargo test -p slicer-scheduler --test wipe_tower_p03_config_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 + AC-N1: percent threading, bounds acceptance/rejection, enum membership | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- undeclared_p03_wipe_tower_keys_stay_hidden_from_other_modules 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2: leakage blocked | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tail -3` | AC-4: docs regenerated | FACT pass/fail |
| `rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md && echo AC4-PASS` | AC-4 grep | FACT AC4-PASS |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness after manifest+src edits | FACT exit=0 |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, or shared scratch state:

- The manifest growth (Step 1) must land before the module read (Step 3) — `ConfigView::from_declared` filters by declared keys; reading an undeclared key is a silent-None, not an error (packet 254 invariant).
- The scheduler bounds fixture mirrors the in-memory `LoadedModuleBuilder` shape, not a manifest TOML fixture (packet 254 precedent).
- The guest WASM freshness gate runs at Step 3 exit and the acceptance ceremony; the tree-support-planner guest was stale on a clean tree at authoring time (pre-existing; unrelated to this surface — if it alone is stale, rebuild it and proceed; a wipe-tower staleness after manifest edits IS this packet's to clear).
- Implementation is recorded against wayfinder ticket 10 (queue precedent; no `docs/07` TASK row).

## Context Discipline Notes

Packet-specific hazards:

- `docs/ORCA_CONFIG_REFERENCE.md` and `docs/15_config_keys_reference.md` are large: never read in full; ranged reads only if a cross-check is ever needed.
- `OrcaSlicerDocumented/` is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — all reads delegated per the orca-delegation snippet (ticket 08 pinned this ledger fact; re-verify the checkout exists on first use).
- Do not read `crates/slicer-ir/src/resolved_config.rs` (1900+ lines) or `crates/slicer-gcode/src/emit.rs` (1300+ lines) in full; the needed facts are in §Verified Grounding.