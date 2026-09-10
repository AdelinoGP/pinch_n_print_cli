# Requirements: core-geometry-dup-review

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE` (segment path), `core/DUP-CORE` (point-to-segment distance)
- Backlog source: `docs/specs/test-quality-remediation-plan.md` (plan wave/item IDs per the user-approved mapping exemption; not `docs/07_implementation_status.md` `TASK-###`)
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The §5.1 `DUP-CORE` audit row names two slicer-core geometry overlaps. Grounded against the tree:

1. `segment_path_preserves_requested_endpoints` (inline `#[cfg(test)] mod tests` in `crates/slicer-core/src/lib.rs`) constructs `segment_path(Point2::from_mm(0.0, 0.0), Point2::from_mm(2.0, 0.0), 0.75)` and asserts only first/last endpoints. `crates/slicer-core/tests/geometry_helpers_tdd.rs` already holds stronger segment-path oracles: `segment_path_subdivides_long_segment_with_endpoints_preserved` (exact per-point coordinates), `segment_path_handles_exact_division_short_segment_and_zero_length` (exact/short/zero-length split), and `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` (a roster asserting non-emptiness, endpoints, and the per-chord cap). The inline test is a strict assertion subset; its one distinct asset is the specific (2.0 mm, 0.75 mm) input tuple, which no pre-edit roster case exercises.
2. `point_to_segment_distance_squared_matches` (inline test module in `crates/slicer-core/src/geometry.rs`) builds the fixture `a=(0,0)`, `b=(10_000,0)`, `p=(5000,5000)` and asserts the wrapper's result against the independent `25_000_000.0_f64` oracle at `< 2.0` tolerance — the identical fixture and oracle already asserted by `closest_point_on_segment_midpoint` in the same module, which additionally pins the projected point within 1 unit of `(5000, 0)`. The one non-subsumed surface is the wrapper identity itself: that `point_to_segment_distance_squared` delegates to `closest_point_on_segment` and returns its `distance_sq`.

Both folds are case-preserving merges under the ADR-0064 earn-their-keep standard: deleting either source test leaves a named regression input uncovered only if the merge drops its distinct input or oracle, which this packet forbids. The slice is coherent because both candidates are geometry-adjacent, share the same three-file edit surface, and are the remaining §5.1 items whose disposition is fold/review rather than additive strengthening.

## In Scope

- Fold `segment_path_preserves_requested_endpoints` into `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` (in `crates/slicer-core/tests/geometry_helpers_tdd.rs`) by extending its `cases` array to the six-tuple contractual union — `(1.0_f32, 0.5_f32)`, `(1.0_f32, 0.75_f32)`, `(2.5_f32, 0.7_f32)`, `(5.0_f32, 2.0_f32)`, `(11.25_f32, 3.0_f32)`, plus the preserved `(2.0_f32, 0.75_f32)` — binding `start`/`end`, adding exact `assert_eq!(points.first(), Some(&start))` / `assert_eq!(points.last(), Some(&end))` endpoint witnesses, a non-vacuity `assert!(!points.is_empty())`, per-chord cap checks, and per-chord `> 0.0` non-degeneracy for every case (all six inputs are nonzero-length); then deleting the inline test from `crates/slicer-core/src/lib.rs`.
- Consolidate `point_to_segment_distance_squared_matches` and `closest_point_on_segment_midpoint` in the inline module of `crates/slicer-core/src/geometry.rs` into one survivor named `point_to_segment_distance_squared_matches` that calls BOTH APIs explicitly on the shared fixture, retains BOTH independent `25_000_000.0_f64` `< 2.0` oracles (one per API), retains the `cp.point.x`/`cp.point.y` ±1 projection witnesses, and adds the wrapper-delegate `assert_eq!(dsq, cp.distance_sq)` witness. Exact implemented assertions: AC-3.
- Preserve the distinct endpoint witness: the (2.0 mm, 0.75 mm) input survives verbatim in the roster test after the fold.
- Record the §7 `core` ledger row update in `docs/specs/test-quality-remediation-plan.md` (Step 2), verified by AC-5's scoped parser and its four exercised synthetic failure controls.

## Out of Scope

- Production behavior, signatures, or doc comments of `segment_path`, `point_to_segment_distance_squared`, `closest_point_on_segment`, `closest_point_on_polygons`, or any other core function.
- `flow_correction_stays_positive_for_vertical_input` (inline in `crates/slicer-core/src/lib.rs`) — owned by `core-strengthen` (row #9).
- `beading_factory.rs` threshold propagation — owned by `core-beading-threshold-review` (row #8).
- All §5.1 items other than the two named candidates (bridge, region mapping, support, wall sequence, flow, paint, brittle, cross, parity, strengthen, retire rows).
- Any `OrcaSlicerDocumented/` inspection. Correction of a generation-time claim: `crates/slicer-core/src/geometry.rs` (like `crates/slicer-core/src/lib.rs`) already carries the standard AGPLv3 OrcaSlicer porting header (derived from `src/libslic3r/Point.hpp`, `src/libslic3r/Geometry.hpp`); this packet ports no new C++ code (test-only edits), so no new attribution text is required.
- New test files, new production modules, WIT/IR/schema changes, or new public symbols.
- Editing `docs/specs/test-quality-remediation-plan.md`'s Packet Queue (parent-owned; the row stays `pending` until independent preflight PASS) or any existing packet directory.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 DUP-CORE row (authoritative source of both candidates); §6 command patterns (`--features host-algos` mandatory on core runs; tee to `target/test-output.log`); §7 ledger (AC-5 target). Read §5.1, §6, §7 by section; never load the whole file.
- `docs/22_test_quality.md` - §1 earn-their-keep counterfactual and §4 derivation rule govern the survivor map.
- `docs/21_data_defaults_and_fixtures.md` - §1/§4 FRU/waiver rules for the roster edit (the `cases` array is a plain array, not a watched struct literal; no waiver needed).
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - retirement standard applied to both folds; delegated FACT if its normative clause is disputed.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - why `check-test-quality --report` runs in the matrix; delegated FACT.

## Acceptance Summary

- Positive: `AC-1` (structural retirement), `AC-2` (six-case roster survivor under an exact single-test filter), `AC-3` (dual-API, dual-oracle consolidation under a fully qualified `--lib` exact filter), `AC-4` (positive full-target count).
- Negative: `AC-N1` (row-#9 item untouched; no `segment_path_*` test left in the lib inline module).
- Ledger: `AC-5` (sole owner `packet.spec.md`; this file and `implementation-plan.md` reference it by ID only).
- Cross-packet impact: none net-new — `core-wall-sequence-dup-merges` (dependency, `generated`) exports no symbols or files; this packet likewise creates none, so rows #8–#14 are unaffected.

## Verification Commands

The authoritative matrix. AC commands live solely in `packet.spec.md` and are referenced here by ID; this matrix holds the packet-level gates only.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | closure gate; all targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate; lint-clean | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal gate on the edited files | FACT: exit 0 |
| `cargo xtask check-test-quality --report` | test-quality gate; report-mode findings for the three edited files must be 0 or waived | FACT: no finding names `crates/slicer-core/tests/geometry_helpers_tdd.rs`, `crates/slicer-core/src/lib.rs`, or `crates/slicer-core/src/geometry.rs` |

ACs `AC-1`–`AC-5` and `AC-N1` are verified by their own pipe-suffixed commands in `packet.spec.md`; dispatch them, do not restate them. Never re-run tests to inspect results — read `target/test-output.log`.

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, or shared scratch state. Per-step pre/postconditions belong in `implementation-plan.md`.

- Step 1 must complete both folds before any verification run: the lib-target exact filter (AC-3) is meaningful only after the geometry.rs consolidation, and AC-2 exercises the roster survivor only after its tuple and assertions land.
- `target/test-output.log` is overwritten every run; capture findings from a run before launching the next.
- Step 2 (ledger) runs only after Step 1's verification is green; its row content is the Step 1 survivor map, so no scratch state crosses steps.

## Context Discipline Notes

Only packet-specific hazards: large ranged/delegated files, tempting reads to skip, and heavy-dispatch return limits.

- `docs/specs/test-quality-remediation-plan.md` is long: read §5.1, §6, and §7 directly by section anchor; never load the whole file to check the ledger.
- Do not open `docs/specs/test-quality-remediation-census.json` in full; the needed fact (`geometry_helpers_tdd` → `required: []`) is already recorded in `packet.spec.md`'s command-discipline note.