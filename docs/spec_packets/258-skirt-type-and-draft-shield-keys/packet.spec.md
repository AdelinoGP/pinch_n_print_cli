---
status: draft
packet: skirt-type-and-draft-shield-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/13-author-packet-p06-others-skirt-skirt-brim.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P06)
context_cost_estimate: M
---

# Packet Contract: skirt-type-and-draft-shield-keys

## Goal

Declare the five OrcaSlicer skirt/draft-shield keys (`skirt_type`, `min_skirt_length`, `skirt_start_angle`, `draft_shield`, `single_loop_draft_shield`) in the `skirt-brim` module manifest with canonical types/defaults/bounds, and wire the three live decision points — `draft_shield`'s enable-the-skirt-on-every-layer span, `single_loop_draft_shield`'s one-wall-per-upper-layer loop count, and `skirt_start_angle`'s first-loop start corner on the first layer — in `SkirtBrim`, declaring `skirt_type` and `min_skirt_length` as declared-with-gap (their canonical decision points do not exist in this tree).

## Scope Boundaries

The packet touches the `skirt-brim` core module only: its TOML manifest, its `src/lib.rs` wiring, and its test directory — plus one integration arm each in `slicer-scheduler` (bounds/enum enforcement) and `slicer-runtime` (CONFIG_BLOCK reachability), and the generated `docs/15_config_keys_reference.md` tables. It does not introduce per-object skirt grouping, extruded-length-limited loop expansion, contour-offset skirt geometry (the port's bounding-box rect loops stay), or seam-style start-point re-seating on the path itself; those stay recorded gaps (queue rows, not this packet's implementation surface).

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 258 derived from disk at authoring time); ticket 05 (packet-list P06 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence).
- Ordering, not gating: packet 257 (P05, same owner module `skirt-brim`) precedes this packet in the queue and touches the same manifest and `SkirtBrim::from_config`. Implement 257 first to avoid same-module merge churn. This packet's Step 1 adds the `toml = "0.8"` dev-dependency add-if-absent so either implementation order works.
- Unblocks: wayfinder ticket 13's resolution; nothing downstream gates on this packet specifically (no other queued packet consumes `skirt-brim` keys).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `skirt-brim` module manifest, **when** its `[config.schema]` is parsed, **then** it contains exactly these five new table entries — `skirt_type` (`type = "enum"`, `values = ["combined", "perobject"]`, `default = "combined"`, `group = "Skirt/Brim"`), `min_skirt_length` (`type = "float"`, `default = 0.0`, `min = 0.0`, no `max`), `skirt_start_angle` (`type = "float"`, `default = -135.0`, `min = -180.0`, `max = 180.0`), `draft_shield` (`type = "enum"`, `values = ["disabled", "enabled"]`, `default = "disabled"`, `group = "Skirt/Brim"`), `single_loop_draft_shield` (`type = "bool"`, `default = false`, `group = "Skirt/Brim"`) — with all five carrying `display` and `group = "Skirt/Brim"`. | `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `skirt_loops > 0` and `draft_shield = "enabled"` in a module config, **when** `SkirtBrim::run_finalization` executes over a multi-layer set, **then** every layer in the set receives `skirt_loops` skirt entities (the shield span ignores `skirt_height`), while brim generation stays layer-0-only; with `draft_shield` absent or `"disabled"`, the layer span is unchanged from the pre-packet behavior (`min(skirt_height, layer count)`). | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `single_loop_draft_shield = true` and `skirt_loops > 1`, **when** skirt entities are generated for a layer with `global_layer_index > 0`, **then** exactly one skirt loop is produced (the innermost, nearest-the-object rect loop), while the first layer keeps the full `skirt_loops` set; with the key absent or `false`, every layer keeps `skirt_loops` loops. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `skirt_start_angle = 45.0` and `skirt_loops >= 1`, **when** the first-layer first (innermost) skirt loop is generated, **then** the loop's start vertex is the rect corner nearest the point `(bbox_center + r·(cos 45°, sin 45°))` with `r` = half-diagonal of the loop's own bounding box — for a non-degenerate rect under 45° that is the corner `(x_max, y_max)` — and every other loop (and every loop on later layers) still starts at `(x_min, y_min)`. At the default `-135.0` the selected corner is `(x_min, y_min)`, so the default path is byte-identical to the pre-packet behavior. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** the scheduler's config bounds index loaded from the real `skirt-brim.toml` manifest, **when** a CLI/sidecar value `skirt_type = "outer_only"` (not in the enum values) or `skirt_start_angle = 200.0` (> max 180) is resolved, **then** resolution rejects the value with the standard enum `TypeMismatch` / numeric `OutOfRange` error instead of passing it through to `ConfigView`. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a slice run whose resolved config carries an explicit `skirt_type = "perobject"` value, **when** the G-code CONFIG_BLOCK is emitted, **then** the line `; skirt_type = perobject` is present exactly once; and when none of the five keys is explicitly set, none of the five appears in the block (packet 254/255/257 precedent: non-percent manifest defaults do not thread into raw config, and `ORCA_CONFIG_PADDING` gains no twins for these keys). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated module-key tables are checked, **then** all five keys appear with canonical types/defaults under the `skirt-brim` owner column and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'single_loop_draft_shield' docs/15_config_keys_reference.md && rg -q 'skirt_start_angle' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** `min_skirt_length = 5.0` and `skirt_type = "perobject"` set in a module config (the two declared-with-gap keys), **when** `SkirtBrim::from_config` constructs and `run_finalization` executes, **then** the output is identical to the same run with both keys absent — declaring them must not perturb behavior, because their canonical consumers (per-object skirt grouping, extruded-length-limited loop expansion) do not exist in this tree. | `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the manifest schema guard, **when** any of the five new keys is removed from `skirt-brim.toml` or its `type`/`default`/`values`/`min`/`max` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest and `src/lib.rs` are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — delegated SUMMARY; governs whether padding may be touched (ruling: it may not).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for the five new keys (owner column `skirt-brim`); the doc has no per-module subheadings, so verification is key-presence, not heading-presence. Verification grep: `rg -q 'single_loop_draft_shield' docs/15_config_keys_reference.md && rg -q 'skirt_start_angle' docs/15_config_keys_reference.md` (embodied as AC-7, corrected from a key-presence probe against the real generated doc). The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 5), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the five keys (types, defaults, min/max bounds, enum value order, mode); authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::has_infinite_skirt` (draft_shield → skirt on every layer), `Print::_make_skirt` (min_skirt_length's extruded-length loop expansion, skirt_type combined/per-object grouping, inner-to-outward loop generation with the exported list reversed outermost-first), `Print::object_skirt_offset` (recorded-gap context only).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::generate_skirt` (single_loop_draft_shield's `start_idx = loops.second - 1` innermost single wall on upper layers; the `first_layer && i == loops.first` rotated-start condition) and `Skirt::find_start_point` (the bbox-center + half-diagonal angle formula the port's start-corner wiring mirrors).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).