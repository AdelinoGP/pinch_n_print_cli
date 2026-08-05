# Design: 138_lightning-distancefield-treenode

## Controlling Code Paths

- Primary code path: new `crates/slicer-core/src/algos/lightning/distance_field.rs` and
  `tree_node.rs`, exported from the packet-137 `crates/slicer-core/src/algos/lightning/mod.rs`
  (which stays a skeleton — no orchestration here).
- Neighboring tests or fixtures: `crates/slicer-core/tests/algo_lightning_tdd.rs` is the
  separate integration-test home, registered in `crates/slicer-core/Cargo.toml` under
  `[[test]]` with `required-features = ["host-algos"]`.
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
  ownership mapped to `Rc<RefCell<Node>>`. The graph has no arena and no back-edges.
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
- Test home: `crates/slicer-core/tests/algo_lightning_tdd.rs`, a separate integration test
  registered in `crates/slicer-core/Cargo.toml` with `required-features = ["host-algos"]`.

## Read-Only Context

- `crates/slicer-core/src/algos/mod.rs` — module registration idiom (`pub mod <name>;`).
- `crates/slicer-core/Cargo.toml` — `algo_lightning_tdd` integration-test registration and
  `required-features = ["host-algos"]`.
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
- "Run `cargo test -p slicer-core --features host-algos --all-targets -- lightning …`; FACT + counts; SNIPPETS ≤ 20 on
  failure".

## Data and Contract Notes

- IR/WIT/manifest: none touched.
- Public API freeze at packet close: `DistanceField::{new, unsupported_point/next, update}`,
  and the `tree_node` graph operations used by 139 — signature changes after close are
  139-recorded deviations with 138 tests co-updated in the same step.
- Resolution: `DistanceField` takes `supporting_radius` as a constructor parameter;
  `m_cell_size = supporting_radius / 6` is derived internally from Orca's
  `radius_per_cell_size = 6`. There is no density-derived resolution in 138.

## Deviations

- 138 ships `propagate_to_next_layer` with the realign step stubbed. The `next_outlines` and
  `outline_locator_resolution` parameters are accepted for API stability but unused; 139's
  `Layer` will fill in the real outline-snap. AC-2 tests the prune+straighten path only; the
  realign path is not exercised until 139.

## Locked Assumptions and Invariants

- Faithful port: behavioral divergence from the Orca primitives requires a
  `DEVIATION_LOG` entry — there is no "improvement" license here (NaN guards and safety
  checks excepted, following the gyroid precedent).
- All distance constants ÷ 100, cited by canonical Orca function name in test comments.
- Deterministic iteration everywhere (no HashMap/HashSet in any hot loop).

## Risks and Tradeoffs

- TreeNode ownership mapping is the port's hardest translation; `Rc<RefCell<Node>>` is the
  selected mapping because the graph has no back-edges.
- Hand-computed test cases can encode a misreading of the C++ — mitigation: each
  behavioral test cites the section dispatch (date + section) its expectation came from,
  making the chain auditable.
- Grid resolution is derived internally from the `supporting_radius` constructor parameter;
  138 does not add density-derived resolution.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 2 — TreeNode port, 750 lines of C++)
- Highest-risk dispatch: the `TreeNode.cpp` section series — five bounded dispatches,
  never a whole-file dump.

## Resolved Questions

- Ownership mapping: `Rc<RefCell<Node>>`; no arena and no back-edges.
- Test home: `crates/slicer-core/tests/algo_lightning_tdd.rs`, registered in
  `crates/slicer-core/Cargo.toml` with `required-features = ["host-algos"]`.
- Resolution parameterization: `DistanceField` accepts `supporting_radius` and derives
  `m_cell_size = supporting_radius / 6`; 138 has no density-derived resolution.
