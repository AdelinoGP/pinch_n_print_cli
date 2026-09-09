# Requirements: 293-speed-other-layers-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/67-author-packet-p60-speed-other-layers-speed-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P60 (Speed / Other layers speed, emitter) is two Tier B keys whose canonical behaviour is emission-time speed selection in `GCode` — one dedicated role speed plus one loop-length-gated loop override — but this port resolves neither: `DefaultGCodeEmitter::resolve_feedrate` maps `InternalSolidInfill` to `sparse_infill_speed` (with a comment naming the missing family), and no small-loop gate exists anywhere in the tree. The rectilinear module parses `internal_solid_infill_speed` into an unused tuple while hardcoding `speed_factor = 1.0` with a comment saying the host owns the role speed. Both keys are true zero-occurrence gaps (no behaviour reads outside map prose), and they form one coherent slice: two arms at the same emitter seam, over roles and loop geometry the emitter already sees, reaching `F` values it already computes. Authoring fewer would split one table update; folding in neighbours (P56 accel's M204 arms, P58's slow-down blend, P61's support-ironing air-filtration) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare `internal_solid_infill_speed` scalar-global float `100.0` (`min 1`, first-wins scalar ingest of Orca vector spellings — ticket-140 precedent) as a `SPEED_KEYS` row with matching `docs/config/host-keys.toml` `[speeds]` row, plus its `ResolvedConfig` twin (the ticket-114 sparse-twin precedent: `to_config_map` arm so modules and the CONFIG_BLOCK see the live value, no `[resolved_config]` row): `100` equals the sparse default it previously shadowed. Enforce the bound as reject-the-slice (DEV-185(a)).
- Declare `small_perimeter_speed` scalar-global percent-capable, default `50%` over `outer_wall_speed` (`min 0`, `0` = auto → `outer * 0.5`), with a host-side `small_perimeter_threshold` twin float `0.0` (`min 0`) — the port's canonical-name speed/threshold pair for the gate. Percent resolution rides the `get_abs_value` machinery (ticket-107 precedent: the port's read-time percent resolver mirroring canonical's `get_abs_value`). Both gain `[resolved_config]` rows; all three bounds reject-the-slice (DEV-185(a)). The existing `classic-perimeters` `small_perimeter_threshold` (width-classification twin, default 0, min 0) is untouched — declared on the module, consumed only by `classify_narrow_island`, never reaching the emitter; the two thresholds are deliberately separate inputs that happen to share a canonical name family (DEV-185(c)).
- Build the internal-solid arm in `resolve_feedrate`'s base-speed match: `InternalSolidInfill` → the new `internal_solid_infill_speed` field (one-line reseat; the old sparse mapping is the pre-packet coupling AC-3 proves broken).
- Build the small-perimeter arm at the per-entity `F` resolution site (`resolve_feedrate`'s caller chain, positioned per packet 291's precedent — after role-speed selection and after 291's blend, before the filament cap): when the threshold is `> 0` and the entity is a closed wall loop (`ExtrusionPath3D::is_closed`, canonical `speed == -1` entry analogue) whose planar length is at or under `threshold * 2 * PI` (the `SMALL_PERIMETER_LENGTH` circumference conversion) and whose role is a wall loop (`is_loop` — `OuterWall`/`InnerWall`/`ThinWall`; bridge/support/sparse excluded per DEV-185(c)), resolve `F` from the small-perimeter value instead of the blended role base — `0` → `outer_wall_speed * 0.5`, percent → against live `outer_wall_speed`, absolute → as-is. The zero threshold silences the whole arm at any speed value (AC-N2).
- Host-only `to_config_map` discipline: the internal-solid key ships its live value (sparse-twin precedent — the module filter passes declared keys through, and the CONFIG_BLOCK renders the resolved map); the small-perimeter pair stays host-only omitted (ticket-42 precedent — emission-control values, not module inputs). CONFIG_BLOCK gains exactly one line at defaults (`internal_solid_infill_speed = 100`), zero other value changes, no padding-table edit (zero twins exist for all three spellings — verified at authoring).
- Annotate the 04 tier table + 05 packet list: P60 2 keys in, owner stands (`crates/slicer-gcode`), packet number 293.

## Out of Scope

- Per-nozzle / per-extruder vector variants of `internal_solid_infill_speed`: canonical declares it a per-nozzle nullable vector read via `NOZZLE_CONFIG`/`get_at`; the vector model stays with ticket 125, and scalar-global is parity-consistent with packets 276/277/279–292 (DEV-185(b)). A vector future stays out of scope. (`small_perimeter_speed` is canonical `coFloatsOrPercents`, also per-nozzle — same ruling, same future.)
- The minimum-cross-section guard (`GCode.cpp` minimum-`mm3_per_mm` collection keyed on the `== 0` speed reads): the volumetric floor this port computes in the same entity loop from rendered geometry needs no such guard (the `F` values it scales can never be the unwritten canonical `-1`), so nothing is ported — named non-borrow, not a silent omission.
- The `Fill.cpp` `role_speed` `erSolidInfill` arm (fill-side duplicate of the same decision): the port resolves `F` once at emission over `entity.path.role` — already carrying the internal-solid role — so porting the arm twice would double the decision, not widen it. Cited as the no-dual-implementation evidence, not a second site.
- `small_support_perimeter_speed` / `small_support_perimeter_threshold` (the support-side sibling pair keyed on support roles): not queued in P60 and not declared here — support entities keep their role `F` at any threshold (canonical's separate `speed_for_path` support closure; the port's entities already carry distinct support roles, so the exclusion is structural, not a stub).
- Wiring the rectilinear module's parsed `internal_solid_infill_speed` into its `speed_factor`: rejected — the module hardcodes 1.0 with the host-owns-speed comment (ticket-114 contract); the host arm is the only implementation, the parsed tuple stays dead (a parse-only stub is not a second decision point).
- Short-travel `outer_wall`-style travel overrides and any jerk/accel-stage sharing (draft packets 289/292): not declared or emitted here; those stages emit command lines, this packet scales `F` values — no shared helper, no ordering edge.
- Roles with no canonical small-perimeter arm (`BottomSolidInfill`, `TopSolidInfill`, `GapFill`, bridge pair, support roles, `Skirt`, `Brim`, travels): excluded from the gate by construction (DEV-185(c)) — recorded in AC-4, never a silent new behaviour.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is an in-module emission parameter, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (speeds are unitless command magnitudes; the loop length is measured in mm straight from entity points with no mm↔unit conversion — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the two keys' declared types/defaults/bounds plus the threshold (`internal_solid_infill_speed` coFloats `100` min 1; `small_perimeter_speed` coFloatsOrPercents `50%`-percent `ratio_over outer_wall_speed` min 1; `small_perimeter_threshold` coFloats `0` min 0; all per-nozzle nullable vectors; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode` speed selection `erSolidInfill` arm (`speed = NOZZLE_CONFIG(internal_solid_infill_speed)`) and the minimum-cross-section guard's `== 0` reads (borrow the arm; the guard is a named non-borrow — see `requirements.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `extrude_loop` small-perimeter gate (the `speed == -1` entry condition, the `loop.length() <= SMALL_PERIMETER_LENGTH(threshold)` test, the `value == 0` → `outer * 0.5` auto arm, the `get_abs_value(outer)` resolution, and the `speed_for_path` perimeter-only application `small_peri_speed > 0 && !is_bridge && is_perimeter`) (borrow the gate shape and all three value arms; port the length test onto entity points and the role test onto `ExtrusionRole` wall variants, pure per-entity — no loop object)
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — `SMALL_PERIMETER_LENGTH` macro (`threshold / SCALING_FACTOR * 2 * PI`: the threshold is a radius, the gate is its circumference — borrow the `* 2 * PI` conversion, not the scaling)
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `role_speed` `erSolidInfill` arm (the same decision seen from the fill side; the port resolves F once at emission, so this arm is ported once — borrow nothing, cite as the no-dual-implementation evidence)
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` speed checks (cite as the warn-only evidence for DEV-185(a); borrow nothing — the port rejects per ticket 113)

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

- Positive: `AC-1` (schema, both keys canonical) through `AC-4` (small-perimeter gate + three value arms); refinements: AC-2 pins the near-identity default (F-identical, exactly +1 CONFIG_BLOCK line — the honest twin line via the resolved map); AC-3 pins the internal-solid/sparse independence (the pre-packet coupling broken, intended); AC-4 pins the wall-loops-only qualification plus all three value arms (auto/absolute/percent) at one threshold.
- Negative: `AC-N1` (bounds rejection incl. the threshold floor and non-numeric spellings); `AC-N2` (zero threshold silences the arm).
- Cross-packet impact: default F stream byte-identical (near-identity — unlike 289's emitting default, like 291's identity except the one honest twin line); CONFIG_BLOCK +1 line (host-live key via the resolved map, not padding); no new command lines on any path (F-only packet in a stream of M-command stages); 291's blend position re-read at implementation, not frozen.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/identity/independence/gate behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean emission stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The internal-solid reseat (Step 2) lands before its AC tests (Step 3); the small-perimeter gate lands with it, not after. All three fields are declared exactly once in Step 1 (no key is redeclared anywhere — the rectilinear module's parsed tuple is dead input, not a declaration). `threshold = 0` leaves no small-perimeter `F` behind — the gate is a stage precondition, not a per-arm branch. Tier-table + packet-list annotation (Step 4) records P60 2-in with no shed key.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `GCode.cpp` are out-of-bounds (delegate per the obligations above); the `SPEED_KEYS` row addition carries the positional-alignment blast radius (`SPEED_META` + `SPEED_BOUNDS` + `SPEED_KEY_COUNT` const asserts — owned by Step 1's dispatch, not discovered via follow-up check); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch); the rectilinear module's dead speed tuple tempts a second implementation site (explicitly rejected — do not touch the module).
