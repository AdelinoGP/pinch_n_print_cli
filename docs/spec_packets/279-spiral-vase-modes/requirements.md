# Requirements: 279-spiral-vase-modes

## Packet Metadata

- Grouped task IDs: `[]` (wayfinder ticket 52; no current `docs/07_implementation_status.md` ownership row)
- Backlog source: `docs/specs/orca-feature-gap/issues/52-author-packet-p44-others-g-code-output-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P45 grouped five live OrcaSlicer keys under the host emitter plus orchestration, but the tree has no SpiralVase processing at all: all four SpiralVase keys are zero-occurrence and `spiral_mode` exists only as a CONFIG_BLOCK padding twin. Claim-time grounding found the one live fragment — a PnP-spelled `spiral_vase` boolean forcing the classic perimeter generator in scheduler dispatch — which is the same decision point as Orca's `spiral_mode`, not a second feature. The packet builds the missing SpiralVase post-processor, the orchestration validation, and the time-lapse gate clause while unifying the mode spelling, so no second mechanism is created.

## In Scope

- `spiral_mode` (`coBool`, canonical default `false`): canonical spelling adopted; scheduler dispatch reads it first with legacy `spiral_vase` as fallback alias; forces classic perimeters; enables the SpiralVase stage and validation.
- `spiral_starting_flow_ratio` (`coFloat`, canonical default `0`): first-enabled-layer flow ramp from the ratio toward 100%; `0` disables that end's ramp.
- `spiral_finishing_flow_ratio` (`coFloat`, canonical default `0`): final-layer flow ramp from 100% toward the ratio in appended transition moves; `0` disables that end's ramp.
- `spiral_mode_smooth` (`coBool`, canonical default `false`): XY interpolation toward the previous layer on/off, byte-identical path when off.
- `spiral_mode_max_xy_smoothing` (`coFloatOrPercent`, canonical default `200%`): cap on the accepted XY interpolation distance; extrusion rescales for changed segment lengths; tiny resulting moves are removed.
- SpiralVase placement as a post-processing filter over generated layers inside the host emitter (canonical `GCode::process_layers` precedent), continuous-Z ramp by cumulative extruding length, relative-extrusion requirement enforced as fatal validation.
- Orchestration validation: reject copies > 1 unless print sequence is ByObject; reject incompatible multi-material regions (canonical `Print::validate` precedent).
- Time-lapse gate: `machine-gcode-emit` suppresses TimeLapse injection while spiral is on (closes the map's time-lapse fog clause for this key).
- Typed host fields, perimeter-manifest alias rows, host-key docs, generated config docs, unit/integration tests, and 04/05 spelling annotations.

## Out of Scope

- Any slicing-side perimeter-count, shell, support, or ironing adjustment beyond the already-live classic-forcing: bounded canonical reads evidenced only the post-processing filter and validation, so deeper slicing changes are a recorded non-borrow until a delegated read proves otherwise (see design.md Open Questions).
- `spiral_vase` removal: the legacy spelling stays as a fallback alias this packet; retiring it is rename-workstream follow-up, not this packet.
- Bambu-only behavior, prime-tower body work, per-tool axis changes, post-processing scripts.
- Any edit or assertion involving `ORCA_CONFIG_PADDING`, CONFIG_BLOCK padding twins, or `crates/slicer-gcode/src/serialize.rs`.
- New WIT/IR fields, schema-version bumps, claim holders, modules.

## Key Dispositions

| Key | Disposition | Behavior-changing decision / reason | Acceptance proof |
| --- | --- | --- | --- |
| `spiral_mode` | retained, live with spelling adoption | classic-forcing under unified spelling + validation + stage enable | AC-1, AC-2, AC-3 |
| `spiral_starting_flow_ratio` | retained, live | first-layer flow ramp | AC-6 |
| `spiral_finishing_flow_ratio` | retained, live | final-layer flow ramp | AC-6 |
| `spiral_mode_smooth` | retained, live | XY smoothing gate | AC-5 |
| `spiral_mode_max_xy_smoothing` | retained, live | smoothing distance cap | AC-5 |

There are zero declaration-only retained keys and zero keys returned to the queue. CONFIG_BLOCK padding is neither a disposition nor evidence.

## Authoritative Docs

- `docs/01_system_architecture.md` - targeted `PostPass::GCodeEmit` and orchestration-validation sections.
- `docs/03_wit_and_manifest.md` - delegated targeted summary for `[config.schema]` host-key behavior.
- `docs/08_coordinate_system.md` - targeted mm/internal-unit checklist for smoothing and Z-ramp math.
- `docs/21_data_defaults_and_fixtures.md` - targeted struct-literal rule.
- `docs/specs/orca-feature-gap/map.md` Notes, Authoring rules 1–6, time-lapse fog patch, and canonical value-spelling note - targeted reads.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` declarations: bool defaults (`spiral_mode=false`, `spiral_mode_smooth=false`), float defaults (`spiral_starting_flow_ratio=0`, `spiral_finishing_flow_ratio=0`), and `FloatOrPercent` default (`spiral_mode_max_xy_smoothing=200%`).
- `OrcaSlicerDocumented/src/libslic3r/GCode/SpiralVase.cpp` — `SpiralVase::process_layer` Z-ramp, XY interpolation, flow-transition, and tiny-move-removal behavior deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SpiralVase.hpp` — `SpiralVase::SpiralVase` constructor role of `spiral_mode_smooth` deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — spiral branch of both `GCode::process_layers` overloads (post-processing-filter placement) deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` spiral copy-count and material-compatibility rejections deliberately borrowed.
- Slicing-side reads (`LayerRegion::make_perimeters`, `PrintObject::detect_surfaces_type`, `Fill/Fill.cpp`) — deliberately NOT borrowed until a delegated read evidences a slicing change beyond classic-forcing.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md` prove every retained key changes behavior at a non-default value, with default-path compatibility (classic dispatch off, no smoothing, no ramps, timelapse intact) covered inside each test.
- Negative: `AC-N1` rejects invalid spellings/types; `AC-N2` enforces the relative-extrusion boundary; `AC-N3` protects CONFIG_BLOCK padding.
- Cross-packet impact: the spiral seam satisfies the map's time-lapse fog obligation for this key; the `spiral_vase` fallback alias is recorded in 04/05 so the rename workstream can retire it later; no other packet owns SpiralVase (packet 264's octagram-spiral infill is a different feature).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test spiral_vase_modes_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Z-ramp, smoothing gate/cap, flow ramps | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Alias dispatch, validation rejections, strict spelling | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test spiral_timelapse_gate_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | TimeLapse suppression under spiral | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration spiral_vase_modes_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Relative-extrusion boundary end to end | FACT pass/fail |
| `cargo xtask build-guests --check` | Manifest/module artifact freshness; rebuild without `--check` if stale | FACT exit code |
| `cargo xtask gen-config-docs --check` | Generated config reference | FACT exit code |
| `cargo check --workspace --all-targets` | All targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal gate | FACT exit code |

## Step Completion Expectations

- Land ResolvedConfig fields and schemas before behavior so invalid spellings cannot reach the SpiralVase stage through the normal resolved path.
- The alias reads `spiral_mode` first and `spiral_vase` only as fallback; when both are present, `spiral_mode` wins and one fallback note is logged, never an error.
- `0` disables its own flow-ramp end only; the other end still ramps.
- Capture `target/test-output.log` findings before each later cargo command overwrites it.

## Context Discipline Notes

- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-gcode/src/emit.rs`, and scheduler execution-plan files are long; locate symbols first and use ranged reads.
- Canonical reads are delegated only; the packet's behavior list comes from bounded authoring-time delegation summaries.
- `spiral_mode_max_xy_smoothing` is a percent-kind key: spell test values the canonical string way and see design.md for the ticket-128 hazard.
