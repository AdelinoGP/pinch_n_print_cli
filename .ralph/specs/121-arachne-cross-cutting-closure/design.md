# Design: 121-arachne-cross-cutting-closure

## Controlling Code Paths

- Primary code path (e2e gate): `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145` (`cube_4color_arachne_outer_walls_close_end_to_end`) — the user-visible acceptance criterion for the chain; record-only across A1–E, blocking in F.
- Primary code path (fixture re-baseline): `crates/slicer-runtime/tests/integration/perimeter_parity.rs:1101-1854` (the `#[ignore]`d `record_*` functions) — F re-records the cross-crate `perimeter_parity` fixtures.
- Neighboring code path: `docs/DEVIATION_LOG.md` (the `D-11X-*` chain) — F adds the closure entry + addenda.
- Neighboring code path: `docs/adr/0034-arachne-faithful-graph-construction.md` — ADR 0035 follows it.
- OrcaSlicer comparison surface: F owns NO new OrcaSlicer parity refs (A1–E own the chain's refs). See `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

- Packet-specific constraint: **F owns NO finding fixes.** N1–N13 are owned by A1–E. F is the cross-cutting closure: e2e gate, cross-crate fixtures, deviation-log closure, ADR 0035. If the e2e gate is still red after A1–E, F diagnoses the residual — it is NOT a finding fix; F surfaces the gap and either files a follow-up packet or fixes the residual in-scope if it's a cross-cutting integration issue (e.g., a stage ordering issue across the chain, not a finding-level divergence).
- Packet-specific constraint: **F cannot close until A1–E are ALL `status: implemented`.** F is the closure gate; if any of A1–E is still `draft` or `active`, F's AC-1 (e2e gate) will fail.
- Packet-specific constraint: **`cargo xtask test --workspace --summary` is the ONE permitted `cargo test --workspace` run**, per the test-discipline contract. F runs it as AC-N1; no other packet in the chain may run it.
- Packet-specific constraint: **WASM staleness MAY apply** if E added `arachne-params` WIT record fields. F runs `cargo xtask build-guests --check` unconditionally (AC-N1's closure ceremony includes the freshness gate). The `wasm-staleness` snippet is included conditionally.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Packet-specific constraint: **F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity` fixtures.** `slicer-core` fixtures are owned by A1–E per-packet. F re-records via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.

## Code Change Surface

- Selected approach: F is a closure packet — no finding fixes, only cross-cutting artifacts. The e2e gate is re-greened (diagnose + fix residual cross-cutting integration issues in-scope, or file a follow-up packet for finding-level residuals); the cross-crate fixtures are re-recorded via `record_*`; the deviation-log closure entry + addenda are appended; ADR 0035 is authored.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,narrow_strip_widening,max_bead_count_cap,complex_multi_feature,cube_4color_arachne}/expected_perimeter_ir.json` (and any sibling fixture files) — re-recorded via the `#[ignore]`d `record_*` functions (`perimeter_parity.rs:1101-1854`). NEVER read directly.
  - `docs/DEVIATION_LOG.md` — new `D-121-CHAIN-CLOSURE` entry + addenda on `D-116A` through `D-120` (no in-place edits).
  - `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) — the chain's architectural decision.
  - `CONTEXT.md` — glossary additions for any terms A1–E didn't carry (central/spine edge, rib edge, quad, junction fan, domain-start, `getNextUnconnected`, `BeadingPropagation`, `getBeading`, transition end, `filterNoncentralRegions`, local maximum, `separateOutInnerContour`).
- Rejected alternatives:
  - **F as a finding-fix packet** — rejected (F owns NO finding fixes; A1–E own their slices. F is closure only.)
  - **Distribute the e2e gate + cross-crate fixtures across A1–E** — rejected during grilling (user decision: dedicated Packet F for closure). A1–E focus on their red tests + per-packet fixtures; F owns the cross-cutting artifacts.
  - **Run `cargo test --workspace` in every packet** — rejected (test discipline: only at Packet F's closure ceremony via `cargo xtask test --workspace --summary`).

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — role: re-record the cross-crate `perimeter_parity` fixtures via the `#[ignore]`d `record_*` functions; expected change: run the `record_*` functions to regenerate the JSONs (the `record_*` functions themselves are unchanged; F runs them).
- `docs/DEVIATION_LOG.md` — role: closure entry + addenda; expected change: append `D-121-CHAIN-CLOSURE` + one-line addenda on `D-116A` through `D-120`.
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) — role: the chain's ADR; expected change: NEW file.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145-1229` — purpose: the e2e gate test (AC-1 oracle).
- `docs/specs/arachne-parity-N1-N13-plan.md` — full; cross-packet policies.
- `docs/DEVIATION_LOG.md` — range-read the `D-11X-*` entries (A1–E's) + the `D-121-CHAIN-CLOSURE` insertion point; do NOT full-read (large file).
- `docs/adr/0034-arachne-faithful-graph-construction.md` — full (short); ADR 0035 follows it.
- `.ralph/specs/116a-arachne-beading-propagation-and-junction-bands/` through `.ralph/specs/120-arachne-postprocess-order-and-remove-small-simplify/` — range-read each `packet.spec.md` frontmatter + `requirements.md` §Problem Statement (SUMMARY-level); purpose: confirm A1–E are `status: implemented` and their acceptance oracles are green before F closes.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — F owns no new OrcaSlicer refs; delegate any diagnostic reads via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*/expected_perimeter_ir.json` — large JSONs (can exceed 10MB); NEVER read directly. Re-record via the `record_*` functions.
- `crates/slicer-core/src/*` — A1–E's scope; F does not touch `slicer-core` (no finding fixes).
- `crates/slicer-core/tests/fixtures/arachne/*.json` — A1–E's per-packet scope; F does not re-baseline `slicer-core` fixtures.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT pass/fail + the `failures.len()/total_checked` summary line (the test prints this at `:1209-1216`)" — purpose: validate AC-1 (e2e closure gate).
- "Run `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1`; return FACT pass/fail" — purpose: validate AC-2 (cross-crate fixtures).
- "Run `cargo xtask test --workspace --summary 2>&1`; return FACT pass/fail + the `PASS`/`FAIL` verdict + the per-binary `test result:` line count" — purpose: validate AC-N1 (closure ceremony).
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — all 7 N1-N4 red tests green, confirms A1/A2 closed)" — purpose: confirm the chain's acceptance oracles are green.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE list" — purpose: guest WASM coherence (mandatory if E added WIT record fields; run unconditionally per AC-N1's closure ceremony).
- "Read `.ralph/specs/116a-arachne-beading-propagation-and-junction-bands/packet.spec.md` frontmatter; return FACT status (draft/active/implemented)" — purpose: confirm A1 is `status: implemented` before F closes. (Repeat for 116b, 117, 118, 119, 120 — 6 dispatches total, or one batched SUMMARY.)
- "Read `docs/DEVIATION_LOG.md`'s `D-116A-JUNCTION-BANDS` through `D-120-POSTPROCESS-ORDER` entries; return LOCATIONS (the line numbers for each entry's addendum insertion point)" — purpose: confirm the addendum targets exist.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. F owns no schema changes. If E added `arachne-params` WIT record fields, F confirms the thread-through is intact (read-only confirmation, not an edit).
- WIT boundary considerations: **none** (unless E added fields and F confirms the thread-through). F does NOT edit WIT.
- Determinism: F's changes (fixture re-baseline + deviation-log + ADR) are deterministic. The `record_*` functions produce deterministic output given the canonical pipeline.

## Locked Assumptions and Invariants

- F owns NO finding fixes (N1–N13 are A1–E's scope).
- F cannot close until A1–E are ALL `status: implemented`.
- F's e2e closure gate (`cube_4color_arachne_outer_walls_close_end_to_end`) is the user-visible acceptance criterion. If it's still red after A1–E, F diagnoses the residual — NOT a finding fix; F surfaces the gap and either files a follow-up packet or fixes the residual in-scope if it's a cross-cutting integration issue.
- F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity` fixtures; `slicer-core` fixtures are A1–E's per-packet scope.
- F's deviation-log closure entry (`D-121-CHAIN-CLOSURE`) has addenda on each of the 6 chain entries, not in-place edits. Supersession pattern.
- ADR 0035 is the next free ADR number after 0034.
- `cargo xtask test --workspace --summary` is the ONE permitted `cargo test --workspace` run (AC-N1).
- F re-records via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.

## Risks and Tradeoffs

- **The e2e gate may still be red after A1–E.** This is the primary risk. If the canonical pipeline still produces non-closing outer walls, F diagnoses: is it a cross-cutting integration issue (e.g., stage ordering across the chain) or a finding-level residual? If cross-cutting, F fixes in-scope; if finding-level, F files a follow-up packet. F does NOT silently absorb a red e2e gate.
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