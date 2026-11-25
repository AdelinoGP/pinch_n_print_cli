---
status: implemented
packet: layer-module-preparation_09_lightning-kernel-portability
task_ids:
  - TASK-576
---

# layer-module-preparation_09_lightning-kernel-portability

## Goal

Execute the module-owned lightning kernel **source** on the five Classic canonical planning inputs
through the real native entry and a test-only WASM guest, compare the planned graph and grounding
witnesses to the independent canonical replay record, establish `clip_lightning` semantics against
constructed final fills, and record a truthful `pass`/`fail`/`inconclusive` Gate B outcome.

The kernel under test is the already-migrated module source
(`modules/core-modules/lightning-infill/src/kernel/{generator,layer,tree_node,distance_field}.rs`),
included into the isolated `feasibility-kernel` crate by `#[path]` — byte-identical source, not a
copy and not a second implementation. The adapters that turn canonical planning layers into
`Generator` inputs and turn `Generator`'s tree graph into `KernelPlan` records are this packet's new
code and live outside the included files.

A mismatch is a completed investigation, not an incomplete packet. The canonical capture does not
contain every population this packet's schema can carry; those populations are recorded as **named**
non-comparable populations and never silently dropped.

## Problem Statement

Packet 08 moved the canonical lightning planner out of the host and into
`modules/core-modules/lightning-infill/src/kernel/`, and proved the moved module reproduces the live
producer's recorded output natively and as a fresh WASM guest. That equivalence is an ownership
claim, not a canonical-parity claim: the producer's own constants and input derivation were frozen
by that packet, and its baseline is the producer's output, not OrcaSlicer's.

Gate B asks the remaining question: does the same kernel source, given the **canonical** planning
domains, reproduce the **canonically recorded** planning graph and grounding witnesses when executed
through the real native entry and a WASM guest? The experimental vehicle is the isolated
`feasibility-kernel` crate, which exists to hold a dependency-light portability proof. The kernel
source is therefore included by `#[path]` from the module, not copied: identical bytes, one source of
truth, no drift.

The canonical capture does not contain every population the packet's schema can carry. It records
the replay graph (nodes, edges, roots, `last_grounding`), the planning layers, parameters, and
clipped output segments — but not per-node origin identity, sampled segments, the per-node provenance
table, or final-fill domains/policy. Gate B must compare exactly the recorded populations, record the
unrecorded ones as named non-comparable populations, and never let the missing populations turn a
non-comparison into a pass.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build, run `cargo xtask build-guests --check` and inspect its exit code: exit 0 fresh, 1 stale or lock divergence, 3 infrastructure (stop). Never use `rg -q 'STALE:'` to decide freshness. Guest execution uses `cargo xtask test`.

## Risks and Tradeoffs

- The algorithm may differ numerically or topologically from the canonical graph. That is a valid B
  `fail` and the packet's product.
- The `#[path]` include makes the module's source an input to the feasibility crate; a module-side
  edit changes Gate B's subject. The outcome's file hashes make the tested revision auditable.
- Canonical origin/provenance/sampling and the clipped comparison stay unverifiable until the
  recorder captures them. They are named blockers, and the packet does not pretend otherwise.
- The production adapter's hardcoded straightening angle is a real canonical-parity deviation; this
  packet records it rather than changing production behavior.
- Guest instantiation/freshness can fail for infrastructure reasons; exit 3 is a stop, not a
  scientific result.
