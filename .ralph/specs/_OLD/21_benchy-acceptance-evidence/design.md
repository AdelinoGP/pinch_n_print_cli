# Design: benchy-acceptance-evidence

## Controlling Code Paths

- Primary acceptance surface: `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`.
- Supporting host surfaces: the real CLI entry path, module discovery, `run_pipeline_with_events`, and the final emit/serialize path already exercised by that file.
- Neighboring tests or fixtures: `resources/benchy.stl`, existing Benchy MVP and diagnosability guards.
- OrcaSlicer comparison surface: the Orca source files that define support, fill, seam, and emit semantics, without relying on a committed golden Benchy artifact.

## Architecture Constraints

- Selected approach: extend the existing real Benchy suite with feature-fragment assertions instead of a byte-for-byte Orca golden diff.
- The packet must stay on the real binary + real module tree + real Benchy fixture path.
- Feature evidence must fail with targeted diagnostics naming the missing family.

## Code Change Surface

- Selected approach:
  - extend `benchy_end_to_end_tdd.rs` with focused feature-evidence assertions and targeted failure messages
  - reuse existing helper functions for loading the real STL, running the binary, and counting GCode fragments
  - add one intermediate live-path seam evidence assertion if final text alone is too weak
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
  - `resources/benchy.stl`
  - `modules/core-modules/`
- Rejected alternatives that were considered and why they were not chosen:
  - byte-for-byte Orca golden diff: rejected because no committed golden Benchy artifact exists in this repo
  - relying only on gross extrusion counts: rejected because TASK-135 explicitly needs feature-family evidence

## Data and Contract Notes

- IR or manifest contracts touched:
  - final emitted `.gcode` feature fragments from packet `11`
  - intermediate seam evidence from packets `14` and `15`
- WIT boundary considerations:
  - none in this packet; it is purely end-to-end acceptance coverage
- Determinism or scheduler constraints:
  - repeated identical Benchy runs must remain byte-deterministic

## Locked Assumptions and Invariants

- The acceptance suite uses the real `resources/benchy.stl` fixture and the real core-module tree.
- Feature-fragment assertions are the chosen parity evidence until a committed golden artifact exists.

## Risks and Tradeoffs

- Risk: a feature fragment may appear spuriously without correct semantics. Mitigation: use both final text evidence and intermediate seam evidence where text alone is weak.
- Risk: the suite can become brittle if it assumes one exact path count. Mitigation: assert presence, balance, and targeted fragments rather than entire file equivalence.

## Open Questions

- None. The packet chooses feature-fragment evidence as the acceptance strategy.