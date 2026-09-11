# Requirements: core-cross

## Packet Metadata

- Grouped task IDs: `core/CROSS`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `M` (approved; largest single step `S`)

## Problem Statement

The §5.1 CROSS audit row flagged the SDK/core wrapper families for review under the KEEP-review disposition. The verified correspondence splits the row into three distinct conclusions that must be recorded so a future audit does not re-litigate them (ADR-0064 §Consequences): (1) the SDK offset wrappers are NOT distinct implementations — native `offset_polygons`/`offset_polygons_with_miter_limit` delegate to `slicer_core::polygon_ops::offset`/`offset_with_miter_limit` and the batch form maps over the singulars, so their cover is a thin-wrapper delegate contract and no duplicate-implementation claim may be made; (2) SDK `simplify_polygon` (inline collinear drop, tolerance reserved) and core `expolygons_simplify` (RDP) are genuinely distinct and both must be kept; (3) SDK `raycast_z_down`/`object_bounds` (MeshSource-trait routing, no core call) and core `AabbTree::bounds`/`raycast_first_hit`/`raycast_all_hits` are genuinely distinct and both must be kept. The review also found one real coverage hole: `offset_with_miter_limit` has zero test coverage anywhere in the tree — its only call site is the SDK wrapper's `Some(miter_limit)` arm — so delegate-contract cover is incomplete without one new test. This packet resolves the row, test-only, by pinning earn-their-keep rationale markers on every retained test and closing the miter-limit gap.

## In Scope

- Marker-comment review pass on the six retained SDK wrapper tests in `crates/slicer-sdk/tests/host_wrappers_tdd.rs`: `object_bounds_returns_host_unavailable_without_source`, `raycast_and_normal_route_through_installed_mesh_source`, `raycast_returns_none_without_source_documented_signal`, `offset_polygons_shrinks_and_grows`, `simplify_polygon_drops_collinear_vertices`, `simplify_polygon_short_input_returned_as_is` — each gains an exact `// KEEP-review (core-cross): ...` marker naming the distinctness/contract conclusion and a named regression input (per AC-1, AC-3, AC-4 marker pins).
- New delegate-contract test `offset_polygons_with_miter_limit_clamps_sharp_miter_corners` in `crates/slicer-sdk/tests/host_wrappers_tdd.rs`, inserted immediately after `offset_polygons_shrinks_and_grows` — the only coverage of the `Some(miter_limit)` → `slicer_core::polygon_ops::offset_with_miter_limit` arm (AC-1).
- Marker pass on the two offset-batch unit tests in `crates/slicer-sdk/src/host_batch.rs`: `batch_offset_keeps_results_aligned_with_inputs`, `batch_forms_agree_with_the_singular_forms` (AC-2).
- Marker pass on the three core counterpart tests in `crates/slicer-core/tests/aabb_tree_tdd.rs`: `bounds_match_unit_cube_vertex_extrema`, `positive_z_raycast_from_below_hits_cube_bottom_face_first`, `raycast_all_hits_returns_sorted_entry_and_exit_intersections` (AC-5).
- Marker pass on the two core counterpart tests in the `mod tests` of `crates/slicer-core/src/polygon_ops.rs`: `offset_round_trip_preserves_hole_nesting`, `expolygons_simplify_preserves_square` (AC-6).
- §7 `core` ledger-row update of `docs/specs/test-quality-remediation-plan.md` recording this packet's dispositions, kept-test roster, oracle tokens, representative validation command, gate tokens, and remaining-gap state (AC-7).
- Read-only verification of the delegation correspondences (host.rs offset arms and MeshSource routing, polygon_ops.rs offset/expolygons_simplify, aabb_tree.rs AabbTree methods) — verified during authoring; the implementer re-confirms only the two load-bearing ranges in each step.

## Out of Scope

- Production code changes of any kind (no edits to `host.rs`, `host_batch.rs` production fns, `polygon_ops.rs` production fns, or `aabb_tree.rs`).
- `docs/specs/test-quality-remediation-census.json` — never touched; this packet adds no test target and removes none, so the census stays valid.
- Other packet directories (`core-flow-consolidation` … `core-brittle`, `core-parity`) — never modified; predecessor files are read-only context.
- The §5.8 SDK CROSS row (host wrappers vs wasm-host simplify) and the §5.11 runtime CROSS row (clip trio ↔ `polygon_ops.rs`) — different families, owned elsewhere.
- Row #14 `core-parity` parity additions and any OrcaSlicer parity work — none applies here; no `OrcaSlicerDocumented/` content is consulted.
- Other tests in the touched files (`clip_polygons_union_produces_nonempty_result_for_real_input` in host_wrappers_tdd.rs is owned by the runtime clip-trio review; `empty_mesh_reports_no_bounds_hits_or_closest_point`, `closest_point_projects_queries_below_and_above_the_cube`, `ray_miss_returns_no_intersections` in aabb_tree_tdd.rs; the other offset unit tests in polygon_ops.rs) — retained untouched, protected by the AC-N1 roster pins.
- `crates/slicer-core/tests/polygon_ops_tdd.rs` — a different test target owned by the DUP-CORE-strengthen row.
- Adding any wrapper-vs-core equality pin for the offset pair — rejected: the wrapper body is exactly the core call (plus join conversion), so such a pin is a self-referential oracle (docs/22_test_quality.md §2.1); the retained behavior tests are the correct delegate-contract cover.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 CROSS row, §6, §7 header, Packet Queue row #13; read only those ranges.
- `docs/22_test_quality.md` - 215 lines; direct read of §§1-3 and §5 (earn-their-keep, false-green taxonomy, legitimate weak forms, R1-R8 table).
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - 44 lines; direct read (burden of proof, survivor-map standard).
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - delegated SUMMARY (report-mode closure obligation).
- `docs/08_coordinate_system.md` - delegated SUMMARY (unit convention behind `100_000` = 10 mm fixture literals).
- `docs/spec_packets/core-brittle/` - delegated predecessor summary; draft, independently `PREFLIGHT PASS`, exports none.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (SDK offset delegate arms + new miter-limit test), `AC-2` (batch offset wrappers), `AC-3` (SDK simplify distinct), `AC-4` (SDK raycast/bounds MeshSource distinct), `AC-5` (core AabbTree counterparts), `AC-6` (core polygon-op counterparts), `AC-7` (ledger core row).
- Negative: `AC-N1` (exact 12-name SDK roster and exact 6-name core roster in file order; required-features rejection of the bare SDK run).
- Cross-packet impact: none — exports from #11/`core-paint` and #12/`core-brittle` are zero (test-only), and this packet exports nothing itself; `core-parity` (row #14) consumes no symbol from here.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test host_wrappers_tdd -- offset_polygons_with_miter_limit_clamps_sharp_miter_corners --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` + marker/`assert_ne!` pins | AC-1: new miter-limit delegate test plus its two markers and live assertion | FACT pass/fail; SNIPPETS <=10 lines on failure |
| `set -euo pipefail; mkdir -p target; for t in batch_offset_keeps_results_aligned_with_inputs batch_forms_agree_with_the_singular_forms; do cargo test -p slicer-sdk --features test --lib "$t" -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; done` + marker pins | AC-2: batch offset wrappers | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test host_wrappers_tdd -- simplify_polygon --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 2 passed; 0 failed;' target/test-output.log` + marker pins | AC-3: SDK simplify pair | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test host_wrappers_tdd -- raycast --nocapture ...` (expect 2 passed) then `... -- object_bounds --nocapture ...` (expect 1 passed) + marker pins | AC-4: SDK raycast/bounds via MeshSource | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; for t in bounds_match_unit_cube_vertex_extrema positive_z_raycast_from_below_hits_cube_bottom_face_first raycast_all_hits_returns_sorted_entry_and_exit_intersections; do cargo test -p slicer-core --features host-algos --test aabb_tree_tdd -- "$t" --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; done` + marker pins | AC-5: core AabbTree counterparts | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; for t in offset_round_trip_preserves_hole_nesting expolygons_simplify_preserves_square; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; done` + marker pins | AC-6: core polygon-op counterparts | FACT pass/fail |
| AC-7 python ledger predicate (packet.spec.md) | AC-7: §7 core-row content | FACT pass/fail |
| AC-N1 roster pythons + bare-run rejection (`if cargo test -p slicer-sdk --test host_wrappers_tdd -- --list ...`) | AC-N1: roster exactness + feature-flag enforcement | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 \| tee target/core-cross-check.log >/dev/null` | compile gate incl. test targets | FACT pass/fail (exit code) |
| `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/core-cross-clippy.log >/dev/null` | lint gate `-D warnings` | FACT pass/fail (exit code) |
| `cargo xtask check-literals` | struct-literal gate (no new literals expected) | FACT pass/fail |
| `cargo xtask check-test-quality --report` scoped to the four touched test surfaces | R-rule findings on touched files (markers/waivers only) | FACT zero-unwaived on the four paths |
| `cargo xtask build-guests --check` (after Step 2 only) | guest fingerprint freshness after the `host_batch.rs` edit | FACT exit code 0/1/3 |

Commands must have small, parseable output suitable for delegation; every cargo **test** run tees to `target/test-output.log` and never re-runs to inspect output.

## Step Completion Expectations

- Steps 1-4 are independent edit surfaces (one file each) and may run in any order; Step 5 (ledger) runs last and depends only on the dispositions being final.
- The new test name inserted by Step 1 is a load-bearing token for AC-1, AC-7, and AC-N1 — no other step may rename or move it.
- The exact marker lines pinned by AC-1 through AC-6 are authored once and never edited by another step.

## Context Discipline Notes

- The §5.1 CROSS row, Packet Queue, and §7 ledger of `docs/specs/test-quality-remediation-plan.md` are ranged reads; the census JSON is never loaded in full (two targeted entries only).
- The SDK test runs compile the whole `slicer-core` graph under `host-algos` (native SDK dependency) plus the sdk dev graph — expect slow builds; delegate every cargo run.
- Authoritative-doc reads (`docs/08_coordinate_system.md`, ADR-0065, predecessor packet summaries) are delegated SUMMARY dispatches, never full loads.
