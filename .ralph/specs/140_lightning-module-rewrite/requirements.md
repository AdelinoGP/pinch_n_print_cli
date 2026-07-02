# Requirements: 140_lightning-module-rewrite

## Packet Metadata

- Grouped task IDs:
  - `TASK-265`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Everything behind the seam is real (137–139: stage, IR, primitives, generator), yet lightning
prints still come from the 512-LOC single-layer stub — grid samples joined to the nearest
boundary, self-linked in violation of ADR-0025 (DEV-081), with none of the canonical
cross-layer tree behavior. The stub is also the roadmap's last self-linking module: until it
emits raw, the linker's "one place linking happens" invariant has a standing exception that
paths cannot even be detected (no module identity on paths). This packet deletes the stub,
samples the committed trees, and closes DEV-081 — completing both the lightning-parity
sub-roadmap and Architecture A's uniformity.

## In Scope

- Rewrite `modules/core-modules/lightning-infill/src/lib.rs`: per region (sparse role,
  `should_emit` gating unchanged), read the layer's tree segments from the 137 view, emit raw
  `SparseInfill` polylines (`speed_factor` from config; `begin_region` origin discipline);
  delete `build_branches`, the grid sampler, distance-sort machinery, and any
  clipping/chaining.
- Mirror only Orca's sampling-side per-layer transformation (delegated
  `Filler::_fill_surface_single` check) — generation is host-side, linking is the linker's.
- Rewrite the module test suite (`tests/lightning_infill_tdd.rs` currently pins stub
  behavior): AC-1 sampling equality, AC-N2 empty-trees totality; keep the module-binding
  test.
- Pipeline test: `lightning_pipeline_linked` (AC-3) in the runtime executor bucket.
- DEV-081 closure edit; TASK-262…265 docs/07 closure sweep; contained lightning re-bless
  (AC-5) + the roadmap-close workspace ceremony.

## Out of Scope

- Any change to the generator/primitives (137–139 closed; defects found here are recorded and
  routed per the ≤ 20-line deviation fence, else packetized).
- Claims/manifest changes (stays `["claim:sparse-fill"]` — lightning solid shells are not a
  thing in Orca or PnP).
- Linker changes — if linked lightning output looks wrong, the fault is triaged to
  emission (here) vs linking (133-follow-up), never patched in the linker from this packet.

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §L4 — full (short).
- `docs/adr/0029-…` — module-sampler contract (delegate SUMMARY).
- `docs/adr/0025-…` §Amendment point 2 — why pass-through detection was never an option.
- `CLAUDE.md` §Test Discipline — the workspace ceremony contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `Filler::_fill_surface_single` only: the per-layer sampling-side handling between `getTreesForLayer` and output (one SUMMARY dispatch).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`. Refinements: AC-1 is count + endpoint
  equality against the view (the module adds NO geometry of its own); AC-3 is the
  Architecture-A uniformity proof (lightning → linker → linked polylines); AC-4/AC-5 are the
  closure artifacts.
- Negative cases: `AC-N1` (wedge byte-identity), `AC-N2` (empty-trees totality — also proves
  the stub fallback is really gone).
- Cross-packet impact: closes DEV-081, TASK-262…265, and the entire infill-parity roadmap
  (129–140).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p lightning-infill 2>&1 \| tee target/test-output.log \| grep "^test result"` | module suite | FACT + counts |
| `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 uniformity | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 | FACT |
| `rg -q 'DEV-081.*Closed' docs/DEVIATION_LOG.md && echo OK` | AC-4 | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | module src edited | FACT |
| `cargo xtask test --workspace --summary` (sub-agent) | roadmap-close ceremony | FACT verdict + failing names only |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT |

## Step Completion Expectations

- The workspace ceremony runs last, after the bless, with the same summary-only dispatch
  contract as packet 136's.
- Bless order: AC-1/AC-3 (geometry/pipeline) green before any expectation re-bless.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: the current module
  `lib.rs` (512 lines — one full read at Step 1, then it mostly gets deleted); the existing
  323-line test file (read once to classify keep/rewrite/delete).
- Likely temptation reads: generator internals (139) "to understand the trees" — the IR view
  contract is the module's whole world; delegate a FACT if an IR semantics question arises.
- Sub-agent return-format hints: ceremony returns the `--summary` verdict block only; bless
  dispatches FACT per expectation (old → new + 1-line justification).
