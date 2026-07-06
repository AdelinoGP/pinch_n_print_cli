# Requirements: 121-arachne-cross-cutting-closure

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

Packets A1 (`116a`), A2 (`116b`), B (`117`), C (`118`), D (`119`), E (`120`)
each own their slice of the N1–N13 fixes and each re-baseline their own
`crates/slicer-core` fixtures + record their e2e closure delta (record-only,
per `docs/specs/arachne-parity-N1-N13-plan.md`'s cross-cutting policy). But
three cross-cutting concerns span the whole chain and are not owned by any
single finding-fix packet: (1) the `cube_4color.3mf` end-to-end outer-wall
closure gate (`cube_4color_arachne_outer_walls_close_end_to_end` at
`crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145`) is
record-only across A1–E and must BLOCK green in F — it is the user-visible
acceptance criterion for the chain (the audit's core symptom: "output still
visibly worse than canonical Orca"); (2) the cross-crate `slicer-runtime`
`perimeter_parity` fixtures (`tapered_wedge`, `narrow_strip_widening`,
`max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`) are
re-recorded via their `#[ignore]`d `record_*` functions
(`crates/slicer-runtime/tests/integration/perimeter_parity.rs:1101-1854`) and
are NOT owned by any single A1–E packet (they reflect the full pipeline); (3)
the deviation-log chain supersession (`D-116A` through `D-120`) needs a
closure entry (`D-121-CHAIN-CLOSURE`) recording that the chain is complete, and
the architectural decision needs a new ADR (`0035-arachne-faithful-emission-and-transitions.md`)
following `docs/adr/0034-arachne-faithful-graph-construction.md`. This packet
closes the chain.

This packet does NOT supersede A1–E for their respective finding fixes; it
records the chain closure and owns the cross-cutting artifacts (e2e gate,
cross-crate fixtures, deviation-log closure, ADR 0035).

## In Scope

- **Re-green the `cube_4color.3mf` e2e outer-wall closure gate**:
  `cube_4color_arachne_outer_walls_close_end_to_end`
  (`crates/slicer-runtime/tests/executor/cube_4color_arachne.rs:1145`). This
  was record-only across A1–E (each packet recorded its failure delta in its
  commit message); F blocks on green. If the gate is still red after A1–E, F
  diagnoses the residual gap (it is NOT a finding fix — F surfaces the gap and
  either files a follow-up packet or fixes the residual in-scope if it's a
  cross-cutting integration issue, not a finding-level divergence).
- **Re-baseline the cross-crate `slicer-runtime` `perimeter_parity` fixtures**:
  re-record via the `#[ignore]`d `record_*` functions
  (`crates/slicer-runtime/tests/integration/perimeter_parity.rs:1101-1854`):
  `record_tapered_wedge` (`:1701`), `record_narrow_strip_widening` (`:1744`),
  `record_max_bead_count_cap` (`:1781`), `record_complex_multi_feature`
  (`:1824`), `record_cube_4color_arachne` (`:1854`). NEVER read the big JSONs
  directly — re-record via the `record_*` functions. The fixtures:
  `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`,
  `complex_multi_feature`, `cube_4color_arachne`.
- **Deviation-log closure entry**: `D-121-CHAIN-CLOSURE` (new ID) documenting
  the chain closure (all N1–N13 fixes in place, e2e closure gate green),
  with addenda on each of `D-116A-JUNCTION-BANDS`,
  `D-116B-CONNECTJUNCTIONS-EMISSION`, `D-117-TRANSITION-ENDS`,
  `D-118-ANGLE-FUDGE-NONCENTRAL`, `D-119-LOCAL-MAXIMA-EPILOGUE`,
  `D-120-POSTPROCESS-ORDER` noting the chain is closed. Supersession pattern
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

- **N1–N13 finding fixes** — owned by A1 (`116a`), A2 (`116b`), B (`117`),
  C (`118`), D (`119`), E (`120`). F owns NO finding fixes. If the e2e gate is
  still red after A1–E, F diagnoses the residual: if it's a cross-cutting
  integration issue (not a finding-level divergence), F may fix it in-scope;
  if it's a finding-level divergence, F files a follow-up packet.
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

- F owns NO finding fixes, so it has NO new OrcaSlicer parity refs. The chain's OrcaSlicer refs are owned by A1–E (see their `requirements.md` §OrcaSlicer Reference Obligations). F's ADR 0035 references the chain's parity surface but does not introduce new refs. If F's implementer diagnoses a residual e2e gap, any OrcaSlicer reads during diagnosis MUST be delegated per this contract.

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
  - F's deviation-log closure entry (`D-121-CHAIN-CLOSURE`) has addenda on each
    of the 6 chain entries (`D-116A` through `D-120`), not in-place edits.
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
| `cargo xtask test --workspace --summary 2>&1 \| tee target/test-output-f-neg1.log` | AC-N1: closure ceremony (the ONE permitted `cargo test --workspace`) | FACT pass/fail + summary line + count |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-f-red-suite-green.log` | All 7 N1-N4 red tests green (the chain's acceptance oracles) | FACT pass (expected — confirms A1/A2 closed) |
| `rg -q 'D-121-CHAIN-CLOSURE' docs/DEVIATION_LOG.md` | Deviation log closure entry present | FACT pass/fail |
| `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md` | ADR 0035 present | FACT pass/fail |
| `rg -q '### Rib edge\|### Junction fan\|### BeadingPropagation\|### Transition end\|### Local maximum' CONTEXT.md` | CONTEXT.md glossary additions (any A1-E gaps F closes) | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (mandatory if E added WIT record fields; run unconditionally) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **F cannot close until A1–E are ALL `status: implemented`.** F is the
  closure gate; if any of A1–E is still `draft` or `active`, F's AC-1 (e2e
  gate) will fail (the fixes aren't in place).
- **F's e2e closure gate is the user-visible acceptance criterion.** If it's
  still red after A1–E, F diagnoses the residual: if it's a cross-cutting
  integration issue (not a finding-level divergence), F may fix it in-scope;
  if it's a finding-level divergence, F files a follow-up packet. F does NOT
  silently absorb a red e2e gate.
- **F re-baselines ONLY the cross-crate `slicer-runtime` `perimeter_parity`
  fixtures.** `slicer-core` fixtures are owned by A1–E per-packet. F re-records
  via the `#[ignore]`d `record_*` functions; NEVER read the big JSONs directly.
- **F's deviation-log closure entry has addenda on each of the 6 chain
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
  + the `D-121-CHAIN-CLOSURE` insertion point; do NOT full-read.
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