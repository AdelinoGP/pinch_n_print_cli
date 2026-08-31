---
status: draft
packet: brim-type-and-brim-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P05)
context_cost_estimate: M
---

# Packet Contract: brim-type-and-brim-keys

## Goal

Declare the five in-scope OrcaSlicer brim keys (`brim_type`, `brim_object_gap`, `brim_ears_max_angle`, `brim_ears_detection_length`, `brim_use_efc_outline`) in the `skirt-brim` module manifest with canonical types/defaults/bounds, and wire the one live decision point — `brim_type`'s `no_brim` gate on brim generation — in `SkirtBrim`, declaring the four keys whose canonical decision points (ear detection, object-contour gap, EFC-compensated outline) do not exist in this tree as declared-with-gap.

## Scope Boundaries

The packet touches the `skirt-brim` core module only: its TOML manifest, its `src/lib.rs` gate, and its test directory — plus one integration arm each in `slicer-scheduler` and `slicer-runtime` for bound enforcement and CONFIG_BLOCK reachability. It does not introduce ear geometry, inner/outer brim contours, object-contour offsetting, or elephant-foot-compensation coupling; those stay recorded gaps (queue rows, not this packet's implementation surface).

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 257 derived from disk at authoring time); ticket 05 (packet-list P05 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence).
- Unblocks: wayfinder ticket 12's resolution; nothing downstream gates on this packet specifically (no other queued packet consumes `skirt-brim` keys).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `skirt-brim` module manifest, **when** its `[config.schema]` is parsed, **then** it contains exactly these five new table entries — `brim_type` (`type = "enum"`, `values = ["auto_brim", "brim_ears", "painted", "outer_only", "inner_only", "outer_and_inner", "no_brim"]`, `default = "auto_brim"`, `group = "Skirt/Brim"`), `brim_object_gap` (`type = "float"`, `default = 0.0`, `min = 0.0`, `max = 2.0`), `brim_ears_max_angle` (`type = "float"`, `default = 125.0`, `min = 0.0`, `max = 180.0`), `brim_ears_detection_length` (`type = "float"`, `default = 1.0`, `min = 0.0`, no `max`), `brim_use_efc_outline` (`type = "bool"`, `default = false`) — with all five in `group = "Skirt/Brim"`. | `cargo test -p skirt-brim --test brim_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** `brim_width > 0` and `brim_type = "no_brim"` in a module config, **when** `SkirtBrim::run_finalization` executes over a non-empty layer set, **then** zero brim entities are pushed to the layer-0 output while skirt generation is unaffected. | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `brim_width > 0` and `brim_type` absent from the config (or `"auto_brim"`), **when** `SkirtBrim::run_finalization` executes, **then** the layer-0 brim output is byte-identical in shape (count, role, bbox offsets) to the pre-packet behavior — the default path is unchanged. | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** the scheduler's config bounds index loaded from the real `skirt-brim.toml` manifest, **when** a CLI/sidecar value `brim_type = "elephant"` (not in the enum values) or `brim_object_gap = 3.0` (> max 2.0) is resolved, **then** resolution rejects the value with the standard out-of-bounds error instead of passing it through to `ConfigView`. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a slice run whose resolved config carries an explicit `brim_type` value, **when** the G-code CONFIG_BLOCK is emitted, **then** the line `; brim_type = <value>` is present exactly once (user/resolved value wins; the static padding entry does not duplicate it). | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Negative Test Cases

- **AC-N1. Given** `brim_width = 0` and `brim_type = "auto_brim"`, **when** `SkirtBrim::run_finalization` executes, **then** no brim entities are pushed — declaring `brim_type` must not enable brim generation when the existing `brim_width > 0` gate says otherwise. | `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the manifest schema guard, **when** any of the five new keys is removed from `skirt-brim.toml` or its `default`/`values`/`min`/`max` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p skirt-brim --test brim_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked for the `skirt-brim` module, **then** all five keys appear with canonical types/defaults and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'brim_use_efc_outline' docs/15_config_keys_reference.md; echo "exit=$?"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p skirt-brim --test brim_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest is a guest-fingerprint input (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — delegated SUMMARY; governs whether padding may be touched (ruling: it may not).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - the five new manifest-declared keys appear in the generated module-key tables with canonical defaults. Verification grep: `rg -q '^## skirt-brim' docs/15_config_keys_reference.md && rg -q 'brim_use_efc_outline' docs/15_config_keys_reference.md` (append as AC-6 below). The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 4), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the five keys (types, defaults, min/max, enum order, UI category); authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area` (mode gating, `brim_object_gap` offset application), `make_brim_ears_auto` (ear angle/detection-length semantics), `use_brim_efc_outline` (EFC-outline gate conditions) — the recorded-gap descriptions rest on these three functions.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::has_brim` (the `brim_type`/`brim_width`/raft interplay the declaration must express when the port's gate eventually grows modes).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).