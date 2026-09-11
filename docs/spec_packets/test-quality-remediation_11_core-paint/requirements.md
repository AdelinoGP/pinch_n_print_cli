# Requirements: core-paint

## Packet Metadata

- Grouped task IDs: `core/PAINT`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The §5.1 `PAINT` row names eight `host-algos`-gated paint unit tests whose current assertions cannot falsify the production behavior they name: three `driver_v2_*` short-circuit tests assert output equality without proving the input was nonempty enough to matter; `prune_one_arc_preserves_border_nodes` ends in no-panic comments with an explicit MAY on the `deleted` outcome; `from_colored_lines_twin_propagation_is_question_mark` claims `?` propagation from a single valid-input `is_ok`; and three dependency probes range from a pure third-party builder call to a meaningful color round-trip. This packet is the one coherent slice that makes each of the eight falsifiable or records the evidenced reason it stays weak, without touching production code or any other core surface.

## In Scope

- Strengthen the three `driver_v2_*` short-circuit tests in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` with nonempty-input guards and short-circuit equality witnesses through production `execute_paint_segmentation`; test names unchanged.
- Strengthen `prune_one_arc_preserves_border_nodes` in `voronoi_prune.rs` with the exact post-state (`deleted == true`, both endpoints' `arc_indices` empty, both node structs surviving) derived from production `remove_nodes_with_one_arc`; test name unchanged.
- Retain `from_colored_lines_twin_propagation_is_question_mark` for the valid-input `Ok` arm with a nonempty-diagram witness and add `from_colored_lines_invalid_input_returns_err_without_panic`, which feeds a single-segment input with first coordinate `2_147_483_648` and asserts `Err(MmuGraphError::CoordinateOverflow(2147483648))`; both drive the production constructor.
- Earn-their-keep review of the three probes: keep `vertex_color_get_set_round_trip` (meaningful `42`/`0` round-trip plus original-diagram isolation) and `edge_is_primary_callable` with value assertions; keep `boostvoronoi_supports_line_segment_sites` only as a compile witness with a `// test-quality:` waiver naming the protected build surface. The eight existing names survive; one Err-path test is added.
- Update only the §7 `core` Ledger row in `docs/specs/test-quality-remediation-plan.md` with this packet's dispositions, surviving/new names, oracle tokens, representative validation, and remaining gap.

## Out of Scope

- Any production behavior, default, signature, or performance change in `paint_segmentation` or elsewhere.
- `triangle_intersect.rs`, `polygon_ops`, prepass slice, bridge/region/support/wall/geometry/beading tests, and all `crates/slicer-core/tests/*` integration targets.
- New fixture files on disk, new test targets, manifest or feature changes, and the committed census manifest.
- Later queue rows (`core-brittle`, `core-cross`, `core-parity`) and any other packet directory.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1 PAINT row, §6 lib feature-correct pattern, §7 ledger ownership, Queue row #11; small sections, direct read.
- `docs/22_test_quality.md` - §§1–4; counterfactual, independent oracles, R3 vacuity, R4 success-only, §3 legitimate weak forms; delegated SUMMARY (doc over 300 lines, one-section use).
- `docs/21_data_defaults_and_fixtures.md` - §§1, 4, 6–7; watched-struct FRU rule for any new fixture literal; delegated SUMMARY.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - earn-their-keep, survivor map, regression-input rule for probes; delegated SUMMARY.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode closure for touched files; delegated SUMMARY.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`; AC-1 pins the driver triple with nonempty guards, AC-2 pins the exact prune post-state, AC-3 pins both twin arms, AC-4 pins the probe dispositions plus waiver, AC-5 pins the accumulated ledger predicate.
- Negative: `AC-N1` pins the feature-gating guard (bare `--lib` filter yields `0 passed`).
- Cross-packet impact: none; predecessor `core-retire` exports nothing and this packet exports nothing (no new API, file, fixture, or target).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `set -euo pipefail; mkdir -p target; for t in driver_v2_empty_mesh_returns_input_slice_ir driver_v2_no_paint_data_short_circuits driver_v2_empty_region_map_short_circuits; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log \|\| exit 1; done` | AC-1 driver triple | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | AC-2 prune post-state | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib from_colored_lines_twin_propagation_is_question_mark -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib from_colored_lines_invalid_input_returns_err_without_panic -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | AC-3 twin both arms | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; for t in boostvoronoi_supports_line_segment_sites vertex_color_get_set_round_trip edge_is_primary_callable; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log \|\| exit 1; done; python3 -c 'import pathlib; ls=pathlib.Path("crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs").read_text(encoding="utf-8").splitlines(); i=next(n for n,l in enumerate(ls) if "fn boostvoronoi_supports_line_segment_sites" in l); w="\n".join(ls[max(0,i-8):i]); assert "test-quality:" in w and any(k in w for k in ("boostvoronoi","with_segments","compile")); print("WAIVER-PASS")'` | AC-4 probes plus waiver | FACT pass/fail |
| AC-5 python ledger predicate in `packet.spec.md` | ledger accumulation | prints `core-paint ledger predicate: PASS` |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --lib driver_v2_empty_mesh_returns_input_slice_ir -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 0 passed; 0 failed;' target/test-output.log` | AC-N1 gating guard | FACT pass/fail |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report crates/slicer-core/src/algos/paint_segmentation/` | report-mode quality gate, touched scope only | FACT findings list; touched files expect zero unwaived |

Commands produce small parseable output suitable for delegation. Every `cargo test` tees to `target/test-output.log`; check/clippy use dedicated logs and never overwrite it. Findings are read from logs, never by re-running.

## Step Completion Expectations

Order is file-grouped to respect the three-file edit cap: drivers first, then prune plus twin Err discovery, then probes plus waiver, then the ledger row. The twin Err fixture depends only on the `from_colored_lines` Err-site inventory taken in its own step; no step depends on another step's edited assertions. The ledger step runs last and preserves accumulated prior `core` content.

## Context Discipline Notes

Packet-specific hazards: the three source files each exceed 300 lines, so every implementation read uses the stated ±40-line windows or a delegated dispatch; the temptation reads to skip are `crates/slicer-core/tests/*` (different test kind, out of scope) and `OrcaSlicerDocumented/` (no parity question here, never load). Libtest substring filters avoid pinning brittle full module paths. Heavy dispatches return at most the bounded formats in `design.md`.
