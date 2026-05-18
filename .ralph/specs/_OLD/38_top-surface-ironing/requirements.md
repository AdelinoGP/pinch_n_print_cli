# Requirements: top-surface-ironing

## Packet Metadata

- Grouped task IDs:
  - `TASK-168` (NEW)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Today the live path lacks any ironing pass over top surfaces. Orca emits a low-flow zigzag pass over the topmost layer's `TopSolidInfill` polygons to smooth the visible top surface. Without ironing, the printed top surface shows extrusion lines and inter-line gaps.

This packet ships a new core module `top-surface-ironing` that runs as `Layer::InfillPostProcess` and emits the ironing pass. The module relies on packet 12-rev1's `is_top_surface` flag and packet 35's `top_solid_layers` plumbing to identify *only the topmost* top-solid layer (not interior layers of the top-solid stack). It uses the existing `support-surface-ironing` module pattern as a template.

## In Scope

- New core module directory: `modules/core-modules/top-surface-ironing/{Cargo.toml, manifest.toml, src/lib.rs}` plus optional `wit-guest` subdirectory if the existing build pattern requires it.
- Manifest declarations:
  - `stage = "Layer::InfillPostProcess"`
  - `[ir-access].reads = ["InfillIR.regions"]`
  - `[ir-access].writes = ["InfillIR.regions"]`
  - read-then-write establishes deterministic ordering after the fill module per `docs/04 §Composable Multi-Writer Patterns`
- Module logic in `src/lib.rs`:
  - filter `solid_infill` paths to `role == TopSolidInfill`
  - detect topmost-layer-of-stack via the `is_top_surface` flag from `SliceRegionView` plus the `top_solid_layers` config (from `RegionMapIR`/config-view): emit ironing only when current layer is the highest layer of the top-solid stack for this region
  - generate zigzag at `ironing_spacing` mm (default 0.2 mm)
  - assign `ironing_flow` × extrusion (default 0.15)
  - tag paths `ExtrusionRole::Ironing`
  - preserve all input paths; ironing paths are appended, never replacing
- Config keys (declared in module `manifest.toml` and central config schema):
  - `ironing: bool` (default `false`)
  - `ironing_speed: f64` (mm/s; default `15.0`)
  - `ironing_flow: f64` (multiplier; default `0.15`)
  - `ironing_spacing: f64` (mm; default `0.2`)
  - `ironing_pattern: String` (default `"rectilinear"`; only `rectilinear` is supported in v1)
- Confirm `ExtrusionRole::Ironing` → `;TYPE:Ironing` mapping in `crates/slicer-host/src/gcode_emit.rs` (FACT in Step 0; add the entry if missing).
- New TDD `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs`.
- New host E2E test `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence`.
- WASM build via `./modules/core-modules/build-core-modules.sh`.

## Out of Scope

- Support-surface ironing (already exists in `modules/core-modules/support-surface-ironing/`).
- Non-rectilinear ironing patterns (concentric, etc.) — separate packet if requested.
- Ironing-specific cooling / temperature overrides (separate config concern).
- Bottom-surface or generic solid-infill ironing.
- Per-region ironing config overrides beyond what `RegionMapIR` already supports.
- Variable ironing spacing across the top surface.

## Authoritative Docs

- `docs/05_module_sdk.md` — `#[slicer_module]` macro and `Layer::InfillPostProcess` patterns. Read directly.
- `docs/02_ir_schemas.md` — `InfillIR.regions[*].solid_infill` and the `ExtrusionRole::Ironing` enum. Read directly; one section.
- `docs/04_host_scheduler.md` — § "Composable Multi-Writer Patterns". Delegate SUMMARY.
- `docs/03_wit_and_manifest.md` — `[ir-access]` declaration rules. Read directly.
- `docs/09_progress_events.md` — for any progress events the module emits (likely none, but FACT to confirm conventions).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_ironing` (~line 1530). Delegate SUMMARY ≤ 200 words.
- `OrcaSlicerDocumented/src/libslic3r/Layer.hpp` — `LayerRegion::make_ironing` declaration. Delegate FACT.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::ironing()`. FACT for invocation order.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md`. Covers (a) ironing path emitted at topmost layer with reduced flow, (b) non-topmost layer suppressed, (c) interior top-solid layer suppressed, (d) `ironing: false` produces nothing, (e) `ironing_spacing` controls stroke count, (f) Benchy E2E shows `;TYPE:Ironing` block.
- Negative cases: bottom-only layer produces nothing; zero `ironing_flow` is a config error.
- Measurable outcomes:
  - `cargo test --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
- Cross-packet impact: none.

## Verification Commands

- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path:
  - `crates/slicer-host/src/gcode_emit.rs` — read only the `ExtrusionRole::Ironing` mapping (FACT-driven small range).
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` — delegate SUMMARY; never load.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads:
  - `crates/slicer-host/src/dispatch.rs` — out of scope unless `Layer::InfillPostProcess` dispatch needs adjustment (verify via FACT first).
  - All other core modules — read only the existing `support-surface-ironing` module as a template.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail.
  - OrcaSlicer SUMMARY → ≤ 200 words.
  - Reference-template module summary → SUMMARY ≤ 200 words for `support-surface-ironing` skeleton.
