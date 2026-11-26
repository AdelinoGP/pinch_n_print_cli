# Requirements: 287-walls-flow-ratios-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/61-author-packet-p54-quality-walls-and-surfaces-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P54 (Quality / Walls and surfaces 1/2, emitter) is nine Tier B keys whose canonical behaviour is emission-time flow scaling in `GCode::extrude_entity`, but this port emits E from `point.flow_factor` with no role-gated multiplier at all — every one of the seven flow ratios is a true zero-occurrence gap, and the master gate that arms six of them lives undeclared in the next packet (P55). The two non-flow keys in the same packet belong to different seams entirely (per-region print order lives in runtime orchestration; perimeter-avoiding detour limits need a planner the port does not have), so authoring all nine into one emitter packet would repeat the declaration-only failure the map's Authoring rule 1 prohibits. This packet is the coherent slice: the seven ratios plus their gate as one emitter stage, with the two foreign-seam keys returned to the queue with their missing features named.

## In Scope

- Declare eight scalar-global keys with canonical defaults/bounds: `bottom_solid_infill_flow_ratio`, `first_layer_flow_ratio`, `gap_fill_flow_ratio`, `inner_wall_flow_ratio`, `internal_solid_infill_flow_ratio`, `outer_wall_flow_ratio`, `overhang_flow_ratio` (float `1.0`, `min 0`, `max 2`) and `set_other_flow_ratios` (bool `false`, adopted from P55 as a split-boundary adjustment).
- Build the role-gated E multiplier in `DefaultGCodeEmitter::emit_gcode`: bottom-solid unconditional; outer/inner/overhang/gap/internal-solid gated on `set_other_flow_ratios`; first-layer modifier on layer 0 excluding `Skirt`/`Brim`; overhang selection via the port's point-level overhang marking (DEV-179(b)).
- Enforce canonical ranges as reject-the-slice validation (ticket-113 rule; deliberate divergence DEV-179(a) — canonical never enforces).
- Bypass `order_lock` paths (ADR-0062/0063): locked paths emit unscaled.
- Host-only omission from the CONFIG_BLOCK (ticket-42 precedent): no padding-table edit, default path byte-identical.
- Annotate the 04 tier table + 05 packet list: P54 9→8 (two returned, gate adopted), P55 9→8 (gate shed); returned keys stay in-scope, unimplemented, with missing features named.

## Out of Scope

- `is_infill_first` (returned to the queue): per-region walls-vs-infill order plus the first-layer-always-walls exception belong to runtime orchestration (`assemble_ordered_entities_with_support_identities`), not emission — the emitter preserves `ordered_entities` and must not reorder. Missing feature named in the tier table; no new ticket (ticket-42 adaptive precedent — a future claim packets it, possibly folding into print-orchestration/object-planning work).
- `max_travel_detour_distance` (returned to the queue): zero-disables detour cap over a perimeter-avoiding planner the port does not have (path-optimization emits direct inter-region travel; the emitter consumes precomputed travels). Wiring a limit with no planner would be declaration-only (rule 1). Missing feature named (avoid-crossing-perimeters planner); no new ticket.
- Per-tool (per-filament/per-extruder) vector variants of any ratio: canonical declares all eight scalar, so no ticket-125 model rides this packet. A vector future stays out of scope.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool/enum spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor re-spells bools.
- Support/sparse/top-solid/print flow ratios (P55, ticket 62): not declared here; P55 depends backward on this packet's gate when authored.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is in-module scaling, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (multipliers are unitless; no mm↔unit math — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` role-to-ratio mapping order (which role selects which of the seven ratios, `bottom_solid` unconditional vs `set_other_flow_ratios`-gated six, first-layer modifier with brim/skirt exclusion) (borrow the mapping order and exclusion set; port it emitter-side onto `entity.path.role` + layer index)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_entity` flow composition shape (geometric volume × print flow × filament flow, then role ratios) (borrow the composition position: multiply the port's `point.flow_factor`-derived E, do not rebuild the volume term)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema, eight keys canonical) through `AC-5` (overhang point-marking selection); refinements: AC-3 pins the five direct role mappings plus bottom-solid unconditionality; AC-4 pins the layer-0 index source (`global_layer_index == 0`) and the exact `Skirt`/`Brim` exclusion set.
- Negative: `AC-N1` (bounds rejection incl. word-form bool spelling riding ticket 132); `AC-N2` (`order_lock` bypass per ADR-0062/0063).
- Cross-packet impact: none at defaults (identity multipliers, host-only omitted, CONFIG_BLOCK stable); P55 (ticket 62) depends backward on this packet's `set_other_flow_ratios` declaration — its packet must not redeclare the key.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test flow_ratio_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. role/gate/first-layer/overhang/lock behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean emission stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

P55's gate is adopted here, so Step 1 declares all eight keys exactly once — P55 must reference, never redeclare. The emitter stage (Step 2) lands before its AC tests (Step 3); the bounds gate (Step 2b) lands with the stage, not after. `is_infill_first` / `max_travel_detour_distance` leave no code behind — their return is a tier-table annotation in Step 4, not a stub declaration.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check).
