# Task Map: 153-arachne-linejunctions-and-stitch-faithfulness

## Backlog Mapping

This packet does NOT correspond to any `TASK-###` or `T-###` in `docs/07_implementation_status.md`. It is a post-D-147 (2026-07-08) faithfulness refactor that closes two remaining PnP-vs-OrcaSlicer divergences ADR-0035 lists as requiring faithful ports. The relevant precedent packets and deviation-log entries are:

| Source | Type | Status | Relationship to Packet 153 |
| --- | --- | --- | --- |
| P141 (`.ralph/specs/141-...`) | Packet | `implemented` | Implements the `BeadingPropagation` side table and canonical `generate_junctions` rewrite (N1+N7). Packet 153's storage restructure builds on the canonical `generate_junctions` body but changes the `EdgeJunctions` type alias. |
| P142 (`.ralph/specs/142-...`) | Packet | `implemented` | Implements canonical `connectJunctions` emission + `is_odd` (N2+N4). Packet 153's storage restructure must preserve the `is_odd` predicate and the `passed_odd_edges` dedup key. |
| P146 (`.ralph/specs/146-...`) | Packet | `implemented` | Implements the canonical post-process order (N11+N12+N13). Packet 153's stitch faithfulness fixes are within the same post-process surface. |
| P147 (`.ralph/specs/147-...`) | Packet | `implemented` | Closes the N1-N13 chain (D-147-CHAIN-CLOSURE). Packet 153 is the post-147 faithfulness refactor for two functions ADR-0035 covers. |
| D-141-JUNCTION-BANDS | Deviation | closed | N1+N7 chain. Packet 153 must not regress any N1 test. |
| D-142-CONNECTJUNCTIONS-EMISSION | Deviation | closed | N2+N4 chain. Packet 153 must not regress any N2 or N4 test. |
| D-147-PARITY-AUDIT-FINDINGS | Deviation | closed | 7 findings; finding #2 ("full-chain walk with proximity-gated append") is the closest precedent to the storage restructure. |
| D-147-CHAIN-CLOSURE | Deviation | closed | N1-N13 chain closure. Packet 153 builds on this closure. |
| D-153-ARACHNE-PERIMETER-PARITY-STALE-GOLDENS | Deviation | closed | Precedent for the re-record step (Step 2 of the implementation plan). Packet 153's fixture re-record follows the same pattern. |

## Cross-Packet Test Anchors

The packet's stability anchors are the regression-locked test suites from earlier packets. Each step in the implementation plan must not regress any of these:

| Test file | Test names | Precedent packet | Packet 153 step that must not regress |
| --- | --- | --- | --- |
| `crates/slicer-core/tests/arachne_annulus_split.rs` | `annulus_outer_and_hole_are_separate_closed_loops` | P142 (implied) | Steps 1, 3, 4 |
| `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` | 3 tests | P141 (N1) | Step 1 (destructure update) |
| `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` | 3 tests | P141 (N7) | Step 1 (destructure update) |
| `crates/slicer-core/tests/generate_toolpaths.rs` | `generate_toolpaths_tapered_wedge`, `outer_wall_closes_for_simple_polygon` | P142, P147 | Steps 2, 4 |
| `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` | N1 red tests | P141 | Step 2 |
| `crates/slicer-core/tests/arachne_parity_red_perimeter_index.rs` | N2 red tests | P142 | Step 2 |
| `crates/slicer-core/tests/arachne_parity_red_is_odd_semantics.rs` | N4 red tests | P142 | Step 2 |
| `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` | N3+N8 red tests | P143 | Step 2 |
| `crates/slicer-core/tests/arachne_parity_red_chain_junctions.rs` | 7 tests | P147 | Step 2 |
| `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` | hexagon test | P145 (N9) | Step 4 (tiny-poly rule must not affect closed-passthrough) |

## Reopen / Supersede

This packet does NOT reopen or supersede any prior packet. It is a faithfulness refactor that builds on the N1-N13 chain closure (P147). The two divergences addressed are not in the N1-N13 finding list (D-147-PARITY-AUDIT-FINDINGS); they are post-chain-closure faithfulness gaps surfaced by the diagnostic that the diagnosis target is retired (`benchy.stl` retired by TASK-240, replaced by `cube_4color.3mf` in the e2e gate).

## OrcaSlicer References by Step

| OrcaSlicer file | Lines | Used in step | Purpose |
| --- | --- | --- | --- |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` | 2013-2079 | Step 1 | `generateJunctions` push order (peak-side to boundary-side) + `LineJunctions` layout |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` | 2290-2298 | Step 1 | Lazy-empty-`LineJunctions` fallback for non-upward edges |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` | 22-30 | Step 3 | `canReverse` parity gate for `VariableWidthLines` specialization |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` | 35-40 | Step 3 (context) | `canConnect` parity gate (PnP's `(inset_idx, is_odd)` grouping is structurally equivalent) |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` | 130-150 | Step 4 | `chain_length + dist < 3 * max_stitch_distance` tiny-poly rule + `chain.size() <= 2` guard |
| `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` | 71-247 | Step 4 (context) | The full `PolylineStitcher::stitch` algorithm (delegate-only; not ported) |

All OrcaSlicer references are delegate-only per the packet's Context Discipline Note and the `orca-delegation` snippet in `requirements.md` §"OrcaSlicer Reference Obligations".

## Doc Impact Cross-Reference

| Doc | Section | Edit | Verification grep |
| --- | --- | --- | --- |
| `CONTEXT.md` | §"Terms" | Delete "Junction fan" entry; add "Edge junctions" entry | `rg -q 'Edge junctions' CONTEXT.md` |
| `docs/DEVIATION_LOG.md` | §"Open / In-progress deviations" table | Add `D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS` row | `rg -q 'D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS' docs/DEVIATION_LOG.md` |
| `docs/adr/0035-arachne-faithful-emission-and-transitions.md` | §"Consequences" | Add cross-reference to packet 153 | `rg -q 'packet 153' docs/adr/0035-arachne-faithful-emission-and-transitions.md` |
