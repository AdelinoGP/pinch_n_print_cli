# Requirements: 235-external-bridge-orientation

## Packet Metadata

- Grouped task IDs: none (this packet has no `docs/07_implementation_status.md` task ID; the backlog slot is ISSUE-84)
- Backlog source: `docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

At HEAD, external bridge orientation comes from `compute_bridge_direction_deg` (`crates/slicer-core/src/algos/mesh_analysis.rs`, private, takes `&[AnchorRun]`): the perpendicular of the longest 3D anchor-edge run. It consumes no lower-layer input and hardcodes 0.0 on degenerate input. Measured divergence (plan §2): the calicat external site emits bridge extrusion at **1.6°** where canonical OrcaSlicer emits **88.6°** — canonical reaches the near-perpendicular answer via its principal-component fallback for fully anchored areas, a mechanism our heuristic lacks entirely. Canonical's active path (the inline `detect_bridging_direction` overloads declared in `BridgeDetector.hpp`, called from `LayerRegion::process_external_surfaces`) derives floating edges by differencing the bridge boundary against `expand(anchors, SCALED_EPSILON)`, scores candidate normals by Σ|edge·normal|, picks minimal cost, returns the perpendicular, and falls back to the minor principal axis when no edge floats. This is one coherent slice: the orientation decision, its geometry provenance (gated areas from 234), and its boundary representation (D6) all change together in the seam 234 creates.

**ISSUE-84 split (explicit):** ISSUE-84 owns both P77 keys — `bridge_angle` and `counterbore_hole_bridging`. This packet covers ONLY the `bridge_angle` half (auto-detection semantics replacing the heuristic). The user-facing `bridge_angle` override-key plumbing (custom angle + relative-angle handling at the call site) and `counterbore_hole_bridging` REMAIN with ISSUE-84 for a later packet.

## In Scope

- Port of the active inline `detect_bridging_direction(Lines, Polygons)` semantics as a pure host function: floating-edge candidate normals (`(dy, −dx)`), `ceil(atan2·1000)` quantization keys, cost = Σ|edge·normal| over all floating edges, minimal-cost winner, return of the perpendicular.
- The `(Polygons, Polygons)` overload's geometry: `overhang_area = difference(to_cover, anchors_area)`, floating edges = polyline difference of the overhang-area boundary against `expand(anchors_area, SCALED_EPSILON)` with SCALED_EPSILON = 1 unit = 10⁻⁴ mm.
- Principal-component fallback for the empty-floating-edge case: return the minor axis; fully degenerate → `{1,0}` → 0.0°.
- ADR-0061 deterministic tie-break: equal accumulated cost resolves to the SMALLEST quantized angle (intentional divergence from canonical first-wins; recorded in the ADR, NOT in DEVIATION_LOG).
- D6 boundary representation: degrees mod 180 across IR/WIT/module boundaries; canonical radians (`PI + atan2`) converted once at the port boundary.
- Wiring the new computation into 234's post-gate seam so `bridge_orientation_deg` is derived from gated geometry, and retiring the mesh-stage heuristic function.
- Net-new test binary `crates/slicer-core/tests/bridge_orientation_tdd.rs` ([[test]] with `required-features = ["host-algos"]`).

## Out of Scope

- `counterbore_hole_bridging` — stays with ISSUE-84 (later packet).
- User-facing `bridge_angle` override config key, manifest schema entries, custom-angle/relative-angle plumbing (canonical's `custom_angle_deg` branches) — later packet under ISSUE-84.
- The legacy `BridgeDetector::detect_angle` class (5° sweep, coverage cost, spacing tie-break) — explicitly rejected alternative; not ported.
- Internal bridge-over-infill construction, `determine_bridging_angle` windowed mean, anchored-polygon generation (233).
- The unsupported-span gate itself and anchor growth for gating (234); this packet only consumes their outputs.
- Flow/speed role changes (F5/F6), fan handling (F9), sparse ±90° alternation (233/D11).
- Any IR/WIT schema change or new scheduler stage.

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` - ~270 lines; direct read (source plan: §2 baseline table, §3/F2 finding, §4 W-B row, §6 invariant list, decisions D5/D6/D9/D10).
- `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - short; direct read (the tie-break contract implemented here).
- `docs/08_coordinate_system.md` - over 300 lines; delegated SUMMARY limited to the mm↔unit conversion rules needed for the ε-expand constant.
- `docs/19_visual_debug.md` - delegated SUMMARY of the visual-debug bundle manifest sections only if Step 3's capture re-check finds direction-bearing fields (see design.md Risks).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — active inline `detect_bridging_direction` overload pair (floating-edge candidates, quantization, cost sum, perpendicular flip, PC minor-axis and `{1,0}` fallbacks); the port target.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `process_external_surfaces` call site and the `PI + atan2(dir.y, dir.x)` radians storage convention converted once at the port boundary (D6).
- `OrcaSlicerDocumented/src/libslic3r/PrincipalComponents2D.cpp` — `compute_principal_components` moment accumulation and zero-area guard behind the fully-anchored fallback.
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — `EPSILON = 1e-4` / `SCALED_EPSILON = scale_(EPSILON)`: the anchor-expansion tolerance for edge differencing.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — `BridgeDetector::detect_angle` legacy sweep class: a separate implementation that the ACTIVE call path does not reach (`LayerRegion::process_external_surfaces` selects the inline `detect_bridging_direction` overloads) and is deliberately NOT borrowed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`. Measurable refinements absent from their Given/When/Then text: AC-1..AC-5, AC-N1, AC-N2 fixtures must be constructed purely from polygon primitives (no mesh fixture dependency) so failures localize to the port, not to fixture generation; AC-N1/AC-N2 use the equal-cost cross fixture (exact floating-point ties), not a rotated diamond; AC-6's rg check accepts name-resolution-equivalent forms (the call may appear via a `prepass_slice::` path-qualified reference rather than a bare identifier).
- Negative: `AC-N1` through `AC-N2`.
- Cross-packet impact: consumes 234's `gate_bridge_areas_by_unsupported_span` output shape (`region.bridge_areas` trimmed by grown anchors) and previous-layer contours; leaves I2/I7 surfaces untouched (guarded by AC-7); retires a symbol that `algo_mesh_analysis_tdd` currently transitively exercises (guarded by AC-8).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd` | All port-semantics ACs (AC-1..AC-6, AC-N1, AC-N2) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -c 'rg -q "fn compute_bridge_direction_deg" crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 \|\| exit 0' && cargo test -p slicer-core --features host-algos --test algo_mesh_analysis_tdd` | Heuristic retired from source (AC-8 structural half) AND mesh suite intact (AC-8 test half) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_a.gcode --module-dir modules/core-modules && cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_b.gcode --module-dir modules/core-modules && cmp target/orient_a.gcode target/orient_b.gcode` plus the identical python M83 parser from AC-7 (byte-identical files make one parse sufficient) | Determinism + I2/I7 regression guards end-to-end (AC-7) | FACT pass/fail; printed bridge-layer count + feedrate set |
| `cargo check --workspace --all-targets` | Gate: every target compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Gate: lint-clean including tests/benches | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate (watched types touched by blast radius) | exit 0 |
| `cargo xtask build-guests --check` | Guest freshness after stash-pop + slicer-core edits (exit 0 fresh / 1 stale / 3 missing wasm-tools) | FACT exit code |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step ordering is load-bearing: Step 1 (pure port + tests) must land before Step 2 (seam wiring) so the wiring step consumes an already-falsifiable function; Step 3 (retirement + fallout) runs last because AC-8's rg check fails until the heuristic is removed.
- The stash pop (D10) happens at the FIRST implementation session before Step 1: the stashed floating-edge heuristic (edge-direction candidates, no overhang pre-difference, no PC fallback, first-wins ties) is DISCARDED/replaced by this port — do not merge it into Step 1's implementation (recorded under design.md rejected alternatives).
- After any stash pop or slicer-core edit, `cargo xtask build-guests --check` exit codes arbitrate guest freshness (0 fresh / 1 stale / 3 missing wasm-tools infra error) before any guest-touching test failure is attributed elsewhere.

## Context Discipline Notes

- `docs/specs/bridge-parity-plan.md` and `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` are the only large planning reads; everything else about canonical behaviour arrives via the delegation snippet's dispatches — never open `OrcaSlicerDocumented/` directly.
- Tempting read to skip: `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` is large; read only the `compute_bridge_direction_deg`/`assemble_bridge_areas` assertion sites flagged by the Step 3 LOCATIONS dispatch.
- Heavy-dispatch return limits: OrcaSlicer dispatches are capped at `LOCATIONS` ≤20 entries / `SUMMARY` ≤200 words / snippets ≤30 lines per the delegation contract.

None packet-specific beyond the above.
