# Implementation Plan: 156-arachne-region-order

## Execution Rules

- Execute one step at a time and do not combine its edit list with another
  step. A later step may not compensate for an unmet earlier exit condition.
- All OrcaSlicer reads are delegated. Every cargo test writes combined output
  to `target/test-output.log` and is inspected from that log.
- This is audit-driven: `task_ids: none` is intentional. `docs/07` has no G12
  row; do not repurpose unrelated `TASK-156`.

## Step 1: Lock canonical core behavior with RED tests

- Objective: make the existing core tests prove the exact missing semantics:
  pair exclusions, edge deduplication, candidate-cell lookup, canonical
  finalized-line walk behavior, and permutation/acyclicity.
- Precondition: packet remains `draft`; the current `region_order.rs` and
  `sparse_point_grid.rs` compile.
- Postcondition: focused tests fail only on the known partial-port behavior;
  they compile and do not use `#[ignore]` or a placeholder assertion.
- Files allowed to read: `crates/slicer-core/tests/region_order_tdd.rs`;
  `crates/slicer-core/tests/sparse_point_grid_tdd.rs`; the matching core files
  only to name the public API.
- Files allowed to edit (<=3): `crates/slicer-core/tests/region_order_tdd.rs`;
  `crates/slicer-core/tests/sparse_point_grid_tdd.rs`.
- Expected sub-agent dispatches:
  - Delegate canonical `getRegionOrder` guards and set semantics from
    `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp`; return
    `SNIPPETS` (<=3 snippets, <=30 lines each).
  - Run `cargo test -p slicer-core --test region_order_tdd --no-run`; return
    `FACT` pass/fail.
  - Run each intended RED filter; return `SNIPPETS` with its assertion only.
- Context cost: S.
- Authoritative docs: `docs/18_arachne_parity_audit.md` G12 section.
- OrcaSlicer refs: `Arachne/WallToolPaths.cpp`; `Arachne/utils/SparseGrid.hpp`;
  `PerimeterGenerator.cpp` finalized-extrusion walk.
- Narrow verification:
  - `cargo test -p slicer-core --test region_order_tdd -- region_order_get_matches_canonical_pair_guards --exact`
  - `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_returns_touched_cell_candidates --exact`
- Exit condition: the tests compile and fail against the partial implementation
  for the named semantic gaps.

## Step 2: Port canonical constraints, grid, and walk

- Objective: correct core region ordering without changing the public pipeline
  contract yet. Apply canonical pair guards before the odd/even predicate, use
  unique edge storage, make the grid candidate-only, and remove force-emission
  recovery.
- Precondition: Step 1 RED tests exist and their failures are recorded.
- Postcondition: generated constraints are canonical and acyclic; every core
  order test is green; `topological_walk` returns a permutation for acyclic
  input without a PnP-only cycle path.
- Files allowed to read: `crates/slicer-core/src/arachne/region_order.rs`;
  `crates/slicer-core/src/arachne/sparse_point_grid.rs`;
  `crates/slicer-ir/src/slice_ir.rs` ranges defining `ExtrusionLine` and
  `ExtrusionJunction`.
- Files allowed to edit (<=3): `crates/slicer-core/src/arachne/region_order.rs`;
  `crates/slicer-core/src/arachne/sparse_point_grid.rs`;
  `crates/slicer-core/tests/region_order_tdd.rs`.
- Expected sub-agent dispatches:
  - Delegate a comparison of the two core functions to the Orca locations in
    Step 1; return `LOCATIONS` (<=15) enumerating semantic mismatches.
  - Run `cargo test -p slicer-core --test region_order_tdd`; return `FACT`.
  - Run `cargo test -p slicer-core --test sparse_point_grid_tdd`; return
    `FACT`.
- Context cost: M.
- Authoritative docs: `docs/08_coordinate_system.md` (coordinates remain
  f32-mm; no unit conversion is introduced).
- OrcaSlicer refs: `WallToolPaths.cpp` `getRegionOrder`;
  `utils/SparseGrid.hpp` lookup; `PerimeterGenerator.cpp` walk.
- Narrow verification:
  - `cargo test -p slicer-core --test region_order_tdd`
  - `cargo test -p slicer-core --test sparse_point_grid_tdd`
- Exit condition: both test binaries pass and a static inspection confirms no
  force-emission branch remains.

## Step 3: Order finalized pipeline output

- Objective: move the region-order call to after Arachne's final line
  post-processing, immediately before `run_arachne_pipeline` returns.
- Precondition: Step 2 is green.
- Postcondition: constraints are built from stitched, simplified, non-empty
  lines; the direct G12 fixture continues to assert outer-before-inner for the
  final output and output remains a permutation.
- Files allowed to read: `crates/slicer-core/src/arachne/pipeline.rs` range
  from `generate_toolpaths` through the return; `region_order.rs`; G12 runtime
  fixture/test files.
- Files allowed to edit (<=3): `crates/slicer-core/src/arachne/pipeline.rs`;
  `crates/slicer-runtime/tests/arachne_parity_round2.rs`;
  `crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs`.
- Expected sub-agent dispatches:
  - Delegate Orca generation-to-walk ordering from `WallToolPaths.cpp` and
    `PerimeterGenerator.cpp`; return `SUMMARY` <=200 words.
  - Run the AC-4 test; return `FACT` pass/fail.
- Context cost: S.
- Authoritative docs: `docs/18_arachne_parity_audit.md` G12 fixture section.
- OrcaSlicer refs: `WallToolPaths.cpp` generation/post-processing sequence;
  `PerimeterGenerator.cpp` ordered-extrusion construction.
- Narrow verification:
  - `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact`
- Exit condition: the region-order call is after empty-line removal and the
  targeted test passes.

## Step 4a: Declare the complete sequence in WIT, SDK, and core parameters

- Objective: add WIT `enum wall-sequence { inner-outer, outer-inner,
  inner-outer-inner }`, add `wall-sequence: wall-sequence` to
  `arachne-params`, and replace the lossy `outer_to_inner: bool` field in the
  SDK and core `ArachneParams` mirrors with the existing
  `slicer_core::perimeter_utils::WallSequence` variant set.
- Precondition: Step 3 is green; the three modes and initial-layer behavior
  are documented in the packet contract.
- Postcondition: WIT, SDK, and core use exactly the three named variants and
  neither ArachneParams type exposes `outer_to_inner`.
- Files allowed to read: `crates/slicer-schema/wit/deps/common.wit` record;
  `crates/slicer-sdk/src/host.rs` Arachne conversion;
  `crates/slicer-core/src/arachne/pipeline.rs` ArachneParams definition.
- Files allowed to edit (<=3): `crates/slicer-schema/wit/deps/common.wit`;
  `crates/slicer-sdk/src/host.rs`; `crates/slicer-core/src/arachne/pipeline.rs`.
- Expected sub-agent dispatches:
  - Delegate `outer_to_inner` call-site inventory; return
    `LOCATIONS` <=20.
  - Run `cargo check -p slicer-sdk`; return `FACT`.
  - Run `cargo check -p slicer-core`; return `FACT`.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md`; `docs/05_module_sdk.md`.
- OrcaSlicer refs: `PrintConfig.hpp` sequence enum and `PrintConfig.cpp`
  default.
- Narrow verification:
  - `cargo check -p slicer-sdk`
  - `cargo check -p slicer-core`
- Exit condition: the two narrow checks pass; the only remaining
  `outer_to_inner` production sites are the WASM host and Arachne module paths
  assigned to Step 4b.

## Step 4b: Propagate the sequence through core, host, and module

- Objective: reconstruct the exact WIT `wall-sequence` value in the WASM host
  and resolve the existing `wall_sequence` config string into the matching
  `WallSequence` variant in the perimeter module.
- Precondition: Step 4a's WIT/SDK/core checks pass and name the three exact
  wall-sequence variants.
- Postcondition: no production path substitutes `false` or a default for a
  module-selected sequence; all ArachneParams literal sites compile.
- Files allowed to read: `crates/slicer-wasm-host/src/host.rs` Arachne
  reconstruction; `modules/core-modules/arachne-perimeters/src/lib.rs` config
  resolution; `crates/slicer-sdk/src/host.rs` ArachneParams literal shape.
- Files allowed to edit (<=3): `crates/slicer-wasm-host/src/host.rs`;
  `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Expected sub-agent dispatches:
  - Inventory every remaining `outer_to_inner` production reference after
    Step 4a; return `LOCATIONS` <=20.
  - Run `cargo check --workspace --all-targets`; return `FACT`.
  - Run `cargo xtask build-guests --check`; return `FACT clean/STALE`.
  - If stale, run `cargo xtask build-guests`; return `FACT`.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md`; `docs/05_module_sdk.md`.
- OrcaSlicer refs: `PrintConfig.hpp` sequence enum and `PrintConfig.cpp`
  default.
- Narrow verification:
  - `cargo check --workspace --all-targets`
  - `cargo xtask build-guests --check`
  - `cargo xtask build-guests` (only when the freshness check reports STALE)
- Exit condition: workspace check passes and guest artifacts are rebuilt if the
  freshness check reports stale.

## Step 5: Prove WIT sequence propagation

- Objective: add a real guest-host integration test to the existing
  `arachne_parity_round2` target that distinguishes all three resolved modes,
  including `InnerOuterInner` on layer 0 and a later layer.
- Precondition: Step 4b propagation check is green and guests have been rebuilt.
- Postcondition: AC-5 captures `WallSequence` immediately after WIT decoding,
  before any module wall construction or role ordering, and asserts all three
  values on layer 0 and a later layer.
- Files allowed to read: `crates/slicer-runtime/tests/arachne_parity_round2.rs`;
  its fixture module; existing guest-host Arachne tests.
- Files allowed to edit (<=3): `crates/slicer-wasm-host/src/host.rs` (test-only
  capture seam); `crates/slicer-runtime/tests/arachne_parity_round2.rs`;
  `crates/slicer-runtime/tests/fixtures/arachne_parity/mod.rs` only if a layer
  fixture is necessary.
- Expected sub-agent dispatches:
  - Locate the narrowest existing runtime helper that invokes the real Arachne
    guest-host boundary; return `LOCATIONS` <=10.
  - Run the AC-5 filter; return `FACT` pass/fail.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` boundary contract.
- OrcaSlicer refs: `PrintConfig.hpp` and `PerimeterGenerator.cpp` sequence
  interpretation.
- Narrow verification:
  - `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_wall_sequence_survives_wasm_boundary --exact`
- Exit condition: the AC-5 command executes exactly one real-boundary test and
  passes for every mode/layer case.

## Step 6: Resolve and commit sequence-aware walls

- Objective: make `arachne-perimeters` resolve all modes, including the
  layer-sensitive `InnerOuterInner` behavior, then commit `WallLoop`s without
  an unconditional ascending `perimeter_index` sort that reverses that result.
- Precondition: Step 5 WIT propagation test is green.
- Postcondition: module output honors all three modes and preserves the exact
  distinguishable core path relation `outer-A < odd-inner-A < outer-B`; a
  global `perimeter_index` sort cannot satisfy the test.
- Files allowed to read: `modules/core-modules/arachne-perimeters/src/lib.rs`
  config resolution and wall build/commit ranges; `crates/slicer-ir/src/slice_ir.rs`
  `WallLoop` fields; existing module tests.
- Files allowed to edit (<=3): `modules/core-modules/arachne-perimeters/src/lib.rs`;
  `modules/core-modules/arachne-perimeters/tests/wall_sequence_commit_tdd.rs`;
  `modules/core-modules/arachne-perimeters/Cargo.toml` only if the test target
  needs explicit registration.
- Expected sub-agent dispatches:
  - Delegate canonical `InnerOuterInner` ordering behavior; return `SNIPPETS`
    <=3 from `PerimeterGenerator.cpp`.
  - Run the module test binary; return `FACT`.
  - Run `cargo xtask build-guests --check`; return `FACT clean/STALE`.
- Context cost: M.
- Authoritative docs: ADR-0011; `docs/15_config_keys_reference.md` wall_sequence.
- OrcaSlicer refs: `PerimeterGenerator.cpp` wall-sequence consumer.
- Narrow verification:
  - `cargo test -p arachne-perimeters --test wall_sequence_commit_tdd`
  - `cargo xtask build-guests --check`
- Exit condition: one fixture per mode passes, with explicit layer-0/later
  sandwich assertions and cross-region path-marker precedence.

## Step 7: Preserve sequence through path optimization

- Objective: remove optimizer role grouping that reverses a committed
  wall-sequence relation while retaining permitted nearest-neighbor travel
  choices.
- Precondition: Step 6 module tests are green.
- Postcondition: a live Arachne Perimeters -> optimizer fixture preserves
  committed sandwich path identities `[1, 0, 2]`, not merely wall roles;
  unrelated optimizer behavior has a regression guard.
- Files allowed to read: `modules/core-modules/path-optimization-default/src/lib.rs`;
  its existing tests; the WIT view fields it consumes.
- Files allowed to edit (<=3): `modules/core-modules/path-optimization-default/src/lib.rs`;
  `crates/slicer-runtime/tests/arachne_wall_sequence_e2e_tdd.rs`; the existing
  runtime test aggregator only if the target is aggregated.
- Expected sub-agent dispatches:
  - Delegate consumer ordering trace from Arachne output to optimizer result;
    return `LOCATIONS` <=15.
  - Run the end-to-end test; return `FACT`.
- Context cost: M.
- Authoritative docs: `docs/01_system_architecture.md` ordering ownership;
  ADR-0011.
- OrcaSlicer refs: `PerimeterGenerator.cpp` final wall-sequence ordering.
- Narrow verification:
  - `cargo test -p slicer-runtime --test arachne_wall_sequence_e2e_tdd`
- Exit condition: all three sequence-mode assertions pass and the live
  sandwich identity order is exactly preserved.

## Step 8: Audit, WIT, and deviation documentation

- Objective: replace stale G12 closure claims with accurate references and
  record the real WIT/ownership change; packet status remains `draft` pending
  Step 9 acceptance ceremony.
- Precondition: Steps 1-7 are green and guests are fresh, excluding only the
  documented unrelated D-104f concentric-infill red.
- Postcondition: audit, WIT documentation, and deviation log accurately
  reflect the completed work; packet status remains draft pending Step 9.
- Files allowed to read: `docs/18_arachne_parity_audit.md` G12 ranges;
  `docs/DEVIATION_LOG.md` D-157; ADR-0011; the packet files.
- Files allowed to edit (<=3): `docs/18_arachne_parity_audit.md`;
  `docs/DEVIATION_LOG.md`; `docs/03_wit_and_manifest.md`.
- Expected sub-agent dispatches:
  - Run each packet Doc Impact grep; return `FACT hit/no-hit`.
  - Run all pipe-suffixed AC commands; return one `FACT` per command.
  - Run `cargo check --workspace --all-targets` and clippy; return `FACT`.
- Context cost: S.
- Authoritative docs: those listed in packet Doc Impact.
- OrcaSlicer refs: none; all parity facts were resolved in Steps 1-7.
- Narrow verification:
  - `cargo xtask build-guests --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Exit condition: the three named document greps pass and D-157 accurately
  records only intentional behaviorally equivalent deviations.

## Step 9: Architecture documentation and acceptance ceremony

- Objective: document final wall-sequence ownership and optimizer preservation,
  then run the complete acceptance ceremony and packet review.
- Precondition: Step 8 documentation greps pass.
- Postcondition: architecture docs and ADR-0011 describe the implemented
  ownership boundary; all AC commands pass; only then may packet status change
  to `implemented`.
- Files allowed to read: `docs/01_system_architecture.md` ordering section;
  `docs/adr/0011-perimeter-module-owns-wall-sequencing.md`; packet AC list.
- Files allowed to edit (<=3): `docs/01_system_architecture.md`;
  `docs/adr/0011-perimeter-module-owns-wall-sequencing.md`;
  `.ralph/specs/156-arachne-region-order/packet.spec.md`.
- Expected sub-agent dispatches:
  - Run every pipe-suffixed AC command; return one `FACT` per command.
  - Run `cargo xtask build-guests --check`, workspace check, and clippy; return
    `FACT` per command.
  - Run each Doc Impact grep; return `FACT hit/no-hit`.
- Context cost: S.
- Authoritative docs: packet Doc Impact list.
- OrcaSlicer refs: none; all parity facts were resolved in Steps 1-7.
- Narrow verification:
  - `rg -q 'own wall sequencing' docs/01_system_architecture.md`
  - `rg -q 'final print order' docs/adr/0011-perimeter-module-owns-wall-sequencing.md`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Exit condition: every AC command and Doc Impact grep passes; the audit and
  D-157 are not marked closed before this point; a full packet review has no
  open blocker; only then change packet status to `implemented`.

## Per-Step Budget Roll-Up

| Step | Cost | Reason |
| --- | --- | --- |
| 1 | S | Focused test contracts only. |
| 2 | M | Canonical core port. |
| 3 | S | Single pipeline placement and fixture assertion. |
| 4a | S | WIT declaration and SDK/runtime binding shape. |
| 4b | M | Core, host, and module propagation. |
| 5 | S | Real WIT-boundary integration evidence. |
| 6 | M | Module commitment and three-mode behavior. |
| 7 | M | Cross-module optimizer behavior. |
| 8 | S | Audit, WIT, and deviation documentation. |
| 9 | S | Architecture documentation and acceptance. |

Aggregate context cost: M. No step is L.

## Packet Completion Gate

- Every step exit condition is met in order.
- Every pipe-suffixed AC command passes with dispatched evidence.
- `cargo xtask build-guests --check`, workspace check, and all-targets clippy
  pass.
- G12 is marked closed only after a full packet review returns APPROVED.
