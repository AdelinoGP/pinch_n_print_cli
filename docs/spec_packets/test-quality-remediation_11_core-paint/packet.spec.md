---
status: draft
packet: test-quality-remediation_11_core-paint
task_ids:
  - 'core/PAINT'
backlog_source: docs/specs/test-quality-remediation-plan.md
depends_on:
  - core-retire
context_cost_estimate: S
copy_note: Generated from the approved core-wave queue row #11; plan wave/item IDs are used under the standing mapping exemption, not TASK-### IDs.
---

# Packet Contract: core-paint

## Goal

Repair the eight `host-algos`-gated paint unit tests so every short-circuit, prune, propagation, and probe claim is falsifiable by a real production behavior while keeping all eight names and their distinct intents.

## Scope Boundaries

The test-only edit is limited to the three unit-test modules under `crates/slicer-core/src/algos/paint_segmentation/`: the three `driver_v2_*` short-circuit tests in `mod.rs`, the twin-propagation test plus the three probe tests in `voronoi_graph.rs`, and the single prune test in `voronoi_prune.rs`. A separate implementation step updates only the `core` row in §7 of `docs/specs/test-quality-remediation-plan.md`; production code, default values, other core tests, fixtures on disk, registrations, contracts, and the census manifest are out of scope.

## Prerequisites and Blockers

- Depends on: `core-retire` (row #10, `generated`; test-only and exports no symbols, APIs, or files consumed here).
- Unblocks: `core-brittle` (row #12) packet generation only.
- Activation blockers: none for draft generation; parent owns the independent preflight.

## Acceptance Criteria

- **AC-1. Given** the three `driver_v2_*` short-circuit tests in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (`driver_v2_empty_mesh_returns_input_slice_ir`, `driver_v2_no_paint_data_short_circuits`, `driver_v2_empty_region_map_short_circuits`) calling production `execute_paint_segmentation`, **when** each feature-correct lib substring filter runs, **then** exactly one test executes and passes per filter with nonempty-input guards and short-circuit equality witnesses (input slice/regions/polygons nonempty before the call; result length, region count, and polygon equality pin the unchanged path). Substring filters are used instead of `--exact` because libtest `--exact` requires the full module path and would pin a brittle path. | `set -euo pipefail; mkdir -p target; for t in driver_v2_empty_mesh_returns_input_slice_ir driver_v2_no_paint_data_short_circuits driver_v2_empty_region_map_short_circuits; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || { echo "FAIL: $t"; exit 1; }; done; echo 'AC-1 PASS'`
- **AC-2. Given** `prune_one_arc_preserves_border_nodes` in `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` driving production `remove_nodes_with_one_arc` on the two-node border/interior fixture, **when** the feature-correct lib substring filter runs, **then** exactly one test executes and passes pinning the exact post-state (`deleted == true` for the sole arc, both endpoints' `arc_indices` empty, both node structs surviving), replacing the current no-panic-only ending. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-3. Given** the retained `from_colored_lines_twin_propagation_is_question_mark` (valid square input yields `Ok`) plus the new `from_colored_lines_invalid_input_returns_err_without_panic` driving production `MMU_Graph::from_colored_lines` on a single-segment input whose first coordinate is `2_147_483_648` (`i32::MAX + 1`), which trips the `to_i32` overflow guard before any geometry is built, **when** each feature-correct lib substring filter runs, **then** each executes exactly one test and passes: the valid arm asserts `is_ok` with a nonempty-diagram witness and the invalid arm asserts `Err(MmuGraphError::CoordinateOverflow(2147483648))` with no panic, proving `?` propagation rather than `unwrap`. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib from_colored_lines_twin_propagation_is_question_mark -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib from_colored_lines_invalid_input_returns_err_without_panic -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`
- **AC-4. Given** the three probe tests in `voronoi_graph.rs` (`boostvoronoi_supports_line_segment_sites`, `vertex_color_get_set_round_trip`, `edge_is_primary_callable`) plus the twin test in the same file (eight existing paint tests in total across the three files), **when** each feature-correct lib substring filter runs, **then** each executes exactly one test and passes; the pure third-party builder probe is kept only as a compile witness with a `// test-quality:` waiver in the comment lines immediately above that probe fn (within the 8 lines preceding it) naming the protected build surface (`boostvoronoi`/`with_segments`/compile witness), while the color round-trip (retained `42`/`0` distinction plus original-diagram isolation) and the primary-edge probe keep their meaningful value assertions. No fully-qualified Rust path grep is used as acceptance; the waiver pin below is a leaf comment token, so the name-resolution-equivalence rule is vacuously satisfied. | `set -euo pipefail; mkdir -p target; for t in boostvoronoi_supports_line_segment_sites vertex_color_get_set_round_trip edge_is_primary_callable; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || { echo "FAIL: $t"; exit 1; }; done; python3 -c 'import pathlib; ls=pathlib.Path("crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs").read_text(encoding="utf-8").splitlines(); i=next(n for n,l in enumerate(ls) if "fn boostvoronoi_supports_line_segment_sites" in l); w="\n".join(ls[max(0,i-8):i]); assert "test-quality:" in w and any(k in w for k in ("boostvoronoi","with_segments","compile")), "immediately-associated waiver in the 8 lines above the builder probe fn must name the protected surface"; print("WAIVER-PASS")' || { echo 'FAIL: immediately-associated compile-witness waiver missing'; exit 1; }; echo 'AC-4 PASS'`
- **AC-5. Given** the real `docs/specs/test-quality-remediation-plan.md`, **when** the implementation updates only its current §7 `core` row, **then** this read-only predicate passes only when the heading matches `^## 7\. Ledger\b` and the span ends at the next `^## Packet Queue\b`, the six-column header is exact, exactly one `core` row has state `partial` or `partial (parenthesized detail)`, Retired/changed symbols contains the exact token `core-paint` plus all nine real test names (eight existing plus the new Err test), Surviving/new coverage contains all nine test names plus the oracle tokens `42`, `is_primary`, `deleted`, `is_err`, and `polygons`, Validation contains a whitespace-normalized token-boundary occurrence of `cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes` with optional trailing flags or a closing code-span backtick allowed, plus the four gate tokens `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask check-test-quality --report`, and Remaining gap contains `core-brittle`, `core-cross`, `core-parity`, and `census-target-scope` while no longer containing `core-paint`; no temporary copy, manufactured row, queue mutation, or synthetic positive is used. | `set -euo pipefail; rg -n '^## 7\. Ledger\b' docs/specs/test-quality-remediation-plan.md >/dev/null; python3 -c 'import re; from pathlib import Path as P; t=P("docs/specs/test-quality-remediation-plan.md").read_text(encoding="utf-8"); m=re.search(r"(?ms)^## 7\. Ledger\b[^\n]*\n(.*?)(?=^## Packet Queue\b)",t); assert m,"missing ledger span"; s=m.group(1); h=[[x.strip() for x in l.strip("|").split("|")] for l in s.splitlines() if l.startswith("| Wave |")]; assert len(h)==1 and h[0]==["Wave","State","Retired/changed symbols","Surviving/new coverage","Validation","Remaining gap"]; r=[l for l in s.splitlines() if re.match(r"^\|\s*core\s*\|",l)]; assert len(r)==1; w,st,ch,su,v,g=[x.strip() for x in r[0].strip("|").split("|")]; q=lambda x,y: re.search(r"(?<![A-Za-z0-9_])"+re.escape(x)+r"(?![A-Za-z0-9_])",y); assert w=="core" and re.fullmatch(r"partial(?:\s*\([^()\n]+\))?",st); assert all(q(x,ch) for x in ["core-paint","driver_v2_empty_mesh_returns_input_slice_ir","driver_v2_no_paint_data_short_circuits","driver_v2_empty_region_map_short_circuits","prune_one_arc_preserves_border_nodes","from_colored_lines_twin_propagation_is_question_mark","from_colored_lines_invalid_input_returns_err_without_panic","boostvoronoi_supports_line_segment_sites","vertex_color_get_set_round_trip","edge_is_primary_callable"]); assert all(q(x,su) for x in ["driver_v2_empty_mesh_returns_input_slice_ir","driver_v2_no_paint_data_short_circuits","driver_v2_empty_region_map_short_circuits","prune_one_arc_preserves_border_nodes","from_colored_lines_twin_propagation_is_question_mark","from_colored_lines_invalid_input_returns_err_without_panic","boostvoronoi_supports_line_segment_sites","vertex_color_get_set_round_trip","edge_is_primary_callable"]); assert all(q(x,su) for x in ["42","is_primary","deleted","is_err","polygons"]); vn=re.sub(r"\s+"," ",v).strip(); own=r"(?<![A-Za-z0-9_])cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes(?=$|[\s;&|\x60])"; assert re.search(own,vn),"validation cell missing representative command"; assert "cargo check --workspace --all-targets" in vn; assert "cargo clippy --workspace --all-targets -- -D warnings" in vn, "clippy gate must carry -- -D warnings"; assert "cargo xtask check-literals" in vn; assert "cargo xtask check-test-quality --report" in vn, "quality gate must carry --report"; assert all(q(x,g) for x in ["core-brittle","core-cross","core-parity","census-target-scope"]); assert not q("core-paint",g),"core-paint must no longer be listed as an open gap"; print("core-paint ledger predicate: PASS")'`

## Negative Test Cases

- **AC-N1. Given** the whole `paint_segmentation` module is gated at `crates/slicer-core/src/algos/mod.rs` so all eight existing tests compile to zero under default features, **when** the driver filter runs without `--features host-algos`, **then** it reports `0 passed` (the silent-green hazard), proving the feature flag in AC-1 through AC-4 is mandatory rather than decorative. | `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --lib driver_v2_empty_mesh_returns_input_slice_ir -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 0 passed; 0 failed;' target/test-output.log`

## Verification

AC-1 through AC-4 and AC-N1 are the full acceptance matrix and must all run; AC-5 binds only the ledger row.

- `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 | tee target/core-paint-check.log >/dev/null`
- `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/core-paint-clippy.log >/dev/null`
- `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`

Cargo **test** commands use `set -euo pipefail`, `mkdir -p target`, `tee target/test-output.log`, and anchored result checks. Workspace check/clippy use the same compact-output wrapper with dedicated logs; they do not overwrite the test log. No workspace suite or guest freshness check is required because this packet is host-side test-only with no guest build input.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 `PAINT` row, §6 feature-correct command/log rules, §7 ledger ownership, Packet Queue row #11, and the row-#10-generated resume pointer; read only those sections.
- `docs/22_test_quality.md` - §§1–4; earn-their-keep, independent expectations, non-vacuity (R3), success-only (R4), and legitimate compile-witness weak forms (§3).
- `docs/21_data_defaults_and_fixtures.md` - §§1, 4, 6–7; watched-struct literal rule and the `ResolvedConfig` scanner blind spot for any new fixture literal.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - earn-their-keep burden, survivor map, and named-regression-input standard for the probe review.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode closure obligation for the three touched files.
- `docs/spec_packets/test-quality-remediation_10_core-retire/` - delegated predecessor summary; draft, independently `PREFLIGHT PASS`, exports none.

## Doc Impact Statement (Required)

Specific same-packet doc edit:

- `docs/specs/test-quality-remediation-plan.md` §7 `Ledger` `core` row - implementation records this packet's FIX/KEEP dispositions, all eight surviving plus one new Err-test name, exact oracle tokens, own representative validation command, and the remaining core-wave gap including the `census-target-scope` program-level gap. Verification grep: `rg -n '^## 7\. Ledger\b|^\| core \|' docs/specs/test-quality-remediation-plan.md`; AC-5 is the required real-plan content predicate and must read the accumulated row rather than manufacture a positive copy.

No other documentation section, queue row, or packet is edited by this packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
