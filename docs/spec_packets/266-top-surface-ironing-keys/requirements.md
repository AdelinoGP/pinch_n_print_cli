# Requirements: 266-top-surface-ironing-keys

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

- Replace the top module's `[config.schema.ironing_enabled]` table with canonical `ironing_type` and add exact canonical schema tables for `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset`, all in `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` with `group = "TopSurface"`; and co-declare `[config.schema.infill_direction]` there, byte-identical to the table `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` already carries, because the base angle is read from it and `ConfigView::from_declared` drops undeclared keys. `infill_direction` is a supporting input, not a fifth P14 key, and is not counted toward P14 coverage.
- Extend `TopSurfaceIroning::from_config` and its stored configuration in `modules/core-modules/top-surface-ironing/src/lib.rs` to parse the four keys plus `infill_direction` through `ConfigView`, reject an unknown mode, and preserve the existing default-off behavior through `ironing_type = "no ironing"`.
- Extend `TopSurfaceIroning::run_infill` and `generate_zigzag_strokes_for_polygon` so `no ironing`, `top`, `topmost`, and `solid` select the available `SliceRegionView` surface fills as specified by AC-2; rotate scan segments from the requested angle; and offset the fill inward by the configured effective inset.
- Make `ironing_angle_fixed` observable the canonical way: fixed mode uses the configured absolute angle; non-fixed mode uses **base + `ironing_angle`**, where the base is the `infill_direction` value the top module reads from its own `ConfigView`. `infill_direction` is the established in-tree base-angle mechanism — `RectilinearInfill::from_config` and `GyroidInfill::from_config` both read it with `config.get("infill_direction")`, and it is a typed host field on `ResolvedConfig` (default `45.0`). Co-declaring it in the top manifest costs no IR, WIT, or host change and makes the ironing angle canonically *relative* rather than absolute. The earlier draft of this packet substituted a layer-index parity turn for the base direction; that invention is withdrawn (see `design.md` §Recorded divergences, DIV-266-A).
- Use `IRONING_LINE_WIDTH` as the existing top-module width proxy for the canonical zero-inset half-width rule; explicit inset values are millimetres and use the existing SDK offset service.
- Add a TOML schema guard at `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_config_schema_tdd.rs` and add `toml = "0.8"` to the module's dev-dependencies if absent. The module has no explicit `[[test]]` entries, so this file is an auto-discovered test binary.
- Extend `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` with mode, angle, inset, and legacy-key regression invariants.
- Update only top-owned configuration fixtures and integration tests: `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`, and `resources/test_config/benchy_combined_feature_evidence.json`.
- Extend `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` with real-manifest enum and bounds arms.
- Regenerate `docs/15_config_keys_reference.md` through `cargo xtask gen-config-docs` and verify it with `--check`.

## Out of Scope

- `modules/core-modules/support-surface-ironing/**`, including its `ironing_enabled` read and manifest. Canonical support ironing consumes `support_ironing`; its migration is P15.
- `ironing_pattern`, `ironing_speed`, `ironing_flow`, and `ironing_spacing` behavior; their existing top-side defaults and rectilinear path remain, except where the new effective inset changes the geometry as required by P14.
- Per-filament ironing overrides (`filament_ironing_*`); PnP has no per-filament module config model for this packet.
- New IR/WIT fields or schema-version changes. `SliceRegionView` already exposes top, bottom, and internal solid fills, and `run_infill` already receives `layer_index`.
- Host `ResolvedConfig`, `docs/config/host-keys.toml`, and `host_keys_doc_lock_tdd.rs`; these four keys are module-owned and are visible through manifest-filtered `ConfigView`.
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) and every CONFIG_BLOCK twin, in both directions: this packet neither adds, corrects, nor asserts one. Under the map's Authoring rule 2 the padding table is not parity evidence and is never a deliverable; whatever these keys do in the CONFIG_BLOCK is a side effect of their being live and is not measured here. The previous draft carried an AC on it; that AC is withdrawn.
- Hand edits to `docs/15_config_keys_reference.md` or `docs/ORCA_CONFIG_REFERENCE.md`.
- Tracking the fill module's *computed* angle rather than the shared `infill_direction` **input**. Verified this session: `RectilinearInfill` uses `base_angle = infill_direction` unmodified for sparse, top-solid, bottom-solid, and internal-solid fill, so the two agree exactly today; `GyroidInfill` adds a module-private `CORRECTION_ANGLE_DEG`, so ironing over a gyroid-filled region would be off by that correction. Closing that gap would require the fill module's angle to reach `SliceRegionView`, which is an IR schema bump on `slicer_ir::slice_ir::SlicedRegion` plus a WIT accessor on `resource slice-region-view` — out of this packet's authorization. Recorded as `DIV-266-B` in `design.md` with rationale, not as fog.
- Canonical's per-region solid-infill **rotation template**, and the `solid_infill_direction` / `sparse_infill_rotate_template` / `solid_infill_rotate_template` keys. Re-derived from disk: those three names have **zero** `.rs` / `.toml` / `.wit` occurrences in this tree and exist only in the draft packet `docs/spec_packets/262a-infill-angle-and-multiline-keys/`. They belong to that packet, not this one; when they land, this module's base angle should follow whatever key they make authoritative.

## Authoritative Docs

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
| `ironing_angle` | coFloat, degrees | `0.0` | min `0`, max `359` | `Layer::make_ironing` computes the angle offset from the base direction before `Fill::fill_surface` | absent from the top manifest and `TopSurfaceIroning` | Add a float table and rotate the existing scan generator by `infill_direction + ironing_angle` (base + offset, canonical shape) |
| `ironing_angle_fixed` | coBool | `false` | none | `Layer::make_ironing` sets `fixed_angle`; the fill implementation skips its alternating-direction branch when fixed | absent from the top manifest and `TopSurfaceIroning` | Add a bool table; `true` uses `ironing_angle` as an absolute angle, `false` uses `infill_direction + ironing_angle` |
| `ironing_inset` | coFloat, mm | `0.0` | min `0`, max `100` | `Layer::make_ironing` maps zero to half the nozzle diameter, otherwise uses the configured inset before intersection and fill | absent from the top manifest and generator; current strokes reach the unoffset polygon | Add a float table and inward-offset surface polygons; zero uses `IRONING_LINE_WIDTH / 2` as the in-tree width proxy |
| `ironing_type` | coEnum `IroningType` | `"no ironing"` | `no ironing`, `top`, `topmost`, `solid` | `Layer::make_ironing` gates layers and selects top/bottom/internal solid surfaces; `Fill::fill_surface` receives each selected ironing area | absent from the top manifest; `ironing_enabled` only gates `top_shell_index == Some(0)` | Replace the top bool gate with the four-mode enum and map modes to the available `SliceRegionView` fills |

Canonical angle nuance: upstream's base angle is the solid fill's own direction, which folds in a per-region rotation template. This port has a single global base direction, `infill_direction`, read identically by `RectilinearInfill::from_config` and `GyroidInfill::from_config`; the top module reads the same key, so the ironing angle is relative to the same base the solid fill used. The residual difference — the per-region rotation template — is `DIV-266-B`, a recorded divergence with rationale, not an unmeasured gap.

## Returned to Queue — unimplemented

**None.** All four P14 keys drive a behaviour this packet builds inside `top-surface-ironing`: `ironing_type` selects surfaces, `ironing_angle` and `ironing_angle_fixed` set the scan direction, `ironing_inset` offsets the filled polygon. None is declared without a consumer.

Adjacent keys that are **not** in this packet and stay unimplemented, named so the queue keeps them visible: `ironing_pattern`, `ironing_speed`, `ironing_flow`, `ironing_spacing` (existing top-side defaults and the rectilinear path stand; `ironing_pattern` in particular is a fill-pattern enum and, per the map's Authoring rule 4, would be claim-holder work rather than an enum declared on this module), and the per-filament `filament_ironing_*` family (blocked on the per-filament config model the map records as Tier-D fog).

## Ruled Dead-in-Canonical

**None.** All four keys are read inside the slicing pipeline: canonical `Layer::make_ironing` (`src/libslic3r/Fill/Fill.cpp`) consumes `ironing_type` to gate layers and select surfaces, `ironing_inset` for the effective inset (zero mapping to half the nozzle diameter), and `ironing_angle` / `ironing_angle_fixed` for the fill angle handed to `Fill::fill_surface`. None of the four is GUI-only, `ConfigManipulation.cpp`-only, or in an `IGNORE`/legacy-alias set.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` exact top manifest schema (plus the `infill_direction` co-declaration); `AC-2` four-mode surface selection; `AC-3` base-plus-offset versus fixed angle; `AC-3b` inset geometry; `AC-4` scheduler enum/bounds enforcement; `AC-5` generated documentation.
- **Map gate (b) coverage.** Each of the four P14 keys has at least one AC asserting a behaviour change at a non-default value: `ironing_type` -> AC-2 (`top`, `topmost`, `solid`, all non-default against `no ironing`); `ironing_angle` -> AC-3 (`15.0`); `ironing_angle_fixed` -> AC-3 (`true`, and the emitted direction differs from the non-fixed run); `ironing_inset` -> AC-3b (`1.0`, with the un-inset run asserted to fall outside the offset so the test cannot pass on unchanged geometry). No key's only evidence is a default-path identity, and no AC asserts a CONFIG_BLOCK line.
- Negative: `AC-N1` legacy top boolean cannot enable the canonical top module.
- Cross-packet impact: P15 owns the support-surface-ironing `support_ironing` migration and must not be made to consume `ironing_type`; packet `262a-infill-angle-and-multiline-keys` may make `solid_infill_direction` the authoritative base for solid fill, at which point this module's base read swaps to it (`[FWD]` in `design.md`). No other queued packet claims these four keys.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p top-surface-ironing --test top_surface_ironing_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 manifest schema guard | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2, AC-3, AC-3b, AC-N1 module behavior | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4 real manifest enforcement | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract integrated_parity_top_surface_ironing 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | top integration fixture migration | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_ironing_per_painted_top_color 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | painted top fixture migration | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e slicing_promotion_e2e_dispatch_regression 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | e2e config migration | FACT pass/fail |
| `cargo xtask gen-config-docs` | regenerate the generated reference | FACT exit code |
| `cargo xtask gen-config-docs --check` | AC-5 generated-reference check | FACT exit code |
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
