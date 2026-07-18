---
status: implemented
packet: 153-arachne-linejunctions-and-stitch-faithfulness
task_ids:
  - none
---

# 153-arachne-linejunctions-and-stitch-faithfulness

## Goal

Reduce the two remaining PnP-vs-OrcaSlicer Arachne divergences that ADR-0035's "faithful algorithm-level port" bar does not yet cover: (1) `EdgeJunctions` storage is a PnP-internal `(from_junctions, to_junctions)` split with `perimeter_index`-slot indexing + `default_extrusion_junction()` placeholders, instead of OrcaSlicer's single `LineJunctions` `Vec<ExtrusionJunction>` per edge ordered peak-side to boundary-side; (2) `stitch_extrusions` lacks OrcaSlicer's `canReverse` (even-line reversal blocking) and the `chain_length + dist < 3 * max_stitch_distance` tiny-polygon non-closure rule.

## Problem Statement

ADR-0035 (Accepted 2026-07-08) established that the Arachne emission, transitions, and post-process surface must be faithful algorithm-level ports of OrcaSlicer's C++ reference. Packet 147 (D-147-CHAIN-CLOSURE, 2026-07-08) closed the N1–N13 chain by fixing the 7 deferred parity-audit findings, but two PnP-internal divergences from OrcaSlicer's reference remain, both in functions ADR-0035 lists as requiring faithful ports.

**Divergence 1 — `EdgeJunctions` storage layout.** `crates/slicer-core/src/arachne/generate_toolpaths.rs:141` defines `type EdgeJunctions = (Vec<ExtrusionJunction>, Vec<ExtrusionJunction>)` — a PnP-internal split into `from_junctions` (at the edge's start vertex) and `to_junctions` (at the resolved-to vertex), indexed by `perimeter_index` slot with `default_extrusion_junction()` placeholders (`:484-490`) for out-of-band beads. OrcaSlicer's `generateJunctions` (`SkeletalTrapezoidation.cpp:2013-2079`, specifically `:2030-2076`) stores one `LineJunctions = std::vector<ExtrusionJunction>` per edge, ordered peak-side (high R) to boundary-side (low R), one entry per in-band bead, `perimeter_index = junction_idx` set at generation. The PnP layout forces downstream code to look up junctions by `perimeter_index` slot, requires placeholder handling, and diverges from the canonical reference that any future parity audit or re-implementation would read.

**Divergence 2 — `stitch_extrusions` post-process faithfulness.** `crates/slicer-core/src/arachne/stitch.rs:71-249` is a faithful port of OrcaSlicer's `PolylineStitcher::stitch` (matching the `(inset_idx, is_odd)` grouping at `stitch.rs:83` and the distance-only join at `stitch.rs:134`) but lacks two canonical behaviors:

- **`canReverse` (even-line reversal blocking):** OrcaSlicer's `PolylineStitcher.cpp:22-30` blocks reversing even (`!is_odd`) wall bands — even walls encode sidedness relative to their neighboring wall and must keep their orientation stable. PnP's `merge_chains` (`stitch.rs:188-214`) reverses any chain in all 4 endpoint combinations, so an even-wall fragment can be flipped to CCW when the canonical wall is CW.
- **Tiny-polygon non-closure rule:** OrcaSlicer (`PolylineStitcher.hpp:136-141`) prevents closing a chain into a polygon when its total length + closing-segment distance is `< 3 * max_stitch_distance` (it might still extend into a longer polyline) and refuses to make 2-vertex polygons (`chain.size() <= 2`). PnP's `finalize_chain` (`stitch.rs:220-241`) closes any chain whose endpoints are within `max_gap`, with no length guard, producing small spurious closed loops where OrcaSlicer would leave the chain open.

This packet is a refactor, not a bug fix. The `arachne_annulus_split` test passes today (`inset0: lines=1 closed=1 sizes=[45]`) and the N1–N13 chain is closed. The refactor brings the two divergent functions closer to their canonical implementations so future maintainers and parity audits don't have to reason about PnP-internal conventions.

## Architecture Constraints

List packet-specific architectural constraints below. For workspace invariants, include the relevant snippet verbatim (and only when applicable):

- (Include `<!-- snippet: wasm-staleness -->` bullet from `references/snippets/wasm-staleness.md` if this packet edits any path that feeds the guest WASM build. Skip if the change surface is host-only.)
  - **Skip:** this packet is host-only (`slicer-core/src/arachne/`). No path in the change surface feeds the guest WASM build. No `cargo xtask build-guests` run is required.
- (Include `<!-- snippet: coord-system -->` bullet from `references/snippets/coord-system.md` if this packet touches geometry, slicing, polygon/mesh ops, or any mm↔unit conversion. Skip for pure G-code text, config parsing, scheduler wiring, etc.)
  - **Skip:** this packet does not change coordinate-system behavior. The existing 1 unit = 100 nm convention is preserved; `UNITS_PER_MM = 10_000` is not modified; no `mm_to_units()` / `units_to_mm()` boundary is touched.
- Packet-specific constraint: The storage restructure must preserve the `perimeter_index = junction_idx` invariant from `generateJunctions:2076` (OrcaSlicer). Every junction emitted by `generate_junctions` must have `perimeter_index = idx` where `idx` is the bead index at generation, NOT a slot index. The current PnP code sets `perimeter_index = idx as u32` at `:473` (correct) but then routes junctions to the `(from_junctions[idx], to_junctions[idx])` slots (`:484-490`). The restructure drops the slot routing and relies on the Vec's push order (innermost = highest perimeter_index first, outermost = perimeter_index 0 last), matching OrcaSlicer's `:2064-2076` push order.
- Packet-specific constraint: The `canReverse` gate in `stitch_extrusions` must preserve the `max_gap` parameter's mm-unit semantics (`stitch.rs:65-66` documents the mm-unit convention). The 3-way `if (go_in_reverse_direction)` two-pass loop of OrcaSlicer's `PolylineStitcher::stitch` is NOT ported (out of scope); only the `canReverse` parity gate is added to the existing greedy pairwise merger.
- Packet-specific constraint: The `3 * max_stitch_distance` tiny-poly rule in `finalize_chain` must use the same `max_gap` value that `stitch_extrusions`'s parameter accepts (mm units, matching `Point3WithWidth`'s coordinate unit). Compute `chain_length` as the sum of Euclidean distances between consecutive junctions along the polyline (XY-only, matching `dist_sq_xy` at `stitch.rs:103-107`).

## Data and Contract Notes

- **IR or manifest contracts touched:** none. The `ExtrusionJunction` struct (its fields: `p: Point3WithWidth`, `perimeter_index: u32`) is unchanged. The `ExtrusionLine` struct is unchanged. The `edge_junctions: BTreeMap<usize, EdgeJunctions>` map is internal to `generate_toolpaths.rs` and not exposed in any WIT or manifest.
- **WIT boundary considerations:** none. The `arachne` module's WIT surface is the `run_perimeters` function in `modules/core-modules/arachne-perimeters/src/lib.rs`, which is not touched by this packet. The `arachne-params` WIT record is not modified.
- **Determinism or scheduler constraints:** the storage restructure must preserve the determinism contract (`output_a == output_b` for two independent graph builds of the same input, per `generate_toolpaths.rs test:289-293`). The single-`Vec` layout, when pushed in a deterministic order (sorted by `edge_idx` from the BTreeMap, peak-side to boundary-side), is deterministic. The `stitch_extrusions` `canReverse` gate is deterministic (rejection is a pure function of `is_odd`); the `3 * max_gap` tiny-poly rule is deterministic (length is a pure function of the chain).

## Locked Assumptions and Invariants

State the invariants the implementation must preserve. If the packet introduces no new invariants and preserves no surprising ones, write `None — change is reversible via existing config defaults; no behavior locks introduced.` Do not omit this section silently.

- **Invariant:** the `arachne_annulus_split` test's `inset0: lines=1 closed=1 sizes=[45]` output must be preserved exactly. If the storage restructure changes the per-inset line counts or junction counts, the restructure is wrong and must be revisited.
- **Invariant:** every emitted junction's `perimeter_index` equals the bead index at generation (`perimeter_index = idx as u32`, matching OrcaSlicer `:2076`). The current PnP code sets this correctly at `generate_toolpaths.rs:473`; the restructure must preserve it.
- **Invariant:** the upward-half-edge-only emission contract (AC-N1 from packet 141) is preserved. Only the upward half of a twin pair (`from.R < to.R`) gets a non-empty `EdgeJunctions` entry; the downward half, flat edges, and same-bead-count edges get explicit empty `Vec` entries (matching OrcaSlicer's lazy-empty-`LineJunctions` at `:2290-2298`).
- **Invariant:** the `is_odd` predicate (BOTH endpoints + 0.005 mm proximity, per packet 142) is preserved. The restructure does not change `is_odd_segment` / `is_odd_endpoint`'s semantic; it only changes the data they read from.
- **Invariant:** the `passed_odd_edges` dedup key (physical edge index, per packet 142 N4) is preserved. The restructure does not change `passed_odd_edges`'s type or key.
- **Invariant:** the `(inset_idx, is_odd)` grouping in `stitch_extrusions` is preserved. The `canReverse` fix is per-group, not per-line; even-line groups block reversal, odd-line groups permit it.
- **Invariant:** the AC-6 already-closed-lines-passthrough in `stitch_extrusions` (`stitch.rs:75-80`) is preserved. Closed lines are never joined or modified.

## Risks and Tradeoffs

- **Risk:** the storage restructure changes the per-bead line counts in `generate_toolpaths_tapered_wedge`, requiring a fixture re-record. The fixture is self-captured (not an OrcaSlicer golden), so re-recording is by design, but the change must be audited to confirm it's a layout change, not a behavior regression.
  - **Mitigation:** AC-4 requires the `outer_wall_closes_for_simple_polygon` test to pass after the fixture re-record. If the simple square's outer wall is now fragmented (multiple spoke fragments instead of one closed ring), the restructure changed geometry, not just storage.
- **Risk:** the `canReverse` gate in `stitch_extrusions` over-restricts even-line joins, producing more unjoined fragments than the pre-refactor version. The `arachne_annulus_split` test (AC-3) is the regression anchor; if `inset0: lines=1 closed=1 sizes=[45]` changes, the `canReverse` gate is too strict.
  - **Mitigation:** the `canReverse` gate only blocks joins that would require reversing an even chain. The annulus's outer loop closes via a `(Start, End)` merge (no reversal), so the gate doesn't affect it. The 49.33% closure residual on `cube_4color` may improve or worsen; neither is in scope for this packet.
- **Risk:** the `3 * max_gap` tiny-poly rule in `finalize_chain` leaves more chains open than the pre-refactor version. The `outer_wall_closes_for_simple_polygon` test (AC-4) requires the simple square's outer wall to close after stitching. If the rule over-rejects, the square's outer wall won't close.
  - **Mitigation:** the rule is calibrated by OrcaSlicer's canonical threshold (`3 * max_stitch_distance`); the simple square test fixture was enlarged to 10mm × 10mm (perimeter 4mm) so its outer wall is comfortably above the `3 * 0.4mm = 1.2mm` threshold. The original 0.2mm × 0.2mm square had a 0.8mm perimeter, which is correctly left open by the faithful rule (matches OrcaSlicer's tiny-poly non-closure). The fixture enlargement is a test update, not a behavior mask.
- **Risk:** the `default_extrusion_junction()` removal breaks some downstream consumer that relied on the placeholder. The placeholder was internal to `chain_junctions_for_bead` (its `from_j.get(bead)` lookup would return the placeholder for out-of-band beads); with the restructure, the lookup is direct (scan the Vec for `perimeter_index == bead`), so no downstream consumer relied on the placeholder.
  - **Mitigation:** AC-1 and AC-2 cover the test files that consume the return type. If a downstream consumer (e.g. `pipeline.rs`) breaks, `cargo check --all-targets` will surface it.
- **Risk:** the new `arachne_stitch_can_reverse.rs` and `arachne_stitch_tiny_polygon.rs` tests are added with minimal fixtures, but the unit-test construction may not exercise the real Arachne graph topology. The tests must use real `ExtrusionJunction` / `ExtrusionLine` values (not synthetic placeholders) and verify the post-stitch output's `is_closed` and junction count.
  - **Mitigation:** the new tests follow the pattern of `crates/slicer-core/tests/arachne_stitch_*.rs` if any exist; otherwise, they construct `ExtrusionLine`s directly via the struct's public fields.
