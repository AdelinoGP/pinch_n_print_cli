# Requirements: core-strengthen

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE-strengthen (excluding wider-bead)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Independent preflight: `PREFLIGHT PASS` (2026-09-10, S0-S8, AC commands, and Doc Impact); the AC-5 lookahead now accepts a closing code-span backtick so the Validation cell can be written in the table's normal backticked style. Authoring result only; no implementation acceptance gate ran.
- Aggregate context cost: `S`

## Problem Statement

The remaining core weak-oracle cases execute real geometry and flow code but do not independently pin the values their names claim to protect: the bridge case checks only self-determinism, vertical flow checks only sign, triangle crossings check only presence, and overlapping-square booleans check only non-empty output. This is one coherent test-quality slice because four existing weak-oracle groups, covering five named test functions, retain their current names and targets and can be strengthened with independently derived numeric or geometric expectations without changing production behavior. The wider-bead oracle is explicitly owned by `core-flow-consolidation` and is not reopened here.

## In Scope

- Preserve the existing test names, inputs, meaningful existing assertions, and test-target placement in all four source-test surfaces; do not add a test file or target.
- In `crates/slicer-core/tests/bridge_over_infill_tdd.rs`, retain the equal-call determinism assertion in `bridging_angle_is_deterministic` and add a finite numeric assertion for the horizontal-anchor/horizontal-area case: `90.0` degrees within `1e-6`, derived from the perpendicular analytic expectation.
- In `crates/slicer-core/src/lib.rs`, retain `flow_correction_stays_positive_for_vertical_input` and replace the sign-only observation with finite plus exact `1.0` assertions for `(0.0, 0.0, 1.0)`.
- In `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs`, retain both existing crossing tests and assert their analytic endpoint pairs using `Point2::from_mm`: `(5.0,0.0)` with `(2.5,5.0)` for the two-edge case, and `(0.0,0.0)` with `(7.5,5.0)` for the vertex-on-plane case. Accept either endpoint orientation because `Line` direction depends on source triangle order.
- In `crates/slicer-core/tests/polygon_ops_tdd.rs`, retain the overlapping squares and non-empty guards, measure returned geometry with an independent `i128` shoelace-area helper (`abs(contour) - sum(abs(holes))`) and order-independent bounds over returned rings, assert operation-specific areas/bounds/component counts using `Point2::from_mm` for expected bounds, and use the existing `shape_signature` helper only for pairwise distinction rather than expected vertex sequences.
- During implementation, update only the `core` row inside the real §7 Ledger span in `docs/specs/test-quality-remediation-plan.md`; preserve accumulated prior evidence, use state `partial`, and record changed, surviving, validation, and remaining-gap evidence in their proper columns.

## Out of Scope

- The wider-bead oracle owned by `core-flow-consolidation`, including any change to its inputs, expectation, or test home.
- Production implementations of `determine_bridging_angle`, `flow_correction`, `triangle_z_intersection`, `Line`, or polygon operations.
- New fixtures, test files, Cargo targets, feature definitions, manifests, public APIs, IR/WIT/schema changes, or generated artifacts.
- Vertex-order or winding-sequence golden assertions for boolean results; exact triangle endpoint orientation is also out of scope.
- Other weak-oracle, retirement, timing, paint, cross-crate, or later core-wave candidates.
- Queue-row bookkeeping in the approved input plan; the parent batch controller owns queue updates. The implementation step owns only the §7 progress row when the tests land.
- OrcaSlicer parity claims or upstream source inspection; all expected values in this packet are self-contained analytic geometry or the existing local behavior contract.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - delegated summary of row #9 approval, plan/item mapping exemption, queue dependency, four approved surfaces, §7 six-column ledger, and predecessor exports.
- `docs/22_test_quality.md` - direct read of §§2.1, 2.5, 2.8, 4, and 5; independent oracles are required, determinism is a limited exception, and source-grep assertions are not behavior tests.
- `docs/21_data_defaults_and_fixtures.md` - direct read of test-only watched-struct literal/FRU and `check-literals` rules.
- `docs/08_coordinate_system.md` - direct read of the Point2 100-nm unit, `Point2::from_mm`, and area conversion rules.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read; each retained test names a regression input that would otherwise slip through.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - direct read; the quality gate remains report-mode until final-wave promotion.
- `docs/spec_packets/core-beading-threshold-review/` - delegated predecessor summary; draft, independently `PREFLIGHT PASS`, with no dependency exports.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`.
- Negative: none; this is test-only behavior strengthening, not a validator, scheduler rule, contract boundary, or enforcement change. Authoring-only synthetic ledger controls are not acceptance evidence and are not emitted as an AC.
- Cross-packet impact: generation depends serially on row #8, but row #8 exports no API, file, fixture, schema, WIT type, manifest entry, or test target. This packet exports none.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd -- --exact bridging_angle_is_deterministic 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | Verify the real bridge integration target and numeric oracle | One anchored result line with one passed and zero failed |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib -- --exact tests::flow_correction_stays_positive_for_vertical_input 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_two_edges 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_one_edge_vertex_on_plane 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | Verify the inline module filters and exact finite/endpoint assertions | Each exact run has one anchored passed/zero-failed result |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | Verify the auto-discovered integration target and numeric boolean geometry oracle | One anchored result line with one passed and zero failed |
| `set -euo pipefail; mkdir -p target; cargo xtask check-literals 2>&1 \| tee target/core-strengthen-literals.log >/dev/null` | Enforce the watched test-struct literal rule after any test helper edits | FACT pass/fail; exit 0 |
| `set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report 2>&1 \| tee target/core-strengthen-quality.log >/dev/null` | Record report-mode findings without pretending the delayed enforcement gate is active | FACT pass/fail; exit 0 |
| AC-5’s real-plan predicate command, reading `docs/specs/test-quality-remediation-plan.md` directly | Verify the real heading-to-queue span, exact six-column header, unique current `core` row, partial grammar, five changed test names, exact surviving oracle tokens, and five later-wave gap tokens; bind Validation anywhere after preserved prior commands to the whitespace-normalized, token-bounded AC-4 representative invocation `cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares`, allowing optional trailing flags or a closing code-span backtick, and rejecting unrelated targets, wrong filters, partial invocations, or missing features | One bounded `core ledger predicate: PASS` line after implementation; an open/current row is expected to fail before implementation |
| `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 \| tee target/core-strengthen-check.log >/dev/null` | Compile all targets after the test-only edits | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/core-strengthen-clippy.log >/dev/null` | Enforce workspace lint cleanliness | FACT pass/fail |

Every Cargo test invocation in this full matrix uses `--features host-algos`, writes combined output to `target/test-output.log` through `tee`, and uses an exact positive filter whose anchored result requires one passed and zero failed. AC-5 records one representative AC-4 command in the ledger only; it does not replace AC-1 through AC-4. No workspace test suite is required by this packet.

## Step Completion Expectations

- Existing test names and target placement remain unchanged across both code steps; no implementation step may silently convert an analytic assertion into a source-text check.
- The polygon helper measures output geometry independently and never compares a result to a locally recomputed boolean operation.
- The ledger step preserves accumulated prior row content and edits no queue row; its Validation proof may follow prior commands and may carry optional trailing flags or a closing code-span backtick, but must contain the whitespace-normalized, token-bounded AC-4 representative command anywhere in that cell. The full AC-1 through AC-4 matrix still runs separately, and closure proves the actual plan row rather than a manufactured copy.

## Context Discipline Notes

Use only the symbol-centered source ranges in `design.md` and the delegated plan/predecessor summaries. Do not load generated artifacts, `target/`, lockfiles, fixtures, or OrcaSlicer sources. Cargo commands are delegated and return only the exact test result or a bounded failure excerpt.
