---
status: implemented
packet: test-quality-remediation_07_core-geometry-dup-review
task_ids:
  - core/DUP-CORE (segment path)
  - core/DUP-CORE (point-to-segment distance)
---

# test-quality-remediation_07_core-geometry-dup-review

## Goal

Consolidate the two slicer-core geometry test-overlap pairs — `segment_path_preserves_requested_endpoints` (inline test module of `crates/slicer-core/src/lib.rs`) and `point_to_segment_distance_squared_matches` + `closest_point_on_segment_midpoint` (inline test module of `crates/slicer-core/src/geometry.rs`) — into case-preserving survivors, preserving the distinct (2.0 mm, 0.75 mm) endpoint input, explicit calls to both geometry APIs, and the independent `25_000_000.0_f64` distance oracle.

## Problem Statement

The §5.1 `DUP-CORE` audit row names two slicer-core geometry overlaps. Grounded against the tree:

1. `segment_path_preserves_requested_endpoints` (inline `#[cfg(test)] mod tests` in `crates/slicer-core/src/lib.rs`) constructs `segment_path(Point2::from_mm(0.0, 0.0), Point2::from_mm(2.0, 0.0), 0.75)` and asserts only first/last endpoints. `crates/slicer-core/tests/geometry_helpers_tdd.rs` already holds stronger segment-path oracles: `segment_path_subdivides_long_segment_with_endpoints_preserved` (exact per-point coordinates), `segment_path_handles_exact_division_short_segment_and_zero_length` (exact/short/zero-length split), and `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` (a roster asserting non-emptiness, endpoints, and the per-chord cap). The inline test is a strict assertion subset; its one distinct asset is the specific (2.0 mm, 0.75 mm) input tuple, which no pre-edit roster case exercises.
2. `point_to_segment_distance_squared_matches` (inline test module in `crates/slicer-core/src/geometry.rs`) builds the fixture `a=(0,0)`, `b=(10_000,0)`, `p=(5000,5000)` and asserts the wrapper's result against the independent `25_000_000.0_f64` oracle at `< 2.0` tolerance — the identical fixture and oracle already asserted by `closest_point_on_segment_midpoint` in the same module, which additionally pins the projected point within 1 unit of `(5000, 0)`. The one non-subsumed surface is the wrapper identity itself: that `point_to_segment_distance_squared` delegates to `closest_point_on_segment` and returns its `distance_sq`.

Both folds are case-preserving merges under the ADR-0064 earn-their-keep standard: deleting either source test leaves a named regression input uncovered only if the merge drops its distinct input or oracle, which this packet forbids. The slice is coherent because both candidates are geometry-adjacent, share the same three-file edit surface, and are the remaining §5.1 items whose disposition is fold/review rather than additive strengthening.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Endpoint witness is exact-unit equality, not the tolerant helper: `Point2` is integer-unit (`i64`) based, so the roster survivor binds `let start = Point2::from_mm(0.0, 0.0); let end = Point2::from_mm(length_mm, 0.0);` and asserts `assert_eq!(points.first(), Some(&start))` / `assert_eq!(points.last(), Some(&end))` — exact unit equality, deliberately NOT routed through the `assert_point2_mm` helper (whose `EPS: f32 = 1.0e-5` mm tolerance cannot carry an exactness witness). The pre-existing tolerant helper stays for other tests; only the survivor's endpoint assertions are exact.
- Preserve both geometry API identities: `point_to_segment_distance_squared` is a thin wrapper over `closest_point_on_segment` returning `cp.distance_sq` (`crates/slicer-core/src/geometry.rs`); the consolidation retains each API's independent numeric oracle and adds the equality witness, never collapsing to equality alone (approved write-gate decision; an equivalence-only oracle is the §2.1 self-referential form that survives a joint regression in both call paths).
- No guest WASM surface is touched (test-only edits in `slicer-core`; `geometry.rs` and `lib.rs` feed no guest), so the `wasm-staleness` snippet does not apply; a failed guest test during verification is attributed per root AGENTS.md's freshness gate, not to this packet.
- Feature-discipline: every `cargo test -p slicer-core` invocation carries `--features host-algos` even though these targets are ungated (census `required: []` for `geometry_helpers_tdd`; the inline modules ride the lib target) — the plan's feature-correct-invocation non-negotiable makes the explicit feature the canonical form.

## Data and Contract Notes

- IR/manifest contracts: none touched — no IR schema, manifest, or config surface appears in the change.
- WIT boundary: none — test-only, host-side.
- Determinism/scheduler constraints: none — the folded tests are pure-function geometry asserts with fixed fixtures.

## Locked Assumptions and Invariants

- The `(2.0 mm, 0.75 mm)` endpoint input tuple is preserved verbatim in the roster survivor (approved write-gate decision; the plan's row #7 requires retaining the distinct endpoint witness unless a case-preserving merge is established — it is established by the roster survivor's superset oracles).
- The wrapper's AND the delegate's independent `25_000_000.0_f64` distance oracles are both retained, alongside the equality witness (approved write-gate decision; equality cannot replace the independent oracles).
- The exact `assert_eq!` endpoint witnesses replace nothing: the tolerant `assert_point2_mm` helper remains for the roster's other uses, and the survivor gains exact integer-unit endpoint equality.
- `flow_correction_stays_positive_for_vertical_input` is not touched by this packet (row #9 ownership).
- No production symbol changes; every behavioral AC is falsifiable by a targeted test run, and only AC-1/AC-N1 make static structural claims.

## Risks and Tradeoffs

- The consolidated test now spans two public APIs in one fn; if a future refactor splits the wrapper from the delegate, the equality witness fails loudly — the intended early warning, with a `// test-quality:` waiver comment naming the deliberate dual-API surface for `check-test-quality` reviewers.
- Census reconciliation is re-derived, never frozen: the implementer re-derives the pre-edit per-target `#[test]` count and the roster tuple count at step start (one grep each) and reconciles the post-edit run against them; no test binary is added or removed, so the binary-count census must not drop.
- The `< 2.0` tolerance on `25_000_000.0` is inherited from the pre-edit tests (rounding of `cp.point` to the unit grid); preserving it, not tightening, avoids re-litigating the float→unit boundary.
