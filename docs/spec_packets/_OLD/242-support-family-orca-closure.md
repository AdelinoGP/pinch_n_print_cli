---
status: implemented
packet: 242-support-family-orca-closure
task_ids:
  - TASK-538
  - TASK-539
  - TASK-540
  - TASK-541
  - TASK-542
  - TASK-543
  - TASK-544
  - TASK-545
  - TASK-546
  - TASK-547
  - TASK-548
  - TASK-549
---

# 242-support-family-orca-closure

## Goal

Close the support-family Orca sequence: prove the inherited 224 invariant suite against the
post-237..241 tree, produce the absorbed 218 e2e `;TYPE:` evidence, dispose every gap-register
row, deviation DEV-141..146, and divergence entry in writing, record all supersessions, close
TASK-335 here and only here, and pass the final human differential gate.

## Problem Statement

Packet 224-support-family-orca-closure closed prematurely twice (2026-08-17 retracted; the
2026-08-20 close left a zero-match `--exact` filter, a deleted-but-recreated negative test, and
two vacuous evidence tests, all amended in-session). Its amended ACs, the gap register
(`docs/specs/_OLD/support-parity-gap-register.md`), the parity audit, and
`docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` are inherited
by this packet, which supersedes 224 and closes the sequence for real: the eleven dependency
packets named in `packet.spec.md`'s frontmatter (237, 238a, 238b, 238c, 239a, 239b, 239c, 239d,
240a, 240b, 241 — the former 239 is superseded by 239a/239b/239c/239d, and the former 240 was
split into 240a + 240b) each change support behavior, so every inherited
closure claim must be re-proven against the post-dependency tree, every routed gap/deviation/
divergence must come back dispositioned in writing, and the absorbed 218 e2e `;TYPE:` evidence
must finally exist. TASK-335 closes here and only here. This is one coherent slice because
closure claims stand or fall together: an un-dispositioned register row invalidates the
differential inspection that the final gate signs.

All eleven direct dependencies are `implemented` as re-derived 2026-09-07. Packet 241's
human-override closure originally left AC-N2 red; implemented packet
241b-support-plan-ownership-seam subsequently restored `traditional_family_tdd` to green and
closed DEV-167. Packet 242 consumes that repaired state and must re-check 241b alongside its
eleven direct dependencies before execution.

## Architecture Constraints

- **Closure-only surface.** This packet audits, records, and re-proves. Production-code edits
  are forbidden except `crates/pnp-cli/src/visual_debug_gcode.rs` when AC-7's new test fails for
  a real parser/renderer reason; every other failure routes back to its owning packet (237..241)
  or becomes a `[BLOCK]`/written waiver — never an in-passing fix.
- **Invariant 16 is enforced by count, not by trust.** Each inherited wrapper runs in its own
  `--exact` invocation and asserts `1 passed`; AC-7 does the same for its standalone pnp-cli test
  binary. The eight wrappers and the `slicer-runtime --test integration` target were re-resolved
  on 2026-09-07. A future wrapper rename turns its command red instead of silently filtering to
  zero.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
  (This packet's normal surface does not feed guest WASM; the check is required before
  attributing ANY guest/parity failure during the closure runs — E4/G-24 — and before the Step 8
  whole-suite ceremony.)
- **E1/E2/E3 discipline.** The two evidence tests stay invariant-only halves; judgement lives in
  the written records below. Golden reblessing is out of scope; if a dependency packet's change
  invalidates a golden, that packet owns it. No Orca-derived constant may be hardcoded into any
  test; no test may read `tmp/*_Orca.gcode` (locked 224 gate shape).
- **E5 totals discipline.** The whole-suite green claim comes only from
  `cargo xtask test --workspace --summary` output in
  `target/test-output.log`; fail-fast truncation has twice produced false greens.

## Data and Contract Notes

- IR/manifest contracts: none added or changed. The packet consumes `SupportPlanIR` /
  `SupportIR` / `ExecutionPlan` shapes as they exist post-241; no schema bump.
- WIT boundary: untouched. If any closure run surfaces a WIT-linked failure, E4 freshness rules
  apply before attribution; a real mismatch routes to the owning packet.
- Determinism/scheduler constraints: `differential_evidence`'s structural invariants depend on
  deterministic family routing and serial/parallel determinism (plan invariants 12/13);
  re-running them after the 239a/239b anchored host-seam + WIT-transport enablement must not
  regress ordering guarantees —
  the suite itself is the tripwire.
- Human-owned reference inputs/artifacts: `tmp/p242-orca-tree.gcode`,
  `tmp/p242-orca-normal.gcode`, `tmp/p242-orca-tree-raft.gcode`,
  `tmp/p242-orca-normal-raft.gcode`, their matching patched `.3mf` and `.gcode.3mf` files, and
  `tmp/p242-vd-{pnp,orca}-{tree,normal}.json`. Render outputs are
  `target/vd-p242-{pnp,orca}-{tree,normal}`. These names are identical to packet.spec.md AC-2 and
  implementation-plan.md Step 6; packet 239/240 artifacts do not satisfy this fresh-set gate.

## Locked Assumptions and Invariants

- The eight wrapper names and eight separate `1 passed` assertions are locked for this packet's
  lifetime; a rename upstream is a deliberate breakage of this AC and must update both sides
  consciously.
- No test reads `tmp/*_Orca.gcode`; no Orca-derived constant enters any test (224 locked gate
  shape stands unchanged).
- Parity claims remain limited to termination, coverage, collision freedom, interfaces,
  independent heights; exact path identity is never claimed.
- Rafts are a front-contiguous positive-index prefix (`GlobalLayer.is_raft`) below the shifted
  model Z stack; band output is raft-only, bottom-shell classification starts at the first
  non-raft layer, and raft paths use canonical `;TYPE:Support` labels. The independent-heights
  axis identifies raft bands by marker/Z membership, never by a custom label.

## Risks and Tradeoffs

- **E1/E2/E3 violations recurring** (the 224 failure modes): mitigated by the written-record
  anchors being grep-verified (AC-2/AC-3/AC-6) and by forbidding golden reblessing here.
- **Zero-match filters (T2):** mitigated by asserted counts everywhere; the measured baseline
  makes any drift visible as a red count, not a green nothing.
- **Fail-fast truncation (T3):** the whole-suite gate uses the mandated
  `cargo xtask test --workspace --summary` entry point and results are read from
  `target/test-output.log`.
- **Stale guests (T4):** freshness check precedes attribution and the Step 8 ceremony.
- **Feature-gated blindness (T5/E6):** the workspace run unifies `host-algos`; if a narrow
  slicer-core run is ever dispatched for diagnosis, it must carry `--features host-algos`.
- **Pre-existing noise misattribution (T10):** G-14/G-25 and G-15 are audited from their live
  premises; stale warning/literal counts are not frozen or credited as fixes.
- **Known whole-suite flake:** `instrument_stderr_is_superset_of_core` can fail when JSONL and
  progress output interleave at a newline. It is not waived: a red occurrence keeps the closure
  gate red and routes the atomic-newline-write repair to the emitter owner.
- **Disproved premises resurrected (T11):** the out-of-scope list names them explicitly.
- Tradeoff: asserting eight exact `1 passed` results couples the AC to the wrapper inventory; accepted
  because the opposite (unasserted filter) is precisely how 224's false green happened.
