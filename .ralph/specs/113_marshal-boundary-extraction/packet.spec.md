---
status: active
packet: 113_marshal-boundary-extraction
task_ids: []
backlog_source: docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md
context_cost_estimate: M
---

# Packet Contract: 113_marshal-boundary-extraction

## Goal

Consolidate every IR↔WIT translation in `slicer-wasm-host` into one in-process `marshal` module whose origin-attribution rule lives in a single `OriginBucket` that is unit-testable without instantiating a WASM component — implementing ADR-0021.

## Scope Boundaries

This packet relocates and de-duplicates host-side marshalling only: it deletes the stale per-world converter copies that ADR-0002 type-unification made byte-identical, then moves the surviving converters, leaf maps, and `*Collected` accumulators into `crates/slicer-wasm-host/src/marshal/`. It introduces no WIT change, no behaviour change, and no guest rebuild; `dispatch.rs` retains all wasmtime mechanics and the per-stage harvest router. The host-services WIT unification (ADR-0002 extension) is explicitly out of scope and lives in packet 114.

## Acceptance Criteria

Origin/backlog note: this slice originates from the 2026-06-16 architecture-review session, not a `docs/07` TASK id; it is governed by ADR-0021. Full scope and verification matrix live in `requirements.md`; criteria below are the single authoritative source.

- **AC-1** — Given the stale per-world converter copies (`finalization_role_ir_to_wit`, `finalization_role_wit_to_ir`, `finalization_path_ir_to_wit`, `convert_postpass_role`, `ir_to_wit_expolygon_prepass`, `ir_to_wit_expolygons_prepass`), When this packet lands, Then none of those function definitions exist anywhere under `crates/slicer-wasm-host/src` and every former call site routes to the single unified converter. | `! rg -nE 'fn (finalization_role_(ir_to_wit|wit_to_ir)|finalization_path_ir_to_wit|convert_postpass_role|ir_to_wit_expolygons?_prepass)\b' crates/slicer-wasm-host/src`

- **AC-2** — Given the new boundary, When the crate builds, Then `crates/slicer-wasm-host/src/marshal/` exists with at least `mod.rs`, `origin.rs`, and `out.rs`, and the entire `marshal/` subtree contains zero `wasmtime` references. | `test -d crates/slicer-wasm-host/src/marshal && ! rg -n 'wasmtime' crates/slicer-wasm-host/src/marshal/`

- **AC-3** — Given `OriginId`, When this packet lands, Then `struct OriginId { object_id: String, region_id: u64 }` is defined in `marshal/origin.rs`, the aliases `PerimeterRegionOrigin` and `SliceRegionOrigin` are removed, and the relocated `*Collected` accumulators carry `Vec<Option<OriginId>>` / `Option<OriginId>` origin fields. | `rg -n 'struct OriginId' crates/slicer-wasm-host/src/marshal/origin.rs && ! rg -nE 'type (PerimeterRegionOrigin|SliceRegionOrigin)\b' crates/slicer-wasm-host/src`

- **AC-4** — Given the all-or-none origin-attribution rule, When this packet lands, Then the `any_tagged` detection and first-seen bucketing loop exist only in `marshal/origin.rs`, and `convert_infill_output`, `convert_perimeter_output`, `convert_support_output` each construct an `OriginBucket` rather than re-implementing the bucket loop. | `! rg -n 'any_tagged' crates/slicer-wasm-host/src/host.rs crates/slicer-wasm-host/src/dispatch.rs`

- **AC-5** — Given `marshal::origin` unit tests, When run, Then four behaviours pass: payloads bucket in first-seen origin order, an untagged payload in tagged mode errors, no-tag mode collapses to one region, and an origin/payload length mismatch errors. | `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal::origin 2>&1 | tee target/test-output.log; rg '^test result' target/test-output.log`

- **AC-6** — Given the relocation is behaviour-preserving, When the `slicer-wasm-host` contract and unit buckets run, Then they pass with zero failures. | `mkdir -p target && cargo test -p slicer-wasm-host --test contract 2>&1 | tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log`

### Negative Test Cases

- **AC-N1** — Given a tagged-mode stream where one payload's parallel origin is `None`, When `OriginBucket::drain` processes it, Then it returns `MarshalError::UntaggedPayload { kind, index }` (identity-preservation guard) and the wrapping `convert_*_output` surfaces a non-empty `Err(String)`. | `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal::origin::tests::untagged_payload_in_tagged_mode_errs 2>&1 | tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log`

- **AC-N2** — Given a tagged-mode stream whose origin slice length differs from its payload count, When `OriginBucket::drain` processes it, Then it returns `MarshalError::OriginLengthMismatch { kind, origins, payloads }` before any region is emitted. | `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal::origin::tests::length_mismatch_errs 2>&1 | tee target/test-output.log; rg 'test result:.*1 passed' target/test-output.log`

## Verification (gate subset)

These are the closure-gate commands; the full matrix is in `requirements.md`.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-wasm-host --lib marshal 2>&1 | tee target/test-output.log; rg '^test result' target/test-output.log`

## Authoritative Docs

- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — the governing decision (module shape, `OriginBucket`, rejected trait design, stage-routing placement).
- `docs/adr/0002-wit-marshalling-type-unification.md` — why the deleted per-world copies are byte-identical (its "Deferred" follow-up).
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — why per-stage harvest routing stays in `dispatch.rs`.
- `CONTEXT.md` — the "Marshalling boundary" concept this module realizes.
- `docs/02_ir_schemas.md` — exact field names for `InfillIR` / `InfillRegion` (`object_id`, `region_id`, `sparse_infill`, `solid_infill`, `ironing`) and the `Perimeter`/`Support` equivalents.

## Doc Impact Statement (Required)

ADR-0021 and the `CONTEXT.md` "Marshalling boundary" term were authored in the originating session and require no further change. No other doc edits are required by this packet. If `docs/07_implementation_status.md` later tracks this work, add a closed entry referencing ADR-0021.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
