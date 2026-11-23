---
status: implemented
packet: test-quality-remediation_13_core-cross
task_ids:
  - 'core/CROSS'
---

# test-quality-remediation_13_core-cross

## Goal

Review the SDK/core wrapper coverage for the offset, simplify, and raycast/bounds families, keep the two genuinely distinct implementation pairs (simplify, raycast/bounds) with earn-their-keep rationale markers on every retained test, retain thin-delegate contract cover for the SDK offset delegation including the currently uncovered miter-limit arm, and record the dispositions in the §7 ledger row.

## Problem Statement

The §5.1 CROSS audit row flagged the SDK/core wrapper families for review under the KEEP-review disposition. The verified correspondence splits the row into three distinct conclusions that must be recorded so a future audit does not re-litigate them (ADR-0064 §Consequences): (1) the SDK offset wrappers are NOT distinct implementations — native `offset_polygons`/`offset_polygons_with_miter_limit` delegate to `slicer_core::polygon_ops::offset`/`offset_with_miter_limit` and the batch form maps over the singulars, so their cover is a thin-wrapper delegate contract and no duplicate-implementation claim may be made; (2) SDK `simplify_polygon` (inline collinear drop, tolerance reserved) and core `expolygons_simplify` (RDP) are genuinely distinct and both must be kept; (3) SDK `raycast_z_down`/`object_bounds` (MeshSource-trait routing, no core call) and core `AabbTree::bounds`/`raycast_first_hit`/`raycast_all_hits` are genuinely distinct and both must be kept. The review also found one real coverage hole: `offset_with_miter_limit` has zero test coverage anywhere in the tree — its only call site is the SDK wrapper's `Some(miter_limit)` arm — so delegate-contract cover is incomplete without one new test. This packet resolves the row, test-only, by pinning earn-their-keep rationale markers on every retained test and closing the miter-limit gap.

## Architecture Constraints

- Test-only change: no production signature, branch, threshold, or output changes; thirteen existing tests are preserved with marker-comment hardening only, twenty-six tests across the four touched files are untouched (five in `host_wrappers_tdd.rs`: the three logging tests `log_capture_records_all_levels_in_order` / `log_level_str_matches_host_mapping` / `take_log_messages_uninstalls_sink`, plus `clip_polygons_union_produces_nonempty_result_for_real_input` and `now_us_is_monotonic_within_a_thread`; three in `aabb_tree_tdd.rs`; eighteen in `polygon_ops.rs`'s `mod tests`; none in `host_batch.rs` since both its tests are marked), and exactly one new test is added (`offset_polygons_with_miter_limit_clamps_sharp_miter_corners`). AC-N1 pins the complete 12-name and 6-name rosters of `host_wrappers_tdd.rs` and `aabb_tree_tdd.rs`; the untouched `polygon_ops.rs` tests rest on AC-6's comment-only diff discipline.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Step 2 edits `crates/slicer-sdk/src/host_batch.rs`, which is inside `crates/slicer-sdk/**` and therefore in the guest fingerprint closure even though the edit is comment-only; the implementer runs `cargo xtask build-guests --check` once after Step 2 and rebuilds if the exit code is non-zero (1 = stale, 3 = infrastructure error, never treated as fresh). Steps 1/3/4 edit tests and docs only, so no guest input.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The new test's fixture literals follow the existing `square()` helper convention (`100_000` units = 10 mm, deltas in mm) — the unit conversion under test lives in core `inflate_once`, not in the test; the coord-system bullet documents that the literals are unit-correct.
- No schema/version constant is bumped; the version-locking constraint does not apply.

## Data and Contract Notes

- IR/manifest contracts: none; no IR field, config key, manifest entry, or schema version is asserted or changed.
- WIT boundary: none; host-side tests only. The `#[cfg(target_arch = "wasm32")]` wrapper arms are untouched and outside test scope.
- Determinism/scheduler constraints: no timing, sleep, ordering, or global-state assertions are added (R8-safe); the new test is deterministic geometry.

## Locked Assumptions and Invariants

- The thirteen existing tests keep their exact names, bodies, and file positions; the new test is inserted at the pinned position; `clip_polygons_union_produces_nonempty_result_for_real_input` and the three unmarked aabb tests stay byte-identical.
- The exact marker strings pinned by AC-1 through AC-6 are the recorded KEEP-review evidence; no marker is added to any other file (confirming grep in the Doc Impact Statement).
- The §7 ledger row accumulates; this packet appends its own tokens and must not rewrite or re-sort other packets' cells.

## Risks and Tradeoffs

- The new test's strict point-count assertion depends on Clipper2's clipped-miter emitting two vertices per clamped corner; the load-bearing pinned assertions are non-empty + `assert_ne!` + strictly-more. **Measured** (2026-09-15 probe, `target/probe-miter2.log`): `+1.0` / limit `1.2` / arc `0.0` gives 4 default-limit points vs **8** clamped points (`assert_ne!` true, strict increase true); limit `1.5+` gives 4 vs 4 (no clamp, as expected, since `1/sin 45° ≈ 1.4142 < 1.5`); `Round` join at `+1.0` gives 56 vs 56 (clamp correctly does not apply). The count is asserted as a strict increase, never an exact value.
- Marker comments can drift from the code they describe; mitigation: comments name exact symbols and inputs already present in the test bodies, verified by the AC literal pins.
- The SDK test runs compile the full native `slicer-core` graph with `host-algos`; mitigated by delegating every cargo run and teeing to `target/test-output.log`.
