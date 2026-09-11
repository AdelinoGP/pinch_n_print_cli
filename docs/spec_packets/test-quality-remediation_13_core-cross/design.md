# Design: core-cross

## Controlling Code Paths

- Primary code path: the SDK wrapper delegation arms on native — `offset_polygons`/`offset_polygons_with_miter_limit` (`crates/slicer-sdk/src/host.rs`) → private `offset_polygons_with_optional_miter_limit` → `slicer_core::polygon_ops::offset` / `offset_with_miter_limit` (`crates/slicer-core/src/polygon_ops.rs`); `offset_polygons_batch` (`crates/slicer-sdk/src/host_batch.rs`) mapping over the singulars; the SDK inline collinear-drop branch of `simplify_polygon` (`host.rs`) and the batch simplify; the MeshSource-trait routing of `raycast_z_down`/`object_bounds` (`host.rs`); and the core counterparts `expolygons_simplify`/private RDP `simplify_polygon` (`polygon_ops.rs`) and `AabbTree::bounds`/`raycast_first_hit`/`raycast_all_hits` (`crates/slicer-core/src/aabb_tree.rs`).
- Neighboring tests/fixtures: `StubMesh` and `test_support` (install/clear mesh source), the `square(0, 100_000)` helper (10 mm × 10 mm), `unit_cube_mesh()`/`empty_mesh()` in `aabb_tree_tdd.rs`, and the polygon-op unit-test fixtures around the two marked tests.
- OrcaSlicer comparison: none; no OrcaSlicer behavior is consulted by this packet.

## Architecture Constraints

- Test-only change: no production signature, branch, threshold, or output changes; thirteen existing tests are preserved with marker-comment hardening only, five more existing tests are roster-protected but untouched, and exactly one new test is added (`offset_polygons_with_miter_limit_clamps_sharp_miter_corners`).
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Step 2 edits `crates/slicer-sdk/src/host_batch.rs`, which is inside `crates/slicer-sdk/**` and therefore in the guest fingerprint closure even though the edit is comment-only; the implementer runs `cargo xtask build-guests --check` once after Step 2 and rebuilds if the exit code is non-zero (1 = stale, 3 = infrastructure error, never treated as fresh). Steps 1/3/4 edit tests and docs only, so no guest input.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The new test's fixture literals follow the existing `square()` helper convention (`100_000` units = 10 mm, deltas in mm) — the unit conversion under test lives in core `inflate_once`, not in the test; the coord-system bullet documents that the literals are unit-correct.
- No schema/version constant is bumped; the version-locking constraint does not apply.

## Code Change Surface

- Selected approach: KEEP-review resolution with earn-their-keep markers (ADR-0064 §Decision requires a named regression input per kept test, and the plan §7 ledger requires a disposition record), plus one gap-closing delegate-contract test. For the offset family the packet makes no duplicate-implementation claim: the wrapper body is the core call (plus join conversion), so the correct cover is behavior-through-wrapper tests (non-empty shrink/grow, clamped-vs-default miter difference) with markers stating "thin-delegate cover", not a wrapper-vs-core equality pin (self-referential oracle, docs/22_test_quality.md §2.1). For simplify and raycast/bounds the packet records the distinct-implementation rationale: SDK inline collinear drop vs core RDP; SDK MeshSource routing (no core call) vs core AabbTree.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-sdk/tests/host_wrappers_tdd.rs` — six markers on `object_bounds_returns_host_unavailable_without_source`, `raycast_and_normal_route_through_installed_mesh_source`, `raycast_returns_none_without_source_documented_signal`, `offset_polygons_shrinks_and_grows`, `simplify_polygon_drops_collinear_vertices`, `simplify_polygon_short_input_returned_as_is`; one new test `offset_polygons_with_miter_limit_clamps_sharp_miter_corners` inserted between `offset_polygons_shrinks_and_grows` and the simplify section, calling `host::offset_polygons_with_miter_limit(&a, -1.0, OffsetJoinType::Miter, 0.0, 1.2)` on `square(0, 100_000)` and asserting both arms non-empty, `assert_ne!(grown, clamped, ...)` against the default-limit `host::offset_polygons(&a, -1.0, OffsetJoinType::Miter, 0.0)` result, and a strict point-count increase on the clamped contour.
  - `crates/slicer-sdk/src/host_batch.rs` — two markers above `batch_offset_keeps_results_aligned_with_inputs` and `batch_forms_agree_with_the_singular_forms` in the `#[cfg(test)]` tests module.
  - `crates/slicer-core/tests/aabb_tree_tdd.rs` — three markers above `bounds_match_unit_cube_vertex_extrema`, `positive_z_raycast_from_below_hits_cube_bottom_face_first`, `raycast_all_hits_returns_sorted_entry_and_exit_intersections`.
  - `crates/slicer-core/src/polygon_ops.rs` — two markers above `offset_round_trip_preserves_hole_nesting` and `expolygons_simplify_preserves_square` in `mod tests`; production functions untouched.
  - `docs/specs/test-quality-remediation-plan.md` — §7 `core` ledger row only (see Read-Only Context for the row format).
- Rejected alternatives and reasons:
  - Retiring any kept test (rejected: each names a regression input — see markers; retirement without replacement violates the KEEP disposition).
  - Adding a wrapper-vs-core delegation equality test for offset (rejected: self-referential oracle — the wrapper body is the core call; the behavior tests are the honest delegate contract).
  - Rewriting any test body beyond the new test (rejected: the audit found no false-green patterns in these tests; bodies already satisfy docs/22 §1 counterfactual).
  - Touching `clip_polygons_union_produces_nonempty_result_for_real_input` (rejected: owned by the §5.11 runtime clip-trio review).
  - Leaving the `Some(miter_limit)` arm uncovered (rejected: the row's goal names both offset delegation arms, and this arm has zero coverage anywhere today — a review finding, closed by the one new test).
  - Asserting an exact clamped-vertex count in the new test (rejected: unmeasured Clipper2 detail; the pinned assertions are non-empty, contour inequality, and strict count increase, which are implementation-verified when the test first runs).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-sdk/tests/host_wrappers_tdd.rs` - role: SDK wrapper coverage home (offset/simplify/raycast/bounds); expected change: seven marker comments (six on retained tests + one above the new test) plus one new miter-limit delegate test.
- `crates/slicer-sdk/src/host_batch.rs` - role: batch offset wrapper unit-test home; expected change: two marker comments (guest-staleness bullet applies).
- `crates/slicer-core/tests/aabb_tree_tdd.rs` - role: core AabbTree counterpart coverage; expected change: three marker comments.
- `crates/slicer-core/src/polygon_ops.rs` - role: core offset/simplify counterpart coverage in `mod tests`; expected change: two marker comments (production code untouched).
- `docs/specs/test-quality-remediation-plan.md` - role: §7 `core` ledger row only; expected change: append this packet's KEEP-review dispositions and evidence while preserving accumulated content (required wave exit item; fifth file justified because four tiny marker surfaces are each one comment-block edit in a file the CROSS row owns by definition — the row spans both crates — and the ledger is a one-row append, not a code surface).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-sdk/src/host.rs` - lines `390-445` and `520-620` only - purpose: verify the MeshSource routing arms and the offset delegation arms + inline simplify branch (verified at authoring; re-check the two arms at implementation).
- `crates/slicer-sdk/src/host_batch.rs` - lines `136-180` and `424-496` only - purpose: native batch mapping + the two unit tests' current text.
- `crates/slicer-core/src/polygon_ops.rs` - lines `420-500`, `700-800`, and `900-1200` only - purpose: `offset`/`offset_with_miter_limit`/`expolygons_simplify` production shapes and the `mod tests` region containing the two marked tests.
- `crates/slicer-core/src/aabb_tree.rs` - lines `40-60` only - purpose: `AabbTree::bounds`/`raycast_first_hit`/`raycast_all_hits` signatures.
- `crates/slicer-core/tests/aabb_tree_tdd.rs` - lines `85-240` only - purpose: the six-test roster and the three marked tests' bodies.
- `crates/slicer-sdk/tests/host_wrappers_tdd.rs` - lines `85-230` only - purpose: the eleven-test roster, `square()` helper, and the five marked tests' bodies.
- `crates/slicer-sdk/Cargo.toml` - lines `95-110` only - purpose: the `host_wrappers_tdd` `required-features = ["test"]` stanza.
- `docs/specs/test-quality-remediation-census.json` - lines `256-270` and `741-755` only - purpose: confirm `aabb_tree_tdd` has `required: []` and `host_wrappers_tdd` has `required: ["test"]`; never loaded in full, never modified.
- `docs/22_test_quality.md` - direct read (215 lines) §§1-3 and §5, or delegated SUMMARY - purpose: earn-their-keep and waiver mapping.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read (44 lines) - purpose: retire-if-unjustified burden of proof.
- `docs/specs/test-quality-remediation-plan.md` - §5.1 CROSS row, §6, §7 ledger header/row, Packet Queue row #13, row-#12 resume pointer; ranged reads only.
- `docs/spec_packets/test-quality-remediation_12_core-brittle/packet.spec.md` - read-only predecessor boundary (ledger-row AC pattern) - purpose: accumulated-row format and gate-token conventions, never edited.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (no parity question in this packet).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `docs/specs/test-quality-remediation-census.json` - never loaded in full, never modified.
- Every other `docs/spec_packets/*` directory - never modified; predecessor files are read-only context.
- `crates/slicer-core/tests/polygon_ops_tdd.rs` and all other `crates/slicer-core/tests/*` beyond `aabb_tree_tdd.rs` - owned by other wave rows.
- `crates/slicer-sdk/src/host.rs` and the production halves of `host_batch.rs`/`polygon_ops.rs` - read-only delegation verification only; no edits.
- `crates/slicer-wasm-host/**` - the §5.8 CROSS family; delegate, never browse.

## Expected Sub-Agent Dispatches

- Question: run a pipe-suffixed AC or gate command and report pass/fail from the tee'd log (never the raw output); scope: workspace cargo only; return: `FACT`; purpose: implementation verification without loading output.
- Question: confirm `cargo xtask check-test-quality --report` lists zero unwaived findings on the four touched test surfaces; scope: the four paths; return: `FACT`; purpose: gate closure.
- Question: confirm the `cargo xtask build-guests --check` exit code after Step 2 (0 fresh / 1 stale / 3 infra); scope: xtask; return: `FACT`; purpose: guest-staleness discipline.
- Question: read the current §7 `core` ledger row and report its exact six cells; scope: `docs/specs/test-quality-remediation-plan.md` §7 row only; return: `FACT`; purpose: Step 5 append base.

## Data and Contract Notes

- IR/manifest contracts: none; no IR field, config key, manifest entry, or schema version is asserted or changed.
- WIT boundary: none; host-side tests only. The `#[cfg(target_arch = "wasm32")]` wrapper arms are untouched and outside test scope.
- Determinism/scheduler constraints: no timing, sleep, ordering, or global-state assertions are added (R8-safe); the new test is deterministic geometry.

## Locked Assumptions and Invariants

- The thirteen existing tests keep their exact names, bodies, and file positions; the new test is inserted at the pinned position; `clip_polygons_union_produces_nonempty_result_for_real_input` and the three unmarked aabb tests stay byte-identical.
- The exact marker strings pinned by AC-1 through AC-6 are the recorded KEEP-review evidence; no marker is added to any other file (confirming grep in the Doc Impact Statement).
- The §7 ledger row accumulates; this packet appends its own tokens and must not rewrite or re-sort other packets' cells.

## Risks and Tradeoffs

- The new test's strict point-count assertion depends on Clipper2's clipped-miter emitting two vertices per clamped corner; the load-bearing pinned assertions are non-empty + `assert_ne!` + strictly-more, and the exact split is verified when the test first runs (documented above under Rejected alternatives — if the count shape differs, the body adjusts without weakening the pinned assertions).
- Marker comments can drift from the code they describe; mitigation: comments name exact symbols and inputs already present in the test bodies, verified by the AC literal pins.
- The SDK test runs compile the full native `slicer-core` graph with `host-algos`; mitigated by delegating every cargo run and teeing to `target/test-output.log`.

## Context Cost Estimate

- Aggregate: `M` (approved)
- Largest step: `S`
- Highest-risk dispatch and required return format: miter-limit test first run; `FACT` pass/fail from the tee'd log.

## Open Questions

None.
