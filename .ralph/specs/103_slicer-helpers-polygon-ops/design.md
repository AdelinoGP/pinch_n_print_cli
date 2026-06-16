# Design: 103_slicer-helpers-polygon-ops

## Controlling Code Paths

- Primary code path: `slicer_helpers::{polygon_ops, medial_axis, polygon_tree, geometry}` gain new exports. `slicer_ir::slice_ir` gains `ThickPolyline`, `Point2WithWidth`, and `variable_width`. `arachne-perimeters/src/lib.rs` deletes its local `Ray`, `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` and consumes the promoted versions from `slicer-helpers::geometry`.
- Neighboring tests / fixtures: 6 new TDD files under `crates/slicer-helpers/tests/` (and one IR-side test under `crates/slicer-ir/tests/`). All fixtures are analytic — no recorded reference outputs. Existing `arachne-perimeters` tests (`boundary_paint_tdd`, `arachne_perimeters_tdd`) act as regression guards for the geometry-promotion step.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Schema-version contract: the new types are additive. `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps by one minor (4.1.0 → 4.2.0 if this packet ships first, 4.2.0 → 4.3.0 if packet 100 ships first). `#[serde(default)]` is unnecessary on the new fields because no existing struct gains them; they appear as new top-level types only.
- WIT type identity: `ThickPolyline` and `Point2WithWidth` are new records in `crates/slicer-schema/wit/deps/ir-types.wit`. No existing record type changes shape in this packet.
- Purity invariant: every new `slicer-helpers` function is a pure function over its inputs. No host-services calls, no logging, no global state. This keeps the helpers safely callable from guest WASM later (M2).

## Code Change Surface

- Selected approach: each primitive lands in its **own file** under `crates/slicer-helpers/src/` (rather than concatenated into `polygon_ops.rs`) so per-file context cost stays small for downstream consumers. `polygon_ops.rs` gains only the two thin wrappers (`offset2_ex`, `opening_ex`, `keep_largest_contour_only`) that build on existing Clipper2 calls; `medial_axis`, `polygon_tree`, `geometry` each get a dedicated file. The IR additions (`ThickPolyline`, `Point2WithWidth`, `variable_width`) ride in `slice_ir.rs` because that's where `ExtrusionPath3D` and `Point3WithWidth` already live; splitting them out would fracture the variable-width contract across files.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-helpers/src/polygon_ops.rs` — add `pub fn offset2_ex`, `pub fn opening_ex`, `pub fn keep_largest_contour_only`.
  - `crates/slicer-helpers/src/medial_axis.rs` (NEW) — `pub fn medial_axis(input, min_width, max_width, &mut out)`.
  - `crates/slicer-helpers/src/polygon_tree.rs` (NEW) — `pub struct PolygonTreeNode`; `pub fn build_polygon_tree`.
  - `crates/slicer-helpers/src/geometry.rs` (NEW) — `pub struct Ray`, `pub struct NearestPointResult`, three `pub fn`s promoted from `arachne-perimeters`.
  - `crates/slicer-helpers/src/lib.rs` — `pub mod medial_axis;`, `pub mod polygon_tree;`, `pub mod geometry;` declarations.
  - `crates/slicer-ir/src/slice_ir.rs` — add `ThickPolyline`, `Point2WithWidth`, `variable_width`; bump schema version.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `thick-polyline` and `point2-with-width` records.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — delete local ray ops; consume `slicer_helpers::geometry`.
  - 7 new test files under `crates/slicer-helpers/tests/` and `crates/slicer-ir/tests/`.
  - `docs/02_ir_schemas.md`, `docs/13_slicer_helpers_crate.md` per the Doc Impact Statement.
- Rejected alternatives that were considered and why they were not chosen:
  - Put all primitives in `polygon_ops.rs`: rejected — file becomes ≥ 600 LOC and conflates Clipper2 wrappers with medial-axis math.
  - Make `medial_axis` return `Vec<ThickPolyline>` instead of taking `&mut out`: rejected — OrcaSlicer's signature uses an out-parameter, and the perimeter modules will repeatedly accumulate into a single buffer per region. Matching the canonical signature avoids a per-call allocation pattern.
  - Add `ThickPolyline` to `slicer-helpers` instead of `slicer-ir`: rejected — IR types belong in `slicer-ir` by convention; `variable_width` is the producer/consumer bridge for `ExtrusionPath3D`, which lives in `slicer-ir`.

## Files in Scope (read + edit)

Primary edit surface lists more than 3 files because the packet creates six independent primitives. The three **most-edited** files (highest LOC delta) are listed first; the rest are justified as small mechanical additions.

- `crates/slicer-helpers/src/medial_axis.rs` (NEW) — role: medial-axis port; expected change: ~200–300 LOC implementation + helper structs.
- `crates/slicer-helpers/src/polygon_tree.rs` (NEW) — role: hole/contour tree builder; expected change: ~80 LOC.
- `crates/slicer-helpers/src/geometry.rs` (NEW) — role: ray ops promoted from `arachne-perimeters`; expected change: ~120 LOC moved verbatim (with cleanup to remove `arachne-perimeters` specific name shadowing).
- `crates/slicer-helpers/src/polygon_ops.rs` — role: additions for `offset2_ex` / `opening_ex` / `keep_largest_contour_only`; expected change: ~50 LOC added.
- `crates/slicer-helpers/src/lib.rs` — role: module declarations; expected change: 3 lines.
- `crates/slicer-ir/src/slice_ir.rs` — role: IR additions; expected change: ~40 LOC for two new structs + one converter + schema bump.
- `crates/slicer-schema/wit/deps/ir-types.wit` — role: WIT mirror of IR additions; expected change: ~12 LOC.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: consume promoted ray ops; expected change: ~50 LOC removed (deletions) + 1 import line.

## Read-Only Context

- `docs/13_slicer_helpers_crate.md` — read full file (≈ 250 lines) — purpose: align new exports with crate convention (module structure, naming, doc-comment style).
- `docs/02_ir_schemas.md` — delegate SUMMARY for §"Variable-width geometry" and §"Schema Versioning" — purpose: confirm `ExtrusionPath3D` shape and additive-bump rules.
- `docs/08_coordinate_system.md` — read full file (≈ 250 lines) — purpose: confirm mm↔unit conversion sites in each new primitive (every primitive crosses this boundary).
- `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist" only (≈ 30 lines) — purpose: comply with type-identity gates after the WIT change.
- `CLAUDE.md` — §"Guest WASM Staleness" and §"WIT/Type Changes Checklist" — purpose: comply with rebuild and identity gates.
- `crates/slicer-helpers/src/polygon_ops.rs` — read current contents to align style — purpose: keep new additions stylistically consistent with existing Clipper2 wrappers.
- `modules/core-modules/arachne-perimeters/src/lib.rs:326-466` — read the existing ray ops to confirm signature preservation during promotion — purpose: ensure Step 5 is a verbatim move with no semantic change.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate parity checks; never load. Especially: `Geometry/MedialAxis.cpp` body, `ClipperUtils.cpp` body. The implementer requests SUMMARYs, not snippets.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `crates/clipper2-rs` or whatever Clipper2 wrapper is used — out of scope; this packet builds on the existing wrapper rather than re-implementing Clipper2.
- All `modules/core-modules/*/src/lib.rs` except `arachne-perimeters/src/lib.rs` — out of scope (no consumer wiring beyond `arachne-perimeters`'s ray-op promotion).
- `classic-perimeters/src/lib.rs` — explicitly out of scope; its consumption of these primitives lands in Phase 5/6 packets.

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace --all-targets` after each step; return FACT (pass/fail) + SNIPPETS (≤ 20 lines on fail)" — purpose: cross-crate compile gate after every step.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` § for `offset2_ex` parameter order; return SUMMARY ≤ 100 words" — purpose: Step 1 parameter-order confirmation.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp` § for the `medial_axis(min, max, &out)` parameter contract and degenerate-input handling; return SUMMARY ≤ 150 words" — purpose: Step 2 contract confirmation.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779` § for the hole/contour containment + child-ordering algorithm; return SUMMARY ≤ 150 words" — purpose: Step 3 algorithm shape.
- "Find all callers of `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` across the workspace; return LOCATIONS ≤ 20 entries" — purpose: confirm Step 5 has only one consumer (`arachne-perimeters`).
- "Run `cargo test -p slicer-helpers --tests`; return FACT (pass/fail count + failing-test names if any)" — purpose: verify all six AC TDDs at packet-close.
- "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list ≤ 5 entries)" — purpose: Step 4 (IR + WIT) closure gate.

## Data and Contract Notes

- IR or manifest contracts touched: `ThickPolyline` and `Point2WithWidth` added; `variable_width` converter introduced. Additive — no existing field shape changes.
- WIT boundary considerations: `thick-polyline` and `point2-with-width` records added. Per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass after the WIT edit before declaring the step done.
- Determinism or scheduler constraints: none. All primitives are pure functions over their inputs. Polygon-tree's child ordering must be deterministic — by ascending `polygon_index` within each parent's children list — to keep downstream consumers' iteration stable.

## Locked Assumptions and Invariants

- Every primitive in `slicer-helpers` is pure: same inputs → same outputs, no I/O, no logging, no global state. The polygon-tree's child ordering is deterministic by ascending source index.
- `1 unit = 100 nm` is the coordinate system across the workspace (per ADR / `docs/08`). Every mm↔unit boundary in this packet uses `Point2::from_mm` / `mm_to_units` (or the analogous typed helpers); raw `* 10_000.0` arithmetic is forbidden.
- `medial_axis`'s `min_width` and `max_width` are inclusive bounds on per-vertex `width` output; medial-axis paths in regions thinner than `min_width` are dropped, regions wider than `max_width` are not produced (they should be handled by full perimeters instead).
- `offset2_ex(polys, neg_delta, pos_delta, …)` parameter order is **negative first, positive second** — matching OrcaSlicer's `ClipperUtils.cpp` signature. Reversing the argument order is a contract break, not a stylistic choice.

## Risks and Tradeoffs

- `medial_axis` correctness vs OrcaSlicer parity: the Rust port targets the documented interface, not the exact C++ algorithm. The rectangle test (AC-2) catches gross-correctness issues; more nuanced parity (acute corners, near-degenerate shapes) will surface during Phase 6 thin-wall integration. Mitigation: AC-2 + AC-N1 catch the most common defect modes (centerline shift, width misreport, degenerate-input crash) early.
- Schema-bump race with packet 100: if packet 100 lands first (4.1.0 → 4.2.0 for `MaterialBoundary` widening), this packet's bump becomes 4.2.0 → 4.3.0. The doc-impact greps allow either bump (the regex matches `4\.[12]\.0.*MaterialBoundary` and `4\.[123]\.0.*ThickPolyline`). Document the ordering rationale in the closure log.
- `keep_largest_contour_only` corner case: ties (two polygons with equal area within float tolerance) must produce a deterministic outcome — pick the polygon with the lower index. Tested in AC-5's test file as a secondary assertion.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — `medial_axis` port has the longest LOC delta and the highest mathematical complexity).
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): "OrcaSlicer `medial_axis` SUMMARY" — MUST return ≤ 150 words. If the SUMMARY returns code or exceeds the cap, the implementer halts and re-dispatches with explicit `≤ 100 words, no code` to enforce.

## Open Questions

- `[FWD]` `medial_axis` tolerance baseline: AC-2 specifies ±0.05 mm on a 1 mm × 10 mm rectangle. For wider shapes (e.g. 2 mm × 10 mm) the tolerance should scale with feature size — exact scaling rule to be documented in `docs/13_slicer_helpers_crate.md` during Step 2 once the port is benchmarked. Implementer chooses the doc'd rule.
- `[FWD]` `PolygonTreeNode` API stability: the `is_contour: bool` field mirrors OrcaSlicer's `PerimeterGeneratorLoop`. If a downstream consumer (Phase 5/6) needs additional metadata (e.g. depth), add it via a separate field rather than retrofitting; flag in Phase 5/6 packet's design.
