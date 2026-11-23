---
status: implemented
packet: test-quality-remediation_09_core-strengthen
task_ids:
  - 'core/DUP-CORE-strengthen (excluding wider-bead)'
---

# test-quality-remediation_09_core-strengthen

## Goal

Strengthen four existing `slicer-core` weak-oracle groups, covering five named test functions, with independent numeric or analytic runtime assertions while preserving their names, targets, and meaningful existing coverage.

## Problem Statement

The remaining core weak-oracle cases execute real geometry and flow code but do not independently pin the values their names claim to protect: the bridge case checks only self-determinism, vertical flow checks only sign, triangle crossings check only presence, and overlapping-square booleans check only non-empty output. This is one coherent test-quality slice because four existing weak-oracle groups, covering five named test functions, retain their current names and targets and can be strengthened with independently derived numeric or geometric expectations without changing production behavior. The wider-bead oracle is explicitly owned by `core-flow-consolidation` and is not reopened here.

## Architecture Constraints

- Test-only strengthening: retain every named test, existing input, and meaningful existing assertion; do not alter production functions, public APIs, manifests, features, fixtures, or test-target registration.
- Independent-oracle discipline: expected values are computed from perpendicular geometry, the documented finite fallback, plane-edge interpolation, and rectangle areas/bounds. Do not call the production function to derive either side of an assertion, and do not add source-text assertions.
- Determinism is retained only as the existing same-input stability claim; it does not substitute for the new bridge numeric oracle.
- Triangle endpoint comparisons must be orientation-insensitive because `triangle_z_intersection` direction depends on source triangle vertex order; polygon checks must not pin an expected vertex sequence or winding order.
- The existing `shape_signature` helper may establish pairwise distinction of operation outputs, but it must not be used as an expected golden signature.
- `cargo xtask check-literals` remains the enforce-mode literal gate; `cargo xtask check-test-quality --report` remains report-only under ADR-0065.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: none; test inputs use existing `Point2::from_mm` and existing `ExPolygon`/`Polygon` values.
- WIT boundary: none.
- Determinism/scheduler constraints: the existing bridge same-input equality remains; no scheduler or runtime behavior is changed. Triangle endpoint orientation is deliberately not locked.

## Locked Assumptions and Invariants

- `determine_bridging_angle` returns degrees and the horizontal anchor’s perpendicular is `90.0` degrees for the approved case.
- `flow_correction(0.0, 0.0, 1.0)` is the documented finite fallback and must be exactly `1.0`.
- Triangle endpoint geometry is the contract; `Line.start`/`Line.end` order is not.
- Boolean result area, envelope, component count, and pairwise operation distinction are contracts; contour vertex order is not.
- All five named test functions and their current target/module placement survive unchanged.
- The §7 `core` row remains `partial`, not `done`, because wider-bead and later core-wave gaps remain.

## Risks and Tradeoffs

- A test-local shoelace measurement could accidentally mirror a production implementation; keep it as a minimal independent measurement of returned rings and compare to fixed rectangle arithmetic, never to another production operation.
- Unit² values are large scaled-integer areas; use `i128` or an equivalent overflow-safe accumulator in the test helper and compare the exact approved values.
- A future triangle implementation may reverse endpoints; unordered pair matching protects geometry without weakening the endpoint requirement.
- Pairwise `shape_signature` checks are intentionally weaker than vertex goldens but are backed by independent areas, bounds, and component counts.
- The real-plan predicate must parse only the real suffix-tolerant §7 span and stop at the next `## Packet Queue`; its Validation matcher must whitespace-normalize, search anywhere after preserved prior commands, use token boundaries, accept optional trailing flags or a closing code-span backtick (the table's normal style), and bind the exact AC-4 polygon invocation rather than an arbitrary `cargo test`. AC-1 through AC-4 remain the full matrix. Authoring-only synthetic full-plan controls are diagnostic and never closure evidence.
