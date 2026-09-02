# Requirements: top-surface-ironing-keys

## Packet Metadata

- Grouped task IDs: none - queue packet; implementation is recorded against [21 - Author packet P14 - Quality / Ironing - top-surface-ironing](../specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md).
- Backlog source: `docs/specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md` (P14 in the wayfinder map "Close the OrcaSlicer FFF feature gap").
- Packet number: allocate one directory prefix from disk at authoring time using the procedure settled by [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md); never reserve a block.
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P14 covers the four canonical Quality / Ironing keys that are absent from the live top-surface-ironing module: `ironing_angle`, `ironing_angle_fixed`, `ironing_inset`, and `ironing_type`. The module currently reads a non-canonical `ironing_enabled` bool and emits horizontal strokes only for `top_shell_index() == Some(0)`, so the canonical mode enum, angle controls, and inset are not reachable.

Authoring re-derived an owner correction. Canonical `Layer::make_ironing` consumes all four keys only for top-surface ironing. PnP's top and support manifests contain independent same-named `ironing_enabled` entries because each module receives a separately filtered `ConfigView`; there is no shared declaration that requires a two-manifest change. P14 therefore changes only top-surface-ironing. The support-surface-ironing gate remains for [22 - Author packet P15 - Support / Support ironing - support-surface-ironing](../specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md), whose canonical key is `support_ironing`.

## In Scope

- Replace the top module's `[config.schema.ironing_enabled]` table with canonical `ironing_type` and add exact canonical schema tables for `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset`, all in `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` with `group = "TopSurface"`.
- Extend `TopSurfaceIroning::from_config` and its stored configuration in `modules/core-modules/top-surface-ironing/src/lib.rs` to parse the four keys through `ConfigView`, reject an unknown mode, and preserve the existing default-off behavior through `ironing_type = "no ironing"`.
- Extend `TopSurfaceIroning::run_infill` and `generate_zigzag_strokes_for_polygon` so `no ironing`, `top`, `topmost`, and `solid` select the available `SliceRegionView` surface fills as specified by AC-2; rotate scan segments from the requested angle; and offset the fill inward by the configured effective inset.
- Use the already passed `layer_index` in the top module to make `ironing_angle_fixed` observable. Fixed mode uses the configured absolute angle. Non-fixed mode uses a deterministic zero-degree base plus a 90-degree layer-index turn before applying the configured offset, because the current `SliceRegionView` has no solid-infill direction metadata. This is an explicit port adaptation, not a claim that PnP has canonical solid-infill-template parity.
- Use `IRONING_LINE_WIDTH` as the existing top-module width proxy for the canonical zero-inset half-width rule; explicit inset values are millimetres and use the existing SDK offset service.
- Add a TOML schema guard at `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_config_schema_tdd.rs` and add `toml = "0.8"` to the module's dev-dependencies if absent. The module has no explicit `[[test]]` entries, so this file is an auto-discovered test binary.
- Extend `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` with mode, angle, inset, and legacy-key regression invariants.
- Update only top-owned configuration fixtures and integration tests: `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`, and `resources/test_config/benchy_combined_feature_evidence.json`.
- Extend `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` with real-manifest enum and bounds arms, and extend `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` with exact-once P14 raw-key assertions.
- Regenerate `docs/15_config_keys_reference.md` through `cargo xtask gen-config-docs` and verify it with `--check`.

## Out of Scope

- `modules/core-modules/support-surface-ironing/**`, including its `ironing_enabled` read and manifest. Canonical support ironing consumes `support_ironing`; its migration is P15.
- `ironing_pattern`, `ironing_speed`, `ironing_flow`, and `ironing_spacing` behavior; their existing top-side defaults and rectilinear path remain, except where the new effective inset changes the geometry as required by P14.
- Per-filament ironing overrides (`filament_ironing_*`); PnP has no per-filament module config model for this packet.
- New IR/WIT fields or schema-version changes. `SliceRegionView` already exposes top, bottom, and internal solid fills, and `run_infill` already receives `layer_index`.
- Host `ResolvedConfig`, `docs/config/host-keys.toml`, and `host_keys_doc_lock_tdd.rs`; these four keys are module-owned and are visible through manifest-filtered `ConfigView`.
- New CONFIG_BLOCK padding or default-list entries. `ironing_type = no ironing` is already an existing padding entry; the other three keys must be emitted only when present in raw user configuration.
- Hand edits to `docs/15_config_keys_reference.md` or `docs/ORCA_CONFIG_REFERENCE.md`.
- Exact canonical solid-infill-template angle parity until an IR input carries the base direction. That follow-up is recorded as map fog rather than hidden in this packet.

## Authoritative Docs

- `docs/02_ir_schemas.md` - delegated SUMMARY of CONFIG_BLOCK exact-once and padding rules.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY of manifest config-schema types, enum values, and inclusive bounds.
- `docs/08_coordinate_system.md` - direct read of the geometry conversion rules; the module's offset helper accepts millimetres while polygon vertices remain scaled integer units.
- `docs/15_config_keys_reference.md` - generated, targeted checks only; it is not a source file.
- `docs/ORCASLICER_ATTRIBUTION.md` - standard porting-header contract for any new translated Rust file.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` - `PrintRegionConfig` declarations for `ironing_type`, `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset`, including canonical defaults and bounds.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - `Layer::make_ironing` mode gates, canonical effective inset, angle calculation, fixed-angle flag, and `Fill::fill_surface` invocation.
- `OrcaSlicerDocumented/tests/fff_print/test_fill.cpp` - the solid-infill rotation-template invariant for ironing.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence

| Key | Canonical type | Canonical default | Bounds / values | Canonical consumer | Current PnP state | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `ironing_angle` | coFloat, degrees | `0.0` | min `0`, max `359` | `Layer::make_ironing` computes the angle offset from the base direction before `Fill::fill_surface` | absent from the top manifest and `TopSurfaceIroning` | Add a float table and rotate the existing scan generator by the configured offset |
| `ironing_angle_fixed` | coBool | `false` | none | `Layer::make_ironing` sets `fixed_angle`; the fill implementation skips its alternating-direction branch when fixed | absent from the top manifest and `TopSurfaceIroning` | Add a bool table and use `layer_index` to make fixed versus non-fixed orientation deterministic |
| `ironing_inset` | coFloat, mm | `0.0` | min `0`, max `100` | `Layer::make_ironing` maps zero to half the nozzle diameter, otherwise uses the configured inset before intersection and fill | absent from the top manifest and generator; current strokes reach the unoffset polygon | Add a float table and inward-offset surface polygons; zero uses `IRONING_LINE_WIDTH / 2` as the in-tree width proxy |
| `ironing_type` | coEnum `IroningType` | `"no ironing"` | `no ironing`, `top`, `topmost`, `solid` | `Layer::make_ironing` gates layers and selects top/bottom/internal solid surfaces; `Fill::fill_surface` receives each selected ironing area | absent from the top manifest; `ironing_enabled` only gates `top_shell_index == Some(0)` | Replace the top bool gate with the four-mode enum and map modes to the available `SliceRegionView` fills |

Canonical angle nuance: the upstream base angle includes top-layer direction and solid-infill rotation-template inputs that are absent from PnP's `SliceRegionView`. P14 uses the existing `layer_index` as a deterministic base-turn input and explicitly records exact template parity as future IR work; it does not claim that the fallback is canonical byte parity.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` exact top manifest schema; `AC-2` four-mode surface selection; `AC-3` angle, fixed-angle, and inset invariants; `AC-4` scheduler enum/bounds enforcement; `AC-5` CONFIG_BLOCK exact-once behavior; `AC-6` generated documentation.
- Negative: `AC-N1` legacy top boolean cannot enable the canonical top module.
- Cross-packet impact: P15 owns the support-surface-ironing `support_ironing` migration and must not be made to consume `ironing_type`; future IR orientation metadata may replace the explicit PnP relative-angle fallback. No other queued packet claims these four keys.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p top-surface-ironing --test top_surface_ironing_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 manifest schema guard | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2, AC-3, AC-N1 module behavior | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4 real manifest enforcement | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-5 CONFIG_BLOCK driver | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract integrated_parity_top_surface_ironing 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | top integration fixture migration | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_ironing_per_painted_top_color 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | painted top fixture migration | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e slicing_promotion_e2e_dispatch_regression 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | e2e config migration | FACT pass/fail |
| `cargo xtask gen-config-docs` | regenerate the generated reference | FACT exit code |
| `cargo xtask gen-config-docs --check` | AC-6 generated-reference check | FACT exit code |
| `cargo xtask build-guests --check` | guest freshness after manifest/source edits | FACT exit code; stale means rebuild without `--check` |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation; every test run writes the required `target/test-output.log`.

## Step Completion Expectations

- The schema guard and manifest are kept in lockstep; the guard must assert all four new entries and the removal of the top-owner `ironing_enabled` table.
- The source behavior and its emission tests land together so every mode and geometry control has an invariant before integration fixtures are migrated.
- Top-owned fixture changes must not alter support-surface-ironing's `ironing_enabled` references; the support module remains a separate ConfigView consumer.
- Generated docs are regenerated only after all manifests and source/config tests are final; guest freshness is checked after the final guest-input changes.

## Context Discipline Notes

- `modules/core-modules/top-surface-ironing/src/lib.rs` is short enough for bounded reads, but only the config parser, stroke generator, and `run_infill` path are needed; do not browse unrelated module code.
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs` and `docs/15_config_keys_reference.md` are read by targeted search/range only.
- Canonical files remain delegated and all cargo commands remain delegated; retain only FACT/LOCATIONS/SNIPPETS returns.
