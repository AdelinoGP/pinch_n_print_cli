# Design: live-support-generation

## Controlling Code Paths

- Primary host path: `crates/slicer-host/src/dispatch.rs` and `crates/slicer-host/src/layer_executor.rs` — real `Layer::Support` dispatch and `SupportIR` commitment.
- Canonical live acceptance generator: `modules/core-modules/tree-support/src/lib.rs`.
- Control generator surface: `modules/core-modules/traditional-support/src/lib.rs`.
- Neighboring tests or fixtures: existing tree-support and traditional-support tests plus a new focused host integration file `crates/slicer-host/tests/live_support_generation_tdd.rs`.
- OrcaSlicer comparison surface: `SupportMaterial.hpp`, `SupportCommon.hpp`, `TreeSupport.hpp`, `TreeSupport3D.hpp`.

## Architecture Constraints

- The packet restores the live host path, not just standalone module geometry.
- Tree-support is the canonical live acceptance target because the parent TASK-120 acceptance run expects tree supports enabled.
- Traditional-support remains in-scope only as a control path for shared host behavior and documented paint precedence.
- The packet must keep exact `ExtrusionRole::SupportMaterial` semantics so packet `11` can serialize them later.

## Code Change Surface

- Selected approach:
  - add one focused host integration file for live support commitment
  - repair the tree-support live dispatch path first
  - keep traditional-support covered as a control generator on the same host surface
  - preserve paint blocker/enforcer precedence on the real path
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `crates/slicer-host/tests/live_support_generation_tdd.rs`
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs`
  - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs`
- Rejected alternatives that were considered and why they were not chosen:
  - restoring only traditional-support: rejected because the parent Workstream 3 acceptance explicitly calls for tree supports enabled
  - collapsing support evidence into final Benchy tests: rejected because the live host commitment gap needs a narrower falsifying check first

## Data and Contract Notes

- IR or manifest contracts touched:
  - `SupportIR.support_paths`, `interface_paths`, `raft_paths`
  - `ExtrusionRole::SupportMaterial`
  - `PaintSemantic::SupportBlocker` and `SupportEnforcer`
  - support-generator claim selection on the live stage path
- WIT boundary considerations:
  - no schema widening is required; the packet stays on existing support-stage inputs and outputs
- Determinism or scheduler constraints:
  - identical support-stage inputs must produce the same committed `SupportIR` across repeated runs

## Locked Assumptions and Invariants

- Tree-support is the acceptance target for the live Benchy path.
- Control-path traditional-support coverage must not become a second acceptance target that expands the packet's scope into generic support parity.

## Risks and Tradeoffs

- Risk: unit tests may already pass while host dispatch still drops committed support. Mitigation: keep host integration tests primary.
- Risk: tree-support behavior may still diverge from traditional-support in legitimate ways. Mitigation: only share role/commit/paint-precedence assertions across both generators.

## Open Questions

- None. The packet chooses one live acceptance generator and one control generator.