---
status: draft
packet: skirt-type-and-draft-shield-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/13-author-packet-p06-others-skirt-skirt-brim.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P06)
context_cost_estimate: M
tier: B
---

# Packet Contract: skirt-type-and-draft-shield-keys

## Goal

Build the five OrcaSlicer skirt/draft-shield decision points inside the `skirt-brim` core module so that every one of `skirt_type`, `min_skirt_length`, `skirt_start_angle`, `draft_shield` and `single_loop_draft_shield` changes emitted skirt geometry at a non-default value: per-object skirt grouping with canonical envelope merging (`skirt_type = "perobject"`), extruded-length loop expansion over a module-side e-per-mm model (`min_skirt_length > 0`), start-corner rotation of the first-layer first loop (`skirt_start_angle`), full-height shield span (`draft_shield = "enabled"`), and innermost-loop-only upper layers (`single_loop_draft_shield = true`). Supporting host work: expose the already-resolved `ResolvedConfig::filament_diameter` through `to_config_map` so the module can compute e-per-mm, and upgrade `serialize.rs`'s hardcoded synthetic `filament_diameter` CONFIG_BLOCK array to be built from that resolved value.

## Scope Boundaries

In scope: the `skirt-brim` module (`skirt-brim.toml` manifest, `src/lib.rs`, `tests/`), one `ResolvedConfig::to_config_map` export line in `crates/slicer-ir/src/resolved_config.rs`, the synthetic-`filament_diameter` branch in `crates/slicer-gcode/src/serialize.rs`, one scheduler bounds/enum arm, one runtime CONFIG_BLOCK arm, and the generated `docs/15_config_keys_reference.md`.

Out of scope: convex-hull skirt geometry (the port's axis-aligned rect loops stay — see `design.md` D-258-1), per-extruder extruded-length rotation (`design.md` D-258-3), the `PrintSequence::ByObject` shared-per-object-skirt `SlicingError`, canonical's `Print::object_skirt_offset` brim-area interaction, seam-style start-point re-seating on already-emitted paths, and any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — number 258 re-derived from disk at authoring time); ticket 05 (packet-list P06 membership); ticket 04 (tier rubric — re-derived to **Tier B** in `design.md`, because this packet builds decision points that do not exist in the tree).
- Ordering, not gating: packets `257a-brim-type-and-object-gap` and `257b-brim-ears` own the same manifest and the same `SkirtBrim::from_config`. Land 257a/257b first to avoid same-module merge churn; this packet's steps are additive to both.
- Unblocks: wayfinder ticket 13's resolution. No queued packet consumes `skirt-brim` keys downstream.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `skirt-brim` module manifest, **when** its `[config.schema]` is parsed, **then** it contains exactly these seven new table entries — `skirt_type` (`type = "enum"`, `values = ["combined", "perobject"]`, `default = "combined"`), `min_skirt_length` (`type = "float"`, `default = 0.0`, `min = 0.0`, no `max`), `skirt_start_angle` (`type = "float"`, `default = -135.0`, `min = -180.0`, `max = 180.0`), `draft_shield` (`type = "enum"`, `values = ["disabled", "enabled"]`, `default = "disabled"`), `single_loop_draft_shield` (`type = "bool"`, `default = false`), `filament_diameter` (`type = "float"`, `default = 1.75`, `min = 0.1`), and `layer_height` (`type = "float"`, `default = 0.2`, `min = 0.01`) — each carrying `display` and `group = "Skirt/Brim"`. The last two are host-owned keys re-declared so `bind_module_config_view` admits them, mirroring `classic-perimeters.toml`'s `layer_height` / `nozzle_diameter` declarations. | `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `skirt_loops = 2`, `skirt_height = 1`, `draft_shield = "enabled"` and a 5-layer collection, **when** `SkirtBrim::run_finalization` runs, **then** all 5 layers receive 2 skirt entities each (10 pushes), whereas the same run with `draft_shield = "disabled"` pushes skirt entities only to layer 0 (2 pushes). | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `single_loop_draft_shield = true`, `skirt_loops = 3`, `draft_shield = "enabled"` and a 3-layer collection, **when** `run_finalization` runs, **then** layer 0 receives 3 skirt entities and layers 1 and 2 receive exactly 1 each — and that single loop is the innermost ring (loop index 0, offset `skirt_distance`); with `single_loop_draft_shield = false` every layer receives 3. | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `skirt_start_angle = 45.0` and `skirt_loops >= 1`, **when** the first-layer innermost skirt loop is generated, **then** its first and last vertex are the rect corner nearest `bbox_center + r·(cos 45°, sin 45°)` (`r` = half-diagonal of that loop's own rect) — for a non-degenerate rect that is `(x_max, y_max)` — the loop still has 5 points and is still closed, and every other loop and every later layer still starts at `(x_min, y_min)`. At the default `-135.0` the selected corner is `(x_min, y_min)`, so default output is byte-identical to pre-packet. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `skirt_type = "perobject"` and a layer whose entities carry two `region_key.object_id` values whose bounding boxes are farther apart than `2 * (skirt_distance + skirt_loops * line_width)`, **when** `run_finalization` runs, **then** `2 * skirt_loops` skirt entities are pushed — one ring set per object, each sized to that object's own bbox and none matching the combined bbox; and **given** the same two objects moved to within that grouping offset, **then** exactly `skirt_loops` entities are pushed around their merged envelope (canonical `Print::_make_skirt` union-find semantics). With `skirt_type = "combined"` both cases push exactly `skirt_loops` entities around the all-object bbox. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `min_skirt_length = 20.0`, `skirt_loops = 1`, `line_width = 0.4`, `layer_height = 0.2`, `filament_diameter = 1.75` and a bbox whose first loop perimeter is too short to reach 20 mm of extruded filament, **when** skirt entities are generated, **then** more than `skirt_loops` rings are emitted, each successive ring offset one `line_width` further **outward**, and the emitted count is the smallest `n` for which the accumulated `Σ perimeter_i · e_per_mm` reaches 20.0, where `e_per_mm = ((line_width - layer_height) * layer_height + π * (layer_height / 2)²) / (π * (filament_diameter / 2)²)`; with `min_skirt_length = 0.0` exactly `skirt_loops` rings are emitted. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** the scheduler's config bounds index loaded from the real `skirt-brim.toml`, **when** `skirt_type = "outer_only"` (not in the enum values), `draft_shield = "limited"` (a canonical legacy value this port does not carry), or `skirt_start_angle = 200.0` (> max 180) is resolved, **then** each is rejected with the enum `TypeMismatch` / numeric `OutOfRange` error rather than reaching `ConfigView`. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** a resolved config with `filament_diameter = 2.85` and two tools in use, **when** the CONFIG_BLOCK is serialized, **then** the block contains `; filament_diameter = 2.85,2.85` exactly once (comma-joined, one entry per tool, built from the resolved value) and never the pre-packet hardcoded `1.75,1.75`; and a raw config that already supplies a `ConfigValue::List` for the key still wins verbatim. | `cargo test -p slicer-gcode --lib serialize::tests::config_block 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** a slice run whose resolved config carries an explicit `skirt_type = "perobject"`, **when** the G-code CONFIG_BLOCK is emitted, **then** the line `; skirt_type = perobject` is present exactly once; and when none of the five skirt keys is explicitly set, none of the five appears in the block and `ORCA_CONFIG_PADDING` gains no twins for them. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated module-key tables are checked, **then** all five skirt keys appear under the `skirt-brim` owner column and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'single_loop_draft_shield' docs/15_config_keys_reference.md && rg -q 'skirt_start_angle' docs/15_config_keys_reference.md && rg -q 'min_skirt_length' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** every one of the five skirt keys left absent from the config, **when** `run_finalization` runs over a multi-object, multi-layer collection, **then** the pushed entity list is identical (count, order, every vertex) to the pre-packet baseline captured in `tests/skirt_brim_tdd.rs` — the default path is unchanged by all five additions. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** `min_skirt_length = 1.0e9` (an unreachable target), **when** skirt entities are generated, **then** loop expansion terminates at the `MAX_MIN_LENGTH_LOOPS` cap without allocating unbounded rings, and the module returns `Ok` rather than hanging or erroring. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest schema guard, **when** any of the seven new keys is removed from `skirt-brim.toml` or its `type`/`default`/`values`/`min`/`max` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest and `src/lib.rs` are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) — governs how a module declares a host-owned key (`filament_diameter`) and receives it through `bind_module_config_view`; delegated SUMMARY.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — governs the `filament_diameter` array form and forbids padding edits; delegated SUMMARY.
- `docs/15_config_keys_reference.md` — generated; regenerate via `cargo xtask gen-config-docs`, verify with `--check`. Never hand-edited.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for the five skirt keys plus the `filament_diameter` re-declaration (owner column `skirt-brim`). The doc has no per-module subheadings, so verification is key-presence, not heading-presence; embodied as AC-10. The edit lands through `cargo xtask gen-config-docs`, never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the five keys (coType, default, min/max, enum value order). Authoring-time evidence is captured in `requirements.md` §Per-Key Canonical Evidence and is not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::_make_skirt` (the whole feature: per-instance `SkirtBrimGroupItem` construction, union-find grouping under `stCombined` vs `stPerObject` with `grouping_offset = skirt_distance + skirt_loops * spacing`, the `append_skirt_loops_for_hull` lambda's outward `offset` + `extruded_length` accumulation against `min_skirt_length`, and the inward-to-outward generation order that is `reverse()`d before export); `Print::has_infinite_skirt` (`draft_shield`); `Print::skirt_flow` (the `mm3_per_mm` source); `Print::object_skirt_offset` (out-of-scope context only).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::generate_skirt` (`single_loop_draft_shield`'s innermost-only `start_idx` on upper layers; the `first_layer && i == loops.first` rotated-start condition), `Skirt::find_start_point` (the bbox-center + half-diagonal angle formula), `GCode::generate_object_skirt_group` and `GCode::process_layer` (per-object vs combined emission points). Note: there is no `Skirt.cpp` in this checkout — the `Skirt` namespace is defined inline in `GCode.cpp`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
