---
status: draft
packet: 307-prime-tower-body-parity
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/122-author-packet-prime-tower-body-parity.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 122 (prime tower body parity) with P22 (ticket 29, wipe_tower_filament) and P24 (ticket 31, timelapse_type) dissolved into it. Folds no authored packet's keys: 254a owns pitch/brim/framework, 254b owns interface/ramming/flat-ironing, 255 owns the seven wall primitives — this packet builds the body assembly, global depth planning, tool selection, and smooth mode around them. Next packet number 307 derived from disk (max 306); DEV-201/202/203 derived from the log (max DEV-200).
---

# Packet Contract: 307-prime-tower-body-parity

## Goal

Grow the port's purge-only prime tower into a real tower body at canonical parity: per-layer `finish_layer` assembly (inner perimeter, sparse/solid infill, outer wall, first-layer brim), top-down depth planning with idle-layer entries, forced-filament tool selection, and smooth-timelapse mode with its traditional-injection suppression.

## Scope Boundaries

The body is assembled inside the existing `wipe-tower` module behind the live `PostPass::LayerFinalization` seam (`run_finalization` receives all layers); no WIT, IR, schema, ADR, or new-module change. Five keys are declared on `wipe-tower` (`wipe_tower_bridging`, `wipe_tower_no_sparse_layers`, `prime_tower_skip_points`, `wipe_tower_filament`, `timelapse_type`) and two reader declarations ride `machine-gcode-emit` (`timelapse_type`, `enable_prime_tower`) for the suppression gate only. In particular, 254a's depth model/pitch/brim builder, 254b's interface block, and 255's wall polygons are consumed as drafted, never re-declared.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 (numbering), 100 (wipe-tower rename) — both resolved; ticket 29's census and seam analysis (resolved, read, not re-derived).
- Unblocks: ticket 141 (`max_layer_height` tower partitions need the planned-depth model); ticket 50's `single_extruder_multi_material_priming` seat (sequences after the body exists).
- Activation blockers: FORWARD-DEP on draft `254a-prime-tower-geometry-keys` (per-layer depth model, scan-line pitch, brim builder, framework forcing, `wipe_tower_config_schema_tdd.rs` + `toml = "0.8"` dev-dep); FORWARD-DEP on draft `254b-prime-tower-interface-and-ramming` (interface purge block coexistence); FORWARD-DEP on draft `255-wipe-tower-geometry-keys` (rib/cone/rectangle wall polygon helpers + rotated frame). Ordering 254a → 254b → 255 → 307; queue-order merge churn in `wipe-tower/src/lib.rs` lands in that order.

## Acceptance Criteria

- **AC-1. Given** the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares exactly five new tables with canonical values — `wipe_tower_bridging` (`float`, `10.0`, no `min`/`max`), `wipe_tower_no_sparse_layers` (`bool`, `false`), `prime_tower_skip_points` (`bool`, `true`), `wipe_tower_filament` (`int`, `0`, `min = 0`, no `max`), `timelapse_type` (`enum`, `values = ["traditional", "smooth"]`, `default = "traditional"`) — and declares none of `wipe_tower_type`, any `filament_tower_interface_*` key beyond 254b's seven, or any 254a/254b/255 key with a different spec; and the `machine-gcode-emit` manifest declares `timelapse_type` (same enum) and `enable_prime_tower` (`bool`, `false`) as reader-only rows. Canonical serialises this enum as `"0"`/`"1"` (`s_keys_map_TimelapseType`, *"using 0,1 to compatible with old files"*), so both readers accept the ingest spellings — see AC-14. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a default config (tower disabled) on the 20 mm box fixture, **when** `run_slice` runs before and after this packet, **then** the emitted entity stream and the CONFIG_BLOCK key count are byte-identical to the parent commit's (tower-disabled path emits nothing new; the new keys add no default CONFIG_BLOCK lines). | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** three tower layers whose per-layer purge depths are 6.0, 2.0, 4.0 mm with `prime_tower_width = 60.0`, **when** planning runs, **then** the planned depths are 6.0, 6.0, 4.0 from the top layer down (each layer takes the max of its own toolchange depth and every layer above within the propagation band) and the tower-wide max depth is 6.0 plus one perimeter width — canonical `WipeTower2::plan_tower`'s backward max-propagation shape. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a five-layer print whose tool changes land on layers 1 and 4 only, **when** `run_finalization` runs with `wipe_tower_no_sparse_layers = false` (default), **then** all five layers carry tower-body entities, and with `= true` only layers 1 and 4 do — canonical's idle-layer entries vs `m_no_sparse_layers`. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a tool-change layer with the body enabled at defaults, **when** `run_finalization` emits that layer, **then** its tower entities appear in `finish_layer` order — inner-perimeter rectangle of the sparse section, CP EMPTY GRID infill lines, outer-wall loop from 255's wall helper, and (first tower layer only) 254a's brim loops — each stamped `ExtrusionRole::WipeTower` on `RegionKey "__wipe_tower__"`, with purge blocks still anchored at their `after_entity_index` and body entities appended via the anchor-free push. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `wipe_tower_bridging = 4.0` with scan-line pitch 0.6 mm, **when** the infill pass runs, **then** sparse lines are spaced 4.0 mm apart (bridging caps the count, pitch floors the spacing: `spacing = max(pitch, bridging)`), and at the default 10.0 the same footprint emits exactly the pitch-spaced count the pre-packet purge block would — pinning that 10.0 is inert for sub-10 mm towers. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the adhesion first tower layer and a mid-print layer whose next layer carries no soluble-lookahead signal, **when** the infill pass runs, **then** the first layer's grid is solid (full-coverage lines at the configured pitch) and the mid layer's is sparse per AC-6 — canonical's solid-on-first-layer arm; the next-layer-soluble arm is a recorded non-borrow (no per-filament solubility model; DEV-201 context). | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `wipe_tower_filament = 2` on a two-tool print whose layer-0 toolchange targets tool 0, **when** `run_finalization` emits the body, **then** every body entity on that layer carries `tool_index = 1` and the layer-boundary synthesis switches to tool 1 (canonical's forced-filament short-circuit, which masks solubility rather than reading it); with `= 0` (default auto) the body keeps the purge's `tc.to_tool`. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** a single-filament print with `enable_prime_tower = true` and `timelapse_type = "smooth"`, **when** `run_finalization` runs, **then** every object layer carries an outer-wall-only tower layer whose planned depth is floored at the first tower layer's depth and equalised so all layers carry the same tower-wide depth (canonical `plan_tower_new` + `update_all_layer_depth` shape, exact arithmetic pinned by the Step 5 canonical dispatch), while `timelapse_type = "traditional"` (default) on the same print emits no tower at all. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** `timelapse_type = "smooth"` with `enable_prime_tower = true` and a non-empty `time_lapse_gcode`, **when** `run_gcode_postprocess` runs, **then** no traditional snapshot injection is emitted for any layer; with `"traditional"` (default) or tower disabled the injection matches the pre-packet `printer_structure` gate exactly — canonical's `(!m_wipe_tower || !m_wipe_tower->enable_timelapse_print())` suppression, DEV-202; and the canonical ingest spelling `"1"` suppresses identically to `"smooth"` (AC-14's adapter arm, emitter side). | `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-11. Given** `prime_tower_skip_points = true` (default) vs `false` on identical fixtures, **when** the outer wall is emitted, **then** the `true` wall carries one gap corridor (open polyline, `use_gap_wall` shape) and the `false` wall is a closed loop; emitter-side travel-avoid routing through the gap is explicitly not built (DEV-203). | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-12. Given** the scheduler bounds index built from both real manifests, **when** resolution runs, **then** `wipe_tower_filament = -1` is rejected out-of-range naming the key, `timelapse_type = "spiral"` is rejected as an unknown enum variant, and the three canonical-bare keys (`wipe_tower_bridging`, `wipe_tower_no_sparse_layers`, `prime_tower_skip_points`) accept any value their manifest permits. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-13. Given** `cargo xtask gen-config-docs` has run, **when** the generated tables are checked, **then** the five tower keys appear under owner `wipe-tower`, the two reader rows under owner `machine-gcode-emit`, and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'wipe_tower_bridging' docs/15_config_keys_reference.md && rg -q 'wipe_tower_no_sparse_layers' docs/15_config_keys_reference.md && rg -q 'timelapse_type' docs/15_config_keys_reference.md; echo "exit=$?"`
- **AC-14. Given** a raw config source carrying the canonical ingest spellings `timelapse_type = "1"` and `timelapse_type = "0"` (canonical `s_keys_map_TimelapseType`), **when** the tower plans, **then** `"1"` selects smooth mode (forced tower, equalised depth, wall-only body) and `"0"` selects traditional exactly as the word spellings do; an unknown spelling is a fatal naming the key — the ticket-100/107 input-adapter class, no silent Traditional fallback. | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Negative Test Cases

- **AC-N1. Given** a `LoadedModule` whose manifest declares none of the tower keys, **when** `bind_module_config_view` binds it against a source containing all five, **then** `ConfigView::get` returns `None` for each — the declarations leak into no other module. | `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** `wipe_tower_filament = 5` on a two-tool print, **when** `run_finalization` runs, **then** it returns fatal naming the key and the valid range (canonical `Print::validate`'s in-range assert, ported as a module error since no static bound can express it). | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest schema guards, **when** any of the seven declared tables is removed or its `type`/`default`/`min`/`max`/`values` drifts from AC-1, **then** the owning guard fails naming the offending key. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** a single-tool print with the tower disabled and `timelapse_type = "smooth"`, **when** the pipeline runs, **then** no tower entity is emitted and traditional injection is unchanged — smooth without an enabled tower is inert (canonical `has_wipe_tower` requires `enable_prime_tower`). | `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — both edited manifests and both `src/lib.rs` files are guest-fingerprint inputs, so this must return exit 0 before closure (exit 3 is `wasm-tools` missing, an infrastructure error, not clean).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline goals; rule-4 PnP-way framing)
- `docs/03_wit_and_manifest.md` - § Known claim IDs + manifest declaration shape (direct, ranges for `[config.schema]` only)
- `docs/08_coordinate_system.md` - delegated SUMMARY (mm↔unit checklist for wall/infill/brim geometry)
- `docs/specs/orca-feature-gap/issues/29-author-packet-p22-multimaterial-filament-for-features-wipe-tower.md` - read in full (189 lines; census + seam analysis, not re-derived)
- `docs/specs/orca-feature-gap/issues/31-author-packet-p24-others-special-mode-wipe-tower.md` - Answer section only (smooth-mode analysis + DEV-168 clause d)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — seven generated rows (five `wipe-tower`, two `machine-gcode-emit`) via `cargo xtask gen-config-docs`, never by hand - `rg -q 'wipe_tower_bridging' docs/15_config_keys_reference.md && rg -q 'timelapse_type' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — three new rows `DEV-201`, `DEV-202`, `DEV-203` (single-tower union; suppression observability; skip-points emitter non-borrow) - `rg -q 'DEV-201' docs/DEVIATION_LOG.md && rg -q 'DEV-202' docs/DEVIATION_LOG.md && rg -q 'DEV-203' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations (type/default/bounds) of the five packet keys + `wipe_tower_type` default (DEV-201 context); `PrintConfigDef::init_fff_params`, never line numbers
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `finish_layer` emission order, solid-vs-sparse rule, `plan_tower` backward propagation, `first_toolchange_to_nonsoluble_nonsupport`, `use_gap_wall`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `insert_wipe_tower_extruder` forced-filament layer membership
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `has_wipe_tower` / `enable_timelapse_print` force-tower conditions
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `process_layer` smooth-suppression clause (DEV-202 origin)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
