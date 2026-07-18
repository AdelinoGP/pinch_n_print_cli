# Requirements: 102_perimeter-modules-foundations

## Packet Metadata

- Grouped task IDs:
  - `T-010` — Create `slicer-perimeter-utils` (shared crate or `slicer-core` submodule)
  - `T-011` — Migrate `classic-perimeters` to consume shared utils
  - `T-012` — Migrate `arachne-perimeters` to consume shared utils
  - `T-013` — Widen `WallBoundaryType::MaterialBoundary` to `Vec<MaterialBoundarySegment>`
  - `T-014` — Update `build_outer_wall_flags` to emit full transition list
  - `T-015` — Plumb `LayerOverrides` per-layer config through `_config: &ConfigView`
  - `T-016` — Replace `let _ = output.…` with `?` propagation in both modules
  - `T-017` — Document `PerimeterOutputBuilder` failure modes + negative-path TDD
  - `T-018` — Reconcile manifest defaults with code fallbacks for `wall_count`, `outer_wall_speed`, `inner_wall_speed`
  - `T-019` — Read `_paint: &PaintRegionLayerView` or document why intentionally unread
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The two perimeter modules (`classic-perimeters` and `arachne-perimeters`) share ≈170 LOC of duplicated paint-propagation, seam-candidate, and point-conversion helpers. That duplication is a maintenance hazard — every future per-vertex flag (T-020 bridge, T-021/T-022 inner-wall material boundary, T-074b/c/d non-planar emission) would otherwise have to land twice with risk of drift. The audit also surfaced four discrete defects that the modules currently carry: `WallBoundaryType::MaterialBoundary { adjacent_tool: u32 }` records only the first transition on a multi-tool polygon and silently drops the rest; `let _ = output.…` swallows `PerimeterOutputBuilder` `Result`s so capacity / contract violations become invisible; the `_config` and `_layer_index` parameters in `run_perimeters` are unread, making the host's `LayerOverrides` mechanism inoperative for these modules; and manifest-vs-code defaults disagree on `wall_count` (3 vs 2), `outer_wall_speed` (30.0 vs 50.0), and `inner_wall_speed` (45.0 vs 50.0) — when manifest validation is bypassed the silent divergence is a latent footgun.

This packet closes the four defects together with the duplication extraction because they all share the same file surface; bundling them avoids two rounds of cherry-picking through the same `lib.rs` files. None of the four defects can be fixed in isolation without first paying the merge cost on the duplicated helpers.

## In Scope

- New module `slicer-core::perimeter_utils` exporting the seven helper functions and one constant currently duplicated between the two perimeter modules.
- `WallBoundaryType` widening: replace `MaterialBoundary { adjacent_tool: u32 }` with `MaterialBoundary { segments: Vec<MaterialBoundarySegment> }` where `MaterialBoundarySegment { point_range: std::ops::Range<u32>, near_tool: Option<u32>, far_tool: Option<u32> }`. Bump `CURRENT_SLICE_IR_SCHEMA_VERSION` from `4.1.0` to `4.2.0` (additive — `#[serde(default)]` migration adapter for the old single-tool shape is acceptable).
- WIT-side mirror in `crates/slicer-schema/wit/deps/ir-types.wit` for the new `material-boundary-segment` record + `wall-boundary-type` variant.
- Both `lib.rs` files migrate to the shared utils, propagate `Result`s via `?`, read per-layer overrides via `_config`, and either consume `_paint` or document its intentional disuse.
- Manifest reconcile: align the `[config.schema]` defaults in `classic-perimeters.toml` and `arachne-perimeters.toml` with the Rust code's `match`-arm fallback values. Pick one source of truth (manifest) and align code to it.
- `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`, `docs/15_config_keys_reference.md` updates per the Doc Impact Statement.
- One negative-path TDD covering builder-Result propagation.
- One TDD covering 3-tool polygon `MaterialBoundary` segment-list shape.
- One TDD covering per-layer config override.
- One TDD covering manifest-vs-code default reconcile.

## Out of Scope

- Per-vertex `is_bridge`, `overhang_quartile`, `flow_factor` propagation — Phase 2 (packet 104).
- Inner-wall `tool_index` propagation (drop hardcoded `Interior`) — Phase 2 (packet 104, depends on T-013 from this packet).
- Wall-sequence reordering (`OuterInner` / `InnerOuter` / `InnerOuterInner`) — Phase 5 (later packet).
- Polygon-op primitives (`medial_axis`, `offset2_ex`, hole-tree, `keep_largest_contour_only`, ray ops) — Phase 4 (packet 103, fully parallel).
- ~~Rename of `arachne-perimeters` → `variable-width-perimeters`~~ — **cancelled** (D-110-DROP-VARIABLE-WIDTH): fake-Arachne module is deleted under P108; `variable-width-perimeters` never ships.
- `infill-fill-partition` host-side hook — already landed in `slicer-runtime`.
- Any change to wall-emission geometry. This packet is purely infrastructural; same input regions → same wall loops.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~600 lines | Range-read §"Phase 1 — Cross-cutting foundations" (~30 lines) and §"Open decision points" rows D-13 / D-14. Delegate the rest. |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for the `WallBoundaryType` definition and the schema-version contract. Read directly only the WallBoundaryType lines once located. |
| `docs/03_wit_and_manifest.md` | ~400 lines | Range-read §"WIT/Type Changes Checklist" only (≤ 30 lines). |
| `docs/05_module_sdk.md` | ~500 lines | Delegate SUMMARY for the `LayerModule` trait + `ConfigView` + `PerimeterOutputBuilder` sections. |
| `docs/08_coordinate_system.md` | ~250 lines | Read directly only if a per-vertex conversion (`expolygon_to_path3d`) issue surfaces. Most likely unused. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — confirm `BASE_SPEED` semantic (≈50 mm/s outer-wall reference) and the per-region config-read pattern. Delegate a `FACT` on the constant name and value; do not read the file.

## Acceptance Summary

- Positive cases: `AC-1` (shared crate exports), `AC-2` (no duplicate definitions in modules), `AC-3` (MaterialBoundary widened + schema bump), `AC-4` (no `let _ = output\.` remaining), `AC-5` (per-layer config override), `AC-6` (manifest-vs-code defaults reconciled). Refinement: AC-3's schema bump is **additive** — the migration adapter must allow `serde::Deserialize` of the old single-tool shape (`{ adjacent_tool: u32 }`) into the new shape (single-element `segments` Vec with `near_tool: Some(_)`, `far_tool: Some(adjacent_tool)`).
- Negative cases: `AC-N1` (builder capacity error surfaces as `ModuleError`), `AC-N2` (3-tool polygon emits 3 transitions, not 1).
- Cross-packet impact: unblocks packet `104_perimeter-propagation-and-surface-rules` (Phase 2 inner-wall material boundary work consumes the widened `MaterialBoundary`). Independent of packet `103_slicer-helpers-polygon-ops`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after IR + WIT changes | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace clippy gate after migration | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --test perimeter_utils_three_tool_boundary_tdd` | Confirms AC-3 + AC-N2 (multi-segment MaterialBoundary) | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-ir --test material_boundary_widening_tdd` | Confirms AC-3 schema-version bump + Vec representation | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract per_layer_config_override_tdd` | Confirms AC-5 per-layer overrides | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract perimeter_builder_capacity_error_tdd` | Confirms AC-N1 builder error propagation | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd` | Confirms AC-6 manifest vs code defaults | FACT pass/fail |
| `cargo xtask build-guests --check` | Confirms WIT change is reflected in guest WASM after rebuild | FACT — STALE list ≤ 5 lines |
| `! rg -q 'let _ = output\.' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs` | Confirms AC-4 (no swallowed Results remaining) | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: no step may regress the existing `boundary_paint_tdd.rs` tests in either perimeter module (they're the canonical paint-propagation regression tests). Each step that migrates helpers must re-run these as a falsifying check before moving to the next step.
- Step ordering rationale: extract shared utils first (Step 1) because every subsequent step modifies one or both module `lib.rs` files; doing the extraction last would force re-doing the consumer migrations. IR widening (Step 2) before per-layer config (Step 3) only because the IR change's failure surface is narrower and faster to falsify.
- Shared scratch state: none.

## Context Discipline Notes

- `crates/slicer-ir/src/slice_ir.rs` is ~1700 lines. The implementer MUST range-read it: use `rg -n 'WallBoundaryType|MaterialBoundary|CURRENT_SLICE_IR_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs` first, then `Read` with `offset`/`limit` ±40 lines around each hit. Loading the whole file is forbidden.
- `crates/slicer-schema/wit/deps/ir-types.wit` — load in full only if < 200 lines; otherwise delegate a SUMMARY of the `wall-boundary-type` variant and adjacent records.
- Both perimeter modules' `lib.rs` files are 400–700 lines. The implementer may load each once (it's the change target) but should not load the entire file during read-only fact-checks — range-read instead.
- Likely temptation read: `modules/core-modules/seam-placer/src/lib.rs` (curious about how it consumes the change). Skip — ADR-0011 settled the wall-sequence ownership; seam-placer is not touched by this packet.
- Sub-agent return-format for the heaviest dispatch: `WallBoundaryType` widening parity check across consumers must return `LOCATIONS` (≤ 20 lines), not `SNIPPETS` — the implementer cares only about call-site count and file paths.
