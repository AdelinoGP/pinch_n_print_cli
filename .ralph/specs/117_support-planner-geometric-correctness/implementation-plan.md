# Implementation Plan: support-planner-geometric-correctness

## Execution Rules

- One atomic step at a time.
- Each step maps back to a grouped task ID (`TASK-254` tip cone, `TASK-255` offset replacement).
- TDD first for both fixes: AC tests are authored RED before the implementation lands.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: Confirm OrcaSlicer formula + polygon_ops::offset signature via dispatches

- Task IDs: `TASK-254`, `TASK-255`
- Objective: confirm the exact two-piece formula being ported and the exact `polygon_ops::offset` call shape.
- Precondition: spec doc and `polygon_ops.rs` available at HEAD.
- Postcondition: implementer has SUMMARY of Orca formula + SNIPPETS of `polygon_ops::offset` signature; both documented in working notes.
- Files allowed to read:
  - `docs/specs/support-modules-orca-port.md` §B5, §B6 directly (≤ 30 lines combined)
  - `docs/08_coordinate_system.md` directly (≤ 30 lines)
- Files allowed to edit (≤ 3): none in this step.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — delegate, never load
  - `crates/slicer-core/src/polygon_ops.rs` lines outside 195-235 — delegate, do not browse the full file
  - the planner's lib.rs body outside lines 880-940 and around line 226
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicer `TreeSupport::calc_branch_radius` second overload from `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return SUMMARY ≤ 200 words confirming the two-piece formula and the upper clamp" — purpose: confirm B5 formula.
  - "Read `crates/slicer-core/src/polygon_ops.rs` lines 195-235; return SNIPPETS showing `pub fn offset` signature + first 10 lines of body" — purpose: confirm B6 call shape.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B5, §B6
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` — delegate
- Verification:
  - Implementer can recite (a) the two-piece formula with exact branch conditions, (b) the `offset` function signature, (c) the delta unit.
- Exit condition: working notes captured (≤ 10 bullets); next steps unblocked.

### Step 2: Author tapered_radius tip-cone tests as RED

- Task IDs: `TASK-254`
- Objective: create `modules/core-modules/support-planner/tests/tapered_radius_tip_cone.rs` with the five tests (AC-1, AC-2, AC-3, AC-4, AC-N1). All MUST fail RED before Step 3 lands the implementation.
- Precondition: Step 1 complete; formula confirmed.
- Postcondition: file exists; five tests compile; AC-1, AC-2, AC-N1 fail (RED) under the current floor-at-`branch_radius` behavior; AC-3 may pass coincidentally (the linear-above branch already returns the same value the floor doesn't reach); AC-4 may pass coincidentally (the upper clamp already exists).
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` lines 880-940 (current `tapered_radius` body)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/tests/tapered_radius_tip_cone.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/support-planner/src/lib.rs` — do not edit; Step 3 owns the implementation
- Expected sub-agent dispatches:
  - "Run `cargo test -p support-planner --test tapered_radius_tip_cone`; return FACT (expected: AC-1, AC-2, AC-N1 fail; AC-3, AC-4 may pass)" — purpose: confirm RED state on the gates that matter.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: same as Step 1 (no new dispatch needed).
- Verification:
  - `cargo test -p support-planner --test tapered_radius_tip_cone` — FACT with at least 3 failures.
- Exit condition: file compiles; AC-1, AC-2, AC-N1 report assertion failures (not compile errors).

### Step 3: Replace tapered_radius body with two-piece formula; migrate parity test

- Task IDs: `TASK-254`
- Objective: turn Step 2's tests GREEN by implementing the two-piece formula. Migrate the existing `radius_tapers_with_distance_to_top` test in `tests/orca_parity_tdd.rs` to match the new tip-cone semantics (re-anchor assertions, or remove the test if its intent is fully covered by the new file).
- Precondition: Step 2 tests RED.
- Postcondition: `tapered_radius` body matches §B5 formula; all five tests in `tapered_radius_tip_cone.rs` GREEN; the migrated parity test GREEN.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` lines 880-940
  - delegated SNIPPETS of `radius_tapers_with_distance_to_top` from `tests/orca_parity_tdd.rs`
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (migration only — re-anchor or remove the one affected test)
- Files explicitly out-of-bounds for this step:
  - `inflate_polygon` and its call site — handled in Step 4 (do not interleave)
  - the new offset test file — handled in Step 5
- Expected sub-agent dispatches:
  - "Run `cargo test -p support-planner --test tapered_radius_tip_cone`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — gate RED→GREEN.
  - "Run `cargo test -p support-planner --test orca_parity_tdd`; return FACT pass/fail" — confirm the migrated test passes.
  - "Run `cargo build -p support-planner`; return FACT pass/fail" — guard against breaking other consumers of `tapered_radius`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B5
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/.../TreeSupport.cpp::calc_branch_radius` — delegate (confirmed in Step 1)
- Verification:
  - `tapered_radius_tip_cone` test file FACT all pass.
  - `tests/orca_parity_tdd.rs` FACT pass (no failing test introduced).
- Exit condition: AC-1, AC-2, AC-3, AC-4, AC-N1 GREEN; full `-p support-planner` test suite GREEN.

### Step 4: Author avoidance offset concave tests as RED

- Task IDs: `TASK-255`
- Objective: create `modules/core-modules/support-planner/tests/avoidance_offset_concave.rs` with AC-6 and AC-7 tests. The tests exercise the planner's call path that builds avoidance polygons; under current `inflate_polygon` semantics they fail (AC-6 by self-intersection detection; AC-7 by missing hole in output). Tests fail RED before Step 5 substitutes `polygon_ops::offset`.
- Precondition: Step 3 complete; the planner compiles and parses.
- Postcondition: file exists; both tests compile; both fail RED.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` around line 226 (existing call site) + around line 901 (existing `inflate_polygon` body) — confirm what to assert about behavior change
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/tests/avoidance_offset_concave.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/support-planner/src/lib.rs` — do not edit; Step 5 owns the implementation
- Expected sub-agent dispatches:
  - "Run `cargo test -p support-planner --test avoidance_offset_concave`; return FACT (expected: both fail)" — confirm RED state.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none additional.
- Verification:
  - `cargo test -p support-planner --test avoidance_offset_concave` — FACT both failures.
- Exit condition: both tests compile and report assertion failures.

### Step 5: Delete inflate_polygon; substitute polygon_ops::offset at the call site

- Task IDs: `TASK-255`
- Objective: turn Step 4's tests GREEN by deleting `inflate_polygon` and calling `slicer_core::polygon_ops::offset` at the prior call site. Update the planner's `Cargo.toml` if it doesn't already declare a path dependency on `slicer-core::polygon_ops` (delegate this check).
- Precondition: Step 4 tests RED.
- Postcondition: AC-5 grep evidence holds; AC-6, AC-7 GREEN.
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` around lines 226 and 901
  - `modules/core-modules/support-planner/Cargo.toml` — confirm `slicer-core` dependency
  - delegated SNIPPETS of `polygon_ops::offset` (from Step 1)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/Cargo.toml` (only if the dependency line needs a feature flag or path adjustment)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/polygon_ops.rs` body outside the signature — do not edit or browse.
- Expected sub-agent dispatches:
  - "Check `modules/core-modules/support-planner/Cargo.toml`; does it already declare `slicer-core` as a dependency? Return FACT yes/no." — purpose: avoid adding a redundant dependency.
  - "Run `cargo build -p support-planner`; return FACT pass/fail" — purpose: confirm compile.
  - "Run `cargo test -p support-planner --test avoidance_offset_concave`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure" — gate RED→GREEN.
  - "Run `! rg -q 'fn inflate_polygon' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'inflate_polygon\\(' modules/core-modules/support-planner/src/lib.rs && rg -q 'slicer_core::polygon_ops::offset' modules/core-modules/support-planner/src/lib.rs`; return FACT pass/fail" — gate AC-5.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §B6
- OrcaSlicer refs: none in this step (Orca Clipper conventions are documented in §B6 directly).
- Verification:
  - AC-5 compound grep FACT pass.
  - `avoidance_offset_concave` test file FACT both GREEN.
  - `cargo build -p support-planner` FACT pass.
- Exit condition: AC-5 PASS; AC-6, AC-7 GREEN; planner compiles.

### Step 6: Guest WASM staleness gate + final packet verification

- Task IDs: `TASK-254`, `TASK-255`
- Objective: confirm guest `.wasm` artifacts caught up; run full packet verification matrix.
- Precondition: Steps 2-5 complete; each prior AC verification PASS.
- Postcondition: every AC command in `packet.spec.md` returns PASS; `cargo xtask build-guests --check` `up to date`.
- Files allowed to read: none beyond prior steps.
- Files allowed to edit (≤ 3): none — verification only.
- Files explicitly out-of-bounds for this step:
  - `target/**`
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <which>`)" — if STALE, dispatch `cargo xtask build-guests`, re-check.
  - "Run AC-1 through AC-7 plus AC-N1 commands sequentially; return FACT (PASS / FAIL list)" — packet-level gate.
  - "Run `cargo clippy -p support-planner --all-targets -- -D warnings`; return FACT pass/fail" — lint gate.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check` FACT `up to date`.
  - Full AC matrix FACT all PASS.
  - `cargo clippy -p support-planner --all-targets -- -D warnings` FACT pass.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery via delegated SUMMARY + small read; no edits. |
| Step 2 | S | Five small RED tests for `tapered_radius`. |
| Step 3 | S | Body rewrite + parity-test migration. |
| Step 4 | S | Two RED tests for offset behavior. |
| Step 5 | S | Function delete + call substitution. |
| Step 6 | S | Verification gate; no edits. |

Aggregate: `S`. No step is L; no step is M.

## Packet Completion Gate

- All six steps complete.
- Every step exit condition met.
- AC-1 through AC-7 + AC-N1 all dispatch FACT PASS.
- `docs/07_implementation_status.md` marks `TASK-254` and `TASK-255` `[x]` (via worker dispatch).
- `cargo xtask build-guests --check` returns `up to date`.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm packet-level gate commands green: `cargo check`, `cargo clippy`, `cargo test -p support-planner`, `cargo xtask build-guests --check`.
- Confirm implementer's peak context usage stayed under 70%.
- Record any packet-local risk before transition; mark `TASK-254` and `TASK-255` `[x]`; transition `packet.spec.md` to `status: implemented`.
