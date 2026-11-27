---
status: draft
packet: 288-walls-flow-compensation-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/62-author-packet-p55-quality-walls-and-surfaces-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 62 (P55).
---

# Packet Contract: 288-walls-flow-compensation-emitter

## Goal

Make the P55 flow family drive host-side emission at parity with canonical `GCode::extrude_entity` role-gated flow plus the small-area infill compensator — ported as a role-gated multiplier stage and a per-segment line-length correction inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P55 is eight Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 62) splits it two ways: five flow ratios plus the small-area bool/model pair are zero-occurrence as behaviour and need new emitter logic (this packet); `reduce_crossing_wall` is padding-only with no decision point (the port has no avoid-crossing-perimeters planner — the emitter consumes precomputed travels). The packet covers seven keys: the five ratios plus the small-area pair. The travel key is returned to the queue as unimplemented with its missing feature named; no queue-count change. Three of the five ratios are gated on packet 287's `set_other_flow_ratios` (draft — backward dependency, never redeclared here). Canonical declares all seven scalar (verified against the map oracle at authoring), so no per-tool vector model rides ticket 125. Defaults are identity, so the default path is byte-identical and no CONFIG_BLOCK line is gained or lost (host-only omitted, ticket-42 precedent; table untouched).

## Prerequisites and Blockers

- Depends on: draft packet 287 (`docs/spec_packets/287-walls-flow-ratios-emitter/`, `status: draft`) for the `set_other_flow_ratios` gate declaration only — explicit FORWARD-DEP, not a satisfied dependency. This packet references the gate; it never redeclares it. Activation sequences after 287 lands.
- Unblocks: wayfinder ticket 62 (P55 closes with one key returned). No dependency edge to any other draft packet: the flow stage multiplies E at emission, independent of 285/286's slope/wipe move shaping and 277's retract wipe.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-180 is the next collision-free ID (LOG max DEV-171; drafts propose DEV-172–DEV-179 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all seven keys are declared as scalar-global with canonical defaults — `print_flow_ratio` float `1.0` (`min 0.01`, `max 2`), `sparse_infill_flow_ratio` float `1.0` (`min 0`, `max 2`), `support_flow_ratio` float `1.0` (`min 0`, `max 2`), `support_interface_flow_ratio` float `1.0` (`min 0`, `max 2`), `top_solid_infill_flow_ratio` float `1.0` (`min 0`, `max 2`), `small_area_infill_flow_compensation` bool `false`, `small_area_infill_flow_compensation_model` string holding canonical's ten-pair default table — with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd schema_declares_all_seven_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (all ratios `1.0`, compensation `false`), **when** a mixed-role fixture (sparse, support, interface, top-solid, solid, walls) is emitted, **then** every emitted E equals the pre-packet E (all multipliers `1.0`, compensator inert) — geometry and E byte-identical — and the CONFIG_BLOCK keeps its line count with zero value changes (host-only omitted, no padding twin shadowed). | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd default_path_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** draft-287 gate `set_other_flow_ratios = true` with one gated ratio at `0.5` and the rest at `1.0`, **when** the mixed-role fixture is emitted, **then** only paths of the selected role emit at half E (`SparseInfill`, `SupportMaterial`, `SupportInterface` — `crates/slicer-ir/src/slice_ir.rs` `ExtrusionRole`) while all other roles emit at full E; with the gate `false` the same non-default ratio emits at full E (gate inert); `top_solid_infill_flow_ratio` and `print_flow_ratio` ignore the gate (canonical unconditional). | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd role_gating_and_master_gate 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `small_area_infill_flow_compensation = true` with the canonical default model, **when** short solid-role segments (length under the model's max) and long segments are emitted, **then** short `InternalSolidInfill` / `TopSolidInfill` / `BottomSolidInfill` segments emit scaled E per the model's interpolation while long segments emit at full E, non-solid roles (walls, sparse, support) emit at full E regardless of length, and with the bool `false` the same short segments emit at full E. | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd small_area_compensation_applies_by_length_and_role 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `print_flow_ratio = 0.5` with all other ratios at `1.0` and the gate `false`, **when** the mixed-role fixture is emitted, **then** every role emits at half E (global multiplier, canonical `GCode::extrude_entity` position ahead of role ratios); with `print_flow_ratio = 1.0` all roles emit identically to defaults. | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd print_flow_ratio_scales_all_roles 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (any ratio `-0.1` or `2.1`, `print_flow_ratio = 0.0` below its `0.01` floor, compensation bool spelled `"yes"`, model text with a malformed `length,factor` line), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins/maxes are GUI hints — DEV-180(a)). | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a path carrying `order_lock` (ADR-0062/0063), **when** emission runs with all ratios at `0.5`, compensation `true`, and the gate `true`, **then** the locked path emits at full E (producer-guaranteed footprint; the emitter neither rescales nor compensates it), while neighbouring unlocked paths of the same role emit scaled E. | `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd locked_paths_bypass_flow_scaling 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — flow ratios must not become claim-held algorithm selectors; rule 4 trigger test does not fire: in-module emission scaling, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'support_flow_ratio' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'support_flow_ratio' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-180" - `rg -q 'DEV-180' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` role-to-ratio mapping (which role selects `sparse_infill` / `support` / `support_interface` under the `set_other_flow_ratios` gate vs `top_solid` / `print_flow_ratio` unconditional, and the composition position ahead of role ratios) (borrow the mapping order and gate split; port it emitter-side onto `entity.path.role`)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_needSAFC` plus `SmallAreaInfillFlowCompensator::modify_flow` (per-segment line-length interpolation gated to solid roles, constructed only when the bool is set and the model non-empty) (borrow the role set and per-segment position; the pattern gate is deliberately not borrowed — DEV-180(c))
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the seven keys' declared types/defaults/bounds (confirm `print_flow_ratio` floor `0.01` vs `0` elsewhere, the bool default, and the ten-pair model default verbatim; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `reduce_crossing_wall` travel-detour reads (`travel_to` via `AvoidCrossingPerimeters`, `init_layer`) (cite as the returned key's missing-planner evidence; borrow nothing)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
