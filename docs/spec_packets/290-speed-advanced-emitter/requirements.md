# Requirements: 290-speed-advanced-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/64-author-packet-p57-speed-advanced-speed-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P57 (Speed / Advanced, emitter) is three Tier B keys whose canonical behaviour is emission-side extrusion-rate smoothing in `GCode::PressureEqualizer` — a G-code-text post-stage that limits how fast the volumetric extrusion rate may change across print moves — but this port has no smoother at all: the three keys are true zero-occurrence gaps, and they form one coherent slice: one post-stage over the final emitted moves, gated by one slope plus one segment length plus one external-only bool. Authoring fewer would split one stage; folding in neighbours (P59 jerk's M205 arms, P58's `slow_down_layers`) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare three scalar-global keys with canonical defaults/bounds: `max_volumetric_extrusion_rate_slope` (float `0.0`, `min 0`), `max_volumetric_extrusion_rate_slope_segment_length` (float `3.0`, `min 0.5`, `max 5`), `extrusion_rate_smoothing_external_perimeter_only` (bool `false`).
- Build the smoothing stage in `crates/slicer-gcode` over the emitted `GCodeIR` moves: the `> 0` master gate (whole stage inert at `0`, AC-N2), the skip list (bridge roles + ironing + the external-only gate over outer walls and `overhang_quartile`-marked points, AC-4), forward/backward rate limiting that rewrites `F` only (E conserved, travels/retracts untouched, AC-3), segment splitting at the configured length with the trivial-delta floor (AC-5).
- Enforce canonical ranges as reject-the-slice validation (ticket-113 rule; deliberate divergence DEV-182(a) — canonical never enforces).
- Text-parsing port of the canonical text stage: the port re-derives per-move volumetric rates from its own IR (move geometry + `flow_factor`-scaled E + sticky feedrate) rather than re-parsing G-code text, and keeps no one-layer lookbehind (the entity loop's output is complete, so the stage sees the whole stream — DEV-182(c)).
- Host-only omission from the CONFIG_BLOCK (ticket-42 precedent): no padding-table edit (zero twins exist for all three — verified at authoring), defaults byte-identical (AC-2 pins it).
- Annotate the 04 tier table + 05 packet list: P57 3 keys in, owner stands (`crates/slicer-gcode`), packet number 290.

## Out of Scope

- Per-nozzle / per-extruder vector variants: all three keys are scalar in canonical (`GCodeConfig`, no `NOZZLE_CONFIG` arm), so no ticket-125 vector arm exists for this family at all.
- CONFIG_BLOCK padding-table derivation (ticket 132) and bool spelling fixes (ticket 132): out-of-bounds; this packet neither edits the table nor re-spells bools (word-form bool rejection rides the existing strict-parse rule).
- The canonical first-layer-travel / short-travel / wipe-tower travel overrides: this family has none of those — canonical's `travel_to` is untouched by the equalizer, and the port keeps that boundary (travels are never limited — recorded in AC-3, never a silent new behaviour).
- Machine-envelope M201 (draft packet 281) and jerk M205 arms (P59, ticket 66): not declared or emitted here; 281's envelope is untouched (this stage adds no stream-opening line, so no FORWARD-DEP — unlike packet 289, which sequences after it).
- Support/solid/inner roles with no canonical skip: limited normally — the skip list is exactly bridge + ironing + the external-only gate; nothing else falls through silently.
- The `adjustable_flow` marker dimmer: always-on in this port (canonical sets `adjustable_flow` only inside `;_EXTRUDE_SET_SPEED` blocks — `GCode::_extrude` — which switch off along the sequential-object travel path, a mode this port does not have per ticket 32 — so the port has no marker blocks at all and every print move is smoothable), recorded, not a silent extension (DEV-182(b)).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this stage is an in-module emission parameter, not cross-module algorithm selection, so no claim holders)
- `docs/08_coordinate_system.md` - direct range read not required (rates are mm³/s over mm-space moves; no IR-unit math — sizing note, not a read claim)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the three keys' declared types/defaults/bounds (confirm slope coFloat `0` min `0`, segment length coFloat `3.0` min `0.5` max `5`, external-only coBool `false`, all scalar; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_do_export` equalizer construction gate (`max_volumetric_extrusion_rate_slope.value > 0` → `make_unique<PressureEqualizer>`, else the stage does not exist) and the `;_EXTRUDE_SET_SPEED` / `;_EXTRUDE_END` marker emission in `GCode::_extrude` (borrow the gate; the port has no marker blocks at all — the blocks switch off only on the sequential-object path per ticket 32 — so every print move is adjustable)
- `OrcaSlicerDocumented/src/libslic3r/GCode/PressureEqualizer.cpp` — ctor (slope × 3600 unit step, segment-length + bool capture, all-roles-equal slope tables, ironing zeroed) and `adjust_volumetric_rate` (the skip list — bridge + ironing + external-only gate — and the forward/backward limiter shape) (borrow the skip list exactly and the limiter shape; the ×3600 is implicit in the port's mm/min rate math — do not re-multiply; the text-parsing and one-layer-lookbehind machinery is not borrowed — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCode/PressureEqualizer.cpp` — `output_gcode_line` (segment splitting at `m_max_segment_length`, the trivial-delta floor, accel-then-decel vs single-slope emission) (borrow the split rule and the floor; port onto `GCodeCommand::Move` insertion)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::process_layers` pipeline order (borrow the after-generation position — the port has no spiral stage and no tbb, so only the position transfers, not the filter shape)

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

- Positive: `AC-1` (schema, three keys canonical) through `AC-5` (segment-length splitting); refinements: AC-3 pins the F-only / E-conserved / travels-untouched contract; AC-4 pins the outer-wall + `overhang_quartile`-marked overhang gate; AC-5 pins the finer-splits-more plus trivial-floor arms.
- Negative: `AC-N1` (bounds rejection incl. segment-length floor/ceiling and word-form bool spelling riding ticket 132); `AC-N2` (master gate — slope `0` silences the whole stage even with non-default companions).
- Cross-packet impact: default output byte-identical (slope `0` — the inert default, unlike packet 289's emitting default); CONFIG_BLOCK stable (host-only omitted); no stream-position dep on any draft packet; arc fitting untouched (packet 284's scope — the tooltip note is tooltip-only, verified no code coupling).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/identity/retime/gate/split behaviour | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new ResolvedConfig fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean smoothing stage | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log \| tail -3` | Prove generated host-keys/docs freshness after Step 1b | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The smoothing stage (Step 2) lands before its AC tests (Step 3); the bounds gate (Step 2b) lands with the stage, not after. The `overhang_quartile` gate in AC-4 reads the point field the producer already stamps (no new producer work — the emitter only reads it). Slope `0` leaves no trace behind — the master gate is a stage precondition, not a per-move branch. Tier-table + packet-list annotation (Step 4) records P57 3-in with no shed key.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/resolved_config.rs` are both over 300 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `PressureEqualizer.cpp` are out-of-bounds (delegate per the obligations above); the `ResolvedConfig` field addition carries struct-literal blast radius (owned by Step 1's LOCATIONS dispatch, not discovered via follow-up check); the E-conservation contract means the stage must never touch `e` — an F-only review of the Step-2 diff is the cheapest falsifier.
