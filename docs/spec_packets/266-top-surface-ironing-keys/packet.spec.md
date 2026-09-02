---
status: draft
packet: top-surface-ironing-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: top-surface-ironing-keys

## Goal

Replace top-surface-ironing's legacy `ironing_enabled` gate with the canonical `ironing_type` mode and wire the canonical `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset` controls through the existing top-surface emission path, with invariant coverage for every mode and geometry control.

## Scope Boundaries

This packet changes the top-surface-ironing manifest and module, top-owned tests and configuration fixtures, scheduler bounds coverage, the existing CONFIG_BLOCK integration coverage, and the generated config reference. It preserves the existing support-surface-ironing `ironing_enabled` gate because canonical support ironing consumes `support_ironing`; that independent support migration belongs to [22 - Author packet P15 - Support / Support ironing - support-surface-ironing](../specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md), not this packet. No IR, WIT, host `ResolvedConfig` field, or new CONFIG_BLOCK padding key is required.

## Prerequisites and Blockers

- Depends on [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md), [05 - Decide packet granularity and grouping](../specs/orca-feature-gap/issues/05-packet-granularity.md), [07 - Document the Orca to Pinch alias map and retire the hand-maintained column](../specs/orca-feature-gap/issues/07-alias-map-and-column-retirement.md), and [106 - Rename ironing keys to Orca names](../specs/orca-feature-gap/issues/106-rename-ironing-keys.md); all are resolved map decisions.
- Unblocks [21 - Author packet P14 - Quality / Ironing - top-surface-ironing](../specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md) for packet authoring completion and supplies the boundary P15 must preserve.
- Activation blockers: none for the draft packet; activation remains a separate explicit `/swarm` decision.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`, **when** its `[config.schema]` is parsed, **then** it contains `ironing_type` as an enum with values `['no ironing', 'top', 'topmost', 'solid']` and default `'no ironing'`, `ironing_angle` as a float with default `0.0`, min `0.0`, max `359.0`, `ironing_angle_fixed` as a bool with default `false`, and `ironing_inset` as a float with default `0.0`, min `0.0`, max `100.0`; each has a `display` and `group = 'TopSurface'`, and the top manifest no longer declares `ironing_enabled`. | `cargo test -p top-surface-ironing --test top_surface_ironing_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** `SliceRegionView` fixtures with top, bottom, and internal solid fills, **when** `TopSurfaceIroning::run_infill` receives each `ironing_type`, **then** `no ironing` emits no paths, `topmost` emits only a region with `top_shell_index() == Some(0)`, `top` also emits deeper top-shell regions, and `solid` emits the available top/bottom/internal solid fills while excluding bridge-only regions; every emitted path has `ExtrusionRole::Ironing`. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd ironing_type 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** a square top fill and a layer index, **when** `ironing_angle` and `ironing_angle_fixed` are varied, **then** a fixed 90-degree angle produces vertical scan segments, a non-fixed angle uses the deterministic layer-index base turn plus the configured degree offset, and the default layer-0 angle preserves the horizontal baseline; when `ironing_inset = 1.0`, all emitted points remain inside the one-millimetre inward offset, while `ironing_inset = 0.0` uses the module's half-`IRONING_LINE_WIDTH` effective inset. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd ironing_angle 2>&1 | tee target/test-output.log | grep -E '^test result'` and `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd ironing_inset 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** the real top-surface-ironing manifest is loaded into `ConfigBoundsIndex`, **when** global configuration resolves, **then** `ironing_type = 'sideways'`, `ironing_angle = -1.0`, and `ironing_inset = -1.0` are rejected with the existing enum `TypeMismatch` or numeric `OutOfRange` errors, while `ironing_type = 'topmost'`, `ironing_angle = 359.0`, and `ironing_inset = 100.0` resolve successfully. | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** a runtime slice with explicit raw values for `ironing_type`, `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset`, **when** the CONFIG_BLOCK is serialized, **then** each exact `; key = value` line appears once, the user `ironing_type` value replaces the existing padded `no ironing` line rather than duplicating it, and no new padding/default entry is added for the three other keys. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** the manifests are final, **when** `cargo xtask gen-config-docs` regenerates `docs/15_config_keys_reference.md`, **then** the generated module table contains the four top-surface-ironing keys under the top owner, omits the removed top-owner `ironing_enabled` row, and the existing support-surface-ironing `ironing_enabled` row remains present; the generated deviation section gains no P14 row. | `cargo xtask gen-config-docs --check && rg -q 'ironing_type' docs/15_config_keys_reference.md && rg -q 'ironing_angle_fixed' docs/15_config_keys_reference.md && rg -q 'support-surface-ironing.*ironing_enabled|ironing_enabled.*support-surface-ironing' docs/15_config_keys_reference.md`

## Negative Test Cases

- **AC-N1. Given** a direct top-surface module config containing legacy `ironing_enabled = true` but no `ironing_type`, **when** `TopSurfaceIroning::from_config` and `run_infill` are invoked, **then** the top module defaults to canonical `no ironing` and emits no ironing path; the support module's separate legacy gate is not changed by this packet. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd legacy_ironing_enabled_is_not_a_top_mode 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` and `cargo xtask build-guests --check; echo "exit=$?"`

## Authoritative Docs

- `docs/02_ir_schemas.md` - delegated summary of the CONFIG_BLOCK contract; it governs exact-once raw-key emission and padding behavior.
- `docs/03_wit_and_manifest.md` - delegated summary of `[config.schema]` enum, float, and bool forms and host-enforced bounds.
- `docs/08_coordinate_system.md` - direct geometry-unit contract; mm configuration values must cross into scaled polygon operations through the existing helpers.
- `docs/15_config_keys_reference.md` - generated output; regenerated and checked, never hand-edited.
- `docs/ORCASLICER_ATTRIBUTION.md` - standard header required if the implementation adds a new Rust file containing translated canonical logic.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` generated module-key table - regenerated by `cargo xtask gen-config-docs`; verify with `rg -q 'ironing_type' docs/15_config_keys_reference.md`, `rg -q 'ironing_angle_fixed' docs/15_config_keys_reference.md`, and the AC-6 support-owner preservation grep. No hand edit is allowed.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` - canonical P14 declarations, enum values, defaults, bounds, and the per-region ownership of the four keys.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - `Layer::make_ironing` mode selection, effective inset, angle offset, fixed-angle flag, and the `Fill::fill_surface` call.
- `OrcaSlicerDocumented/tests/fff_print/test_fill.cpp` - invariant coverage for ironing angle selection and the solid-infill rotation-template case.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
