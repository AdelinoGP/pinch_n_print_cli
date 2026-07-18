---
status: implemented
packet: 147-arachne-cross-cutting-closure
task_ids:
  - none
---

# 147-arachne-cross-cutting-closure

## Goal

Close the Arachne parity N1–N13 packet chain: fix the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS (the cross-cutting closure — these span the chain, not owned by any single A1–E packet), re-green the `cube_4color.3mf` end-to-end outer-wall closure gate, re-baseline the cross-crate `slicer-runtime` perimeter_parity fixtures (the stragglers after A1–E's per-packet re-baselines), register deviation-log supersession entries for the chain, and author ADR `0035-arachne-faithful-emission-and-transitions.md`.

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

## Architecture Constraints

- Packet-specific constraint: **F owns the 7 deferred parity-audit findings from D-147-PARITY-AUDIT-FINDINGS.** N1–N13 are owned by A1–E; F owns the findings the deep parity audit surfaced AFTER A1–E closed. These span `generate_toolpaths.rs`, `stitch.rs`, `centrality.rs`, `graph.rs` — they are the cross-cutting closure. In addition, F owns the cross-cutting artifacts: e2e gate, cross-crate fixtures, deviation-log closure, ADR 0035.
- Packet-specific constraint: **F cannot close until A1–E are ALL `status: implemented`.** F is the closure gate; if any of A1–E is still `draft` or `active`, F's AC-1 (e2e gate) will fail.
- Packet-specific constraint: **`cargo xtask test --workspace --summary` is the ONE permitted `cargo test --workspace` run**, per the test-discipline contract. F runs it as AC-N1; no other packet in the chain may run it.
- Packet-specific constraint: **WASM staleness MAY apply** if E added `arachne-params` WIT record fields. F runs `cargo xtask build-guests --check` unconditionally (AC-N1's closure ceremony includes the freshness gate). The `wasm-staleness` snippet is included conditionally.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Packet-specific constraint: **F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity` fixtures.** `slicer-core` fixtures are owned by A1–E per-packet. F re-records via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.

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
