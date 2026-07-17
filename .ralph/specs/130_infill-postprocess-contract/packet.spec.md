---
status: implemented
packet: 130_infill-postprocess-contract
task_ids:
  - TASK-255
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 130_infill-postprocess-contract

## Goal

Make `Layer::InfillPostProcess` usable as the infill-linker's home: `run_infill_postprocess`
gains a read-only `prior-infill` input mirroring `InfillIR`'s region buckets (ADR-0028 Option
1b), and `perimeter-region-view` gains six fields — the four partitioned fill polygons plus
`tool-index` and `wall-source-region-id` — with `world-layer` bumped 1.0.0 → 2.0.0.

## Scope Boundaries

This packet changes the WIT contract, SDK types/trait, macros glue, host dispatch/marshal
population, and every exhaustive construction/match on `PerimeterRegionView` across the
workspace (~30 files), plus contract tests and a postprocess echo test-guest. It does NOT
implement any linking (packet 133), does not change `LayerStageCommit::InfillPostProcess`
(replace stays, per the full-re-emit contract), and does not change the `InfillIR` struct.

## Prerequisites and Blockers

- Depends on: nothing (parallel-safe with 129; lands second in the serial order).
- Unblocks: `131_per-region-config-delivery` (adjacent WIT churn), `133_infill-linker-module`
  (consumes the whole contract).
- Activation blockers: none — Option 1a/1b, commit semantics, and the field list were all
  resolved in the 2026-07-01 grilling (ADR-0028 §Amendment).

## Acceptance Criteria

- **AC-1. Given** a layer where `Layer::Infill` emitted paths into `InfillIR` regions, **when**
  `run_infill_postprocess` dispatches to a test guest that echoes its `prior-infill` input,
  **then** the echoed per-region counts of `sparse_infill`, `solid_infill`, and `ironing`
  paths equal the committed `InfillIR`'s counts for the same `(object_id, region_id)` keys. | `cargo test -p slicer-runtime --test contract -- infill_postprocess_prior_ir 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a `SliceIR` region with non-empty `sparse_infill_area`, `top_solid_fill`,
  `bottom_solid_fill`, and `bridge_areas`, **when** the host builds the
  `PerimeterRegionView` for `run_infill_postprocess`, **then** all four polygon fields on the
  view equal the `SliceIR` region's partitioned polygons (same counts, same vertex data). | `cargo test -p slicer-runtime --test contract -- infill_postprocess_partitioned_polygons 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a region whose `variant_chain` contains `("material", ToolIndex(2))`,
  **when** the view is built, **then** `tool_index == 2`; **given** a region with no material
  variant whose interned config carries `extensions["extruder"] = 1` (resolved via
  `RegionMapIR::config_for(region_key)` — `extensions` lives on `ResolvedConfig`, not on
  `RegionMapIR`), **then** `tool_index == 1`; **given** neither, **then** `tool_index == 0`. | `cargo test -p slicer-runtime --test contract -- infill_postprocess_tool_index_precedence 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** a virtual variant region without a per-variant `PerimeterIR` entry (the
  `region_partition.rs:123-144` case), **when** the view is built, **then** its
  `wall_source_region_id == Some(<base region_id>)`; **given** a region with its own
  `PerimeterIR` entry, **then** `wall_source_region_id == None`. | `cargo test -p slicer-runtime --test contract -- infill_postprocess_wall_source 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** `PerimeterRegionViewBuilder` in the SDK test-support fixtures, **when** a
  test sets all six new fields, **then** the built view returns them via the matching
  accessors. | `cargo test -p slicer-sdk --features test -- perimeter_region_view_builder 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-6. Given** the canonical WIT sources, **when** greping the world version, **then**
  `world-layer` declares `2.0.0` and `perimeter-region-view` declares all six new fields
  (`sparse-infill-area`, `top-solid-fill`, `bottom-solid-fill`, `bridge-areas`, `tool-index`,
  `wall-source-region-id`). | `rg -c 'sparse-infill-area|top-solid-fill|bottom-solid-fill|bridge-areas|tool-index|wall-source-region-id' crates/slicer-schema/wit/deps/ir-types.wit && rg -q '1\.1\.0' crates/slicer-schema/wit/deps/world-layer/world-layer.wit && echo WIT-OK`

## Negative Test Cases

- **AC-N1. Given** a slice with NO module registered at `Layer::InfillPostProcess`, **when**
  the layer executes, **then** the committed `InfillIR` is byte-identical to the
  post-`Layer::Infill` IR (the per-module loop runs zero iterations, so the
  `InfillPostProcess` wipe/replace commit arm at `layer_executor.rs:1768` is never reached —
  stage loop at `:288`). | `cargo test -p slicer-runtime --test contract -- infill_postprocess_absent_module_preserves_infill 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** the existing contract suite, **when** the six fields default to
  empty/None in fixtures that don't set them, **then** no pre-existing contract test changes
  result. | `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep "^test result"`

## Authoritative Docs

- `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` — load in
  full (~200 lines; §Amendment 2026-07-01 is binding).
- `docs/03_wit_and_manifest.md` — delegate; load only the `world-layer` and view-resource
  sections by rg.
- `CLAUDE.md` §"WIT/Type Changes Checklist" + §"Guest WASM Staleness" — binding ceremony.
- `docs/02_ir_schemas.md` — delegate; `InfillIR` section only (no struct change expected —
  confirm, don't assume).

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` §world-layer / §perimeter-region-view — six new fields +
  `prior-infill` param + 2.0.0 version — `rg -q 'wall-source-region-id' docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md` §run_infill_postprocess — new signature with `prior-infill` input
  and the full-re-emit contract sentence — `rg -q 'prior-infill|prior_infill' docs/05_module_sdk.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
