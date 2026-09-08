---
status: implemented
packet: 235-external-bridge-orientation
task_ids: []
---

# 235-external-bridge-orientation

## Goal

Replace the longest-anchor-run external bridge orientation heuristic with a faithful port of canonical's active inline `detect_bridging_direction` semantics — floating-edge candidates over the gated bridge geometry, principal-component fallback, `SCALED_EPSILON` anchor expand, the ADR-0061 deterministic tie-break, and degrees-mod-180 boundary representation — so external bridge lines satisfy invariant I3.

## Problem Statement

At HEAD, external bridge orientation comes from `compute_bridge_direction_deg` (`crates/slicer-core/src/algos/mesh_analysis.rs`, private, takes `&[AnchorRun]`): the perpendicular of the longest 3D anchor-edge run. It consumes no lower-layer input and hardcodes 0.0 on degenerate input. Measured divergence (plan §2): the calicat external site emits bridge extrusion at **1.6°** where canonical OrcaSlicer emits **88.6°** — canonical reaches the near-perpendicular answer via its principal-component fallback for fully anchored areas, a mechanism our heuristic lacks entirely. Canonical's active path (the inline `detect_bridging_direction` overloads declared in `BridgeDetector.hpp`, called from `LayerRegion::process_external_surfaces`) derives floating edges by differencing the bridge boundary against `expand(anchors, SCALED_EPSILON)`, scores candidate normals by Σ|edge·normal|, picks minimal cost, returns the perpendicular, and falls back to the minor principal axis when no edge floats. This is one coherent slice: the orientation decision, its geometry provenance (gated areas from 234), and its boundary representation (D6) all change together in the seam 234 creates.

**ISSUE-84 split (explicit):** ISSUE-84 owns both P77 keys — `bridge_angle` and `counterbore_hole_bridging`. This packet covers ONLY the `bridge_angle` half (auto-detection semantics replacing the heuristic). The user-facing `bridge_angle` override-key plumbing (custom angle + relative-angle handling at the call site) and `counterbore_hole_bridging` REMAIN with ISSUE-84 for a later packet.

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
