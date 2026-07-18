# Design: 112_arachne-extrusion-and-wireup

## Controlling Code Paths

- **Centrality filter.** `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (NEW) carries `filter_central(graph: &mut SkeletalTrapezoidationGraph, params: &CentralityParams)`. The function walks the half-edge graph marking each edge's `central: bool` based on the OrcaSlicer predicate (an edge is "central" iff its bead-trace stays within the polygon and meets the transition-filter-distance constraint). Mirrors OrcaSlicer `SkeletalTrapezoidation::filterCentral`. **As-shipped signature (packet 113b):** gained a third parameter, `filter_central(graph, params, transitioning_angle_rad: f64)` — the quad/rib topology rework needed the transition angle threaded in explicitly rather than derived from `params` alone; see `crates/slicer-core/src/arachne/pipeline.rs`'s call site for the live value.
- **Bead-count assignment.** `bead_count.rs` carries `assign_bead_counts(graph: &mut SkeletalTrapezoidationGraph, strategy: &dyn BeadingStrategy) -> Result<(), BeadCountError>`. For each central edge, computes `r_avg = (r_min + r_max) / 2.0` and calls `strategy.optimal_bead_count(2.0 * r_avg)`. AC-N1 enforces that centrality must run first: if any edge's `central` flag was never set (sentinel value), return `BeadCountError::CentralityNotRun`.
- **Propagation.** `propagation.rs` carries `propagate_beadings_upward(graph)` and `propagate_beadings_downward(graph)`. Each traverses the half-edge graph propagating bead counts to neighboring edges and marking transition edges as `TransitionMiddle` or `TransitionEnd` per OrcaSlicer.
- **Toolpath generation.** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (NEW) — `generate_toolpaths(graph: &SkeletalTrapezoidationGraph) -> Vec<VariableWidthLines>`. Emits per-inset lines sorted by `inset_idx` ascending. `VariableWidthLines` is a type alias for `Vec<ExtrusionLine>` (the inset's lines, all sharing the same `inset_idx`). **As-shipped signature (closing `D-112-TOOLPATH-WIDTH`, Step 9D):** gained a second parameter, `generate_toolpaths(graph, strategy: &dyn BeadingStrategy)` — per-junction widths are now read from `strategy.compute()`'s real `Beading` output instead of a geometric approximation; see that deviation's `docs/DEVIATION_LOG.md` entry.
- **Post-process pipeline.** Three sequential transformations on `Vec<ExtrusionLine>`:
  - `stitch.rs::stitch_extrusions(lines, max_gap) -> Vec<ExtrusionLine>` — joins open polylines within `max_gap`; primary perimeters (closed, `inset_idx == 0`) untouched.
  - `simplify.rs::simplify_toolpaths(lines, dp_epsilon) -> Vec<ExtrusionLine>` — DP simplification per line; preserves junction widths.
  - `remove_small.rs::remove_small_lines(lines, min_length_factor, min_width) -> Vec<ExtrusionLine>` — drops odd, non-closed lines shorter than `min_length_factor * min_width`; primary (closed, `inset_idx == 0`) NEVER removed.
- **IR types.** `crates/slicer-ir/src/slice_ir.rs` adds:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
  pub struct ExtrusionJunction {
      pub p: Point3WithWidth,
      #[serde(default)]
      pub perimeter_index: u32,
  }
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
  pub struct ExtrusionLine {
      pub junctions: Vec<ExtrusionJunction>,
      pub inset_idx: u32,
      #[serde(default)]
      pub is_odd: bool,
      #[serde(default)]
      pub is_closed: bool,
  }
  ```
  Plus a converter `pub fn extrusion_line_to_extrusion_path3d(line: &ExtrusionLine, role: ExtrusionRole) -> ExtrusionPath3D` that wraps junctions into an `ExtrusionPath3D` (`.points: Vec<Point3WithWidth>`, `.role`, `.speed_factor`) for assignment to `WallLoop.path: ExtrusionPath3D`. Do NOT produce a bare `Vec<Point3WithWidth>` — the field type is `ExtrusionPath3D`. Schema version bumps minor: implementer re-reads `CURRENT_SLICE_IR_SCHEMA_VERSION` at activation (live value = `4.6.0` per `slice_ir.rs:213`; P105 carried it to 4.4.0 for `GapFill`) and increments minor by 1 (→ `4.7.0`). Do NOT hardcode the target if a parallel branch bumps first.
- **Real wire-up.** `modules/core-modules/arachne-perimeters/src/lib.rs` — IMPLEMENT `run_perimeters` in the P110-created empty skeleton (which returns `Ok(())` + `warn!` only; the old 512-line iterative-inset fake was DELETED by P108/T-090) with the Voronoi/beading-based pipeline:
  1. Build `BeadingFactoryParams` from `_config` reads (11 `m_params.*` keys registered in P111 — FORWARD-DEP on P111).
  2. For each `SlicedRegion`:
     a. `let preprocessed = preprocess_input_outline(region.polygons(), &params)?;` — FORWARD-DEP on P110: `preprocess_input_outline` is in `crates/slicer-core/src/arachne/preprocess.rs`.
     b. If MMU: `let per_color = preprocess_per_color_inputs(painted_cells, tie_break)?;` then process each color's preprocessed cell separately.
     c. `let mut skt = SkeletalTrapezoidationGraph::from_polygons(&preprocessed)?;` — FORWARD-DEP on P110. Voronoi construction is internal to `from_polygons` (which takes `&[ExPolygon]`); there is no standalone `voronoi_from_segments`/`polygon_to_segments`/`from_voronoi` API — those symbols do not exist.
     e. `filter_central(&mut skt, &centrality_params);` — defined in this packet (T-220).
     f. `let strategy = BeadingStrategyFactory::create_stack(&beading_params);` — FORWARD-DEP on P111.
     g. `assign_bead_counts(&mut skt, &*strategy)?;` — defined in this packet (T-221).
     h. `propagate_beadings_upward(&mut skt); propagate_beadings_downward(&mut skt);` — defined in this packet (T-222).
     i. `let lines = generate_toolpaths(&skt);` — defined in this packet (T-223).
     j. `let lines = stitch_extrusions(lines, preferred_bead_width_outer - 100);` (slicer units; `preferred_bead_width_outer` is the real `BeadingFactoryParams` field — there is no `bead_width_x` field; inner-wall joins may instead derive the gap from `optimal_width`).
     k. `let lines = simplify_toolpaths(lines, dp_epsilon);`
     l. `let lines = remove_small_lines(lines, min_length_factor, min_width);`
     m. Per line, convert via `extrusion_line_to_extrusion_path3d(line, role)` and assign the resulting `ExtrusionPath3D` to `WallLoop.path` — the field type is `ExtrusionPath3D`, NOT `Vec<Point3WithWidth>`. Emit a `WallLoop` with `loop_type` per `inset_idx` (0 → `LoopType::Outer`, ≥1 → `LoopType::Inner`; `LoopType::GapFill` IS available — P105/T-062b added it additively at 4.4.0 — so odd/gap-fill lines may map to `GapFill` where appropriate). Count walls via `walls.len()` — `PerimeterRegion` has no `wall_count` field; the count is `walls: Vec<WallLoop>`.
  3. Return `Ok(())`. The P110 skeleton's `warn!`-only path is replaced by the above pipeline.

**Correction (post-implementation): real architecture is a WIT host-service bridge, not an in-guest call chain.** The step-by-step pipeline (a)-(m) above cannot run inside `arachne-perimeters` as written, because that module compiles to a `wasm32` guest component and `slicer-core`'s Voronoi/SkeletalTrapezoidation/beading code is gated behind the host-only `host-algos` feature (rayon + boostvoronoi are not WASM-portable) — a WASM guest cannot call it directly. This design section's original premise ("no `generate_arachne_walls` — that was in the P108-deleted fake, which is gone") was wrong on that point: the real implementation introduces a NEW WIT host-service, also named `generate-arachne-walls` (coincidental name reuse with the deleted P108 in-guest function — a different mechanism entirely), mirroring the existing `medial-axis` host service. `arachne-perimeters::run_perimeters` calls `slicer_sdk::host::generate_arachne_walls(polygons, &params)`, which on native targets calls `slicer_core::arachne::pipeline::run_arachne_pipeline` directly, and on `wasm32` guest builds marshals the call across the WIT boundary to the host, which runs the identical native pipeline on the guest's behalf and returns `Vec<ExtrusionLine>`. The guest module then does only steps (m) (classify + convert to `ExtrusionPath3D` + assemble `WallLoop`s) — steps (a)-(l) execute host-side inside the bridge. See `D-112-HOSTSVC-BRIDGE` in `docs/DEVIATION_LOG.md`, `modules/core-modules/arachne-perimeters/src/lib.rs`'s own module doc comment, and `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` (formalizing this bridge and the pre-existing `medial-axis` bridge it mirrors as a reusable pattern).

## Neighboring Tests & Fixtures

- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` exists from P109. T-231 extends it with an `arachne_perimeter_parity` test function that iterates the 4 new fixture directories. The cube_4color Arachne fixture is NEW and self-captured — no `cube_4color_orca.gcode` (nor any `cube_4color*` directory) exists under `perimeter_parity/` today (existing dirs: bridge, holed_square, multi_tool_triangle, overhang_ramp, solid_square, spiral_vase_cone); P109's cube_4color coverage lives in the executor test suite, not as a perimeter_parity fixture. The implementer records a fresh self-captured baseline per the repo's parity-harness convention.
- `crates/slicer-runtime/tests/executor/` already carries `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` from P109. The new `arachne_perimeters_simple_square_produces_walls` test (AC-9) lives in the same dir, mirroring the patterns established by M1 executor tests.
- Per-function unit fixtures (centrality, bead_count, propagation, generate_toolpaths, stitch, simplify, remove_small) live under `crates/slicer-core/tests/fixtures/arachne/`. Recorded JSON; small files; committed; never regenerated within this packet.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** Every constant translated from OrcaSlicer in the centrality / bead-count / propagation / generate_toolpaths code passes through `/100` (1 unit = 100 nm vs OrcaSlicer's 1 unit = 1 nm). `preferred_bead_width_outer - 100` (slicer units) is the stitch gap (OrcaSlicer's `bead_width_x - 1nm` maps to `BeadingFactoryParams::preferred_bead_width_outer - 100` after the /100 conversion; `BeadingFactoryParams` has no `bead_width_x` field). Any constant > 1000000 in geometry code is a red flag.

<!-- snippet: wasm-staleness -->
- **Guest WASM staleness.** T-224 edits IR (adds `ExtrusionLine` + `ExtrusionJunction`), and T-230 edits `arachne-perimeters/src/lib.rs`. Both invalidate guest WASM. After every IR edit AND after Step 9 (T-230 wire-up), the implementer MUST run `cargo xtask build-guests --check`; if STALE, rebuild without `--check` BEFORE running any host/executor test. Failure to rebuild causes Arachne parity tests (AC-10) to fail with a typed-instantiation error masquerading as a wire-up bug.

- **Schema additive change.** T-224 adds new IR types but does NOT remove or rename existing ones. `#[serde(default)]` on all new optional fields keeps round-trip safety with pre-bump fixtures (AC-N2). Schema bump is minor-version — the implementer re-reads the actual `CURRENT_SLICE_IR_SCHEMA_VERSION` at activation (live value at refinement = `4.6.0`; P105/P106/P109 shipped, P105 carried it to 4.4.0 for `GapFill`) and increments minor by 1 (→ `4.7.0`). Do NOT hardcode a target if a parallel branch bumps first.
- **No mid-pipeline panics.** Every `slicer-core::arachne::*` and `skeletal_trapezoidation::*` function returns `Result<_, _>`. Internal `unwrap()` is forbidden; debug-asserts are allowed for invariant checks.
- **No floating-point HashMap keys.** Same as P111 — determinism is required. Stitch's gap-comparison uses sorted Vec, not HashMap-by-distance.

## Selected Approach

**Pipeline-first wire-up; IR additions are additive.** The slicer-core extensions (Steps 1–7) are independent of one another at the function level — each takes an immutable `SkeletalTrapezoidationGraph` (or mutates it for centrality/bead_count/propagation) and produces or transforms `Vec<ExtrusionLine>`. The IR additions (Step 8) come AFTER the helpers are green because the `ExtrusionLine` IR type's shape is informed by `generate_toolpaths`'s output (specifically, the `is_odd`/`is_closed` flags emerge from the toolpath-emission code). The real wire-up (Step 9) fills the P110-created empty skeleton in one shot — the skeleton's `warn!`-only `run_perimeters` becomes the real call chain.

Rejected alternatives:
- **Land IR types first, then wire helpers around them.** Tempting (test-first IR), but the IR shape depends on the helpers' output; landing IR first risks rework. Rejected.
- **Wire `run_perimeters` incrementally (single-color first, MMU later).** Single-color is the simpler half; MMU is the T-P96-E half. Bundling avoids two wire-up passes and matches the user's "as few as logically possible" preference. Rejected as a split: T-P96-E preprocessing already shipped in P110; this packet ONLY consumes it.

For T-231 fixtures: 4 fresh fixtures + cube_4color Arachne extension. Rejected: shipping a smaller fixture set (e.g., 2 fresh + cube_4color). Reason: the 4 fresh fixtures each test a different Arachne strategy interaction (Distributed → Redistribute → Widening → Limited), and dropping any masks a regression class.

## Code Change Surface

| File | Status | Step | Notes |
| --- | --- | --- | --- |
| `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` | NEW | Step 1 | T-220 |
| `crates/slicer-core/tests/centrality.rs` | NEW | Step 1 | AC-1 |
| `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` | NEW | Step 1 | 3 fixtures |
| `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` | NEW | Step 2 | T-221 |
| `crates/slicer-core/tests/bead_count.rs` | NEW | Step 2 | AC-2 + AC-N1 |
| `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` | NEW | Step 3 | T-222 |
| `crates/slicer-core/tests/propagation.rs` | NEW | Step 3 | AC-3 |
| `crates/slicer-core/src/arachne/generate_toolpaths.rs` | NEW | Step 4 | T-223 |
| `crates/slicer-core/tests/generate_toolpaths.rs` | NEW | Step 4 | AC-4 |
| `crates/slicer-core/src/arachne/stitch.rs` | NEW | Step 5 | T-225 |
| `crates/slicer-core/tests/stitch.rs` | NEW | Step 5 | AC-6 |
| `crates/slicer-core/src/arachne/simplify.rs` | NEW | Step 6 | T-226 |
| `crates/slicer-core/tests/simplify.rs` | NEW | Step 6 | AC-7 |
| `crates/slicer-core/src/arachne/remove_small.rs` | NEW | Step 7 | T-227 |
| `crates/slicer-core/tests/remove_small.rs` | NEW | Step 7 | AC-8 + AC-N3 |
| `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` | EDIT | Steps 1–3 | `pub mod` registrations |
| `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` | EDIT | Steps 2–3 | add `STHalfEdge` fields: `bead_count: Option<u32>` [Step 2], `is_transition_middle: bool` + `is_transition_end: bool` [Step 3] |
| `crates/slicer-core/src/arachne/mod.rs` | EDIT | Steps 4–7 | `pub mod` registrations |
| `crates/slicer-ir/src/slice_ir.rs` | EDIT | Step 8 | T-224 — ExtrusionLine + ExtrusionJunction + schema bump |
| `crates/slicer-schema/wit/deps/ir-types.wit` | EDIT | Step 8 | WIT records |
| `crates/slicer-wasm-host/src/host.rs` | EDIT | Step 8 | Populate new types if exposed to guests |
| `crates/slicer-sdk/src/views.rs` | EDIT (maybe) | Step 8 | Read accessor for new types if exposed; defer to follow-on otherwise |
| `crates/slicer-ir/tests/extrusion_line_roundtrip.rs` | NEW | Step 8 | AC-5 + AC-N2 |
| `modules/core-modules/arachne-perimeters/src/lib.rs` | EDIT | Step 9 | T-230 real wire-up (IMPLEMENT into the P110 empty skeleton; the old iterative-inset impl was DELETED by P108, not rewritten) |
| `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs` | NEW | Step 9 | AC-9 |
| `crates/slicer-runtime/tests/executor/main.rs` | EDIT | Step 9 | S7 REQUIRED: add `mod arachne_perimeters_simple_square;` |
| `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/` | NEW | Step 10 | T-231 fixture 1 |
| `crates/slicer-runtime/tests/fixtures/perimeter_parity/narrow_strip_widening/` | NEW | Step 10 | T-231 fixture 2 |
| `crates/slicer-runtime/tests/fixtures/perimeter_parity/max_bead_count_cap/` | NEW | Step 10 | T-231 fixture 3 |
| `crates/slicer-runtime/tests/fixtures/perimeter_parity/complex_multi_feature/` | NEW | Step 10 | T-231 fixture 4 |
| `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/` | NEW | Step 10 | NEW self-captured baseline — no pre-existing `cube_4color_orca.gcode`/dir in `perimeter_parity/` to extend |
| `crates/slicer-runtime/tests/integration/perimeter_parity.rs` | EDIT | Step 10 | Arachne suite entry (FORWARD-DEP on P109) |
| `crates/slicer-runtime/tests/integration/main.rs` | EDIT | Step 10 | S7 REQUIRED: ensure `mod perimeter_parity;` present (may land in P109) |
| `docs/DEVIATION_LOG.md` | EDIT | Step 11 | T-232 — D-7/D-9/D-15 closures |
| `docs/01_system_architecture.md` | EDIT | Step 11 | T-233 — Tier-2 caveat removal |
| `docs/02_ir_schemas.md` | EDIT | Step 11 | ExtrusionLine/ExtrusionJunction entries + version bump rationale |
| `docs/07_implementation_status.md` | EDIT | Step 11 | M2 complete |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | EDIT | Step 11 | Flip T-220..T-234 rows + M2 marker |

## Read-Only Context

| File | Range | Purpose |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | Phases 12 + 13 rows | Task definitions |
| `docs/02_ir_schemas.md` | `Point3WithWidth` + `WallLoop` + schema-versioning section | T-224 IR shape |
| `docs/03_wit_and_manifest.md` | WIT type declaration syntax | T-224 WIT records |
| `docs/05_module_sdk.md` | `PerimeterOutputBuilder` API | T-230 wire-up |
| `docs/specs/orca-mmu-perimeter-investigation.md` | full | T-231 cube_4color Arachne extension |
| `docs/01_system_architecture.md` | Tier-2 section | T-233 |
| `docs/07_implementation_status.md` | M2 status section | T-234 + Doc Impact |
| `docs/DEVIATION_LOG.md` | D-7/D-9/D-15 entries | T-232 |
| `CLAUDE.md` | §"Test Discipline" + §"Guest WASM Staleness" | T-234 + WASM rebuild constraint |
| `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` | full (post-P110) | Centrality / bead_count / propagation operate on this graph |
| `crates/slicer-core/src/beading/mod.rs` | full (post-P111) | T-221 calls `BeadingStrategy::optimal_bead_count` |
| `crates/slicer-core/src/beading/factory.rs` | full (post-P111) | T-230 calls `BeadingStrategyFactory::create_stack` |
| `crates/slicer-core/src/voronoi.rs` | full (post-P110) | Internal to `SkeletalTrapezoidationGraph::from_polygons`; T-230 does NOT call it directly (no `voronoi_from_segments` API) |
| `crates/slicer-core/src/arachne/preprocess.rs` | full (post-P110) | T-230 calls `preprocess_input_outline` + `preprocess_per_color_inputs` |
| `modules/core-modules/classic-perimeters/src/lib.rs` | range — `WallLoop` emission pattern only | T-230 mirrors emission patterns |
| `modules/core-modules/arachne-perimeters/src/lib.rs` | full (P110 skeleton — empty `LayerModule` impl + `warn!`) | Implement in Step 9 |
| `crates/slicer-runtime/tests/integration/perimeter_parity.rs` | range-read (~1554 LOC — do not full-read) | T-231 extension |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` (~3000 LOC) — multiple SUMMARY dispatches; never direct-read.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` (~2500 LOC) — multiple SUMMARY dispatches; never direct-read.
- `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.h` — ONE LOCATIONS dispatch (≤ 10 entries) for struct fields; never direct-read.
- Other M1/M2 packet directories — closed or not yet shipped.
- `target/`, lockfiles, generated bindgen output.
- Vendored crates (boostvoronoi source) — never direct-read.

## Expected Sub-Agent Dispatches

| Step | Dispatch | Scope | Return format |
| --- | --- | --- | --- |
| Step 1 | OrcaSlicer SUMMARY — `filterCentral / filterNoncentralRegions` | SkeletalTrapezoidation.cpp | ≤ 200 words: centrality predicate + filter loop |
| Step 2 | OrcaSlicer SUMMARY — `optimal_bead_count` call site + R-derivation | SkeletalTrapezoidation.cpp | ≤ 100 words: r_min/r_max/r_avg → optimal_bead_count |
| Step 3 | OrcaSlicer SUMMARY — `propagateBeadingsUpward / Downward` | SkeletalTrapezoidation.cpp | ≤ 200 words: propagation body + transition markers |
| Step 4 | OrcaSlicer SUMMARY — `generateToolpaths` | SkeletalTrapezoidation.cpp | ≤ 200 words: emission + inset_idx sort |
| Step 5 | OrcaSlicer SUMMARY — `stitch_extrusions` | WallToolPaths.cpp | ≤ 150 words: gap-join + primary preservation |
| Step 6 | OrcaSlicer SUMMARY — `simplifyToolPaths` | WallToolPaths.cpp | ≤ 100 words: DP epsilon |
| Step 7 | OrcaSlicer SUMMARY — `removeSmallLines` | WallToolPaths.cpp | ≤ 100 words: removal rule + primary invariant |
| Step 8 | OrcaSlicer LOCATIONS — `ExtrusionLine` + `ExtrusionJunction` | ExtrusionEntity.h | ≤ 10 entries: struct fields |
| Step 9 | `cargo xtask build-guests` | n/a | FACT clean / STALE list (must be CLEAN after) |
| Step 10 | OrcaSlicer SUMMARY ×4 — per Arachne fixture | various | ≤ 100 words per fixture: expected PerimeterIR shape |
| Step 11 | None (docs work) | n/a | n/a |
| Step 12 | `cargo xtask test --workspace --summary 2>&1 \| tee target/test-output.log \| tail -20` | n/a | FACT pass/fail + summary line + count (gated entry point — fires guest-WASM freshness check) |
| All steps | `cargo test -p <crate>` narrow | n/a | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |

## Data & Contract Notes

- **`SkeletalTrapezoidationGraph` (post-P110, extended this packet)**: edges already carry `r_min`, `r_max`, `central`. This packet adds `bead_count: Option<u32>` (set by `assign_bead_counts`), `is_transition_middle: bool`, `is_transition_end: bool` (set by propagation).
- **`VariableWidthLines`**: type alias for `Vec<ExtrusionLine>`. Each `ExtrusionLine` carries its own `inset_idx`; the outer `Vec<VariableWidthLines>` returned by `generate_toolpaths` is sorted by `inset_idx`.
- **`ExtrusionLine` invariants**: `junctions.len() >= 2`; `is_closed == true` implies `junctions.first().p == junctions.last().p` (within ε); `is_odd` is true iff `inset_idx` is odd; `inset_idx == 0` represents the outermost wall.
- **`ExtrusionJunction.perimeter_index`**: zero-based index within the wall sequence at that vertex. Used by P112's downstream consumers (none yet — defer to follow-on if no consumer).
- **`CentralityParams`**: bundles `transition_filter_dist: f64`, `min_central_distance: f64`. Read from `BeadingFactoryParams`.
- **`BeadCountError`**: `{ CentralityNotRun, InvalidGraph(String) }`.

## Locked Assumptions and Invariants

- `SkeletalTrapezoidationGraph` from P110 is immutable through preprocessing; centrality/bead_count/propagation mutate it via `&mut` references.
- `generate_toolpaths` output's `inset_idx` is monotone: lines at lower indices are outer; the outer Vec is sorted by inset_idx ascending.
- `stitch_extrusions`'s primary preservation: any `ExtrusionLine` where `is_closed == true && inset_idx == 0` is NEVER touched. This is the invariant under AC-6.
- `remove_small_lines`'s primary preservation: same invariant under AC-8 (and AC-N3 negative test). The function MUST check `is_closed && inset_idx == 0` BEFORE the length check.
- Schema bump is minor (`#[serde(default)]` on new optional fields). Live value at refinement = `4.6.0`; P105/P106/P109 shipped (P105 carried it to 4.4.0 for `GapFill`). Implementer re-reads the actual constant at activation and increments minor by 1 (→ `4.7.0`) — the target is NOT hardcoded here. The implementer MUST run AC-N2 (legacy deserialization) before flipping status — if the test fails, the migration adapter is wrong.
- centrality, bead-count, propagation, generate_toolpaths, stitch, simplify, remove_small placed in `slicer-core` (extending P110/P111 surfaces). `ExtrusionLine`/`ExtrusionJunction` IR additions remain in `slicer-ir` (no rename). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.

## Risks and Tradeoffs

- **Real wire-up surface.** T-230 replaces the placeholder with ~50+ lines of pipeline call chain. Risk: edge cases in MMU (per-color iteration) or in degenerate inputs (empty polygons after preprocess). Mitigation: AC-9 (simple square) catches the happy path; AC-10's 4 fixtures + cube_4color cover MMU + edge cases.
- **Schema bump cross-cuts.** T-224 bumps `CURRENT_SLICE_IR_SCHEMA_VERSION`. Live value at refinement = `4.6.0` (P105/P106/P109 shipped). If any parallel branch also bumps before activation, the implementer reconciles by reading the actual constant first and incrementing minor by 1 from that value.
- **Workspace test ceremony surface.** T-234 (gated `cargo xtask test --workspace`) takes >11 minutes; the dispatch returns only the summary line + count. If a regression surfaces, the implementer runs targeted re-checks against the specific failing test — does NOT re-run the whole suite.
- **OrcaSlicer SUMMARY drift.** SUMMARYs ≤ 200 words may omit subtleties (e.g., `propagateBeadingsUpward`'s tie-break when an edge has two upstream beadings). Mitigation: the per-fixture goldens are the source of truth; if a function can't make a golden green after 2 attempts, re-dispatch a tighter SUMMARY for that specific edge case.
- **boostvoronoi version drift.** This packet doesn't change `Cargo.toml`'s `boostvoronoi` pin from P110; if a major boostvoronoi update lands during M2 work and breaks the wrapper, the implementer pins forward via a follow-on (NOT in this packet).

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 9 (real wire-up). Sub-step budget: M. If Step 9 reaches 60% context, the implementer hands off (the MMU per-color branch is the most likely overflow).
- Highest-risk dispatch: Step 4's `generateToolpaths` SUMMARY — the function is the densest in `SkeletalTrapezoidation.cpp`. If the SUMMARY returns > 250 words, re-dispatch tighter focused on the inset-emission loop body.
- Step 12 (workspace ceremony) is S — the implementer reads only the FACT pass/fail + summary line + count returned by the dispatch.

## Open Questions

- **[FWD]** At activation, what is `CURRENT_SLICE_IR_SCHEMA_VERSION`? Resolve at Step 8 via `rg -n 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs`. Live value at refinement = `4.6.0` (→ target `4.7.0`). Bump minor by 1 from whatever the activation-time value is; do NOT assume `4.7.0` if another branch bumps first.
- **[FWD]** Should `ExtrusionLine`/`ExtrusionJunction` be exposed via WIT/view accessors to OTHER modules besides `arachne-perimeters`? Likely not in this packet — defer to a follow-on if any consumer surfaces. Update `crates/slicer-sdk/src/views.rs` ONLY if a consumer exists at activation time.
- **[FWD]** Cube_4color Arachne reference: does it reuse the same `.gcode` file recorded by P109 / T-P96-C3 (Classic) or does Arachne need its own reference because per-color preprocessing diverges from per-edge skip-mask? Per the roadmap's T-P96-E acceptance ("Cube_4color parity test (T-P96-C3) passes for Arachne"), the SAME reference applies — Arachne's per-color preprocessing should produce parity-equivalent output to Classic's per-edge skip-mask. Confirm at Step 10 by running the test and reading the diff; if reference must be Arachne-specific, the implementer records `cube_4color_arachne.gcode` alongside the M1 reference.
- **None [BLOCK].** Every blocking question is resolved by P105's investigation (D-13, D-15) or by P110's ADR-0023 (D-7) or by P111's T-215b (D-9).
