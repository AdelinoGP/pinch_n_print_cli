# Implementation Plan: 153-arachne-linejunctions-and-stitch-faithfulness

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (`none` — this is a refactor packet; no `TASK-###` maps to it).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Restructure `EdgeJunctions` storage to single `Vec<ExtrusionJunction>` per edge

- Task IDs: `none`
- Objective: Replace `type EdgeJunctions = (Vec<ExtrusionJunction>, Vec<ExtrusionJunction>)` with `type EdgeJunctions = Vec<ExtrusionJunction>`, matching OrcaSlicer's `LineJunctions` layout. Push junctions in peak-side-to-boundary-side order with `perimeter_index = junction_idx`. Insert explicit empty `Vec` entries for non-upward / flat / same-bead-count edges.
- Precondition: `arachne_annulus_split` test passes with `inset0: lines=1 closed=1 sizes=[45]` (pre-refactor baseline confirmed).
- Postcondition: `cargo check -p slicer-core --features host-algos --all-targets` compiles, with all 5 test files that destructured the old return type updated. `arachne_annulus_split` still passes with the same `inset0: lines=1 closed=1 sizes=[45]` output.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — lines 130-200, 231-495, 540-700
  - `crates/slicer-core/src/arachne/stitch.rs` — lines 60-250 (to confirm `stitch_extrusions` doesn't read `edge_junctions` directly)
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — lines 160-410
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` — lines 120-210
  - `crates/slicer-core/tests/arachne_annulus_split.rs` — full (142 lines)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `EdgeJunctions` type alias, `default_extrusion_junction` removal, `generate_junctions` restructure, `chain_junctions_for_bead` lookup update, `is_odd_segment` / `is_odd_endpoint` lookup update, `emit_chain_lines` lookup update.
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` — destructuring at `:176`, `:280`, `:388`.
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` — destructuring at `:186`.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — delegate only; do not load directly.
  - `crates/slicer-core/src/arachne/stitch.rs` — not touched in this step.
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` — re-recorded in Step 2 only if per-bead line counts shift.
- Expected sub-agent dispatches:
  - "Find the exact push order in `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2064-2076`; return SNIPPETS ≤ 30 lines showing the `for` loop direction and the `emplace_back(junction, width, junction_idx)` calls. Confirm whether junctions are pushed peak-side first (high R, high `junction_idx`) or boundary-side first." — purpose: ground the push order in the new `generate_junctions` body. Return format: SNIPPETS.
  - "Run `cargo check -p slicer-core --features host-algos --all-targets 2>&1 | tee target/test-output-153-check.log`; return the first compile error per file on failure, or FACT pass on success." — purpose: gate Step 1's compile.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log`; return the `inset0:` debug line and the test result." — purpose: AC-3 stability anchor.
- Context cost: `M` (1261-line file with 6 functions to update; 2 test files to update; 1 OrcaSlicer dispatch; 3 verification runs).
- Authoritative docs:
  - `docs/07_implementation_status.md` lines 317-328 — confirm N1-N13 chain is closed.
  - `docs/DEVIATION_LOG.md` lines 60-76 — confirm what was already implemented.
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — confirm the faithfulness bar.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` lines 2013-2079 — delegate only.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` lines 2290-2298 — delegate only (lazy-empty-`LineJunctions` fallback).
- Verification:
  - `cargo check -p slicer-core --features host-algos --all-targets 2>&1 | tee target/test-output-153-check.log` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log` — dispatch as FACT pass/fail + `inset0:` line.
- Exit condition: `cargo check` passes; `arachne_annulus_split` passes with `inset0: lines=1 closed=1 sizes=[45]` unchanged.

### Step 2: Re-record tapered-wedge fixture and run N1-N4 regression suite

- Task IDs: `none`
- Objective: After the storage restructure, `generate_toolpaths_tapered_wedge` may report a different per-bead line count (the single-`Vec` layout changes how `chain_junctions_for_bead` resolves the same-bead-count edge case). Re-record the self-captured baseline fixture and run the N1-N4 red test suite to confirm no regression.
- Precondition: Step 1 exit condition met (`arachne_annulus_split` passes with same output).
- Postcondition: `generate_toolpaths_tapered_wedge` passes (fixture re-recorded if needed); `outer_wall_closes_for_simple_polygon` passes; all 5 N1-N4 red test binaries pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/tests/generate_toolpaths.rs` — full (446 lines), but the implementer should range-read around the test function (`:276-375`) and the `write_or_compare_baseline` helper (`:204-274`).
  - `crates/slicer-core/tests/arachne_annulus_split.rs` — full (142 lines) — confirm still passing.
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` — read to confirm pre-refactor structure; delete only if re-recording is needed.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` — delete + regenerate via `cargo test` (not a hand edit; the test re-seeds on missing fixture).
  - `crates/slicer-core/tests/generate_toolpaths.rs` — only if the test's assertion text needs updating to match the new fixture's per-bead line counts (unlikely; the test is invariant-asserting, not line-count-asserting).
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — no read needed; the fixture is self-captured, not an OrcaSlicer golden.
- Expected sub-agent dispatches:
  - "Diff `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` against git HEAD; return the changed keys and their old/new values. If the diff is non-empty, return SNIPPETS of the diff." — purpose: decide whether the fixture needs re-recording.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- --nocapture 2>&1 | tee target/test-output-153-ac4.log`; return the per-test pass count and the new `line_counts` array if the fixture was re-recorded." — purpose: AC-4.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-153-ac5.log`; return the per-binary pass count." — purpose: AC-5.
- Context cost: `S` (one fixture re-record + 2 test runs; no source code changes).
- Authoritative docs:
  - `docs/07_implementation_status.md` lines 317-328 — confirm N1-N4 regression status.
  - `docs/DEVIATION_LOG.md` lines 60-76 — confirm D-141, D-142 red tests are the regression anchors.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- --nocapture 2>&1 | tee target/test-output-153-ac4.log` — dispatch as FACT pass/fail + per-test pass count.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-153-ac5.log` — dispatch as FACT pass/fail + per-binary pass count.
- Exit condition: All 5 N1-N4 red test binaries pass; `generate_toolpaths_tapered_wedge` passes (with re-recorded fixture if needed); `outer_wall_closes_for_simple_polygon` passes.

### Step 3: Port `canReverse` (even-line reversal blocking) into `stitch_extrusions`

- Task IDs: `none`
- Objective: Add the `canReverse` parity gate to `stitch_extrusions`. When the group's `is_odd == false` (even lines), only `(End, Start)` and `(Start, End)` merges are permitted (no reversal); reject candidates that would require reversing an even chain. Odd-line groups retain the current 4-way merge behavior.
- Precondition: Step 2 exit condition met (N1-N4 red tests all pass; storage restructure is stable).
- Postcondition: New `arachne_stitch_can_reverse.rs` test passes (AC-6: even-line reversal rejected; AC-N2: odd-line reversal permitted). `arachne_annulus_split` test still passes with the same `inset0: lines=1 closed=1 sizes=[45]` output.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/stitch.rs` — full (249 lines), but the implementer should focus on `:71-99` (entry), `:118-158` (group), `:163-180` (pick_better), `:188-214` (merge_chains).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` lines 22-47 — delegate only.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 160-170 — delegate only (the `!canReverse(nearby) && nearby_would_be_reversed` guard).
  - `crates/slicer-core/src/arachne/pipeline.rs` line 390 — confirm the call site signature is unchanged.
  - `crates/slicer-core/tests/arachne_annulus_split.rs` — full (142 lines) — stability anchor.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/stitch.rs` — `stitch_extrusions` (pass `is_odd` to `stitch_group`), `stitch_group` (accept `is_odd`, reject reversal for even), `merge_chains` (gate reversal on `is_odd`).
  - New `crates/slicer-core/tests/arachne_stitch_can_reverse.rs` — write the test from scratch.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — delegate only.
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — not touched in this step.
- Expected sub-agent dispatches:
  - "Find the `VariableWidthLines` specialization of `canReverse` at `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp:22-30`; return SNIPPETS of the body and the `canConnect` body at `:35-40`. Confirm the return condition: even lines return false, odd lines return true." — purpose: ground the `canReverse` port. Return format: SNIPPETS ≤ 30 lines.
  - "Run `cargo test -p slicer-core --test arachne_stitch_can_reverse -- --nocapture 2>&1 | tee target/test-output-153-ac6.log`; return FACT pass/fail." — purpose: AC-6 + AC-N2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log`; return the `inset0:` debug line and the test result." — purpose: regression anchor.
- Context cost: `S` (one function family to update; one new test; 2 OrcaSlicer dispatch lines; 2 verification runs).
- Authoritative docs:
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — confirm the faithfulness bar includes `stitch_extrusions`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` lines 22-30 — delegate only.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 160-170 — delegate only.
- Verification:
  - `cargo test -p slicer-core --test arachne_stitch_can_reverse -- --nocapture 2>&1 | tee target/test-output-153-ac6.log` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log` — dispatch as FACT pass/fail + `inset0:` line.
- Exit condition: `arachne_stitch_can_reverse` passes; `arachne_annulus_split` still passes with same `inset0` output.

### Step 4: Port tiny-polygon non-closure rule into `finalize_chain`

- Task IDs: `none`
- Objective: Add the `chain_length + dist < 3 * max_stitch_distance` and `chain.size() <= 2` guards to `finalize_chain`. Compute `chain_length` as the sum of Euclidean distances between consecutive junctions along the polyline (XY-only, matching `dist_sq_xy` at `stitch.rs:103-107`). If the sum is below the threshold, leave the chain open (`is_closed = false`) instead of closing it.
- Precondition: Step 3 exit condition met (`canReverse` ported; `arachne_annulus_split` regression-anchored).
- Postcondition: New `arachne_stitch_tiny_polygon.rs` test passes (AC-7: sub-`3*max_gap` chain stays open; AC-N3: `>= 3*max_gap` chain closes). `arachne_annulus_split` still passes; `outer_wall_closes_for_simple_polygon` still passes (simple square fixture enlarged to 10mm × 10mm in the refactor; its outer wall perimeter 4mm is comfortably above `3 * 0.4mm = 1.2mm`).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/stitch.rs` lines 100-241 — focus on `dist_sq_xy` (`:103-107`), `endpoint_pos` (`:109-114`), `stitch_group` (`:118-158`), `finalize_chain` (`:220-241`).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 71-247 — delegate only.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 130-150 — delegate only (the `chain_length + dist < 3 * max_stitch_distance` check and the `chain.size() <= 2` guard).
  - `crates/slicer-core/tests/generate_toolpaths.rs` lines 397-446 — confirm `outer_wall_closes_for_simple_polygon` still passes.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/stitch.rs` — `finalize_chain` (add `chain_length` + `3 * max_gap` guard).
  - New `crates/slicer-core/tests/arachne_stitch_tiny_polygon.rs` — write the test from scratch.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — delegate only.
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — not touched in this step.
- Expected sub-agent dispatches:
  - "Find the `chain_length + dist < 3 * max_stitch_distance` check at `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp:136-141`; return SNIPPETS of the surrounding 15 lines, including the `chain.size() <= 2` guard at `:138`. Confirm the `chain_length` unit (it's the sum of `(make_point(chain[i]) - make_point(chain[i-1])).norm()`, in `coord_t` units, which are scaled mm)." — purpose: ground the tiny-poly rule. Return format: SNIPPETS ≤ 30 lines.
  - "Run `cargo test -p slicer-core --test arachne_stitch_tiny_polygon -- --nocapture 2>&1 | tee target/test-output-153-ac7.log`; return FACT pass/fail." — purpose: AC-7 + AC-N3.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log`; return the `inset0:` debug line and the test result." — purpose: regression anchor.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- outer_wall_closes_for_simple_polygon --nocapture 2>&1 | tee target/test-output-153-ac4b.log`; return FACT pass/fail." — purpose: confirm `outer_wall_closes_for_simple_polygon` still passes (the simple square fixture was enlarged to 10mm × 10mm; its outer wall perimeter 4mm is comfortably above `3 * 0.4mm = 1.2mm`).
- Context cost: `S` (one function to update; one new test; 1 OrcaSlicer dispatch; 3 verification runs).
- Authoritative docs:
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — confirm the faithfulness bar includes `stitch_extrusions`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 71-247 — delegate only.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 130-150 — delegate only.
- Verification:
  - `cargo test -p slicer-core --test arachne_stitch_tiny_polygon -- --nocapture 2>&1 | tee target/test-output-153-ac7.log` — dispatch as FACT pass/fail.
  - `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log` — dispatch as FACT pass/fail + `inset0:` line.
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- outer_wall_closes_for_simple_polygon --nocapture 2>&1 | tee target/test-output-153-ac4b.log` — dispatch as FACT pass/fail.
- Exit condition: `arachne_stitch_tiny_polygon` passes; `arachne_annulus_split` still passes with same `inset0` output; `outer_wall_closes_for_simple_polygon` still passes.

### Step 5: Update `CONTEXT.md`, `docs/DEVIATION_LOG.md`, `docs/adr/0035-arachne-faithful-emission-and-transitions.md` and run final gate

- Task IDs: `none`
- Objective: Delete the "Junction fan" entry from `CONTEXT.md` and add an "Edge junctions" entry. Add `D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS` to `docs/DEVIATION_LOG.md`. Add a cross-reference to packet 153 in `docs/adr/0035-arachne-faithful-emission-and-transitions.md` §"Consequences". Run the final `cargo xtask test -p slicer-core --features host-algos --summary` gate and `cargo clippy`.
- Precondition: Step 4 exit condition met (all stitch faithfulness fixes in place; regression-anchored).
- Postcondition: All 8 ACs green (AC-1 through AC-8); all 3 AC-Ns green (AC-N1 through AC-N3); doc edits landed; gate commands green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `CONTEXT.md` — full (357 lines), focus on §"Terms" and the "Junction fan" entry at `:265-269`.
  - `docs/DEVIATION_LOG.md` — read the header table at `:42-48` to confirm the table format, then read lines 60-76 (D-141, D-142, D-147 entries) for the row style.
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — full (139 lines), focus on §"Consequences" at `:104-118`.
- Files allowed to edit (≤ 3):
  - `CONTEXT.md` — delete "Junction fan" entry, add "Edge junctions" entry.
  - `docs/DEVIATION_LOG.md` — add `D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS` row.
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — add cross-reference to packet 153 in §"Consequences".
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — no read needed; this step is doc edits + final gate.
  - All source files — not touched in this step.
- Expected sub-agent dispatches:
  - "Run `cargo xtask test -p slicer-core --features host-algos --summary 2>&1 | tee target/test-output-153-ac8.log`; return the summary digest (one `test result:` line per test binary, the final PASS/FAIL verdict, and the full-output path)." — purpose: AC-8 final gate.
  - "Run `cargo clippy -p slicer-core --features host-algos --all-targets -- -D warnings 2>&1 | tee target/test-output-153-clippy.log`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: clippy clean.
  - "Run `rg -q 'D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS' docs/DEVIATION_LOG.md`; return FACT (hit/miss)." — purpose: doc impact grep.
  - "Run `rg -q 'Edge junctions' CONTEXT.md`; return FACT (hit/miss)." — purpose: doc impact grep.
  - "Run `rg -q 'packet 153' docs/adr/0035-arachne-faithful-emission-and-transitions.md`; return FACT (hit/miss)." — purpose: doc impact grep.
- Context cost: `S` (3 doc edits; 1 final gate; 1 clippy; 3 doc greps; 1 summary).
- Authoritative docs:
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — read full.
  - `docs/DEVIATION_LOG.md` — read the header table.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask test -p slicer-core --features host-algos --summary 2>&1 | tee target/test-output-153-ac8.log` — dispatch as summary digest.
  - `cargo clippy -p slicer-core --features host-algos --all-targets -- -D warnings 2>&1 | tee target/test-output-153-clippy.log` — dispatch as FACT pass/fail.
  - The 3 doc greps are the doc-impact verification from `packet.spec.md` §"Doc Impact Statement".
- Exit condition: `cargo xtask test -p slicer-core --features host-algos --summary` reports PASS; `cargo clippy` clean; all 3 doc greps return hits; packet ready to move to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Storage restructure: 1261-line file with 6 functions; 2 test files; 1 OrcaSlicer dispatch; 3 verification runs. |
| Step 2 | S | Fixture re-record + 2 test runs; no source changes. |
| Step 3 | S | One function family update; 1 new test; 2 OrcaSlicer dispatches; 2 verification runs. |
| Step 4 | S | One function update; 1 new test; 1 OrcaSlicer dispatch; 3 verification runs. |
| Step 5 | S | 3 doc edits; 1 final gate; 1 clippy; 3 doc greps. |

Aggregate: M (largest step is M, the rest are S). No step exceeds M; no L step. The packet is within the 80k read budget declared in `packet.spec.md` §"Context Discipline Note".

## Packet Completion Gate

- All 5 steps complete with their exit conditions met.
- Every step's verification command dispatched and returned PASS.
- AC-1 through AC-8 (positive) green; AC-N1 through AC-N3 (negative) green.
- `CONTEXT.md`, `docs/DEVIATION_LOG.md`, `docs/adr/0035-arachne-faithful-emission-and-transitions.md` edits landed; 3 doc greps return hits.
- `cargo xtask test -p slicer-core --features host-algos --summary` reports PASS; `cargo clippy -p slicer-core --features host-algos --all-targets -- -D warnings` clean.
- `docs/07_implementation_status.md` updated (via worker dispatch — never edited by loading the full backlog into the implementer's context): add a packet row in the P148-P152 area noting packet 153 as `status: implemented` and the date.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-8 + AC-N1 through AC-N3). Each must return PASS.
- Confirm packet-level verification commands are green (`cargo check`, `cargo clippy`, `cargo xtask test --summary`).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`. Known risks (from `design.md` §"Risks and Tradeoffs"): the `canReverse` gate's effect on the `cube_4color` residual (out of scope; may improve or worsen), the `3 * max_gap` tiny-poly rule's effect on the hexagon test (AC-4 of packet 147; expected not to affect, but noted in Open Questions).
- Confirm the implementer's peak context usage stayed within its declared band (≤ 80k for this packet; ≤ 150k absolute); if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
