# Design: 138_lightning-distancefield-treenode

## Controlling Code Paths

- Primary code path: new `crates/slicer-core/src/algos/lightning/distance_field.rs` and
  `tree_node.rs`, exported from the packet-137 `crates/slicer-core/src/algos/lightning/mod.rs`
  (which stays a skeleton — no orchestration here).
- Neighboring tests or fixtures: unit tests co-located (`#[cfg(test)]`) — match the
  neighboring lightning-free algo's convention; the slice is the standard pattern (e.g.
  `crates/slicer-core/src/algos/mesh_analysis.rs` co-located tests). If the implementation
  step finds a different convention in use, the test home is decided by FACT and recorded
  in the implementation plan.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Determinism: NO hash-container iteration anywhere in the primitives (Orca's own port
  uses ordered structures; PnP mirrors that) — 139's whole-print determinism test
  depends on it.
- Attribution headers mandatory (ported code; see `docs/ORCASLICER_ATTRIBUTION.md`).

## Code Change Surface

- Selected approach: faithful 1:1 port of both primitives, structure-preserving (same
  function decomposition as the C++ where Rust allows), with `NodeSPtr`-style shared
  ownership mapped to `Rc<RefCell<…>>` or index-based arenas — pick whichever the FIRST
  sectioned read shows is loop-carried; an index arena is preferred if cycles/back-edges
  appear (cleaner `Drop`, deterministic iteration). Both are pre-authorized (Step 2
  resolves via the back-edge FACT).
- Exact changes: two new files + `mod.rs` exports + tests; nothing else.
- Rejected alternatives: (a) re-deriving a "cleaner" lightning algorithm — rejected:
  parity is the goal; (b) porting TreeNode with raw pointers/unsafe — rejected: arena
  or Rc suffices; (c) merging both primitives into one file — rejected: 1,133 lines of
  Rust source + 750 lines of OrcaC++ to compare against; the file split mirrors the
  upstream structure reviewers will compare against.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/lightning/distance_field.rs` (new).
- `crates/slicer-core/src/algos/lightning/tree_node.rs` (new).
- `crates/slicer-core/src/algos/lightning/mod.rs` (exports only; the 137 skeleton
  untouched).
- Test home: co-located `#[cfg(test)] mod tests` blocks inside each new file, matching
  the convention in `crates/slicer-core/src/algos/mesh_analysis.rs` (FACT-confirmed at
  Step 1).

## Read-Only Context

- `crates/slicer-core/src/algos/mod.rs` — module registration idiom (`pub mod <name>;`).
- `crates/slicer-core/src/algos/mesh_analysis.rs` — co-located `#[cfg(test)]` test
  convention (ranged; this is the FACT source for "test home").
- `crates/slicer-ir/src/slice_ir.rs` — `Point2` + `mm_to_units()` accessors (used in
  the port at every mm↔unit boundary).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `crates/slicer-runtime/**`, `modules/**` — untouched.
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "FACT: which test-home convention does `crates/slicer-core/src/algos/mesh_analysis.rs`
  use (co-located `#[cfg(test)]` or a separate `tests/algo_*_tdd.rs`)?; LOCATIONS ≤ 5" —
  Step 1 driver.
- "SUMMARY of `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` public
  API + cell representation; then SNIPPETS ≤ 30 lines of the constructor seeding loop
  and `update`" — Step 1 driver (2-3 dispatches).
- "SUMMARY of `TreeNode.hpp` ownership model (`NodeSPtr`, parent/child links); FACT: are
  there back-edges/cycles?" — ownership-mapping decision input.
- "Sectioned SNIPPETS of `TreeNode.cpp`: (1) attachment/creation, (2)
  `propagateToNextLayer`, (3) straightening, (4) rerooting, (5) pruning; ≤ 30 lines each"
  — Step 2 drivers.
- "FACT with file:line: the supporting radius, smoothing magnitude, and prune length
  constants (values + units)" — constants table (÷100 applied in code, cited in tests).
- "Run `cargo test -p slicer-core -- lightning …`; FACT + counts; SNIPPETS ≤ 20 on
  failure".

## Data and Contract Notes

- IR/WIT/manifest: none touched.
- Public API freeze at packet close: `DistanceField::{new, unsupported_point/next, update}`,
  and the `tree_node` graph operations used by 139 — signature changes after close are
  139-recorded deviations with 138 tests co-updated in the same step.
- Resolution constants: if the Orca primitives take grid resolution as a constructor
  parameter, 138 declares it; if Orca derives it from density, 138 takes a parameter and
  139 supplies it from the resolved config (FACT at Step 2).

## Locked Assumptions and Invariants

- Faithful port: behavioral divergence from the Orca primitives requires a
  `DEVIATION_LOG` entry — there is no "improvement" license here (NaN guards and safety
  checks excepted, following the gyroid precedent).
- All distance constants ÷ 100, cited by Orca file:line in test comments.
- Deterministic iteration everywhere (no HashMap/HashSet in any hot loop).

## Risks and Tradeoffs

- TreeNode ownership mapping is the port's hardest translation; the arena fallback is
  pre-authorized (see Selected approach) to prevent a mid-port redesign stall.
- Hand-computed test cases can encode a misreading of the C++ — mitigation: each
  behavioral test cites the section dispatch (date + section) its expectation came from,
  making the chain auditable.
- Grid-resolution constants may be config-coupled in Orca (density-dependent); if so,
  the primitives take them as parameters and 139 supplies them — record the
  parameterization in the design notes.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 2 — TreeNode port, 750 lines of C++)
- Highest-risk dispatch: the `TreeNode.cpp` section series — five bounded dispatches,
  never a whole-file dump.

## Open Questions

- `[FWD]` Ownership mapping (Rc vs index arena) — decided by the back-edge FACT at
  Step 2 start; both are pre-authorized.
- `[FWD]` Test home (co-located `#[cfg(test)]` vs separate `algo_lightning_tdd.rs`) —
  match crate convention found at Step 1.
- `[FWD]` Resolution constant parameterization (Orca derives vs accepts) — resolved by
  the constants FACT at Step 2.
