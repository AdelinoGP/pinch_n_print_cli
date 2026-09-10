# Design: 303-elefant-foot-slice-postprocess

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/algos/elephant_foot.rs` (new, ungated kernel) consumed by `modules/core-modules/elefant-foot/src/lib.rs` (new guest module) on stage `Layer::SlicePostProcess`, writing back through `SlicePostprocessBuilder::set_polygons` (`crates/slicer-sdk/src/builders.rs`).
- Neighboring tests/fixtures: `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` and `crates/slicer-core/tests/aabb_tree_tdd.rs` for the `algo_*_tdd` shape; `modules/core-modules/gyroid-infill/tests/{gyroid_infill_tdd.rs,slicer_module_binding_tdd.rs}` for the module test pair; `crates/slicer-wasm-host/test-guests/dispatch-layer-slice-postprocess-guest/src/lib.rs` for the `RegionKey` construction shape on this exact stage.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **The kernel must be ungated.** `crates/slicer-core/src/algos/mod.rs` gates every module except `bridge_over_infill` behind `#[cfg(feature = "host-algos")]`. `elephant_foot` joins `bridge_over_infill` as ungated, because a WASM guest links `slicer-core` without that feature (`modules/core-modules/gyroid-infill/Cargo.toml` is the dependency precedent) and `host-algos` pulls in `rayon` and `boostvoronoi`, neither of which belongs in a guest. The kernel therefore may not use `rayon`, may not use anything behind the feature, and its test target may not carry `required-features`.
- **`Layer::SlicePostProcess` is the right seam and is currently empty.** It sits between `Layer::PaintRegionAnnotation` and `Layer::Perimeters` in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`), and the layer executor treats it as a merge into the existing `SliceIR` rather than a primary commit (`crates/slicer-runtime/src/layer_executor.rs`). It is dispatched per layer with that layer's regions, which is exactly the shape of canonical's per-layer taper — this is why the packet does **not** follow packet 297's host-prepass choice: 297 needed the layer *above* (a cross-layer read a per-layer stage cannot serve), whereas elephant-foot reads only the layer's own footprint. Zero production modules occupy the stage today; the seam is exercised only by `dispatch-layer-slice-postprocess-guest`, so treat first-occupancy surprises (ordering, commit merge, progress events) as expected discovery, not as a defect in this design.
- **Rule 4 does not fire.** Canonical has exactly one elephant-foot implementation and no enum selecting between alternatives, so there is no claim to hold or require. `[claims] holds = []`, `requires = []`. Do not invent a claim ID.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- **ADR conformance (no amendment).** The two most recently landed ADRs govern extrusion-path ordering: `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` and `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md`. Neither is contradicted: `Layer::SlicePostProcess` runs strictly **before** `Layer::Perimeters`, so no extrusion path — locked or otherwise — exists yet when this module mutates region footprints. The packet conforms and authors no ADR, so no slot is allocated (the next free number is `0064` should that ever change).
- Schema/version constants and event-specific locking: **not applicable** — this packet bumps no `PROGRESS_EVENT_SCHEMA_VERSION` and no other public version constant. `[compatibility] min-ir-schema` / `max-ir-schema` in the new manifest are copied from an existing layer module's manifest, not raised.

## Code Change Surface

- **Selected approach.** Port canonical's kernel verbatim in shape into `slicer-core`, then wrap it in a thin guest module that owns only the gate, the taper and the write-back. The split matters: the kernel is pure, deterministic and testable natively without any WASM machinery (AC-1 through AC-4 run as ordinary `slicer-core` tests), while the module holds nothing but config reads and a loop, so the WASM-side surface stays small enough to reason about on a stage that has never carried a production module.
- **Exact functions, traits, manifests, tests, and fixtures.**
  - `slicer_core::algos::elephant_foot::elephant_foot_compensation(&ExPolygon, f32, f32) -> ExPolygon` and `elephant_foot_compensation_many(&[ExPolygon], f32, f32) -> Vec<ExPolygon>`. Internals mirror canonical: tiny-contour early-out on bbox extent and area; a simplify at this tree's epsilon (see the constant note below); contour resample; per-point distance query; per-point delta; banded smoothing; variable inward offset; orientation validation on the way out.
  - A file-private `SegmentGrid` inside `elephant_foot.rs` standing in for canonical's `EdgeGrid::Grid`, built at cell size `0.7 * search_radius` where `search_radius = min_contour_width_compensated + min_contour_width * 0.5`.
  - `modules/core-modules/elefant-foot/src/lib.rs` — `#[slicer_module]` implementing `run_slice_postprocess(layer_index, regions, paint, output, config)`.
  - `modules/core-modules/elefant-foot/elefant-foot.toml` — `[module] id = "com.core.elefant-foot"`, `[stage] id = "Layer::SlicePostProcess"`, `[ir-access] reads = ["SliceIR"] writes = ["SliceIR"]`, `[claims]` both empty, seven `[config.schema]` rows (two owned, five re-declared).
  - `crates/slicer-core/src/algos/mod.rs` — one ungated `pub mod elephant_foot;` declaration.
  - Tests as listed in `requirements.md` §In Scope.
  - **Epsilon constant note.** Canonical's simplify tolerance is `SCALED_EPSILON`. **That symbol is not reusable here.** This tree's only `SCALED_EPSILON` is a file-private `const SCALED_EPSILON: i128 = 1;` in `crates/slicer-core/src/smooth_outward.rs` — not `pub`, not re-exported, and a different type and magnitude from canonical's. The nearest siblings, `SCALED_EPSILON_MM` (`crates/slicer-core/src/algos/bridge_over_infill.rs`) and `SCALED_EPSILON_SQ` (`crates/slicer-core/src/medial_axis.rs`), are likewise file-private to their modules. `elephant_foot.rs` therefore declares its **own** file-private epsilon in this tree's units (1 unit = 100 nm), documents the canonical value it stands in for, and neither imports nor widens the visibility of any existing one.
- **Rejected alternatives and reasons.**
  - *Uniform `offset(-compensation)`.* Rejected: it is the exact failure mode canonical's width limiter exists to prevent, and AC-4 is written specifically to fail it.
  - *Host prepass builtin (packet 297's shape).* Rejected: 297 needed cross-layer reads; this pass does not, and the tier table assigns a module. Using the module seam also keeps the correction replaceable by a fork, which is the map's PnP-way rule 4 disposition even where no claim is minted.
  - *A new host service for variable offset (WIT change).* Rejected: the kernel is pure Rust and links directly into the guest via `slicer-core`, so a WIT change would buy nothing and would cost every guest a rebuild.
  - *Reusing `slicer_core::AabbTree`.* Rejected on inspection: it indexes 3D triangles (`AabbTree::new(IndexedTriangleSet)`, `closest_point(Point3)`), not 2D contour segments.
  - *Re-declaring the full seven-field `RoleWidthContext` key set.* Rejected: only the outer-wall precedence chain is reachable here, so four width keys are declared and the remaining `RoleWidthContext` fields are zeroed in an exhaustive literal.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/elephant_foot.rs` - role: the ported kernel and its private segment grid; expected change: new file with the mandatory porting header.
- `modules/core-modules/elefant-foot/src/lib.rs` - role: the guest module body — gate, taper, per-region write-back; expected change: new file, generated by `pnp_cli module new` then filled in.
- `modules/core-modules/elefant-foot/elefant-foot.toml` - role: stage, claims, IR access and the seven config rows; expected change: new file, scaffolded then filled in.

Extras beyond the three primaries, each a one-line mechanical edit and justified rather than split out: `crates/slicer-core/src/algos/mod.rs` (one `pub mod` line), the three new test files (they cannot be authored anywhere else), and the two wayfinder asset annotations plus the deviation row in Step 7. Splitting the packet on these would separate a declaration from its module and a test from the code it pins.

## Read-Only Context

- `crates/slicer-wasm-host/test-guests/dispatch-layer-slice-postprocess-guest/src/lib.rs` - whole file (~50 lines) - purpose: the exact `RegionKey { variant_chain, layer_index, object_id, region_id }` construction and `set_polygons(&key, &[..])` call shape on this stage.
- `crates/slicer-sdk/src/builders.rs` - the `impl SlicePostprocessBuilder` block only - purpose: `set_polygons` signature and the `polygon_updates()` accessor AC-5 asserts on.
- `crates/slicer-sdk/src/views.rs` - the `SliceRegionView` field list and accessor block only - purpose: which per-region facts are available (`polygons`, `object_id`, `region_id`, `effective_layer_height`, `variant_chain`).
- `crates/slicer-schema/wit/deps/ir-types.wit` - the `resource slice-region-view` and `resource slice-postprocess-builder` blocks only - purpose: confirm no WIT change is needed.
- `crates/slicer-core/src/flow.rs` - the `resolve_role_width`, `RoleWidthContext`, `line_width_to_spacing` and `NegativeSpacingError` definitions only - purpose: derive `min_contour_width` exactly as canonical's `Flow` overload does.
- `crates/slicer-core/src/algos/mod.rs` - whole file (~26 lines) - purpose: confirm `bridge_over_infill` is the ungated precedent before adding `elephant_foot` beside it.
- `modules/core-modules/gyroid-infill/Cargo.toml` - whole file - purpose: the `slicer-core`-in-a-guest dependency precedent.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` - the `[config.schema.support_raft_layers]`, `[config.schema.outer_wall_line_width]`, `[config.schema.line_width]`, `[config.schema.nozzle_diameter]` and `[config.schema.layer_height]` rows only (large file) - purpose: copy re-declared rows verbatim rather than re-deriving their bounds and descriptions.
- `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` - the `[module]` through `[compatibility]` header block only - purpose: the manifest header shape for a layer-stage module.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-scheduler/src/execution_plan.rs` - this packet adds no stage and changes no ordering; delegate any symbol lookup
- `crates/slicer-gcode/src/serialize.rs` - the `ORCA_CONFIG_PADDING` twin is rule-2 non-evidence and rides ticket 132; do not open it
- `modules/core-modules/skirt-brim/**` - `brim_use_efc_outline` is out of scope
- Every other `docs/spec_packets/*/` directory - never modify another packet
- `crates/slicer-core/src/arachne/**` and the `arachne_*` tests - unrelated; a stray edit there is the most likely accidental scope leak in this crate

## Expected Sub-Agent Dispatches

- Question: What is the overall pass shape of canonical `elephant_foot_compensation` — early-outs, stages in order, and the inputs each stage consumes?; scope: `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp`; return: `SUMMARY` (<=200 words); purpose: Step 1.
- Question: Return the contour resample plus per-point delta computation, and `smooth_compensation_banded`, verbatim; scope: `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp`; return: `SNIPPETS` (<=2 snippets, 30 lines each); purpose: Step 2.
- Question: In `PrintObject::slice_volumes`, what exactly gates elephant-foot compensation and how is the per-layer value computed?; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SUMMARY` (<=200 words); purpose: Step 4.
- Question: Confirm the coFloat/coInt declarations, defaults and `min` bounds of the two `elefant_foot_*` keys; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (<=5 lines); purpose: Step 3.
- Question: What is the layer-stage ownership model and what triggers the claim-system rule-4 module split?; scope: `docs/01_system_architecture.md`; return: `SUMMARY` (<=200 words); purpose: Step 3.
- Question: List every required `[config.schema]` field key and its allowed values for a `float` and an `int` row; scope: `docs/03_wit_and_manifest.md`; return: `FACT` (<=5 lines); purpose: Step 3.
- Question: Does any test target in `crates/slicer-core/Cargo.toml` named `algo_*_tdd` carry `required-features`?; scope: `crates/slicer-core/Cargo.toml`; return: `FACT` (<=5 lines); purpose: Step 2 (protects the ungated invariant).

## Data and Contract Notes

- IR/manifest contracts: `SliceIR` region polygons are replaced, never appended to. `[ir-access] reads`/`writes` are both `["SliceIR"]`. The manifest's seven config rows are the module's entire config surface; five are re-declarations of keys other modules already own, which the manifest schema permits and which `support_raft_layers` in `classic-perimeters.toml` and `arachne-perimeters.toml` establishes.
- WIT boundary: **unchanged**. `slice-postprocess-builder.set-polygons` and every `slice-region-view` accessor this packet needs already exist. If any step finds itself editing `crates/slicer-schema/wit/**`, the design is wrong — stop and re-scope.
- Determinism/scheduler constraints: the stage is layer-parallel; the module must be a pure function of `(layer_index, regions, config)` with no cross-layer or cross-region state. The kernel must not use `rayon` (both because it is layer-parallel already and because `rayon` is behind `host-algos`). Iteration order over regions follows the dispatched `regions` slice so the emitted update order is deterministic.

## Locked Assumptions and Invariants

- **Ungatedness of `slicer_core::algos::elephant_foot` is locked** for as long as a guest module calls it. Adding `#[cfg(feature = "host-algos")]` to it, or `required-features` to `algo_elephant_foot_tdd`, breaks the guest build and silently empties the narrow test run respectively.
- **The `Layer::SlicePostProcess` write contract is locked to full replacement**: `set_polygons` overwrites a region's polygons, so the module must pass the compensated union of the region's own footprints and nothing else.
- Everything else is reversible via config defaults: `elefant_foot_compensation = 0.0` makes the module a no-op, and removing the module directory removes the behaviour entirely.

## Risks and Tradeoffs

- **The kernel is the packet's real cost.** Canonical's file is ~646 lines; the borrowed pass is roughly 250 of them plus the grid. It is split across Steps 1 and 2 so neither is an L step, but it remains the most likely place to overrun. If Step 2 cannot land within an M budget, split it again at the smoothing boundary rather than simplifying the algorithm.
- **First production module on an empty stage.** Ordering, commit-merge and progress-event behaviour on `Layer::SlicePostProcess` have only ever been exercised by a test guest. Budget for one round of surprise in Step 5 and diagnose it against `dispatch-layer-slice-postprocess-guest`'s behaviour, not against another stage's.
- **`outer_wall_line_width` is `float_or_percent` with `ratio_over = nozzle_diameter` and an auto sentinel of `0`.** Resolving it wrong silently changes `min_contour_width` and therefore every AC-2/AC-4 number. Resolve it through the config view's `get_abs_value` and the `resolve_role_width` fallback chain, never by reading the raw magnitude. Ticket 128 is open on the units of exactly this key class; if its ruling lands first, re-derive rather than assume.
- **Bit-identity with canonical is not claimed** — the substituted acceleration structure and this tree's polygon tolerance make invariant tests (containment, area monotonicity, width-limited shrink) the right evidence, per the parity-evidence standard. Do not author a golden fixture.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the kernel body)
- Highest-risk dispatch and required return format: the `ElephantFootCompensation.cpp` resample-and-delta read — `SNIPPETS`, at most 2 snippets of 30 lines. Reject any reply that returns the file or paraphrases the algorithm instead of quoting it, and redispatch narrower.

## Open Questions

- `[FWD]` Does the module need to preserve `infill_areas`, `top_solid_fill`, `bottom_solid_fill`, `internal_solid_fill` and `sparse_infill_area` consistency after shrinking `polygons`? `set_polygons` replaces only the footprint, and on the first layers these derived areas are produced downstream by `PrePass::ShellClassification`, which has already run. Resolve in Step 5 by dispatching a `LOCATIONS` query for the consumers of those `SliceRegionView` fields at `Layer::Infill`, and if any of them is fed from the pre-compensation footprint, record it as a fourth clause on the deviation row rather than widening the packet.
- **Resolved at authoring (was a `[FWD]`): edition membership needs no edit.** `xtask/src/editions.rs` hardcodes no core-module list — its `load_editions_from` reads `dist/editions.toml` and validates names against `discover_guests(ws_root)` filtered to `GuestTree::Core`, so a new directory under `modules/core-modules/` is picked up automatically. `dist/editions.toml`'s `integrated_modules` lists only the three modules compiled natively for the `hybrid` edition (`classic-perimeters`, `arachne-perimeters`, `tree-support-planner`); everything else ships as a WASM guest in all three editions. `elefant-foot` therefore ships as a guest everywhere with **no** edit to either file. Ordering constraint if that is ever revisited: `validate_edition_names` rejects a name in `editions.toml` whose guest is not yet discoverable, so the module must build before it may be listed.
