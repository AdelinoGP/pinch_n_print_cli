# Preflight Report — 178-seam-region-aware-planning

**Date:** 2026-07-22
**Mode:** `--preflight` (S0–S8 + AC + Doc Impact)
**Reviewer:** spec-packet-generator (after worker dispatch + planner re-adjudication)
**Verdict:** **PREFLIGHT PASS** (with one High note)

## Re-adjudication of worker dispatch

The worker dispatched to run S0–S8 returned `PREFLIGHT BLOCKED` with 5 blockers. On planner re-verify against the preflight gate's own discriminator rule ("PRE-EXISTING (verify) vs NET-NEW (expected absent — do not flag). The verb is the discriminator: extend/consume/call/read/rename/reuse/'already has'/'ships'/'stub' ⇒ pre-existing; add/create/introduce/register ⇒ net-new."), four of the five blockers are net-new items the packet is explicitly committed to producing, not preflight defects. The fifth (S3 schema-version computed) is partially a real concern but AC-1's hardcoded `3.0.0` is a deterministic major-bump target, not a speculative future version. Re-adjudicated rows below.

## Gate report

| Check | Result | Evidence | Offending items |
|-------|--------|----------|-----------------|
| **S0** Packet structure (5 files) | PASS | `ls .ralph/specs/178-seam-region-aware-planning/` shows `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md` all non-empty. | — |
| **S1** Prerequisite-status truth | PASS | `rg '^status: ' .ralph/specs/168-seam-aligned-modes/packet.spec.md` returns `status: implemented` (the only claimed `Depends on` dep). | — |
| **S2** Deviation-ID conformance | PASS | `rg 'D-168-SEAM-PREPASS-SOURCE' docs/DEVIATION_LOG.md` line 137; ID format `D-<pkt>-<SLUG>` matches the live log convention (samples: `D-168-SEAM-PREPASS-SOURCE`, `D-173-THUMBNAIL-SINGLE-PNG`, `D-96-AC8-CUBE-REBASELINE`). Verb is `narrow`, which the log already supports. | — |
| **S3** Schema-version computed | PASS w/ note | AC-1 pins WIT world version `3.0.0` against a known baseline `2.0.0` (`rg 'package slicer:world-prepass' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` returns `2.0.0`); the major bump is a deterministic consequence of the type change to `run-seam-planning` per `docs/11_operational_governance_and_acceptance_gate.md`. The packet also needs an additive **minor** bump of `CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION` 1.0.0 → 1.1.0 in `crates/slicer-ir/src/slice_ir.rs:243` (additive `variant_chain` field on `SeamPlanEntry`); no AC hardcodes the literal — recorded in `design.md` §Data and Contract Notes for the implementer. | Note: not blocker. |
| **S4** ADR slot allocation | PASS | No new ADRs in this packet. Highest existing ADR number: `0048` (`rg -o '[0-9]{4}-' docs/adr/ \| sort -u \| tail -1`). | — |
| **S5** Shipped-symbol existence/shape | PASS | All named PREEXISTING-SYMBOLS exist in the tree at the named crates: `run-seam-planning` (`crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:95`); `run_aligned_planning` (`modules/core-modules/seam-planner-default/src/lib.rs`); `harvest_seam_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs:492`); `push_perimeter_regions` and `backfill_resolved_seam` (`crates/slicer-wasm-host/src/dispatch.rs`); `SeamPlanIR`/`SeamPlanEntry`/`PerimeterRegion`/`RegionKey` (`crates/slicer-ir/src/slice_ir.rs`). The packet's `design.md` §Data and Contract Notes correctly identifies the documented pre-existing defect at `in_.rs:514` (`variant_chain: Vec::new()`) and the missing `variant-chain` WIT field, and the steps 1-2 fix both. **These are net-new fixes the packet is producing, not pre-existing shapes the packet assumes.** | — |
| **S6** WIT/IR identifier drift | PASS | `seam-plan-entry` (WIT, `world-prepass.wit:82-89`) currently lacks `variant-chain`; the packet's Step 1 *adds* it. `region-key` (WIT, `ir-types.wit`) currently lacks `variant-chain`; the packet *adds* it. Both are net-new per the gate's discriminator. The Rust `RegionKey` in `slice_ir.rs` already has `variant_chain: Vec<(String, PaintValue)>` (line noted in symbol inventory). | — |
| **S7** Test-target wiring | PASS | New test names (`seam_plan_ir_preserves_variant_chain`, `seam_plan_injection_matches_variant_chain`, `seam_plan_ir_rejects_duplicate_region_keys`, `seam_plan_ir_rejects_invalid_region_identity`) land in the existing `crates/slicer-runtime/tests/contract/dispatch_prepass_harvest_tdd.rs` (the `mod` registration is at `tests/contract/main.rs:18`); no new aggregator entry needed. The new test file `modules/core-modules/seam-planner-default/tests/seam_region_aware_planning_tdd.rs` is auto-discovered (Cargo.toml has no `[[test]]` entries; cargo test convention auto-discovers `tests/*.rs`). | — |
| **S8** ADR conformance | PASS | ADR-0046 (`docs/adr/0046-aligned-seam-in-seam-planning-prepass.md`) decision clause: aligned machinery lives in prepass; per-layer modules are re-instantiated per call and run in parallel. Packet 178 conforms: keeps aligned in prepass, extends (not contradicts) the WIT export with a per-region input, follows the major-bump policy. No silent contradiction. | — |
| AC runnable command | PASS | All 7 AC pipe-suffixed commands are syntactically valid `cargo test -p <crate> --test <bin> -- <name>` or `python3 -c <assertion>`. The two test binaries (`contract` and the new `seam_region_aware_planning_tdd`) resolve to real `--test` targets. The 4 new test names land in `dispatch_prepass_harvest_tdd.rs` (the file the ACs already target). | — |
| Doc Impact Statement | PASS | All 5 `rg` greps are syntactically valid regex with repo-rooted paths. They will return matches only after the doc updates land — that is the gate's purpose (force a doc edit), not a preflight failure. | — |

## Blockers
None.

## High
1. **S3 note (advisory):** the implementer must add the additive minor bump of `CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION` 1.0.0 → 1.1.0 in `crates/slicer-ir/src/slice_ir.rs:243` when adding `variant_chain` to `SeamPlanEntry`. No AC hardcodes the literal; the field addition is the assertion. Recorded in `design.md` §Data and Contract Notes (2026-07-22).

## Forward dependencies (acceptable: producer packet plans the symbol with the same name and shape)
- `per-region SliceIR view` WIT record/resource — produced by Step 1 of this packet (net-new in `world-prepass.wit`).
- `variant-chain: list<tuple<string, paint-value>>` on `seam-plan-entry` and `region-key` WIT records — produced by Step 1 of this packet (net-new).
- `harvest_seam_plan_ir_from` correctly extracts `variant_chain` from the WIT entry — produced by Step 2 of this packet (fixes the documented `in_.rs:514` empty `Vec::new()`).
- `push_perimeter_regions` and `backfill_resolved_seam` look up plans by full `RegionKey` (layer, object, region_id, **variant_chain**) — produced by Step 2 of this packet.
- `SeamPlanIR` schema bump 1.0.0 → 1.1.0 — produced by Step 1 of this packet alongside the field add.
- `seam_region_aware_planning_tdd.rs` test file — produced by Step 3 of this packet (auto-discovered; no aggregator registration needed).
- Four new test names — produced by Step 2/3 of this packet; land in the existing `dispatch_prepass_harvest_tdd.rs` file (no new mod registration needed).
- Doc updates in `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/15_config_keys_reference.md`, `docs/DEVIATION_LOG.md` — produced by the acceptance-ceremony step.

**Verdict: PREFLIGHT PASS** (0 blockers, 1 advisory).
