---
status: draft
packet: 90
task_ids: [TASK-240]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 90 — `benchy.stl` → `regression_wedge.stl` Swap

## Goal

Retire `resources/benchy.stl` (11 MB, ~200k triangles) as a test fixture by authoring `resources/regression_wedge.stl` (≤ 50 KB, ~200 triangles, deliberate feature inventory: 40 mm tall body, 45° overhang on one side, 5 mm flat top, 8 mm flat bottom, 10 mm bridge gap on the front face, ≥ 25 × 25 mm ironable top section), migrating every test that currently exercises `benchy.stl` to consume `regression_wedge.stl` (renaming `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs` with function-prefix sweep `benchy_*` → `slice_*` or `wedge_*`), updating the 4 known non-test references (`crates/slicer-runtime/tests/common/slicer_cache.rs:135`, `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`, `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32`), and deleting `resources/benchy.stl` so the workspace test bench wall-clock drops by the dominant multi-minute share that benchy slicing currently consumes.

## Scope Boundaries

This packet is the STL counterpart of packet 89's 3MF migration: it retires a single heavyweight real-mesh fixture (~11 MB) in favor of a small purpose-built mesh that satisfies every assertion class the existing benchy tests carry (22 CLI-SHAPE, 17 SHAPE-DEPENDENT, 3 STRUCTURAL per the roadmap audit). All assertion content is preserved or strengthened; no test is silently weakened or skipped. The `regression_wedge.stl` is authored deterministically (the same authoring procedure produces byte-identical output) and documented at the closure log.

## Prerequisites and Blockers

- Depends on: none. Independent of packet 89; the two retirements can ship in either order.
- Unblocks: nothing structurally — but the wall-clock improvement compounds with packet 89's improvement, so downstream packets (P1a onwards) benefit from running on a faster cold-cache test bench.
- Activation blockers: confirmation that the team has a deterministic STL-authoring procedure (or one is documented in this packet's closure log). The mesh is engineered, not arbitrary.

## Acceptance Criteria

### AC-1 — `resources/regression_wedge.stl` exists, ≤ 50 KB, contains the documented feature inventory

**Given** the migration target,
**When** the resources directory and the file's geometry are inspected,
**Then** `resources/regression_wedge.stl` exists; its byte size is ≤ 50 × 1024 bytes; and a structural inspection (via `pnp_cli` analyze command or a sub-agent geometry summary) confirms it contains: a 40 mm-tall solid, a 45° overhang on one side, a flat top of at least 25 × 25 mm, a flat bottom of at least 25 × 25 mm, a horizontal bridge gap ≥ 10 mm wide on the front face, and a top section large enough for ironing (≥ 25 × 25 mm).

| `test -f resources/regression_wedge.stl && [ $(wc -c < resources/regression_wedge.stl) -le 51200 ]`

### AC-2 — `resources/benchy.stl` is deleted

**Given** the migration,
**When** `resources/` is inspected,
**Then** `benchy.stl` does not exist on disk.

| `test ! -f resources/benchy.stl`

### AC-3 — Zero references to `benchy.stl` survive in the tree

**Given** the deletion in AC-2,
**When** the workspace is grepped (excluding this packet's own files),
**Then** no file under `crates/`, `modules/`, `docs/`, or `.ralph/` mentions `benchy.stl`.

| `rg -n --glob '!.ralph/specs/90_regression-wedge-stl-swap/**' 'benchy\.stl' crates/ modules/ docs/ .ralph/ ; test $? -eq 1`

### AC-4 — `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` is renamed to `slice_end_to_end_tdd.rs`; function-prefix sweep applied; tests pass

**Given** the source file containing 42 tests with `benchy_*` function names,
**When** the file is renamed and the function-name prefix sweep is applied (CLI-SHAPE tests keep generic `slice_*` naming; SHAPE-DEPENDENT tests adopt `wedge_*` naming where the assertion targets a wedge feature),
**Then** the renamed file compiles and every test passes against `resources/regression_wedge.stl`.

| `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

### AC-5 — The 17 SHAPE-DEPENDENT tests assert against wedge features and pass

**Given** the SHAPE-DEPENDENT category (per the roadmap audit: tests asserting markers like `;TYPE:Top surface`, `;TYPE:Bridge`, `;TYPE:Ironing`, retract-pair counts, layer count > 100, etc.),
**When** the migrated tests run against the wedge,
**Then** each marker the wedge has a corresponding feature for is asserted (top surface — flat top, bridge — bridge gap, ironing — ironable top section), and the layer-count assertion is calibrated to the wedge's 40 mm height at the default 0.2 mm layer height (≥ 180 layers, comparable to benchy's > 100).

| `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && rg -nE ';TYPE:Top surface|;TYPE:Bridge|;TYPE:Ironing' crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`

### AC-6 — 4 non-test reference sites updated

**Given** the four known reference sites,
**When** each is edited,
**Then** `crates/slicer-runtime/tests/common/slicer_cache.rs:135`, `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`, and `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32` each reference `resources/regression_wedge.stl` instead of `resources/benchy.stl`, and the respective tests pass.

| `! rg -q 'benchy\.stl' crates/slicer-runtime/tests/common/slicer_cache.rs crates/slicer-model-io/tests/stl_roundtrip_tdd.rs crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs && cargo test -p slicer-model-io --test stl_roundtrip_tdd 2>&1 | tee target/test-output.log && cargo test -p slicer-runtime --test integration live_module_loading 2>&1 | tee -a target/test-output.log && cargo test -p pnp-cli --test slice_instrumentation_fork_tdd 2>&1 | tee -a target/test-output.log`

### AC-7 — Wall-clock improvement measured and recorded

**Given** the swap,
**When** `cargo clean -p slicer-runtime` followed by `time cargo test -p slicer-runtime` runs before and after the swap,
**Then** the wall-clock for the after-swap run is at least 60 seconds shorter than the before-swap run (the roadmap estimates multi-minute; a 60-second floor is conservative). The before/after numbers are recorded in the closure log.

| `cargo clean -p slicer-runtime && /usr/bin/time -f '%e' cargo test -p slicer-runtime 2>&1 | tee target/test-output.log`

### AC-8 — Authoring procedure for `regression_wedge.stl` is documented in the closure log

**Given** the deterministic-fixture requirement,
**When** the wedge is authored,
**Then** the closure log records (a) the source from which the wedge was generated — either a parametric script (preferred) or a CAD export with the exact tool + parameters, (b) the byte SHA-256 of the resulting STL, (c) instructions for any future regeneration. If a future reader cannot reproduce the file from the documentation, the authoring is incomplete.

| Manual check (closure-log review). No automated grep, but the closure log must contain the SHA-256 of the wedge: `sha256sum resources/regression_wedge.stl`.

## Negative Test Cases

### AC-N1 — No silently weakened assertion in migrated tests

**Given** that SHAPE-DEPENDENT migrations could be tempted to relax assertions if the wedge does not produce an exact marker the benchy produced,
**When** the migrated test file is reviewed,
**Then** no test removes a marker assertion without replacing it with an equivalent or stronger one against a wedge feature, and the closure log enumerates every assertion that was rewritten (old → new) with a one-line rationale.

Manual check via closure-log review. The implementer MUST include the assertion-diff in the closure log.

| `git log -p -- crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs | grep -cE '^[+-]\s*assert'`

### AC-N2 — `regression_wedge.stl` is deterministic across regenerations

**Given** the determinism requirement,
**When** the wedge is regenerated using the documented authoring procedure,
**Then** the resulting file byte-for-byte matches the original (same SHA-256). If the procedure is non-deterministic, the documentation must call this out explicitly and pin a canonical SHA-256 that authors verify against.

| Manual check via re-running the authoring procedure and comparing `sha256sum`. No commit-time machine gate.

### AC-N3 — `regression_wedge.stl` ≤ 50 KB

**Given** the storage-reclaim goal,
**When** the file size is measured,
**Then** the byte count is ≤ 51,200 bytes. A larger mesh defeats the purpose; the wedge should be a low-poly engineered mesh, not a high-resolution CAD export.

| `[ $(wc -c < resources/regression_wedge.stl) -le 51200 ]`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log` (42 migrated tests green)
4. `rg -n --glob '!.ralph/specs/90_regression-wedge-stl-swap/**' 'benchy\.stl' crates/ modules/ docs/ .ralph/ ; test $? -eq 1`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b — benchy.stl → regression_wedge.stl swap" (~60 lines; read directly).
- `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` — read only the test bodies, not in full (file may be ≥ 600 lines; range-read by test name).
- `docs/06_e2e_testing.md` (if it exists) — test classification conventions. Range-read or delegate.

## Doc Impact Statement

A list of specific doc sections that this packet modifies:

- `crates/slicer-runtime/tests/common/slicer_cache.rs` line 135 (doc-comment or motivating example mentioning benchy) — `rg -q 'regression_wedge' crates/slicer-runtime/tests/common/slicer_cache.rs && ! rg -q 'benchy\.stl' crates/slicer-runtime/tests/common/slicer_cache.rs`.

No `docs/*.md` changes required — this packet is a test-fixture migration.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None. The fixture migration does not borrow from OrcaSlicer. The wedge is engineered to satisfy pinch_n_print's existing assertion classes, not to mirror any OrcaSlicer test fixture.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
