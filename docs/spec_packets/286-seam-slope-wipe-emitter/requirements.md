# Requirements: 286-seam-slope-wipe-emitter

## Packet Metadata

- Grouped task IDs: none (wayfinder queue packet; no `TASK-###` slice applies)
- Backlog source: `docs/specs/orca-feature-gap/issues/60-author-packet-p53-quality-seam-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (sum of per-step costs in `implementation-plan.md`)

## Problem Statement

P53 (Quality / Seam 2/2, host emitter) is the slope-variant and loop-wipe half of the seam feature: canonical `GCode::extrude_loop` gates a sloped-seam ramp on `seam_slope_type` and five modifiers, and emits inward loop wipes on `wipe_before_external_loop` / `wipe_on_loops`. This port emits no slope ramp and no loop wipe — six keys have zero occurrences under `crates/`/`modules/`/`xtask/`, and the other two exist only as `ORCA_CONFIG_PADDING` twins (rule 2: not evidence). Packet 285 built the scarf/slope emission stage this packet generalises additively; without this packet its `seam_slope_type`-family behaviour and all loop wiping stay uncovered. One coherent slice: all eight keys land in the same `emit_gcode` loop region, behind the same gates, in one session-sized change.

## In Scope

- Eight scalar-global `ResolvedConfig` fields at canonical defaults (table in `packet.spec.md` AC-1), with `docs/config/host-keys.toml` `[resolved_config]` rows and `machine-gcode-emit.toml` `[config.schema.*]` tables; the new guard binary `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs` is authored in Step 1 (AC verification rule: the asserted behaviour needs an emit driver, and no existing binary drives `emit_gcode` loop fixtures for slope/wipe).
- Slope ramp generalising 285's scarf stage: type/loop-match gate, inner-walls gate, min-length gate, steps-modulated ramp, entire-loop variant, start-height shift (AC-2–AC-5).
- Loop-end wipe site: pre-leave move (`wipe_on_loops`), pre-external move (`wipe_before_external_loop`), and the at-most-one-wipe precedence where a loop end coincides with a retract site (AC-6).
- Bounds rejection for the four ranged keys (DEV-178(a)); `order_lock` bypass for ramp and wipe (AC-N1, AC-N2).
- `to_config_map` arms for `seam_slope_type` and `wipe_on_loops` only, so the live values shadow their padding twins (284 precedent); the other six keys are host-only omitted (ticket-42 precedent); the padding table itself is untouched (rule 2).
- DEV-178 row and the `gen-config-docs` regen for the eight keys.

## Out of Scope

- Packet 285's scarf behaviour itself (its stage is the base, not the deliverable; implementation sequences after it lands).
- Draft 277's retract-wipe `Move` (separate site, separate packet; only the coincidence precedence is pinned here).
- Canonical `Layer::is_perimeter_compatible` grouping unless the implementer's delegated read shows the loop-type gate is insufficient (record dropping it as a divergence, do not build grouping speculatively).
- Concentric-fill seam-gap clipping (DEV-177(b) — no concentric filler exists).
- CONFIG_BLOCK reader spellings (ticket 132 owns all value spellings; this packet only shadows twins).
- Any per-tool vector model (ticket 125 owns it; all eight keys are canonical-scalar).

## Authoritative Docs

- `docs/00_project_overview.md` - large; delegated SUMMARY (modular-pipeline constraint: slope/wipe stay emitter-side, no new module seam).
- `docs/08_coordinate_system.md` - small (285 lines); direct range read for the mm↔unit boundary only.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - delegated LOCATIONS (P53 rows: Tier B, owner `crates/slicer-gcode`).
- `docs/11_operational_governance_and_acceptance_gate.md` - delegated SUMMARY only if the implementer touches validation error codes beyond the ticket-113 pattern.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` slope gating order and `ExtrusionLoopSloped` ramp construction (borrow the gate order; port it emitter-side onto 285's stage)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` loop-wipe emission and neighbour-qualification shape (borrow the trigger sites; the retract-wipe split stays with draft 277)
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` — `Layer::is_perimeter_compatible` slope-compatibility grouping (decide whether the emitter needs the grouping or the loop-type gate suffices)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`; no measurable refinements beyond their Given/When/Then text. Coverage: every key drives a behaviour change at a non-default value — `seam_slope_type` (AC-3), `seam_slope_steps` / `seam_slope_min_length` / `seam_slope_entire_loop` (AC-4), `seam_slope_inner_walls` / `seam_slope_start_height` (AC-5), `wipe_on_loops` / `wipe_before_external_loop` (AC-6); schema/defaults pinned by AC-1, default identity by AC-2.
- Negative: `AC-N1` (bounds rejection), `AC-N2` (`order_lock` bypass).
- Cross-packet impact: implementation sequences after packet 285 (additive generalisation; activation-blocked until 285 lands); coincidence precedence with draft 277's retract wipe pinned by AC-6 with no dependency edge; `seam_gap` clipping and scarf arms from 285 must stay green throughout (Step 4 re-runs 285's guard binary if landed, else records the skip).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Proves AC-1–AC-6 + AC-N1 + AC-N2 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | No struct-literal / type blast radius from the new `ResolvedConfig` fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | Regen freshness for the eight keys | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness after the `machine-gcode-emit.toml` schema edit (exit 0 fresh; rebuild without `--check` if stale) | FACT exit code only |

## Step Completion Expectations

Step 2 (slope) lands before Step 3 (wipe); both run against the Step-1 schema. Step 4 re-runs the full new guard binary plus 285's guard binary when 285 has landed (records the skip otherwise), regens docs, and writes DEV-178. Shared scratch state: the closed-loop emit fixture built in Step 1 is reused by Steps 2–4.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` is a large file — implementer reads only the `emit_gcode` loop region plus 285's stage (ranges in `design.md`); `OrcaSlicerDocumented/` is delegate-only; the guard binary is new construction, not a read of existing tests. Heavy dispatches (Orca reads, struct-literal blast radius) carry the bounded return formats in `implementation-plan.md`.
