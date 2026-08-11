# Requirements: raft-geometry

## Packet Metadata

- Grouped task IDs: `TASK-324`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`SupportPlanIR.raft_plan` has no geometry synthesizer. ADR-0009 requires a Layer::Infill synthesizer that populates `SlicedRegion.raft_fill` and lets a `claim:raft-fill` holder render `SupportIR.raft_paths`. The prior packet draft also left the scheduled `GlobalLayer.index: u32` and the SDK `LayerModule::run_infill(_layer_index: u32)` contract unsigned, so negative selectors and WIT `layer-idx: s32` cannot reach the module safely.

## In Scope

- Add `modules/core-modules/raft-default/` with id `com.core.raft-default`, stage `Layer::Infill`, `claim:raft-fill`, and synthesizer-only implementation.
- Read `SupportPlanIR.raft_plan`, `SliceIR.regions[].polygons`, and `LayerPlanIR.global_layers[].z`; populate `SlicedRegion.raft_fill` for exactly `raft_layers` negative prefix indices.
- Add `claim:raft-fill` handling to the existing Layer::Infill rectilinear module without duplicating scan-line algorithms.
- Migrate `GlobalLayer.index`, `ObjectLayerRef.local_layer_index`, `ObjectLayerRef.global_layer_index`, `SliceIR.global_layer_index`, and `SupportIR.global_layer_index` to `i32`.
- Own every struct literal, conversion, fixture, and assertion that compiles against those fields, including the known `u32::MAX` support-IR assertion only where it is a migrated field.
- Change `LayerModule::run_infill` to `i32`; change `crates/slicer-macros` glue to pass the WIT `i32` unchanged; migrate host/runtime infill boundary values, SDK guest implementations, macro tests, and affected call sites.
- Keep WIT `type layer-idx = s32` unchanged and preserve unrelated `u32` fields, including `SupportGeometryKey.global_support_layer_index` and `SupportGeometryViewEntry.global_support_layer_index`, whose `u32::MAX` sentinel indexes model-resolution support geometry rather than the visual-debug schedule.
- Carry negative schedule indices through runtime capture and visual-debug resolution without `as u32` conversion; use typed `raft_paths` capture as the decisive gate if PNG rendering remains unsupported for negative selectors.
- Add the workspace member and visual-debug request fixtures, and update the architecture, IR schema, and manifest/WIT documentation sections.

## Out of Scope

- A Layer::Support raft module, `raft-generator` id, or `claim:raft-generator` claim.
- Changes to `RaftPlan` configuration, support planner propagation, support fallback/interface generation, or final G-code role ordering.
- New pattern math or Orca numerical parity.
- Unrelated model-only keys and views (`RegionKey`, seam-plan fields, region-segmentation fields), scheduler budget indices, finalization fields, and support-geometry sentinel fields unless a concrete migrated-field conversion requires an edit.

## Authoritative Docs

- `docs/adr/0009-raft-as-layer-infill-role.md` - direct; ADR-0009 decision.
- `docs/specs/support-generation-remediation-plan.md` - direct; queue and scope.
- `docs/specs/support-generation-defect-verified-findings.md` - direct bounded; evidence.
- `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/08_coordinate_system.md` - delegated sections.

## Acceptance Summary

Reference criteria only from `packet.spec.md`: positive `AC-1` through `AC-6`; negative `AC-N1`. Cross-packet impact: consumes TASK-322's `RaftPlan`; exposes `com.core.raft-default`, `claim:raft-fill`, signed schedule indices, and the signed Layer::Infill SDK contract to TASK-327.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p raft-default --test raft_geometry_tdd --all-targets` | Carrier count, clipping, determinism, and no-op behavior | FACT pass/fail |
| `cargo test -p slicer-ir --all-targets` | Signed IR definitions, literals, sentinel assertion fallout | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test wit_boundary_tdd --all-targets` | WIT `s32` boundary and infill call path | FACT pass/fail |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-raft.json --output target/vd-raft --overwrite` | Typed capture and conditional PNG limitation | FACT pass/fail; bounded log on unsupported PNG |
| `cargo check --workspace --all-targets` | Workspace, macro, host, runtime, and module integration | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

The signed scheduled-layer migration lands before module integration and owns the complete inventory from `rg -l 'GlobalLayer\s*\{|ObjectLayerRef\s*\{|LayerPlanIR\s*\{|SliceIR\s*\{|SupportIR\s*\{|global_layer_index:|global_support_layer_index:|u32::MAX' crates modules`. The infill contract migration separately inventories `run_infill`, WIT call boundaries, macro glue, SDK implementations, guests, and tests. Prefix indices remain negative; model indices remain non-negative; support-geometry `u32::MAX` remains unchanged.

## Context Discipline Notes

Delegate large architecture/schema reads and all cargo commands. Never load generated bindings, `target/`, or lockfiles. Keep the migration inventory bounded to scheduled IR fields, infill contract symbols, their literal/conversion/assertion sites, and the explicitly preserved sentinel fields.
