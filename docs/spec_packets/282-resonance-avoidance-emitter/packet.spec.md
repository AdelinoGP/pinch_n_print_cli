---
status: draft
packet: 282-resonance-avoidance-emitter
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 55 (P48).
---

# Packet Contract: 282-resonance-avoidance-emitter

## Goal

Make `resonance_avoidance`, `min_resonance_avoidance_speed`, and `max_resonance_avoidance_speed` drive host-emitter external-perimeter feedrate adjustment at parity with canonical `GCode::_extrude`.

## Scope Boundaries

P48 is three Tier B keys owned by the host emitter (`crates/slicer-gcode`). All three are zero-occurrence in `crates/`, `modules/`, and `xtask/` (only generated `target/*.gcode` CONFIG_BLOCK echoes); the packet builds the missing decision point inside `DefaultGCodeEmitter::resolve_feedrate` as scalar-global `ResolvedConfig` fields with a `host-keys.toml` mirror. No new module, no IR/WIT change, no per-tool vector model (canonical is scalar; the ticket-125 ruling is not engaged).

## Prerequisites and Blockers

- Depends on: none. Adjacent draft packets noted, not consumed: 267 (power-recovery envelope builder) and 281 (motion-limits envelope builder) — 281's requirements names M593 input-shaping as P48 scope, but canonical's P48 reads emit no M593, only feedrate adjustment, so there is no shared code surface.
- Unblocks: nothing downstream. P48 closes alone.
- Activation blockers: none. All symbols below are live on HEAD (verified 2026-09-07); DEV-174 is the next collision-free ID (log max DEV-171, drafts 276/277/281 propose DEV-171/172/173 — 276 collides with the landed LOG DEV-171, so 174 is first free).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** `resonance_avoidance` (bool, default `false`), `min_resonance_avoidance_speed` (float, default `70.0`), and `max_resonance_avoidance_speed` (float, default `120.0`) are declared as scalar-global `ResolvedConfig` fields with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd schema_declares_three_keys 2>&1 | tail -5`
- **AC-2. Given** default config (`resonance_avoidance = false`), **when** an OuterWall entity is emitted, **then** output feedrates are byte-identical to HEAD (avoidance off is identity). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd default_path_is_identity 2>&1 | tail -5`
- **AC-3. Given** `resonance_avoidance = true`, min `70.0`, max `120.0`, **when** an OuterWall move resolves at factored speed `80.0` (lower half, midpoint `95.0`), **then** the emitted feedrate uses `70.0` (`std::min(80, 70)` parity). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd lower_half_clamps_to_min 2>&1 | tail -5`
- **AC-4. Given** `resonance_avoidance = true`, min `70.0`, max `120.0`, **when** an OuterWall move resolves at factored speed `100.0` (upper half), **then** the emitted feedrate uses `120.0` (boost-to-max parity). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd upper_half_boosts_to_max 2>&1 | tail -5`
- **AC-5. Given** `resonance_avoidance = true`, **when** an OuterWall move resolves at factored speed `130.0` (above max `120.0`), **then** the emitted feedrate uses `130.0` unchanged (loop-disable parity). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd above_max_disables_avoidance 2>&1 | tail -5`
- **AC-6. Given** `resonance_avoidance = true`, min `70.0`, max `120.0`, **when** a non-OuterWall move (SparseInfill) resolves at `80.0`, **then** the emitted feedrate uses `80.0` unchanged (external-perimeter-only parity). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd non_outer_wall_unaffected 2>&1 | tail -5`
- **AC-7. Given** the generated config reference, **when** inspected after `cargo xtask gen-config-docs`, **then** the three keys appear with canonical defaults and no new default-deviation row (DEV-174 is a behaviour record, not a default mismatch). | `cargo test -p slicer-runtime --test unit host_keys_doc_lock 2>&1 | tail -3`

## Negative Test Cases

- **AC-N1. Given** `min_resonance_avoidance_speed = -1.0` (or max negative), **when** the emitter resolves, **then** the slice is rejected with a stable validation error (min-0 enforcement; canonical never enforces — DEV-174(a)). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd negative_speed_rejected 2>&1 | tail -5`
- **AC-N2. Given** `min_resonance_avoidance_speed = 130.0` above max `120.0`, **when** the emitter resolves, **then** the slice is rejected (inverted-range guard). | `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd inverted_range_rejected 2>&1 | tail -5`
- **AC-N3. Given** the packet lands, **when** `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` is inspected, **then** no resonance row was added (rule 2: padding is never evidence; keys thread via the resolved config). | `rg -q 'resonance_avoidance' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd 2>&1 | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraints on emitter changes)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct range read of P48 rows only (tier B, owner `crates/slicer-gcode`)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - direct range read of P48 section only (3-key membership)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'resonance_avoidance' docs/15_config_keys_reference.md`
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'min_resonance_avoidance_speed' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-174" - `rg -q 'DEV-174' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declaration types + defaults for the three keys (already grounded: coBool false / coFloat 70 / coFloat 120)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` resonance block: enable gate, max-disable, lower/upper-half adjustment, per-loop reset (already grounded verbatim at authoring)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
