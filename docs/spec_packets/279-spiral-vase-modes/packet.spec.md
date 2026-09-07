---
status: draft
packet: 279-spiral-vase-modes
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/52-author-packet-p44-others-g-code-output-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: 279-spiral-vase-modes

## Goal

Make `spiral_mode` and its four SpiralVase keys (`spiral_starting_flow_ratio`, `spiral_finishing_flow_ratio`, `spiral_mode_smooth`, `spiral_mode_max_xy_smoothing`) drive live spiral-vase decisions through the host emitter and orchestration validation, unifying the existing live `spiral_vase` perimeter dispatch under Orca's spelling; nothing returns to the queue.

## Scope Boundaries

This packet adds a SpiralVase post-processing stage to `DefaultGCodeEmitter` (continuous-Z ramp, optional XY smoothing under its cap, first/last-layer flow ramps, tiny-move removal), adopts canonical `spiral_mode` as the mode spelling with legacy `spiral_vase` as a fallback alias (no second mechanism), adds orchestration validation (single-copy unless ByObject, material compatibility, relative-extrusion requirement), and extends the machine-gcode-emit time-lapse gate with the missing `!spiral` clause. It adds no WIT/IR field beyond config, no claim, and no module; CONFIG_BLOCK padding, post-processing scripts, and the prime-tower body are excluded.

## Prerequisites and Blockers

- Depends on: wayfinder decisions 06, 101, and 107 named by ticket 52 (all resolved at authoring time; re-derive before activation).
- Unblocks: ticket 52's packet-authoring closure; provides the spiral seam the map's time-lapse fog patch waits on.
- Activation blockers: none; status remains `draft` until explicitly activated.

## Acceptance Criteria

- **AC-1. Given** the scheduler's perimeter-claim dispatch, **when** `spiral_mode = true`, **then** the classic perimeter generator is forced for the `perimeter-generator` claim regardless of `wall_generator` (Arachne incompatible); **when** `spiral_mode` is absent but legacy `spiral_vase = true`, the same forcing applies; **when** both are absent, dispatch follows `wall_generator`. | `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** spiral mode enabled, **when** the print has more than one copy and print sequence is not ByObject, **then** orchestration validation rejects the slice with a fatal spiral-copies error naming the copy count. | `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** spiral mode enabled, **when** the regions carry incompatible multiple materials, **then** orchestration validation rejects the slice with a fatal spiral-material error. | `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** two consecutive spiral-enabled layers, **when** the emitter runs its SpiralVase stage, **then** the later layer's initial Z equals the earlier layer's Z and Z ramps continuously across the loop by cumulative extruding length (no layer-change Z step inside the spiral). | `cargo test -p slicer-gcode --test spiral_vase_modes_tdd z_ramp_continuous 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** spiral smoothing enabled vs disabled, **when** `spiral_mode_smooth = true` with `spiral_mode_max_xy_smoothing = <cap>`, **then** XY positions interpolate toward the nearest point on the previous layer only while within the cap and extrusion rescales for the changed segment length; **when** `spiral_mode_smooth = false`, XY is byte-identical to the unsmoothed path. | `cargo test -p slicer-gcode --test spiral_vase_modes_tdd xy_smoothing_gated_by_flag_and_cap 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** relative extrusion and non-zero ratios, **when** `spiral_starting_flow_ratio = <r1>` and `spiral_finishing_flow_ratio = <r2>`, **then** the first enabled layer ramps flow from `r1` to 100% and the final layer ramps from 100% to `r2` in appended transition moves; **when** a ratio is `0` (canonical default), that end emits no transition moves. | `cargo test -p slicer-gcode --test spiral_vase_modes_tdd flow_ramps_first_last_layers 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** a print with time-lapse injection configured, **when** spiral mode is enabled, **then** `machine-gcode-emit` emits no TimeLapse injection (the `!spiral` clause); **when** spiral mode is off on a non-suppressing structure, the TimeLapse injection remains. | `cargo test -p machine-gcode-emit --test spiral_timelapse_gate_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Negative Test Cases

- **AC-N1. Given** the owner schemas loaded, **when** `spiral_mode = "yes"` (word-form) or `spiral_mode_max_xy_smoothing` carries a non-numeric string, **then** resolution rejects each with a type error; canonical bool spelling is the only accepted bool form. | `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** spiral mode enabled with absolute extrusion, **when** the slice reaches validation, **then** it fails with a fatal spiral-requires-relative-extrusion error before any SpiralVase processing. | `cargo test -p slicer-runtime --test integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N3. Given** map Authoring rule 2, **then** this packet's diff does not change `ORCA_CONFIG_PADDING` or `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/01_system_architecture.md` - targeted `PostPass::GCodeEmit` and print-orchestration validation sections.
- `docs/03_wit_and_manifest.md` - delegated targeted summary for `[config.schema]` host-key behavior.
- `docs/08_coordinate_system.md` - targeted mm/internal-unit checklist for XY smoothing and Z-ramp math.
- `docs/21_data_defaults_and_fixtures.md` - targeted struct-literal rule (ResolvedConfig gains fields).
- `docs/specs/orca-feature-gap/map.md` Notes, Authoring rules 1–6, time-lapse fog patch, and canonical value-spelling note - targeted reads.

## Doc Impact Statement (Required)

- `docs/config/host-keys.toml` `[resolved_config]` - add `spiral_mode`, `spiral_starting_flow_ratio`, `spiral_finishing_flow_ratio`, `spiral_mode_smooth`, `spiral_mode_max_xy_smoothing`; verify with `rg -q 'spiral_mode' docs/config/host-keys.toml && rg -q 'spiral_starting_flow_ratio' docs/config/host-keys.toml && rg -q 'spiral_finishing_flow_ratio' docs/config/host-keys.toml && rg -q 'spiral_mode_smooth' docs/config/host-keys.toml && rg -q 'spiral_mode_max_xy_smoothing' docs/config/host-keys.toml`.
- `docs/15_config_keys_reference.md` - regenerate, never hand-edit, after declarations land; verify with `cargo xtask gen-config-docs --check`.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` and `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - annotate `spiral_mode` adopted as canonical spelling over live PnP `spiral_vase` (dispatch unified, no count change; P45 membership stays 5 keys); verify with `rg -q 'spiral_mode.*spiral_vase' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md && rg -q 'spiral_mode' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`.
- Perimeter owner manifests (`arachne-perimeters`, `classic-perimeters`) - declare `spiral_mode` alongside the existing `spiral_vase` row during the alias transition; verify with `rg -q 'spiral_mode' modules/core-modules/arachne-perimeters/*.toml && rg -q 'spiral_mode' modules/core-modules/classic-perimeters/*.toml`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` declarations: bool defaults (`spiral_mode=false`, `spiral_mode_smooth=false`), float defaults (`spiral_starting_flow_ratio=0`, `spiral_finishing_flow_ratio=0`), and `FloatOrPercent` default (`spiral_mode_max_xy_smoothing=200%`).
- `OrcaSlicerDocumented/src/libslic3r/GCode/SpiralVase.cpp` — `SpiralVase::process_layer` Z-ramp, XY interpolation, flow-transition, and tiny-move-removal behavior deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SpiralVase.hpp` — `SpiralVase::SpiralVase` constructor role of `spiral_mode_smooth` deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — spiral branch of both `GCode::process_layers` overloads (post-processing-filter placement) deliberately borrowed.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` spiral copy-count and material-compatibility rejections deliberately borrowed.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
