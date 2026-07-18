# Design: 153-arachne-linejunctions-and-stitch-faithfulness

## Controlling Code Paths

- **Primary code path:** `crates/slicer-core/src/arachne/generate_toolpaths.rs::generate_toolpaths` (`:903-1035`) calls `generate_junctions` (`:231-495`) to build `edge_junctions: BTreeMap<usize, EdgeJunctions>`, then walks each domain collecting `full_chain: Vec<usize>` (`:946-1017`), then calls `emit_chain_lines` (`:711-803`) per domain with `chain_junctions_for_bead` (`:554-597`) to produce per-bead polylines and `is_odd_segment` / `is_odd_endpoint` (`:609-694`) to compute `is_odd`. The storage restructure changes the type of every entry in `edge_junctions` from `(Vec, Vec)` to `Vec` and updates the lookups in the four downstream functions.
- **Primary code path (stitch):** `crates/slicer-core/src/arachne/stitch.rs::stitch_extrusions` (`:71-99`) groups open lines by `(inset_idx, is_odd)` (`:83`), calls `stitch_group` (`:118-158`) which calls `merge_chains` (`:188-214`) and `pick_better` (`:163-180`), then `finalize_chain` (`:220-241`) closes chains whose endpoints are within `max_gap`. The stitch faithfulness fixes thread `is_odd` into `stitch_group` and add the `canReverse` gate to `merge_chains`; add `chain_length` and the `3 * max_gap` guard to `finalize_chain`.
- **Neighboring tests or fixtures:**
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — 3 tests, destructures `(from_j, _to_j)` at `:176` and `:280` and `:388`; must update to single `Vec`.
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` — 3 tests, destructures `(from_junctions, to_junctions)` at `:186`; must update to single `Vec`.
  - `crates/slicer-core/tests/arachne_annulus_split.rs` — 1 test, uses `generate_toolpaths` (public entry); passes through the storage restructure without API change. Stability anchor: `inset0: lines=1 closed=1 sizes=[45]`.
  - `crates/slicer-core/tests/generate_toolpaths.rs` — 2 tests, uses `generate_toolpaths`; `outer_wall_closes_for_simple_polygon` (`:397-446`) passes through. `generate_toolpaths_tapered_wedge` (`:276-375`) may need fixture re-record if per-bead line counts shift.
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` — self-captured baseline, re-recorded by deleting and re-running `generate_toolpaths_tapered_wedge` (per `generate_toolpaths.rs test:201-274`).
  - New: `crates/slicer-core/tests/arachne_stitch_can_reverse.rs` — covers AC-6 + AC-N2.
  - New: `crates/slicer-core/tests/arachne_stitch_tiny_polygon.rs` — covers AC-7 + AC-N3.
- **OrcaSlicer comparison surface:** see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load). Do not restate the delegation rules here.

## Architecture Constraints

List packet-specific architectural constraints below. For workspace invariants, include the relevant snippet verbatim (and only when applicable):

- (Include `<!-- snippet: wasm-staleness -->` bullet from `references/snippets/wasm-staleness.md` if this packet edits any path that feeds the guest WASM build. Skip if the change surface is host-only.)
  - **Skip:** this packet is host-only (`slicer-core/src/arachne/`). No path in the change surface feeds the guest WASM build. No `cargo xtask build-guests` run is required.
- (Include `<!-- snippet: coord-system -->` bullet from `references/snippets/coord-system.md` if this packet touches geometry, slicing, polygon/mesh ops, or any mm↔unit conversion. Skip for pure G-code text, config parsing, scheduler wiring, etc.)
  - **Skip:** this packet does not change coordinate-system behavior. The existing 1 unit = 100 nm convention is preserved; `UNITS_PER_MM = 10_000` is not modified; no `mm_to_units()` / `units_to_mm()` boundary is touched.
- Packet-specific constraint: The storage restructure must preserve the `perimeter_index = junction_idx` invariant from `generateJunctions:2076` (OrcaSlicer). Every junction emitted by `generate_junctions` must have `perimeter_index = idx` where `idx` is the bead index at generation, NOT a slot index. The current PnP code sets `perimeter_index = idx as u32` at `:473` (correct) but then routes junctions to the `(from_junctions[idx], to_junctions[idx])` slots (`:484-490`). The restructure drops the slot routing and relies on the Vec's push order (innermost = highest perimeter_index first, outermost = perimeter_index 0 last), matching OrcaSlicer's `:2064-2076` push order.
- Packet-specific constraint: The `canReverse` gate in `stitch_extrusions` must preserve the `max_gap` parameter's mm-unit semantics (`stitch.rs:65-66` documents the mm-unit convention). The 3-way `if (go_in_reverse_direction)` two-pass loop of OrcaSlicer's `PolylineStitcher::stitch` is NOT ported (out of scope); only the `canReverse` parity gate is added to the existing greedy pairwise merger.
- Packet-specific constraint: The `3 * max_stitch_distance` tiny-poly rule in `finalize_chain` must use the same `max_gap` value that `stitch_extrusions`'s parameter accepts (mm units, matching `Point3WithWidth`'s coordinate unit). Compute `chain_length` as the sum of Euclidean distances between consecutive junctions along the polyline (XY-only, matching `dist_sq_xy` at `stitch.rs:103-107`).

## Code Change Surface

- **Selected approach:**
  - **Storage restructure:** replace `type EdgeJunctions = (Vec<ExtrusionJunction>, Vec<ExtrusionJunction>)` with `type EdgeJunctions = Vec<ExtrusionJunction>`. In `generate_junctions`, push junctions in OrcaSlicer's high-R-to-low-R order with `perimeter_index = junction_idx`. Insert explicit empty `Vec` entries for non-upward / flat / same-bead-count edges (matching OrcaSlicer lazy-empty-`LineJunctions` at `:2290-2298`). Update `chain_junctions_for_bead` to look up the junction for bead `b` by scanning the Vec for the entry whose `perimeter_index == b` (or by computing the slot as `len - 1 - b` if the Vec is guaranteed contiguous, which it is in the new layout). Update `is_odd_segment` and `is_odd_endpoint` to read from the single Vec.
  - **Stitch `canReverse`:** add an `is_odd` parameter to `stitch_group` and `merge_chains` (currently implicit via the group key). When `!is_odd`, only allow `(End, Start)` and `(Start, End)` merges — reject candidates that would require reversing an even chain.
  - **Stitch tiny-poly rule:** add `chain_length` computation to `finalize_chain` (sum of segment distances). Add the `chain_length + closing_dist < 3 * max_gap` and `junctions.len() <= 2` guards to the closure decision.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`: `EdgeJunctions` type alias (`:141`), `default_extrusion_junction` (`:176-188`, delete), `generate_junctions` (`:231-495`, restructure push order + insert empty entries for non-upward edges), `chain_junctions_for_bead` (`:554-597`, single-Vec lookup), `is_odd_segment` (`:609-644`, single-Vec lookup), `is_odd_endpoint` (`:647-694`, single-Vec lookup), `emit_chain_lines` (`:711-803`, single-Vec lookup).
  - `crates/slicer-core/src/arachne/stitch.rs`: `stitch_extrusions` (`:71-99`, pass `is_odd` to `stitch_group`), `stitch_group` (`:118-158`, accept `is_odd`), `merge_chains` (`:188-214`, reject reversal for even lines), `finalize_chain` (`:220-241`, add `chain_length` + `3 * max_gap` guard).
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs`: update destructuring at `:176`, `:280`, `:388` from `(from_j, _to_j)` to single `Vec`. Update assertion text (e.g. "from_j non-empty" → "Vec non-empty"). Update per-test width checks (`from_j[0].p.width` → `junctions[0].p.width`).
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs`: update destructuring at `:186` from `(from_junctions, to_junctions)` to single `Vec`. Update assertion text.
  - New `crates/slicer-core/tests/arachne_stitch_can_reverse.rs`: feed two even lines (reversal required to join) and two odd lines (reversal required to join), assert even unjoined and odd joined.
  - New `crates/slicer-core/tests/arachne_stitch_tiny_polygon.rs`: feed one even line with total length < 3*max_gap, assert `is_closed = false`. Feed one even line with total length >= 3*max_gap, assert `is_closed = true`.
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json`: re-record by deleting and re-running `generate_toolpaths_tapered_wedge` (self-captured baseline).
  - `CONTEXT.md`: delete "Junction fan" entry (`:265-269` in the current file), add "Edge junctions" entry.
  - `docs/DEVIATION_LOG.md`: add `D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS` row in the "Open / In-progress deviations" table.
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md`: add cross-reference to packet 153 in §"Consequences".
- **Rejected alternatives that were considered and why they were not chosen:**
  - **Port OrcaSlicer's `addToolpathSegment` per-quad per-bead emission model verbatim (with proximity-gated append).** Rejected: D-147 finding #2 ("full-chain walk with proximity-gated append") was implemented in packet 147 as the post-sub-run-split `emit_chain_lines`; the per-quad model would require restructuring `emit_chain_lines` to a per-quad loop and is a deeper refactor than this packet's faithfulness bar requires. The current `emit_chain_lines` matches the spirit of the canonical algorithm (full-chain walk, one polyline per bead, `is_odd` predicate, `passed_odd_edges` dedup) and the `arachne_annulus_split` test passes. Defer the per-quad model to a future packet if parity audits demand it.
  - **Restructure `EdgeJunctions` to use OrcaSlicer's `LineJunctions` name (rename the type alias).** Rejected: the type alias is internal (`pub(crate)`-equivalent via `type` at module scope); the name is PnP-internal and renaming adds churn without functional benefit. The layout is restructured, not the name.
  - **Port OrcaSlicer's two-pass `for go_in_reverse_direction` loop into `stitch_extrusions` as part of the `canReverse` fix.** Rejected: the two-pass loop is a deeper structural change (re-reversing the seed chain, the `3*max_stitch_distance` preference bias, the `closest_is_closing_polygon` flag) that would require restructuring `stitch_group` from a greedy pairwise merger to a per-chain growth algorithm. Out of scope; the `canReverse` parity gate alone is the minimal faithfulness fix this packet commits to.
  - **Add `canConnect` (odd/even parity gate) to `stitch_extrusions`.** Rejected: PnP already groups by `(inset_idx, is_odd)` (`stitch.rs:83`), which is structurally equivalent to OrcaSlicer's per-`wall_idx` caller loop + `canConnect` parity check (`PolylineStitcher.cpp:35-40`). The behavior is already correct; no change needed.
  - **Add the `+0.01mm` close-preference bias to `finalize_chain`.** Rejected: the bias is a tie-breaker that doesn't change the geometric outcome. Out of scope; the tiny-poly rule is the meaningful correctness fix.

## Files in Scope (read + edit)

List the files the implementer is expected to read and edit. Target ≤ 3 primary files. If more than 3 are unavoidable, justify each one — and consider splitting the packet.

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: the `EdgeJunctions` storage and the 4 downstream functions that read it; expected change: type alias restructure, 4 function updates, `default_extrusion_junction` removal.
- `crates/slicer-core/src/arachne/stitch.rs` — role: the `stitch_extrusions` post-processor; expected change: `canReverse` gate + `3 * max_gap` tiny-poly rule.
- `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — role: regression-locked test for `generate_junctions` width correctness; expected change: destructuring update.
- `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` — role: regression-locked test for the upward-half-edge-only emission; expected change: destructuring update.
- New `crates/slicer-core/tests/arachne_stitch_can_reverse.rs` — role: new test for the `canReverse` gate; expected change: write from scratch.
- New `crates/slicer-core/tests/arachne_stitch_tiny_polygon.rs` — role: new test for the tiny-poly rule; expected change: write from scratch.

The 3 primary files are `generate_toolpaths.rs`, `stitch.rs`, and the two existing test files (treated as one logical file for the 3-file budget). The two new test files are the packet's own test surface, not external dependencies.

Additional touched files (not in the 3-file budget but required for the refactor to compile and pass):
- `CONTEXT.md` (glossary update)
- `docs/DEVIATION_LOG.md` (deviation entry)
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (cross-reference)
- `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` (re-record, delete + regenerate)

## Read-Only Context

Files the implementer is allowed to read but not edit. Include line-range hints whenever the file is > 300 lines. The implementer should range-read these, not load them in full.

- `docs/07_implementation_status.md` — read lines 317-328 (P141-P147 rows + M2 closure) — purpose: confirm the N1-N13 chain is closed and the residual is the `#[ignore]`d `cube_4color` gate.
- `docs/DEVIATION_LOG.md` — read lines 60-76 (D-141, D-142, D-146, D-147 entries) — purpose: confirm what was already implemented and what the current behavior is.
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — read full (139 lines) — purpose: this packet implements two of the functions ADR-0035 lists.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full (137 lines) — purpose: background on the `find_quad` / `quad_peak_position` quad topology.
- `crates/slicer-core/src/arachne/pipeline.rs` — read line 390 only (the `stitch_extrusions` call site) — purpose: confirm the call site signature doesn't need to change.
- `crates/slicer-core/tests/generate_toolpaths.rs` — read lines 100-200 (the `run_pipeline` helper + `factory_params`) — purpose: confirm the test infrastructure is unchanged.
- `crates/slicer-core/tests/arachne_annulus_split.rs` — read full (142 lines) — purpose: stability anchor; the test passes today and must continue to pass with the same `inset0: lines=1 closed=1 sizes=[45]` output.

## Out-of-Bounds Files

Files the implementer must NOT load directly. The implementer should delegate any fact-checks against this list.

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` (2421 lines) — delegate parity checks; never load
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` (264 lines) — delegate parity checks; never load
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` — delegate parity checks; never load
- `target/`, `Cargo.lock`, generated code — never load
- Vendored deps under `vendor/` or equivalent — never load
- `crates/slicer-core/src/arachne/separate_inner_contour.rs`, `crates/slicer-core/src/arachne/remove_small.rs`, `crates/slicer-core/src/arachne/simplify.rs` — no touch; do not browse
- `crates/slicer-core/src/arachne/preprocess.rs` — no touch; do not browse
- `crates/slicer-core/src/skeletal_trapezoidation/**` — no touch; the quad topology is correct and locked
- Crates outside `slicer-core` — delegate trait/impl lookups; do not browse

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make. This list is not exhaustive but should cover the predictable ones.

- "Find the exact line in `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` where `generateJunctions` pushes junctions into `LineJunctions`; return SNIPPETS of `:2064-2076` (the inner `for` loop, ≤ 30 lines) and the struct definition of `LineJunctions` from `ExtrusionLine.hpp` (delegate separately). Confirm the push order (peak-side first vs boundary-side first) by reading the `junction_idx` indexing direction." — purpose: ground the storage restructure.
- "Find the exact `shorter_then` call at `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2217` and the `scaled<coord_t>(0.010)` constant; return the surrounding 10 lines. Also return the `from_is_3way`/`to_is_3way` computation at `:2359-2360` and the `passed_odd_edges.emplace(quad_start->next)` at `:2361`." — purpose: ground the `canReverse` parity gate (note: this packet doesn't port `addToolpathSegment` per-quad emission, but the `canReverse` check is structurally related).
- "Find the `VariableWidthLines` specialization of `canReverse` at `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp:22-30`; return SNIPPETS of the body and the `canConnect` body at `:35-40`. Confirm the return condition: even lines return false, odd lines return true." — purpose: ground the `stitch_extrusions` `canReverse` port.
- "Find the `chain_length + dist < 3 * max_stitch_distance` check at `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp:136-141`; return SNIPPETS of the surrounding 15 lines, including the `chain.size() <= 2` guard at `:138`." — purpose: ground the `finalize_chain` tiny-poly rule.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log`; return the `inset0:` debug line and the test result." — purpose: AC-3 stability anchor.
- "Run `cargo check -p slicer-core --features host-algos --all-targets 2>&1 | tee target/test-output-153-check.log`; return the first compile error per file on failure, or 'all targets compile' on success." — purpose: gate.

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

## Context Cost Estimate

- **Aggregate (sum across all steps):** `M` (5 implementation steps, 2 OrcaSlicer dispatches, 8 verification runs, all per-crate).
- **Largest single step:** `M` (Step 1: storage restructure + 4 function updates + 2 test destructuring updates + fixture re-record; this is the heaviest step by far).
- **Highest-risk dispatch (the one whose return could blow budget if mis-shaped):** the OrcaSlicer `generateJunctions` layout dispatch. If the sub-agent returns the full `for` loop (30+ lines) plus the struct definition (another 30+ lines), the implementer's context fills with verbatim OrcaSlicer source. Mitigation: enforce the return-format hint strictly (SNIPPETS ≤ 30 lines, no more than 2 snippets). If the sub-agent overshoots, re-dispatch with a tighter scope.

## Open Questions

- Resolve any ambiguity here before the packet becomes `active`.
- If an open question would change scope, interfaces, or verification strategy, the packet must remain `draft` until it is answered.
- If an open question requires reading an out-of-bounds file to answer, escalate to a delegation plan rather than admitting the file into scope.
- Mark forward-looking questions (implementer can resolve mid-flight) with `[FWD]`. Mark activation-blocking questions with `[BLOCK]`.

- [FWD] The per-bead line count shift in `generate_toolpaths_tapered_wedge` after the storage restructure: will the fixture's `line_counts` array stay the same, or shift? If it shifts, is the shift a layout artifact (junction count changes per line) or a behavior regression (different number of lines emitted)? Resolve by deleting the fixture, re-running `generate_toolpaths_tapered_wedge`, and diffing the new fixture against the pre-refactor version (which can be recovered from git history if needed).
- [FWD] The `canReverse` gate's effect on the `cube_4color_arachne_outer_walls_close_end_to_end` e2e gate (currently `#[ignore]`d at 49.33% closure). The gate may improve or worsen the residual; either is acceptable for this packet (the residual is explicitly out of scope). Resolve by re-running the `#[ignore]`d test after the refactor and noting the result in the packet's closure summary; do not gate the packet on the result.
- [FWD] The `3 * max_gap` tiny-poly rule's effect on the `arachne_local_maxima_single_beads` hexagon test (AC-4 of packet 147). The hexagon is closed pre-stitch and passes through `stitch_extrusions` via the AC-6 closed-passthrough, so the rule should not affect it. Resolve by re-running the test after the refactor; if it fails, the `finalize_chain` guard over-rejects and the threshold needs adjustment.
