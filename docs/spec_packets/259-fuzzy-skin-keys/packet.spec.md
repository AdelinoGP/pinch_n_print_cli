---
status: draft
packet: fuzzy-skin-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/14-author-packet-p07-others-fuzzy-skin-fuzzy-skin.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P07)
context_cost_estimate: M
---

# Packet Contract: fuzzy-skin-keys

## Goal

Declare the seven OrcaSlicer fuzzy-skin keys (`fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`) in the `fuzzy-skin` module manifest with canonical types/defaults/bounds, and wire the two live decision points — `fuzzy_skin`'s loop-selection gate (which wall loops are fuzz candidates) and `fuzzy_skin_first_layer`'s layer-0 gate — in `FuzzySkinModule`, declaring the other five as declared-with-gap (their canonical decision points — the Arachne extrusion-line mode switch and the libnoise coherent-noise modules — do not exist in this tree).

## Scope Boundaries

The packet touches the `fuzzy-skin` core module only: its TOML manifest, its `src/lib.rs` wiring, and its test directory — plus one integration arm each in `slicer-scheduler` (bounds/enum enforcement) and `slicer-runtime` (CONFIG_BLOCK reachability), a one-line value correction in the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`: the pre-existing `fuzzy_skin` padding entry says `"none"`, contradicting the canonical default `disabled_fuzzy` this packet declares — corrected, with no entries gained or lost), and the generated `docs/15_config_keys_reference.md` tables. It does not introduce coherent-noise generation (Perlin/Billow/RidgedMulti/Voronoi/Ripple), Arachne extrusion-line width-modifying fuzz (`fuzzy_skin_mode`), hole-loop identification (the IR has no `LoopType::Hole`), or painted-region segmentation (canonical `apply_fuzzy_skin_segmentation`); those stay recorded gaps (queue rows, not this packet's implementation surface).

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 259 derived from disk at authoring time); ticket 05 (packet-list P07 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence); ticket 103 (fuzzy-skin rename — resolved; the module already carries the Orca names `fuzzy_skin_thickness`/`fuzzy_skin_point_distance`).
- Ordering, not gating: packet 258 (P06, owner `skirt-brim`) precedes this packet in the queue but touches a different module — no same-module merge churn.
- Unblocks: wayfinder ticket 14's resolution; nothing downstream gates on this packet specifically (no other queued packet consumes `fuzzy-skin` keys).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `fuzzy-skin` module manifest, **when** its `[config.schema]` is parsed, **then** it contains exactly these seven new table entries — `fuzzy_skin` (`type = "enum"`, `values = ["none", "external", "hole", "all", "allwalls", "disabled_fuzzy"]`, `default = "disabled_fuzzy"`, `group = "Fuzzy Skin"`), `fuzzy_skin_first_layer` (`type = "bool"`, `default = false`, `group = "Fuzzy Skin"`), `fuzzy_skin_mode` (`type = "enum"`, `values = ["displacement", "extrusion", "combined"]`, `default = "displacement"`, `group = "Fuzzy Skin"`), `fuzzy_skin_noise_type` (`type = "enum"`, `values = ["classic", "perlin", "billow", "ridgedmulti", "voronoi", "ripple"]`, `default = "classic"`, `group = "Fuzzy Skin"`), `fuzzy_skin_octaves` (`type = "int"`, `default = 4`, `min = 1`, `max = 10`, `group = "Fuzzy Skin"`), `fuzzy_skin_persistence` (`type = "float"`, `default = 0.5`, `min = 0.01`, `max = 1`, `group = "Fuzzy Skin"`), `fuzzy_skin_scale` (`type = "float"`, `default = 1.0`, `min = 0.1`, `max = 500`, `group = "Fuzzy Skin"`) — with all seven carrying `display` and `group = "Fuzzy Skin"`. | `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a module config with `fuzzy_skin = "external"` and `fuzzy_skin_first_layer = true` over a region carrying one outer wall (perimeter_index 0) and one inner wall, **when** `FuzzySkinModule::run_wall_postprocess` executes at layer 0, **then** the outer wall's path is perturbed and the inner wall's path is byte-identical to its input; with `fuzzy_skin = "disabled_fuzzy"` (or absent) every wall passes through byte-identical; with `fuzzy_skin = "all"` the outer wall is perturbed and the inner wall passes through (canonical's hole half of `all` has no IR representation — recorded divergence); with `fuzzy_skin = "allwalls"` both the outer and inner walls are perturbed; with `fuzzy_skin = "hole"` every wall passes through byte-identical (no hole-loop identification exists in the IR — recorded gap). | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `fuzzy_skin = "external"` in a module config, **when** `run_wall_postprocess` executes at layer 0 with `fuzzy_skin_first_layer` absent (default false), **then** every wall passes through byte-identical; at layer 1 the outer wall is perturbed; and with `fuzzy_skin_first_layer = true` the outer wall is perturbed at layer 0 as well. | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** the scheduler's config bounds index loaded from the real `fuzzy-skin.toml` manifest, **when** a CLI/sidecar value `fuzzy_skin = "bogus"` (not in the enum values), `fuzzy_skin_octaves = 0` (< min 1), or `fuzzy_skin_scale = 600.0` (> max 500) is resolved, **then** resolution rejects the value with the standard enum `TypeMismatch` / numeric `OutOfRange` error instead of passing it through to `ConfigView`. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a slice run whose resolved config carries an explicit `fuzzy_skin = "external"` value, **when** the G-code CONFIG_BLOCK is emitted, **then** the line `; fuzzy_skin = external` is present exactly once (the padding entry is dedup-suppressed by `emit_config_kv`); and at defaults the block emits `; fuzzy_skin = disabled_fuzzy` and `; fuzzy_skin_mode = displacement` — the two pre-existing `ORCA_CONFIG_PADDING` entries, the former corrected from `"none"` to the canonical default by this packet (no entries gained or lost) — and none of the other five keys appears (packet 254/255/257/258 precedent: non-percent manifest defaults do not thread into raw config, and no padding twins are added). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated module-key tables are checked, **then** all seven keys appear with canonical types/defaults under the `fuzzy-skin` owner column and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'fuzzy_skin_scale' docs/15_config_keys_reference.md && rg -q 'fuzzy_skin_first_layer' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** `fuzzy_skin = "external"` and `fuzzy_skin_first_layer = true` set in a module config, **when** the five declared-with-gap keys are additionally set (`fuzzy_skin_mode = "combined"`, `fuzzy_skin_noise_type = "perlin"`, `fuzzy_skin_octaves = 8`, `fuzzy_skin_persistence = 0.8`, `fuzzy_skin_scale = 2.0`) and `run_wall_postprocess` executes, **then** the output is identical to the same run with the five keys absent — declaring them must not perturb behavior, because their canonical consumers (the Arachne extrusion-line mode switch, the libnoise coherent-noise modules) do not exist in this tree. | `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the manifest schema guard, **when** any of the seven new keys is removed from `fuzzy-skin.toml` or its `type`/`default`/`values`/`min`/`max` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest and `src/lib.rs` are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker needs the `[config.schema]` contract; the enum `values` field form is grounded in-tree (`seam-planner-default.toml`, `tree-support-planner.toml`).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for the seven new keys (owner column `fuzzy-skin`); the doc has no per-module subheadings, so verification is key-presence, not heading-presence. Verification grep: `rg -q 'fuzzy_skin_scale' docs/15_config_keys_reference.md && rg -q 'fuzzy_skin_first_layer' docs/15_config_keys_reference.md` (embodied as AC-6, key-presence probe against the real generated doc). The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 4), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `should_fuzzify` (the type/first-layer gates: `fuzzify_contours`/`fuzzify_holes` loop selection, `!config.fuzzy_first_layer && layer_id <= 0`), `apply_fuzzy_skin` (Polygon and `Arachne::ExtrusionLine` overloads), `fuzzy_polyline` (the port's algorithm source), `fuzzy_extrusion_line` (the `switch (cfg.mode)` — Arachne-only), `get_noise_module` (libnoise module construction: `SetFrequency(1 / cfg.noise_scale)`, `SetOctaveCount(cfg.noise_octaves)`, `SetPersistence(cfg.noise_persistence)`; `UniformNoise` for classic), `fuzzy_polyline_ripple`/`fuzzy_extrusion_line_ripple` (Ripple dispatch).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the seven keys (types, defaults, min/max bounds, enum value order, mode); authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `PRINT_CONFIG_CLASS_DEFINE(PrintRegionConfig, ...)` member list (all seven keys live in `PrintRegionConfig`, the per-region config class).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic`/`process_arachne` call sites (unconditional; the gate is inside `should_fuzzify`), `group_region_by_fuzzify` (recorded-gap context for the `none`/painted path).
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `apply_fuzzy_skin_segmentation` (painted-region segmentation; recorded-gap context for `none`).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
