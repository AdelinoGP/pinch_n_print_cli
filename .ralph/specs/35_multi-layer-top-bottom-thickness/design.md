# Design: multi-layer-top-bottom-thickness

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/layer_slice.rs` — `execute_layer_slice` and the `classify_region_surfaces` helper landed by packet 12-rev1. This packet widens the Z window in the helper.
  - `crates/slicer-host/src/layer_executor.rs:295-310` — production caller. Reads `blackboard.region_map()` and forwards.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` (from 12-rev1) — must remain green; this packet adds a sibling `multi_layer_thickness_tdd.rs`.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — extend with `benchy_multi_layer_top_bottom_evidence`.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells()`. We do not port the inter-layer propagation algorithm; we achieve equivalent semantics by widening the Z window in the facet-projection classifier.

## Architecture Constraints

- **No new fine-layer-height slicing pass** (same constraint inherited from 12-rev1).
- **No change to per-layer parallel execution.**
- **Use `RegionMapIR` from blackboard** as the per-region resolved-config source. `RegionMapIR` is produced by host-built-in `PrePass::RegionMapping`; immutable post-prepass.
- **Per-region config keys reuse existing schema where possible.** Confirm in Step 0 whether `top_solid_layers` and `bottom_solid_layers` are already declared in the central config schema (`docs/03_wit_and_manifest.md`); add them if absent.
- **Defaults match Orca**: `top_solid_layers = 3`, `bottom_solid_layers = 3`. Verified via OrcaSlicer FACT delegation.

## Code Change Surface

- Selected approach:
  - Extend `classify_region_surfaces` to take `top_solid_layers: u32` and `bottom_solid_layers: u32` (resolved at the call site from `RegionMapIR`). The function computes the window by walking `LayerPlanIR.global_layers` from the current index. Window truncation at object/global extent yields `f32::INFINITY` (top) or `f32::NEG_INFINITY` (bottom), preserving the existing single-layer semantics as the `N=1` case.
  - Extend `execute_layer_slice` signature with `region_map: Option<&RegionMapIR>` and (new) `layer_plan: Option<&LayerPlanIR>` so the helper can walk multi-layer windows. Caller already has both available.
  - Inside `execute_layer_slice`'s region loop: resolve `(top_solid_layers, bottom_solid_layers)` per `(layer_idx, object_id, region_id)` from `region_map.entries[*].config`; use Orca defaults when missing.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/layer_slice.rs` — extend `execute_layer_slice` signature; widen window in `classify_region_surfaces`; resolve config per region.
  - `crates/slicer-host/src/layer_executor.rs:295-310` — pass `blackboard.region_map()` and `blackboard.layer_plan()` references.
  - `crates/slicer-host/tests/multi_layer_thickness_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append new `benchy_multi_layer_top_bottom_evidence` test.
  - mechanical: existing `execute_layer_slice` test callers in `crates/slicer-host/tests/layer_slice_tdd.rs` get a `None` for the new `region_map` parameter (and `layer_plan` if added).
- Rejected alternatives that were considered and why they were not chosen:
  - **Compute the window from `N × layer_height`** — rejected: incorrect for variable layer height, which is supported by `LayerPlanIR.global_layers[*].z`. Walking the actual `z` array is correct.
  - **Pass `top_solid_layers` / `bottom_solid_layers` as direct function arguments to `execute_layer_slice`** — rejected: per-region resolution belongs inside the function, not at every caller. `RegionMapIR` is the right abstraction.
  - **Bake defaults into `mesh_analysis.rs`** — rejected: defaults are config concerns, not classification concerns; mesh-analysis stays config-free.

## Files in Scope (read + edit)

- `crates/slicer-host/src/layer_slice.rs` — primary edit; extend signatures and window logic.
- `crates/slicer-host/src/layer_executor.rs` — secondary edit; thread the additional blackboard refs (lines `280-360`).
- `crates/slicer-host/tests/multi_layer_thickness_tdd.rs` (NEW).
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append one new test.

## Read-Only Context

- `crates/slicer-host/src/blackboard.rs` — read lines `192-220` only — purpose: confirm `region_map()` accessor pattern.
- `crates/slicer-host/src/region_mapping.rs` — read only the public `RegionMapIR` access surface and `entries[*].config` lookup helpers (range-read; do NOT load whole file).
- `crates/slicer-ir/src/slice_ir.rs` — read lines `680-740` (`LayerPlanIR.global_layers` and `RegionPlan` definitions).
- `docs/02_ir_schemas.md` — `RegionMapIR` section.
- `docs/04_host_scheduler.md` — § "RegionMapIR Compilation"; § "Per-Layer Execution"; § "Blackboard Structure" (delegate SUMMARY).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks.
- `target/`, `Cargo.lock` — never load.
- `crates/slicer-host/src/dispatch.rs` — out of scope.
- `crates/slicer-host/src/prepass.rs` — out of scope.
- `wit/` — no WIT changes.
- `crates/slicer-sdk/` — no SDK changes.
- `modules/core-modules/` — no module changes.

## Expected Sub-Agent Dispatches

- "Are `top_solid_layers` and `bottom_solid_layers` already declared in the central config schema (search `crates/slicer-ir/src/` and `crates/slicer-host/src/` for the keys); return FACT yes/no with file:line evidence" — purpose: validate Step 0.
- "Run `cargo test -p slicer-host --test multi_layer_thickness_tdd`; return FACT pass/fail with the failing test list" — purpose: validate Steps 2–3.
- "Run `cargo test -p slicer-host --test external_surface_classification_tdd`; return FACT pass/fail" — purpose: confirm 12-rev1 regression-safe.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence`; return FACT pass/fail" — purpose: validate Step 4.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp::discover_horizontal_shells` defaults for `top_solid_layers` and `bottom_solid_layers`; return FACT (numeric defaults only)" — purpose: confirm Orca-default values for Step 1.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `RegionMapIR.entries[*].config` — read-only consumer; no schema change.
  - Config schema may need `top_solid_layers` and `bottom_solid_layers` keys added (Step 0 FACT decides).
  - `SliceIR.schema_version` — no further bump (12-rev1 already moved to `1.1.0`; this packet's behavior is value-driven, not schema-driven).
- WIT boundary considerations: none.
- Determinism or scheduler constraints:
  - `RegionMapIR.entries[*].config` is `Arc`-shared from PrePass; read-only across rayon workers; deterministic.

## Locked Assumptions and Invariants

- `LayerPlanIR.global_layers[*].z` is sorted ascending and represents the actual sliced layer Z values (variable-height friendly).
- `RegionMapIR.entries` indexed by `(global_layer_index, object_id, region_id)` — same key shape used by 12-rev1 lookups.
- `top_solid_layers = 3`, `bottom_solid_layers = 3` are Orca defaults (confirm via FACT in Step 0).
- An object's "topmost active layer" is implicit — when the window walk truncates at the global layer count, the upper-Z bound becomes `f32::INFINITY`, which makes any TopSurface facet above the layer fall inside the window. This correctly captures objects that end at the global slice ceiling.

## Risks and Tradeoffs

- **`top_solid_layers = 0` (user explicitly disables)**: requires an early-return in the helper. Tested in negative case.
- **Window walks across object boundaries**: if two objects are stacked and `LayerPlanIR.global_layers[i+1]` is the first layer of object B (not the next layer of object A), the helper still uses `global_layers[i+1].z` as the upper bound. This is correct for the FACET-projection algorithm because the facet itself belongs to a specific object — `classify_region_surfaces` already filters facets by `region.object_id`. No cross-object contamination.
- **Variable layer height + tall windows**: walking `global_layers` always uses the actual stored Z, so variable height is honored automatically.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3: signature extension + window logic).
- Highest-risk dispatch: `cargo test --workspace` after Step 4 — pass FACT-only return.

## Open Questions

- Step 0 dispatch resolves: are `top_solid_layers` and `bottom_solid_layers` already in the config schema? If yes, no schema work; if no, add them (still S cost).
- Step 0 dispatch resolves: confirm Orca defaults are exactly `3` (not `4` or `5`). If a different default, update the constants.

Both open questions are answered by Step-0 FACT dispatches; neither blocks activation conceptually — they only affect Step 1's exact line count.
