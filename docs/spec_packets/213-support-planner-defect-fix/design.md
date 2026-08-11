# Design: support-planner-defect-fix

## Controlling Code Paths

- Primary code path: `SupportPlanner::plan_for_object` emission loop in `modules/core-modules/support-planner/src/lib.rs:603-694`; `tapered_radius` at `:1290-1311`.
- Neighboring tests/fixtures: `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`, `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- A lone propagated node is emitted only when it survives `drop`, has `dist_to_top > 0`, has no surviving MST edge, and passes the existing collision policy; use a degenerate two-point segment at the current layer Z.
- `MAX_BRANCH_RADIUS_MM` remains `6.0`; the new floor is `MIN_BRANCH_RADIUS = 0.4`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a lone-node emission arm adjacent to the existing MST and fresh-contact arms; change only the lower clamp and constant.
- Exact functions, traits, manifests, tests, and fixtures: `plan_for_object`, `tapered_radius`, and focused planner tests; no manifest or schema changes.
- Rejected alternatives and reasons: changing propagation/drop logic would broaden the fix and alter established merge behavior; changing the IR shape is unnecessary.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` - role: planner implementation; expected change: lone-node segment and radius floor.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - role: focused radius/planner assertions; expected change: regression assertions if current fixtures can exercise the helper.

## Read-Only Context

- `modules/core-modules/support-planner/src/lib.rs` - lines `480-585`, `603-724`, `760-860`, `1288-1311` only - node lifecycle, emission, propagation, and radius formula.
- `crates/slicer-sdk/src/prepass_types.rs` - lines `250-285` only - branch segment shape.
- `docs/specs/support-generation-defect-verified-findings.md` - lines `56-86`, `128-136`, `138-177` - authoritative defect evidence.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- Fallback support modules, WIT/IR schemas, raft/interface modules, and G-code emitters - no change required.

## Expected Sub-Agent Dispatches

- Question: confirm Orca lone-node continuation locations; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `LOCATIONS`; purpose: parity adjudication.
- Question: identify focused planner test entry points for `tapered_radius` and branch segment output; scope: `modules/core-modules/support-planner/tests/**`; return: `LOCATIONS`; purpose: test placement.
- Question: run guest freshness and focused tests; scope: repository commands only; return: `FACT`; purpose: validation.

## Data and Contract Notes

- IR/manifest contracts: no changes; `branch_segments` remains `Vec<Vec<Point3WithWidth>>`.
- WIT boundary: unchanged; guest module source changes require guest rebuild.
- Determinism/scheduler constraints: preserve active-node iteration order, `drop` decisions, and existing MST ordering.

## Locked Assumptions and Invariants

- A segment's two points are equal in XY and Z for a lone node, with width `tapered_radius(...) * 2.0`.
- Dropped nodes and collision-rejected nodes never emit.
- Radius output is always within `[0.4, 6.0]`.

## Risks and Tradeoffs

- A floor may make contact tips wider than the prior zero-width output; this is intentional for renderability and extrusion validity.
- Visual evidence for `PrePass::SupportGeometry` may have `typed_capture: null`; PNGs and `Layer::Support` typed captures remain the evidence path.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: visual-debug run; `FACT` with manifest path and bounded failure output.

## Open Questions

None.
