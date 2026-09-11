---
status: draft
packet: test-quality-remediation_09_core-strengthen
task_ids:
  - 'core/DUP-CORE-strengthen (excluding wider-bead)'
backlog_source: docs/specs/test-quality-remediation-plan.md
context_cost_estimate: S
copy_note: Generated from the approved core-wave queue; implementation and queue bookkeeping remain downstream responsibilities.
---

# Packet Contract: core-strengthen

## Goal

Strengthen four existing `slicer-core` weak-oracle groups, covering five named test functions, with independent numeric or analytic runtime assertions while preserving their names, targets, and meaningful existing coverage.

## Scope Boundaries

This packet changes only the existing bridge-angle, vertical-flow, triangle-intersection, and polygon-boolean test groups across four test surfaces, plus the owning `core` progress row in the remediation plan during implementation. It adds no production behavior, fixture, public schema, manifest, or test target, and the wider-bead oracle remains owned by `core-flow-consolidation`.

## Prerequisites and Blockers

- Depends on: `core-beading-threshold-review` (`status: draft`, independently `PREFLIGHT PASS`, and exporting no symbols, files, fixtures, APIs, or test targets); the dependency serializes packet generation only.
- Unblocks: queue row #10 `core-retire` packet generation after this packet is generated and independently reviewed.
- Activation blockers: an explicit activation request is still required; status remains `draft`. Independent preflight passed on 2026-09-10 (S0-S8, AC commands, and Doc Impact all PASS) after the AC-5 lookahead was widened to accept a closing code-span backtick; that is an authoring result, not an implementation acceptance result.

## Acceptance Criteria

- **AC-1. Given** the existing `bridging_angle_is_deterministic` case with a horizontal `(0,0)`–`(10,0)` anchor and horizontal `(0,0)`–`(4,0)` area edge, **when** the exact integration filter runs, **then** it retains the original same-input determinism equality and independently asserts the returned angle is finite and within `1e-6` of the analytic perpendicular value `90.0` degrees. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd -- --exact bridging_angle_is_deterministic 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-2. Given** the existing `flow_correction_stays_positive_for_vertical_input` case with `(dx,dy,dz) = (0.0,0.0,1.0)`, **when** the exact library filter `tests::flow_correction_stays_positive_for_vertical_input` runs, **then** it asserts the returned value is finite and exactly `1.0`, replacing the sign-only observation while preserving the existing test name and vertical-input case. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib -- --exact tests::flow_correction_stays_positive_for_vertical_input 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-3. Given** the existing `triangle_crossing_two_edges` and `triangle_crossing_one_edge_vertex_on_plane` cases, **when** the exact library filters `algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_two_edges` and `algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_one_edge_vertex_on_plane` run, **then** the first asserts the unordered endpoint pair `Point2::from_mm(5.0, 0.0)`/`Point2::from_mm(2.5, 5.0)` for the `z=5.0` crossing, and the second asserts the unordered pair `Point2::from_mm(0.0, 0.0)`/`Point2::from_mm(7.5, 5.0)`; either endpoint orientation is accepted because source vertex order is not a contract. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_two_edges 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_one_edge_vertex_on_plane 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-4. Given** the existing `boolean_ops_produce_expected_presence_for_overlapping_squares` case with `a = [0,10] × [0,10]` mm and `b = [5,15] × [0,10]` mm, **when** the exact polygon integration filter runs, **then** union has one component, area `15_000_000_000` unit², and bbox `(0,0)`–`(15,10)` mm; intersection has one component, area `5_000_000_000` unit², and bbox `(5,0)`–`(10,10)` mm; difference has one component, area `5_000_000_000` unit², and bbox `(0,0)`–`(5,10)` mm; XOR has two components, area `10_000_000_000` unit², and bbox `(0,0)`–`(15,10)` mm; and the existing `shape_signature` helper reports every pair of operation results distinct without pinning any expected vertex order. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-5. Given** the real `docs/specs/test-quality-remediation-plan.md`, **when** the implementation updates only its current §7 `core` row, **then** this read-only predicate passes only when the real heading matches `^## 7\. Ledger\b[^\n]*$` and ends at the next `^## Packet Queue\b`, the ledger header is exactly `Wave | State | Retired/changed symbols | Surviving/new coverage | Validation | Remaining gap`, exactly one `core` row has state `partial` or `partial (parenthesized detail)`, Retired/changed symbols contains the exact token `core-strengthen` and all five real test names, Surviving/new coverage contains all five test names plus the exact oracle tokens `90.0`, `1.0`, `(5.0,0.0)`, `(2.5,5.0)`, `(0.0,0.0)`, `(7.5,5.0)`, `15_000_000_000`, `5_000_000_000`, `10_000_000_000`, `(0,0)-(15,10)`, `(5,0)-(10,10)`, `(0,0)-(5,10)`, `components=1`, `components=2`, and `shape_signature`, and Validation contains, anywhere after accumulated prior commands, a whitespace-normalized token-boundary occurrence of this exact representative invocation: `cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares`; optional trailing flags or a closing markdown code-span backtick are allowed after the exact filter, so the Validation cell may be written in the table's normal backticked style, while unrelated targets, wrong filters, partial invocations, or missing `--features host-algos` fail; AC-1 through AC-4 remain the full acceptance-test matrix and this representative command is only the ledger proof; Remaining gap contains `core-retire`, `core-paint`, `core-brittle`, `core-cross`, and `core-parity`; no temporary copy, manufactured row, queue mutation, or synthetic positive is used by this acceptance command. | `set -euo pipefail; rg -n '^## 7\. Ledger\b' docs/specs/test-quality-remediation-plan.md >/dev/null; python3 -c 'import re; from pathlib import Path as P; t=P("docs/specs/test-quality-remediation-plan.md").read_text(encoding="utf-8"); m=re.search(r"(?ms)^## 7\. Ledger\b[^\n]*\n(.*?)(?=^## Packet Queue\b)",t); assert m,"missing ledger span"; s=m.group(1); h=[[x.strip() for x in l.strip("|").split("|")] for l in s.splitlines() if l.startswith("| Wave |")]; assert len(h)==1 and h[0]==["Wave","State","Retired/changed symbols","Surviving/new coverage","Validation","Remaining gap"]; r=[l for l in s.splitlines() if re.match(r"^\|\s*core\s*\|",l)]; assert len(r)==1; w,st,ch,su,v,g=[x.strip() for x in r[0].strip("|").split("|")]; q=lambda x,y: re.search(r"(?<![A-Za-z0-9_])"+re.escape(x)+r"(?![A-Za-z0-9_])",y); n="bridging_angle_is_deterministic flow_correction_stays_positive_for_vertical_input triangle_crossing_two_edges triangle_crossing_one_edge_vertex_on_plane boolean_ops_produce_expected_presence_for_overlapping_squares".split(); assert w=="core" and re.fullmatch(r"partial(?:\s*\([^()\n]+\))?",st) and q("core-strengthen",ch) and all(q(x,ch) for x in n); z=re.sub(r"\s+","",su); e="90.0 1.0 (5.0,0.0) (2.5,5.0) (0.0,0.0) (7.5,5.0) 15_000_000_000 5_000_000_000 10_000_000_000 (0,0)-(15,10) (5,0)-(10,10) (0,0)-(5,10) components=1 components=2 shape_signature".split(); vn=re.sub(r"\s+"," ",v).strip(); own=r"(?<![A-Za-z0-9_])cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares(?=$|[\s;&|\x60])"; assert all(q(x,z) for x in n) and all(x in z for x in e) and re.search(own,vn) and all(x in g for x in "core-retire core-paint core-brittle core-cross core-parity".split()); print("core ledger predicate: PASS")'`

## Verification

AC-1 through AC-4 are the full acceptance-test matrix and must all run; AC-5 binds only the ledger’s representative Validation-cell proof to the exact AC-4 invocation.

- `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 | tee target/core-strengthen-check.log >/dev/null`
- `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/core-strengthen-clippy.log >/dev/null`
- `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - delegated summary of the approved queue row, predecessor exports, ledger/queue boundary, and standing plan/item mapping exemption.
- `docs/22_test_quality.md` - direct read of independent-oracle, determinism, source-grep, and test-quality gate requirements.
- `docs/21_data_defaults_and_fixtures.md` - direct read of test literal/fixture discipline and `check-literals` behavior.
- `docs/08_coordinate_system.md` - direct read of Point2 units, mm conversion, and area/coordinate conversion rules.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read of the earn-their-keep regression-input standard.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - direct read of report-mode quality-gate expectations.
- `docs/spec_packets/test-quality-remediation_08_core-beading-threshold-review/` - delegated predecessor summary; draft, independently `PREFLIGHT PASS`, and exports none.

## Doc Impact Statement (Required)

Specific same-packet doc edit:

- `docs/specs/test-quality-remediation-plan.md` §7 `Ledger` `core` row - implementation records this packet’s partial progress, exact surviving oracles, own representative validation command, and remaining core-wave gap. Verification grep: `rg -n '^## 7\. Ledger\b|^\| core \|' docs/specs/test-quality-remediation-plan.md`; AC-5 is the required real-plan content predicate and must read the accumulated row rather than manufacture a positive copy.

No other documentation section, queue row, or packet is edited by this packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
