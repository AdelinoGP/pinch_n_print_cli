# Design: 139_lightning-layer-generator

## Controlling Code Paths

- Primary code path: new `crates/slicer-core/src/algos/lightning/layer.rs` and
  `generator.rs` building on the 138 primitives; the 137 skeleton
  `generate_lightning_trees` in `algos/lightning/mod.rs` becomes the real driver (per
  object: build generator over the committed sparse outlines top-down, then
  `convert_to_lines` per layer into `LightningTreeIR` segments).
- Input access: the producer reads the committed `SliceIR` sparse-infill outlines the
  same way the support-geometry producer reads its whole-print inputs (LOCATIONS-dispatch
  the pattern, then ranged reads).
- Neighboring tests or fixtures: `crates/slicer-core` lightning test home (138) gains
  the generator tests; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`
  (137) gains the commits-real-trees case.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The two-pass structure is load-bearing (ADR-0029): outlines pass THEN growth pass,
  both top-down over all layers — do not fuse or reorder the passes.
- The 137 skip promise stays: no lightning holder → no generator construction at all.
- Deterministic output (AC-4) — inherits 138's no-hash-iteration rule; the per-layer
  segment ordering frozen at close is 140's input contract.

## Code Change Surface

- Selected approach: faithful orchestration port. The `Generator` equivalent takes
  per-object inputs (sparse outlines per layer + the density-coupled resolution
  constants — parameterized per the 138 decision) and runs
  `generate_initial_internal_overhangs` then `generate_trees`; `trees_for_layer` →
  `convert_to_lines` produces the per-layer 2-point segments the producer stores.
- Exact changes: two new files + `mod.rs` wiring; the producer body in `mod.rs`
  replaces the 137 skeleton's `// 139 wiring point` comment with the real driver; tests
  in the 138 test home + extension of the 137 executor test.
- **DEVIATION-CLOSURE additions (D-137-LIGHTNING-PER-OBJECT-COLLAPSE):**
  - `LightningTreeEntry` (`crates/slicer-ir/src/slice_ir.rs:1215`) gains a
    `region_id: RegionId` field (a `u64` type alias at `slice_ir.rs:36`; the
    precedent at `SupportPlanEntry.region_id: RegionId` at `:1129`) between
    `global_layer_index` and `tree_edge_segments`.
    Precedent: `SupportPlanEntry.region_id` at `slice_ir.rs:1129`. The
    `Default` impl stays derive-driven (no manual override needed — `u64::default()`
    is `0`, the same default the existing `SupportPlanEntry` uses for region_id).
  - `crates/slicer-wasm-host/src/dispatch.rs:1383` updates the per-region keying:
    the line `let wildcard_region = String::from("*");` is replaced by
    `let key = (entry.object_id.clone(), entry.region_id.to_string());` and the
    `data.lightning_tree_segments.entry(key).or_default();` call mirrors
    `data.support_plan_segments.entry(key).or_default();` at `dispatch.rs:1353`.
  - `crates/slicer-sdk/src/traits.rs:195-199` updates
    `lightning_tree_segments_for(object_id: &str, region_id: u64)` (no longer
    `_region_id`) — the `region_id` argument is now part of the filter:
    `entry.global_layer_index == self.layer_index as i32 && entry.object_id == object_id && entry.region_id == region_id`.
  - The `Default` derive on `LightningTreeEntry` is preserved (no manual impl needed
    for the new `u64` field — `u64::default()` is the `0` value that
    `SupportPlanEntry.region_id` already uses for the single-region-default case).
    A 137-era test in
    `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`
    that passed a hardcoded `region_id = 0` is preserved as the
    `single_region_default` case; a new
    `lightning_tree_per_region_roundtrip_tdd.rs` covers the multi-region case.
- Rejected alternatives: (a) committing tree topology and converting to lines in the
  module — rejected: ADR-0029's compact-IR note and the module-sampler contract; (b)
  lazy per-layer generation — rejected in 137 already; (c) parallelizing the growth
  pass — rejected for v1: determinism first, profile later; (d) per-region skip
  predicate iterating the print's regions — rejected by the packet 137 review's
  blast-radius research (no per-region `ResolvedConfig` exists; the predicate is
  print-wide, intentional).

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/lightning/layer.rs` (new).
- `crates/slicer-core/src/algos/lightning/generator.rs` (new).
- `crates/slicer-core/src/algos/lightning/mod.rs` (replace skeleton body; `// 139
  wiring point` comment deleted).
- `crates/slicer-ir/src/slice_ir.rs` (add `region_id: RegionId` to
  `LightningTreeEntry`; mirror `SupportPlanEntry.region_id` at `:1129`).
- `crates/slicer-wasm-host/src/dispatch.rs:1383` (per-region HashMap keying).
- `crates/slicer-sdk/src/traits.rs:195-199` (honor `region_id` in
  `lightning_tree_segments_for`).
- Test homes: the 138 lightning test file (add generator tests beside the primitive
  tests); `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (extend
  with the commits-real-trees case AND the per-region keying case for AC-3);
  `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`
  (new; AC-N3); `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`
  (137; preserved with a per-region assertion added).

## Read-Only Context

- `crates/slicer-core/src/algos/lightning/{distance_field,tree_node}.rs` — the 138
  APIs (own module).
- The support-geometry producer's whole-print input access — LOCATIONS dispatch, then
  ranged.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/lightning-infill/**` — 140's surface.
- WIT files (the 137 WIT signature stays at `2.2.0`; the per-region refinement is
  host-side — IR field + dispatch keying + SDK projection; the
  `lightning-tree-segments` method already accepts `region-id`).
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "Sectioned SUMMARY + SNIPPETS (≤ 30 lines each) of `Generator.cpp`: constructor
  inputs, `generateInitialInternalOverhangs`, the `generateTrees` two-pass loop,
  `getTreesForLayer`".
- "Sectioned SUMMARY + SNIPPETS of `Layer.cpp`: `generateNewTrees`, `reconnectRoots`,
  `convertToLines`".
- "FACT with file:line: dilation constant for overhang detection; per-layer move
  distance; any density-coupled resolution inputs from `FillLightning.cpp`'s
  `build_generator`".
- "LOCATIONS ≤ 10: how the support-geometry producer receives whole-print `SliceIR`
  inputs".
- "Run `cargo test -p slicer-core -- lightning_generator …`; FACT + counts; SNIPPETS
  ≤ 20 on failure".

## Data and Contract Notes

- IR: extends the 137 `LightningTreeIR` with one new field — `region_id:
  RegionId` on `LightningTreeEntry` (mirroring `SupportPlanEntry.region_id:
  RegionId` at `:1129`; `RegionId` is a `pub type RegionId = u64;` alias
  at `slice_ir.rs:36`, so the WIT-boundary plumbing — `region_id.to_string()`
  at `dispatch.rs:1353` — is identical to the support-plan keying). No
  schema-version bump (the additive field is backward-compatible at the IR
  level; existing 137 test fixtures that used `region_id = 0` still parse).
- Determinism: layer iteration strictly top-down by index; per-layer tree iteration
  in creation order; the new per-region keying is `region_id`-integer-sorted
  (matches `SupportPlanEntry.region_id` access pattern at `slice_ir.rs:1129`).

## Locked Assumptions and Invariants

- Two-pass top-down structure preserved (ADR-0029).
- 138 primitive APIs frozen (deviations recorded, tests co-updated).
- Skip promise (no holder → no work) preserved.
- Faithful port: constants ÷ 100, cited; behavioral divergence → `DEVIATION_LOG`.

## Risks and Tradeoffs

- Generator constructor inputs are density-coupled in Orca — the parameterization
  decided in 138 must line up; a mismatch surfaces at Step 1's constants FACT and is
  resolved by adjusting the producer's parameter sourcing (config keys read host-side),
  recorded.
- Synthetic multi-layer fixtures must be small enough to hand-verify continuity — 3-5
  layers, single overhang; resist realistic-model tests here (that is 140's pipeline
  smoke).
- Memory: whole-print `LightningTreeIR`; the compact 2-point storage decision (137)
  bounds it; no further mitigation this packet.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `L` (Step 0 — the per-region IR + dispatch + SDK projection
  bundle is one atomic coupled change set; partial state breaks the 137
  `LighttningTreeEntry` construction sites). Step 2 (the `Layer.cpp` port, 448
  lines) is `M` with the tripwire armed.
- Highest-risk dispatch: the `Layer.cpp` section series (540-line total — ≥ 4 sections).
- Step 0's L rating: the IR field, the dispatch keying, the SDK accessor, and
  the existing 137 roundtrip test are all coupled — splitting them across
  steps would leave the workspace un-compiling at the seam (the dispatch
  reads the IR's `region_id`; the SDK accessor reads the dispatch's
  HashMap; the test asserts both). One atomic step.

## Open Questions

- `[FWD]` Density-coupled generator inputs: which config keys feed the resolution
  constants (from `FillLightning.cpp`'s `build_generator` construction) — resolved by
  the constants FACT; the producer reads them host-side from the object's resolved
  config.
- `[FWD-resolved]` Per-region skip predicate: 137 collapsed to print-wide
  (`default_resolved_config.sparse_fill_holder` at `prepass.rs:670`). The packet 137
  review's investigation confirmed this is the intentional extent of the deviation —
  `ResolvedConfig::sparse_fill_holder` is print-wide, not per-region, and no
  per-region `ResolvedConfig` exists in the IR. 139's contribution is the
  per-region IR + dispatch + SDK projection; the skip-predicate is unchanged.
