---
status: implemented
packet: 205e-integrated-parity-harness
task_ids:
  - TASK-331
backlog_source: docs/specs/integrated-modules-architecture-205c-205e-plan.md (queue row 3)
context_cost_estimate: M
---

# Packet Contract: 205e-integrated-parity-harness

## Goal

Consolidate integrated native/WASM parity setup and comparator scaffolding so new parity gates keep the full structural, tolerance, and negative-test contract without repeating transport boilerplate.

## Scope Boundaries

This packet owns test-only parity harnesses and comparator scaffolding under `crates/slicer-runtime/tests/common` and the 21 integrated parity contract tests. It must preserve every existing module-specific fixture, config, claim set, stage family, comparator tolerance, and non-vacuity self-test. It does not alter production dispatch, module algorithms, or the parity definition.

## Prerequisites and Blockers

- Depends on: 205c and 205d draft; activation requires both packets to be implemented, so the harness can consume stable dispatch and registry surfaces.
- Unblocks: cheaper addition of future integrated module parity gates.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the 21 existing `integrated_parity_*_tdd.rs` modules, **when** the contract test binary is compiled and run, **then** all 21 module parity tests remain mounted and execute through both native and WASM dispatch paths. | `rg -c '^mod integrated_parity_.*_tdd;' crates/slicer-runtime/tests/contract/main.rs | rg -q '^21$' && cargo test -p slicer-runtime --test contract --all-targets 2>&1 | rg -q '^test result: ok'`
- **AC-2. Given** a parity test's module-specific fixture, claims, config, stage ID, and comparator, **when** the shared harness runs it, **then** it constructs one WASM and one native `CompiledModuleLive`, executes both on equivalent input, and returns the existing comparator result without changing the `ParityTolerance` defaults (`coord_mm = 1e-3`, `closure_mm = 1e-3`, `max_bead_width_factor = 2.0`). | `rg -q 'coord_mm: f32' crates/slicer-runtime/tests/common/parity_invariants.rs && rg -q '1e-3' crates/slicer-runtime/tests/common/parity_invariants.rs && rg -q 'max_bead_width_factor' crates/slicer-runtime/tests/common/parity_invariants.rs && echo PASS`
- **AC-3. Given** all six comparator families, **when** their self-tests run, **then** dropped geometry, dropped support entries, changed finalization entities, changed seam entries, changed layer-plan entries, and changed infill paths are rejected, while ULP-scale perturbations are accepted. | `cargo test -p slicer-runtime --test contract -- parity_invariants_selftest 2>&1 | rg -q '^test result: ok'`
- **AC-4. Given** the parity harness migration, **when** source is inspected, **then** a helper named `run_integrated_parity` exists under `crates/slicer-runtime/tests/common`, and all 21 `integrated_parity_*_tdd.rs` files invoke it. | `test -f crates/slicer-runtime/tests/common/integrated_parity_harness.rs && rg -q 'run_integrated_parity' crates/slicer-runtime/tests/common/integrated_parity_harness.rs && [ "$(rg -l 'run_integrated_parity' crates/slicer-runtime/tests/contract/integrated_parity_*_tdd.rs | wc -l)" -eq 21 ] && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a comparator input with one dropped path or entry, **when** the relevant self-test runs, **then** it returns an error rather than passing vacuously. | `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_path 2>&1 | rg -q '^test result: ok'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract --all-targets`

## Authoritative Docs

- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - direct read of structural-invariant and non-vacuity rules.
- `docs/adr/0056-integrated-modules-native-dispatch.md` - direct read of the dual-dispatch parity gate.
- `docs/spec_packets/205a-integrated-edition-coverage/packet.spec.md` - direct read of parity test pattern.
- `docs/spec_packets/205b-native-transport-completion/packet.spec.md` - direct read of later parity additions.
- `CONTEXT.md` - delegated lookup for structural invariant and self-captured baseline.

## OrcaSlicer Reference Obligations

This packet preserves existing structural parity only; it introduces no new OrcaSlicer behavior or reference fixture. Existing parity evidence remains governed by ADR-0042 and the current comparator self-tests.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/07_implementation_status.md` - mark `TASK-331` complete when this packet closes; verify with `rg -q 'TASK-331' docs/07_implementation_status.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
