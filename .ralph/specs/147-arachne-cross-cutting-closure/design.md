# Design: 147-arachne-cross-cutting-closure

## Controlling Code Paths

- Primary code path (e2e gate): `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145` (`cube_4color_arachne_outer_walls_close_end_to_end`) — the user-visible acceptance criterion for the chain; record-only across A1–E, blocking in F.
- Primary code path (fixture re-baseline): `crates/slicer-runtime/tests/integration/perimeter_parity.rs:1101-1854` (the `#[ignore]`d `record_*` functions) — F re-records the cross-crate `perimeter_parity` fixtures.
- Neighboring code path: `docs/DEVIATION_LOG.md` (the `D-11X-*` chain) — F adds the closure entry + addenda.
- Neighboring code path: `docs/adr/0034-arachne-faithful-graph-construction.md` — ADR 0035 follows it.
- OrcaSlicer comparison surface: F owns NO new OrcaSlicer parity refs (A1–E own the chain's refs). See `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

- Packet-specific constraint: **F owns the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS.** N1–N13 are owned by A1–E; F owns the findings the deep parity audit surfaced AFTER A1–E closed. These span `generate_toolpaths.rs`, `stitch.rs`, `centrality.rs`, `graph.rs` — they are the cross-cutting closure. In addition, F owns the cross-cutting artifacts: e2e gate, cross-crate fixtures, deviation-log closure, ADR 0035.
- Packet-specific constraint: **F cannot close until A1–E are ALL `status: implemented`.** F is the closure gate; if any of A1–E is still `draft` or `active`, F's AC-1 (e2e gate) will fail.
- Packet-specific constraint: **`cargo xtask test --workspace --summary` is the ONE permitted `cargo test --workspace` run**, per the test-discipline contract. F runs it as AC-N1; no other packet in the chain may run it.
- Packet-specific constraint: **WASM staleness MAY apply** if E added `arachne-params` WIT record fields. F runs `cargo xtask build-guests --check` unconditionally (AC-N1's closure ceremony includes the freshness gate). The `wasm-staleness` snippet is included conditionally.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Packet-specific constraint: **F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity` fixtures.** `slicer-core` fixtures are owned by A1–E per-packet. F re-records via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.

## Code Change Surface

- Selected approach: F fixes the 7 deferred parity-audit findings (the cross-cutting closure) + re-greens the e2e gate + re-baselines cross-crate fixtures + deviation-log closure + ADR 0035. The finding fixes are the direct cause of the e2e gate failure (finding #2 is the prime blocker).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **Finding #2 (has_bead sub-run split):** `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `emit_chain_lines` (lines ~696-810) + `chain_junctions_for_bead` (lines ~554-620). Restructure to walk the full chain and append junctions per-edge with proximity gate (matching canonical `addToolpathSegment` at `SkeletalTrapezoidation.cpp:2198-2234`).
  - **Finding #1 (is_closed pre-stitch):** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (lines ~846, ~934) + `crates/slicer-core/src/arachne/stitch.rs` (line ~76, AC-6 skip). Set `is_closed=false` pre-stitch, remove AC-6 skip, verify N9 hexagon test.
  - **Finding #3 (filter_noncentral_regions):** `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (lines ~398-480). Port canonical walk direction, bead-count recompute, distance budget, distance gate scope.
  - **Finding #4 (connectJunctions merge):** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (lines ~624-642). Port canonical perimeter_index overlap removal + concatenation.
  - **Finding #5 (connectJunctions is_odd):** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (lines ~674-706). Port canonical both-endpoints + 0.005mm proximity.
  - **Finding #6 (generateJunctions transition interpolation):** `crates/slicer-core/src/arachne/generate_toolpaths.rs` or `pipeline.rs`. Port canonical `interpolate(low, 1.0-tr, high)` at nonzero `transition_ratio`.
  - **Finding #7 (collapseSmallEdges Pattern B):** `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (lines ~346-407). Add canonical Pattern B (full-quad bypass).
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,narrow_strip_widening,max_bead_count_cap,complex_multi_feature,cube_4color_arachne}/expected_perimeter_ir.json` — re-recorded via the `#[ignore]`d `record_*` functions. NEVER read directly.
  - `docs/DEVIATION_LOG.md` — new `D-147-CHAIN-CLOSURE` entry + addenda on `D-141` through `D-146` + update `D-147-PARITY-AUDIT-FINDINGS` to Closed.
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) — the chain's architectural decision.
  - `CONTEXT.md` — glossary additions for any terms A1–E didn't carry.
- Rejected alternatives:
  - **F as a finding-fix packet** — rejected (F owns NO finding fixes; A1–E own their slices. F is closure only.)
  - **Distribute the e2e gate + cross-crate fixtures across A1–E** — rejected during grilling (user decision: dedicated Packet F for closure). A1–E focus on their red tests + per-packet fixtures; F owns the cross-cutting artifacts.
  - **Run `cargo test --workspace` in every packet** — rejected (test discipline: only at Packet F's closure ceremony via `cargo xtask test --workspace --summary`).

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: findings #1, #2, #4, #5, #6 (emit_chain_lines, chain_junctions_for_bead, is_closed sites, connectJunctions merge, is_odd predicate, transition interpolation).
- `crates/slicer-core/src/arachne/stitch.rs` — role: finding #1 (AC-6 removal).
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — role: finding #3 (filter_noncentral_regions 4 deviations).
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — role: finding #7 (collapseSmallEdges Pattern B).
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — role: re-record the cross-crate `perimeter_parity` fixtures via the `#[ignore]`d `record_*` functions.
- `docs/DEVIATION_LOG.md` — role: closure entry + addenda + D-147-PARITY-AUDIT-FINDINGS update.
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) — role: the chain's ADR.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145-1229` — purpose: the e2e gate test (AC-1 oracle).
- `crates/slicer-core/tests/arachne_invariants.rs` — purpose: the open-ring test (AC-3 oracle).
- `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` — purpose: the hexagon test (AC-4 oracle).
- `crates/slicer-core/tests/arachne_construction_epilogue.rs` — purpose: the construction epilogue test (AC-7 oracle).
- `docs/specs/arachne-parity-N1-N13-plan.md` — full; cross-packet policies.
- `docs/DEVIATION_LOG.md` — range-read the `D-11X-*` entries (A1–E's) + the `D-147-PARITY-AUDIT-FINDINGS` entry + the `D-147-CHAIN-CLOSURE` insertion point; do NOT full-read (large file).
- `docs/adr/0034-arachne-faithful-graph-construction.md` — full (short); ADR 0035 follows it.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate any reads via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — large JSONs (can exceed 10MB); NEVER read directly. Re-record via the `record_*` functions.
- `crates/slicer-core/tests/fixtures/arachne/*.json` — A1–E's per-packet scope; F does not re-baseline `slicer-core` fixtures (unless the finding fixes require it — in which case narrow re-baselining is in-scope).

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT pass/fail + the `failures.len()/total_checked` summary line (the test prints this at `:1209-1216`)" — purpose: validate AC-1 (e2e closure gate).
- "Run `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1`; return FACT pass/fail" — purpose: validate AC-2 (cross-crate fixtures).
- "Run `cargo xtask test --workspace --summary 2>&1`; return FACT pass/fail + the `PASS`/`FAIL` verdict + the per-binary `test result:` line count" — purpose: validate AC-N1 (closure ceremony).
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — all 7 N1-N4 red tests green, confirms A1/A2 closed)" — purpose: confirm the chain's acceptance oracles are green.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE list" — purpose: guest WASM coherence (mandatory if E added WIT record fields; run unconditionally per AC-N1's closure ceremony).
- "Read `.ralph/specs/141-arachne-beading-propagation-and-junction-bands/packet.spec.md` frontmatter; return FACT status (draft/active/implemented)" — purpose: confirm A1 is `status: implemented` before F closes. (Repeat for 142, 143, 144, 145, 146 — 6 dispatches total, or one batched SUMMARY.)
- "Read `docs/DEVIATION_LOG.md`'s `D-141-JUNCTION-BANDS` through `D-146-POSTPROCESS-ORDER` entries; return LOCATIONS (the line numbers for each entry's addendum insertion point)" — purpose: confirm the addendum targets exist.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. F owns no schema changes. If E added `arachne-params` WIT record fields, F confirms the thread-through is intact (read-only confirmation, not an edit).
- WIT boundary considerations: **none** (unless E added fields and F confirms the thread-through). F does NOT edit WIT.
- Determinism: F's changes (fixture re-baseline + deviation-log + ADR) are deterministic. The `record_*` functions produce deterministic output given the canonical pipeline.

## Locked Assumptions and Invariants

- F owns the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS (these span the chain, not owned by any single A1–E packet).
- F cannot close until A1–E are ALL `status: implemented`.
- F's e2e closure gate (`cube_4color_arachne_outer_walls_close_end_to_end`) is the user-visible acceptance criterion. The 7 finding fixes are the direct cause of the e2e gate failure (finding #2 is the prime blocker).
- F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity` fixtures; `slicer-core` fixtures are A1–E's per-packet scope (unless the finding fixes require narrow re-baselining).
- F's deviation-log closure entry (`D-147-CHAIN-CLOSURE`) has addenda on each of the 6 chain entries, not in-place edits. Supersession pattern. `D-147-PARITY-AUDIT-FINDINGS` addendum updated to Closed.
- ADR 0035 is the next free ADR number after 0034.
- `cargo xtask test --workspace --summary` is the ONE permitted `cargo test --workspace` run (AC-N1).
- F re-records via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.

## Risks and Tradeoffs

- **Finding #2 (has_bead sub-run split) is the critical path.** This is the prime open-ring blocker. The fix requires restructuring `emit_chain_lines` to match canonical's full-chain walk with proximity-gated append. Risk: the restructure may affect other bead-index-dependent logic downstream. Mitigation: run the full N1–N4 red test suite after the fix.
- **Finding #1 (is_closed pre-stitch) is coupled to #2.** The fix was applied+reverted this session because it broke the N9 hexagon test (stitch merged a 7th junction). Risk: the hexagon test assertion may be too strict (canonical may also produce 7). Mitigation: verify canonical hexagon junction count via delegated OrcaSlicer read before re-applying.
- **The cross-crate `perimeter_parity` fixtures may drift significantly.** The canonical pipeline produces different output than the PNP "ADAPTATION"; the re-baselined fixtures lock in the canonical output. Risk is contained by the self-capture pattern (the fixtures guard self-regression, not OrcaSlicer ground truth) + the N1–N4 red tests (the real parity oracles).
- **ADR 0035's scope is the whole chain.** Authoring a single ADR for 6 packets' worth of architectural decisions risks an over-long document. Mitigation: ADR 0035 references A1–E's `requirements.md` §Problem Statement for per-finding detail; the ADR itself records the chain-level decision (canonical emission + transitions + post-process, superseding the "ADAPTATION"), not per-finding mechanics.
- **`cargo xtask test --workspace --summary` is expensive** (~11 minutes, >1000 tests). F runs it ONCE as AC-N1; the `--summary` flag keeps the digest compact (per-binary `test result:` line count + `PASS`/`FAIL` verdict). The full output is on disk at `target/test-output.log` for drill-down.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — the cross-crate fixture re-baseline + `cargo xtask test --workspace --summary` closure ceremony, the bulk of the work + the longest single dispatch).
- Highest-risk dispatch: the `cargo xtask test --workspace --summary` run — it takes ~11 minutes and its output could blow budget if the `--summary` digest is mis-shaped. Required return format: `FACT pass/fail + the PASS/FAIL verdict + the per-binary test result: line count`; the full output is on disk at `target/test-output.log` for drill-down (never re-run).

## Open Questions

- [FWD] If the e2e gate is still red after A1–E, is the residual a cross-cutting integration issue (F fixes in-scope) or a finding-level divergence (F files a follow-up packet)? F's implementer diagnoses via `pnp_cli slice --instrument-stderr` (per `docs/17_agent_debugging.md` + the `debug-pipeline` skill) and surfaces the gap. This is a mid-flight decision, not activation-blocking.
- [FWD] Does ADR 0035 need a separate "process lesson" section (like 0034's OrcaSlicer-read-delegation losing caller-loop context)? Likely yes — the chain's grilling decisions (A split into A1/A2, the e2e record-only→block-in-F policy, the fixture re-baseline distributed-per-packet policy) are process lessons worth recording. The implementer decides the ADR's structure.
- [FWD] Does F need to update `docs/07_implementation_status.md` for the chain closure? Yes — F records the chain closure (M2 Real Arachne N1–N13 parity complete) via a worker dispatch (never by loading the full backlog into the implementer's context).

None activation-blocking.