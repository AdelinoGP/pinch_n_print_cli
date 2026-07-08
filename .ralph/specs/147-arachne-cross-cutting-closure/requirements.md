# Requirements: 147-arachne-cross-cutting-closure

## Packet Metadata

- Grouped task IDs: **none** (this is the cross-cutting closure packet for the
  Arachne parity N1–N13 chain; provenanced by
  `docs/specs/arachne-parity-N1-N13-plan.md`'s cross-cutting policies — the
  e2e record-only→block-in-F policy, the fixture re-baseline
  distributed-per-packet→F-closes-stragglers policy, the deviation-log
  supersession chain, and ADR 0035).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packets A1 (`141`), A2 (`142`), B (`143`), C (`144`), D (`145`), E (`146`)
each own their slice of the N1–N13 fixes and each re-baseline their own
`crates/slicer-core` fixtures + record their e2e closure delta (record-only,
per `docs/specs/arachne-parity-N1-N13-plan.md`'s cross-cutting policy). But
the deep canonical parity audit (9 sub-agents, D-147-PARITY-AUDIT-FINDINGS)
surfaced 7 divergences that span the chain and are not owned by any single
A1–E finding-fix packet. These 7 findings ARE the cross-cutting closure —
they are the residual gaps preventing the e2e gate from going green. In
addition, three cross-cutting concerns span the whole chain: (1) the
`cube_4color.3mf` end-to-end outer-wall closure gate is the user-visible
acceptance criterion; (2) the cross-crate `slicer-runtime` `perimeter_parity`
fixtures need re-baselining; (3) the deviation-log chain supersession + ADR
0035 need authoring. This packet closes the chain.

This packet does NOT supersede A1–E for their respective finding fixes; it
records the chain closure and owns the cross-cutting artifacts (e2e gate,
cross-crate fixtures, deviation-log closure, ADR 0035).

## In Scope

- **Fix the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS:**
  1. **`has_bead` sub-run split (#2 — PRIME open-ring blocker):** Restructure `emit_chain_lines` to walk the full chain and append junctions per-edge with the proximity gate (matching canonical `addToolpathSegment` at `SkeletalTrapezoidation.cpp:2198-2234`). Files: `generate_toolpaths.rs` (emit_chain_lines + chain_junctions_for_bead).
  2. **`is_closed` pre-stitch (#1 — coupled to #2):** Set `is_closed=false` pre-stitch (matching canonical `WallToolPaths.cpp:802` post-stitch). Remove AC-6 skip in stitch. Verify N9 hexagon test. Files: `generate_toolpaths.rs`, `stitch.rs`.
  3. **`filter_noncentral_regions` 4 deviations (#3):** Port canonical walk direction (upward only via `next->twin->next`), bead-count recompute (`getOptimalBeadCount` + `transition_ratio=0`), distance budget (start at 0), distance gate scope (only different-bead-count branch). Files: `centrality.rs`.
  4. **`connectJunctions` merge divergence (#4):** Port canonical prev/next junction merge (perimeter_index overlap removal + concatenation). Files: `generate_toolpaths.rs`.
  5. **`connectJunctions` is_odd predicate (#5):** Port canonical is_odd predicate (both endpoints + 0.005mm proximity). Files: `generate_toolpaths.rs`.
  6. **`generateJunctions` transition interpolation (#6):** Port canonical `interpolate(low, 1.0-tr, high)` at nonzero `transition_ratio` in `populate_beading_propagation`. Files: `generate_toolpaths.rs` or `pipeline.rs`.
  7. **`collapseSmallEdges` Pattern B (#7):** Add canonical Pattern B (full-quad bypass) to `collapse_small_edges`. Files: `graph.rs`.
- **Re-green the `cube_4color.3mf` e2e outer-wall closure gate**:
  `cube_4color_arachne_outer_walls_close_end_to_end`
  (`crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145`). This
  was record-only across A1–E; F blocks on green. The finding fixes above are
  the direct cause of the e2e gate failure (finding #2 is the prime blocker).
- **Re-baseline the cross-crate `slicer-runtime` `perimeter_parity` fixtures**:
  re-record via the `#[ignore]`d `record_*` functions
  (`crates/slicer-runtime/tests/integration/perimeter_parity.rs:1101-1854`):
  `record_tapered_wedge` (`:1701`), `record_narrow_strip_widening` (`:1744`),
  `record_max_bead_count_cap` (`:1781`), `record_complex_multi_feature`
  (`:1824`), `record_cube_4color_arachne` (`:1854`). NEVER read the big JSONs
  directly — re-record via the `record_*` functions. The fixtures:
  `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`,
  `complex_multi_feature`, `cube_4color_arachne`.
- **Deviation-log closure entry**: `D-147-CHAIN-CLOSURE` (new ID) documenting
  the chain closure (all N1–N13 fixes in place, e2e closure gate green),
  with addenda on each of `D-141-JUNCTION-BANDS`,
  `D-142-CONNECTJUNCTIONS-EMISSION`, `D-143-TRANSITION-ENDS`,
  `D-144-ANGLE-FUDGE-NONCENTRAL`, `D-145-LOCAL-MAXIMA-EPILOGUE`,
  `D-146-POSTPROCESS-ORDER` noting the chain is closed. Supersession pattern
  (new ID + addenda, no in-place edits to A1–E's narratives).
- **ADR `0035-arachne-faithful-emission-and-transitions.md`** (NEW) in
  `docs/adr/`: records the architectural decision for the chain — canonical
  `generateJunctions`/`connectJunctions` emission (A1/A2), transition ends +
  `generateExtraRibs` (B), `filterNoncentralRegions` + configured angle (C),
  local maxima + construction epilogue (D), canonical post-process order (E),
  superseding the PNP "ADAPTATION" divergence documented in 113c's ADR 0034.
  Authored alongside F's closure. Next free ADR number after 0034.
- **`cube_4color_arachne_outer_walls_close_end_to_end` as a permanent test**:
  confirm the test (already at `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145`)
  is green and remains a permanent regression guard. F does NOT write a new
  test — it confirms the existing one is green.
- **`CONTEXT.md` glossary additions** (if not already done by A1–E): central/
  spine edge, rib edge, quad (Arachne), junction fan, domain-start,
  `getNextUnconnected`, `BeadingPropagation`, `getBeading`, transition end,
  `filterNoncentralRegions`, local maximum, `separateOutInnerContour`. F
  adds any that A1–E didn't carry.
- **`cargo xtask test --workspace --summary` closure ceremony**: the ONE
  permitted `cargo test --workspace` run, per
  `docs/specs/arachne-parity-N1-N13-plan.md` test discipline. F runs it as
  AC-N1.

## Out of Scope

- **N1–N13 finding fixes** — owned by A1 (`141`), A2 (`142`), B (`143`),
  C (`144`), D (`145`), E (`146`). F owns the 7 deferred parity-audit
  findings from D-147-PARITY-AUDIT-FINDINGS (these span the chain, not owned
  by any single A1–E packet).
- **`slicer-core` fixture re-baselines** — owned by A1–E per-packet (each
  re-baselines only the fixtures its own stage touches). F owns ONLY the
  cross-crate `slicer-runtime` `perimeter_parity` fixtures.
- **New WIT/IR schema changes** — F owns no schema changes (A1–E own their own
  scope decisions; E may have added `arachne-params` WIT fields for the
  distance gates).
- **`OrcaSlicerDocumented/` C++ oracle build** — declined.

## Authoritative Docs

- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies
  (e2e record-only→block-in-F, fixture re-baseline
  distributed-per-packet→F-closes-stragglers, deviation-log supersession
  pattern, ADR 0035, `cargo xtask test --workspace --summary` closure ceremony).
- `docs/DEVIATION_LOG.md` — all `D-11X-*` entries (A1–E's); read full; F adds
  the chain-closure addendum.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full (short);
  ADR 0035 follows it.
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — F
  carries this contract forward verbatim (even though F owns no new OrcaSlicer
  refs, the contract governs any diagnostic reads F's implementer makes).

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `has_bead` / `addToolpathSegment`: `SkeletalTrapezoidation.cpp:2198-2234` (proximity-gated append) + `:2273-2366` (full-chain do-while walk) + `:2302-2327` (prev/next junction merge by perimeter_index overlap removal)
- `is_closed`: `WallToolPaths.cpp:790-803` (post-stitch closure) + `PolylineStitcher.hpp` (stitch never inspects is_closed)
- `filter_noncentral_regions`: `SkeletalTrapezoidation.cpp:811-866` (walk direction + getOptimalBeadCount recompute + transition_ratio=0 + distance budget at 0 + distance gate scope)
- `connectJunctions` merge: `SkeletalTrapezoidation.cpp:2302-2327` (perimeter_index overlap removal + concatenation)
- `connectJunctions` is_odd: `SkeletalTrapezoidation.cpp:2344-2354` (both endpoints + 0.005mm proximity)
- `generateJunctions` transition interpolation: `SkeletalTrapezoidation.cpp:2091-2127` (interpolate at transition_ratio)
- `collapseSmallEdges` Pattern B: `SkeletalTrapezoidationGraph.cpp:310-431` (Pattern A middle-edge-only + Pattern B full-quad bypass)

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (e2e closure gate green), `AC-2` (cross-crate
  `perimeter_parity` fixtures re-baselined and green) from `packet.spec.md`.
- Negative cases: `AC-N1` (`cargo xtask test --workspace --summary` PASS — the
  closure ceremony).
- Cross-packet impact: F closes the chain. When F is `status: implemented`, the
  Arachne parity N1–N13 chain is complete.
- Refinements not captured in Given/When/Then:
  - F's e2e closure gate is the user-visible acceptance criterion for the chain
    (the audit's core symptom: "output still visibly worse than canonical Orca").
    If it's still red after A1–E, F diagnoses the residual — it is NOT a finding
    fix; F surfaces the gap and either files a follow-up packet or fixes the
    residual in-scope if it's a cross-cutting integration issue.
  - F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity`
    fixtures; `slicer-core` fixtures are owned by A1–E per-packet.
  - F's deviation-log closure entry (`D-147-CHAIN-CLOSURE`) has addenda on each
    of the 6 chain entries (`D-141` through `D-146`), not in-place edits.
  - ADR 0035 is the next free ADR number after 0034 (confirmed in
    `docs/specs/arachne-parity-N1-N13-plan.md`).
  - `cargo xtask test --workspace --summary` is the ONE permitted
    `cargo test --workspace` run, per the test-discipline contract.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-f-ac1.log` | AC-1: e2e closure gate green | FACT pass/fail; SNIPPETS ≤ 20 lines on failure (the `failures.len()/total_checked` summary line) |
| `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 \| tee target/test-output-f-ac2.log` | AC-2: cross-crate perimeter_parity fixtures green | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_invariants -- outer_wall_is_closed_ring_for_simple_polygons --nocapture 2>&1 \| tee target/test-output-f-ac3.log` | AC-3: open-ring failure fixed (has_bead sub-run split) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --no-fail-fast 2>&1 \| tee target/test-output-f-ac4.log` | AC-4: hexagon test passes (is_closed pre-stitch coordinated) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-f-ac5.log` | AC-5+AC-6: N1-N4 red tests stay green after finding fixes | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --no-fail-fast 2>&1 \| tee target/test-output-f-ac7.log` | AC-7: construction epilogue green (collapseSmallEdges Pattern B) | FACT pass/fail |
| `cargo xtask test --workspace --summary 2>&1 \| tee target/test-output-f-neg1.log` | AC-N1: closure ceremony (the ONE permitted `cargo test --workspace`) | FACT pass/fail + summary line + count |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-f-red-suite-green.log` | All 7 N1-N4 red tests green (the chain's acceptance oracles) | FACT pass (expected — confirms A1/A2 closed) |
| `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md` | Deviation log closure entry present | FACT pass/fail |
| `rg -q 'D-147-PARITY-AUDIT-FINDINGS.*Closed' docs/DEVIATION_LOG.md` | D-147-PARITY-AUDIT-FINDINGS addendum updated to Closed | FACT pass/fail |
| `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md` | ADR 0035 present | FACT pass/fail |
| `rg -q '### Rib edge\|### Junction fan\|### BeadingPropagation\|### Transition end\|### Local maximum' CONTEXT.md` | CONTEXT.md glossary additions (any A1-E gaps F closes) | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (mandatory if E added WIT record fields; run unconditionally) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **F owns the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS.** These span the chain (generate_toolpaths.rs, stitch.rs, centrality.rs, graph.rs) and are not owned by any single A1–E packet. F is the cross-cutting closure: the finding fixes + e2e gate + cross-crate fixtures + deviation-log closure + ADR 0035.
- **F cannot close until A1–E are ALL `status: implemented`.** F is the closure gate; if any of A1–E is still `draft` or `active`, F's AC-1 (e2e gate) will fail (the fixes aren't in place).
- **F's e2e closure gate is the user-visible acceptance criterion.** The 7 finding fixes are the direct cause of the e2e gate failure (finding #2 is the prime blocker). Once the fixes are in place, AC-1 should go green.
- **F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity`
  fixtures.** `slicer-core` fixtures are owned by A1–E per-packet. F re-records
  via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.
- **F's deviation-log closure entry (`D-147-CHAIN-CLOSURE`) has addenda on each of the 6 chain
  entries**, not in-place edits. Supersession pattern.
- **ADR 0035 is the next free ADR number after 0034.**
- **`cargo xtask test --workspace --summary` is the ONE permitted
  `cargo test --workspace` run**, per the test-discipline contract. F runs it
  as AC-N1; no other packet in the chain may run it.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (~1854+ LOC)
  is the primary edit target for the fixture re-baseline — range-read the
  `record_*` function signatures (`:1101-1854`); do NOT full-read (the file
  is large; the `record_*` functions are `#[ignore]`d and self-documenting).
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — large JSON files
  (`expected_perimeter_ir.json` can exceed 10MB per 113c's notes); NEVER read
  these directly. Re-record via the `record_*` functions.
- `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145` —
  range-read the `cube_4color_arachne_outer_walls_close_end_to_end` test (the
  e2e gate); do NOT full-read the file (it's ~1229 LOC).
- `docs/DEVIATION_LOG.md` — large; range-read the `D-11X-*` entries (A1–E's)
  + the `D-147-CHAIN-CLOSURE` insertion point; do NOT full-read.
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (F owns no new
  refs; delegate any diagnostic reads), `modules/core-modules/arachne-perimeters/`
  (F's surface is closure artifacts, not module code), `slicer-sdk`/`slicer-wasm-host`
  (no WIT change in F unless E added fields and F confirms the thread-through).
- Sub-agent return-format hints for the heaviest dispatches: the e2e closure
  gate test run should return FACT + the `failures.len()/total_checked`
  summary line (the test prints this at `:1209-1216`). The `cargo xtask test
  --workspace --summary` run should return FACT + the `PASS`/`FAIL` verdict +
  the per-binary `test result:` line count (the `--summary` flag produces a
  compact LLM-friendly digest).