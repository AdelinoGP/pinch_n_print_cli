---
status: draft
packet: 136_infill-parity-integration
task_ids:
  - TASK-261
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 136_infill-parity-integration

## Goal

Close the main infill-parity roadmap: the M3 modifier-infill e2e fixture, `infill_overlap`
CLI exposure, restoration + single re-bless of every `carved: infill-parity D6` golden, the
no-linker degraded-output guard, and the workspace acceptance ceremony.

## Scope Boundaries

Integration and closure only — no algorithm changes. This packet proves packets 129–135
compose: modifier densities reach the modules, the linker links across the pipeline, output
is visually and geometrically sane, and the golden baseline is re-established in one
justified bless event. The lightning sub-roadmap (137–140) follows separately with its own
contained bless.

## Prerequisites and Blockers

- Depends on: `129`–`135` (all closed).
- Unblocks: `137_lightning-prepass-contract` (roadmap continuation).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a cube fixture with a centered infill-modifier volume (base density 0.15,
  modifier 0.40, both roles on `rectilinear-infill`), **when** sliced end-to-end, **then**
  the g-code contains exactly one wall set per layer (zero wall loops at the modifier
  boundary) and the sparse infill exhibits two distinct line spacings whose ratio matches
  0.40/0.15 within 10%. | `cargo test -p slicer-runtime --test e2e -- modifier_infill_two_densities 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the AC-1 slice, **when** the committed post-`InfillPostProcess` `InfillIR`
  is inspected, **then** every sparse path lies inside its own sub-region's polygon, paths
  adjacent to the wall-less shared arc reach it within 0.5 × spacing (no unfilled ring), and
  both regions' paths are linked (every bucket's mean points-per-path > 2). | `cargo test -p slicer-runtime --test e2e -- modifier_infill_boundary_anchoring 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** `resources/regression_wedge.stl` sliced with `--report`, **when** the run
  completes, **then** the HTML report exists and the committed `InfillIR` contains linked
  sparse polylines (mean points-per-path > 2 — raw 2-point output would fail); the visual
  confirmation of linked paths (no disjoint-segment travel storms) is recorded in the
  closure log. | `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the CLI, **when** `infill_overlap` is set via the existing config-binding
  mechanism (pattern: `fill_holder` CLI binding), **then** the linker receives the value
  (0.30 produces a measurably different overlap boundary than the 0.45 default). | `cargo test -p slicer-ir -- infill_overlap_cli_binding 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** the carve list from packets 131–135, **when** this packet closes, **then**
  zero `carved: infill-parity D6` markers remain in the test tree, and every restored test
  carries its re-blessed expectation with a closure-log justification per fixture. | `rg -c 'carved: infill-parity D6' --glob '*.rs' | wc -l | grep -q '^0$' && echo RESTORED`

## Negative Test Cases

- **AC-N1. Given** a module set WITHOUT `infill-linker` (module-dir excluding it), **when** a
  slice runs, **then** it completes without error and the committed `InfillIR` is raw
  disjoint output (sparse mean points-per-path ≤ 2 for rectilinear regions) — degraded, not
  failed (ADR-0025 trade-off pin). | `cargo test -p slicer-runtime --test integration -- no_linker_module_degraded_raw_output 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo xtask test --workspace --summary` (packet-close acceptance ceremony — dispatch to a
  sub-agent; FACT verdict + failing-test names only)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 5 — closure contract.
- `docs/specs/modifier-region-infill.md` §Phase M3 — the fixture's assert list.
- `docs/16_slicer_report.md` — report format (delegate; only if the report assert needs it).

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — TASK-254…TASK-261 rows flipped to closed with closure
  notes — `rg -q 'TASK-261.*[Cc]losed' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
