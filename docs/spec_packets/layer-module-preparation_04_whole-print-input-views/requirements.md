# Requirements: whole-print-input-views

## Packet Metadata

- TASK-574; backlog `docs/07_implementation_status.md`, approved queue row 04.
- Status draft; aggregate M; draft contracts only, no implementation authorization.

## Problem Statement

The tree's `PrepassStageInput` has mesh and optional layer/slice/map/support inputs but no surface classification and is not a preparation resource. Existing ordinary views do not grant preparation access. A host borrow alone cannot give guest algorithms typed shell, bridge, surface and complete source/configuration context. Input transport is one bounded slice of TASK-574, separate from scheduling and private algorithm payloads.

## In Scope

- Immutable committed snapshot and checked batch indices over canonical LayerPlanIR; retain shared Arc products, not unfinished Layer arenas or copied per-layer whole prints.
- Complete field inventory in design: enriched slices, filtered region mapping, surface classification including object-keyed annotation maps, mesh geometry/paint/query projections, seam and optional/required support analysis/geometry/plan.
- Declaration vocabulary and pre-invocation prerequisite validation; distinguish undeclared, declared optional absent and present empty.
- Owner-only selected sources/configs using #01 without reimplementing eligibility, paint gates, holder lookup or model-key authority. Source contexts for every schema-backed Layer stage remain descriptions of inputs, not final output coverage.
- Capability-owned PreparationPaintValue/PreparationVariantChain/PreparationRegionKey and paint annotation/mesh records preserve all native Flag/Scalar/ToolIndex/Custom variants losslessly. Ordinary shared WIT remains unchanged. A nonempty Custom-chain literal/reverse-conversion witness covers all four slice/map insertion permutations on both production adapters.
- Extend #03's production backings, fresh WASM resources and native facade, keeping transaction and seven piece-transport error semantics intact; independently pinned literal observations on both adapters.
- NET-NEW shared observation source with five thin same-source native/WASM fixture packages: PathOptimization for generic reads; Perimeters/Infill for model/member/roles; Support/AnchoredEvents for non-region sources (raft uses Support). Each artifact pairs preparation with exactly one real ordinary stage; no spoofed owner stage or multi-stage module. Native path dev-dependencies, existing build-guests discovery, explicit host contract registration and real FrozenPreparationBinding driver are required.
- Add only the four exact canonical-doc sections in packet.spec during implementation.

## Out of Scope

- #05 scheduling/activation/cancellation/release policy; this packet supplies checked constructors and reads only.
- #06 geometric diagnostic projection publication; `.views()` is retained unchanged, not confused with `input_reads()`.
- #07 lightning migration, geometry reconstruction/approximation or a parity promise; no Orca ports, no retirement of lightning contracts here.
- Repairing paint producer preservation, adding future perimeter/infill/support toolpath reads, cross-owner provider plans, first-match/global-holder targeting, fabricated non-region RegionKeys.
- Changes to existing IR public fields/schema constants, PrepassStageInput fields, ordinary worlds/stage signatures, `LoadedModule`, existing native entry shape or legacy manifest names.
- Generation-time production/tests/docs outside these five files, plan/backlog edits, gates, activation, commits.

## Authoritative Docs

Bounded authorities: docs/01 PrePass order and immutable ownership; docs/02 IR 2/3/4/6 (SliceIR is IR 6); docs/03 canonical WIT and declared boundary; docs/05 call-local lifecycle; ADR-0066 Decision/Consequences, ADR-0045 typed per-stage versions, ADR-0056 single native/WASM module model. Fixture rules: docs/21 §1 and docs/22 §1–2. Source plan read context/selected-target distinction is normative; it is not evidence that proposed APIs exist.

## Acceptance Summary

- AC-1: field completeness and unselected read context; AC-2/3: full model/non-region source contracts; AC-4: mesh/seam/support; AC-5: stable bounded batches/shared backing; AC-6: all-variant paint identity, Custom chains and literal permutation/reverse-conversion witness.
- AC-N1: already-invoked resource grants/owner errors versus before-call unsupported declaration; AC-N2: required prerequisites and selection before call; AC-N3: before-call malformed snapshots versus inside-call ranges/unknown objects/overflow. Only prevalidation asserts zero invocations; resource negatives assert nonzero invocation markers.
- #01 and #03 are matched FORWARD-DEPs, not implemented dependencies; #05/#06 consume the public forward ledger. No TASK-574 completion by this packet alone.

## Verification Commands — authoritative matrix

Each cargo test invocation captures combined output to `target/test-output.log`, uses `set -o pipefail`, and verifies the named exact test actually ran. Do not re-run for truncated output; inspect the log. Every cargo run is delegated at implementation time with FACT pass/fail, at most 20 failure lines. Commands below are not executed during authoring.

| Command | Purpose | Return |
| --- | --- | --- |
| Every independent packet.spec AC command | Real preparation adapters/resources and literal positive/negative witnesses | FACT, one executed test each |
| `cargo build --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` | Mandatory WIT test/bench/example compile gate (includes tests) | FACT |
| `cargo check --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` | All target types and binding consumers | FACT |
| `cargo clippy --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures -- -D warnings` | All-target lint acceptance | FACT |
| `cargo xtask check-literals` | New DTO and fixture literal churn | FACT |
| `cargo xtask check-test-quality --report` | Fix/justify findings in touched tests; independent oracles/non-vacuity | FACT plus touched findings |
| `cargo xtask build-guests --check` | Artifact-verified WIT/code freshness, exit 0 only clean | FACT exit code; 1 rebuild, 3 infrastructure blocker |
| `cargo xtask build-guests` then `cargo xtask build-guests --check` if stale | Rebuild all stale affected fixture/core guests and restage | FACT |

No speculative `cargo test --workspace` requirement. If later explicitly authorized, use `cargo xtask test --summary --workspace` only after all narrow commands pass; not an AC command. SDK existing test-feature targets need `--features test`; this packet's assertions live in the host contract binary, whose existing metadata has no required features. #03 creates `preparation-test-fixtures` (not a currently existing host feature).

## Step Completion Expectations

- WIT/SDK mirror types must compile on native and generated WASM paths before guest runtime validation. No raw IR struct widening is needed.
- Snapshot construction validates structural integrity; `from_committed` validates owner's declared requirements and shared selection before entering `FrozenPreparationBinding::prepare`. Runtime calls the constructor in #05, not in this packet.
- A snapshot may contain optional products not granted to an owner: selection uses authoritative internal inputs, but a resource read still checks its grant; optionality never bypasses grants.
- Fixture assertions first prove nonempty polygons/candidates/target context, then compare each leg against independently written literals. Cross-leg equality alone does not close an AC. No skip-if-artifact-missing path.
- Input read errors do not poison output. Caught handler errors may be followed by successful publication; fixture-returned failure after staged valid output must publish nothing via #03's normal failed-transaction rule. Never substitute PoisonedOutput for an input AccessDenied/RangeError/UnknownObject.

## Context Discipline Notes

Source plan and slice_ir.rs are long: use named-symbol ranged reads. Never inspect large JSON fixtures, generated bindings, lockfiles or full target artifacts. Re-derive all mutable inventories at activation; do not freeze test counts, future versions, or source line numbers as identifiers.
