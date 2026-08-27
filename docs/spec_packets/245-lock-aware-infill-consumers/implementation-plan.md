# Implementation Plan: 245-lock-aware-infill-consumers

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: infill-linker locked passthrough + swept-footprint carve

- Task IDs: `TASK-355`
- Objective: make `process_bucket_role` append locked paths verbatim and carve their swept footprint
  out of untagged fill of the same region.
- Precondition: packet 244 (draft) introduces `ExtrusionPath3D.order_lock: Option<u64>`;
  `orchestrate_tdd.rs` compiles against the current linker.
- Postcondition: `AC-1`, `AC-2`, `AC-N1` pass; locked paths are never clipped/linked; untagged fill
  is differenced by the swept footprint (trapezoids + round vertex disks).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/src/orchestrate.rs` - lines `40-360` (RoleBoundaries,
    process_bucket_role, append_paths, link_* helpers)
  - `crates/slicer-runtime/src/visual_debug_render.rs` - lines `639-687` (swept_fill_shape precedent)
  - `crates/slicer-core/src/polygon_ops.rs` - `difference_ex`/`union_ex` signatures
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` - full (fixture helpers)
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/src/orchestrate.rs`
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/visual_debug_render.rs` (read-only precedent)
  - `crates/slicer-ir/**`, `crates/slicer-schema/**`, `crates/slicer-sdk/**`
- Blast-radius discipline: not applicable (no struct field or schema constant added).
- Expected sub-agent dispatches:
  - Question: which `append_paths`/bucket helpers already append verbatim without clipping, and their
    exact signatures? scope: `modules/core-modules/infill-linker/src/orchestrate.rs`; return: `LOCATIONS`
  - Question: does `slicer_core::polygon_ops` expose a circle/round-disk polygon builder, or must the
    linker approximate vertex disks itself? scope: `crates/slicer-core/src/polygon_ops.rs`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - single-caller rule
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p infill-linker --test orchestrate_tdd -- locked_paths_bypass_linking_and_clipping --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p infill-linker --test orchestrate_tdd -- locked_swept_footprint_carved_from_untagged_fill --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p infill-linker --test orchestrate_tdd -- locked_path_crossing_fill_domain_not_clipped --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
- Exit condition: the three named tests pass and the full `orchestrate_tdd` binary is green.

### Step 2: path-optimization-default locked-block candidates

- Task IDs: `TASK-355`
- Objective: coalesce each locked block into one non-reversible nearest-neighbor candidate in
  `group_then_nearest_neighbor`.
- Precondition: Step 1 complete; `path-optimization-default` compiles against packet 244's
  `OrderedEntityView.order_lock`.
- Postcondition: `AC-3`, `AC-N2` pass; a locked block is never split, reversed, or internally
  reordered.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/path-optimization-default/src/lib.rs` - lines `40-260` (nearest_neighbor_permutation, group_then_nearest_neighbor) and `404-700` (inline tests)
- Files allowed to edit (at most 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (host enforcement is packet 244's)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: how does `nearest_neighbor_permutation` consume its `&[&OrderedEntityView]` input (does
    it read `original_index` and a start/end point per entity)? scope: `modules/core-modules/path-optimization-default/src/lib.rs`; return: `SNIPPETS` (≤30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` - wall-subsequence precedent
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p path-optimization-default --lib -- tests::locked_block_is_single_non_reversible_candidate --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p path-optimization-default --lib -- tests::locked_block_never_split_or_reversed --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
- Exit condition: the two named tests pass and the full `--lib` target is green.

### Step 3: G-code emission locked bypass

- Task IDs: `TASK-355`
- Objective: make `DefaultGCodeEmitter::emit_gcode` skip Douglas-Peucker and `min_segment_length`
  pruning for locked paths.
- Precondition: Steps 1-2 complete; `slicer-gcode` compiles against packet 244's carrier.
- Postcondition: `AC-4` passes; every authored point of a locked path is emitted.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - lines `255-600` (emit_gcode simplification + emission loop)
  - `crates/slicer-gcode/src/serialize.rs` - lines `26-58` (tolerance_for_role)
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only; tolerance mapping unchanged)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: none (the simplification site is already located at lines ~504-547).
- Context cost: `S`
- Authoritative docs: none beyond the plan.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- locked_paths_bypass_simplification_and_min_segment --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
- Exit condition: the named test passes and the full `gcode_emit_tdd` binary is green.

### Step 4: structural parity (all-`None` neutrality) + docs

- Task IDs: `TASK-355`
- Objective: prove all-`None` neutrality across the three consumers and land ADR-0063 plus the two
  doc amendments.
- Precondition: Steps 1-3 complete.
- Postcondition: `AC-5` passes; ADR-0063 exists; `docs/02_ir_schemas.md` and `CONTEXT.md` amended.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - lines `596-626` (the invariant to amend)
  - `CONTEXT.md` - lines `263-280` (the Infill entry to amend)
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - Appendix A (ADR draft)
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` (neutrality test)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (neutrality test)
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (neutrality test)
  - `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` (new)
  - `docs/02_ir_schemas.md`
  - `CONTEXT.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (orchestrator owns it)
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` (orchestrator owns it)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: confirm the next-free ADR number is 0063 (0062 is packet 244's). scope: `docs/adr/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - Appendix A (ADR draft, verbatim content)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p infill-linker --test orchestrate_tdd -- all_none_locks_neutrality --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p path-optimization-default --lib -- tests::all_none_locks_neutrality --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `cargo test -p slicer-gcode --test gcode_emit_tdd -- all_none_locks_neutrality --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed"`
  - `rg -q '^# ADR-0063' docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md`
  - `rg -q 'order-lock' docs/02_ir_schemas.md && rg -q 'self-clipping' docs/02_ir_schemas.md`
  - `rg -q 'order-lock' CONTEXT.md`
- Exit condition: all three neutrality tests pass and all three doc greps return matches.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | linker passthrough + carve; two dispatches |
| Step 2 | S | optimizer block coalescing |
| Step 3 | S | emitter bypass |
| Step 4 | S | parity tests + docs |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
