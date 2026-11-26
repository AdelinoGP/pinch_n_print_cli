# Requirements: 291-slow-down-layers-initial-layer-speed

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/65-author-packet-p58-speed-initial-layer-speed-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P58 (Speed / Initial layer speed, emitter) is one Tier B key whose canonical behaviour is emission-side speed selection in `GCode::_extrude` — layer 0 prints at the first-layer speeds, layers 1..N-1 blend linearly from the first-layer speed toward the role speed, layer N+ prints at full role speed — but this port has no such stage: `slow_down_layers` is a true zero-occurrence gap (no spelling anywhere under `crates/`/`modules/`/`xtask/` outside map prose), and the emitter's `resolve_feedrate` resolves every layer at the same role speed. It forms one coherent slice by itself: one scalar key feeding one per-entity blend arm inside the existing owner's per-point `F` resolution. Authoring fewer is impossible (one key); folding in neighbours (P59 jerk's M205 arms, P60's role speeds) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare one scalar-global key with canonical default/bounds: `slow_down_layers` (int `0`, `min 0`).
- Build the blend arm in `crates/slicer-gcode` inside the per-entity `F` resolution (`resolve_feedrate`'s caller chain, after role-speed selection, before the filament-cap position canonical holds): the `> 1` master gate (whole arm inert at `0`/`1`, AC-2), the `is_perimeter` first-layer-speed selection (`feedrate_config.initial_layer_speed` for perimeter roles, `feedrate_config.initial_layer_infill_speed` otherwise — both already live in the `FeedrateConfig` table `run.rs` wires from the raw config), the `first_layer_speed < speed` guard (never slow a role that is already slower), the `lerp(first, speed, layer_index / slow_down_layers)` shape with the layer counted from the first model layer (raft prefix excluded — the port emits no raft layers today, so the offset is a code comment, not a branch), the `BottomSolidInfill` exemption plus the flat `Skirt`/`Brim` hold (both deliberate port divergences, DEV-183(b) — canonical never blends `erBottomSurface` only because its role speed IS the infill base, a `first_layer_speed < speed` mootness this port's PnP-only `bottom_surface_speed` does not share; and the port resolves both skirt and brim to `skirt_speed` with no `erBrim` role-speed arm at the borrowed site, so both are held flat rather than blending one and overriding the other).
- Enforce the type contract at extraction (non-integer spellings reject as `TypeMismatch` — the shared `extract_int_as_u32` contract, `wall_loops` precedent). Canonical `min 0` needs no further runtime bound: post-extraction values are `u32`, so no representable violation exists, and the negative-`Int` wrap is the shared pre-existing extractor contract — recorded in DEV-183(a) as map context, not introduced here (deliberate non-event, not a divergence: canonical never enforces either).
- Host-only omission from `to_config_map` and hence the CONFIG_BLOCK (ticket-42/P35 precedent — the `to_config_map` body is hand-written per key, so omission is one absent `m.insert`; the block renders the raw config map, which carries no such key): no padding-table edit (zero twins exist — verified at authoring), defaults byte-identical (AC-2 pins it).
- Annotate the 04 tier table + 05 packet list: P58 1 key in, owner stands (`crates/slicer-gcode`), packet number 291.

## Out of Scope

- Per-nozzle / per-extruder vector variants: the key is scalar `coInt` in canonical (`PrintConfig.cpp`, no `NOZZLE_CONFIG` arm), so no ticket-125 vector arm exists for this key at all.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor touches bool spellings (no bool in scope).
- The canonical `#if 0`-disabled first-layer-over-raft arm (`first_layer_acceleration_over_raft`-style over-raft branch inside `_extrude`'s accel selection): dead code upstream, not borrowed — the port has no over-raft first-layer concept and gains none here (recorded in DEV-183(c), not a silent omission).
- The canonical filament volumetric cap (`filament_max_volumetric_speed`, Tier D deferred) and the Klipper resonance arms (packet 282's scope): the blend sits before the cap position conceptually, but the port has no cap to order against — no interaction, no dep.
- Raft-prefix layer emission: the port emits no raft layers (raft is tree-planner metadata, not emitted prefix layers), so the raft-offset arm is documented as a comment, not built as a branch. If raft prefix layers ever emit, the blend's layer counter must exclude them (recorded in design `[FWD]`).
- Machine-envelope M201 (draft packet 281) and jerk M205 arms (P59, ticket 66): not declared or emitted here; no stream-position dep (this packet adds no stream-opening line and reorders nothing — unlike packet 289, which sequences after 281).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this arm is an in-module emission parameter, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (speeds are mm/s scalars over layer indices; no IR-unit math — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the key's declared type/default/bounds (confirm `slow_down_layers` coInt `0` min `0`, scalar; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` layer-blend arms (the `m_config.slow_down_layers > 1` gate, the `is_perimeter` first-layer-speed selection, the `first_layer_speed < speed` guard, the `lerp(first, speed, layer / slow_down_layers)` shape, the raft-prefix offset arm, and the `erBottomSurface`/`erSkirt` site rows — whose flatness this port re-derives as explicit skips in DEV-183(b); borrow the gate, the selection, the guard, and the shape, not the site rows verbatim; the `#if 0` over-raft first-layer arm is not borrowed — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` / `extrude_multi_path` / `extrude_path` `speed_for_path` call shape (borrow the fact that the blend sits *after* role-speed selection and *before* the filament-cap — the port's blend arm sits at the same position in `resolve_feedrate`'s caller chain, not inside the base-speed match)

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema, single key canonical) through `AC-4` (exempt roles); refinements: AC-2 pins both spellings of the inert default (`0` and `1`, canonical's `> 1` gate) plus the `to_config_map` omission; AC-3 pins the layer-0 exact-first-speed plus the linear `layer / N` ramp plus the never-slow guard; AC-4 pins the `BottomSolidInfill` skip and the flat skirt/brim hold (both DEV-183(b) port divergences, not borrowed canonical).
- Negative: `AC-N1` (type rejection of Float/String/Bool spellings at the extractor; no runtime bounds case exists — `u32` post-extraction).
- Cross-packet impact: default output byte-identical (key `0` — the inert default, like packet 290's slope-`0` gate and unlike packet 289's emitting default); CONFIG_BLOCK stable (host-only omitted); no stream-position dep on any draft packet; accel/jerk/smoothing stages untouched (packets 289/290/P59 scope — no shared helper).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test speed_p58_slow_down_layers_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/identity/blend/exemption behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig field | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean blend arm | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The blend arm (Step 2) lands before its AC tests (Step 3); no separate bounds step exists (there is no runtime bound to build — `u32` post-extraction). The `initial_layer_speed` / `initial_layer_infill_speed` bases in AC-3/AC-4 read the already-live `FeedrateConfig` fields `run.rs` wires from the raw config via `FeedrateConfig::from_raw_config` (no new speed plumbing — the emitter's `feedrate_config` already carries them; the packet's only new schema is the `slow_down_layers` count itself). `slow_down_layers <= 1` leaves no trace behind — the master gate is a stage precondition on the arm, not a per-move branch. The first-layer-speed selection must match canonical's `is_perimeter` predicate over this tree's `ExtrusionRole` variants (walls, not `ThinWall`--or-`GapFill` guesses — re-derive at implementation). Tier-table + packet-list annotation (Step 4) records P58 1-in with no shed key.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp::_extrude` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check); the never-slow guard means the arm must compare resolved first-layer speed against the resolved role speed *before* blending — an arm that blends unconditionally fails AC-3's guard case regardless of test colour.
