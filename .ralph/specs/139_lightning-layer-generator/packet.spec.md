---
status: draft
packet: 139_lightning-layer-generator
task_ids:
  - TASK-264
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 139_lightning-layer-generator

## Goal

Port the lightning orchestration — `Lightning::Layer` (`generateNewTrees`, `reconnectRoots`,
`convertToLines`) and `Generator` (`generateInitialInternalOverhangs` + the two top-down
all-layers passes of `generateTrees`, `getTreesForLayer`) — into
`crates/slicer-core/src/algos/lightning/`, and wire the packet-137 producer so
`PrePass::LightningTreeGen` commits real per-layer tree segments into `LightningTreeIR`.

## Scope Boundaries

Host-side orchestration port plus producer wiring — completes the generation pipeline behind
the 137 seam. The `lightning-infill` module still runs its stub (rewired in 140), so no
user-visible output changes for lightning prints yet, and non-lightning prints stay skipped.
Cross-layer behavior (overhang seeding, continuity, determinism) is the test focus.

## Prerequisites and Blockers

- Depends on: `137_lightning-prepass-contract` (seam), `138_lightning-distancefield-treenode`
  (primitives; API frozen).
- Unblocks: `140_lightning-module-rewrite`.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a two-layer synthetic object where layer N's sparse outline extends beyond
  layer N−1's, **when** `generate_initial_internal_overhangs` runs, **then** the overhang
  region for layer N equals outline(N) minus the dilated outline(N−1) (ported dilation
  constant, ÷100), within Clipper tolerance. | `cargo test -p slicer-core -- lightning_generator_overhangs 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a synthetic prism with a single internal overhang near its top, **when**
  `generate_trees` runs top-down, **then** trees exist on every layer between the overhang
  and its support ground, and each layer's tree endpoints lie within the per-layer move
  distance of the layer below's trees or outline (continuity, ported bound). | `cargo test -p slicer-core -- lightning_generator_tree_continuity 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a generated object, **when** `trees_for_layer` output is compared with the
  producer-committed `LightningTreeIR` for the same layers, **then** they are identical —
  the 137 producer now commits real segments (empty-skeleton behavior gone for
  lightning-configured prints). | `cargo test -p slicer-runtime --test executor -- lightning_producer_commits_trees 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the same input run twice, **when** the committed `LightningTreeIR`s are
  compared, **then** they are byte-identical (whole-pipeline determinism over the 138
  primitives). | `cargo test -p slicer-core -- lightning_generator_deterministic 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a uniform prism with no internal overhangs, **when** generation runs,
  **then** the committed `LightningTreeIR` is valid with zero tree segments on every layer
  (no spurious trees). | `cargo test -p slicer-core -- lightning_generator_no_overhang_no_trees 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** a default-config wedge slice (no lightning holder), **when** run, **then**
  the g-code SHA is byte-identical (skip path untouched). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo test -p slicer-runtime --test executor -- lightning 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L3 — full read (short).
- `docs/adr/0029-…` — delegate SUMMARY.
- `docs/ORCASLICER_ATTRIBUTION.md` — header template.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.hpp` / `.cpp` (171/587) — `generateNewTrees`, `reconnectRoots`, `convertToLines` (sectioned dispatches).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.hpp` / `.cpp` (261/475) — constructor sequence (`Generator.cpp:189-190`), `generateInitialInternalOverhangs`, `generateTrees` two-pass structure (`Generator.cpp:342`), `getTreesForLayer`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp:145` — `build_generator` inputs (what per-object data the generator consumes).

## Doc Impact Statement (Required)

**`none`** — completes the implementation behind the packet-137 contract; the IR, stage, and
view documentation landed with 137 (docs/02 + docs/03), and the architecture with ADR-0029.
No new public surface.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
