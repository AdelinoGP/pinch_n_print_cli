---
status: draft
packet: 290-speed-advanced-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/64-author-packet-p57-speed-advanced-speed-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 64 (P57).
---

# Packet Contract: 290-speed-advanced-emitter

## Goal

Make the P57 extrusion-rate-smoothing family drive host-side emission at parity with canonical `PressureEqualizer` — ported as a GCodeIR post-stage inside `crates/slicer-gcode` that limits volumetric-rate slope across print moves, with no new module, IR field, or WIT change.

## Scope Boundaries

P57 is three Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 64) holds all three in: every key is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree — no smoother exists anywhere under `crates/`/`modules/`. The packet declares all three scalar-global (all three are scalar in canonical — no ticket-125 vector model) and builds the smoothing stage over the emitted `GCodeIR` stream. Defaults ARE identity — canonical's `max_volumetric_extrusion_rate_slope` is `0`, and at `0` canonical never constructs the equalizer — so the default stream is byte-identical; that is pinned by AC-2. Keys stay host-only omitted from the CONFIG_BLOCK (ticket-42 precedent; zero padding twins exist — verified at authoring — table untouched).

## Prerequisites and Blockers

- Depends on: nothing. The stage reads the final `GCodeIR` moves regardless of which upstream packets land; canonical's arc-fitting note ("this parameter disables arc fitting") is tooltip-only — grep finds no code coupling between the equalizer and arc fitting (arc fitting branches on `fitting_result.empty()` in `GCode::_extrude`, independently) — and arc fitting itself is packet 284's scope, not a dep. The draft spiral packet (279) is not a dep either: canonical orders spiral before the equalizer, but the smoother consumes final moves either way.
- Unblocks: wayfinder ticket 64 (P57 closes with all three in). No edge to any other draft packet.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-182 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-181 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all three keys are declared as scalar-global with canonical defaults — `max_volumetric_extrusion_rate_slope` float `0.0` (`min 0`), `max_volumetric_extrusion_rate_slope_segment_length` float `3.0` (`min 0.5`, `max 5`), `extrusion_rate_smoothing_external_perimeter_only` bool `false` — with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd schema_declares_all_three_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (slope `0`), **when** a mixed-role fixture (outer/inner walls, sparse, one travel) is emitted, **then** the command stream is byte-identical to the pre-packet stream — same command count, same `F` values, same `E` values — and the CONFIG_BLOCK keeps its line count with zero value changes (host-only omitted, no padding twin shadowed). | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd default_path_is_byte_identical 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `max_volumetric_extrusion_rate_slope` set to a small non-default value (e.g. `5.0`), **when** a high-flow → low-flow transition fixture is emitted (fast wide outer wall into slow narrow sparse), **then** at least one move's `F` is rewritten downward versus the unsaturated stream, every move's `E` is unchanged (the stage only retimes, never re-volumes), and no travel/retract command is modified. | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd slope_limit_retimes_transition 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `extrusion_rate_smoothing_external_perimeter_only = true` with smoothing engaged, **when** fixtures with an inner-wall transition and an outer-wall transition are emitted, **then** the inner-wall transition's `F` values are untouched while the outer-wall transition's are retimed; an entity carrying any `overhang_quartile`-marked point counts as overhang for this gate. | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd external_only_gate 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** smoothing engaged on a long straight move, **when** `max_volumetric_extrusion_rate_slope_segment_length` is `1.0` versus `5.0`, **then** the `1.0` run emits strictly more `Move` commands over the same span (finer splitting), both runs conserve total `E`, and a rate delta below the trivial floor emits a single unmodified move. | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd segment_length_controls_splitting 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (`max_volumetric_extrusion_rate_slope = -1`, `max_volumetric_extrusion_rate_slope_segment_length = 0.4` or `5.1`, bool spelled `"yes"`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins/maxes are GUI hints — DEV-182(a)). | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** `max_volumetric_extrusion_rate_slope = 0` with the segment length and bool at non-default values, **when** the fixture is emitted, **then** the stream is byte-identical to defaults (the slope master gate owns the whole stage — the companions are inert without it). | `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd zero_slope_disables_whole_stage 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test speed_p57_ers_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — smoothing retimes no E and selects no algorithm; rule 4 trigger test does not fire: in-module emission parameter, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'max_volumetric_extrusion_rate_slope' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'max_volumetric_extrusion_rate_slope' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-182" - `rg -q 'DEV-182' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the three keys' declared types/defaults/bounds (confirm slope coFloat `0` min `0`, segment length coFloat `3.0` min `0.5` max `5`, external-only coBool `false`, all scalar; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_do_export` equalizer construction gate (`max_volumetric_extrusion_rate_slope.value > 0` → `make_unique<PressureEqualizer>`, else the stage does not exist) and the `;_EXTRUDE_SET_SPEED` / `;_EXTRUDE_END` marker emission in `GCode::_extrude` (borrow the gate; the port has no marker blocks at all — the blocks switch off only on the sequential-object path per ticket 32 — so every print move is adjustable)
- `OrcaSlicerDocumented/src/libslic3r/GCode/PressureEqualizer.cpp` — ctor (slope × 3600 unit step, segment-length + bool capture, all-roles-equal slope tables, ironing zeroed) and `adjust_volumetric_rate` (the skip list — bridge + ironing + external-only gate — and the forward/backward limiter shape) (borrow the skip list exactly and the limiter shape; the ×3600 is implicit in the port's mm/min rate math — do not re-multiply; the text-parsing and one-layer-lookbehind machinery is not borrowed — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCode/PressureEqualizer.cpp` — `output_gcode_line` (segment splitting at `m_max_segment_length`, the trivial-delta floor, accel-then-decel vs single-slope emission) (borrow the split rule and the floor; port onto `GCodeCommand::Move` insertion)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::process_layers` pipeline order (borrow the after-generation position — the port has no spiral stage and no tbb, so only the position transfers, not the filter shape)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
