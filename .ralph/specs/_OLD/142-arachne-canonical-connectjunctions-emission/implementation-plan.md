# Implementation Plan: 142-arachne-canonical-connectjunctions-emission

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Canonical `connectJunctions` per-quad emission + `perimeter_index = bead_idx`

- Task IDs:
  - `none` (N2 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N2)
- Objective: Rewrite `chain_junctions_for_bead`/`emit_chain_lines`/`generate_toolpaths` (symbol-search — the `:401-758` range is pre-`9367d239`, do not trust literally) to the canonical per-quad `connectJunctions` scheme — from/to pairing, `perimeter_index` pop-back merge, `addToolpathSegment` line growth **including 3-or-more-way junction detection in the domain-chain walk itself** (see `packet.spec.md`'s AC-4 and Goal-section scope correction — this is a REQUIRED part of this step, not optional hardening: the current `find_quad` + plain `.twin`-hop walk drives straight through a genuine branch vertex, e.g. a square's medial-axis center where 4 spokes meet, merging unrelated spokes into one fragmented chain), `new_domain_start` flag. Set `perimeter_index = junction_idx` (bead/inset index) at junction generation (symbol-search `generate_junctions`; was `:315,326` pre-fix). Delete `assign_perimeter_indices` (symbol-search in `pipeline.rs`; was `:384-390` pre-fix) + its call site (was `:373`) — **do not touch the `populate_beading_propagation` call A1 added to this same file.** Update `arachne_pipeline.rs:122` in place to assert `perimeter_index == line.inset_idx`.
- Precondition: A1's `generate_junctions` fix (commit `9367d239`) is present on this branch — `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast` passes (all 3). **Do NOT require `141`'s `packet.spec.md status: implemented`** — per the reverse-coupling discovery (`packet.spec.md`'s Prerequisites section), 141 cannot reach `implemented` until THIS step's AC-4 is green, so gating on 141's status would deadlock. Gate on the regression-test command above instead.
- Postcondition: AC-1 (N2 red test) passes — every junction carries `perimeter_index == line.inset_idx`. AC-N1 (`arachne_pipeline.rs:122` updated) passes. **AC-4 passes** — A1's own AC-1/AC-2 (`arachne_parity_red_junction_bands.rs`), `outer_wall_closes_for_simple_polygon`, `generate_toolpaths_tapered_wedge`, `outer_wall_is_closed_ring_for_simple_polygons`, and the 2 `arachne_parity_red_chain_junctions.rs` tests all go GREEN — this is the concrete evidence the 3-way-junction fix actually works, not a "nice to have." `arachne_generate_junctions_canonical_regression.rs`'s 3 tests STAY GREEN (confirms this step didn't reintroduce A1's fixed bugs while rewriting the surrounding chain walk). N3, N4 red tests stay RED (Step 2 owns N4).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — full-read for this step (primary edit target; line numbers shifted significantly at `9367d239`, re-locate by symbol name).
  - `crates/slicer-core/src/arachne/pipeline.rs` — the `assign_perimeter_indices` deletion + call site (symbol-search; also note the `populate_beading_propagation` call A1 added — do not remove it).
  - `crates/slicer-core/tests/arachne_pipeline.rs` — lines `:120-150` (the in-place update target).
  - `crates/slicer-core/tests/arachne_parity_red_perimeter_index.rs` — full (157 lines); AC-1 oracle.
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full; AC-4's primary oracle (A1's AC-1/AC-2).
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — full (read-only for this step; pins A1's 3 fixed bugs in isolation — must stay green, do not edit its assertions to make it pass).
- Files allowed to edit (≤ 6) — **expanded 2026-07-06 (first swarm run found AC-4
  cannot close without three test-fixture corrections that the original ≤3 edit
  list excluded; see "Known Implementation Hazard" addendum at the end of this
  step block)**:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `crates/slicer-core/src/arachne/pipeline.rs`
  - `crates/slicer-core/tests/arachne_pipeline.rs` — the `:122` in-place update.
  - `crates/slicer-core/tests/arachne_parity_red_perimeter_index.rs` — **AC-1
    fixture correction**: the hand-built graph puts `bead_count: Some(2)` on the
    LOW-R vertex (`v1`, R=1mm) and leaves the PEAK (`v0`, R=3mm) with
    `bead_count: None`. Canonical `generateJunctions` resolves the beading at
    the peak (`getOrCreateBeading(edge->to, ...)`, `SkeletalTrapezoidation.cpp
    :2029`); with no `bead_count` at the peak, no in-band beads are emitted and
    no inset>0 line is produced, so the `saw_nonzero_inset` guard fires before
    the `perimeter_index == inset_idx` assertion can even run. **Fix the
    fixture**: move `bead_count: Some(2)` to the peak vertex (and set the
    low-R vertex's `bead_count` to `None` or `Some(1)` as the canonical
    lower-bead-count side). Do NOT weaken the `perimeter_index == inset_idx`
    assertion — that is the AC.
  - `crates/slicer-core/tests/arachne_parity_red_chain_junctions.rs` — **AC-4
    fixture correction**: both `constant_radius_chain_to_junction_lands_at_end_vertex_not_start`
    and `f3_invariant_chain_has_one_junction_per_endpoint_at_shared_vertex`
    build graphs with ALL vertices at `distance_to_boundary: 1_000_000.0`
    (identical R) — i.e. flat/constant-R central spine edges. Canonical
    `generateJunctions` skips flat edges (`from_r >= to_r` → continue,
    `:2017`; same-bead-count skip `:2024-2027`), so no junctions are emitted
    on these edges and the chain walk has nothing to carry — the "expected at
    least one inset bucket" failure is correct behavior, not a bug. These
    fixtures pre-date A1's canonical rewrite and encode the old
    flat-edge-emits-junctions assumption. **Fix the fixtures**: replace the
    flat constant-R central edges with upward edges (varying R, e.g.
    `v0.R = 1_000_000`, `v1.R = 3_000_000`, `v2.R = 1_000_000` — a peak at
    v1) AND add `EdgeType::EXTRA_VD` rib edges connecting the spine to the
    boundary-side vertices, so the chain walk routes through ribs as
    canonical requires. `f3_invariant_junction_widths_are_finite_and_positive`
    (the 3rd test, currently passing) may also need its fixture checked for
    the same constant-R issue — verify it still passes after the rib additions;
    if it was passing only because the flat-edge path happened to produce
    finite widths, fix it too. Do NOT weaken the
    `constant_radius_chain_to_junction_...` / `f3_invariant_chain_has_one_junction_...`
    assertions — those are the ACs.
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — **AC-4
    oracle; edit ONLY if the 1.7mm-from-boundary failure persists after the
    chain-walk fix below lands green for `outer_wall_closes_for_simple_polygon`
    and `outer_wall_is_closed_ring_for_simple_polygons`**. The rectangle/square
    fixtures here are run through the FULL `run_arachne_pipeline` (not a
    hand-built graph), so their geometry is real Voronoi output — do NOT edit
    the fixture geometry. If the 1.7mm failure persists after the chain-walk
    fix, the failure is in `generate_junctions`'s near-start snap (141's
    deeper scope) and must be reported as a finding, NOT fixed by editing this
    test. The only permitted edit to this file is re-recording an
    assertion-strength-preserving baseline if the chain-walk fix shifts the
    junction positions within the existing ≤0.6mm/≤0.15mm tolerances and the
    test was previously passing around a now-corrected edge case — and that
    edit must be justified in the commit message with the before/after
    measured values.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs:632` (`is_odd` — Step 2's scope)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C's π hack / fudge)
  - `crates/slicer-sdk/src/host.rs:717` and `crates/slicer-wasm-host/src/host.rs:1814` (wire-type-transparent; NOT edited)
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — A1's 3 bug-regression locks; read-only; do NOT edit its assertions to make it pass
  - `OrcaSlicerDocumented/...` (delegate)

### Known Implementation Hazard (Step 1) — added 2026-07-06 after first swarm run

**A first swarm run (this session) implemented the chain-walk rewrite +
3-way-detection + `perimeter_index = bead_idx` + `assign_perimeter_indices`
deletion + `arachne_pipeline.rs:122` in-place update, and correctly kept A1's
3 regression locks green and N3/N4 red. It then reported the remaining AC-4
failures as "pre-existing fixture defects" and self-deferred. Independent
verification found this was a PARTIAL deflection — two of the three claimed
"defects" are real work this step must do; the third is a chain-walk bug the
swarm misattributed to 141. Do not repeat these:**

1. **The 3-way-junction detection must count flat/rib edges, not just upward
   half-edges.** The first run's `compute_vertex_degree` counted only edges
   with `from.R < to.R` (upward half-edges); flat/constant-R rib edges
   contribute 0 to degree, so a vertex reached only by flat ribs (e.g. the
   medial-axis center of a rectangle, where the constant-R spine meets ribs
   from both sides) never triggers the 3-way stop — the walk drives straight
   through it, stitching ribs from opposite sides into one chain, producing
   the 1.7mm-from-boundary junction (near the spine at R=2mm, not the outer
   bead at R=0.2mm) on `arachne_parity_red_junction_bands`. Canonical
   `addToolpathSegment`'s "not a 3-way" check
   (`SkeletalTrapezoidation.cpp:2198-2234`) is NOT an upward-edge-degree
   count — it checks whether the vertex is a junction of DISTINCT QUAD
   DOMAINS. **Before re-implementing, dispatch an OrcaSlicer SUMMARY of
   `:2198-2234` asking specifically**: "what is the exact predicate that
   identifies a 3-way (or higher) junction — edge degree, quad-domain count,
   or something else? Do flat/constant-R edges and rib (`EXTRA_VD`) edges
   contribute to the count?" The degree counter (or replacement predicate)
   must account for flat-edge/rib contributions so a constant-R spine vertex
   where ribs converge is recognized as a branch point.

2. **The three test fixtures above (`arachne_parity_red_perimeter_index.rs`,
   `arachne_parity_red_chain_junctions.rs`, and conditionally
   `arachne_parity_red_junction_bands.rs`) are in this step's edit list
   precisely so AC-4 can close.** The first run's "pre-existing fixture
   defect" framing for `arachne_parity_red_chain_junctions.rs` was a
   deflection: `packet.spec.md:127` already anticipated that these fixtures
   "relied on a flat central edge also emitting to bridge the chain, which
   canonical's flat-edge skip plus this AC's rib-based connectivity must
   replace" — replacing that connectivity is THIS step's deliverable, and
   updating the fixtures to exercise the rib-based path (instead of the
   removed flat-edge path) is part of that deliverable. Fix the fixtures by
   adding ribs + varying R as described in each file's edit-list entry above;
   do NOT weaken the assertions.

3. **Do NOT misattribute chain-walk failures to `generate_junctions` (141).**
   The first run reported the 1.7mm `arachne_parity_red_junction_bands`
   failure as a `generate_junctions` "near-start snap" issue owned by 141.
   A1's 3 regression locks are green; D-141 says `generate_junctions` is
   "genuinely canonical and ground-truth-verified." A junction landing at
   1.7mm (near the R=2mm medial axis) on a 4mm rectangle is the chain walk
   emitting at/near the peak vertex, not `generate_junctions` misplacing the
   junction — `generate_junctions` produces the correct interpolated outer-
   bead position, but the chain walk's missing rib-based routing sends it to
   the peak instead. If the 1.7mm failure persists AFTER the chain-walk fix
   (#1) lands green for `outer_wall_closes_for_simple_polygon` and
   `outer_wall_is_closed_ring_for_simple_polygons`, THEN it is a
   `generate_junctions` concern and must be reported as a finding (not fixed
   in this step, not deflected).
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2283-2327` `connectJunctions` — explicitly ask for the per-quad from/to pairing + `perimeter_index` pop-back merge; return ≤ 200 words, no code unless asked" — purpose: confirm emission rewrite.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2198-2234` `addToolpathSegment` — explicitly ask HOW it detects a 3-or-more-way junction (not just the extend-vs-new-line decision) and what it does instead of extending through one; return ≤ 200 words" — purpose: confirm the 3-way detection this step's AC-4 requires, not just line-growth.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture`; return FACT (pass) or SNIPPETS (fail + ≤ 20 lines)" — purpose: validate AC-1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture`; return FACT pass/fail" — purpose: validate AC-N1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass or SNIPPETS (fail)" — purpose: validate AC-4's primary oracle — MUST be pass, not "expected fail", by the end of this step.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths --no-fail-fast`; return FACT pass or SNIPPETS (fail)" — purpose: validate AC-4 (`outer_wall_closes_for_simple_polygon`, `generate_toolpaths_tapered_wedge`).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture`; return FACT pass or SNIPPETS (fail)" — purpose: validate AC-4.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_chain_junctions --no-fail-fast`; return FACT pass or SNIPPETS (fail)" — purpose: validate AC-4 (`constant_radius_chain_to_junction_lands_at_end_vertex_not_start`, `f3_invariant_chain_has_one_junction_per_endpoint_at_shared_vertex`).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast`; return FACT pass (all 3 — confirms this step did NOT reintroduce A1's fixed bugs while rewriting the chain walk) or SNIPPETS (fail)" — purpose: regression gate on A1's own fix.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT fail (expected — N4 stays red, Step 2 owns it)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 stays red)" — purpose: gate scope.
  - "Find all callers of `assign_perimeter_indices`; return LOCATIONS" — purpose: confirm no orphan call sites.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" (lines ~1091-1150) — `ExtrusionJunction`/`ExtrusionLine` field shapes.
  - `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry, INCLUDING its 2026-07-06 correction paragraph — read full; this is where AC-4's root cause is documented in detail.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2283-2327` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2198-2234` — delegate, with the 3-way-detection question above.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2064-2077` — delegate (`perimeter_index = junction_idx`).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture 2>&1 | tee target/test-output-a2-step1-ac1.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture 2>&1 | tee target/test-output-a2-step1-neg1.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test generate_toolpaths --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-a2-step1-ac4.log` — FACT pass (AC-4, all of it).
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture 2>&1 | tee target/test-output-a2-step1-ac4b.log` — FACT pass (AC-4).
  - `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression --no-fail-fast 2>&1 | tee target/test-output-a2-step1-a1-regression.log` — FACT pass (A1's 3 bugs stay fixed).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-a2-step1-stays-red.log` — FACT fail (expected — N4/N3 stay red).
- Exit condition: AC-1 + AC-N1 + AC-4 (all of it) pass; A1's bug-regression locks stay green; N4/N3 stay red; `cargo check -p slicer-core --all-targets` passes. **The three test-fixture corrections in the expanded edit list above are REQUIRED for AC-1 and AC-4's `arachne_parity_red_chain_junctions` sub-tests to pass — they are not optional hardening. If AC-4's `arachne_parity_red_junction_bands` sub-test (the full-pipeline rectangle/square) still fails after the chain-walk fix and the two hand-built fixture corrections, report it as a finding (likely a `generate_junctions` near-start-snap concern for 141's deeper scope) — do NOT defer the entire step on it, and do NOT weaken its assertion.**

### Step 2: Canonical `is_odd` per-segment + `passed_odd_edges` + fixture re-baseline + deviation log

- Task IDs:
  - `none` (N4 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N4)
- Objective: Replace `is_odd: bead_idx % 2 == 1` (`generate_toolpaths.rs:632`) with the canonical per-segment rule (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost junction, endpoint proximity 0.005 mm to peak node). Rework `passed_odd_edges` to key on the physical edge. Re-baseline affected fixtures (`toolpaths_tapered_wedge.json`, `stitch_*.json` if they exist). Add the `D-142-CONNECTJUNCTIONS-EMISSION` deviation-log entry + `D-141-JUNCTION-BANDS` addendum.
- Precondition: Step 1 is green (canonical `connectJunctions` emission + `perimeter_index = bead_idx` land first; `is_odd` is computed per segment during the `connectJunctions` walk, so it depends on Step 1's quad structure).
- Postcondition: AC-2 (even bead count → no `is_odd`) and AC-3 (inset-1 survives `remove_small_lines`) pass. N1, N2 stay GREEN. N3 stays RED. Affected fixtures re-baselined. `D-142-CONNECTJUNCTIONS-EMISSION` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — lines `:620-640` (the `is_odd` site) + the `passed_odd_edges` site (range-read; A2's Step 1 already touched this file).
  - `crates/slicer-core/tests/arachne_parity_red_is_odd_semantics.rs` — full (194 lines); AC-2 + AC-3 oracle + `FixedBeadingStrategy`/`two_bead_single_edge_graph` fixture.
  - `crates/slicer-core/src/arachne/stitch.rs` — line `:83` (the `is_odd` grouping key — read-only confirmation).
  - `crates/slicer-core/src/arachne/remove_small.rs` — line `:57` (the `is_odd && !is_closed` gate — read-only confirmation).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-142-CONNECTJUNCTIONS-EMISSION` + one-line addendum on `D-141-JUNCTION-BANDS`; no in-place edits)
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` (re-record via self-capture; never read directly)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/stitch.rs` and `remove_small.rs` (A2 changes the `is_odd` *producer*, not the consumers — read-only confirmations only)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2344-2354` canonical `is_odd` — ask for the four conditions (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost, endpoint proximity 0.005 mm) and the `passed_odd_edges` physical-edge key (`:2355-2361`); return ≤ 200 words" — purpose: confirm `is_odd` rewrite.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-2 + AC-3.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --no-fail-fast`; return FACT pass (expected — N1/N2 stay green)" — purpose: gate A2 didn't regress A1/Step 1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 stays red)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths --test stitch --test remove_small 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/a2-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + the `failures.len()/total_checked` summary line — purpose: record the e2e closure delta (record-only; A2 does NOT block on green)" — purpose: record delta for commit message.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" — `ExtrusionLine::is_odd` field shape.
  - `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2344-2354` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2355-2361` — delegate (`passed_odd_edges`).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp:62-70` — delegate (`is_odd` semantics).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — delegate (`removeSmallLines` gate).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-a2-step2-ac.log` — FACT pass (AC-2 + AC-3).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --no-fail-fast 2>&1 | tee target/test-output-a2-step2-stays-green.log` — FACT pass (N1/N2 stay green).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-a2-step2-n3-red.log` — FACT fail (expected — N3 stays red).
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths --test stitch --test remove_small 2>&1 | tee target/test-output-a2-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-142-CONNECTJUNCTIONS-EMISSION' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-2, AC-3 pass; N1/N2 stay green; N3 stays red; generate_toolpaths/stitch/remove_small regression green; `D-142-CONNECTJUNCTIONS-EMISSION` present; `cargo check -p slicer-core --all-targets` and `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N2 connectJunctions emission + perimeter_index) | M | Heaviest dispatch: `connectJunctions` SUMMARY. |
| Step 2 (N4 is_odd + passed_odd_edges + fixtures + deviation log) | M | Heaviest dispatch: `is_odd` SUMMARY + regression suite. |

Aggregate: M + M = M (Step 2 shares Step 1's `generate_toolpaths.rs` context). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-3, AC-N1 dispatched and returned PASS).
- N1, N2 stay GREEN; N3 stays RED (scope boundary gates).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-142-CONNECTJUNCTIONS-EMISSION` present in `docs/DEVIATION_LOG.md` with addendum on `D-141-JUNCTION-BANDS`.
- Affected `slicer-core` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-3, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2 "stays green" and N3 "stays red" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.