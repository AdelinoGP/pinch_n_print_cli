---
status: draft
packet: 278-gcode-output-emitter-modes
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/51-author-packet-p44-others-g-code-output-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: 278-gcode-output-emitter-modes

## Goal

Make `exclude_object`, `gcode_comments`, `gcode_flavor`, `gcode_label_objects`, and `reduce_infill_retraction` control their live G-code decisions, using the host emitter/dialect for output syntax and the existing path-optimization module for retract policy; return mis-owned `filename_format` and the Bambu-only `support_object_skip_flush` rider to the queue with their missing carriers named.

## Scope Boundaries

This packet adds object-boundary commands and labels, opt-in extrusion diagnostics, exact flavor validation, and a configurable internal-travel retract policy. It uses `PrintEntity.region_key.object_id`, existing `GcodeFlavor`, and the existing `Layer::PathOptimization` decision surface; it adds no WIT/IR field, claim, or module. `filename_format`, Bambu `M624`/`M625`, CONFIG_BLOCK padding, and post-processing scripts are excluded.

## Prerequisites and Blockers

- Depends on resolved wayfinder decisions 06, 101, and 107 named by ticket 51; re-derive their status before activation.
- Unblocks ticket 51's packet-authoring closure and provides the generic object-boundary seam needed by later Bambu labeling work.
- Activation blockers: none; status remains `draft` until explicitly activated.

## Acceptance Criteria

- **AC-1. Given** a layer whose entities form object runs `part-a`, `part-b`, then `part-a`, **when** `exclude_object = true` and `gcode_flavor = "klipper"`, **then** `DefaultGCodeEmitter::emit_gcode` emits one leading `EXCLUDE_OBJECT_DEFINE NAME=pnp_706172742d61` and one `...pnp_706172742d62`, and brackets the three runs with `EXCLUDE_OBJECT_START NAME=<name>` / `EXCLUDE_OBJECT_END NAME=<name>` in run order; **when** `exclude_object = false`, none of `EXCLUDE_OBJECT_DEFINE`, `EXCLUDE_OBJECT_START`, `EXCLUDE_OBJECT_END`, or `M486` occurs. | `cargo test -p slicer-gcode --test gcode_output_modes_tdd exclude_object_klipper_runs 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** the same object runs and `exclude_object = true`, **when** flavor is `marlin`, `marlin2`, or `reprapfirmware`, **then** object names are assigned zero-based ordinals by lexicographically sorted raw object ID and every run is bracketed by `M486 S<ordinal>` / `M486 S-1`; **when** flavor is `repetier`, none of `M486` or `EXCLUDE_OBJECT_` occurs. | `cargo test -p slicer-gcode --test gcode_output_modes_tdd exclude_object_firmware_matrix 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** object ID `part-a`, **when** `gcode_label_objects = true` (canonical default), **then** every contiguous run is bracketed by `; OBJECT_START id=part-a` and `; OBJECT_END id=part-a`; **when** the non-default `gcode_label_objects = false`, those labels are absent while enabled `M486`/`EXCLUDE_OBJECT_` markers remain unchanged. CR/LF in an ID is escaped as `_` in comments and encoded in firmware names, never emitted as a new G-code line. | `cargo test -p slicer-gcode --test gcode_output_modes_tdd human_object_labels_and_injection_guard 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** a two-segment sparse-infill entity, **when** the non-default `gcode_comments = true`, **then** the command stream contains `; filament: tool=0 role=Sparse infill` immediately before that entity's first extrusion move; **when** `gcode_comments = false` (canonical default), that diagnostic is absent while structural `;LAYER_CHANGE`, `;Z:`, `;HEIGHT:`, and `;TYPE:Sparse infill` comments remain byte-identical. | `cargo test -p slicer-gcode --test gcode_output_modes_tdd verbose_extrusion_comments_gate 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** the exact accepted `gcode_flavor` strings `marlin`, `klipper`, `reprapfirmware`, `repetier`, and `marlin2`, **when** they are resolved through the loaded G-code schema carrier and runtime flavor path, **then** each maps to the matching `GcodeFlavor` variant and a non-default `klipper` run emits `EXCLUDE_OBJECT_START` plus `SET_PRESSURE_ADVANCE ADVANCE=0.0500`, while otherwise-identical `marlin` emits `M486 S0` plus `M900 K0.0500`. | `cargo test -p slicer-runtime --test integration gcode_output_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** two consecutive non-perimeter `OrderedEntityView` entries with the same `RegionKey`, a non-empty matching `PerimeterRegionView::sparse_infill_area()`, and a travel segment contained by that polygon, **when** `reduce_infill_retraction = true` (non-default), **then** `PathOptimizationDefault::run_path_optimization` emits the travel with no `Retract`, `Unretract`, or `ZHop`; **when** the key is `false` (canonical default), it emits the matched `Retract` / travel `Move` / `Unretract` and configured Z-hop. A perimeter destination, an empty sparse area, a segment leaving the polygon, or different region keys retracts in both modes. | `cargo test -p path-optimization-default --test travel_policy_tdd reduce_infill_retraction 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** `support_object_skip_flush` remains queued, **when** this packet is complete, **then** it has zero behavioral production occurrences under `crates/` and `modules/` after excluding `ORCA_CONFIG_PADDING`'s serializer row; the packet does not fabricate Bambu `M624`/`M625` without a Bambu flavor and a filament-flush toolchange carrier. | `rg -n 'support_object_skip_flush' crates modules --glob '!**/target/**' --glob '!crates/slicer-gcode/src/serialize.rs' --glob '*.rs' --glob '*.toml' && echo FAIL || echo PASS`

## Negative Test Cases

- **AC-N1. Given** the owner manifests loaded into `ConfigBoundsIndex`, **when** `gcode_flavor = "Marlin2"`, `exclude_object = "true"`, or `reduce_infill_retraction = "yes"`, **then** resolution rejects each with `slicer_ir::resolved_config::ConfigResolutionError::TypeMismatch`; case folding and word-form booleans are not silently accepted. | `cargo test -p slicer-scheduler --test scheduler_integration gcode_output_modes_bounds_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** `filename_format` is returned to host-export, **then** this packet adds no production occurrence of it: `pnp_cli slice` retains explicit `--output <PATH>` or stdout behavior, and the emitter never chooses filesystem paths. | `git diff --unified=0 -- crates modules | grep -q 'filename_format' && echo FAIL || echo PASS`
- **AC-N3. Given** map Authoring rule 2, **then** this packet's diff does not change `ORCA_CONFIG_PADDING` or `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test gcode_output_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/01_system_architecture.md` - `PostPass::GCodeEmit`, G-code flavor resolution, and CLI output lifecycle sections (targeted reads).
- `docs/03_wit_and_manifest.md` - manifest config schema and bounds contract (delegated targeted summary during implementation).
- `docs/08_coordinate_system.md` - mm/internal-unit conversion contract for sparse-area segment containment (targeted checklist).
- `docs/21_data_defaults_and_fixtures.md` - struct-literal churn gate (targeted read if a field blast radius changes).
- `docs/ORCASLICER_ATTRIBUTION.md` - required header only if implementation creates a new Rust source file containing translated canonical logic.

## Doc Impact Statement (Required)

- `docs/config/host-keys.toml` `[resolved_config]` - add host-consumed `exclude_object`, `gcode_comments`, and `gcode_label_objects`; verify with `rg -q 'exclude_object' docs/config/host-keys.toml && rg -q 'gcode_comments' docs/config/host-keys.toml && rg -q 'gcode_label_objects' docs/config/host-keys.toml`.
- `docs/15_config_keys_reference.md` - regenerate, never hand-edit, after the two owner manifests declare the retained keys; verify with `cargo xtask gen-config-docs --check`.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` and `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - annotate `filename_format` returned to host-export, `reduce_infill_retraction` owner-corrected to `path-optimization-default`, and `support_object_skip_flush` sequenced after a Bambu/M624 flush carrier; verify with `rg -q 'filename_format.*host-export' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md && rg -q 'support_object_skip_flush' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — exact types/defaults and five accepted `gcode_flavor` spellings for all retained/returned keys.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::apply_print_config`, `GCode::process_layer`, `GCode::_extrude`, `GCode::needs_retraction`, and `GCode::_print_first_layer_extruder_temperatures` behavior.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::apply_print_config` and flavor-specific command/comment syntax.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` and `OrcaSlicerDocumented/src/libslic3r/PrintBase.cpp` — `Print::output_filename` / `PrintBase::output_filename`, proving `filename_format` owns export-path naming rather than command emission.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
