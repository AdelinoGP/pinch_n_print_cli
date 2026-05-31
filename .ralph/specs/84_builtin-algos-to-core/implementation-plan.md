# Packet 84 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Files allowed to read per step are bounded. The six moved files total ~3 360 LOC; NEVER loaded in full. Section-by-section grep + ±50-line reads only.
- The packet closure gate runs on narrow per-crate tests, NOT `cargo test --workspace` (per the deepening-batch policy; checkpoint packets are P83/P85/P88).
- P83 MUST be closed before this packet starts (Step 0 verifies).
- The `slicer-ir` and `slicer-core` edits trigger guest WASM staleness; Step 8 rebuilds guests and re-verifies `--check` clean before Step 9 runs the SHA parity check.

---

## Step 0 — Verify P83 closure + capture pre-packet baselines

**Objective.** Confirm `slicer-wasm-host` is in place, ADR-0004/0005 are committed, and capture the g-code SHA carried forward from P83.

**Precondition.** P83 is `superseded`. Working tree clean.

**Postcondition.** Baselines in the log: g-code SHA, `slicer-runtime` test count, `slicer-core` test count, `slicer-ir` test count.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `test -f crates/slicer-wasm-host/Cargo.toml && test ! -f crates/slicer-runtime/src/wit_host.rs && test -f docs/adr/0004-runner-traits-in-slicer-wasm-host.md`. Return FACT pass/fail.
- Dispatch: g-code SHA. `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p84-baseline.gcode && sha256sum /tmp/p84-baseline.gcode`. Return FACT `<hex>`.
- Dispatch: pre-packet test counts. `cargo test -p slicer-core -p slicer-ir -p slicer-runtime 2>&1 | tail -10`. Return SNIPPET.

**Context cost: S.**

**Narrow verification.** All three returns positive.

**Falsifying check / exit condition.** P83 verification fails → abort.

---

## Step 1 — Enumerate algorithm/glue boundaries and consumer sites

**Objective.** Build the precise lists of edit sites.

**Precondition.** Step 0 green.

**Postcondition.** Three lists in the log:
- (a) Per-file algorithm body line range vs glue (`BuiltinProducer` static, `commit_*` fn, `Blackboard::*` calls).
- (b) Test files under `crates/slicer-runtime/tests/` referencing the six modules.
- (c) Non-moving runtime files importing from the six modules.
- (d) `BuiltinProducer` trait signature from `dag.rs`.

**Files allowed to read.** None directly — dispatches.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.** Four dispatches matching design.md §"Expected Sub-Agent Dispatches" #1, #2, #3, #4.

**Context cost: S.**

**Narrow verification.** Four lists populated.

**Falsifying check / exit condition.** If algorithm/glue boundaries are unclear in any file → re-dispatch a SUMMARY for that specific file.

---

## Step 2 — Prework: move `FeedrateConfig` to `slicer-ir`; add regression test

**Objective.** `FeedrateConfig` lives in `slicer-ir`; the two known consumers (`gcode_emit.rs` and the soon-to-move `overhang_classifier`) import it from there.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo build --workspace` green (assertion: `overhang_classifier.rs` still in slicer-runtime, but now imports `slicer_ir::FeedrateConfig` instead of `crate::gcode_emit::FeedrateConfig`).

**Files allowed to read.** `crates/slicer-runtime/src/gcode_emit.rs:64-151` (struct + Default impl), `crates/slicer-ir/src/lib.rs` (re-export block).
**Files allowed to edit.**
1. `crates/slicer-ir/src/feedrate.rs` — CREATE (copy the struct + Default impl).
2. `crates/slicer-ir/src/lib.rs` — add `pub mod feedrate;` + re-export.
3. `crates/slicer-ir/tests/feedrate_default_tdd.rs` — CREATE the regression test.
4. `crates/slicer-runtime/src/gcode_emit.rs` — delete the struct (L64-151); add `use slicer_ir::FeedrateConfig;`.
5. `crates/slicer-runtime/src/overhang_classifier.rs` — change `use crate::gcode_emit::FeedrateConfig;` to `use slicer_ir::FeedrateConfig;`.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Build green; `[ $(rg -l 'pub struct FeedrateConfig' crates/ | wc -l) -eq 1 ]` and the matching file is under `crates/slicer-ir/`.

**Falsifying check / exit condition.** Build fails referencing `FeedrateConfig` → check imports in `gcode_emit.rs` and `overhang_classifier.rs`.

---

## Step 3 — Scaffold `slicer-core/src/algos/` and `slicer-runtime/src/builtins/`

**Objective.** Empty module skeletons exist; workspace still builds.

**Precondition.** Step 2 complete.

**Postcondition.** `crates/slicer-core/src/algos/mod.rs` and `crates/slicer-runtime/src/builtins/mod.rs` exist with empty module declarations. `cargo build --workspace` green (the old files in `slicer-runtime/src/` still exist; the new dirs are unused stubs).

**Files allowed to read.** `crates/slicer-core/src/lib.rs`, `crates/slicer-runtime/src/lib.rs`.
**Files allowed to edit.**
1. `crates/slicer-core/src/algos/mod.rs` — CREATE with the six `pub mod` declarations + empty per-algo files (each contains a single `// placeholder` comment).
2. `crates/slicer-core/src/lib.rs` — add `pub mod algos;`.
3. `crates/slicer-runtime/src/builtins/mod.rs` — CREATE with the per-producer `pub mod` declarations + empty per-producer files.
4. `crates/slicer-runtime/src/lib.rs` — add `pub mod builtins;`. **Do NOT yet delete the six `pub mod <algo>;` declarations** — the old files still exist.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Build green.

**Falsifying check / exit condition.** Build fails → likely a module-path typo in the new mod.rs files.

---

## Step 4 — Bulk move: six algorithms to `slicer-core/src/algos/`, six wrappers to `slicer-runtime/src/builtins/`

**Objective.** The bulk move. After this step, the six old files are deleted, the new `slicer-core` algo files hold the verbatim algorithm bodies, the new `slicer-runtime/builtins/` files hold the `*_PRODUCER` statics + commit glue.

**Precondition.** Step 3 complete.

**Postcondition.** `cargo build --workspace` green (after `lib.rs` is updated to no longer declare the deleted files — done in Step 5).

**Files allowed to read.** The six moved files, line-range only per Step 1 dispatch #1.
**Files allowed to edit.**
1. The six `crates/slicer-core/src/algos/<algo>.rs` files — populate with the algorithm bodies (from the corresponding `slicer-runtime/src/<algo>.rs`).
2. The six `crates/slicer-runtime/src/builtins/<algo>_producer.rs` files — populate with the `*_PRODUCER` static + `BuiltinProducer` impl that delegates to `slicer_core::algos::<algo>::execute_*` and commits via `Blackboard::*`.
3. Delete the six `crates/slicer-runtime/src/<algo>.rs` files.

For each file, the split is:
- Move into `slicer-core`: all `pub fn execute_*`, `pub fn classify_*`, error enum (e.g., `MeshAnalysisError`), and any pure-algorithm helpers.
- Keep in `slicer-runtime/builtins/`: the `*_PRODUCER: BuiltinProducer` static, the `impl BuiltinProducer` body, the `fn commit_*_builtin` (if it existed), and the `use slicer_core::algos::<algo>::*;` import.

**Expected sub-agent dispatches.** None during the move itself — the verification dispatches come in Step 5.

**Context cost: M.**

**Narrow verification.** All six `crates/slicer-runtime/src/<algo>.rs` files are deleted. All six `crates/slicer-core/src/algos/<algo>.rs` files exist and grep matches the `pub fn execute_*` (or `classify_layers`) entry per file.

**Falsifying check / exit condition.** A `pub(crate)` helper in the old file is referenced from outside → re-dispatch Step 1 #3 with a tighter scope; promote the helper to `pub` in `slicer-core` if it's algorithm-side, or inline it into the wrapper if it's glue-side.

---

## Step 5 — Rewire `slicer-runtime/src/lib.rs` + `gcode_emit.rs` import; update `runtime_builtins()`

**Objective.** `slicer-runtime` compiles against the new module structure.

**Precondition.** Step 4 complete.

**Postcondition.** `cargo build --workspace` green. `cargo clippy --workspace --all-targets -- -D warnings` green.

**Files allowed to read.** `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/gcode_emit.rs`.
**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — delete the 6 `pub mod <algo>;` declarations; delete or update the corresponding `pub use ...;` re-export lines; update `runtime_builtins()` to reference the new `builtins::*_producer::*_PRODUCER` paths.
2. `crates/slicer-runtime/src/gcode_emit.rs` — `use crate::overhang_classifier::classify_layers;` → `use slicer_core::classify_layers;` (or whatever the chosen `algos::` path is). FeedrateConfig import was rewired in Step 2.
3. Any non-moving runtime file surfaced by Step 1 dispatch #3 — update its `use crate::<algo>::*` paths to the new locations.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Both green. `runtime_builtins()` returns the same count of producers (>= 7 entries per AC-3).

**Falsifying check / exit condition.** Build fails → likely a missed `use crate::<algo>::*` in a non-moving file; consult Step 1 dispatch #3.

---

## Step 6 — Migrate / create per-algorithm tests in `slicer-core/tests/`

**Objective.** Six per-algorithm unit tests pass in `slicer-core`.

**Precondition.** Step 5 complete.

**Postcondition.** `cargo test -p slicer-core` green. At least six test files exist under `crates/slicer-core/tests/`. No test imports `slicer_runtime::Blackboard` or `slicer_runtime::*Producer`.

**Files allowed to read.** Test files surfaced in Step 1 dispatch #2.
**Files allowed to edit.**
1. `crates/slicer-core/tests/algo_mesh_analysis_tdd.rs` — CREATE or MOVE from `slicer-runtime/tests/`.
2. Five more analogous test files (paint_segmentation, prepass_slice, support_geometry, mesh_segmentation, overhang_classifier).
3. `crates/slicer-runtime/tests/integration/main.rs` and `executor/main.rs` aggregators — drop `mod <test>;` declarations for any moved tests.
4. Non-moved tests that previously used `slicer_runtime::execute_*` paths — rewire to `slicer_core::*`.

For `overhang_classifier`: the new test exercises `classify_layers(&mut layers, &FeedrateConfig::default())` against a small two-layer fixture (one wall layer, one overhanging wall layer with non-zero overhang speeds) and asserts that the second layer's wall points carry expected `overhang_quartile` annotations.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test -p slicer-core`. Return FACT pass/fail + count.

**Context cost: M.**

**Narrow verification.** Tests pass; count ≥ 6 algorithm-specific tests.

**Falsifying check / exit condition.** A test fails on `import slicer_runtime::*` → the test was inadvertently dragged with the algorithm code; rewrite the test to use only IR types.

---

## Step 7 — Per-crate test gates for `slicer-runtime`, `slicer-ir`, `pnp-cli`

**Objective.** No regressions in the host-side suites.

**Precondition.** Step 6 complete.

**Postcondition.** `cargo test -p slicer-ir -p slicer-runtime -p pnp-cli` green. `slicer-runtime` test count delta = -(number of tests moved to `slicer-core`); `slicer-ir` test count delta = +1 (the new `feedrate_default_tdd`).

**Files allowed to read.** None.
**Files allowed to edit.** None (test rewires were Step 6).

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-ir`. Return FACT pass/fail + count.
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count + delta vs Step 0 baseline.
- Dispatch: `cargo test -p pnp-cli`. Return FACT pass/fail + count.

**Context cost: M.**

**Narrow verification.** All three green; deltas match expectations.

**Falsifying check / exit condition.** A test fails referencing a moved type → identify whether the test is the host wrapper's responsibility (rewire to `slicer_runtime::builtins::*`) or the algorithm's (move to `slicer-core/tests/`).

---

## Step 8 — Rebuild guests and confirm `--check` clean

**Objective.** The slicer-ir + slicer-core edits invalidated guests; rebuild.

**Precondition.** Step 7 green.

**Postcondition.** `cargo xtask build-guests --check` reports zero STALE.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo xtask build-guests`. Return FACT pass/fail + duration.
- Dispatch: `cargo xtask build-guests --check`. Return FACT clean/STALE-list.

**Context cost: S.**

**Narrow verification.** Both green.

**Falsifying check / exit condition.** Guest fails to build → likely an unintended change in `slicer-ir/src/feedrate.rs` (e.g., a `serde` derive added that breaks bindgen).

---

## Step 9 — AC-8 g-code SHA parity

**Objective.** Post-packet g-code SHA = Step 0 baseline SHA.

**Precondition.** Step 8 green.

**Postcondition.** SHAs match.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p84.gcode && sha256sum /tmp/benchy-p84.gcode`. Return FACT `<hex>`.

**Context cost: S.**

**Narrow verification.** SHAs match.

**Falsifying check / exit condition.** SHA divergence → bisect by reverting each algorithm move in isolation; the divergent algorithm is likely one where the wrapper's `Blackboard::replace_*` is being called with a slightly different value than the original `commit_*_builtin` produced.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 P83 verification + baselines | S |
| 1 Enumerate boundaries + consumers | S |
| 2 FeedrateConfig prework | S |
| 3 Scaffold algos/ + builtins/ | S |
| 4 Bulk move + wrapper creation | M |
| 5 Rewire lib.rs + runtime_builtins() | M |
| 6 Migrate / create slicer-core tests | M |
| 7 Per-crate test gates | M |
| 8 Guest rebuild + `--check` clean | S |
| 9 g-code SHA parity | S |

Aggregate: **M.** No L step. Total step count: 10.

## Packet Completion Gate

Per the deepening-batch policy, workspace tests do NOT run at P84 close.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests` (rebuild) green, then `cargo xtask build-guests --check` clean.
4. `cargo test -p slicer-core -p slicer-ir -p slicer-runtime -p pnp-cli` — green; counts delta as expected (algorithm tests migrated; FeedrateConfig regression test added).
5. AC-8 post-packet SHA = Step 0 baseline SHA.
6. `region_mapping.rs` still in `crates/slicer-runtime/src/` (AC-N2).

## Acceptance Ceremony

- All 9 ACs (AC-1 .. AC-9) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- Implementation log records: Step 0 baseline SHA, Step 9 post-packet SHA, pre/post test counts per crate, list of moved tests (LOC delta per crate), confirmation that `FeedrateConfig` field set is unchanged.
- `status: draft` → `status: superseded` after gate green AND user confirms closure.
- No new ADR for P84 — the wrapper-keeps-commit pattern is already recorded in ADR-0001.
