# Requirements: 115_finalization-postpass-role-recovery-fix

## Problem Statement

The WIT `extrusion-role` enum has 12 variants and omits the reserved builtin roles `PrimeTower` and `Skirt`; the IR enum has them. Crossing IR→WIT encodes them as `Custom("<builtin tag>")`. The layer-world WIT→IR converter recovers them (`Custom(tag) => PrimeTower/Skirt`), but the finalization and postpass copies keep `Custom(s) => Custom(s)` — they never recover.

This is a latent bug, traced while implementing packet 113 (full analysis in ADR-0021's 2026-06-16 amendment). `Skirt`/`PrimeTower` entities live in `LayerCollectionIR.ordered_entities` before `PostPassLayerFinalization`. A finalization guest that re-emits paths returns the role as `Custom("…/skirt@1")`; the immediately following `PostPassGCodeEmit` then misclassifies it — `resolve_feedrate` falls back to `outer_wall_speed` instead of `skirt_speed`/`prime_tower_speed`, `orca_type_label` emits `;TYPE:Custom` instead of `;TYPE:Skirt/Brim`, and the skirt-travel-insertion filter (`role == ExtrusionRole::Skirt`) misses the entity entirely. The postpass copy is currently inert (its output feeds `GCodeIR`, which no later stage matches on by typed role) but is the same defect and is fixed for consistency. Existing tests cover only the outbound encoding, so the lossy round-trip is undetected.

## Task Mapping

No open `docs/07` TASK id. Governed by ADR-0021 (amendment). Sibling of the refactor packet 113.

## In Scope

- Delete the two lossy WIT→IR role converters that packet 113 relocated into `marshal` (`finalization_role_wit_to_ir`, `convert_postpass_role`).
- Point the finalization and postpass inbound role conversion at the single recovering `marshal::leaf::convert_extrusion_role`.
- Add a `marshal::leaf` round-trip unit test (PrimeTower/Skirt recovered).
- Add a finalization dispatch contract test: a guest-emitted Skirt/PrimeTower entity commits back as the typed variant.

## Out of Scope

- Any change to the marshal module structure, `OriginBucket`, accumulators, or other converters (packet 113 owns those).
- Any change to `slicer-gcode/emit.rs` — it already classifies correctly *given a typed role*; the bug is upstream in the converter, and fixing the converter restores emit.rs's correct path. No WIT change; no guest rebuild.
- The postpass behavioural surface beyond converter consistency (no postpass module currently reads the typed role).

## Authoritative Docs

- `docs/adr/0021` §Amendment (2026-06-16) — read; the decision and root cause.
- `docs/04_host_scheduler.md` STAGE_ORDER (~174–203) — **delegate** a FACT confirming finalization precedes GCODE_EMIT, if needed.
- Layer-world `convert_extrusion_role` (`host.rs`/`marshal` after 113) — the correct recovery reference.

## Acceptance Summary

Authoritative criteria are AC-1…AC-3 and AC-N1 in `packet.spec.md`. Refinements:

- AC-2/AC-3 must be written **TDD-red first** against the post-113 tree (they fail because the round-trip yields `Custom`), then made green by the fix (AC-N1 documents this transition).
- AC-1's single-converter check is what proves the divergence collapsed rather than a third variant being added.

## Verification Commands

| ID | Command | Delegation hint |
|----|---------|-----------------|
| AC-1 | `! rg -n 'fn (finalization_role_wit_to_ir\|convert_postpass_role)\b' crates/slicer-wasm-host/src/marshal/` and `rg -c 'fn convert_extrusion_role\b' crates/slicer-wasm-host/src/marshal/leaf.rs` | FACT: first empty, second==1 |
| AC-2 | `cargo test -p slicer-wasm-host --lib marshal::leaf::tests::extrusion_role_round_trip_recovers_builtin_roles 2>&1 \| tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log` | FACT: 1 passed |
| AC-3 | `cargo test -p slicer-wasm-host --test contract finalization_role_round_trip 2>&1 \| tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log` | FACT: pass/fail + first failing assertion |
| Gate | `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings` | FACT: exit code + first error |

## Step Completion Expectations (cross-step invariants)

- AC-2/AC-3 tests are authored and confirmed RED before the converter change (AC-N1), then GREEN after. Do not write the fix first.

## Context Discipline Notes (packet-specific)

- Small packet (S). The change is one converter + its call sites + two tests. Do not re-survey the marshal module — 113 already placed everything; locate the lossy variant with `rg` and edit in place.
