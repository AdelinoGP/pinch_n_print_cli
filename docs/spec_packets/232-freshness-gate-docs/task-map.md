# Task Map: 232-freshness-gate-docs

Single task ID, seven steps: the crosswalk exists because `TASK-343` restates one contract across nine documentation surfaces plus CI, and because the packet is the terminal row of an approved four-packet queue whose upstream behaviour must stay traceable to the doc that describes it.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-343` | `Step 1` | `CLAUDE.md` §"## Guest WASM Staleness (MUST follow)"; `docs/specs/guest-freshness-artifact-verification-plan.md` (C1, C2, C9, C11, R5-3) | none (documentation) | none | `S` | Proves AC-1, AC-2, AC-3. The hand-maintained input-path list and the 2026-07-25 `shared_crates` anecdote lose their referent when packet 231 deletes `shared_input_paths` |
| `TASK-343` | `Step 2` | `docs/03_wit_and_manifest.md` staleness-guard table row + §"### Build & Freshness Contract (Normative)"; R5-12 | none; reads `crates/slicer-runtime/tests/contract/guest_fixture_freshness_tdd.rs` read-only | none | `M` | Proves AC-4, AC-5, AC-6. Two independent gates, each rule attributed; the host-side gate's guest count comes from a FACT dispatch (8 on 2026-08-19, not the plan's 10) |
| `TASK-343` | `Step 3` | `docs/05_module_sdk.md` "**Guest rebuild obligation.**"; `docs/07_implementation_status.md` TASK-146b row; `CLAUDE.md` §"In-Tree Citation Style" | none | none | `S` | Proves AC-8, AC-9, AC-10. Repins two symbols that never existed: `stage_wit_mtime` -> `stage_wit_snapshot`, `compute_shared_mtime` -> `compute_shared_freshness`, both retired by packet 231 |
| `TASK-343` | `Step 4` | `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` §Amendments; `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` | none | none | `S` | Proves AC-11, AC-12, AC-N3. Amends existing ADRs only — no new ADR slot is allocated. ADR-0054 is explicitly **not** touched; packet 231 owns it |
| `TASK-343` | `Step 5` | `docs/specs/guest-freshness-artifact-verification-plan.md` §"CONTEXT.md term"; `.claude/skills/spec-packet-generator/SKILL.md` §"Packet Ownership" | `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`, `.claude/skills/spec-review/SKILL.md`, `CONTEXT.md` | none | `S` | Proves AC-13, AC-14, AC-15. The snippet becomes an exit-code contract (R5-3) with its `<!-- snippet: wasm-staleness -->` marker preserved; existing verbatim copies in other packets are frozen by user ruling |
| `TASK-343` | `Step 6` | `docs/specs/guest-freshness-artifact-verification-plan.md` (R5-10) | `.github/workflows/ci.yml` `test` job; `xtask/src/wit_verify.rs` test skip-guards (conditional) | none | `S` | Proves AC-16, AC-17, AC-N2. `cargo test -p xtask` must sit after `Install wasm-tools`; the wit_verify edit may end as a recorded verified no-op |
| `TASK-343` | `Step 7` | `docs/07_implementation_status.md` §"### Workstream 5 — Governance and closure drift"; `CLAUDE.md` §"Ledger Facts Must Be Re-derived" | none (ledger + workspace gates) | none | `S` | Proves AC-18, AC-N1, AC-N4. Re-derive the free TASK ID at write time and report any renumbering |

Copied from `implementation-plan.md`: aggregate `M`, largest step `M`, no L step, no split required.

## Upstream/downstream crosswalk

| Relationship | Packet | Task ID | What crosses the boundary |
| --- | --- | --- | --- |
| Forward-dep (must be `implemented` first) | `docs/spec_packets/229-wit-verify-declaration-model` | `TASK-340` | The declaration model and canonical-coverage audit this packet's prose describes; also the post-229 state of `xtask/src/wit_verify.rs`'s test skip-guards, which decides whether Step 6 edits that file |
| Forward-dep (must be `implemented` first) | `docs/spec_packets/230-output-based-guest-freshness` | `TASK-341` | Artifact decoding in `--check`, the `EXIT_FRESH` / `EXIT_STALE` / `EXIT_INFRA_ERROR` contract stated in Steps 1, 2 and 5, and the `v2-` fingerprint |
| Forward-dep (must be `implemented` first) | `docs/spec_packets/231-guest-closure-fingerprint` | `TASK-342` | The per-guest dependency-closure input set, the code-inputs-only fingerprint, the deletion of `compute_shared_freshness` / `stage_wit_snapshot` / `shared_input_paths`, and the unconditional pnp_cli rebuild. Packet 231 also owns ADR-0054, which this packet must not touch |
| Downstream | none | — | Terminal row of the queue in `docs/specs/guest-freshness-artifact-verification-plan.md`. The plan file and all four packet directories are committed together per that plan's commit rule |
