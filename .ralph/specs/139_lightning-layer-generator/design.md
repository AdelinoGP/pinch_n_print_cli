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
- Rejected alternatives: (a) committing tree topology and converting to lines in the
  module — rejected: ADR-0029's compact-IR note and the module-sampler contract; (b)
  lazy per-layer generation — rejected in 137 already; (c) parallelizing the growth
  pass — rejected for v1: determinism first, profile later.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/lightning/layer.rs` (new).
- `crates/slicer-core/src/algos/lightning/generator.rs` (new).
- `crates/slicer-core/src/algos/lightning/mod.rs` (replace skeleton body; `// 139
  wiring point` comment deleted).
- Test homes: the 138 lightning test file (add generator tests beside the primitive
  tests); `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (extend
  with the commits-real-trees case).

## Read-Only Context

- `crates/slicer-core/src/algos/lightning/{distance_field,tree_node}.rs` — the 138
  APIs (own module).
- The support-geometry producer's whole-print input access — LOCATIONS dispatch, then
  ranged.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/lightning-infill/**` — 140's surface.
- WIT/SDK files — the 137 contract is frozen; nothing to change.
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

- IR: fills the 137 `LightningTreeIR` — no schema change; if a field proves missing
  (e.g. per-tree grouping needed by 140), that is a 137-contract deviation recorded
  here with a minor schema bump, not a silent extension.
- Determinism: layer iteration strictly top-down by index; per-layer tree iteration
  in creation order.

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
- Largest single step: `M` (Step 2 — `Layer.cpp` port, 448 lines)
- Highest-risk dispatch: the `Layer.cpp` section series (540-line total — ≥ 4 sections).

## Open Questions

- `[FWD]` Density-coupled generator inputs: which config keys feed the resolution
  constants (from `FillLightning.cpp`'s `build_generator` construction) — resolved by
  the constants FACT; the producer reads them host-side from the object's resolved
  config.
