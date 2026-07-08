# Implementation Plan: 147-arachne-cross-cutting-closure

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the cross-cutting closure policies in `docs/specs/arachne-parity-N1-N13-plan.md`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.
- **Narrow verification only:** use `cargo test -p <crate> --test <file>` during implementation. `cargo test --workspace` is FORBIDDEN until the closure ceremony (Step 5).
- **Tee all test output** to `target/test-output-f-*.log` and read from the file.
- **OrcaSlicer reads ONLY via sub-agent delegation.** Never load `OrcaSlicerDocumented/` directly.
- **--nocapture must come after --** in cargo test commands (Windows/bash).

## Steps

### Step 1: Fix has_bead sub-run split (#2) + is_closed pre-stitch (#1) — the coupled pair

- Task IDs:
  - `none` (chain closure — parity-audit findings #2 and #1)
- Objective: Restructure `emit_chain_lines` to walk the full chain and append junctions per-edge with the proximity gate (matching canonical `addToolpathSegment` at `SkeletalTrapezoidation.cpp:2198-2234`). Set `is_closed=false` pre-stitch (matching canonical `WallToolPaths.cpp:802`). Remove AC-6 skip in stitch. Verify the N9 hexagon test — if it breaks (7th junction merged), diagnose whether canonical also produces 7 (test assertion too strict) or PNP's odd spokes are too close (separate fix).
- Precondition: A1–E are `status: implemented` (confirmed in Step 0). The open-ring failure `outer_wall_is_closed_ring_for_simple_polygons` is FAILING ("open line with 3 junctions" on wedge_trapezoid).
- Postcondition: The open-ring test is GREEN. The hexagon test is GREEN (or diagnosed + corrected). N1/N2/N3/N4 stay green. No regressions.
- Files allowed to read:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `emit_chain_lines` (lines ~696-810), `chain_junctions_for_bead` (lines ~554-620), `is_closed` sites (lines ~846, ~934).
  - `crates/slicer-core/src/arachne/stitch.rs` — AC-6 skip (line ~76).
  - `crates/slicer-core/tests/arachne_invariants.rs` — the open-ring test oracle.
  - `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` — the hexagon test oracle.
- Files allowed to edit:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (emit_chain_lines + chain_junctions_for_bead + is_closed sites)
  - `crates/slicer-core/src/arachne/stitch.rs` (AC-6 removal)
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` (re-baseline if needed)
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer read of `SkeletalTrapezoidation.cpp:2198-2234` + `:2273-2366`; return LOCATIONS or SUMMARY" — purpose: ground-truth for the has_bead fix.
  - "Delegate OrcaSlicer read of `WallToolPaths.cpp:790-803` + `PolylineStitcher.hpp`; return LOCATIONS or SUMMARY" — purpose: ground-truth for is_closed.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture 2>&1 | tee target/test-output-f-ac3.log`; return FACT pass/fail" — purpose: validate AC-3 (open-ring fix).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --no-fail-fast 2>&1 | tee target/test-output-f-ac4.log`; return FACT pass/fail" — purpose: validate AC-4 (hexagon test).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step1-red-suite.log`; return FACT pass" — purpose: N1-N4 stay green.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths --no-fail-fast 2>&1 | tee target/test-output-f-step1-regression.log`; return FACT pass" — purpose: generate_toolpaths regression.
  - "Run `cargo test -p slicer-core --features host-algos --test stitch --no-fail-fast 2>&1 | tee target/test-output-f-step1-stitch.log`; return FACT pass" — purpose: stitch regression.
  - "Run `cargo check -p slicer-core --all-targets 2>&1`; return FACT pass/fail" — purpose: compile check.
  - "Run `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings 2>&1`; return FACT pass/fail" — purpose: clippy.
- Context cost: `M`
- Authoritative docs:
  - D-147-PARITY-AUDIT-FINDINGS (findings #1 and #2) in `docs/DEVIATION_LOG.md`.
  - OrcaSlicer refs (delegated): `SkeletalTrapezoidation.cpp:2198-2234`, `:2273-2366`, `WallToolPaths.cpp:790-803`, `PolylineStitcher.hpp`.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture 2>&1 | tee target/test-output-f-ac3.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --no-fail-fast 2>&1 | tee target/test-output-f-ac4.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step1-red-suite.log` — FACT pass.
- Exit condition: open-ring test GREEN; hexagon test GREEN (or diagnosed + corrected); N1-N4 stay green; no regressions; `cargo check` + `cargo clippy` clean.

### Step 2: Fix filter_noncentral_regions 4 deviations (#3)

- Task IDs:
  - `none` (chain closure — parity-audit finding #3)
- Objective: Port canonical walk direction (upward only via `next->twin->next`), bead-count recompute (`getOptimalBeadCount(d2b*2)` + `transition_ratio=0`), distance budget (start at 0), distance gate scope (only different-bead-count branch). Matches canonical `SkeletalTrapezoidation.cpp:811-866`.
- Precondition: Step 1 is green (open-ring test + hexagon test + N1-N4).
- Postcondition: N1-N4 stay green. `cargo check` + `cargo clippy` clean. The centrality walk-direction / bead-count / distance-budget / distance-gate fixes are in place.
- Files allowed to read:
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — `dissolve_noncentral_gap` + `filter_noncentral_regions` (lines ~398-480).
- Files allowed to edit:
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer read of `SkeletalTrapezoidation.cpp:811-866`; return LOCATIONS or SUMMARY" — purpose: ground-truth for the filter_noncentral_regions fix.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step2-red-suite.log`; return FACT pass" — purpose: N1-N4 stay green.
  - "Run `cargo test -p slicer-core --features host-algos --test centrality --no-fail-fast 2>&1 | tee target/test-output-f-step2-centrality.log`; return FACT pass" — purpose: centrality regression.
  - "Run `cargo check -p slicer-core --all-targets 2>&1`; return FACT pass/fail" — purpose: compile check.
  - "Run `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings 2>&1`; return FACT pass/fail" — purpose: clippy.
- Context cost: `S`
- Authoritative docs:
  - D-147-PARITY-AUDIT-FINDINGS (finding #3) in `docs/DEVIATION_LOG.md`.
  - OrcaSlicer refs (delegated): `SkeletalTrapezoidation.cpp:811-866`.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step2-red-suite.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test centrality --no-fail-fast 2>&1 | tee target/test-output-f-step2-centrality.log` — FACT pass.
- Exit condition: N1-N4 stay green; centrality regression green; `cargo check` + `cargo clippy` clean.

### Step 3: Fix connectJunctions merge (#4) + is_odd predicate (#5)

- Task IDs:
  - `none` (chain closure — parity-audit findings #4 and #5)
- Objective: Port canonical prev/next junction merge (perimeter_index overlap removal + concatenation, matching `SkeletalTrapezoidation.cpp:2302-2327`). Port canonical is_odd predicate (both endpoints + 0.005mm proximity, matching `:2344-2354`).
- Precondition: Step 2 is green.
- Postcondition: N1-N4 stay green. `cargo check` + `cargo clippy` clean.
- Files allowed to read:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `chain_junctions_for_bead` (lines ~554-620), `connectJunctions` merge (lines ~624-642), `is_odd_single_bead` (lines ~674-706).
- Files allowed to edit:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer read of `SkeletalTrapezoidation.cpp:2302-2327` + `:2344-2354`; return LOCATIONS or SUMMARY" — purpose: ground-truth for merge + is_odd fixes.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step3-red-suite.log`; return FACT pass" — purpose: N1-N4 stay green.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths --no-fail-fast 2>&1 | tee target/test-output-f-step3-regression.log`; return FACT pass" — purpose: generate_toolpaths regression.
  - "Run `cargo check -p slicer-core --all-targets 2>&1`; return FACT pass/fail" — purpose: compile check.
  - "Run `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings 2>&1`; return FACT pass/fail" — purpose: clippy.
- Context cost: `S`
- Authoritative docs:
  - D-147-PARITY-AUDIT-FINDINGS (findings #4 and #5) in `docs/DEVIATION_LOG.md`.
  - OrcaSlicer refs (delegated): `SkeletalTrapezoidation.cpp:2302-2327`, `:2344-2354`.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step3-red-suite.log` — FACT pass.
- Exit condition: N1-N4 stay green; `cargo check` + `cargo clippy` clean.

### Step 4: Fix collapseSmallEdges Pattern B (#7) + generateJunctions transition interpolation (#6)

- Task IDs:
  - `none` (chain closure — parity-audit findings #7 and #6)
- Objective: Add canonical Pattern B (full-quad bypass) to `collapse_small_edges` (matching `SkeletalTrapezoidationGraph.cpp:310-431`). Port canonical `interpolate(low, 1.0-tr, high)` at nonzero `transition_ratio` in `populate_beading_propagation` (matching `SkeletalTrapezoidation.cpp:2091-2127`).
- Precondition: Step 3 is green.
- Postcondition: `arachne_construction_epilogue` green. N1-N4 stay green. `cargo check` + `cargo clippy` clean.
- Files allowed to read:
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `collapse_small_edges` (lines ~346-407).
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` or `pipeline.rs` — `populate_beading_propagation` (transition interpolation site).
  - `crates/slicer-core/tests/arachne_construction_epilogue.rs` — the construction epilogue test oracle.
- Files allowed to edit:
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (collapseSmallEdges Pattern B)
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` or `pipeline.rs` (transition interpolation)
- Expected sub-agent dispatches:
  - "Delegate OrcaSlicer read of `SkeletalTrapezoidationGraph.cpp:310-431`; return LOCATIONS or SUMMARY" — purpose: ground-truth for Pattern B.
  - "Delegate OrcaSlicer read of `SkeletalTrapezoidation.cpp:2091-2127`; return LOCATIONS or SUMMARY" — purpose: ground-truth for transition interpolation.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --no-fail-fast 2>&1 | tee target/test-output-f-ac7.log`; return FACT pass/fail" — purpose: validate AC-7 (construction epilogue).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step4-red-suite.log`; return FACT pass" — purpose: N1-N4 stay green.
  - "Run `cargo check -p slicer-core --all-targets 2>&1`; return FACT pass/fail" — purpose: compile check.
  - "Run `cargo clippy -p slicer-core --all-targets --features host-algos -- -D warnings 2>&1`; return FACT pass/fail" — purpose: clippy.
- Context cost: `S`
- Authoritative docs:
  - D-147-PARITY-AUDIT-FINDINGS (findings #6 and #7) in `docs/DEVIATION_LOG.md`.
  - OrcaSlicer refs (delegated): `SkeletalTrapezoidationGraph.cpp:310-431`, `SkeletalTrapezoidation.cpp:2091-2127`.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --no-fail-fast 2>&1 | tee target/test-output-f-ac7.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-f-step4-red-suite.log` — FACT pass.
- Exit condition: construction epilogue green; N1-N4 stay green; `cargo check` + `cargo clippy` clean.

### Step 5: E2e closure gate + cross-crate fixtures + deviation-log + ADR + closure ceremony

- Task IDs:
  - `none` (chain closure — cross-cutting artifacts)
- Objective: Run the e2e closure gate (AC-1). Re-baseline cross-crate `slicer-runtime` `perimeter_parity` fixtures via `#[ignore]`d `record_*` functions (AC-2). Add `D-147-CHAIN-CLOSURE` + addenda on `D-141` through `D-146` + update `D-147-PARITY-AUDIT-FINDINGS` to Closed. Author ADR `0035-arachne-faithful-emission-and-transitions.md`. Add `CONTEXT.md` glossary gaps. Update `docs/07_implementation_status.md`. Run `cargo xtask test --workspace --summary` (AC-N1 — closure ceremony, the ONE permitted workspace test run).
- Precondition: Steps 1-4 are green (all 7 finding fixes in place, N1-N4 green, no regressions).
- Postcondition: AC-1 (e2e gate) green. AC-2 (cross-crate perimeter_parity) green. AC-N1 (cargo xtask test --workspace --summary) PASS. D-147-CHAIN-CLOSURE present. D-147-PARITY-AUDIT-FINDINGS Closed. ADR 0035 present. CONTEXT.md glossary complete. cargo xtask build-guests --check clean. docs/07 updated.
- Files allowed to read:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — range-read the `record_*` function signatures (`:1101-1854`); do NOT full-read.
  - `docs/DEVIATION_LOG.md` — range-read the `D-11X-*` entries + `D-147-PARITY-AUDIT-FINDINGS` + the `D-147-CHAIN-CLOSURE` insertion point.
  - `docs/adr/0034-arachne-faithful-graph-construction.md` — full (short); ADR 0035 follows it.
  - `CONTEXT.md` — range-read existing glossary entries.
- Files allowed to edit:
  - `docs/DEVIATION_LOG.md` (closure entry + addenda + D-147-PARITY-AUDIT-FINDINGS update)
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW)
  - `CONTEXT.md` (glossary additions)
  - `docs/07_implementation_status.md` (chain closure update)
- (Secondary edits: `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — re-recorded via `record_*`; the JSONs are regenerated, not hand-edited.)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 | tee target/test-output-f-ac1.log`; return FACT pass/fail + the `failures.len()/total_checked` summary line" — purpose: validate AC-1 (e2e closure gate).
  - "Run the `#[ignore]`d `record_*` functions; return FACT pass/fail" — purpose: re-baseline cross-crate fixtures (AC-2).
  - "Run `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output-f-ac2.log`; return FACT pass/fail" — purpose: validate AC-2.
  - "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output-f-neg1.log`; return FACT pass/fail + the `PASS`/`FAIL` verdict + the per-binary `test result:` line count" — purpose: validate AC-N1 (closure ceremony).
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE list" — purpose: guest WASM coherence.
  - "Run `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md`; return FACT pass/fail" — purpose: confirm deviation-log closure entry.
  - "Run `rg -q 'D-147-PARITY-AUDIT-FINDINGS.*Closed' docs/DEVIATION_LOG.md`; return FACT pass/fail" — purpose: confirm D-147-PARITY-AUDIT-FINDINGS updated.
  - "Run `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md`; return FACT pass/fail" — purpose: confirm ADR 0035.
  - "Run `rg -q '### Rib edge\|### Junction fan\|### BeadingPropagation\|### Transition end\|### Local maximum' CONTEXT.md`; return FACT pass/fail" — purpose: confirm CONTEXT.md glossary.
  - "Update `docs/07_implementation_status.md` for the chain closure (M2 Real Arachne N1–N13 parity complete); return FACT pass/fail" — purpose: record the chain closure in the backlog.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/arachne-parity-N1-N13-plan.md` — cross-packet policies (deviation-log supersession, ADR 0035, `cargo xtask test --workspace --summary` closure ceremony).
  - `docs/adr/0034-arachne-faithful-graph-construction.md` — ADR 0035 follows it.
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 | tee target/test-output-f-ac1.log` — FACT pass (AC-1).
  - `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output-f-ac2.log` — FACT pass (AC-2).
  - `cargo xtask test --workspace --summary 2>&1 | tee target/test-output-f-neg1.log` — FACT PASS (AC-N1).
  - `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md` — FACT pass.
  - `rg -q 'D-147-PARITY-AUDIT-FINDINGS.*Closed' docs/DEVIATION_LOG.md` — FACT pass.
  - `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md` — FACT pass.
  - `cargo xtask build-guests --check` — FACT clean.
- Exit condition: AC-1, AC-2, AC-N1 pass; D-147-CHAIN-CLOSURE present; D-147-PARITY-AUDIT-FINDINGS Closed; ADR 0035 present; CONTEXT.md glossary complete; cargo xtask build-guests --check clean; docs/07 updated for the chain closure.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (has_bead + is_closed) | M | Heaviest dispatch: OrcaSlicer delegation for 2 refs + 6+ test runs. |
| Step 2 (filter_noncentral_regions) | S | Single file, single OrcaSlicer ref. |
| Step 3 (connectJunctions merge + is_odd) | S | Single file, single OrcaSlicer ref. |
| Step 4 (collapseSmallEdges + transition interp) | S | Two files, two OrcaSlicer refs. |
| Step 5 (closure artifacts) | M | Heaviest dispatch: `cargo xtask test --workspace --summary` (~11 minutes). |

Aggregate: M + S + S + S + M = M. Step 1 is the critical path.

## Packet Completion Gate

- All steps complete with exit conditions met.
- Packet acceptance criteria green (AC-1 through AC-7, AC-N1 dispatched and returned PASS).
- ALL 7 N1–N4 red tests green (the chain's acceptance oracles).
- A1–E are ALL `status: implemented`.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-147-CHAIN-CLOSURE` present in `docs/DEVIATION_LOG.md` with addenda on `D-141` through `D-146`.
- `D-147-PARITY-AUDIT-FINDINGS` updated to Closed in `docs/DEVIATION_LOG.md`.
- ADR `0035-arachne-faithful-emission-and-transitions.md` present in `docs/adr/`.
- `CONTEXT.md` glossary complete (any A1–E gaps F closed).
- Cross-crate `perimeter_parity` fixtures re-baselined via `record_*` (never read directly).
- `docs/07_implementation_status.md` updated for the chain closure (M2 Real Arachne N1–N13 parity complete) via worker dispatch.
- `packet.spec.md` ready to move to `status: implemented`.
- The Arachne parity N1–N13 chain is COMPLETE.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-7, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm ALL 7 N1–N4 red tests are green (the chain's acceptance oracles).
- Confirm A1–E are ALL `status: implemented`.
- Run `cargo xtask test --workspace --summary` as the closure ceremony (AC-N1); record the `PASS`/`FAIL` verdict + per-binary `test result:` line count. The full output is on disk at `target/test-output.log` for drill-down (never re-run).
- Record the e2e closure gate's `failures.len()/total_checked` summary explicitly (should be `0/N` — all sub-loops close).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
- The Arachne parity N1–N13 chain is COMPLETE; record this in `docs/07_implementation_status.md`.
