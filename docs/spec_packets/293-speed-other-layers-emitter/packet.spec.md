---
status: draft
packet: 293-speed-other-layers-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/67-author-packet-p60-speed-other-layers-speed-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 67 (P60).
---

# Packet Contract: 293-speed-other-layers-emitter

## Goal

Make the P60 other-layers-speed pair drive host-side emission at parity with canonical `GCode` speed selection — `internal_solid_infill_speed` as the dedicated `erSolidInfill` role speed plus `small_perimeter_speed` as the loop-length-gated small-loop override — ported as two arms inside `crates/slicer-gcode` over the live `FeedrateConfig` table, with no new module, IR field, or WIT change.

## Scope Boundaries

P60 is two Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 67) holds both in: each key is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree. `internal_solid_infill_speed` is parse-only dead in `rectilinear-infill` (read into an unused tuple, factor hardcoded 1.0) while the emitter maps `InternalSolidInfill` to `sparse_infill_speed`; `small_perimeter_speed` has no spelling anywhere under `crates/`/`modules/`/`xtask/` outside map prose, and no loop-length gate exists at the emission seam. The packet declares `internal_solid_infill_speed` scalar-global (DEV-185(b): canonical declares it a per-nozzle nullable vector; the vector model stays with ticket 125) and `small_perimeter_speed` as a scalar-global percent-capable value over `outer_wall_speed` with a host-side `small_perimeter_threshold` twin (DEV-185(c)). Defaults are near-identity — `internal_solid_infill_speed` 100 equals the sparse default it previously shadowed, and the small-perimeter arm is inert at the zero threshold — so the default *stream* is byte-identical; the CONFIG_BLOCK gains exactly one line (the internal-solid twin via the resolved map; the small-perimeter pair stays host-only omitted per the ticket-42 precedent). The small-perimeter arm keys on closed wall loops only, measured from entity points against the threshold circumference (DEV-185(c)).

## Prerequisites and Blockers

- Depends on: nothing. Both arms read the entity loop's own `role` + `points` plus the already-wired `FeedrateConfig`/`ResolvedConfig` tables; neither needs any upstream packet's output. Packet 291 (P58, draft) blends at the same per-entity F site — anchor awareness, not a dep: this packet's small-perimeter override wraps the blended base, and 291's draft position is re-read from its `packet.spec.md` at implementation time, not frozen here.
- Related work, not a blocker: draft packet 289 (`docs/spec_packets/289-speed-acceleration-emitter/`, `status: draft`) built the adjacent per-entity M204 stage in the same `emit.rs` loop; this packet shares no helper with it and emits no command lines (F values only) — no FORWARD-DEP, no ordering edge.
- Unblocks: wayfinder ticket 67 (P60 closes with both keys in). No edge to any other draft packet.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-185 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-184 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** both keys are declared scalar-global with canonical defaults — `internal_solid_infill_speed` float `100.0` (`min 1`, first-wins scalar ingest of Orca vector spellings, ticket-140 precedent) as a `SPEED_KEYS` row with matching `docs/config/host-keys.toml` `[speeds]` row plus its `ResolvedConfig` twin (sparse-twin precedent: `to_config_map` arm, no `[resolved_config]` row), and `small_perimeter_speed` percent-capable `50%` over `outer_wall_speed` (`min 0`, `0` = auto) with a `small_perimeter_threshold` host twin float `0.0` (`min 0`), each with matching `[resolved_config]` rows. | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd schema_declares_both_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config under flavor `marlin2`, **when** a mixed-role fixture (outer wall, internal-solid, sparse, one small closed wall loop) is emitted, **then** every `F` value is byte-identical to the pre-packet stream (internal-solid still resolves 100; the small-perimeter arm is inert at threshold `0`), and the CONFIG_BLOCK gains exactly one line (`internal_solid_infill_speed = 100` via the `to_config_map` twin; the host-only small-perimeter pair stays omitted per the ticket-42 precedent) with zero other value changes (newly-live key, no padding twin shadowed — verified zero twins at authoring — table untouched). | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd default_stream_is_byte_identical_plus_one_config_line 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `sparse_infill_speed = 120` with `internal_solid_infill_speed` at default, then `internal_solid_infill_speed = 80`, **when** sparse + internal-solid fixtures are emitted, **then** the first run emits internal-solid at `F6000` while sparse emits `F7200` (independence — the pre-packet coupling is broken, intended), and the second run emits internal-solid at `F4800` with sparse, wall, top-surface, and bridge `F` values unchanged. | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd internal_solid_is_independent_of_sparse 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `small_perimeter_threshold = 10` (gate circumference `10 * 2 * PI` mm) with `small_perimeter_speed = 15`, **when** a small closed wall loop (length under the gate), a large closed wall loop, a short sparse path, and a short bridge path are emitted, **then** the small wall loop carries `F900`, the large loop keeps its role `F`, and the sparse/bridge entities keep their role `F` (wall-loops-only qualification — canonical `!is_bridge && is_perimeter`, DEV-185(c)); with `small_perimeter_speed = 0` the small loop carries `outer_wall_speed * 0.5` (canonical auto arm); with a percent spelling the value resolves against the live `outer_wall_speed` (50% of 60 → `F1800`). | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd small_perimeter_gate_and_resolution 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (`internal_solid_infill_speed = -1` or a non-numeric spelling, `small_perimeter_speed` negative, `small_perimeter_threshold = -1`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins are GUI hints — DEV-185(a)). | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** `small_perimeter_threshold = 0` with `small_perimeter_speed = 15` and a tiny closed wall loop, **when** the fixture is emitted, **then** the loop keeps its role `F` — no small-perimeter `F` appears anywhere in the output (zero threshold silences the whole arm at any speed value). | `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd zero_threshold_silences_small_perimeter_arm 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test speed_p60_other_layers_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — speeds scale no E and select no algorithm; rule 4 trigger test does not fire: in-module emission parameter, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'small_perimeter_speed' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` sections "[speeds]" + "[resolved_config]" - `rg -q 'internal_solid_infill_speed' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-185" - `rg -q 'DEV-185' docs/DEVIATION_LOG.md`

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

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
