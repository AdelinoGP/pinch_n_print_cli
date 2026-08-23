# Requirements: 234-bridge-false-site-gating

## Packet Metadata

- Grouped task IDs: none (no ISSUE/TASK file exists for this backlog slot)
- Backlog source: `docs/specs/bridge-parity-plan.md` §4 W-A row (new packet, no prior owner)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

At HEAD, bridge material floods every layer. Measured baseline (plan §2, commit `9048cd37`): ~160 of 174 layers carry Bridge-type extrusion on calicat versus canonical's exactly 2 sites; total bridge-labelled extrusion 7924.9 mm versus 950.6 mm. The root cause is `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`), which stamps mesh-derived bridge candidates onto any layer whose cross-section intersects the facet footprint, without canonical's unsupported-span test. The region-partition precedence `bridge > bottom > top > sparse` (`crates/slicer-runtime/src/region_partition.rs`) then claims those areas from infill roles, so false bridge sites displace genuine bottom/top/sparse fill on nearly every layer.

This packet is the second of three in the bridge-parity sequence (D3: internal-first → false-site gating → external orientation). It closes the false-site/classification gap (F1) by porting canonical's unsupported-span test — bridge area minus the UNGROWN lower-layer contours (canonical `voids = diff(voids, *lower_layer_covered)`) — and demotes the existing mesh-validity filter to at most a cheap pre-filter. It builds on top of 233's seam: after both land, false-site suppression and internal-bridge construction coexist.

## In Scope

- A new host-side function `gate_bridge_areas_by_unsupported_span(region: &mut SlicedRegion, lower_layer_slices: Option<&[ExPolygon]>)` in `crates/slicer-core/src/algos/prepass_slice.rs` that computes `bridge area −` ungrown lower-layer contours (the canonical unsupported-span test); it does not perform expansion-zone growth.
- Wiring the gating into the post-slice `PrePass::ShellClassification` host built-in (`commit_shell_classification_builtin`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`), reading the same-object lower-layer region `polygons` from the already-committed `SliceIR` — no new scheduler data dependency (see Q3 resolution in `design.md`).
- The pre-filter decision: retain `BridgeRegion.is_valid` (min-length + anchor-width pass/fail) as a cheap pre-filter, with a recorded measurement plan to justify or discard it (see `design.md` §"Locked Assumptions and Invariants").
- A net-new flat test file `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (plus its `[[test]]` entry with `required-features = ["host-algos"]` in `crates/slicer-core/Cargo.toml`) hosting AC-1, AC-2, AC-N1, AC-N2.
- Blast-radius fallout: `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (which calls `assemble_bridge_areas` directly and asserts non-empty `bridge_areas`) — **outcome (2026-08-23):** the two non-empty call sites pass unchanged (the gate runs post-slice). The file's 3 pre-existing failures at HEAD were CLOSED as part of this packet (stamper empty-`facet_indices` guard, `is_valid` filter in `classify_region_surfaces`, gate `is_bridge` reset + test update). Any golden/parity baseline asserting the current flooded behaviour was enumerated and found to use self-constructed fixtures (unaffected); the one golden e2e failure (`slicing_precision_integration_tdd::legacy_zero_matches_golden`, an M73 progress-line byte mismatch) is stash-verified pre-existing and unrelated to bridges.

## Out of Scope

- External-bridge orientation (F2) — packet 235 (`detect_bridging_direction` floating-edge/PC-fallback port, ADR-0061 tie-break, D6 degrees-mod-180).
- Internal bridge-over-infill relocation (F3/F8) — packet 233 (`InternalBridgeInfill` enum variant, `bridge_over_infill` module, `determine_bridging_angle`/`construct_anchored_polygon`).
- Coverage/anchoring parity (F4) — its primitives (`construct_anchored_polygon`, expansion-zone growth constants) are delivered in 233/235; this packet's span test subtracts UNGROWN lower-layer contours and consumes no expansion-zone growth.
- Flow/speed role correctness (F5/F6) and the sparse ±90° alternation (F7) — 233.
- Fan handling and label naming (F9) — bundled elsewhere.
- Any change to the region-partition precedence order itself (verified correct at HEAD).

## Authoritative Docs

- `docs/04_host_scheduler.md` - 1673 lines; delegate a SUMMARY of §"Fixed Stage Order" and §"PrePass Execution" (the `OverhangAnnotation`/`ShellClassification`/`prev_layer_boundaries` facts).
- `docs/08_coordinate_system.md` - direct read (coordinate/expansion-constant conversion).
- `docs/specs/bridge-parity-plan.md` - direct read (source plan).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::process_external_surfaces` (both overloads): the unsupported-area analysis flow, the `stBottomBridge` surface-type assignment, the `voids = diff(voids, *lower_layer_covered)` removal of lower-layer-supported voids, and the `detect_bridging_direction(to_polygons(initial), to_polygons(lower_layer->lslices))` call that is the canonical "bridge area minus anchor areas" test.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — the `detect_bridging_direction(const Polygons &to_cover, const Polygons &anchors_area)` overload and its floating-edge computation (polyline difference of the bridge boundary against `expand(anchors, SCALED_EPSILON)`), the precise port target for the unsupported-span test.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — `BridgeDetector::unsupported_edges` (`diff_pl(to_polylines(bridge_expolygon), grown_lower)`), the concrete "bridge area minus lower-layer anchors" geometry (its `grown_lower` is a single offset by extrusion spacing for direction detection; this packet's span test subtracts UNGROWN contours per the dead-overload `voids = diff(voids, *lower_layer_covered)`).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (solid-underneath → zero bridge area), `AC-2` (unsupported span → bridge area retained), `AC-3` (bridge.obj site exists + no flooding), `AC-4` (I6 pairwise disjoint), `AC-5` (overhang.obj first/second-layer zero bridge + site exists + no flooding).
- Negative: `AC-N1` (fully-supported candidate rejected to zero), `AC-N2` (ungated candidates cannot silently return).
- Cross-packet impact: 233's `InternalBridgeInfill` role and `bridge_over_infill` module are prerequisites, not touched here. 235 consumes the gated `bridge_areas` as its orientation input — the gating must not change the geometry of surviving bridge sites, only which sites survive.
- Model oracle (AC-5): `resources/overhang.obj` was translated into the printable bed (XY −1346.56/−564.40, Z −17.0; the slicer requires models to start at Z=0 and has no auto-centering). It is a two-level model: a solid base and an overhanging top. The lowest layer (base) has no lower layer, and the second-lowest layer's lower layer is the solid base, so both must carry zero Bridge-type extrusion; the overhang top carries the known unsupported site. The structural assertions (first/second layer zero bridge, site exists, no flooding) are unchanged.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd` | AC-1/AC-2/AC-N1/AC-N2 unit gating | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-runtime --test integration -- region_partition_tdd::ac2_precedence_pairwise_disjoint_under_partial_overlap --nocapture` | AC-4 I6 disjointness | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --output target/bridge_false_site.gcode --module-dir modules/core-modules && python3 -c "…" target/bridge_false_site.gcode` | AC-3 e2e site-existence + no-flooding | FACT `bridge_layers=N/M z=[…]` |
| `cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/overhang_false_site.gcode --module-dir modules/core-modules && python3 -c "…" target/overhang_false_site.gcode` | AC-5 e2e site-existence + no-solid-underneath | FACT `bridge_layers=N/M z=[…]` |
| `cargo xtask build-guests --check` | guest freshness after any `slicer-core` edit | exit 0 fresh / 1 stale / 3 missing wasm-tools |
| `cargo xtask check-literals` | struct-literal churn gate | exit 0 / violation list |

## Step Completion Expectations

- The gating function must be pure (no blackboard access): it takes `&mut SlicedRegion` and `Option<&[ExPolygon]>` lower-layer slices, so it is unit-testable in `slicer-core` without a running host. A missing value means no lower layer and clears candidates; a present empty slice list means an existing empty lower layer and retains candidates.
- The post-slice wiring step must not change the `STAGE_ORDER` or add a manifest `[stage]`/`[ir-access]`/`[claims]` entry — the gating rides the existing `PrePass::ShellClassification` host built-in.
- The pre-filter measurement (keep/discard) is recorded in the packet's completion notes, not deferred to a later packet.

## Context Discipline Notes

- `docs/04_host_scheduler.md` is 1673 lines — delegate a SUMMARY, never load it whole.
- `OrcaSlicerDocumented/` is out-of-bounds for direct loads — delegate per the orca-delegation snippet.
- `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` is large (1000+ lines) — ranged reads only; the `assemble_bridge_areas` call sites are at the `bridge_areas must be non-empty` assertions.
