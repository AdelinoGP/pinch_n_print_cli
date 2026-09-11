# Design: core-geometry-dup-review

## Controlling Code Paths

- Primary code path: the inline `#[cfg(test)] mod tests` at the tail of `crates/slicer-core/src/lib.rs` (holds `segment_path_preserves_requested_endpoints` and, untouched, `flow_correction_stays_positive_for_vertical_input`) and the inline `#[cfg(test)] mod tests` at the tail of `crates/slicer-core/src/geometry.rs` (holds `closest_point_on_segment_midpoint`, `closest_point_on_segment_degenerate`, `point_to_segment_distance_squared_matches`, `closest_point_on_polygons_none_on_empty`).
- Neighboring tests/fixtures: `crates/slicer-core/tests/geometry_helpers_tdd.rs` — the surviving roster test `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` with its `cases` array and the `segment_lengths_mm`/`assert_point2_mm` helpers (`EPS: f32 = 1.0e-5`).
- OrcaSlicer comparison: none required by this packet — the edited files already carry the standard AGPLv3 porting header (`crates/slicer-core/src/geometry.rs` derives from `src/libslic3r/Point.hpp` / `src/libslic3r/Geometry.hpp`), and the edits port no new C++ code, so the orca-delegation snippet is omitted from `packet.spec.md`/`requirements.md` per the snippet's own rule.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Endpoint witness is exact-unit equality, not the tolerant helper: `Point2` is integer-unit (`i64`) based, so the roster survivor binds `let start = Point2::from_mm(0.0, 0.0); let end = Point2::from_mm(length_mm, 0.0);` and asserts `assert_eq!(points.first(), Some(&start))` / `assert_eq!(points.last(), Some(&end))` — exact unit equality, deliberately NOT routed through the `assert_point2_mm` helper (whose `EPS: f32 = 1.0e-5` mm tolerance cannot carry an exactness witness). The pre-existing tolerant helper stays for other tests; only the survivor's endpoint assertions are exact.
- Preserve both geometry API identities: `point_to_segment_distance_squared` is a thin wrapper over `closest_point_on_segment` returning `cp.distance_sq` (`crates/slicer-core/src/geometry.rs`); the consolidation retains each API's independent numeric oracle and adds the equality witness, never collapsing to equality alone (approved write-gate decision; an equivalence-only oracle is the §2.1 self-referential form that survives a joint regression in both call paths).
- No guest WASM surface is touched (test-only edits in `slicer-core`; `geometry.rs` and `lib.rs` feed no guest), so the `wasm-staleness` snippet does not apply; a failed guest test during verification is attributed per root AGENTS.md's freshness gate, not to this packet.
- Feature-discipline: every `cargo test -p slicer-core` invocation carries `--features host-algos` even though these targets are ungated (census `required: []` for `geometry_helpers_tdd`; the inline modules ride the lib target) — the plan's feature-correct-invocation non-negotiable makes the explicit feature the canonical form.

## Code Change Surface

- Selected approach: two case-preserving folds, each moving the folded test's distinct asset into an existing stronger survivor, then deleting the weaker inline test.
- Exact functions, traits, manifests, tests, and fixtures:
  - `segment_path` (`crates/slicer-core/src/lib.rs`, `pub fn segment_path(start: Point2, end: Point2, max_len_mm: f32) -> Vec<Point2>`): production fn untouched; only its inline test module loses `segment_path_preserves_requested_endpoints` (with the `use super::{flow_correction, segment_path}` import trimmed to `flow_correction` if the fold leaves `segment_path` unused in that mod).
  - `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` (`crates/slicer-core/tests/geometry_helpers_tdd.rs`): extend `cases` to the six-tuple union listed in AC-2, bind `start`/`end`, and assert per iteration: `assert!(!points.is_empty())`; `assert_eq!(points.first(), Some(&start))`; `assert_eq!(points.last(), Some(&end))`; every chord `len <= max_len_mm + EPS`; and every chord `len > 0.0` for every case — all six inputs are nonzero-length, so each adjacent chord is non-degenerate and the chord loop is never vacuous. The array is a plain `(f32, f32)` array, not a watched-type struct literal, so `check-literals` is unaffected.
  - `point_to_segment_distance_squared` and `closest_point_on_segment` (`crates/slicer-core/src/geometry.rs`, both `pub fn`, unchanged signatures): consolidate `point_to_segment_distance_squared_matches` and `closest_point_on_segment_midpoint` into one survivor retaining the name `point_to_segment_distance_squared_matches` with the exact implemented assertions enumerated in AC-3 (fixture, both explicit calls, two independent `25_000_000.0_f64` `< 2.0` oracles, the `cp.point.x`/`cp.point.y` ±1 projection witnesses, and `assert_eq!(dsq, cp.distance_sq)`). `closest_point_on_segment_degenerate` and `closest_point_on_polygons_none_on_empty` remain untouched (distinct degenerate/empty cases).
- Rejected alternatives and reasons:
  - Keep both midpoint tests separate: rejected — identical fixture and oracle make the pair a strict-subset overlap, and the audit's DUP-CORE disposition is FIX (merge preserving union of cases), not KEEP.
  - Replace either numeric oracle with wrapper==delegate only: rejected — the independent `25_000_000.0_f64` oracle is retained per the approved decision; an equivalence-only assertion would be the self-referential oracle of `docs/22_test_quality.md` §2.1 and survives a joint regression in both call paths.
  - Verify the folded survivor by grepping its source for the tuple/assertions: rejected — a source-text match checks the editor buffer, not behavior (`docs/22_test_quality.md` §2.5); behavioral claims are proven by AC-2/AC-3's exact-filter test runs, and static source checks are reserved for the structural retirement claims (AC-1, AC-N1).
  - Merge `closest_point_on_segment_degenerate` into the consolidated test: rejected — its degenerate `a == b` input is a distinct case with its own 3-4-5 oracle, and folding it would couple unrelated regression inputs.
  - Add a new standalone test file for the preserved tuple: rejected — the roster test already asserts the same oracle set; a new file would add a target without adding a case.

## Files in Scope (read + edit)

- `crates/slicer-core/src/lib.rs` - role: fold-1 deletion site (inline test module tail); expected change: remove `segment_path_preserves_requested_endpoints`, trim the now-unused `segment_path` import in the test mod, leave `flow_correction_stays_positive_for_vertical_input` and all production code byte-identical.
- `crates/slicer-core/src/geometry.rs` - role: fold-2 consolidation site (inline test module); expected change: consolidate the two midpoint/distance tests into `point_to_segment_distance_squared_matches` per AC-3; no production change.
- `crates/slicer-core/tests/geometry_helpers_tdd.rs` - role: fold-1 survivor (roster test); expected change: six-tuple union + bound endpoints + exact `assert_eq!` witnesses + non-vacuity + chord cap + nonzero-chord non-degeneracy.

## Read-Only Context

- `crates/slicer-core/src/lib.rs` - the `pub fn segment_path` definition block only - purpose: confirm production semantics (subdivide when `length > max_len_mm`, endpoints always first/last) are untouched by the fold.
- `docs/specs/test-quality-remediation-plan.md` - §5.1 DUP-CORE row, §6 patterns, §7 ledger (`core` row) - purpose: authoritative candidate list and Step 2 ledger target; never load the whole file.
- `docs/specs/test-quality-remediation-census.json` - `slicer-core` entry only - purpose: `geometry_helpers_tdd` → `required: []`; do not parse the whole manifest.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (no new parity surface in this packet).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load (`target/test-output.log` reads are the one exception).
- Unrelated crates - delegate symbol lookups; do not browse.
- `docs/specs/test-quality-remediation-plan.md` Packet Queue table and all `docs/spec_packets/*` directories other than this packet - parent-owned; never edit.
- `crates/slicer-core/src/perimeter_utils.rs`, `crates/slicer-core/src/algos/lightning/layer.rs` - read-only consumers of `closest_point_on_segment`; their call sites are unaffected by the test consolidation (verified: the same-name functions in `modules/core-modules/tree-support-planner/src/lib.rs` and `crates/slicer-core/src/aabb_tree.rs` are separate local helpers, not this symbol).

## Expected Sub-Agent Dispatches

- Question: return the anchored `test result:` lines and `running 1 test` lines from `target/test-output.log` for the AC-2/AC-3/AC-4 runs; scope: `target/test-output.log`; return: `FACT`; purpose: adjudicate without re-running.
- Question: extract the post-edit §7 `core` row verbatim; scope: `docs/specs/test-quality-remediation-plan.md` §7; return: `SNIPPETS` (≤20 lines); purpose: adjudicate AC-5 without a full-file read.

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

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: the Step 1 post-edit log FACTs (AC-2/AC-3 exact counts, AC-4 positive count) — `FACT`, bounded to three greps of `target/test-output.log`.

## Open Questions

- None.