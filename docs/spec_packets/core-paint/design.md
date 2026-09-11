# Design: core-paint

## Controlling Code Paths

- Primary code path: `execute_paint_segmentation` short-circuit guards (empty mesh, no paint data, empty region map) in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`; `remove_nodes_with_one_arc` border/interior degree-1 rule in `voronoi_prune.rs`; `MMU_Graph::from_colored_lines` twin `?` propagation in `voronoi_graph.rs`; third-party `boostvoronoi::builder::Builder::with_segments` plus `diagram2` vertex color and edge `is_primary` accessors exercised by the probes.
- Neighboring tests/fixtures: `empty_mesh`, `one_layer_slice_ir`, `region_map_with_base_entry`, `mesh_with_no_paint`, `mesh_with_paint`, `empty_region_map`, `synthetic_square_input`, `MMU_Graph::from_parts` two-node fixture; the triangle-intersect lib tests strengthened by `core-strengthen` share the module gate but no test body.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Test-only change: no production signature, default, branch, or output changes; all eight existing names are preserved and exactly one new Err-test name is added.
- Feature gating is load-bearing: the module gate at `crates/slicer-core/src/algos/mod.rs` compiles all eight existing tests to zero under default features, so every test command uses `--features host-algos --lib` with substring filters; the bare-feature negative control in AC-N1 guards the silent-green hazard.
- Struct-literal discipline: any new fixture literal of a watched `pub` struct with ≥5 named fields uses a `..` rest or an `// exhaustive:` waiver; `ResolvedConfig` is a documented scanner blind spot, not a watched literal, per the `core-retire` grounding.
- Coordinate units: geometry-adjacent only for the prune `pt(0,0)`/`pt(100,0)` fixture, which is preserved verbatim, not recomputed.
- Determinism: no timing, sleep, ordering, or global-state assertions are added; the driver equality witnesses are structural (`len`, region count, polygon equality), not pointer identity.

## Code Change Surface

- Selected approach: strengthen-in-place for seven of the eight existing tests (all but the waived builder probe: nonempty guards and exact post-state/value assertions around the existing fixtures and production calls), retain the valid twin arm, add one Err-path test for the twin constructor derived from a real `return Err` site, and waive exactly one compile-witness probe with a reasoned `// test-quality:` comment. No fixture helper is deleted; no test is renamed.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` `driver_v2_tests`: `driver_v2_empty_mesh_returns_input_slice_ir`, `driver_v2_no_paint_data_short_circuits`, `driver_v2_empty_region_map_short_circuits` — add nonempty pre-assertions plus unchanged-path equality; keep fixtures and production call.
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` `tests::prune_one_arc_preserves_border_nodes` — replace MAY/no-panic comments with the exact pins `deleted == true`, both endpoints' `arc_indices` empty, both node structs surviving; keep the two-node `Border` arc fixture.
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` `tests::from_colored_lines_twin_propagation_is_question_mark` — keep valid-input `Ok` arm, add nonempty-diagram witness; new `from_colored_lines_invalid_input_returns_err_without_panic` feeds a single-segment input with first coordinate `2_147_483_648` (`i32::MAX + 1`) and asserts `Err(MmuGraphError::CoordinateOverflow(2147483648))` with no panic.
  - Same file probes: `boostvoronoi_supports_line_segment_sites` kept as compile witness with waiver; `vertex_color_get_set_round_trip` keeps `0`/`42`/isolation pins; `edge_is_primary_callable` keeps the `is_primary` existence pin over the square input.
- Rejected alternatives and reasons: retiring the driver triple as vacuous (rejected: each guards a distinct early-return branch worth pinning once nonempty); asserting pointer `Arc` identity for the empty-mesh arm (rejected: structural equality is the stable contract); sentinel-only twin move without an Err arm (rejected: rebuilds the success-only false green this packet exists to remove); gating or moving these tests to integration targets (rejected: they are lib unit tests by construction and the census tracks integration targets only).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - role: driver short-circuit tests; expected change: strengthen three test bodies only.
- `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` - role: twin test plus three probes; expected change: strengthen valid arm, add Err test, resolve probes with one waiver.
- `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` - role: prune post-state test; expected change: replace no-panic ending with exact pins.
- `docs/specs/test-quality-remediation-plan.md` - role: §7 `core` ledger row only; expected change: append this packet's dispositions and evidence while preserving accumulated content (required wave exit item; fourth file justified because it is a one-row ledger append, not a code surface).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-core/src/algos/mod.rs` - lines `1-40` only - purpose: confirm the `host-algos` module gate wording for AC-N1.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - lines `1881-1920` and `2081-2130` only - purpose: test module header plus the three driver bodies pre-edit.
- `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` - lines `242-260` and `539-580` only - purpose: tests module header plus prune body pre-edit.
- `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` - lines `1360-1380`, `1533-1610`, and `1649-1670` only - purpose: tests header, probe bodies, twin body pre-edit.
- `docs/22_test_quality.md` - delegated SUMMARY only - purpose: R3/R4 and compile-witness legitimacy mapping.
- `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY only - purpose: watched-struct rest/waiver rule.
- `docs/spec_packets/core-retire/packet.spec.md` - read-only predecessor boundary - purpose: scope and ledger-style conformance, never edited.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (no parity question in this packet).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-core/tests/*` - different test kind; not owned by this packet.
- `crates/slicer-core/src/polygon_ops.rs`, `crates/slicer-core/src/algos/prepass_slice.rs`, triangle-intersect sources outside the read windows - owned by prior packets.
- `docs/specs/test-quality-remediation-census.json` - committed census; target-level scope unchanged by lib-unit edits.
- Every other `docs/spec_packets/*` directory - never modified; predecessor files are read-only context.

## Expected Sub-Agent Dispatches

- Question: inventory the `return Err` sites inside `MMU_Graph::from_colored_lines` and state the smallest bad-diagram input reaching one; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`; return: `LOCATIONS`; purpose: twin Err-fixture derivation.
- Question: confirm the `remove_nodes_with_one_arc` deletion rule for a degree-1 interior node joined by a `Border` arc; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`; return: `SNIPPETS`; purpose: prune post-state pins.
- Question: run each pipe-suffixed AC and gate command and report pass/fail from the tee'd log; scope: workspace cargo only; return: `FACT`; purpose: implementation verification without loading output.

## Data and Contract Notes

- IR/manifest contracts: none; no IR field, config key, manifest entry, or schema version is asserted or changed.
- WIT boundary: none; host-side unit tests with no guest build input.
- Determinism/scheduler constraints: none; no scheduler, ordering, or timing surface is touched.

## Locked Assumptions and Invariants

- All eight existing test names survive; the single added Err-test name is fixed by AC-3.
- The two-node prune fixture (`Border` arc from node 0 to node 1) and the square `synthetic_square_input` are preserved; strengthening adds assertions, not new geometries.
- The `42`/`0` color distinction, the `is_primary` existence claim, and the driver polygon-equality witnesses remain independently expected literals, never derived from the code under test.

## Risks and Tradeoffs

- The twin Err input depends on the `to_i32` overflow guard (`|v| > i32::MAX` checked before the zero-length skip, so `&[]` and degenerate in-range segments stay `Ok`); mitigation: the discovery dispatch re-derives the guard before the twin edit, and the fixed `2_147_483_648` coordinate (`i32::MAX + 1`, not a magic number) keeps the intent readable if the limit ever moves.
- Prune post-state derivation may show the current MAY comment was right to hedge (e.g. `delete_arc` clears both endpoints); mitigation: pin whatever the implementation deterministically does, verified by running the single filter, rather than asserting a preferred semantic.
- Probe waiver may be misread as permission for future success-only tests; mitigation: the waiver names the single protected build surface and cites the earn-their-keep decision, and the gate step checks the touched scope stays at zero unwaived findings.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: twin Err-site inventory; `LOCATIONS` with at most 20 file:line entries.

## Open Questions

None.
