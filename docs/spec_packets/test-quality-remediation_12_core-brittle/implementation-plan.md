# Implementation Plan: core-brittle

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Keep and harden the footprint timing guard

- Task IDs: `core/BRITTLE`
- Objective: record the earn-their-keep decision for the 1200-facet 3s guard without changing its protection.
- Precondition: the test name resolves only in `crates/slicer-core/src/algos/mesh_analysis.rs`.
- Postcondition: the `footprint.len() == 1200` equality and `from_secs(3)` guard are unchanged and exactly this marker line is added above the test: `// KEEP-review (core-brittle): 118s Benchy regression guard; batched union measured 61ms vs 20.6s incremental (338x); 3s threshold keeps ~50x headroom.`
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mesh_analysis.rs` - lines `890-900`
  - `crates/slicer-core/src/algos/mesh_analysis.rs` - lines `949-985`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
  - `crates/slicer-core/benches/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: run the AC-1 command and report pass/fail; scope: `cargo test -p slicer-core --features host-algos --lib`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 BRITTLE footprint entry plus §6 log discipline
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - delegated SUMMARY for keep burden
- Verification:
  - AC-1 command from `packet.spec.md` - FACT pass/fail
- Exit condition: the lib substring filter prints `1 passed` and the `from_secs(3)` pin succeeds.

### Step 2: Keep and harden the overhang timing guard

- Task IDs: `core/BRITTLE`
- Objective: record the earn-their-keep decision for the 1200-layer 1s guard without changing its protection.
- Precondition: Step 1 filter passes; the test name resolves only in `overhang_annotation.rs`.
- Postcondition: the `result.is_empty()` witness and `from_secs(1)` guard are unchanged and exactly this marker line is added above the test: `// KEEP-review (core-brittle): O(layers) sweep guard; timed region excludes one-time slicing setup; 1s threshold on 1200 pre-sliced layers.`
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines `549-560`
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines `705-745`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
  - `crates/slicer-core/benches/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: run the AC-2 command and report pass/fail; scope: `cargo test -p slicer-core --features host-algos --lib`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 BRITTLE overhang entry
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - delegated SUMMARY for keep burden
- Verification:
  - AC-2 command from `packet.spec.md` - FACT pass/fail
- Exit condition: the lib substring filter prints `1 passed` and the `from_secs(1)` pin succeeds.

### Step 3: Keep and harden the cache ratio guard plus gate closure

- Task IDs: `core/BRITTLE`
- Objective: record the earn-their-keep decision for the 1.5x ratio guard while preserving the inherited 6-name roster (plus the optional retire 7th name per the FORWARD-DEP) and closing the quality gate for the touched scope.
- Precondition: Steps 1–2 filters pass; the ratio test name resolves only in `algo_prepass_slice_tdd.rs`.
- Postcondition: the `cached_elapsed * 3 < uncached_elapsed * 2` guard is unchanged, the file still holds the 6 expected test names in order (optionally followed by the retire 7th name per the FORWARD-DEP), exactly this marker line is added above the test: `// KEEP-review (core-brittle): per-layer footprint-recomputation guard; cached-vs-uncached ratio self-normalizes across machines; 1.5x minimum (measured ~3.5x debug / ~2.3x release).`, and the touched scope shows zero unwaived gate findings.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - lines `1-10`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - lines `395-470`
  - `crates/slicer-core/Cargo.toml` - lines `26-50`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-core/benches/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: run the AC-3 and AC-N1 commands and report pass/fail; scope: cargo only; return: `FACT`
  - Question: confirm zero unwaived `check-test-quality --report` findings for the three touched files; scope: the three paths only; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 BRITTLE cache entry plus §6 gating rules
  - `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - delegated SUMMARY for report-mode closure
  - `docs/spec_packets/test-quality-remediation_10_core-retire/packet.spec.md` - AC-N1 roster source, read-only
- Verification:
  - AC-3 command from `packet.spec.md` - FACT pass/fail
  - AC-N1 command from `packet.spec.md` - FACT pass/fail
  - `cargo xtask check-test-quality --report crates/slicer-core/src/algos/mesh_analysis.rs crates/slicer-core/src/algos/overhang_annotation.rs crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - FACT zero unwaived findings for touched tests
- Exit condition: the exact integration filter prints `1 passed`, the roster python prints `ROSTER-PASS`, the bare-feature control prints `0 passed`, and the ratio pin succeeds.

### Step 4: Record the §7 core ledger row

- Task IDs: `core/BRITTLE`
- Objective: append this packet's KEEP-review dispositions and evidence to the accumulated `core` row without touching the queue or any other row.
- Precondition: Steps 1–3 filters pass; the §7 heading and single `core` row exist.
- Postcondition: the `core` row state is `partial`, its cells carry the packet token, all three guard names, oracle tokens, representative validation plus four gates, and the remaining gap lists `core-cross`, `core-parity`, `census-target-scope` without `core-brittle`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger span only
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
  - `docs/specs/test-quality-remediation-plan.md` ## Packet Queue span
- Expected sub-agent dispatches:
  - Question: run the AC-4 python predicate and report its verdict; scope: plan file only; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ownership plus Queue row #12 boundary
- Verification:
  - AC-4 python predicate from `packet.spec.md` - prints `core-brittle ledger predicate: PASS`
- Exit condition: the predicate prints PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one lib guard, comment-only |
| Step 2 | S | one lib guard, comment-only |
| Step 3 | S | one integration guard plus roster and gate |
| Step 4 | S | one ledger row append |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile. `cargo test` invocations use the narrowest target selector instead (`--lib` with a substring filter, or `--test <file>`), which is mutually exclusive with `--all-targets`.
