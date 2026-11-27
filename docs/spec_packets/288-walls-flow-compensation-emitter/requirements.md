# Requirements: 288-walls-flow-compensation-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/62-author-packet-p55-quality-walls-and-surfaces-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P55 (Quality / Walls and surfaces 2/2, emitter) is eight Tier B keys whose canonical behaviours are emission-time flow decisions in `GCode::extrude_entity` and its small-area compensator, but this port emits E from `point.flow_factor` with no role-gated multiplier and no per-segment correction at all — five ratios and the compensation pair are true zero-occurrence gaps, and three of the ratios are armed by the master gate that packet 287 adopts. The eighth key in the same packet belongs to a different seam entirely (perimeter-avoiding travel needs a planner the port does not have), so authoring all eight into one emitter packet would repeat the declaration-only failure the map's Authoring rule 1 prohibits. This packet is the coherent slice: the five ratios plus the small-area pair as one emitter stage with a backward gate dependency, with the travel key returned to the queue with its missing feature named.

## In Scope

- Declare seven scalar-global keys with canonical defaults/bounds: `print_flow_ratio` (float `1.0`, `min 0.01`, `max 2` — the floor differs from every other ratio), `sparse_infill_flow_ratio` / `support_flow_ratio` / `support_interface_flow_ratio` / `top_solid_infill_flow_ratio` (float `1.0`, `min 0`, `max 2`), `small_area_infill_flow_compensation` (bool `false`), `small_area_infill_flow_compensation_model` (string holding canonical's ten-pair default table verbatim).
- Build the role-gated E multiplier in `DefaultGCodeEmitter::emit_gcode`: `print_flow_ratio` global and `top_solid_infill_flow_ratio` unconditional; sparse/support/interface gated on draft-287's `set_other_flow_ratios` (referenced, never redeclared — activation sequences after 287).
- Build the small-area per-segment correction at the same seam: when the bool is set and the model parses, each emitted segment's `dE` for `InternalSolidInfill` / `TopSolidInfill` / `BottomSolidInfill` scales by the model's line-length interpolation; all other roles and over-long segments pass through. The canonical pattern gate (`_needSAFC`'s rectilinear-family check) is deliberately omitted — the emitter has no pattern visibility (patterns are holder-selected module identity, rule 4 holder-only) — recorded as DEV-180(c); default holders are rectilinear-family so defaults stay faithful.
- Enforce canonical ranges as reject-the-slice validation (ticket-113 rule; deliberate divergence DEV-180(a) — canonical never enforces) including strict rejection of malformed model lines.
- Bypass `order_lock` paths (ADR-0062/0063): locked paths emit unscaled and uncompensated.
- Host-only omission from the CONFIG_BLOCK (ticket-42 precedent): no padding-table edit, default path byte-identical. (`reduce_crossing_wall`'s `("reduce_crossing_wall", "0")` twin stays untouched — the key is returned, not declared.)
- Annotate the 04 tier table + 05 packet list: P55 9→7+1 (seven in, gate shed to 287 already recorded, `reduce_crossing_wall` returned); returned key stays in-scope, unimplemented, with the missing planner named.

## Out of Scope

- `reduce_crossing_wall` (returned to the queue): the enable for perimeter-avoiding travel (`AvoidCrossingPerimeters::travel_to` + `init_layer`, `GCode.cpp`) over a planner the port does not have (path-optimization emits direct inter-region travel; the emitter consumes precomputed travels — ticket-61's detour precedent). Wiring the bool alone would be declaration-only (rule 1). Missing feature named (avoid-crossing-perimeters planner, shared with ticket-61's `max_travel_detour_distance`); no new ticket.
- `set_other_flow_ratios` (adopted into packet 287 by ticket 61): referenced here as a draft FORWARD-DEP, never redeclared. Redeclaring it would split the gate across two schemas.
- Per-tool (per-filament/per-extruder) vector variants of any ratio: canonical declares all seven scalar, so no ticket-125 model rides this packet. A vector future stays out of scope.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool/enum spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor re-spells bools.
- P54 keys (packet 287): not declared here; this packet depends backward on 287's gate when authored.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is in-module scaling, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (multipliers and the line-length interpolator input are unitless/symmetric; the segment length is computed in the emitter's working units on both sides of the scaling — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` role-to-ratio mapping (which role selects `sparse_infill` / `support` / `support_interface` under the `set_other_flow_ratios` gate vs `top_solid` / `print_flow_ratio` unconditional, and the composition position ahead of role ratios) (borrow the mapping order and gate split; port it emitter-side onto `entity.path.role`)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_needSAFC` plus `SmallAreaInfillFlowCompensator::modify_flow` (per-segment line-length interpolation gated to solid roles, constructed only when the bool is set and the model non-empty) (borrow the role set and per-segment position; the pattern gate is deliberately not borrowed — DEV-180(c))
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the seven keys' declared types/defaults/bounds (confirm `print_flow_ratio` floor `0.01` vs `0` elsewhere, the bool default, and the ten-pair model default verbatim; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `reduce_crossing_wall` travel-detour reads (`travel_to` via `AvoidCrossingPerimeters`, `init_layer`) (cite as the returned key's missing-planner evidence; borrow nothing)

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema, seven keys canonical) through `AC-5` (global print multiplier); refinements: AC-3 pins the three gated role mappings plus the two unconditional keys and the gate-inert arm; AC-4 pins the compensator's length interpolation, solid-role restriction, and bool-inert arm.
- Negative: `AC-N1` (bounds + malformed-model rejection incl. `print_flow_ratio`'s distinct floor); `AC-N2` (`order_lock` bypass per ADR-0062/0063 over both the multiplier and the compensator).
- Cross-packet impact: none at defaults (identity multipliers, inert compensator, host-only omitted, CONFIG_BLOCK stable); activation sequences after draft 287 (gate reference, never a redeclaration).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test walls_p55_flow_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. role/gate/small-area/global/lock behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean emission stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

287's gate is referenced here, so Step 1 declares seven keys exactly — `set_other_flow_ratios` must not appear in the schema diff. The emitter stage (Step 2) lands before its AC tests (Step 3); the bounds gate (Step 2b) lands with the stage, not after. `reduce_crossing_wall` leaves no code behind — its return is a tier-table annotation in Step 4, not a stub declaration.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check); the model string adds a parse-failure surface (owned by Step 2's strict-parse arm, not deferred to review).
