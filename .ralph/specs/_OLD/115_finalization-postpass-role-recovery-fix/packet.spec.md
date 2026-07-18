---
status: implemented
packet: 115_finalization-postpass-role-recovery-fix
task_ids: []
backlog_source: docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md
context_cost_estimate: S
---

# Packet Contract: 115_finalization-postpass-role-recovery-fix

## Goal

Collapse the inbound (WIT→IR) `extrusion-role` converter in `marshal` to the single recovering form so finalization and postpass recover the reserved builtin roles `PrimeTower`/`Skirt` from their `Custom` tags — fixing the latent finalization misclassification — and pin it with round-trip and dispatch regression tests.

## Scope Boundaries

This is a behaviour-changing bugfix, deliberately separated from the behaviour-preserving extraction in packet 113. Packet 113 relocated the divergent inbound role converters into `marshal` verbatim (preserving today's lossy behaviour); this packet deletes the lossy variant and points finalization and postpass at the recovering converter, then adds the regression tests the bug never had. It touches only the inbound role conversion path and its tests — no marshal restructuring, no WIT change, no guest rebuild.

## Acceptance Criteria

Origin/backlog note: latent bug surfaced while implementing packet 113; root cause and pipeline analysis recorded in ADR-0021's 2026-06-16 amendment. No open `docs/07` TASK id.

- **AC-1** — Given the divergence is a bug not a seam, When this packet lands, Then `marshal` exposes one inbound WIT→IR extrusion-role converter — the recovering `convert_extrusion_role` (marshal/leaf.rs) — the two lossy variants `finalization_role_wit_to_ir` and `convert_postpass_role` are deleted, and the finalization and postpass call sites use the recovering one. | `! rg -n 'fn (finalization_role_wit_to_ir|convert_postpass_role)\b' crates/slicer-wasm-host/src/marshal/` and `rg -c 'fn convert_extrusion_role\b' crates/slicer-wasm-host/src/marshal/leaf.rs` (expect `1`)

- **AC-2** — Given the recovering converter, When `convert_extrusion_role(&ir_to_wit_extrusion_role(&r))` runs for `r ∈ {PrimeTower, Skirt}`, Then the result is the original typed variant (round-trip identity restored), not `Custom`. | `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal::leaf::tests::extrusion_role_round_trip_recovers_builtin_roles 2>&1 | tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log`

- **AC-3** — Given a finalization guest that re-emits a `Skirt` (and a `PrimeTower`) entity, When the finalization stage commits the result back to IR, Then the committed `PrintEntity.role` is `ExtrusionRole::Skirt` / `ExtrusionRole::PrimeTower` (the typed variant), not `Custom("…/skirt@1")`. | `mkdir -p target && cargo test -p slicer-wasm-host --test contract finalization_role_round_trip 2>&1 | tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log`

### Negative Test Cases

- **AC-N1** — Given the **AC-3 finalization contract test** written first (TDD red), When run against the pre-115 (packet-113) code, Then it FAILS (the committed role is `Custom("…/skirt@1")`), proving it exercises the bug; it passes after the fix. (The AC-2 unit round-trip targets `convert_extrusion_role` directly, which already recovered builtin tags pre-115 — so AC-2 is a permanent guard on the surviving converter, green pre- and post-fix, not a falsifier. The genuine RED→GREEN regression is AC-3.) | (manual gate — record the AC-3 red-then-green transition; no standing command)

## Verification (gate subset)

- `cargo check --workspace --all-targets`
- `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal::leaf 2>&1 | tee target/test-output.log; rg '^test result' target/test-output.log`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/adr/0021-…origin-bucket.md` §"Amendment (2026-06-16)" — the root-cause analysis and the decision that the divergence is a bug.
- `docs/04_host_scheduler.md` — STAGE_ORDER confirming `PostPassLayerFinalization` runs before `PostPassGCodeEmit` (why the finalization loss is consequential).
- The builtin role tags `BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG` / `..._SKIRT_TAG` and the layer-world recovery in `convert_extrusion_role` (the correct reference behaviour).

## Doc Impact Statement (Required)

None beyond this packet. ADR-0021's amendment (authored when 113 was refined) already records the decision; on close, optionally note the fix landed.

## Prerequisites / Blockers

- **Blocked by packet 113.** The inbound role converters must already live in `marshal` (113 relocates them). Do not start 115 until 113 closes.

## Deviations

- **[AC-1 / postpass call-site — recovers at the collect step, not the push site]** — The spec said "point the postpass inbound role conversion at `convert_extrusion_role`". As implemented, postpass `push_move` (host.rs:3405) stores the raw WIT `cmd.role` and recovery via `convert_extrusion_role` happens at the existing downstream site `marshal/out.rs:539`. The deleted `convert_postpass_role` was a WIT→WIT field-identity cast (postpass role *is* the layer role post-remap), so this is behaviour-preserving and still routes through the single recovering converter — converting once at the collect step rather than redundantly at the push site. This also corrects ADR-0021's amendment, which had characterized the postpass converter as a lossy WIT→IR map: postpass recovered downstream all along, so only finalization was the genuine bug. Verified firsthand: `out.rs:539` = `convert_extrusion_role(&cmd.role)`.

- **[AC-N1 — only AC-3 was the genuine RED→GREEN falsifier]** — implementation-plan Step 1 said "both new tests FAIL" before the fix. In practice only the AC-3 finalization contract test was RED (committed role `Custom("…/skirt@1")` → typed `Skirt` after the fix); the AC-2 unit round-trip was GREEN pre- and post-fix because it targets `convert_extrusion_role`, which already recovered builtin tags. AC-2 is a permanent guard on the surviving converter, not a falsifier. AC-N1 wording corrected accordingly.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
