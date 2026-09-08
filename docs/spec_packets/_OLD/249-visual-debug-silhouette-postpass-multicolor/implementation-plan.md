# Implementation Plan: 249-visual-debug-silhouette-postpass-multicolor

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Step 1 is independent of packet 247. Steps 2–6 require packet 247 **implemented** (they consume `SilhouetteView`, `SilhouetteSlabSchedule`, `render_silhouette_composite`, the silhouette branch, and the 1.2.0 validation surface). Nothing in any step reads or conditions on packet 248's work.

## Steps

### Step 1: Capture shape — `PostpassCaptureShape` + `postpass_stage_captures`

- Task IDs: `TASK-449`
- Objective: extract `run_postpass_taps`'s row-building loop into pub `postpass_stage_captures` with a `PerLayer`/`WholePrint` shape enum; `run_postpass_taps` gains the shape parameter; the sole call site passes `PerLayer` (WholePrint is wired in Step 5).
- Precondition: clean tree; `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd` green.
- Postcondition: AC-1 (both arms) green; the agent-determinism suite green byte-for-byte (the `PerLayer` extraction is a pure refactor).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `run_postpass_taps` and its call site in `run_model_source` (~lines 1100–1460)
  - `crates/slicer-runtime/src/postpass.rs` — `PostPassCapture` definition only (~lines 210–230)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**`; `crates/pnp-cli/src/visual_debug_gcode.rs`; packet dirs 247/248/250/251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or constant changes; `run_postpass_taps`'s signature change has exactly one call site (LOCATIONS-verified 2026-08-27: the `postpass_output` block in `run_model_source`). Re-verify with `rg -n 'run_postpass_taps' crates/pnp-cli/src` before editing.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd` then `--test visual_debug_agent_determinism_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — fact 8 + D10 only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- postpass_whole_print_shape_one_capture_per_tap 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: `WholePrint` over a 3-layer fixture returns 1 capture per tap holding all 3 layers; `PerLayer` returns 3×taps rows each holding the full payload; the determinism suite is green with zero assertion edits.

### Step 2: Renderer — styled entry point, LayerFinalization role arm, schedule gating

- Task IDs: `TASK-450`
- Objective: `render_silhouette_composite_styled` exists (247's fn delegates with `RenderStyle::default()`); the whole-print `CapturedIr::LayerFinalization` arm draws schedule-gated layers with half-width-inflated segment intervals and the pinned role paint order.
- Precondition: **packet 247 implemented**; Step 1 not required.
- Postcondition: AC-3, AC-4, AC-5 green; 247's entire silhouette suite green unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — 247's composite region + `role_color` + `RenderStyle` (~ranged, anchor on symbols)
  - `crates/slicer-ir/src/slice_ir.rs` — `LayerCollectionIR`/`PrintEntity`/`Point3WithWidth`/`ExtrusionRole` definitions only (~lines 2160–2850)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`; `crates/slicer-runtime/src/layer_executor.rs`; `crates/slicer-runtime/src/postpass.rs`
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd`; return: `FACT pass/fail` + failing names
  - Question: `cargo xtask check-literals` exit code (new `LayerCollectionIR`/`PrintEntity` fixtures); return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D8 LayerFinalization row + slab-source note, D2, §7 only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- finalized_layer_slabs_and_half_width_inflation 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- schedule_filter_gates_whole_print_layers 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: pixel assertions pass for slab bottoms (0 then previous z), inflation extents, schedule gating, and role paint order; removing the schedule filter in a scratch build flips AC-4 (falsifiability check, then revert); 247's tests show zero diffs.

### Step 3: Renderer — tool classes and the per-capture `ToolColorUnavailable` contract

- Task IDs: `TASK-450`
- Objective: `ColorBy::Tool` renders per-(layer, tool) unions in ascending tool order with `style.tool_colors`; every non-tool-carrying capture arm fails closed with the existing `RenderError::ToolColorUnavailable`; determinism + default-equivalence pinned.
- Precondition: Step 2 complete.
- Postcondition: AC-6, AC-8 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — the composite region + `ToolColorUnavailable` variant only
  - `crates/slicer-runtime/src/visual_debug_style.rs` — `ColorBy`/`ToolColors` only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`; `crates/slicer-runtime/src/lib.rs` (frozen after Step 2)
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D17 + R6 only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- tool_classes_paint_ascending_tool_index 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- styled_composite_is_deterministic_and_default_equivalent 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: tool overlap paints tool 1 over tool 0; a `CapturedIr::Slice` capture under tool mode returns `ToolColorUnavailable` naming the tap; repeat renders byte-identical; the default-style call equals 247's entry point byte-for-byte.

### Step 4: Validation — whitelist LayerFinalization, lift the blanket tool rejection, retire the 247 and 248 pins

- Task IDs: `TASK-451`
- Objective: `SILHOUETTE_TAP_STAGE_IDS` gains `"PostPass::LayerFinalization"` only; the blanket silhouette `color_by: "tool"` → `InvalidColorBy` rejection is removed; `tool_color_source` R7 rules stay; **both** prior validation-time pins of that rejection are retired in the same step — 247's `silhouette_tool_coloring_rejected_role_accepted` and 248's `model_silhouette_tool_coloring_still_rejected` (if 248 has landed; its absence clause is vacuous until then — queue-order artifact) — replaced by AC-N2's per-capture `ToolColorUnavailable` contract.
- Precondition: packet 247 implemented; Steps 2–3 complete (the render-time contract the lift hands over to must exist).
- Postcondition: AC-N1, AC-N3 green; the retarget's validation half green (the render half lands in Step 5's e2e).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `validate_request` silhouette region + `SILHOUETTE_TAP_STAGE_IDS` only
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` — the silhouette test region only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (frozen after Step 3); `crates/pnp-cli/src/visual_debug_gcode.rs`
- Expected sub-agent dispatches:
  - Question: what does `silhouette_tool_coloring_rejected_role_accepted` assert and on which source?; scope: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`; return: `FACT`
  - Question: run `cargo test -p pnp-cli --test visual_debug_validation_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `S`
- Authoritative docs:
  - `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — `[FWD to packet 249]` bullet only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `! rg -q 'silhouette_tool_coloring_rejected_role_accepted' crates/`
  - `! rg -q 'model_silhouette_tool_coloring_still_rejected' crates/`
- Exit condition: GCodeEmit silhouette still rejects with `SilhouetteUnsupportedForTap`; tool coloring passes validation on a silhouette request; the R7 misuse cases still reject; both old test names are absent and every other 247 (and, if landed, 248) validation pin passes.

### Step 5: Assembly — WholePrint wiring, schedule build, grouping, filenames, palette; end-to-end tests

- Task IDs: `TASK-451`
- Objective: silhouette bundles drive `run_postpass_taps` with `WholePrint`; the postpass group's schedule comes from the capture's finalized z-diffs filtered to selection; grouping key becomes (tap, view, color mode); `_tool` filenames, `color_by`/`tool_color_source` entry fields, `tool_palette` emission, `layers_rendered` = selection ∩ finalized.
- Precondition: Steps 1–4 complete; `cargo xtask build-guests --check` exit 0 (rebuild if 1; never proceed on 3).
- Postcondition: AC-2, AC-7, AC-9, AC-N2, AC-N4 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — 247's silhouette branch + the postpass output block + `tool_palette_entries`/`filament_tool_colors` region
  - `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` — wedge harness helpers only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`; `crates/pnp-cli/src/visual_debug_gcode.rs`; `modules/core-modules/**` sources
- Expected sub-agent dispatches:
  - Question: `cargo xtask build-guests --check` exit code; return: `FACT (0/1/3)`
  - Question: run `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D10, D17, §5 manifest sketch only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- postpass_silhouette_bundle_entry_shape 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- silhouette_tool_on_blackboard_tap_fails_tool_color_unavailable 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: the wedge bundle matches AC-2 field-for-field; role+tool dual specs yield two distinctly-named images with `tool_palette`; the blackboard-tap tool request fails with `ToolColorUnavailable` and writes no bundle content; subset/all-layers framing identical.

### Step 6: Regression sweep + docs/19

- Task IDs: `TASK-451`
- Objective: prove the top-down postpass path byte-stable (AC-10) and document the postpass silhouette, the single whole-print capture, and tool coloring in `docs/19_visual_debug.md`.
- Precondition: Steps 1–5 complete.
- Postcondition: AC-10 and AC-11 green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — full if ≤300 lines post-247/248, else the silhouette section range
- Files allowed to edit (at most 3):
  - `docs/19_visual_debug.md`
- Files explicitly out of bounds:
  - all `crates/**` (code frozen); `docs/07_implementation_status.md` (completion-gate dispatch only)
- Expected sub-agent dispatches:
  - Question: run `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd` and `cargo test -p slicer-runtime --test visual_debug_postpass_tap_tdd`; return: `FACT pass/fail` per binary
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D2 occlusion caveat + D17 caveat text only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `rg -q 'single whole-print capture' docs/19_visual_debug.md && rg -q 'tool-colored silhouette' docs/19_visual_debug.md && echo PASS`
- Exit condition: both regression binaries green with zero assertion edits; both doc greps pass and the tool paragraph states the per-tool occlusion caveat where a PNG-reading agent will see it.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Capture-shape refactor + unit pins + byte-stability regression |
| Step 2 | M | Styled entry + LayerFinalization arm + schedule gating |
| Step 3 | S | Tool classes + fail-closed contract |
| Step 4 | S | Validator lift + retarget |
| Step 5 | M | Assembly + wedge e2e |
| Step 6 | S | Regression sweep + docs |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` (add TASK-449..451 rows) through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: none (nothing superseded; 247's `[FWD to packet 249]` obligations discharged).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected: render-time-vs-validation-time tool-incompatibility discovery; postpass execution cost inherent to the tap class).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
