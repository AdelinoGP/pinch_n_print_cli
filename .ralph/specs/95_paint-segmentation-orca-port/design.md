# Design: 95_paint-segmentation-orca-port

## Controlling Code Paths

- Primary code paths: a NEW module `crates/slicer-core/src/algos/paint_segmentation/` (replaces the single broken file `paint_segmentation.rs`), `crates/slicer-core/src/polygon_ops.rs` (new helpers — sub-step 0), `crates/slicer-core/src/triangle_mesh_slicer.rs` (extended with `slice_mesh_slabs` — sub-step 10), `crates/slicer-core/Cargo.toml` (boostvoronoi dep — sub-step 7), `crates/slicer-runtime/src/prepass.rs` (driver position change — sub-step 15), `crates/slicer-runtime/src/blackboard.rs` (drop `paint_regions` accessor + `commit_paint_regions` — sub-step 16), `crates/slicer-runtime/src/layer_executor.rs:494-528` + `slice_postprocess.rs:24, 302` (drop rtree field, no-op per-layer annotation — sub-step 17), `crates/slicer-core/src/paint_region.rs` (DELETE — sub-step 16), `crates/slicer-ir/src/slice_ir.rs` (drop `PaintRegionIR` / `LayerPaintMap` / `SemanticRegion` — sub-step 16).
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` (12 RED → GREEN) + `cube_fuzzy_painted_tdd.rs` (12 RED → GREEN); new per-sub-step unit tests under `crates/slicer-core/src/algos/paint_segmentation/*` and `crates/slicer-core/tests/`.
- OrcaSlicer comparison surface: see `requirements.md`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Phase-isolation invariant: each phase's output type is documented and unit-tested in isolation. Phase 3 outputs `Vec<PaintedLine>`; Phase 4 outputs `Vec<ColoredSegment>`; Phase 6 outputs per-semantic ExPolygon maps; Phase 7 outputs the final variant-chain map. Mixing phase outputs across kernel functions is forbidden.
- H561 invariant: the boostvoronoi color slot is dual-use (winding-tag AND graph metadata). The Rust crate exposes it as `Vertex::get_color() -> ColorType` / `Vertex::set_color(ColorType) -> ColorType` / `Vertex::or_color(ColorType) -> ColorType` (matching `Edge::get_color`/`set_color`/`or_color`) — note `get_color`, NOT `color()` as in the OrcaSlicer C++ source. Use typed-state wrappers `VoronoiVertex` (boost-emitted) and `GraphVertex` (post-pruning) to prevent silent mix; never read `get_color()` on a `GraphVertex`-tagged value.
- Boostvoronoi-`twin` invariant: `Edge::twin()` returns `Result<EdgeIndex, BvError>` (not a bare pointer like the C++ source). Every twin-walk in the kernel MUST `?`-propagate the `BvError` — never `.unwrap()` or `.expect()` on `twin()` in non-test code, since a graph-construction error here corrupts every downstream Phase 4d/4e/4f walk and the H567 explicit-index-tracking invariant assumes the walk completed without error.
- H562 invariant: repair sentinel for `extract_colored_segments` is `Option<usize>::None`, never `usize::MAX`. `usize::MAX` is a valid graph node index in extreme cases; using it as a sentinel is the OrcaSlicer-port bug we explicitly avoid.
- H565 invariant: do NOT replicate the OrcaSlicer bug of hardcoding extruder 0's nozzle. Read each extruder's own nozzle from `config-view`.
- H566 invariant: degree-bounded dedup uses `HashSet<EdgeKey>` with `debug_assert!(degree <= 20)` (graph degrees are bounded by triangle adjacency in practice).
- H567 invariant: explicit index tracking in `extract_colored_segments`, NOT pointer arithmetic.
- Driver-position invariant: paint-segmentation runs AFTER `host:slice` (consumes `SliceIR`) and AFTER `host:shell_classification` (consumes surface classification). Writes via `replace_slice_ir` — the same blackboard contract `host:mesh_segmentation` follows for its slot.
- D14 invariant: modifier-volume support (`SupportEnforcer` / `SupportBlocker`) routes to `segment_annotations`, NOT region-split. This is critical — modifier-volume support is a per-contour-point property, not a variant axis.
- D15 invariant: per-variant polygons may be empty (no geometric coverage) but the entry still exists in `RegionMapIR` (placed by P1c). This packet just populates the polygons.
- **Empty-polygon ownership (handed off from P93 refinement)**: `RegionPlan` entries arriving from P93's `RegionMapIR` may carry empty per-variant polygons unconditionally (P93 follows D15 by emit-without-gating). P95 has the polygons in hand via `replace_slice_ir` and owns the empty-polygon gate. **Decision deferred to sub-step 13 (the integrating driver)**: the integrator chooses whether to filter empty-polygon entries when emitting `SlicedRegion`s, OR to leave them as no-ops downstream. The packet-level acceptance does not pre-bind that choice.

## Code Change Surface

- Selected approach: bottom-up — helpers first (sub-steps 0-1-2-3), then per-phase drivers (4-5-6-7-8-9-10-11-12), then the integrating driver (13), then preserve modifier-volume (14), then wire and delete old surface (15-16-17). Each sub-step is gated by its own unit tests before the next begins. Phase 4's `boostvoronoi` spike at sub-step 7 is the risk gate.
- Exact functions, traits, manifests, tests, or fixtures expected to change: **see `task-map.md`** for the per-sub-step crosswalk to the roadmap's table. Highlights:
  - **`crates/slicer-core/src/algos/paint_segmentation/`** — new directory replacing the single file. Modules: `triangle_intersect.rs`, `edge_grid.rs`, `painted_line.rs`, `preprocess.rs`, `phase3.rs`, `colorize.rs`, `voronoi_graph.rs`, `voronoi_prune.rs`, `extract_segments.rs`, `top_bottom.rs`, `compose_variants.rs`, `modifier_volumes.rs`, `mod.rs`. Each module ≤ 200 LOC where possible.
  - **`crates/slicer-core/src/polygon_ops.rs`** — 9 new public functions (sub-step 0).
  - **`crates/slicer-core/src/triangle_mesh_slicer.rs`** — `slice_mesh_slabs` (sub-step 10).
  - **`crates/slicer-core/Cargo.toml`** — `boostvoronoi` dep (sub-step 7).
  - **`crates/slicer-runtime/src/prepass.rs`** — new insertion point for `host:paint_segmentation` between `shell_classification` and `support_geometry`; delete the old position (around line 374's `paint_segmentation_producer` invocation if it was wired there in P94 — verify Step 1 dispatch).
  - **`crates/slicer-runtime/src/blackboard.rs`** — drop `paint_regions` accessor + `commit_paint_regions`.
  - **`crates/slicer-runtime/src/layer_executor.rs:494-528`** — drop `run_paint_annotation` body (no-op or remove).
  - **`crates/slicer-runtime/src/slice_postprocess.rs:24, 302`** — drop rtree field + obsolete annotation shim.
  - **`crates/slicer-core/src/paint_region.rs`** — DELETE entire file.
  - **`crates/slicer-ir/src/slice_ir.rs`** — drop `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`.
- Rejected alternatives that were considered and why they were not chosen:
  - **Port Phase 5 in this packet**: too much surface in one packet. Defer to P4 (96).
  - **Keep PaintRegionIR as a parallel slot to SliceIR**: rejected per D8 (inline; delete). The rtree path is dead code after this packet.
  - **Use `spade` Voronoi from day 1**: rejected — try `boostvoronoi` first (spec §2 default). If sub-step 7 spike fails, fall back to `spade`.
  - **Drop H561 typed-state wrappers as overkill**: rejected — the dual-use vertex.color() pattern is the single most common OrcaSlicer-port bug source.

## Files in Scope (read + edit)

Per the sub-step list above. The full per-sub-step file list lives in `task-map.md`. Aggregate: ~15 new files in the new module + ~6 existing files edited. Each individual sub-step touches ≤ 3 files.

## Read-Only Context

- `docs/specs/orca-paint-segmentation-parity.md` — range-read per Phase per sub-step.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P3".
- `docs/02_ir_schemas.md` — SliceIR / SlicedRegion sections.
- `docs/04_host_scheduler.md` — prepass driver shape.
- `docs/08_coordinate_system.md` — coordinate constants.
- `crates/slicer-core/src/algos/paint_segmentation.rs` (the old broken file) — read briefly during sub-step 16's deletion to confirm consumers; the modifier-volume sub-pipeline lives at lines 374-417 and is salvaged into `modifier_volumes.rs`.
- `crates/slicer-core/src/paint_region.rs` — read briefly during sub-step 16 to confirm no surprising consumers.
- `boostvoronoi` crate (crates.io name `boostvoronoi`, current v0.12.1) — Rust port of Boost 1.76.0 `polygon::voronoi` (Fortune's algorithm; line-segment sites supported per upstream README). Canonical source + README: <https://codeberg.org/eadf/boostvoronoi_rs>. Pre-confirmed APIs (no spike needed): `Vertex::get_color/set_color/or_color`, `Edge::is_primary/is_secondary/twin/next/prev/vertex0/cell/is_linear/is_curved/get_color/set_color/or_color`, `ColorType` alias — see <https://docs.rs/boostvoronoi/0.12.1/boostvoronoi/prelude/struct.Vertex.html> and `.../struct.Edge.html`. Delegate any further docs fetch via WebFetch SUMMARY (≤ 200 words); do NOT clone the repo into the workspace.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- Binary 3MF / STL fixtures — never `Read`.
- The 12 cube_4color RED tests + 12 cube_fuzzy_painted RED tests at `crates/slicer-runtime/tests/executor/cube_*_tdd.rs` — read only the failure messages, not the test bodies in full. Each file may exceed 300 lines.

## Expected Sub-Agent Dispatches

(Heavy; this packet is the largest. Representative subset.)

- "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` lines [Phase X range]; return SUMMARY ≤ 200 words" — per sub-step.
- "Run `cargo test -p slicer-core paint_segmentation::<sub_module> 2>&1 | tee target/test-output.log`; return FACT pass/fail" — per sub-step gate.
- "Spike `boostvoronoi`: build a synthetic 4-line graph + dump vertex coords + dump `vertex.color()` + dump infinite-edge clipping behavior; return SUMMARY ≤ 200 words on API support for line-segment sites, vertex.color metadata, infinite-edge clipping via `is_primary()`/`twin()`" — sub-step 7 risk gate.
- "Run `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — AC-17.
- "Run `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log`; return FACT pass/fail" — AC-18.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p95-wedge.gcode && sha256sum /tmp/p95-wedge.gcode`; return FACT (sha256)" — AC-19.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p95-cube-1.gcode && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p95-cube-2.gcode && diff -q /tmp/p95-cube-1.gcode /tmp/p95-cube-2.gcode`; return FACT exit 0/non-0" — AC-N3 determinism.

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion` are DELETED (D8). `SlicedRegion.variant_chain` (added in P1a) now becomes meaningfully populated. `SlicedRegion.segment_annotations` (renamed in P1a) becomes the routing destination for non-region-split semantics.
- WIT boundary considerations: deleting `PaintRegionIR` is breaking for any WIT consumer that referenced it. The `wit/` files likely never referenced PaintRegionIR (the rtree was host-only), but Step 1 dispatch confirms.
- Determinism or scheduler constraints: `boostvoronoi` vertex output ordering — confirm in sub-step 7 that the crate provides deterministic ordering OR add a sort pass.

## Locked Assumptions and Invariants

- **Phases 1-4-6-7 (not 5) shipped this packet**: Phase 5 in packet 96. Documented in the closure log.
- **`Option<usize>` repair sentinel (H562)**: forbids `usize::MAX` as a sentinel anywhere in the new module.
- **Driver position D1**: paint-segmentation runs POST-`host:slice`, between `host:shell_classification` and `host:support_geometry`. The DAG validator (via `required_slots` table) enforces.
- **Modifier-volume routing D14**: SupportEnforcer / SupportBlocker volumes route to `segment_annotations`, NOT to a region-split. Spec §7 + roadmap D14.
- **`PaintRegionIR` and related types deleted**: no transitional shim remains.
- **Short-circuit telemetry pattern**: when the driver short-circuits on no-paint-data input, it emits `ProgressEventType::StageStart` then immediately `ProgressEventType::StageComplete` with `elapsed_ms == 0` and zero intervening `ProgressEventType::ModuleStart` events. No new `ProgressEventType::StageSkipped` variant is added — that schema change is deferred. Workspace today has no `StageSkipped` (per `docs/09_progress_events.md` + `crates/slicer-runtime/src/progress_events.rs::ProgressEventType`); reuse the existing channel.

## Risks and Tradeoffs

- **Risk: `boostvoronoi` API doesn't match spec assumptions** (sub-step 7). Mitigation: API spike at the START of Phase 4 work; fall back to `spade` + custom Voronoi wrapper or cxx-bridge to OrcaSlicer's boost::polygon::voronoi. Document in `docs/specs/orca-paint-segmentation-parity.md` open Q5.
- **Risk: Phase 6 `slice_mesh_slabs` more involved than expected** (sub-step 10). Mitigation: separately landable; verifiable with a single cube_4color RED test that targets the top face's two tool indices (the "projection coverage" test).
- **Risk: per-semantic Voronoi pass count balloons on contrived inputs**. Mitigation: spec §6 threading model + Rayon par_iter; document scaling.
- **Risk: modifier-volume sub-pipeline diverges from main paint pipeline**. Mitigation: unit-test the mix (modifier-volume SupportEnforcer + facet Material on same layer).
- **Risk: removing PaintRegionIR breaks a consumer not in the audit list**. Mitigation: `rg -nl 'PaintRegionIR|PaintRegionRTreeIndex|point_in_paint_region' crates/` post-delete; expect 0.
- **Tradeoff: large packet vs. fine-grained packets.** The IR-contract switch (delete PaintRegionIR + inline + driver position change) cannot be partially landed. Smaller packets would leave the workspace uncompilable mid-port.

## Context Cost Estimate

- Aggregate: `M` (despite 17 sub-steps; each is bounded).
- Largest single step: `M` (sub-step 13 — the integrating driver).
- Highest-risk dispatch: sub-step 7's `boostvoronoi` API spike SUMMARY. The return shapes the next 3-4 sub-steps.

## Open Questions

- `[FWD]` — Does `boostvoronoi 0.12.1` emit Voronoi vertices in a deterministic order across re-runs of the same input? Spec §6 + roadmap require deterministic output to keep g-code SHA stable (AC-19, AC-N3). The crate documents no explicit ordering guarantee; the spike at sub-step 7 must include a "construct twice, compare vertex sequence" check. If non-deterministic, the kernel adds a post-construction sort pass keyed on `(x, y, get_id())` before any downstream phase reads vertices. Failure mode is bounded (sort) — not a packet-redesign blocker; resolvable mid-flight at Step 5. ~~Previously [BLOCK] on `vertex.color()` and edge `is_primary()`/`twin()` accessors — RESOLVED: confirmed via docs.rs (`Vertex::get_color`, `Edge::is_primary`, `Edge::twin -> Result<EdgeIndex, BvError>`). API-naming + Result-twin hazards recorded in Architecture Constraints (H561 / boostvoronoi-twin invariants).~~
- `[FWD]` — The exact line number of the existing `paint_segmentation_producer` invocation in `prepass.rs` may have drifted between roadmap-write and now (P94 wiring may have shifted neighbors). Sub-step 15 dispatch confirms.
- `[FWD]` — Where does `paint_segmentation_producer.rs`'s `MESH_SEGMENTATION_PRODUCER`-pattern constant live and what stage_id does it currently claim? After the new driver wires in at the new position, the old constant either retargets to the new stage or is deleted.
