# Implementation Plan: 250-visual-debug-silhouette-gcode-emit

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Promote the rectangular flow-width closed form into slicer-runtime

- Task IDs: `TASK-452`
- Objective: land `pub fn silhouette_flow_width_mm(e_delta_mm, length_mm, slab_height_mm, filament_diameter_mm) -> f64` in `crates/slicer-runtime/src/visual_debug_render.rs` (byte-equivalent math to packet 248's `silhouette_segment_width_mm`), re-export it, and make the pnp-cli fn a one-line delegation.
- Precondition: packet 248 implemented (FORWARD-DEP — its `silhouette_segment_width_mm` and pinning test `flow_width_roundtrip_absolute_and_relative_modes` exist in the tree).
- Postcondition: one closed form in the workspace; 248's gcode-silhouette suite green unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — the `silhouette_segment_width_mm` fn only (locate by symbol)
  - `crates/slicer-runtime/src/visual_debug_render.rs` — the silhouette helper region only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs`; packet dirs 247–249/251; `crates/slicer-gcode/**`
- Expected sub-agent dispatches:
  - Question: landed name/signature of 248's width fn and union helper; scope: `crates/pnp-cli/src/visual_debug_gcode.rs`, `crates/slicer-runtime/src/visual_debug_render.rs`; return: `FACT`
  - Question: run `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd` tee'd to `target/test-output.log`; return: `FACT pass/fail`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D13/D16 rows only (ranged)
- OrcaSlicer refs: none (no parity).
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo check -p slicer-runtime -p pnp-cli --all-targets` — FACT pass/fail
- Exit condition: 248's suite passes with the delegating body; a `rg -c 'PI|consts::PI'`-level duplicate of the formula no longer exists in `visual_debug_gcode.rs`. Falsifying exit: 248's `flow_width_roundtrip_absolute_and_relative_modes` fails → the promoted math is not byte-equivalent; fix the promotion, never the test.

### Step 2: Inversion walk — `gcode_emit_silhouette_segments` (TDD)

- Task IDs: `TASK-452`
- Objective: implement the pub inversion walk over `GCodeIR.commands` (position/`last_e`/tool carry, consecutive-`Some` differencing, travel carry, negative-delta skip with carry update, 3D length, containment-then-nearest bucketing, W4 warning strings) plus its TDD suite (AC-2, AC-3 fixtures, bucketing unit cases).
- Precondition: Step 1 done; packet 247 implemented (its `visual_debug_silhouette_tdd` test file and `SilhouetteSlabSchedule` exist — FORWARD-DEP).
- Postcondition: `gcode_emit_silhouette_segments` + `GcodeEmitSegment` pub, re-exported, unit-pinned; AC-2's emitter round-trip (via `slicer_runtime::DefaultGCodeEmitter` `with_resolved_config`, `filament_diameter = 2.85`) and AC-3's hand-built stream both green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` — the `e_position` accumulation region only
  - `crates/slicer-ir/src/slice_ir.rs` — `GCodeCommand`/`GCodeIR` definitions only
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` — fixture helpers only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`; packet dirs 247–249/251
- Blast-radius discipline: no watched-struct field additions; new fixture literals of `GCodeIR`/`LayerCollectionIR` use `..Default::default()`; `PrintEntity` literals take the existing `// exhaustive:` waiver pattern.
- Expected sub-agent dispatches:
  - Question: run the new tests tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
  - Question: `cargo xtask check-literals` exit code; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — fact 9 + D11 (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: AC-2 and AC-3 test names pass; a deliberately-wrong delta reading (treating `Some(e)` as a delta) is demonstrably caught by AC-2 (the round-trip width would be off by orders of magnitude). Falsifying exit: the emitter round-trip misses `0.5` by more than `1e-3` → re-derive the walk against `emit.rs`, do not widen the tolerance.

### Step 3: Emitter-config fidelity fix in `run_postpass_taps` + fallout sweep

- Task IDs: `TASK-453`
- Objective: add `.with_resolved_config((*ctx.default_resolved_config).clone())` to the `DefaultGCodeEmitter` in `run_postpass_taps`; sweep for tests pinning absolute captured-stream bytes; run the top-down postpass regression suites.
- Precondition: none beyond a compiling tree (independent of Steps 1–2; packet 249's `run_postpass_taps` signature may already differ — adapt in place).
- Postcondition: captured postpass streams reflect the request's resolved config; AC-N4's determinism suite green; any surfaced baseline re-pinned to canonical-correct output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `run_postpass_taps` only (locate by symbol)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - (only if the sweep surfaces a stale self-captured baseline) that one test file
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**`; `crates/slicer-runtime/src/postpass.rs`
- Expected sub-agent dispatches:
  - Question: does any test pin absolute bytes/values of a model-source postpass `typed_capture` or rendered postpass PNG (not two-run comparisons)?; scope: `crates/pnp-cli/tests/`, `crates/slicer-runtime/tests/`; return: `LOCATIONS ≤10`
  - Question: `cargo xtask build-guests --check` exit code (0/1/3), then run `visual_debug_agent_determinism_tdd` tee'd; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D11's resolved-config clause (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: the one-line fix is in; sweep returned and dispositioned; determinism suite green. Falsifying exit: a baseline failure explained as "unrelated" without the guest-freshness check and the LOCATIONS sweep — prohibited; treat as this step's bug.

### Step 4: Renderer entry `render_gcode_emit_silhouette` (TDD)

- Task IDs: `TASK-452`
- Objective: implement the dedicated composite entry (segments → per-(slab, class) unions → rectangles in 249's class order or ascending tool → `Projector`/`Canvas` → PNG + warnings; `MissingGeometryField` on zero rectangles) plus pixel TDD for AC-4/5/6/8/N1.
- Precondition: Step 2 done; packet 249 implemented (role rank order and `RenderStyle` precedents — FORWARD-DEP).
- Postcondition: AC-4 (containment, W4-negative), AC-5 (nearest + W4), AC-6 (tool tracking), AC-8 (determinism), AC-N1 (all-negative fail-closed) green at the renderer level.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — 247/249's composite internals only (locate by symbol)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`; packet dirs 247–249/251
- Expected sub-agent dispatches:
  - Question: run the step's tests tee'd; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D8 GCodeEmit row + §6 W4 + §7 (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: all five AC test names pass; `render_silhouette_composite_styled`'s own suite (249's tests) still green — the frozen entry points are untouched. Falsifying exit: any 249 byte-equivalence test fails → the shared machinery was mutated, not reused; revert to reuse.

### Step 5: Validation lift, schedule plumbing, assembly branch, retirements

- Task IDs: `TASK-453`
- Objective: `SILHOUETTE_TAP_STAGE_IDS` += `"PostPass::GCodeEmit"`; plumb the finalized `(index, z)` schedule out of `run_postpass_taps` and make it the single slab source for both postpass groups; wire the GCodeEmit group to `render_gcode_emit_silhouette` (filenames, `layers_rendered`, tool palette); retire 249's `gcode_emit_silhouette_still_rejected`, add `gcode_emit_silhouette_accepted`, drop the GCodeEmit arm from 247's `silhouette_unsupported_taps_rejected_with_reasons`; author the new bundle test binary (AC-1, AC-7, AC-9).
- Precondition: Steps 3–4 done; packets 247+249 implemented (silhouette branch + `postpass_stage_captures` in the tree — FORWARD-DEP).
- Postcondition: AC-1/7/9 and AC-N2/N3 green end-to-end.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — silhouette branch + `run_postpass_taps` regions only
  - `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs` — wedge harness helpers only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_emit_silhouette_tdd.rs` (new)
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` (consumed via the Step-4 exports only); packet dirs 247–249/251
- Expected sub-agent dispatches:
  - Question: landed shape of 249's schedule construction + grouping key in the silhouette branch; scope: `crates/pnp-cli/src/visual_debug.rs`; return: `SNIPPETS ≤2×30 lines`
  - Question: `cargo xtask build-guests --check` exit code (0/1/3), then run the two edited pnp-cli binaries tee'd; return: `FACT pass/fail`
  - Question: `cargo xtask check-literals` exit code; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D10 + §5 manifest sketch (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — 249's suite must stay green after the schedule refactor
- Exit condition: all three binaries green; `rg -q 'gcode_emit_silhouette_still_rejected' crates/` returns nothing. Falsifying exit: 249's suite reports byte drift from the schedule refactor → the plumbed source is not equivalent to the landed one; reconcile before proceeding.

### Step 6: Docs

- Task IDs: `TASK-454`
- Objective: write the `docs/19_visual_debug.md` GCodeEmit-silhouette paragraph (AC-10's exact anchors `Z-containment` and `testable mainly against itself`), append GCodeEmit to 248's deposited-width/rectangular-model caveats, and note the W4 warning in the warnings inventory.
- Precondition: Step 5 done (the documented behavior exists).
- Postcondition: AC-10 greps pass; no duplicated caveat text.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — silhouette section only (locate by heading)
- Files allowed to edit (at most 3):
  - `docs/19_visual_debug.md`
- Files explicitly out of bounds:
  - `docs/02_ir_schemas.md`; `docs/DEVIATION_LOG.md` (no new deviation rows in this packet)
- Expected sub-agent dispatches: none (single bounded doc edit).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D11's known-cost clause + D16 caveat (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'Z-containment' docs/19_visual_debug.md && rg -q 'testable mainly against itself' docs/19_visual_debug.md && echo PASS` — FACT
- Exit condition: both greps pass and the section names W4. Falsifying exit: either phrase already present before this step wrote it (would make AC-10 vacuous) — verified absent on 2026-08-27; re-verify before writing.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | formula promotion + delegation |
| Step 2 | M | inversion walk + emitter round-trip TDD |
| Step 3 | S | one-line fidelity fix + fallout sweep |
| Step 4 | M | renderer entry + pixel TDD |
| Step 5 | M | assembly, lift, retirements, e2e binary |
| Step 6 | S | docs |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (nearest-slab approximation on nonplanar prints; second-inversion self-testability).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
