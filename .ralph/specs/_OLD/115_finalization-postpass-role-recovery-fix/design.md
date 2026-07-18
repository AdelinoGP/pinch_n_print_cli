# Design: 115_finalization-postpass-role-recovery-fix

## Controlling Code Paths / Likely Surfaces

After packet 113 lands, the inbound role conversion lives in `marshal` (exact names per 113's implementation; locate with `rg`):

- `crates/slicer-wasm-host/src/marshal/leaf.rs` — `convert_extrusion_role` (recovering, ex-layer, ~line 301) and the two lossy variants 113 relocated for finalization/postpass: `finalization_role_wit_to_ir` (~450) and `convert_postpass_role` (~470). The fix deletes the two lossy ones.
- The finalization and postpass call sites that 113 pointed at the lossy variants — repoint to `convert_extrusion_role`. (Pre-113 the lossy functions were `finalization_role_wit_to_ir` ← host.rs:3674 and `convert_postpass_role` ← host.rs:4330; after 113 they live in `marshal/leaf.rs`.)
- `marshal/leaf.rs` `#[cfg(test)] mod tests` — add the round-trip test.
- `crates/slicer-wasm-host/tests/contract/` — add the finalization dispatch round-trip test.

## Neighboring Tests / Fixtures

- Pre-existing outbound-only tests (`*_ir_to_wit_preserves_reserved_builtin_roles`) — leave as-is; they still pass.
- `crates/slicer-wasm-host/tests/common/` dispatch fixtures — use to drive a finalization guest that re-emits a Skirt/PrimeTower entity.

## Architecture Constraints

- Host-only change: `marshal/leaf.rs` + a host test. **No path in this packet feeds the guest WASM build**, so no `cargo xtask build-guests` is required (no `wasm-staleness` constraint). No geometry/mm math (no `coord-system` constraint).
- The recovering converter is already the layer-world behaviour; this packet makes finalization/postpass use the *same* function — completing ADR-0021's "one converter per concept".

## Selected Approach

Collapse to the single recovering `convert_extrusion_role`; delete the two lossy variants. **Rejected**: keeping two converters behind a `recover: bool` flag or a world parameter — that would re-encode the bug as configuration and reintroduce exactly the per-world parameterization ADR-0021 rejected. The divergence is a defect, not a policy.

## Explicit Code Change Surface

- `crates/slicer-wasm-host/src/marshal/leaf.rs` (delete lossy variant; add round-trip test).
- The finalization/postpass call sites in `marshal`/`dispatch.rs`/`host.rs` that 113 wired to the lossy variant (repoint — ≤1 file beyond leaf.rs).
- `crates/slicer-wasm-host/tests/contract/<new>.rs` (finalization dispatch round-trip test).

## Read-Only Context the Implementer Needs

- ADR-0021 §Amendment (root cause + decision).
- The recovering arm of `convert_extrusion_role` (the reference): `Custom(s) if s == PRIME_TOWER_TAG => PrimeTower`, `… SKIRT_TAG => Skirt`, `Custom(s) => Custom(s)`.

## Out-of-Bounds Files

- `slicer-gcode/src/emit.rs` — correct already; do not edit (a tempting but wrong "fix site").
- The rest of `marshal/` (origin, accumulators, out, in_) — 113's territory.
- `target/`, lockfiles, `OrcaSlicerDocumented/**`, guest modules.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-wasm-host --lib marshal::leaf::tests::extrusion_role_round_trip_recovers_builtin_roles`; return FACT pass/fail." (red before fix, green after).
- "Run `cargo test -p slicer-wasm-host --test contract finalization_role_round_trip`; return FACT pass/fail + first failing assertion."
- STAGE_ORDER FACT from `docs/04` (only if the test needs the precise stage name).

## Data and Contract Notes

- Recovering converter (single source): map the 12 base variants 1:1, then `Custom(s) if s == PRIME_TOWER_TAG => PrimeTower`, `Custom(s) if s == SKIRT_TAG => Skirt`, else `Custom(s.clone())`. Tags are `BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG` / `..._SKIRT_TAG`.
- Round-trip test asserts: for `r ∈ {PrimeTower, Skirt}`, `convert_extrusion_role(&ir_to_wit_extrusion_role(&r)) == r`.

## Locked Assumptions and Invariants

- The recovering form is the *correct* behaviour for all worlds (layer already uses it; nothing relies on the lossy form — postpass's typed role is read by no later stage).
- Outbound (IR→WIT) encoding is unchanged; only inbound recovery changes.

## Risks and Tradeoffs

- **Behaviour change** (the point): finalization/postpass roles that were `Custom(tag)` become typed. Risk that some test pinned the *lossy* output — search for assertions expecting `Custom("…/skirt@1")`/`prime_tower` on a finalization/postpass path and update them as part of the fix (they were pinning a bug). Mitigated by running the `slicer-wasm-host` contract bucket.
- Low blast radius: one converter, two call sites, two tests.

## Context Cost Estimate

- Aggregate: **S**. Largest step: the dispatch round-trip test (M-ish authoring, S context).

## Open Questions

- `None.` The bug-vs-intended question is resolved (bug; ADR-0021 amendment). If the contract bucket reveals a test that asserted the old lossy finalization/postpass output, update it in the same step and note it — it was pinning the defect.
