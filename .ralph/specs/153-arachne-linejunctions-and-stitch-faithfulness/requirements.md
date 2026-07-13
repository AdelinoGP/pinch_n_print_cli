# Requirements: 153-arachne-linejunctions-and-stitch-faithfulness

## Packet Metadata

- Grouped task IDs: `none` (this packet is a post-D-147 faithfulness refactor; no `TASK-###` or `T-###` maps to it in `docs/07`).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (2 source files, 5 test files, 2 OrcaSlicer references, all delegated/range-read)

## Problem Statement

ADR-0035 (Accepted 2026-07-08) established that the Arachne emission, transitions, and post-process surface must be faithful algorithm-level ports of OrcaSlicer's C++ reference. Packet 147 (D-147-CHAIN-CLOSURE, 2026-07-08) closed the N1–N13 chain by fixing the 7 deferred parity-audit findings, but two PnP-internal divergences from OrcaSlicer's reference remain, both in functions ADR-0035 lists as requiring faithful ports.

**Divergence 1 — `EdgeJunctions` storage layout.** `crates/slicer-core/src/arachne/generate_toolpaths.rs:141` defines `type EdgeJunctions = (Vec<ExtrusionJunction>, Vec<ExtrusionJunction>)` — a PnP-internal split into `from_junctions` (at the edge's start vertex) and `to_junctions` (at the resolved-to vertex), indexed by `perimeter_index` slot with `default_extrusion_junction()` placeholders (`:484-490`) for out-of-band beads. OrcaSlicer's `generateJunctions` (`SkeletalTrapezoidation.cpp:2013-2079`, specifically `:2030-2076`) stores one `LineJunctions = std::vector<ExtrusionJunction>` per edge, ordered peak-side (high R) to boundary-side (low R), one entry per in-band bead, `perimeter_index = junction_idx` set at generation. The PnP layout forces downstream code to look up junctions by `perimeter_index` slot, requires placeholder handling, and diverges from the canonical reference that any future parity audit or re-implementation would read.

**Divergence 2 — `stitch_extrusions` post-process faithfulness.** `crates/slicer-core/src/arachne/stitch.rs:71-249` is a faithful port of OrcaSlicer's `PolylineStitcher::stitch` (matching the `(inset_idx, is_odd)` grouping at `stitch.rs:83` and the distance-only join at `stitch.rs:134`) but lacks two canonical behaviors:

- **`canReverse` (even-line reversal blocking):** OrcaSlicer's `PolylineStitcher.cpp:22-30` blocks reversing even (`!is_odd`) wall bands — even walls encode sidedness relative to their neighboring wall and must keep their orientation stable. PnP's `merge_chains` (`stitch.rs:188-214`) reverses any chain in all 4 endpoint combinations, so an even-wall fragment can be flipped to CCW when the canonical wall is CW.
- **Tiny-polygon non-closure rule:** OrcaSlicer (`PolylineStitcher.hpp:136-141`) prevents closing a chain into a polygon when its total length + closing-segment distance is `< 3 * max_stitch_distance` (it might still extend into a longer polyline) and refuses to make 2-vertex polygons (`chain.size() <= 2`). PnP's `finalize_chain` (`stitch.rs:220-241`) closes any chain whose endpoints are within `max_gap`, with no length guard, producing small spurious closed loops where OrcaSlicer would leave the chain open.

This packet is a refactor, not a bug fix. The `arachne_annulus_split` test passes today (`inset0: lines=1 closed=1 sizes=[45]`) and the N1–N13 chain is closed. The refactor brings the two divergent functions closer to their canonical implementations so future maintainers and parity audits don't have to reason about PnP-internal conventions.

## In Scope

- Restructure `EdgeJunctions` from `(Vec<ExtrusionJunction>, Vec<ExtrusionJunction>)` to `Vec<ExtrusionJunction>` per edge, matching OrcaSlicer's `LineJunctions` layout (one `Vec` per upward edge, ordered peak-side to boundary-side, `perimeter_index = junction_idx`).
- Insert explicit empty `Vec` entries for non-upward / flat / same-bead-count edges in `generate_junctions` (instead of `continue`-ing at `:279` / `:290`), matching OrcaSlicer's lazy-empty-`LineJunctions` fallback at `SkeletalTrapezoidation.cpp:2290-2298`. This makes `connectJunctions`'s `edge_from_peak->twin` read deterministic.
- Remove `default_extrusion_junction()` (`:176-188`) — the placeholder is no longer needed once per-edge storage no longer uses `perimeter_index` slot indexing.
- Update `chain_junctions_for_bead` (`:554-597`), `is_odd_segment` (`:609-644`), `is_odd_endpoint` (`:647-694`), and `emit_chain_lines` (`:711-803`) to read from the new single-`Vec` layout. The junction lookup `from_j.get(bead)` / `to_j.get(bead)` becomes `junctions[len - 1 - rev_idx]` (OrcaSlicer's innermost-to-outermost pairing, `SkeletalTrapezoidation.cpp:2336-2338`).
- Update `arachne_generate_junctions_canonical_regression.rs` and `arachne_junction_upward_half_edge_only.rs` to destructure the new return type.
- Port `canReverse` into `stitch_extrusions`: even (`!is_odd`) lines must not be reversed during a join; only `(End, Start)` and `(Start, End)` merges (no reversal) are permitted for even-line groups. Odd-line groups retain the current 4-way merge behavior.
- Port the `3 * max_stitch_distance` tiny-polygon non-closure rule into `finalize_chain`: compute the chain's total polyline length, add the closing-segment distance, and skip closure if the sum is `< 3 * max_gap`. Also reject closure when `junctions.len() <= 2`.
- Add `arachne_stitch_can_reverse.rs` and `arachne_stitch_tiny_polygon.rs` integration tests covering the two new stitch behaviors (AC-6, AC-7, AC-N2, AC-N3).
- Re-record `tests/fixtures/arachne/toolpaths_tapered_wedge.json` if the storage restructure changes the per-bead line counts (likely, since the single-`Vec` layout may differ from the split layout at the junction-merge sites).
- Update `CONTEXT.md`: delete the "Junction fan" entry, add an "Edge junctions" entry.
- Add `D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS` to `docs/DEVIATION_LOG.md`.
- Add a cross-reference to packet 153 in `docs/adr/0035-arachne-faithful-emission-and-transitions.md` §"Consequences".

## Out of Scope

- The `cube_4color_arachne_outer_walls_close_end_to_end` e2e closure gate (49.33% closure, `#[ignore]`d by user decision 2026-07-08). That residual is out of scope for this packet and is a separate session's work (D-147-CHAIN-CLOSURE's tracked residual).
- Rewriting `connectJunctions` to use a per-quad per-bead emission model with `addToolpathSegment`'s proximity-gated append. D-147 finding #2 ("full-chain walk with proximity-gated append") was implemented in packet 147 as the post-sub-run-split `emit_chain_lines`; the per-quad model is a deeper refactor that would change `emit_chain_lines`'s shape and is not required for this packet's faithfulness bar.
- Restructuring `EdgeJunctions` to use OrcaSlicer's `LineJunctions` name (the type alias is internal; the name is PnP-internal). Only the layout is restructured.
- Adding `canConnect` (odd/even parity gate) to `stitch_extrusions`. PnP already groups by `(inset_idx, is_odd)` (`stitch.rs:83`), which is structurally equivalent to OrcaSlicer's per-`wall_idx` caller loop + `canConnect` parity check (`PolylineStitcher.cpp:35-40`); the behavior is already correct.
- Touching the `snap_distance = scaled(0.01)` 0.01 mm close-preference bias in `PolylineStitcher::stitch` (OrcaSlicer `:146` and `:153`). PnP's `stitch_extrusions` closes chains when endpoints are within `max_gap` without the 0.01 mm preference bias. The bias is a tie-breaker that doesn't change the geometric outcome; including it would require restructuring `stitch_group` into OrcaSlicer's two-pass `for go_in_reverse_direction` shape, which is out of scope.
- Touching the wall-direction reversal rules in `stitch_extrusions` beyond the `canReverse` parity gate. OrcaSlicer's `canReverse` only blocks reversal for even lines; the `for go_in_reverse_direction` two-pass loop has additional complexity (re-reversing the seed chain after the second pass) that is a separate refactor.
- Any guest WASM, WIT, or module SDK change. This packet is host-only (`slicer-core`).
- Any `pnp_cli`, `pnp_cli slice`, or gcode emit change. The refactor is internal to the emission core.
- The `is_odd` semantics or `passed_odd_edges` dedup key. Both are locked by D-142-CONNECTJUNCTIONS-EMISSION.

## Authoritative Docs

- `docs/07_implementation_status.md` — read lines 317-328 (P141-P147 rows + M2 closure). ~300 lines; read directly.
- `docs/DEVIATION_LOG.md` — read entries D-141-JUNCTION-BANDS, D-142-CONNECTJUNCTIONS-EMISSION, D-147-PARITY-AUDIT-FINDINGS, D-147-CHAIN-CLOSURE (lines 60-76). Delegated; the full file is 104 lines, read directly is fine.
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — read full (139 lines); this packet implements two of the functions ADR-0035 lists.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full (137 lines); background on the `find_quad` / `quad_peak_position` quad topology that constrains the `connectJunctions` port.

For each doc, note size and whether the implementer should load it directly or delegate. Default rule: delegate any doc > 300 lines.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

When this packet touches parity with OrcaSlicer's Arachne pipeline, implementers must read OrcaSlicer source through the `OrcaSlicerDocumented` reference at `F:/slicerProject/OrcaSlicerDocumented/src/libslic3r/Arachne/`. Do not load OrcaSlicer source directly into the implementer's context; dispatch a focused sub-agent with the exact `file:line` question and a tight return format (FACT pass/fail, or SNIPPETS ≤ 30 lines, or LOCATIONS ≤ 20 entries). The sub-agent's return is the implementer's input — never the OrcaSlicer source itself. Two files govern this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` lines 2013-2079 (`generateJunctions`, the per-edge `LineJunctions` layout) and lines 2198-2235 (`addToolpathSegment`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` lines 22-47 (`canReverse`, `canConnect`, `isOdd` for `VariableWidthLines` specialization) and `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 71-247 (the `PolylineStitcher::stitch` algorithm including the `3 * max_stitch_distance` tiny-poly rule at `:136-141`).

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` through `AC-8` from `packet.spec.md`. Add any packet-specific refinements here that didn't fit the Given/When/Then form (measurable outcomes, exact field names, count thresholds).
- Negative cases: `AC-N1` through `AC-N3` from `packet.spec.md`.
- Cross-packet impact: this packet's refactor must not regress any of the 21 arachne test binaries or any of the N1–N4 red tests (regression-locked by packet 142 and packet 147). The `arachne_annulus_split` test's `inset0: lines=1 closed=1 sizes=[45]` output is the concrete stability anchor for the storage restructure — if the count or closed count changes, the restructure is wrong.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the 2–3 gate commands; this section is the authoritative list with delegation hints.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check -p slicer-core --features host-algos --all-targets 2>&1 | tee target/test-output-153-check.log` | All targets (including the 21 arachne test binaries) compile after the storage restructure. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure showing the first compile error per file. |
| `cargo clippy -p slicer-core --features host-algos --all-targets -- -D warnings 2>&1 | tee target/test-output-153-clippy.log` | Clippy clean across all targets. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure. |
| `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only -- --nocapture 2>&1 | tee target/test-output-153-ac1.log` | AC-1: storage restructure destructure compiles and tests pass. | FACT pass/fail + test output's `inset0:` / `edges[0]` lines. |
| `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression -- --nocapture 2>&1 | tee target/test-output-153-ac2.log` | AC-2: 3 canonical regression tests pass with single-`Vec` destructuring. | FACT pass/fail + per-test pass count. |
| `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log` | AC-3: annulus test passes with `inset0: lines=1 closed=1 sizes=[45]` (pre-refactor value). | FACT pass/fail + the `inset0:` debug line. |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- --nocapture 2>&1 | tee target/test-output-153-ac4.log` | AC-4: tapered-wedge + simple-square tests pass after fixture re-record. | FACT pass/fail + per-test pass count. |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-153-ac5.log` | AC-5: N1–N4 red tests stay green (no regression from refactor). | FACT pass/fail + per-binary pass count. |
| `cargo test -p slicer-core --test arachne_stitch_can_reverse -- --nocapture 2>&1 | tee target/test-output-153-ac6.log` | AC-6 + AC-N2: new `canReverse` test passes (even-line reversal rejected, odd-line reversal permitted). | FACT pass/fail. |
| `cargo test -p slicer-core --test arachne_stitch_tiny_polygon -- --nocapture 2>&1 | tee target/test-output-153-ac7.log` | AC-7 + AC-N3: new tiny-poly test passes (sub-`3*max_gap` chains stay open, `>= 3*max_gap` chains close). | FACT pass/fail. |
| `cargo xtask test -p slicer-core --features host-algos --summary 2>&1 | tee target/test-output-153-ac8.log` | AC-8: per-crate summary reports PASS. | Summary digest (per the `--summary` contract). |

All verification commands must be delegation-friendly (small, parseable output) so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

## Step Completion Expectations

For each step in `implementation-plan.md`, the canonical fields (precondition, postcondition, files, dispatches, cost) live in that file. This section calls out only **cross-step** expectations that the step list cannot express:

- **Cross-step invariant:** no step may regress the `arachne_annulus_split` test's `inset0: lines=1 closed=1 sizes=[45]` output. This is the concrete stability anchor; if a step changes the annulus output, the previous step's restructure is wrong and must be revisited before proceeding.
- **Cross-step invariant:** no step may regress any of the 21 arachne test binaries. A test that fails to compile due to the storage destructuring change must be updated in the same step that introduces the change, not deferred.
- **Step ordering rationale:** Step 1 (storage restructure + downstream call-site updates) must land before Step 2 (stitch faithfulness fixes) because the stitch fixes are independent of the storage layout, but the storage restructure's compile errors across 5 test files would block any concurrent change. Doing one refactor at a time keeps the diff reviewable and the bisection path clean.
- **Cross-step shared scratch state:** none. The two refactors touch disjoint functions and disjoint test files. If a future step discovers coupling, that step must explicitly surface it.

## Context Discipline Notes

Document any context-budget hazards **specific to this packet**. Workspace-wide discipline lives in the `context-discipline` snippet in `packet.spec.md`; do not restate it here.

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (1261 lines) — the storage restructure touches `EdgeJunctions` (`:141`), `default_extrusion_junction()` (`:176-188`), `generate_junctions` (`:231-495`), `chain_junctions_for_bead` (`:554-597`), `is_odd_segment` (`:609-644`), `is_odd_endpoint` (`:647-694`), and `emit_chain_lines` (`:711-803`). Range-read each function in ±40-line windows; do not load the file in full.
  - `crates/slicer-core/src/arachne/stitch.rs` (249 lines) — the stitch faithfulness fixes touch `stitch_extrusions` (`:71-99`), `stitch_group` (`:118-158`), `pick_better` (`:163-180`), `merge_chains` (`:188-214`), and `finalize_chain` (`:220-241`). Range-read each function in ±30-line windows.
- **Likely temptation reads (files the implementer might curiosity-open) and why they should be skipped:**
  - `crates/slicer-core/src/arachne/pipeline.rs` — only `stitch_extrusions` call site at `:390`; read just that line, not the file.
  - `crates/slicer-core/src/arachne/separate_inner_contour.rs` — no touch; do not load.
  - `crates/slicer-core/src/arachne/remove_small.rs`, `crates/slicer-core/src/arachne/simplify.rs` — no touch; do not load.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` (2421 lines) — delegate all reads. The sub-agent's return is the implementer's input.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` (264 lines) — delegate all reads.
- **Sub-agent return-format hints for the heaviest dispatches:**
  - OrcaSlicer `generateJunctions` layout dispatch: "Return SNIPPETS of the per-edge push loop at `:2064-2076` (the `emplace_back(junction, width, junction_idx)` calls) and the struct definition of `LineJunctions` from `ExtrusionLine.hpp` (delegate separately). Verify the ordering (peak-side first vs boundary-side first) by reading `:2064`'s `for` loop direction and the `junction_idx` indexing."
  - OrcaSlicer `addToolpathSegment` proximity check dispatch: "Return the exact `shorter_then` call at `:2217` and the `scaled<coord_t>(0.010)` constant, with the surrounding 10 lines for context. Return the `from_is_3way`/`to_is_3way` computation at `:2359-2360` and the `passed_odd_edges.emplace(quad_start->next)` at `:2361`."
  - OrcaSlicer `canReverse` dispatch: "Return SNIPPETS of the `VariableWidthLines` specialization at `PolylineStitcher.cpp:22-30` (the `canReverse` body) and the `canConnect` body at `:35-40`. Verify the return condition: `if ((*ppi.polygons)[ppi.poly_idx].is_odd) return true; else return false;`."
  - OrcaSlicer tiny-poly rule dispatch: "Return SNIPPETS of the `chain_length + dist < 3 * max_stitch_distance` check at `PolylineStitcher.hpp:136-141`, with the surrounding 15 lines for context. Return the `chain.size() <= 2` guard at `:138`."
