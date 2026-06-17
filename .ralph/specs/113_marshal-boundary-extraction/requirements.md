# Requirements: 113_marshal-boundary-extraction

## Problem Statement

WIT↔IR marshalling in `slicer-wasm-host` is spread across ~40 free functions in two files: the marshal-in projections and marshal-out harvest converters in `host.rs` (5225 LoC) and the postpass converters plus the 230-LoC `deconstruct_layer_ctx` router in `dispatch.rs` (2585 LoC). Tracing one type across the seam means bouncing ~2700 lines inside `host.rs`, then into a second file. Two specific costs motivate this packet:

1. **Stale per-world duplication.** ADR-0002 unified the four worlds' geometry/config Rust types via `bindgen!`'s `with:` remap, which silently made several "per-world" converters byte-identical (`ir_to_wit_extrusion_role` ≡ `finalization_role_ir_to_wit`; `convert_extrusion_role` ≡ `finalization_role_wit_to_ir` ≡ `convert_postpass_role`; `ir_to_wit_expolygon_prepass` ≡ `ir_to_wit_expolygon`). They are dead copies, not live variation — ADR-0002 named their removal as a "Deferred" follow-up.

2. **The bug-prone logic is untestable in isolation.** The origin-attribution rule — guest output re-bucketed to its source region via `(object_id, region_id)` tuples under an all-or-none tagging contract with finite-float validation — is re-implemented three times inside `convert_infill_output` (135 LoC), `convert_perimeter_output` (194 LoC), and `convert_support_output` (113 LoC), and is exercised only through full wasmtime dispatch. A silent regression in identity preservation cannot be caught by a fast unit test today.

ADR-0021 resolves both: one `marshal` module of flat functions over a shared, unit-testable `OriginBucket`.

## Task Mapping

No open `docs/07_implementation_status.md` TASK id covers this work — it originates from the 2026-06-16 architecture-review session and is governed by **ADR-0021**. Closest closed references for context only: TASK-150 (converter widening), TASK-247 (dead dispatch-arm deletion).

## In Scope

- Delete stale per-world converters: `finalization_role_ir_to_wit`, `finalization_role_wit_to_ir`, `finalization_path_ir_to_wit`, `convert_postpass_role`, `ir_to_wit_expolygon_prepass`, `ir_to_wit_expolygons_prepass`; repoint callers to the single unified converter.
- Create `crates/slicer-wasm-host/src/marshal/` with `mod.rs`, `origin.rs`, `out.rs`, `leaf.rs`, `in_.rs`, `accumulators.rs`.
- Introduce `OriginId` struct (replacing `PerimeterRegionOrigin` / `SliceRegionOrigin`), structured `MarshalError`, and `OriginBucket<R>`.
- Move `*Collected` accumulators (`InfillOutputCollected`, `PerimeterOutputCollected`, `SupportOutputCollected`, `SlicePostprocessCollected`, `GcodeOutputCollected`) into `marshal/accumulators.rs`; their builder methods remain on `HostExecutionContext`.
- Move leaf maps, marshal-in projections, and marshal-out converters into `marshal/`; rewrite the three bucketing converters on `OriginBucket`.
- Update `host.rs` Host-trait impls and `dispatch.rs` to call `marshal::*`; `dispatch.rs` keeps the thin per-stage harvest router and all wasmtime mechanics.
- Add `marshal::origin` unit tests covering ordering, all-or-none, anonymous collapse, length mismatch.

## Out of Scope

- Any WIT change (`crates/slicer-schema/wit/**`) — none required; no guest rebuild.
- Host-services trait unification / `slicer:common` remap — packet 114.
- Moving the per-stage harvest match into `marshal` (ADR-0021 rejected this; routing stays in `dispatch.rs`).
- The four `bindgen!` invocations and the runner-trait seams (ADR-0005) — untouched.
- Behaviour changes to any converter's output for valid input.

## Authoritative Docs

- `docs/adr/0021-…origin-bucket.md` (~140 lines) — read in full; it is the spec.
- `docs/adr/0002-wit-marshalling-type-unification.md` (~55 lines) — read the "Deferred" section.
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — skim; rationale for stage-routing placement.
- `docs/02_ir_schemas.md` (> 600 lines) — **delegate** a FACT for the exact `InfillIR`/`InfillRegion`/`PerimeterIR`/`SupportIR` field names; do not read in full.
- `CONTEXT.md` — the "Marshalling boundary" entry (concept only).

## Acceptance Summary

Authoritative criteria are AC-1…AC-6 and AC-N1/AC-N2 in `packet.spec.md`. Measurable refinements:

- AC-1 deletion set is exactly six function names; partial deletion (any one remaining) fails the packet.
- AC-4's "single home" means `any_tagged` appears in `marshal/origin.rs` only — its absence from both `host.rs` and `dispatch.rs` is the falsifier.
- AC-5/AC-N1/AC-N2 test names are normative: `buckets_by_first_seen_origin_order`, `untagged_payload_in_tagged_mode_errs`, `anonymous_mode_collapses_to_one_region`, `length_mismatch_errs` in `marshal::origin::tests`.
- AC-6 is the behaviour-preservation guard: the `contract` bucket must show `0 failed`; any new failure means the relocation changed observable output.

## Verification Commands

| ID | Command | Delegation hint |
|----|---------|-----------------|
| AC-1 | `! rg -nE 'fn (finalization_role_(ir_to_wit|wit_to_ir)|finalization_path_ir_to_wit|convert_postpass_role|ir_to_wit_expolygons?_prepass)\b' crates/slicer-wasm-host/src` | FACT: empty match = pass |
| AC-2 | `test -d crates/slicer-wasm-host/src/marshal && ! rg -n 'wasmtime' crates/slicer-wasm-host/src/marshal/` | FACT: exit 0 = pass |
| AC-3 | `rg -n 'struct OriginId' crates/slicer-wasm-host/src/marshal/origin.rs && ! rg -nE 'type (PerimeterRegionOrigin|SliceRegionOrigin)\b' crates/slicer-wasm-host/src` | FACT: both clauses pass |
| AC-4 | `! rg -n 'any_tagged' crates/slicer-wasm-host/src/host.rs crates/slicer-wasm-host/src/dispatch.rs` | FACT: empty match = pass |
| AC-5 | `cargo test -p slicer-wasm-host --lib marshal::origin 2>&1 \| tee target/test-output.log; rg '^test result' target/test-output.log` | FACT: `0 failed`, ≥4 tests |
| AC-6 | `cargo test -p slicer-wasm-host --test contract 2>&1 \| tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log` | FACT: pass/fail + first failing assertion |
| AC-N1 | `cargo test -p slicer-wasm-host --lib marshal::origin::tests::untagged_payload_in_tagged_mode_errs 2>&1 \| tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log` | FACT: 1 passed |
| AC-N2 | `cargo test -p slicer-wasm-host --lib marshal::origin::tests::length_mismatch_errs 2>&1 \| tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log` | FACT: 1 passed |
| Gate | `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings` | FACT: exit code + first error |

## Step Completion Expectations (cross-step invariants)

- Every step after Step 1 must keep `cargo check --workspace --all-targets` green — the move is incremental; the crate never sits broken between steps.
- No step may alter the observable output of any converter for valid input; the `contract` bucket (AC-6) is the standing guard and is re-run at the completion gate.

## Context Discipline Notes (packet-specific)

- `host.rs` (5225) and `dispatch.rs` (2585) both exceed the 600-line direct-read limit. Operate by line range from `design.md`'s surface map; never open either in full.
- Do not delegate a re-read of ADR-0021 — its key signatures are reproduced in `design.md`.
