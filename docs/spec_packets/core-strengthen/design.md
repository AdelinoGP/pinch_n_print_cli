# Design: core-strengthen

## Controlling Code Paths

- Primary code path: existing test calls to `determine_bridging_angle` (`crates/slicer-core/src/algos/bridge_over_infill.rs`), `flow_correction` (`crates/slicer-core/src/lib.rs`), `triangle_z_intersection` and local `Line` (`crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs`), and `union`/`intersection`/`difference`/`xor` (`crates/slicer-core/src/polygon_ops.rs`). Production code is read-only.
- Neighboring tests/fixtures: four weak-oracle groups across four surfaces, covering `bridging_angle_is_deterministic` (`crates/slicer-core/tests/bridge_over_infill_tdd.rs`), `flow_correction_stays_positive_for_vertical_input` (`crates/slicer-core/src/lib.rs`), `triangle_crossing_two_edges` and `triangle_crossing_one_edge_vertex_on_plane` (`crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs`), and `boolean_ops_produce_expected_presence_for_overlapping_squares` plus `shape_signature` (`crates/slicer-core/tests/polygon_ops_tdd.rs`). No fixture file is added.
- OrcaSlicer comparison: no parity comparison applies; all new expectations are local analytic or numeric geometry contracts.

## Architecture Constraints

- Test-only strengthening: retain every named test, existing input, and meaningful existing assertion; do not alter production functions, public APIs, manifests, features, fixtures, or test-target registration.
- Independent-oracle discipline: expected values are computed from perpendicular geometry, the documented finite fallback, plane-edge interpolation, and rectangle areas/bounds. Do not call the production function to derive either side of an assertion, and do not add source-text assertions.
- Determinism is retained only as the existing same-input stability claim; it does not substitute for the new bridge numeric oracle.
- Triangle endpoint comparisons must be orientation-insensitive because `triangle_z_intersection` direction depends on source triangle vertex order; polygon checks must not pin an expected vertex sequence or winding order.
- The existing `shape_signature` helper may establish pairwise distinction of operation outputs, but it must not be used as an expected golden signature.
- `cargo xtask check-literals` remains the enforce-mode literal gate; `cargo xtask check-test-quality --report` remains report-only under ADR-0065.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: augment five existing test functions across four weak-oracle groups with independent runtime assertions, using small test-local measurement helpers only where the public result type has no area/bounds method; the polygon helper uses `i128` shoelace sums, `abs(contour) - sum(abs(holes))`, and min/max bounds over returned rings; then record the partial progress in the real §7 ledger row.
- Exact functions, traits, manifests, tests, and fixtures: `bridging_angle_is_deterministic`; `flow_correction_stays_positive_for_vertical_input`; `triangle_crossing_two_edges`; `triangle_crossing_one_edge_vertex_on_plane`; `boolean_ops_produce_expected_presence_for_overlapping_squares`; existing `shape_signature`; no new function is required in production and no fixture is introduced.
- Rejected alternatives and reasons: replacing tests with new files would lose existing target/name coverage; asserting polygon vertex vectors would make output order a false contract; keeping only `is_some`, sign, or non-empty checks would preserve the weak oracles; consulting OrcaSlicer would add an unsupported parity claim to an analytic-only packet.

## Files in Scope (read + edit)

The five surfaces are intentionally split so no step edits more than two files; the extra files beyond the template’s usual three are the four user-approved existing test surfaces plus the separately owned progress row.

- `crates/slicer-core/tests/bridge_over_infill_tdd.rs` - role: existing bridge integration test; expected change: retain determinism and add the `90.0`-degree finite numeric oracle.
- `crates/slicer-core/src/lib.rs` - role: inline flow unit test; expected change: retain the vertical input and assert finite exact `1.0`.
- `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` - role: inline triangle intersection tests; expected change: assert both analytic endpoint pairs orientation-insensitively.
- `crates/slicer-core/tests/polygon_ops_tdd.rs` - role: existing boolean integration test and `shape_signature` helper; expected change: add independent area/bounds/component assertions and pairwise distinction.
- `docs/specs/test-quality-remediation-plan.md` - role: progress ledger only; expected change: update the real `core` row in §7 to `partial` with accumulated evidence and the packet’s own validation/gap markers; do not edit `## Packet Queue`.

## Read-Only Context

- `crates/slicer-core/src/algos/bridge_over_infill.rs` - symbol `determine_bridging_angle`; read its return type and degree/perpendicular behavior only.
- `crates/slicer-core/src/lib.rs` - lines `286-330` only - `flow_correction` and the inline `tests` module around the existing vertical test; read the fallback branch and exact module path only.
- `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` - `Line`, `triangle_z_intersection`, `pt3`, and the two existing tests; read the endpoint representation and module path only.
- `crates/slicer-core/tests/bridge_over_infill_tdd.rs` - imports, existing bridge test, and exact integration target registration.
- `crates/slicer-core/tests/polygon_ops_tdd.rs` - `square`, `shape_signature`, boolean test, and nearby order-agnostic geometry assertion patterns.
- `crates/slicer-core/src/polygon_ops.rs` - lines `291-350` only - public boolean operation signatures and no-public-area-method fact; do not copy the private production area implementation into the test.
- `crates/slicer-ir/src/slice_ir.rs` - `Point2::from_mm`, `Point2` fields, and `ExPolygon`/`Polygon` shapes only.
- `crates/slicer-core/Cargo.toml` - existing `bridge_over_infill_tdd` target and auto-discovered `polygon_ops_tdd` target facts only.
- `docs/specs/test-quality-remediation-plan.md` - delegated sections `§7 Ledger`, `## Packet Queue`, approval note, and dependency-export note; never read generated output or lockfiles.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` and any external OrcaSlicer checkout - no parity behavior is claimed.
- `target/**`, `Cargo.lock`, generated code, vendored dependencies, and large fixture files - never load.
- All production implementation files for editing; they are read-only context only.
- Any `slicer-core` test file not listed above, any new test target/file, and all other crate waves.
- `docs/specs/test-quality-remediation-plan.md` outside the §7 `core` row and its heading-to-queue boundary; the parent owns queue-row bookkeeping.
- `docs/spec_packets/core-beading-threshold-review/**` for edits; its exports are reconstructed only through the delegated summary.

## Expected Sub-Agent Dispatches

- Question: run each exact test filter and return only the anchored one-passed/zero-failed result; scope: the four existing `slicer-core` test surfaces; return: `FACT` on success or bounded failure `SNIPPETS`.
- Question: verify the real §7 heading-to-queue span and update one accumulated `core` row without touching queue rows; scope: `docs/specs/test-quality-remediation-plan.md`; return: `FACT` with the real-plan predicate result.
- Question: run `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask check-test-quality --report`; scope: workspace gates; return: `FACT` pass/fail.

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

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: the real-plan predicate and row-scope verification; return `FACT` with one bounded success line or the first failing invariant.

## Open Questions

None. The packet is intentionally `draft` pending independent preflight; no activation or implementation decision is being made here.
