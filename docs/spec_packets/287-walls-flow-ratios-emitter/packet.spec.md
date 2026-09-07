---
status: draft
packet: 287-walls-flow-ratios-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/61-author-packet-p54-quality-walls-and-surfaces-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 61 (P54).
---

# Packet Contract: 287-walls-flow-ratios-emitter

## Goal

Make the P54 flow-ratio family drive host-side emission at parity with canonical `GCode::extrude_entity` role-gated flow — ported as a new role-gated multiplier stage inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P54 is nine Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 61, 2026-09-07) split it three ways: the seven flow ratios are zero-occurrence as behaviour and need new emitter logic (this packet); `is_infill_first` is zero-occurrence with the wrong owner (ordering lives in runtime orchestration, not emission); `max_travel_detour_distance` is padding-only with no decision point (the port has no avoid-crossing-perimeters planner — the emitter consumes precomputed travels). The packet covers eight keys: the seven ratios plus the `set_other_flow_ratios` master gate adopted from P55 as a split-boundary adjustment (the 05 list calls split boundaries proposals; the gate controls this family's ratios, so the decision lives in one place — ticket-35 fold precedent). The two ordering/travel keys are returned to the queue as unimplemented with their missing features named; no queue-count change. Canonical declares all eight scalar (verified against the map oracle at authoring), so no per-tool vector model rides ticket 125. Defaults are identity (`1.0` ratios, gate `false`), so the default path is byte-identical and no CONFIG_BLOCK line is gained or lost (host-only omitted, ticket-42 precedent; table untouched).

## Prerequisites and Blockers

- Depends on: none for authoring. Implementation is standalone — the gate is adopted into this packet, so no forward dependency on P55 (ticket 62); P55 sheds the gate (9→8) and its remaining ratios depend backward on this packet's gate when authored.
- Unblocks: wayfinder ticket 61 (P54 closes with two keys returned; P55 re-sized 9→8). No dependency edge to any draft packet: the flow stage multiplies E at emission, independent of 285/286's slope/wipe move shaping and 277's retract wipe.
- Activation blockers: none. All symbols below are live on HEAD (verified 2026-09-07); DEV-179 is the next collision-free ID (LOG max DEV-171; drafts propose DEV-172–DEV-178 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all eight keys are declared as scalar-global with canonical defaults — `bottom_solid_infill_flow_ratio` float `1.0` (`min 0`, `max 2`), `first_layer_flow_ratio` float `1.0` (`min 0`, `max 2`), `gap_fill_flow_ratio` float `1.0` (`min 0`, `max 2`), `inner_wall_flow_ratio` float `1.0` (`min 0`, `max 2`), `internal_solid_infill_flow_ratio` float `1.0` (`min 0`, `max 2`), `outer_wall_flow_ratio` float `1.0` (`min 0`, `max 2`), `overhang_flow_ratio` float `1.0` (`min 0`, `max 2`), `set_other_flow_ratios` bool `false` — with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (all ratios `1.0`, gate `false`), **when** a mixed-role fixture (outer/inner walls, gap fill, internal/bottom solid, overhang-marked walls, first layer + higher layer) is emitted, **then** every emitted E equals the pre-packet E (multiplier `1.0` throughout; gate `false` leaves the six gated ratios inert and `bottom_solid_infill_flow_ratio` at `1.0` is identity) — geometry and E byte-identical — and the CONFIG_BLOCK keeps its line count with zero value changes (host-only omitted, no padding twin shadowed). | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd default_path_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `set_other_flow_ratios = true` with one ratio at `0.5` and the rest at `1.0`, **when** the mixed-role fixture is emitted above the first layer, **then** only paths of the selected role emit at half E (outer→`OuterWall`, inner→`InnerWall`, gap→`GapFill`, internal-solid→`InternalSolidInfill`, bottom-solid→`BottomSolidInfill` unconditional on the gate, overhang→overhang-marked walls) while all other roles emit at full E; with the gate `false` the same non-default ratio emits at full E (gate inert). | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd role_gating_and_master_gate 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `set_other_flow_ratios = true` and `first_layer_flow_ratio = 0.5`, **when** layer 0 and layer 1 are emitted, **then** layer-0 eligible roles emit at half E and layer-1 roles at full E; layer-0 `Skirt`/`Brim` roles emit at full E regardless (canonical exclusion); with `first_layer_flow_ratio = 1.0` both layers emit identically. | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd first_layer_modifier_and_exclusions 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `set_other_flow_ratios = true` and `overhang_flow_ratio = 0.5`, **when** walls with and without overhang marking are emitted, **then** only the overhang-marked walls emit at half E (selection via the port's point-level overhang marking, DEV-179(b)) while unmarked walls of the same role emit at full E. | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd overhang_selection_via_point_marking 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (any ratio `-0.1` or `2.1`, `set_other_flow_ratios = "yes"`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins/maxes are GUI hints — DEV-179(a)). | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a path carrying `order_lock` (ADR-0062/0063), **when** emission runs with all ratios at `0.5` and the gate `true`, **then** the locked path emits at full E (producer-guaranteed footprint; the emitter neither rescales nor reshapes it), while neighbouring unlocked paths of the same role emit at half E. | `cargo test -p slicer-gcode --test flow_ratio_emission_tdd locked_paths_bypass_flow_scaling 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test flow_ratio_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — flow ratios must not become claim-held algorithm selectors; rule 4 trigger test does not fire: in-module emission scaling, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'outer_wall_flow_ratio' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'outer_wall_flow_ratio' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-179" - `rg -q 'DEV-179' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` role-to-ratio mapping order (which role selects which of the seven ratios, `bottom_solid` unconditional vs `set_other_flow_ratios`-gated six, first-layer modifier with brim/skirt exclusion) (borrow the mapping order and exclusion set; port it emitter-side onto `entity.path.role` + layer index)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` flow composition shape (geometric volume × print flow × filament flow, then role ratios) (borrow the composition position: multiply the port's `point.flow_factor`-derived E, do not rebuild the volume term)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
