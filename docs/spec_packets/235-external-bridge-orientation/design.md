# Design: 235-external-bridge-orientation

## Controlling Code Paths

- Primary code path: `compute_bridge_direction_deg` (`crates/slicer-core/src/algos/mesh_analysis.rs`, private, takes `&[AnchorRun]`) — THIS packet's replacement surface. Its output flows into `BridgeRegion.bridge_direction_deg` via `compute_bridge_metrics`, and the region-level winner into `region.bridge_orientation_deg` inside `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`). The port lands as a pure function in `prepass_slice.rs` next to 234's gate, and the seam invocation lives in `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`), adjacent to 234's `gate_bridge_areas_by_unsupported_span` call.
- Neighboring tests/fixtures: `crates/slicer-core/tests/algo_mesh_analysis_tdd.rs` (flat, `required-features = ["host-algos"]`; exercises mesh analysis end-to-end), `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (234's net-new test, the seam's neighbor), `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (asserts on `bridge_direction_deg` values from the heuristic — Step 3 fallout), `crates/slicer-sdk/tests/test_support_slice_region_view_builder_setters_tdd.rs` (`bridge_orientation_deg` round-trips).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Port target is the ACTIVE inline path only: the `detect_bridging_direction(Lines, Polygons)` / `(Polygons, Polygons)` overload pair declared in canonical `BridgeDetector.hpp`, which is what canonical's active call path selects — `LayerRegion::process_external_surfaces` (`LayerRegion.cpp`) invokes the inline overload directly for bridge-angle assignment. The classic `BridgeDetector::detect_angle` class (`BridgeDetector.cpp`; 5° sweep, coverage cost, spacing tie-break) is a separate legacy sweep implementation that this call path does not reach, so it is an explicit rejected alternative (see below). No class port: a free pure function mirroring the inline overloads.
- **Geometry provenance [FWD-1 answer to 234]:** after 234's span gate trims `bridge_areas` by grown anchors, this packet derives floating edges from the GATED (trimmed) polygons plus the RAW previous-layer contours — i.e. floating edges = boundary of `gated_bridge_area` minus `expand(raw_lower_layer_slices, SCALED_EPSILON)`. Rationale: canonical's `(Polygons, Polygons)` overload computes `overhang_area = diff(to_cover, anchors)` then differs its boundary against `expand(anchors, SCALED_EPSILON)`. Canonical's `to_cover` at the call site is the untrimmed bridge expolygon and its `anchors_area` is `lower_layer->lslices` — so canonical itself trims by raw lower slices FIRST and uses the SAME raw slices for the ε-expand. Our pipeline's equivalent of "the bridge area that survived unsupported-span analysis" is precisely 234's gated output; re-differencing it against raw contours reproduces canonical's two-step semantics without double subtraction (the gate's grown-anchor difference removes interior overlap; the ε-expand only absorbs boundary-edge coincidence within one unit). Consuming pre-gate candidates instead would score edges canonical never sees (edges buried under the lower layer).
- **Expansion constants do not compose [FWD-2 answer to 234]:** 234 grows anchor areas 0.1 mm × up to 5 steps for GATING (deciding which area survives); this packet's SCALED_EPSILON expand (1 unit = 10⁻⁴ mm) is a separate tiny tolerance for EDGE DIFFERENCING (deciding which boundary edges float). They are applied at different stages to different ends: the gate runs once when areas are finalized; the ε-expand runs inside the orientation computation on the already-gated result against RAW contours (not the gate's grown anchors). There is no double-application: the 0.1 mm growth never enters the orientation input path, and the ε tolerance is never used to grow areas.
- Tie-break per D5 + ADR-0061 (`docs/adr/0061-deterministic-bridge-orientation-tie-break.md` — exists; cite, never recreate): among candidates whose accumulated cost equals the minimum (exact equality on the dot-product sum), choose the SMALLEST quantized angle key (`ceil(atan2(n.y, n.x) · 1000)`). This is an intentional divergence from canonical's hash-order first-wins selection, recorded in the ADR — NOT a DEVIATION_LOG row.
- Boundary representation per D6: degrees mod 180 across IR/WIT/module boundaries; radians converted ONCE at the port boundary. Canonical stores `PI + atan2(dir.y, dir.x)` radians CCW-from-X; the port returns degrees mod 180 directly and no downstream surface changes representation.
- Schema/version constants: none touched — no event wire format, no version constant, no schema bump in this packet.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The ε-expand constant is canonical `SCALED_EPSILON = scale_(EPSILON)` with `EPSILON = 1e-4` mm — which is EXACTLY 1 internal unit here, so `expand(anchors, 1.0_units)` needs no conversion arithmetic; do not rescale it by 100 again.

## Code Change Surface

- Selected approach: add a pure `detect_bridging_direction_deg(to_cover: &[ExPolygon], anchors: &[ExPolygon]) -> f32` (plus a private floating-edge helper) to `crates/slicer-core/src/algos/prepass_slice.rs`, implementing: (1) `overhang_area = difference(to_cover, anchors)`; (2) `floating_edges` = boundary polylines of `overhang_area` minus `expand(anchors, 1 unit)`; (3) if empty → principal components of `overhang_area`, return the MINOR axis (fully degenerate → `{1,0}` → 0.0°); (4) else candidate set = unique normals of floating edges (`(dy, −dx)` normalized), quantization keys `ceil(atan2·1000)`, cost = Σ|edge·normal| over all floating edges, minimal-cost winner, return the perpendicular as degrees mod 180 with the ADR-0061 smallest-quantized-angle tie-break. Add `update_external_bridge_orientation(region: &mut SlicedRegion, lower_layer_slices: &[ExPolygon])` that applies it to the gated `bridge_areas` and writes `region.bridge_orientation_deg` (no-op when empty). Wire it into `commit_shell_classification_builtin` immediately after 234's gate call. Retire `compute_bridge_direction_deg` from `mesh_analysis.rs`.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-core/src/algos/prepass_slice.rs` — add `detect_bridging_direction_deg`, `update_external_bridge_orientation`, private `floating_edges_of_gated_area` helper; remove the heuristic's write-through of `best_orientation_deg` if superseded by the seam (keep `assemble_bridge_areas` stamping behavior otherwise intact).
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `commit_shell_classification_builtin` calls `update_external_bridge_orientation(region, previous_layer_contours)` after 234's `gate_bridge_areas_by_unsupported_span`, using the same `prev_layer_boundaries.get(&global_layer_index)` lookup (map keyed by CURRENT global layer index holding previous-layer contours — semantics confirmed by the existing consumer `crates/slicer-wasm-host/src/marshal/in_.rs`).
  - `crates/slicer-core/src/algos/mesh_analysis.rs` — delete `compute_bridge_direction_deg`; `compute_bridge_metrics` keeps populating `BridgeRegion.bridge_direction_deg` only where still consumed by non-orientation surfaces (or zeroes it if no consumer remains — decided by the Step 3 LOCATIONS dispatch).
  - `crates/slicer-core/Cargo.toml` — add `[[test]] name = "bridge_orientation_tdd"` with `required-features = ["host-algos"]` (flat tests/*.rs auto-register only through this entry under feature gating).
  - `crates/slicer-core/tests/bridge_orientation_tdd.rs` — net-new flat test home: AC-1..AC-6, AC-N1, AC-N2 fixtures built purely from polygon primitives.
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` — update assertions pinned to heuristic angles (`bridge_direction_deg ≈ 120°`, `within ±2° of 30.0` sites) to the ported semantics or to explicit pre-port expectations.
- Rejected alternatives and reasons:
  - Porting the legacy `BridgeDetector::detect_angle` class (5° sweep, coverage cost, spacing tie-break) — rejected: a separate legacy sweep implementation that the ACTIVE call path never reaches (`LayerRegion::process_external_surfaces` selects the inline `detect_bridging_direction` overload pair for bridge-angle assignment); AC-N2 guards against its accidental reintroduction into our tree.
  - Keeping/refining the stash's floating-edge heuristic (edge-direction candidates, no overhang pre-difference, no PC fallback, first-wins ties; measured 0° on calicat; pops at the FIRST implementation session per D10) — rejected: DISCARDED and replaced wholesale by this port; its candidate direction convention (edge directions, not normals) and missing fallbacks are exactly the measured divergence.
  - Computing orientation during `PrePass::Slice` inside `assemble_bridge_areas` — rejected: same scheduler constraint 234 hit (lower-layer data not committed while layers slice in parallel); the gated geometry does not exist until `PrePass::ShellClassification`.
  - A new `f32` radians field or WIT type for bridge direction — rejected: D6 fixes the boundary representation at degrees mod 180; the existing `bridge_orientation_deg` surfaces suffice.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/prepass_slice.rs` - role: primary change surface (pure port + region updater); expected change: add three functions, adjust `assemble_bridge_areas`'s orientation write-through.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: seam wiring; expected change: invoke `update_external_bridge_orientation` post-gate in `commit_shell_classification_builtin`.
- `crates/slicer-core/tests/bridge_orientation_tdd.rs` (net-new) - role: test home for the port; expected change: AC-1..AC-6, AC-N1, AC-N2.
- `crates/slicer-core/Cargo.toml` - role: register the net-new test target; expected change: `[[test]]` entry.
- `crates/slicer-core/src/algos/mesh_analysis.rs` - role: heuristic retirement; expected change: remove `compute_bridge_direction_deg` (Step 3 only).
- `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` - role: blast-radius fallout; expected change: re-pin angle assertions.

## Read-Only Context

- `crates/slicer-core/src/algos/prepass_slice.rs` - lines 197-256 and 225-275 only - purpose: current `assemble_bridge_areas` body and `best_orientation_deg` write-through.
- `crates/slicer-core/src/algos/mesh_analysis.rs` - lines 486-510 and 640-690 only - purpose: `compute_bridge_direction_deg` signature/body and its sole caller `compute_bridge_metrics` (retirement blast radius).
- `crates/slicer-ir/src/slice_ir.rs` - lines 599-693 only - purpose: `BridgeRegion.bridge_direction_deg` field shape and `SurfaceClassificationIR.prev_layer_boundaries`.
- `docs/specs/bridge-parity-plan.md` - §2 table, §3/F2 bullets, §6 invariant list only - purpose: measured baseline and I3/I4/I7 wording.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-core/src/algos/bridge_over_infill.rs` (233's net-new module) - prerequisite, not this packet's surface; do not edit
- `modules/core-modules/*/src/**` - consume `bridge_orientation_deg` through the region view (`rectilinear-infill/src/lib.rs` reads it); routing changes are out of bounds
- `crates/slicer-schema/wit/**` - no WIT change permitted (D6 keeps degrees mod 180)

## Expected Sub-Agent Dispatches

- Question: exact bodies of both inline `detect_bridging_direction` overloads in `BridgeDetector.hpp` (candidate construction, quantization keys, cost accumulation, perpendicular flip, PC fallback ordering) and the `PI + atan2` storage line in `LayerRegion.cpp`'s `process_external_surfaces`; scope: `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` + `LayerRegion.cpp`; return: `SUMMARY` (≤200 words) + `LOCATIONS` (≤20 entries); purpose: Step 1 port fidelity.
- Question: `compute_principal_components` behaviour contracts (zero-area guard, axis sort order, EPSILON covariance branch); scope: `OrcaSlicerDocumented/src/libslic3r/PrincipalComponents2D.cpp`; return: `SUMMARY` (≤150 words); purpose: Step 1 minor-axis fallback.
- Question: every caller/consumer of `compute_bridge_direction_deg` and every test asserting literal `bridge_direction_deg`/`bridge_orientation_deg` values (including visual-debug captures snapshotting direction fields and golden/parity baselines keyed on orientation); scope: `crates/*/src` + `crates/*/tests` + `modules/*/src`; return: `LOCATIONS` (≤20 entries); purpose: Step 3 retirement fallout list.
- Question: confirm `detect_angle`'s distinguishing markers (angle step constant, coverage cost, spacing tie-break) in `BridgeDetector.cpp` so AC-N2's rg patterns name real sweep constructs, and confirm the ACTIVE call path (`LayerRegion::process_external_surfaces`) selects the inline `detect_bridging_direction` overload rather than `detect_angle`; scope: `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` + `LayerRegion.cpp`; return: `SNIPPETS` (≤30 lines); purpose: Step 3 negative-guard grounding.

## Data and Contract Notes

- IR/manifest contracts: no new IR field; `BridgeRegion.bridge_direction_deg` and `region.bridge_orientation_deg` keep their types and units (degrees mod 180). No manifest/TOML key added (`bridge_angle` override is out of scope).
- WIT boundary: unchanged — `bridge_orientation_deg` crosses as degrees today and stays degrees (D6).
- Determinism/scheduler constraints: the port is pure polygon math with a total order on candidates (cost, then quantized-angle key), so output is reproducible run-to-run and build-to-build (ADR-0061 consequence). The seam runs inside sequential `PrePass::ShellClassification` reading the committed `SliceIR`; no new cross-layer dependency and no `STAGE_ORDER` change beyond 234's.

## Locked Assumptions and Invariants

- Applicable invariant (D9): **I3** — external bridge lines run within ±5° of perpendicular to floating edges, or of the minor principal axis when fully anchored. Regression guards: I4/I7 untouched surfaces must not regress (I7 guarded by AC-7's uniform-feedrate check; I4 belongs to 233 but shares the `bridge_orientation_deg` plumbing).
- The tie-break rule is LOCKED by ADR-0061: smallest quantized angle on exact cost equality. Do not substitute epsilon-tolerant cost comparison or stable-hash ordering.
- Geometry provenance is LOCKED by the FWD-1 resolution above: gated (trimmed) `bridge_areas` + raw previous-layer contours; revisiting requires reopening 234's design, not a silent local choice.
- The stash's heuristic is LOCKED OUT: D10 pops the stash at the first implementation session, and its orientation heuristic is discarded (plan §5 salvage map), not merged.

## Risks and Tradeoffs

- Blast radius of the changed orientation value: (a) `assemble_bridge_areas`'s `best_orientation_deg` write-through and `compute_metrics` consumers in `mesh_analysis.rs`; (b) `rectilinear-infill` module reads `bridge_orientation_deg()` from the region view — emitted bridge line angles change on external sites; (c) visual-debug captures that snapshot direction-bearing fields may shift pixels; (d) golden/canonical-parity baselines keyed on orientation output. All four enumerated; Step 3's LOCATIONS dispatch closes the list before retirement.
- Watched-type struct literals: `BridgeRegion` (≥5 named fields, `crates/slicer-ir/src/slice_ir.rs`) literals in tests need a `..` rest or an `// exhaustive:` waiver; `cargo xtask check-literals` is a listed gate and runs as the preflight of `cargo xtask test`.
- The PC fallback introduces eigendecomposition numerics absent today; a naive implementation can emit near-but-not-exactly axis-aligned angles on symmetric fixtures, breaking exactness assertions. Mitigation: mirror canonical's covariance-EPSILON branch (axis-aligned shortcut) and assert with the fixture-symmetry exactness AC-3 relies on.
- Removing the heuristic while `bridge_detector_tdd.rs` still asserts heuristic angles will fail loudly — acceptable (TDD signal), sequenced last in Step 3.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1: port fidelity + numeric fixtures)
- Highest-risk dispatch and required return format: the Step 1 overload-body SUMMARY+LOCATIONS dispatch — a misread perpendicular-flip or quantization detail silently inverts the returned axis.

## Open Questions

None `[FWD]` — both of 234's forward questions are answered above (geometry provenance; expansion constants do not compose). None `[BLOCK]`.
