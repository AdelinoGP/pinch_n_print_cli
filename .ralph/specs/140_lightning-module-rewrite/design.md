# Design: 140_lightning-module-rewrite

## Controlling Code Paths

- Primary code path: `modules/core-modules/lightning-infill/src/lib.rs` — `run_infill`
  keeps its region loop, `should_emit(SparseInfill)` gate, `begin_region` origin
  discipline (at `lib.rs:117`), and config reads (`on_print_start`); the body between
  them becomes: view (via
  `PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id)` in
  `crates/slicer-sdk/src/traits.rs`) → layer tree segments → raw `push_sparse_path`
  per segment/polyline. `build_branches` (at `lib.rs:234`, called from
  `lib.rs:195`) and the grid-sampling helpers `nearest_boundary_point`,
  `polygon_bbox_mm`, `point_in_expolygon`, `point_in_polygon` are deleted.
- Neighboring tests or fixtures: `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`
  (323 lines — pins stub behavior; rewritten), `tests/slicer_module_binding_tdd.rs`
  (kept); `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new)
  + `crates/slicer-runtime/tests/executor/main.rs` mod-line registration.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (one sampling-side SUMMARY; delegate).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- ADR-0029 module-sampler contract: NO generation, NO clipping, NO chaining in the
  module — sample and emit raw; the linker (133) clips and connects.
- Raw-emit uniformity (ADR-0025 + Amendment): this packet removes the roadmap's last
  self-linking exception; nothing may reintroduce path connection here.
- The `cargo xtask test --workspace` ceremony only via `--summary` dispatch (CLAUDE.md).

## Code Change Surface

- Selected approach: thin sampler. The view returns the (object, region, layer)'s tree
  segments the 139 producer committed; the module maps each segment to a
  `Point3WithWidth` (z derived from the dispatching `layer_index`; width from
  `self.line_width`; mm conversion at the boundary from integer-unit IR segments to
  `f32` coordinates), tagged `SparseInfill` + the `speed_factor` from config, pushed
  under the correct region origin. The only judgment call — whether Orca's Filler
  applies a sampling-side transform worth mirroring — is settled by the single
  delegated SUMMARY before coding.
- Exact changes:
  - `modules/core-modules/lightning-infill/src/lib.rs` — net −300+ lines expected;
    body swap; deleted functions listed above.
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — rewritten
    around the sampler contract (AC-1 + AC-N2); the module-binding test
    (`slicer_module_binding_tdd.rs`) is kept verbatim.
  - `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) +
    `crates/slicer-runtime/tests/executor/main.rs` mod-line — AC-3.
  - `docs/DEVIATION_LOG.md` (DEV-081 row) + `docs/07_implementation_status.md`
    (TASK-262…265 closure sweep, via dispatch) — closure artifacts.
- Rejected alternatives: (a) keeping the stub as a fallback when trees are empty —
  rejected: empty trees mean nothing to print (AC-N2); a silent stub fallback would
  mask producer bugs; (b) emitting per-tree connected polylines (walking the tree)
  instead of raw segments — rejected: that is self-linking by another name; the
  linker chains; (c) module-side clipping to the region polygon — rejected: linker
  re-clips (ADR-0025).

## Files in Scope (read + edit)

- `modules/core-modules/lightning-infill/src/lib.rs` — role: the rewrite; expected
  change: stub deleted, sampler in.
- `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — role: TDD;
  expected change: rewritten around the sampler contract.
- `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) +
  `crates/slicer-runtime/tests/executor/main.rs` (mod-line registration) — role:
  AC-3.
- `docs/DEVIATION_LOG.md` (DEV-081 row) + `docs/07_implementation_status.md` (closure
  sweep, via dispatch) — role: closure artifacts.

## Read-Only Context

- `crates/slicer-sdk/src/traits.rs` — the 137 `lightning_tree_segments_for` accessor
  (ranged; the view-pattern anchor).
- One 134/135-era module test file — the raw-emit test idiom (ranged).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — one delegated SUMMARY only; never load.
- `crates/slicer-core/src/algos/lightning/**` — 138/139's closed surface (defects
  routed, not patched here).
- `modules/core-modules/infill-linker/**` — triage boundary (requirements §Out of
  Scope).
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "SUMMARY ≤ 200 words: `FillLightning.cpp` `Filler::_fill_surface_single` — what
  happens between `getTreesForLayer` and output; is any transform applied the PnP
  sampler must mirror?" — the one Orca question.
- "Run `cargo test -p lightning-infill …`; FACT + counts; SNIPPETS ≤ 20 on failure".
- "Run `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked …`;
  FACT".
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- "Run `cargo xtask test --workspace --summary`; verdict block ONLY" — roadmap-close
  ceremony.
- "Flip DEV-081 to Closed + docs/07 TASK-262…265 closure notes; FACT + the two Doc
  Impact greps".

## Data and Contract Notes

- IR/WIT: consumer only; no contract change. If the view proves insufficient (e.g.
  missing per-tree grouping the sampler needs), that is a 137-contract deviation —
  minor bump routed through a recorded deviation, not an inline hack.
- Emission: mm at `ExtrusionPath3D` (`f32` `points: Vec<Point3WithWidth>`) from
  integer-unit IR segments via `slicer_ir::units_to_mm(...)` (the one mm↔unit boundary
  in the packet). z derived from the dispatching `layer_index` and the layer Z table.
- Determinism: emission order = IR segment order (frozen by 139).

## Locked Assumptions and Invariants

- Manifest stays `holds = ["claim:sparse-fill"]`.
- No generation/clipping/chaining in the module — sampler only.
- Empty trees → empty emission, slice completes (AC-N2) — no stub fallback exists.
- Non-lightning output byte-identical (AC-N1).
- DEV-081 closes here or the packet does not close.

## Risks and Tradeoffs

- The old test suite encodes stub semantics wholesale — rewriting it risks losing
  genuine invariants (module binding, role tagging, origin discipline); those
  specific tests are kept/adapted, and each deletion names the stub behavior it
  encoded.
- Linked-lightning visual quality is new territory (no OrcaSlicer golden to compare
  linked output against, since Orca links differently) — the bless justification
  leans on AC-1 (sampling fidelity) + AC-3 (pipeline integrity) + the HTML-report
  visual note.
- The roadmap-close ceremony may surface cross-packet debt; triage per the fence,
  record honestly (the packet-126 lesson: never flip closure before the ceremony).

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 2 — rewrite + test-suite reconciliation)
- Highest-risk dispatch: the workspace ceremony — summary-only contract.

## Open Questions

- `[FWD]` Whether Orca's Filler applies a sampling-side transform to mirror —
  settled by the single delegated SUMMARY before Step 2 codes the sampler.
- `[FWD]` Whether the live DEV-081 row already has a `Closed` status in the log's
  convention or whether the packet must change the row to `Closed` (FACT at Step 4
  start; the AC-4 grep tolerates both shapes).
