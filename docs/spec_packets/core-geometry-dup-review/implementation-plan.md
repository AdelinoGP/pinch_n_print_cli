# Implementation Plan: core-geometry-dup-review

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every AC command lives solely in `packet.spec.md` (`AC-1`–`AC-5`, `AC-N1`); steps reference them by ID and never restate them. Every `cargo test -p slicer-core` invocation carries `--features host-algos` (plan §6 feature-correct form, applied regardless of the targets being ungated), opens with `set -euo pipefail` and `mkdir -p target`, pipes combined output through `tee target/test-output.log` with console stdout suppressed, and adjudicates from anchored greps of the log — a failing test or unmatched grep exits nonzero, with no `echo FAIL` fallback. Findings are read from `target/test-output.log`, never by re-running.

## Steps

### Step 1: Fold both geometry test-overlap pairs

- Task IDs: `core/DUP-CORE` (segment path); `core/DUP-CORE` (point-to-segment distance)
- Objective: Consolidate `segment_path_preserves_requested_endpoints` into the roster survivor's six-case union with exact endpoint witnesses (AC-2), and `point_to_segment_distance_squared_matches`/`closest_point_on_segment_midpoint` into one dual-API, dual-oracle test (AC-3).
- Precondition: working tree is the post-`core-wall-sequence-dup-merges`-generation tree (that packet is `generated` only; its non-implementation is irrelevant — it exports no symbols and touches none of this packet's three files). Re-derive, do not trust a pinned count: one grep gives the pre-edit per-target `#[test]` counts for `crates/slicer-core/src/geometry.rs`, `crates/slicer-core/src/lib.rs`, and `crates/slicer-core/tests/geometry_helpers_tdd.rs`, and one read of the roster `cases` array gives the pre-edit tuple set — these are the census anchors for AC-4's reconciliation.
- Postcondition: `AC-1`, `AC-2`, `AC-3`, `AC-4`, and `AC-N1` all return PASS; the per-target test-fn census reconciles (no test fn added or removed anywhere except the one consolidation −1 in `geometry.rs` and the one fold −1 in the lib inline module; the roster survivor gains no new fn — the preserved tuple lands inside it), and no test binary is added or removed (binary-count census unchanged).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/lib.rs` - the `pub fn segment_path` block and the inline test mod tail
  - `crates/slicer-core/src/geometry.rs` - inline test mod
  - `crates/slicer-core/tests/geometry_helpers_tdd.rs` - whole file (short)
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 DUP-CORE row, §6 patterns only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/lib.rs`
  - `crates/slicer-core/src/geometry.rs`
  - `crates/slicer-core/tests/geometry_helpers_tdd.rs`
- Files explicitly out of bounds:
  - `docs/specs/test-quality-remediation-plan.md` (Step 2's edit; parent-owned queue rows stay `pending` — this packet does not flip them)
  - All production functions in the three in-scope files (test modules only)
  - `crates/slicer-core/src/perimeter_utils.rs`, `crates/slicer-core/src/algos/lightning/layer.rs`, `crates/slicer-core/src/aabb_tree.rs`, `modules/core-modules/tree-support-planner/src/lib.rs` (same-name helpers are distinct symbols)
  - Every other `docs/spec_packets/*` directory; `OrcaSlicerDocumented/`; `target/` beyond `test-output.log` reads
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: no struct field, schema constant, or version constant changes. The `cases` tuple extension is a plain array literal (not a watched-type struct literal); the only compile-visible change is test-module-internal. No LOCATIONS sweep needed; recorded here so the implementer does not dispatch one.
- Expected sub-agent dispatches:
  - Question: return the anchored `test result:` and `running 1 test` lines from `target/test-output.log` for the AC-2/AC-3/AC-4 runs; scope: `target/test-output.log`; return: `FACT`; purpose: adjudicate without re-running
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 row (candidates), §6 (invocation form) - ranged read
  - `docs/22_test_quality.md` - §1/§4 (survivor map discipline) - ranged read
- OrcaSlicer refs:
  - None - no new parity surface; both edited files already carry the AGPLv3 porting header and no new C++ code is ported.
- Verification: run the pipe-suffixed commands of `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-N1` exactly as written in `packet.spec.md` (behavioral ACs by targeted exact-filter runs; structural ACs by their narrow static checks), plus the packet-level gates `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask check-test-quality --report` (matrix in `requirements.md`). Each returns FACT pass/fail or bounded failure SNIPPETS from `target/test-output.log`.
- Exit condition: AC-1..AC-4 and AC-N1 all green plus all four packet gates clean; an AC-3 failure falsifies the consolidation and reverts to the pre-edit pair pending redesign.

### Step 2: Record the §7 core ledger row for this packet

- Task IDs: `core/DUP-CORE` (segment path; point-to-segment distance)
- Objective: update the current `docs/specs/test-quality-remediation-plan.md` §7 `core` row to a `partial (core-geometry-dup-review)` state recording this packet's retired/consolidated symbols, surviving coverage, validation command, and remaining gap, without overwriting any other row (AC-5).
- Precondition: Step 1's exit condition holds (all FACTs green).
- Postcondition: AC-5 passes against the real plan file; the synthetic controls below (each naming the falsified state it must reject) have each been exercised against the exact checker text extracted from AC-5, proving the checker rejects the reviewer-measured false-positives (done-with-partial-note, missing `closest_point_on_segment_midpoint`, validation without `--features host-algos --test`, gap missing an item) and the prior invalid states before the edit is trusted.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger section only (anchor: `^## 7\. Ledger`, bounded by `^## Packet Queue`)
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md`
- Files explicitly out of bounds:
  - Every ledger row other than the `core` row in `docs/specs/test-quality-remediation-plan.md` (all non-`core` rows are preserved exactly as they stand at execution time — sibling packets may already be recorded as implemented, and this packet must not overwrite or require any particular sibling state)
  - The §"Packet Queue" table and its resume notes (parent-owned; queue updates happen outside this packet and the row remains `pending` until independent preflight PASS)
  - All `docs/spec_packets/*` directories other than this packet
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not triggered: a Markdown table row edit; no code compiles against it.
- Expected sub-agent dispatches:
  - Question: extract the post-edit §7 `core` row verbatim; scope: `docs/specs/test-quality-remediation-plan.md` §7; return: `SNIPPETS` (≤20 lines); purpose: adjudicate AC-5 without a full-file read
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ledger conventions (rows store re-derivable facts, never frozen counts) - ranged read
- OrcaSlicer refs:
  - None - doc-only step.
- Verification: AC-5's pipe-suffixed parser command exactly as written in `packet.spec.md` (sole owner of the ledger criterion), plus synthetic doc-only checks (shell/python, no cargo): the exact AC-5 checker text is extracted from `packet.spec.md` and run against synthetic plan copies in the session temp dir, covering the reviewer-measured false-positives (State `done; partial note`, retired column omitting `closest_point_on_segment_midpoint`, validation missing `--features host-algos --test`, gap missing an item) plus the previously exercised invalid states (empty `open` row despite queue-text slug mentions, wrong-column symbol placement, bare `done`, duplicate `core` row) and one valid updated row.
- Exit condition: AC-5 green on the real file and every synthetic falsified state rejected; a row whose State is `done` (or any non-`partial` grammar) for the `core` wave is invalid by construction (this packet closes only its two §5.1 items).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | both folds + targeted exact-filter verification |
| Step 2 | S | §7 ledger row + synthetic controls |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read. (This packet's slice has no `docs/07` TASK row — the mapping exemption applies; record the ledger via Step 2 instead and do not open `docs/07` beyond the delegated fact-check.)
- Reconcile reopened/superseded status transitions: none — no packet is superseded or reopened by this slice.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the ±2.0 tolerance is inherited, not re-derived (see `design.md` Risks).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile; targeted `cargo test` invocations (AC-2, AC-3, AC-4) match their AC commands exactly and carry no `--all-targets`.