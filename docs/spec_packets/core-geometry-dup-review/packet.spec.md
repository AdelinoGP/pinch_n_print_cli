---
status: draft
packet: core-geometry-dup-review
task_ids:
  - core/DUP-CORE (segment path)
  - core/DUP-CORE (point-to-segment distance)
backlog_source: docs/specs/test-quality-remediation-plan.md
context_cost_estimate: S
---

# Packet Contract: core-geometry-dup-review

## Goal

Consolidate the two slicer-core geometry test-overlap pairs — `segment_path_preserves_requested_endpoints` (inline test module of `crates/slicer-core/src/lib.rs`) and `point_to_segment_distance_squared_matches` + `closest_point_on_segment_midpoint` (inline test module of `crates/slicer-core/src/geometry.rs`) — into case-preserving survivors, preserving the distinct (2.0 mm, 0.75 mm) endpoint input, explicit calls to both geometry APIs, and the independent `25_000_000.0_f64` distance oracle.

## Scope Boundaries

This packet edits test code only, in exactly three files: the inline `#[cfg(test)]` modules of `crates/slicer-core/src/lib.rs` and `crates/slicer-core/src/geometry.rs`, and `crates/slicer-core/tests/geometry_helpers_tdd.rs`. Production behavior, signatures, and all other core tests are out of bounds, as are all other queue items (`flow_correction_stays_positive_for_vertical_input` belongs to `core-strengthen`; the beading threshold items belong to `core-beading-threshold-review`). A separate step records the §7 `core` ledger row in `docs/specs/test-quality-remediation-plan.md`.

## Prerequisites and Blockers

- Depends on: `core-wall-sequence-dup-merges` (row #6, `generated`; test-only merge, exports no net-new symbols — generation dependency does not imply implementation).
- Unblocks: `core-beading-threshold-review` (row #8).
- Activation blockers: none at generation time.

## Acceptance Criteria

- **AC-1. Given** the post-edit tree, **when** `crates/slicer-core/src/lib.rs` is searched, **then** the folded inline test `segment_path_preserves_requested_endpoints` is absent (structural retirement only; behavioral proof of the survivor lives in AC-2). | `if rg -q 'segment_path_preserves_requested_endpoints' crates/slicer-core/src/lib.rs; then exit 1; fi`
- **AC-2. Given** the folded survivor `segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases` in `crates/slicer-core/tests/geometry_helpers_tdd.rs`, which iterates the contractual six-case union `(1.0_f32, 0.5_f32)`, `(1.0_f32, 0.75_f32)`, `(2.5_f32, 0.7_f32)`, `(5.0_f32, 2.0_f32)`, `(11.25_f32, 3.0_f32)`, `(2.0_f32, 0.75_f32)` — the last preserving the folded inline test's distinct endpoint input exactly — and per iteration asserts `assert_eq!(points.first(), Some(&start))` and `assert_eq!(points.last(), Some(&end))` against the requested `Point2::from_mm(0.0, 0.0)` / `Point2::from_mm(length_mm, 0.0)`, `assert!(!points.is_empty())`, every chord length `<= max_len_mm + EPS`, and every chord length `> 0.0` for every case (all six inputs are nonzero-length), **when** that single test is run under the feature-correct invocation, **then** exactly 1 test executes and passes. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test geometry_helpers_tdd segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases -- --exact 2>&1 | tee target/test-output.log >/dev/null; grep -qF 'running 1 test' target/test-output.log; grep -qF 'test result: ok. 1 passed; 0 failed;' target/test-output.log`
- **AC-3. Given** the consolidated survivor `point_to_segment_distance_squared_matches` in the inline test module of `crates/slicer-core/src/geometry.rs`, holding fixture `a = Point2 { x: 0, y: 0 }`, `b = Point2 { x: 10_000, y: 0 }`, `p = Point2 { x: 5_000, y: 5_000 }`, **when** the test is run, **then** it passes having asserted: the explicit wrapper call `let dsq = point_to_segment_distance_squared(p, a, b);` and the explicit delegate call `let cp = closest_point_on_segment(p, a, b);`; two INDEPENDENT distance oracles — `assert!((dsq - 25_000_000.0_f64).abs() < 2.0)` and `assert!((cp.distance_sq - 25_000_000.0_f64).abs() < 2.0)` — one per API, neither replaced by the equality witness; the `closest_point_on_segment_midpoint` proximity witnesses `assert!((cp.point.x - 5_000).abs() <= 1)` and `assert!(cp.point.y.abs() <= 1)`; and the wrapper-delegate witness `assert_eq!(dsq, cp.distance_sq)`. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib geometry::tests::point_to_segment_distance_squared_matches -- --exact 2>&1 | tee target/test-output.log >/dev/null; grep -qF 'running 1 test' target/test-output.log; grep -qF 'test result: ok. 1 passed; 0 failed;' target/test-output.log`
- **AC-4. Given** the post-edit `geometry_helpers_tdd` target, **when** the full target runs under the feature-correct invocation, **then** a positive count of tests executes and all pass (actual count read from the log; not a frozen census). | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test geometry_helpers_tdd 2>&1 | tee target/test-output.log >/dev/null; grep -Eq 'test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log`
- **AC-5. Given** `docs/specs/test-quality-remediation-plan.md` after Step 2, **when** its §7 Ledger section is parsed, **then** exactly one `core` wave row exists within that section whose six named columns record: State `partial` (optionally followed by parenthesized detail; never `done` or `open` in any position); Retired/changed symbols naming `segment_path_preserves_requested_endpoints`, `point_to_segment_distance_squared_matches`, AND `closest_point_on_segment_midpoint`; Surviving/new coverage naming `geometry_helpers_tdd` AND the `(2.0, 0.75)` tuple witness; Validation carrying the feature-correct command `cargo test -p slicer-core --features host-algos --test geometry_helpers_tdd` (whitespace-normalized prefix); Remaining gap naming all remaining core-wave items (beading, strengthen, retire, paint, brittle, cross, parity); with every non-`core` ledger row preserved as it stands at execution time (this packet's edit boundary is the `core` row only). | `python3 - <<'PY'
import re, sys
doc = open('docs/specs/test-quality-remediation-plan.md', encoding='utf-8').read()
m = re.search(r'(?ms)^## 7\. Ledger.*?(?=^## Packet Queue)', doc)
assert m, 'section 7 Ledger not found'
rows = [l for l in m.group(0).splitlines() if l.startswith('| core ')]
assert len(rows) == 1, f'exactly one core row required, got {len(rows)}'
cells = [c.strip() for c in rows[0].split('|')[1:-1]]
assert len(cells) == 6, f'six named columns required, got {len(cells)}'
wave, state, retired, surviving, validation, gap = cells
assert wave == 'core', f'wave column must be core, got {wave!r}'
assert re.fullmatch(r'partial(\s*\([^()]*\))?', state), f'state must be partial with optional parenthesized detail, got {state!r}'
for sym in ('segment_path_preserves_requested_endpoints', 'point_to_segment_distance_squared_matches', 'closest_point_on_segment_midpoint'):
    assert sym in retired, f'retired/changed symbol missing: {sym!r}'
assert 'geometry_helpers_tdd' in surviving and '2.0, 0.75' in surviving, f'surviving coverage missing: {surviving!r}'
norm = ' '.join(validation.split())
assert norm.startswith('cargo test -p slicer-core --features host-algos --test geometry_helpers_tdd'), f'validation command must carry the exact feature-correct invocation, got {norm!r}'
for item in ('beading', 'strengthen', 'retire', 'paint', 'brittle', 'cross', 'parity'):
    assert item in gap, f'remaining core-wave item missing from gap: {item!r}'
print('LEDGER-PASS')
PY`
- **AC-N1. Given** the post-edit `crates/slicer-core/src/lib.rs`, **when** it is searched, **then** the row-#9-owned inline test `flow_correction_stays_positive_for_vertical_input` still exists and no `segment_path_*` test function remains in that file's inline test module. | `if rg -q 'fn segment_path_' crates/slicer-core/src/lib.rs; then exit 1; fi; rg -q 'fn flow_correction_stays_positive_for_vertical_input' crates/slicer-core/src/lib.rs`

Command discipline (packet-specific): every `cargo`/shell wrapper above opens with `set -euo pipefail` and `mkdir -p target`, pipes combined output through `tee target/test-output.log` with console stdout suppressed (`>/dev/null`), and adjudicates from the log with anchored greps — `test result: ok. 1 passed; 0 failed;` for exact filters, a positive `[1-9][0-9]* passed; 0 failed` count for full targets. A failing test or unmatched grep aborts with a nonzero exit; no `echo FAIL` fallback exists. A zero-match `--exact` filter is failure, not success. Every core invocation carries `--features host-algos` (plan §6 feature-correct form) even though these targets are ungated (census `required: []` for `geometry_helpers_tdd`; the inline modules ride the lib target). Findings are read from `target/test-output.log`, never by re-running.

## Negative Test Cases

- **AC-N1** is the structural regression guard: absorbing the vertical-flow item or leaving a `segment_path_*` test in `src/lib.rs` fails its negation/existence greps. Given the packet changes no validator, scheduler rule, contract boundary, or error path, no additional rejection-path AC is authored.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- AC-3's command (the fully qualified `--lib` exact filter) is the primary contract proof; AC-2 and AC-4 cover the roster survivor and its target.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 DUP-CORE row (source of both fold candidates), §6 verification patterns (`--features host-algos`, tee to `target/test-output.log`), §7 ledger (AC-5 target); read by section, never loaded whole.
- `docs/22_test_quality.md` - §1 earn-their-keep counterfactual and §4 derivation rule govern the survivor map.
- `docs/21_data_defaults_and_fixtures.md` - §1/§4 FRU/waiver rules for the roster edit (the `cases` array is a plain array, not a watched struct literal; no waiver needed).

## Doc Impact Statement (Required)

- **`docs/specs/test-quality-remediation-plan.md`** §7 Ledger - the current `core` wave row is updated to a `partial (core-geometry-dup-review)` state via Step 2, preserving all prior ledger entries as they stand (earlier sibling packets may already be recorded as implemented; this packet touches only the `core` row). Section-anchor grep: `rg -q '^## 7\. Ledger' docs/specs/test-quality-remediation-plan.md`; content proof is AC-5's scoped parser (never a global slug grep — the Packet Queue already names this packet, so a global grep is vacuously green). No other doc section claims geometry-test structure; no other doc is edited.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).