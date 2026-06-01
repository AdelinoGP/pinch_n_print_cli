# Implementation Plan — Packet 80

## Execution Rules

- This packet hard-depends on packet 79 (`status: implemented`). Step 1 verifies.
- Ordering for each relocation: (a) write destination file, (b) verify destination compiles + tests pass via narrow `cargo test -p <module> --test <new_name>`, (c) delete source file, (d) update aggregator. Reordering breaks the build.
- All cargo invocations delegated; return `FACT: pass/fail + ≤ 5 lines`.
- Narrow tests only at per-step level. The closure gate runs `cargo test -p wipe-tower -p support-planner -p slicer-runtime` — three packages. **Do NOT run `cargo test --workspace`** in this packet; packet 79 already activated the workspace-test escape clause for the bulk migration, and this packet's scope doesn't warrant repeating it.

## Steps

### Step 1 — Preflight: verify packet 79 closed; capture pre-baseline test-function counts

- **Task IDs**: TASK-229
- **Objective**: Confirm packet 79's `status: implemented`; capture pre-packet-80 test-function counts (via grep) for wipe-tower, support-planner, slicer-runtime. Grep counting is deterministic, sub-second, and avoids three heavy cargo runs (cargo test then runs at steps 2/3 narrowly and steps 5/9 broadly — no need for a fourth full sweep here).
- **Precondition**: Packet 79's `packet.spec.md` exists with `status: implemented`.
- **Postcondition**: Three test-function counts recorded in the implementation log.
- **Files to read**: `.ralph/specs/79_core-modules-test-migration-and-builder-extension/packet.spec.md` frontmatter.
- **Files to edit**: none.
- **Expected dispatches**: dispatch 4 (pre-baseline counts via grep).
- **Context cost**: S
- **Narrow verification**: `grep -E '^status:' .ralph/specs/79_core-modules-test-migration-and-builder-extension/packet.spec.md | grep -q implemented && echo "wipe-tower: $(rg -c '^#\[(tokio::)?test\]' modules/core-modules/wipe-tower/tests/ 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')" && echo "support-planner: $(rg -c '^#\[(tokio::)?test\]' modules/core-modules/support-planner/tests/ 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')" && echo "slicer-runtime: $(rg -c '^#\[(tokio::)?test\]' crates/slicer-runtime/tests/ 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')"`
- **Exit condition**: P79 closed; three counts noted (support-planner should be 0 pre-packet-80).

### Step 2 — Relocate `wipe_tower_bed_bounds.rs` (write destination first)

- **Task IDs**: TASK-229
- **Objective**: AC-1 satisfied. The relocated file exists at the new path, compiles, and `cargo test -p wipe-tower --test bed_bounds_tdd` passes.
- **Precondition**: Step 1 complete.
- **Postcondition**: Destination file exists; tests pass at the destination; source file still exists (will be deleted in step 4).
- **Files to read**: `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` (full read, ≈ 200 lines per recon — under the 600-line cap, direct read OK). `modules/core-modules/wipe-tower/Cargo.toml` (confirm dev-dep present from packet 79).
- **Files to edit**:
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (new — verbatim contents with helper bodies rewritten per design.md §Controlling Code Paths)
  - `modules/core-modules/wipe-tower/Cargo.toml` (no-op if dev-dep already present; add if missing)
- **Expected dispatches**: dispatch 1 (assertion snapshot), dispatch 3 (imports + helper bodies).
- **Context cost**: M
- **Narrow verification**: `cargo test -p wipe-tower --test bed_bounds_tdd`
- **Exit condition**: green.

### Step 3 — Relocate `prepass_support_generation_orca_parity_tdd.rs` (write destination first; switch to `#[module_test]`)

- **Task IDs**: TASK-229
- **Objective**: AC-2 + AC-5 satisfied. The relocated file exists at the new path, compiles, and `cargo test -p support-planner` passes. The `#[test]` + manual `install_log_capture` pair is replaced with `#[module_test]`.
- **Precondition**: Step 2 complete.
- **Postcondition**: Destination file exists; tests pass; source file still exists (will be deleted in step 4); `support-planner/Cargo.toml`'s existing empty `[dev-dependencies]` section gains its first entry.
- **Files to read**: `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` (full read, ≈ 550 lines — direct read OK). `modules/core-modules/support-planner/Cargo.toml` (pre-state).
- **Files to edit**:
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (new — verbatim contents with `#[test]` → `#[module_test]` switch and `install_log_capture` removal per design.md §Controlling Code Paths)
  - `modules/core-modules/support-planner/Cargo.toml` (populate the existing empty `[dev-dependencies]` section with `slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }`)
- **Expected dispatches**: dispatch 2 (assertion snapshot), dispatch 3 (imports + helpers).
- **Context cost**: M
- **Narrow verification**: `cargo test -p support-planner`
- **Exit condition**: green. **Note**: this is the first time `cargo test -p support-planner` runs anything (pre-baseline was 0 tests).

### Step 4 — Delete source files; update aggregator

- **Task IDs**: TASK-229
- **Objective**: AC-1 + AC-2 + AC-3 satisfied. Source files gone from runtime; `executor/main.rs` no longer declares them.
- **Precondition**: Steps 2-3 complete (destinations green).
- **Postcondition**: `crates/slicer-runtime/tests/executor/{wipe_tower_bed_bounds,prepass_support_generation_orca_parity_tdd}.rs` don't exist; `crates/slicer-runtime/tests/executor/main.rs` doesn't declare those mods.
- **Files to read**: `crates/slicer-runtime/tests/executor/main.rs` (the lines around 36 and 42 — read a ±5 window before editing).
- **Files to edit**:
  - Delete `crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs`
  - Delete `crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs`
  - `crates/slicer-runtime/tests/executor/main.rs` (remove 2 `mod` lines)
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `! test -f crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs && ! test -f crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs && ! grep -qE '^mod (wipe_tower_bed_bounds|prepass_support_generation_orca_parity_tdd);' crates/slicer-runtime/tests/executor/main.rs && ! rg "use (wipe_tower|support_planner)::" crates/slicer-runtime/tests/ 2>/dev/null && cargo check -p slicer-runtime --tests`
- **Exit condition**: all checks pass (including AC-N1's structural-signal rg); `slicer-runtime` still compiles.

### Step 5 — Verify slicer-runtime regression: no broken tests after the moves

- **Task IDs**: TASK-229
- **Objective**: AC-7 satisfied. `cargo test -p slicer-runtime` passes; pre/post count delta = `-N1 - N2` (where N1, N2 are the test function counts from the two moved files).
- **Precondition**: Step 4 complete.
- **Postcondition**: `slicer-runtime` test sweep green; count delta confirmed.
- **Files to read**: none.
- **Files to edit**: none.
- **Expected dispatches**: dispatch 7 (slicer-runtime regression).
- **Context cost**: M
- **Narrow verification**: `cargo test -p slicer-runtime`
- **Exit condition**: green; delta confirmed.

### Step 6 — Add `NOT RELOCATABLE` annotation to `slicing_promotion_e2e_regression_tdd.rs`

- **Task IDs**: TASK-230
- **Objective**: AC-6 (partial). The file's top 25 lines contain the documented comment.
- **Precondition**: Step 5 complete.
- **Postcondition**: Comment added between existing doc-comment and `#![allow(missing_docs)]`.
- **Files to read**: `crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs` (top 30 lines to confirm structure).
- **Files to edit**: same file (insertion only, no behavior change).
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `head -25 crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs | grep -qE 'NOT RELOCATABLE' && head -25 crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs | grep -qE 'commit_(shell_classification|slice)_builtin|Blackboard'`
- **Exit condition**: grep checks pass.

### Step 7 — Add `NOT RELOCATABLE` annotation to `gcode_part_cooling_emission_tdd.rs`

- **Task IDs**: TASK-230
- **Objective**: AC-6 (partial).
- **Precondition**: Step 6 complete.
- **Postcondition**: Comment added.
- **Files to read**: `crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs` (top 30 lines).
- **Files to edit**: same file.
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `head -25 crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs | grep -qE 'NOT RELOCATABLE' && head -25 crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs | grep -qE 'DefaultGCodeEmitter|DefaultGCodeSerializer'`
- **Exit condition**: grep checks pass.

### Step 8 — Add `NOT RELOCATABLE` annotation to `gcode_skirt_brim_emission_tdd.rs`

- **Task IDs**: TASK-230
- **Objective**: AC-6 fully satisfied.
- **Precondition**: Step 7 complete.
- **Postcondition**: Comment added.
- **Files to read**: `crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs` (top 30 lines).
- **Files to edit**: same file.
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `head -25 crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs | grep -qE 'NOT RELOCATABLE' && head -25 crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs | grep -qE 'DefaultGCodeEmitter|Blackboard'`
- **Exit condition**: grep checks pass.

### Step 9 — Final closure: workspace check + clippy + targeted test sweep + guest staleness

- **Task IDs**: TASK-230
- **Objective**: All closure gates green.
- **Precondition**: Steps 1-8 complete.
- **Postcondition**: Packet ready for ceremony.
- **Files to read / edit**: none.
- **Expected dispatches**: dispatch 5 (wipe-tower test), dispatch 6 (support-planner test), dispatch 7 (slicer-runtime test), dispatch 8 (guest staleness).
- **Context cost**: M (three test sweeps + guest rebuild).
- **Narrow verification (the closure gates)**:
  1. `cargo xtask build-guests --check` (rebuild if STALE)
  2. `cargo check --workspace --all-targets`
  3. `cargo clippy --workspace --all-targets -- -D warnings`
  4. `cargo test -p wipe-tower -p support-planner -p slicer-runtime`
- **Exit condition**: all four gates clean.

## Per-Step Budget Roll-Up

| Step | Cost | Cumulative |
|---|---|---|
| 1 | S | S |
| 2 | M | M |
| 3 | M | L⁻ |
| 4 | S | L⁻ |
| 5 | M | L |
| 6 | S | L |
| 7 | S | L |
| 8 | S | L |
| 9 | M | L |

**Aggregate**: M-L. No single step is L. The packet is small (9 steps; most S); the L aggregate reflects two M-cost test sweeps (steps 5, 9) rather than oversized individual steps. Step 1's pre-baseline uses grep counting (S), not test runs. Single-context completion is feasible without handoff.

## Packet Completion Gate

The four closure gates from Step 9. Run in order; halt at first failure.

## Acceptance Ceremony

After the closure gates pass:

- Update `packet.spec.md` frontmatter: `status: implemented`, add `closed: <ISO date>`.
- Append closure detail to `docs/07_implementation_status.md`: change TASK-229 and TASK-230 from `[ ]` to `[x]`; for each, add `Closed YYYY-MM-DD — packet 80; verified by cargo test -p {wipe-tower, support-planner, slicer-runtime}` suffix.
- Record the test-count deltas in the closure commit message: pre-packet-80 `wipe-tower` count → post (+N), pre `support-planner` count (0) → post (+N), pre `slicer-runtime` count → post (-N -M).
- The 77-80 sequence is complete. End state per the original architectural plan: one canonical `slicer_sdk::test_support` surface, `#[module_test]` honest, `MockHost` adapting `MeshSource`, all 20 core-modules using the shared builders where they fit, every misplaced test relocated, three legitimately-runtime tests annotated.
