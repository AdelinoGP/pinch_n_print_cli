---
status: draft
packet: 280-bed-mesh-adaptive-placeholders
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/53-author-packet-p46-printer-machine-bed-mesh-emitter.md (wayfinder ticket 53)
context_cost_estimate: M
---

# Packet Contract: 280-bed-mesh-adaptive-placeholders

## Goal

Make the machine-gcode-emit post-pass compute conservative adaptive bed-mesh bounds, probe counts, and algorithm placeholders from its emitted moves while retaining the four canonical printer/machine settings as typed module inputs.

## Scope Boundaries

This packet owns `adaptive_bed_mesh_margin`, `bed_mesh_max`, `bed_mesh_min`, and `bed_mesh_probe_distance` in `machine-gcode-emit`, plus the derived scalar site variables consumed by machine-start-G-code substitution. It does not add host keys, WIT/IR fields, a new module, or serializer padding; canonical first-layer-hull behavior is recorded where this post-pass seam necessarily diverges.

## Prerequisites and Blockers

- Depends on the existing machine-gcode-emit `ConfigView`, `GCodeCommand::Move`, and single-pass placeholder substitution.
- Unblocks ticket 53's P46 packet-authoring closure and machine-start-G-code bed-mesh templates.
- Activation blockers: none; status remains `draft` until explicitly activated.

## Acceptance Criteria

- **AC-1. Given** canonical defaults (`bed_mesh_min=(-99999,-99999)`, `bed_mesh_max=(99999,99999)`, `bed_mesh_probe_distance=(50,50)`, `adaptive_bed_mesh_margin=0`), **when** two otherwise-identical streams use a small and a large XY `Move` bounding box, **then** each adaptive bound follows its stream, and the larger box has probe counts no smaller than the small box with algorithm selected from the product. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd defaults_small_large_bbox 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** a nonzero `adaptive_bed_mesh_margin`, **when** the post-pass computes placeholders, **then** each unclamped adaptive min is reduced and max increased by exactly the margin in mm. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd margin_expands_bounds 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** explicit `bed_mesh_min` and `bed_mesh_max`, **when** the moved bounding box plus margin exceeds them, **then** `adaptive_bed_mesh_min_{x,y}` and `adaptive_bed_mesh_max_{x,y}` are clamped independently to those configured limits. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd configured_bounds_clamp 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** either probe-distance component below `1.0` mm, **when** probe counts are computed, **then** the component is floored to `1.0` before `ceil(size / distance) + 1`, and each axis remains at least `3`. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd probe_distance_floor 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** probe-count products on both sides of `6`, **when** placeholders are emitted, **then** product `<= 6` selects `lagrange` and product `> 6` selects `bicubic`, with `bed_mesh_probe_count_x` and `_y` exposing the exact integer counts. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd algorithm_boundary 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** `gcode_flavor=klipper` in `ConfigView`, or no flavor key, **when** counts are computed, **then** Klipper counts are raised to at least `4` per axis while an absent flavor uses the non-Klipper/Marlin behavior without failing substitution. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd klipper_and_missing_flavor 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** `machine_start_gcode` contains `[adaptive_bed_mesh_min_x]`, `[adaptive_bed_mesh_min_y]`, `[adaptive_bed_mesh_max_x]`, `[adaptive_bed_mesh_max_y]`, `[bed_mesh_probe_count_x]`, `[bed_mesh_probe_count_y]`, and `[bed_mesh_algo]`, **when** the post-pass runs, **then** every token resolves to its derived value and no unresolved-key warning is produced. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd template_substitution_end_to_end 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-8. Given** ADR-0050 §2 pins the placeholder domain as manifest keys plus alias only, **when** this packet lands its module site-variable escape, **then** `docs/DEVIATION_LOG.md` contains row `D-280-ADR-0050-AMENDED` and `docs/adr/0050-custom-gcode-architecture.md` contains an `## Amendment — <date> (packet 280)` section quoting the contested "exactly" sentence verbatim. | `rg -q 'ADR-0050-AMENDED' docs/DEVIATION_LOG.md && rg -q 'Amendment.*packet 280' docs/adr/0050-custom-gcode-architecture.md && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** a manifest value for any point input that is not a two-element float-list, **when** `machine-gcode-emit` resolves configuration, **then** it rejects with `TypeMismatch`; a negative probe distance is likewise rejected by the declared minimum. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd malformed_point_type_rejected 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** map Authoring rule 2, **when** this packet is implemented, **then** `crates/slicer-gcode/src/serialize.rs` and `ORCA_CONFIG_PADDING` have no diff or new bed-mesh serialization row. | `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd no_serialize_padding_change 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/01_system_architecture.md` - targeted `PostPass::GCodeEmit` and module-site-variable sections.
- `docs/03_wit_and_manifest.md` - delegated `[config.schema]`, float-list, and `ConfigView` summary.
- `docs/08_coordinate_system.md` - targeted mm/internal-unit checklist.
- `docs/specs/orca-feature-gap/map.md` - Authoring rules 1-6 and owner derivation.
- `docs/ORCASLICER_ATTRIBUTION.md` - header obligation only if a new translated Rust source is created.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - regenerate from the module manifest; verify with `rg -q 'adaptive_bed_mesh_margin' docs/15_config_keys_reference.md && rg -q 'bed_mesh_probe_distance' docs/15_config_keys_reference.md`.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - annotate all four keys as Tier B, owner `machine-gcode-emit`; verify with `rg -q 'adaptive_bed_mesh_margin.*machine-gcode-emit' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - annotate P46 as packet 280 with four retained keys; verify with `rg -q 'P46.*280' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`.
- `docs/DEVIATION_LOG.md` - one new row `D-280-ADR-0050-AMENDED` (re-derive the free `D-` number when writing it); verify with `rg -q 'ADR-0050-AMENDED' docs/DEVIATION_LOG.md`.
- `docs/adr/0050-custom-gcode-architecture.md` - one new `## Amendment` section for packet 280 quoting the section-2 exactly sentence verbatim; verify with `rg -q 'packet 280' docs/adr/0050-custom-gcode-architecture.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` declarations for the four retained keys, exact point/float types, defaults, and minimums.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::apply_print_config` adaptive bounds, probe-count, algorithm, and flavor-floor behavior.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — first-layer bounding-box/convex-hull inputs and placeholder publication consumed by machine-start templates.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
