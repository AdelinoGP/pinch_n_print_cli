---
status: draft
packet: prime-tower-geometry-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P02, geometry half)
context_cost_estimate: M
tier: B
---

# Packet Contract: prime-tower-geometry-keys

Split half A of former packet 254. The interface-feature and ramming cluster moved to `254b-prime-tower-interface-and-ramming` under the ticket's own rule "split by feature if the packet exceeds the B ceiling of 12 keys".

## Goal

Build the three prime-tower **geometry** decision points inside the `wipe-tower` core module: `prime_tower_infill_gap` drives the purge block's scan-line pitch, `prime_tower_brim_width` builds first-layer brim rings around the tower footprint (including canonical's `-1` Auto sentinel resolved from tower height), and `prime_tower_enable_framework` forces every layer's tower depth to the first tower layer's. Framework is meaningless without a per-layer depth model, so this packet also builds one: the layer's tower depth becomes `n_tool_changes(layer) × block_depth`, with each tool change's purge block seated at its own depth offset instead of every block overlapping at `tower_y` as they do today.

## Scope Boundaries

In scope: `modules/core-modules/wipe-tower/wipe-tower.toml` (three new `[config.schema.*]` tables plus a `layer_height` declaration), `modules/core-modules/wipe-tower/src/lib.rs` (`from_config`, `run_finalization`, `generate_purge_paths`, plus new brim and depth helpers), the module's four test files, one scheduler bounds arm, one runtime leakage arm, and the generated `docs/15_config_keys_reference.md`.

Out of scope: the ten interface / ramming / travel-avoid keys (`254b` owns nine of them; `prime_tower_skip_points` is returned to the queue — see `requirements.md`), canonical's runtime **re-fitting** of `m_extra_spacing` to meet a minimum tower depth (`WipeTower::plan_tower` / `plan_tower_new`), cone / rib / fillet wall shapes (packet 255's surface), the non-first-layer "brim chamfer" taper, and any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 (packet numbering — `254a` derived from disk at authoring), 05 (P02 key membership), and 100 (the wipe-tower rename workstream whose `bed_shape` → `printable_area` value-format adaptation this manifest already carries).
- Ordering, not gating: `254b-prime-tower-interface-and-ramming` and packet `255-wipe-tower-geometry-keys` share the `wipe-tower` manifest and `from_config`. Land `254a` first — `254b`'s interface body is expressed in terms of the per-layer depth model this packet builds.
- Unblocks: `254b` (depth model), wayfinder ticket 09's resolution.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares the 8 pre-existing keys (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`) **plus exactly four new tables**: `prime_tower_infill_gap` (`type = "percent"`, `default = "150%"`, `min = 100`, no `max`), `prime_tower_brim_width` (`type = "float"`, `default = 3.0`, `min = -1.0`, no `max`), `prime_tower_enable_framework` (`type = "bool"`, `default = false`), and `layer_height` (`type = "float"`, `default = 0.2`, `min = 0.01`) — 12 keys total, and **none** of the nine `254b` keys or `prime_tower_skip_points`. `wipe_tower_config_schema_tdd.rs` is a **new** standalone test binary (`wipe-tower/tests/` has no aggregator `main.rs`, so no `mod` registration is needed) and requires adding `toml = "0.8"` to `wipe-tower`'s dev-dependencies, which today has only `slicer-sdk`. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `prime_tower_infill_gap = "200%"`, `line_width = 0.4` and a layer carrying one tool change, **when** `run_finalization` emits the purge block, **then** consecutive scan lines are spaced `0.8` mm apart in Y (pitch `= (200/100) × line_width`), and the emitted scan-line count for a fixed block depth is exactly half the count the same fixture produces at `"100%"`. The schema default `"150%"` yields pitch `0.6` mm, replacing today's hardcoded `y += line_width` advance — a change at defaults this packet owns, with the pitch-pinned baselines in `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` updated to the formula values. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a layer carrying **three** tool changes, `prime_volume`, `line_width` and `layer_height` fixed, **when** `run_finalization` runs, **then** the three purge blocks occupy three **disjoint** Y bands — block `k` spans `[tower_y + k·block_depth, tower_y + (k+1)·block_depth)` — and the layer's total tower depth is `3 × block_depth`, where `block_depth = prime_volume / (line_width × layer_height × tower_width)`. Pre-packet, all three blocks started at `tower_y` and overlapped; the AC asserts no two blocks share a scan-line Y. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `prime_tower_enable_framework = true` and a print whose first tower layer carries 3 tool changes while a later layer carries 1, **when** `run_finalization` runs, **then** the later layer's tower depth equals the first tower layer's (`3 × block_depth`) — its single purge block is padded out to the first layer's depth by extending the scan-line span, mirroring canonical `WipeTower::generate_wipe_tower_blocks`' `block.layer_depths[layer_id] = block.layer_depths[0]`; with `prime_tower_enable_framework = false` (default) the later layer keeps its own `1 × block_depth`. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `prime_tower_brim_width = 6.0`, `line_width = 0.4`, `layer_height = 0.2`, **when** the **first layer that receives tower entities** is emitted, **then** `loops_num = floor((6.0 + spacing/2) / spacing)` rect loops are pushed with `ExtrusionRole::WipeTower`, each offset one further `spacing` **outward** from the tower footprint rect, where `spacing = line_width - layer_height × (1 − π/4)` (canonical `WipeTower2::finish_layer`'s spacing formula); no brim loops appear on any later layer; and with `prime_tower_brim_width = 0.0` no brim loop is emitted at all. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `prime_tower_brim_width = -1.0` (canonical's Auto sentinel) and a layer collection whose top layer `z` is `50.0`, **when** the brim is generated, **then** the effective width is `50.0 / 100 × 8 = 4.0` mm (canonical `WipeTower::get_auto_brim_by_height`: `max_height < 100 ? max_height/100 × 8 : 8`), and for a collection whose top `z` is `150.0` it is exactly `8.0` mm. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the per-layer depth model and the brim, **when** the bed-bounds check runs, **then** it validates the rectangle `[tower_x, tower_x + tower_width + brim_extent] × [tower_y − brim_extent, tower_y + max_layer_depth + brim_extent]` against the `printable_area` polygon — not today's conservative `tower_width`-square — and a tower whose deepest layer plus brim escapes the bed returns `ModuleError::fatal(3, ...)` naming the offending corner. | `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the scheduler's bounds index built from the real `wipe-tower` manifest, **when** config resolution runs, **then** `prime_tower_infill_gap = "99%"` and `prime_tower_brim_width = -2.0` are rejected by `ConfigBoundsIndex::check` with out-of-range errors naming the key, while `"110%"` and the Auto sentinel `-1.0` are accepted; and the percent-typed schema default `"150%"` is threaded into `ResolvedConfig.extensions` as `ConfigValue::Percent(150.0)` (the packet-185 path via `ConfigBoundsIndex::schema_defaults`), whereas the bool/float defaults are not — they stay manifest-side behind the module's read fallback. **This packet authors the arm**: `config_bounds_enforcement_tdd` exists and is registered in `crates/slicer-scheduler/tests/integration/main.rs`, but carries no wipe-tower case today (its manifest-driven cases use the tree-support and traditional-support planner manifests); no file named `wipe_tower_config_bounds_tdd.rs` exists anywhere in the tree, contrary to the former packet 254's citation. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the three prime-tower keys appear under owner `wipe-tower` and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'prime_tower_infill_gap' docs/15_config_keys_reference.md && rg -q 'prime_tower_brim_width' docs/15_config_keys_reference.md && rg -q 'prime_tower_enable_framework' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** a `LoadedModule` whose manifest declares none of the prime-tower keys, **when** `bind_module_config_view` binds it against a source map containing `prime_tower_infill_gap`, **then** the resulting `ConfigView::get("prime_tower_infill_gap")` is `None` — the new declarations leak no wipe-tower config into modules that did not opt in. **This packet authors the arm**: no test named `undeclared_prime_tower_keys_stay_hidden_from_other_modules` exists anywhere in the tree (verified at authoring; the former packet 254 cited it as pre-existing and was wrong). It lands in the already-registered `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, which already exercises `bind_module_config_view` hiding undeclared keys. | `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** `enable_prime_tower = false`, **when** `run_finalization` runs with all three keys set to non-default values, **then** no entity is pushed at all — the geometry additions stay behind the module's existing enable gate. | `cargo test -p wipe-tower --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest schema guard, **when** any of the four new keys is removed from `wipe-tower.toml` or its `type`/`default`/`min`/`max` drifts from AC-1's exact table, **or** any of the nine `254b` keys appears in this manifest, **then** the guard fails naming the offending key. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest and `src/lib.rs` are guest-fingerprint inputs (`guest_input_paths`, `xtask/src/build_guests.rs`, which covers the parent module's `src/` and its depth-1 `*.toml`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) — a module sees only its declared keys; governs the `layer_height` re-declaration and AC-N1.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — forbids padding edits; delegated SUMMARY.
- `docs/08_coordinate_system.md` — the module works in plain mm floats, not scaled units; canonical's `scale_()` must not be ported.
- `docs/15_config_keys_reference.md` — generated by `cargo xtask gen-config-docs`, never hand-edited.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for `prime_tower_infill_gap`, `prime_tower_brim_width`, `prime_tower_enable_framework` and the `layer_height` re-declaration under owner `wipe-tower`. Verification is key-presence (the doc has no per-module subheadings), embodied as AC-9. The edit lands through `cargo xtask gen-config-docs`, never by hand.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — declarations of `prime_tower_infill_gap` (coPercent, default 150, min 100) and `prime_tower_brim_width` (coFloat, default 3.0, min −1, `gui_type = f_enum_open` with `-1` labelled "Auto"). Captured in `requirements.md` §Per-Key Canonical Evidence; not re-read unless disputed.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` — the `WipeTower::WipeTower` constructor (`m_extra_spacing = prime_tower_infill_gap/100`, `m_tower_framework`), `WipeTower::generate` / `generate_new` (`dy = m_extra_spacing × m_perimeter_width`), `WipeTower::align_perimeter`, `WipeTower::generate_wipe_tower_blocks` (framework forcing `layer_depths[layer_id] = layer_depths[0]`), `WipeTower::plan_tower_new` (the `-1` Auto resolution), `WipeTower::get_auto_brim_by_height`, `WipeTower::finish_layer` / `finish_layer_new` (brim loops).
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `WipeTower2::finish_layer` (the brim spacing formula `spacing = m_perimeter_width − m_layer_height × (1 − π/4)`, `loops_num = (brim_width + spacing/2) / spacing`, outward `offset` per loop). Note: `WipeTower2` does **not** read `prime_tower_infill_gap`.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::wipe_tower_data` (the Print-side `-1` override using max object Z). **There is no `Print::plan_tower_new`**; an earlier authoring of this packet cited that non-existent symbol and it has been corrected.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
