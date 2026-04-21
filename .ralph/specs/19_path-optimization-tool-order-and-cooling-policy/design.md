# Design: path-optimization-tool-order-and-cooling-policy

## Controlling Code Paths

- Host ordering and deferred tool-change path: `crates/slicer-host/src/layer_executor.rs` and `crates/slicer-host/src/dispatch.rs`.
- Module surface: `modules/core-modules/path-optimization-default/src/lib.rs`.
- Documentation surfaces: `docs/05_module_sdk.md` and `docs/07_implementation_status.md`.
- Neighboring tests or fixtures: new `tool_ordering_tdd.rs` plus existing path-optimization queue tests in `dispatch_tdd.rs`.
- OrcaSlicer comparison surface: `ToolOrdering.cpp` and `CoolingBuffer.hpp`.

## Architecture Constraints

- Selected approach: implement mixed-tool ordering on the live path-optimization surface and explicitly reject live cooling overrides on the docs path.
- The packet must not add a new cooling/fan WIT or config surface.
- The deferred `ToolChange` queue already exists and must remain the only live tool-change emission surface.

## Code Change Surface

- Selected approach:
  - add focused host tests for mixed-tool ordering and redundant-tool-change suppression
  - implement grouped tool ordering and tool-change emission on the live path
  - update docs to close TASK-152c explicitly on the rejection path
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/tool_ordering_tdd.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md`
- Rejected alternatives that were considered and why they were not chosen:
  - adding a new live cooling override surface: rejected because it would widen the packet into postpass control and fan-speed semantics
  - bundling tool ordering into packet `18`: rejected because packet `18` already owns the broader entity-ordering slice

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.tool_changes`
  - tool-index-bearing entities on the path-optimization surface
  - docs-only rejection path for cooling overrides
- WIT boundary considerations:
  - no WIT widening is expected in this packet
- Determinism or scheduler constraints:
  - identical mixed-tool inputs must produce identical `ToolChange` sequences

## Locked Assumptions and Invariants

- Tool ordering is implemented.
- Cooling overrides are intentionally unsupported on the live path-optimization surface.

## Risks and Tradeoffs

- Risk: mixed-tool ordering can regress redundant tool changes. Mitigation: keep a negative suppression test.
- Risk: docs-only rejection could drift from code behavior. Mitigation: grep-based acceptance and explicit wording in both docs surfaces.

## Open Questions

- None. The packet chooses implementation for tool ordering and documentation rejection for cooling overrides.