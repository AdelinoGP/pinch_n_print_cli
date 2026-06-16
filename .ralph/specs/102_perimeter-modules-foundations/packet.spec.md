---
status: draft
packet: 102_perimeter-modules-foundations
task_ids:
  - T-010
  - T-011
  - T-012
  - T-013
  - T-014
  - T-015
  - T-016
  - T-017
  - T-018
  - T-019
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 102_perimeter-modules-foundations

## Goal

Establish shared infrastructure for both perimeter modules: extract duplicated paint/seam/conversion helpers into `slicer-helpers::perimeter_utils`, widen `WallBoundaryType::MaterialBoundary` to carry per-segment transition lists, plumb per-layer config overrides through `run_perimeters`, and propagate `PerimeterOutputBuilder` `Result`s via `?`.

## Scope Boundaries

Touches `slicer-helpers`, `slicer-ir`, `slicer-schema/wit`, both `core-modules/{classic,arachne}-perimeters` lib.rs + manifest, and `slicer-sdk` view/trait surface for the per-layer config plumbing. The packet is purely infrastructural — module wall-emission geometry stays identical to the pre-packet behavior; only the read-paths, helper locations, and IR shape change. Per-vertex flag propagation (Phase 2) and surface-driven wall-count rules (Phase 3) ship as separate downstream packets.

## Prerequisites and Blockers

- Depends on:
  - infill-fill-partition Phase 2.0 — landed (verified: `SlicedRegion.sparse_infill_area` at `crates/slicer-ir/src/slice_ir.rs:1268`, `slice-region-view::sparse-infill-area` accessor at line 41, schema 4.1.0). T-013's schema bump is additive (4.1.0 → 4.2.0).
- Unblocks:
  - Packet `104_perimeter-propagation-and-surface-rules` (Phases 2 + 3 of M1) — depends on the shared utils crate and per-layer config plumbing this packet establishes.
  - Packet `103_slicer-helpers-polygon-ops` (Phase 4) is independent of this packet and may proceed in parallel.
- Activation blockers: none — all decisions from grilling session closed (D-1 through D-15).

## Acceptance Criteria

- **AC-1. Given** a fresh workspace, **when** `cargo check --workspace --all-targets` runs after the new `slicer-helpers::perimeter_utils` module is created, **then** the module compiles and exports the symbols `build_outer_wall_flags`, `has_adjacent_material_change`, `find_adjacent_tool`, `extract_tool_index`, `default_feature_flags`, `expolygon_to_path3d`, `generate_seam_candidates`, and the constant `BASE_SPEED`. | `cargo check -p slicer-helpers --all-targets 2>&1 | tee target/test-output.log && rg -q 'pub (fn|const) (build_outer_wall_flags|has_adjacent_material_change|find_adjacent_tool|extract_tool_index|default_feature_flags|expolygon_to_path3d|generate_seam_candidates|BASE_SPEED)' crates/slicer-helpers/src/perimeter_utils.rs`
- **AC-2. Given** both `classic-perimeters/src/lib.rs` and `arachne-perimeters/src/lib.rs`, **when** the migration to consume `slicer-helpers::perimeter_utils` completes, **then** neither file contains its own definition of any of `build_outer_wall_flags`, `has_adjacent_material_change`, `find_adjacent_tool`, `extract_tool_index`, `default_feature_flags`, `expolygon_to_path3d`, or `generate_seam_candidates` (each must be a `use` import, not a local `fn`). | `! rg -q '^fn (build_outer_wall_flags|has_adjacent_material_change|find_adjacent_tool|extract_tool_index|default_feature_flags|expolygon_to_path3d|generate_seam_candidates)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs`
- **AC-3. Given** the widened IR, **when** `WallBoundaryType::MaterialBoundary` is constructed for a polygon with three transitions across four distinct tool indices `[1, 2, 3, 1]`, **then** the variant carries a `Vec<MaterialBoundarySegment>` of length 3 in clockwise order, each segment naming `near_tool` + `far_tool` for that transition (not just the first one), and `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps to `4.2.0`. | `cargo test -p slicer-ir --test material_boundary_widening_tdd -- --nocapture 2>&1 | tee target/test-output.log && rg -q 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{ major: 4, minor: 2, patch: 0' crates/slicer-ir/src/slice_ir.rs`
- **AC-4. Given** a `PerimeterOutputBuilder` mock that returns `Err` on a specific call, **when** either perimeter module invokes a method that returns `Result`, **then** the error propagates via `?` to the module's `ModuleError` return rather than being silently discarded (no remaining `let _ = output\.` patterns in either module). | `! rg -q 'let _ = output\.' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs`
- **AC-5. Given** a `LayerOverrides` block that sets `wall_count = 5` for layer index 5 with a base `wall_count = 2`, **when** `run_perimeters` is invoked for layer 0 and layer 5, **then** layer 0 emits 2 walls and layer 5 emits 5 walls (config re-resolves per-layer via the `_config: &ConfigView` parameter, not just once at `on_print_start`). | `cargo test -p slicer-runtime --test contract per_layer_config_override_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the manifests at `modules/core-modules/{classic,arachne}-perimeters/*.toml`, **when** the manifest default for any of `wall_count`, `outer_wall_speed`, `inner_wall_speed` is compared to the Rust code's `match`-arm fallback in the same module's `on_print_start`, **then** the values match (single source of truth — the code fallback equals the manifest default). | `cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd -- --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a `PerimeterOutputBuilder` that is constructed at capacity and rejects further `push_wall_loop` calls with `Err("builder at capacity".into())`, **when** `run_perimeters` is invoked with a non-empty region, **then** the module returns `Err(ModuleError::…)` whose message contains `"builder at capacity"` (not silently `Ok(())`). | `cargo test -p slicer-runtime --test contract perimeter_builder_capacity_error_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a 3-tool polygon `[T1, T1, T2, T2, T3, T3]` with three transitions (T1→T2, T2→T3, T3→T1), **when** `build_outer_wall_flags` constructs the `MaterialBoundary`, **then** the resulting `Vec<MaterialBoundarySegment>` has length 3 and contains all three transitions (NOT length 1 with just the first transition — which was the pre-T-013 lossy behavior). | `cargo test -p slicer-helpers --test perimeter_utils_three_tool_boundary_tdd -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-ir --test material_boundary_widening_tdd && cargo test -p slicer-helpers --test perimeter_utils_three_tool_boundary_tdd && cargo test -p slicer-runtime --test contract per_layer_config_override_tdd perimeter_builder_capacity_error_tdd && cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 1 tasks T-010 through T-019; ADR-0011 wall-sequencing context. Range-read §"Phase 1 — Cross-cutting foundations" and §"Open decision points" (D-13 carrier mechanism).
- `docs/02_ir_schemas.md` — `WallBoundaryType` definition, schema-version contract. Delegate SUMMARY for the WallBoundaryType section.
- `docs/03_wit_and_manifest.md` — WIT type-identity rules for the `SliceIR` accessors that change with the schema bump. Range-read §"WIT/Type Changes Checklist".
- `docs/05_module_sdk.md` — `LayerModule` trait, `ConfigView`, `PerimeterOutputBuilder` contract. Delegate SUMMARY for the failure-mode contract section.

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/02_ir_schemas.md` §"WallBoundaryType" — widen the variant to carry `Vec<MaterialBoundarySegment>` and document the migration adapter — `rg -q 'MaterialBoundarySegment' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"Schema Versioning" — record the 4.1.0 → 4.2.0 bump rationale — `rg -q '4\.2\.0.*MaterialBoundary' docs/02_ir_schemas.md`
- `docs/05_module_sdk.md` §"PerimeterOutputBuilder failure modes" — new section documenting the failure-mode contract (capacity, contract violation, `Result` propagation expectation) — `rg -q 'PerimeterOutputBuilder failure modes' docs/05_module_sdk.md`
- `docs/15_config_keys_reference.md` — reconcile the manifest-vs-code defaults for `wall_count`, `outer_wall_speed`, `inner_wall_speed` — `rg -q 'wall_count.*default: 2' docs/15_config_keys_reference.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — only as the historical reference for `BASE_SPEED` (≈50 mm/s) and the per-region config-read pattern. No new parity behavior is added in this packet; the delegation contract is only for confirming the shared-utils constants and signatures align with the canonical OrcaSlicer surface area.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
