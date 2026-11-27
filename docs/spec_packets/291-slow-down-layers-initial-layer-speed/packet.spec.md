---
status: draft
packet: 291-slow-down-layers-initial-layer-speed
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/65-author-packet-p58-speed-initial-layer-speed-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 65 (P58).
---

# Packet Contract: 291-slow-down-layers-initial-layer-speed

## Goal

Make `slow_down_layers` drive host-side emission at parity with canonical `GCode::_extrude` — a layer-gated speed blend from the first-layer speeds toward the role speeds over the first N object layers — ported as a per-entity factor stage inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P58 is one Tier B key owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 65) holds the single key in: it is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree — no `slow_down_layers` spelling exists anywhere under `crates/`/`modules/`/`xtask/` outside map prose, and no emitter path blends layer-0 speeds across later layers today. The packet declares the key scalar-global (canonical `coInt` scalar — no ticket-125 vector model) and builds the blend inside `DefaultGCodeEmitter::emit_gcode`'s per-entity `F` resolution. Defaults ARE identity — canonical's `slow_down_layers` is `0`, and at `0` (or `1`, canonical's `> 1` gate) the branch does not exist — so the default stream is byte-identical; that is pinned by AC-2. The key stays host-only omitted from `to_config_map` and hence from the CONFIG_BLOCK (the block renders the raw config map, which carries no such key — packet-42/P35 precedent; zero padding twins exist — verified at authoring — table untouched).

## Prerequisites and Blockers

- Depends on: nothing. The stage reads the entity loop's own `global_layer_index` plus the already-wired `FeedrateConfig` first-layer speeds; it needs no upstream packet's output. Packet 289 (P56) is position-adjacent only (per-path motion emission in the same `emit.rs`): the accel selection renders `M204` lines, this packet scales `F` values — no shared helper, no ordering edge, no FORWARD-DEP.
- Unblocks: wayfinder ticket 65 (P58 closes with the single key in). No edge to any other draft packet.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-183 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-182 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** `slow_down_layers` is declared as scalar-global int `0` (canonical `min 0` recorded in the matching `docs/config/host-keys.toml` `[resolved_config]` row; `min 0` is type-level — `u32` — not a runtime bound, DEV-183(a)) with a matching `docs/config/host-keys.toml` `[resolved_config]` row. | `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd schema_declares_slow_down_layers 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (`slow_down_layers = 0`) and `slow_down_layers = 1`, **when** a two-layer fixture (outer wall + sparse on layer 0 and layer 1) is emitted, **then** both runs are byte-identical to the pre-packet stream — same command count, same `F` values — and `to_config_map` omits the key (host-only omitted, so the CONFIG_BLOCK gains no line at defaults and no padding twin is shadowed). | `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd default_and_one_are_byte_identical 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `slow_down_layers = 3` with `initial_layer_speed = 10` (perimeters) and `initial_layer_infill_speed = 20` (non-perimeters) against faster role speeds (e.g. outer 50, sparse 100, all via the live `FeedrateConfig` table), **when** layers 0–3 are emitted, **then** layer 0 uses the first-layer speeds exactly, layers 1–2 blend linearly toward the role speed (`lerp(first, role, layer / slow_down_layers)` with the layer counted from the first model layer, not from a raft prefix), layer 3+ is the unblended role speed, and a role whose first-layer speed is already ≥ the role speed is never slowed. | `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd blend_ramps_over_configured_layers 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `slow_down_layers = 3`, **when** fixtures with `BottomSolidInfill` (bottom solid), `Skirt`, and `Brim` entities on a blended layer are emitted, **then** the `BottomSolidInfill` entity keeps its base speed and skirt/brim entities keep their `skirt_speed`. Rationale (DEV-183(b), not borrowed canonical): in canonical the blend can never touch `erBottomSurface` anyway — its role speed IS `initial_layer_infill_speed`, so `first_layer_speed < speed` is moot there — while this port resolves `BottomSolidInfill` to the PnP-only `bottom_surface_speed`, so without the exemption the arm would invent a ramp canonical never has; and the port resolves both `Skirt` and `Brim` to `skirt_speed` with no `erBrim` role-speed arm at the borrowed site to blend against, so both are held flat (canonical's post-blend `erSkirt` override order is borrowed for `Skirt`). | `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd exempt_roles_keep_base_speed 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** non-integer spellings (`slow_down_layers = 2.5` as Float, `"3"` as String, `true` as Bool) at the config source, **when** the value reaches the extractor, **then** each is rejected with a `TypeMismatch` error naming the key (the `extract_int_as_u32` contract, `wall_loops` precedent — canonical `min 0` needs no further runtime bound: post-extraction values are `u32`, so no representable violation exists; the negative-`Int` wrap is the shared pre-existing extractor contract, recorded in DEV-183(a), not introduced here). | `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd non_integer_spellings_reject 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — the blend scales speeds the emitter already resolves; rule 4 trigger test does not fire: in-module emission parameter, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "host-speeds generated block" - `rg -q 'slow_down_layers' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b — one `resolved_config.rs::ResolvedConfig` int row; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'slow_down_layers' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-183" - `rg -q 'DEV-183' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the key's declared type/default/bounds (confirm `slow_down_layers` coInt `0` min `0`, scalar; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` layer-blend arms (the `m_config.slow_down_layers > 1` gate, the `is_perimeter` first-layer-speed selection, the `first_layer_speed < speed` guard, the `lerp(first, speed, layer / slow_down_layers)` shape, the raft-prefix offset arm, and the `erBottomSurface`/`erSkirt` site rows — whose flatness this port re-derives as explicit skips in DEV-183(b) because its role-speed table differs at those rows; borrow the gate, the selection, the guard, and the shape, not the site rows verbatim; the `#if 0` over-raft first-layer arm is not borrowed — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` / `extrude_multi_path` / `extrude_path` `speed_for_path` call shape (borrow the fact that the blend sits *after* role-speed selection and *before* the filament-cap — the port's blend arm sits at the same position in `resolve_feedrate`'s caller chain, not inside the base-speed match)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
