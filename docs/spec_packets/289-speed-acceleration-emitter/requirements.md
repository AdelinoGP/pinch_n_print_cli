# Requirements: 289-speed-acceleration-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/63-author-packet-p56-speed-acceleration-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P56 (Speed / Acceleration, emitter) is eleven Tier B keys whose canonical behaviour is emission-time acceleration selection in `GCode::_extrude` (one accel per extrusion path via a role-precedence chain) plus `GCode::travel_to` (travel accel) rendered through `GCodeWriter::set_acceleration_internal`'s per-flavor M204/SET_VELOCITY_LIMIT forms — but this port emits no accel command at all: `GcodeFlavor::set_acceleration` / `set_travel_acceleration` exist as unwired builders and `DefaultGCodeEmitter::emit_gcode` never calls them. All eleven keys are true zero-occurrence gaps, and they form one coherent slice: one selection stage over the roles the emitter already sees, rendered through helpers it already owns. Authoring fewer would split one precedence chain; folding in neighbours (P59 jerk's M205 arms, P47's machine-envelope M201) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare eleven scalar-global keys with canonical defaults/bounds: `accel_to_decel_enable` (bool `true`), `accel_to_decel_factor` (float `50.0`, `min 1`, `max 100`), `default_acceleration` (float `500.0`, `min 0`), `initial_layer_acceleration` (float `300.0`, `min 0`), `inner_wall_acceleration` (float `10000.0`, `min 0`), `outer_wall_acceleration` (float `500.0`, `min 0`), `top_surface_acceleration` (float `500.0`, `min 0`), `travel_acceleration` (float `10000.0`, `min 0`), `bridge_acceleration` (float-or-percent `50%` over `outer_wall_acceleration`, `min 0`), `sparse_infill_acceleration` (float-or-percent `100%` over `default_acceleration`, `min 0`), `internal_solid_infill_acceleration` (float-or-percent `100%` over `default_acceleration`, `min 0`).
- Build the per-entity acceleration-selection stage in `DefaultGCodeEmitter::emit_gcode`: canonical precedence (first-layer → bridge → sparse → internal-solid → outer → inner → top-surface → default), `> 0` fallthrough per arm, `default_acceleration > 0` master gate (whole stage inert at `0`), travel accel via the separate-travel flavor gate, Klipper `ACCEL_TO_DECEL` suffix under the enable bool.
- Enforce canonical ranges as reject-the-slice validation (ticket-113 rule; deliberate divergence DEV-181(a) — canonical never enforces).
- Stateless port of canonical's layer restore: canonical emits `default_acceleration` on the 2nd layer and the wipe-tower `set_accelerations` restore; the port re-selects per entity (first-layer arm keys on `global_layer_index == 0`), so no restore state is needed (DEV-181(c)).
- Host-only omission from the CONFIG_BLOCK (ticket-42 precedent): no padding-table edit (zero twins exist for all eleven — verified at authoring), defaults newly emit M204 lines (the intended default output change, AC-2 pins it).
- Annotate the 04 tier table + 05 packet list: P56 11 keys in, owner stands (`crates/slicer-gcode`), packet number 289.

## Out of Scope

- Per-nozzle / per-extruder vector variants of the nine accel keys: canonical declares them per-nozzle nullable vectors (`NOZZLE_CONFIG`, `get_abs_value_at` with nozzle index); the vector model stays with ticket 125, and scalar-global is parity-consistent with packets 276/277/279–288 (DEV-181(b)). A vector future stays out of scope.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor re-spells bools (word-form bool rejection rides the existing strict-parse rule).
- Short-travel overhang/external-perimeter travel overrides, first-layer travel (`initial_layer_travel`, a P60 key), wipe-tower travel accel, and the Calib-PA-line use of `outer_wall_acceleration`: canonical travel arms with no port-side role/index seam at the travel call site today — `[FWD]` re-checks in `design.md`, not stubs in this packet.
- Machine-envelope M201/M204-R (draft packet 281) and jerk M205 arms (P59, ticket 66): not declared or emitted here; 281 is a stream-position FORWARD-DEP (this packet's lines follow its envelope), P59 reuses the same flavor helpers later.
- Support/solid roles with no canonical arm (`BottomSolidInfill`, `GapFill`, `SupportMaterial`, `SupportInterface`, `Skirt`, `Brim`): fall through to `default_acceleration` — recorded in AC-3, never a silent new behaviour.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is an in-module emission parameter, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (accel values are unitless command magnitudes; no mm↔unit math — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eleven keys' declared types/defaults/bounds (confirm `default_acceleration` 500 / `travel_acceleration` 10000 / `inner_wall_acceleration` 10000 / `initial_layer_acceleration` 300 / `outer_wall_acceleration` 500 / `top_surface_acceleration` 500, the three percent defaults with their `ratio_over` bases, and `accel_to_decel_factor` coPercent 50 min 1 max 100; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` acceleration-selection chain (precedence order first-layer → bridge → sparse → internal-solid → outer → inner → top-surface → default, the `> 0` fallthrough per arm, and the `default_acceleration > 0` master gate) (borrow the order and gates; port the selection emitter-side onto `entity.path.role` + `global_layer_index`, pure per-entity — no stateful restore)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::travel_to` travel-acceleration selection (`travel_acceleration` default, first-layer and short-travel overhang/external-perimeter overrides, wipe-tower accel) plus `GCode::_do_export` Calib-PA-line use of `outer_wall_acceleration` (borrow the default + flavor split; short-travel overrides and the Calib arm are `[FWD]` — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_acceleration_internal` per-flavor forms (legacy-Marlin `M204 S`, Marlin2/RRF `M204 P`, Klipper `SET_VELOCITY_LIMIT ACCEL` + `ACCEL_TO_DECEL` under `accel_to_decel_enable`, Repetier `M201`, Bambu none) and `GCodeWriter::supports_separate_travel_acceleration` (which flavors take a separate travel command) (borrow the forms; the port's existing `GcodeFlavor` arms already match — reuse, do not fork)
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` acceleration checks (machine-max caps/warnings per role, `accel_to_decel_factor` range) (cite as the warn-and-cap-only evidence for DEV-181(a); borrow nothing — the port rejects per ticket 113)

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

- Positive: `AC-1` (schema, eleven keys canonical) through `AC-6` (Klipper decel suffix); refinements: AC-3 pins the five direct role mappings plus the six fallthrough roles and the `0`-falls-through rule; AC-4 pins the three percent bases (`bridge` over outer, sparse/internal-solid over default); AC-5 pins the exact separate-travel flavor set.
- Negative: `AC-N1` (bounds rejection incl. factor floor/ceiling and word-form bool spelling riding ticket 132); `AC-N2` (master gate — `default_acceleration = 0` silences the whole stage).
- Cross-packet impact: default output newly emits M204 lines (intended — pre-packet stream has none); CONFIG_BLOCK stable (host-only omitted); stream position follows 281's envelope (FORWARD-DEP); P59 reuses the flavor helpers under canonical names.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/role/percent/flavor/decel/gate behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean emission stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The selection stage (Step 2) lands before its AC tests (Step 3); the bounds gate (Step 2b) lands with the stage, not after. The percent keys need their bases resolvable in tests (Step 1 declares all eleven exactly once — no key is redeclared anywhere). `default_acceleration = 0` leaves no accel line behind — the master gate is a stage precondition, not a per-arm branch. Tier-table + packet-list annotation (Step 4) records P56 11-in with no shed key.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check); `GcodeFlavor` arms are reuse-only (read `crates/slicer-gcode/src/flavor.rs` lines 74–120 at most — do not fork the forms).
