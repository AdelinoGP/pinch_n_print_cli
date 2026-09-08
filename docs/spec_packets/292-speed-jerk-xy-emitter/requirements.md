# Requirements: 292-speed-jerk-xy-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/66-author-packet-p59-speed-jerk-xy-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P59 (Speed / Jerk (XY), emitter) is eight Tier B keys whose canonical behaviour is emission-time jerk selection in `GCode::_extrude` (one jerk per extrusion path via a role-precedence chain) plus `GCode::travel_to` (travel jerk) plus the `GCode::process_layer` first-layer header (`initial_layer_jerk` emit-and-restore and the Marlin-only `default_junction_deviation` arm), rendered through `GCodeWriter::set_jerk_xy` / `set_junction_deviation` per-flavor M205/M207/SET_VELOCITY_LIMIT forms — but this port emits no jerk command at all: `GcodeFlavor::set_jerk_xy` / `set_junction_deviation` exist as unwired builders and `DefaultGCodeEmitter::emit_gcode` never calls them. All eight keys are true zero-occurrence gaps, and they form one coherent slice: one selection stage over the roles the emitter already sees, rendered through helpers it already owns. Authoring fewer would split one precedence chain; folding in neighbours (P56 accel's M204 arms, P47's machine-envelope M205, P60's role speeds) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare eight scalar-global keys with canonical defaults/bounds: `default_jerk` (float `0.0`, `min 0`), `default_junction_deviation` (float `0.0`, `min 0`, `max 0.3`), `infill_jerk` (float `9.0`, `min 0`), `initial_layer_jerk` (float `9.0`, `min 0`), `inner_wall_jerk` (float `9.0`, `min 0`), `outer_wall_jerk` (float `9.0`, `min 0`), `top_surface_jerk` (float `9.0`, `min 0`), `travel_jerk` (float `12.0`, `min 0`). Ingest is first-wins scalar over Orca vector spellings (the `extract_float_or_first` precedent — ticket 140's `extract_u32_or_first` sits beside it; full Orca vector ingest rides ticket 125).
- Build the per-entity jerk-selection stage in `DefaultGCodeEmitter::emit_gcode`: canonical precedence (first-layer → outer → inner → top-surface → infill → default), `> 0` fallthrough per arm, `default_jerk > 0` master gate (whole stage inert at `0`), travel jerk via the travel path (`travel_jerk` default; no first-layer percent override, no short-travel override — both named non-borrows, DEV-184(d)), first-layer `M205 J` under the Marlin2-only flavor gate (DEV-184(c)).
- Enforce canonical ranges as reject-the-slice validation (ticket-113 rule; deliberate divergence DEV-184(a) — canonical only warns and caps).
- Stateless port of canonical's first-layer restore: canonical emits `initial_layer_jerk` on the first-layer header and restores `default_jerk` on the second; the port re-selects per entity (first-layer arm keys on `global_layer_index == 0`), so no restore state is needed (DEV-184(c)).
- Host-only omission from the CONFIG_BLOCK (ticket-42 precedent): no padding-table edit (zero twins exist for all eight — verified at authoring), defaults byte-identical (AC-2 pins it — the inverse of packet 289's emitting default).
- Annotate the 04 tier table + 05 packet list: P59 8 keys in, owner stands (`crates/slicer-gcode`), packet number 292.

## Out of Scope

- Per-nozzle / per-extruder vector variants of all eight keys: canonical declares them per-nozzle nullable vectors (`NOZZLE_CONFIG`, `get_at` with nozzle index); the vector model stays with ticket 125, and scalar-global is parity-consistent with packets 276/277/279–291 (DEV-184(b)). A vector future stays out of scope.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor touches bool spellings (no bool in scope).
- `initial_layer_travel_jerk` (canonical's `coFloatsOrPercents` percent-over-`travel_jerk` first-layer travel override resolved via `get_abs_value_at`): not queued in P59 and not declared here — travels use `travel_jerk` on every layer (named non-borrow, DEV-184(d)). It is a queue-completeness question for ticket 123's source audit, not a silent omission.
- Short-travel `outer_wall_jerk` override (canonical's `travel.length() < retraction_minimum_travel` near external/overhang perimeters in `GCode::travel_to`), the `GCode::_do_export` Calib-PA-line use of `outer_wall_jerk`, and per-role `Print::validate` machine-max caps: canonical travel/calibration arms with no port-side travel-length/index seam at the travel call site today — named non-borrows in DEV-184(d), `[FWD]` re-checks in `design.md`, not stubs in this packet.
- Machine-envelope M205/M201 (draft packet 281), accel M204 arms (draft packet 289), and jerk-affecting speed stages (packets 290/291): not declared or emitted here; 281 is a stream-position FORWARD-DEP (this packet's lines follow its envelope), 289 is position-adjacent only (per-path `M204` vs per-path `M205` — no shared helper, no dep).
- Roles with no canonical arm (`BottomSolidInfill`, `GapFill`, `BridgeInfill`/`InternalBridgeInfill`, support roles, `Skirt`, `Brim`): fall through to `default_jerk` — recorded in AC-3, never a silent new behaviour.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is an in-module emission parameter, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (jerk values are unitless command magnitudes; no mm↔unit math — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm `default_jerk` 0 / `default_junction_deviation` 0 max 0.3 / the five role jerks 9 / `travel_jerk` 12, all `coFloats` per-nozzle nullable vectors; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` jerk-selection chain (precedence order first-layer → outer → inner → top-surface → infill → default, the `> 0` fallthrough per arm, and the `default_jerk > 0` master gate) plus `GCode::process_layer` first-layer header (the `initial_layer_jerk` emit-and-restore and the Marlin-only `default_junction_deviation` arm) (borrow the order and gates; port the selection emitter-side onto `entity.path.role` + `global_layer_index`, pure per-entity — no stateful restore)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::travel_to` travel-jerk selection (`travel_jerk` default, `initial_layer_travel_jerk` percent-over-base first-layer override via `get_abs_value_at`, short-travel `outer_wall_jerk` override near external/overhang perimeters, all gated on `default_jerk > 0`) plus `GCode::_do_export` Calib-PA-line use of `outer_wall_jerk` (borrow the default only; the two overrides and the Calib arm are named non-borrows — see `requirements.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_jerk_xy` per-flavor forms (Marlin-family `M205 X/Y`, Repetier `M207 X`, Klipper `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY`) and `GCodeWriter::set_junction_deviation` (Marlin-only `M205 J` under `m_max_junction_deviation > 0`) (borrow the forms; the port's existing `GcodeFlavor` arms already match — reuse, do not fork)
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` jerk checks (the `== 1` artifact warning on default/outer/inner, the machine-max exceedance list, the Marlin+JD `ignore_jerk_validation` skip) (cite as the warn-only evidence for DEV-184(a); borrow nothing — the port rejects per ticket 113)

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

- Positive: `AC-1` (schema, eight keys canonical) through `AC-6` (first-layer junction deviation); refinements: AC-2 pins the master-gate identity (byte-identical defaults — the inverse of 289's emitting default); AC-3 pins the five direct role mappings plus the six fallthrough roles and the `0`-falls-through rule; AC-4 pins flat `travel_jerk` on every layer plus the two named travel non-borrows; AC-5 pins the exact per-flavor jerk forms, per-stream print/travel dedup, and the `M566` absence; AC-6 pins the Marlin2-only `J` line on layer 0 only.
- Negative: `AC-N1` (bounds rejection incl. the `0.3` JD ceiling and non-numeric spellings riding ticket 132's strict parse); `AC-N2` (master gate — `default_jerk = 0` silences the whole stage).
- Cross-packet impact: default output byte-identical (key `0` — the inert default, like packet 290's slope-`0` gate and unlike packet 289's emitting default); CONFIG_BLOCK stable (host-only omitted); stream position follows 281's envelope (FORWARD-DEP); 289 is position-adjacent only (no shared helper, no dep); P60 reuses the travel call site later under names kept stable here.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/identity/role/travel/flavor/JD/gate behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean emission stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The selection stage (Step 2) lands before its AC tests (Step 3); the bounds gate lands with the stage, not after. All eight fields are declared exactly once in Step 1 (no key is redeclared anywhere). `default_jerk = 0` leaves no jerk line behind — the master gate is a stage precondition, not a per-arm branch. Tier-table + packet-list annotation (Step 4) records P59 8-in with no shed key.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check); `GcodeFlavor` jerk arms are reuse-only (read `crates/slicer-gcode/src/flavor.rs` lines 99–118 at most — do not fork the forms).
