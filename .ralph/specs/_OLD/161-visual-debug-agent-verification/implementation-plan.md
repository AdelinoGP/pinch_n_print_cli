# Implementation Plan: 161-visual-debug-agent-verification

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-271`.
- TDD per feature: author the named `*_tdd.rs` assertion first, then implement to green.
- After any edit to `crates/slicer-ir/**` or `crates/slicer-runtime/**`, run
  `cargo xtask build-guests --check` before attributing a guest/host failure.
- Every field below is an independent context-budget contract; capture reads
  committed slots/arena/finalized IR only — add no module/WIT/Blackboard API.

## Steps

### Step 1: Reconcile drifted design and IR docs

- Task IDs: `TASK-271`
- Objective: correct the `docs/specs/visual-pipeline-debug.md` tap inventory (SeamPlanIR `region_key`/`chosen_candidate.point` not `seam_xy`; `RegionPlan.config` as `ConfigId`; mm-unit flags on seam/branch; drop the standalone LayerPlanning row -> overlay note; RegionMapping as a `SliceIR` join) and add a normative `SupportGeometryIR` definition to `docs/02_ir_schemas.md`.
- Precondition: field shapes grounded from `crates/slicer-ir/src/slice_ir.rs`.
- Postcondition: AC-8 passes; downstream steps cite correct field names.
- Files allowed to edit (<=3): `docs/specs/visual-pipeline-debug.md`, `docs/02_ir_schemas.md`.
- Out of bounds: all code; other docs.
- Dispatch: exact `SupportGeometryIR`/`SupportGeometryKey` shape — `SNIPPETS` <=1, 30 lines.
- Context cost: `M`
- Verification: the AC-8 `python3` check - FACT pass/fail.
- Exit: docs assert the corrected fields and the `SupportGeometryIR` definition.

### Step 2: Packet-160 cleanup and TravelMove doc fix

- Task IDs: `TASK-271`
- Objective: remove the stale "not yet wired" header and blanket `#![allow(dead_code)]` from `visual_debug_gcode.rs` (:32-37); correct the `TravelMove` doc comment in `slice_ir.rs` (:2080) to state millimeters, not "100 nm".
- Postcondition: AC-9 passes; no new clippy dead-code warnings introduced.
- Files allowed to edit (<=3): `crates/pnp-cli/src/visual_debug_gcode.rs`, `crates/slicer-ir/src/slice_ir.rs`.
- Out of bounds: `TravelMove` type/fields (doc comment only); all other code.
- Context cost: `S`
- Verification: the AC-9 `python3` check; `cargo xtask build-guests --check` (slice_ir edit); `cargo clippy -p pnp-cli --all-targets -- -D warnings` - FACT.
- Exit: header/allow gone, `TravelMove` doc says mm, guests fresh, clippy clean.

### Step 3: Blackboard-read capture — SliceIR family

- Task IDs: `TASK-271`
- Objective: add a Blackboard-read capture entry point reading committed slots off `PrepassContext` (`run.rs:636`) via `Blackboard` accessors (`blackboard.rs:141-271`); add `CapturedIr::Slice(SliceIR)` and tap ids for `Layer::Slice`, `PaintSegmentation`, `Layer::PaintRegionAnnotation`/`SlicePostProcess`; wire them in `visual_debug.rs` `run_model_source`.
- Postcondition: these taps capture the `SliceIR` slot with no per-layer arena execution; AC-1 (SliceIR subset) passes.
- Files allowed to edit (<=3): `crates/slicer-runtime/src/layer_executor.rs`, `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`.
- Out of bounds: per-layer arena path; render code; validation.
- Dispatch: exact `PrepassContext`/`Blackboard` slot accessor names — `LOCATIONS` <=20.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --all-targets --test visual_debug_blackboard_tap_tdd -- blackboard_tap_capture_contracts --exact 2>&1 | tee target/test-output.log` - FACT from log; `cargo xtask build-guests --check`.
- Exit: SliceIR-family taps capture prepass-only with pinned fields.

### Step 4: Blackboard-read capture — classification/seam/support/regionmapping

- Task IDs: `TASK-271`
- Objective: add `CapturedIr` variants for `SurfaceClassificationIR` (MeshAnalysis, OverhangAnnotation), `SeamPlanIR` (SeamPlanning), the SupportGeometry composite (`SupportGeometryIR`+`SupportPlanIR`), and the RegionMapping composite (`RegionMapIR`+`SliceIR` for the render-time join); wire tap ids in `visual_debug.rs`.
- Postcondition: all eight Blackboard taps capture their committed slot(s); AC-1 passes fully with corrected fields (`chosen_candidate.point`, `region_key`, `ConfigId`).
- Files allowed to edit (<=3): `crates/slicer-runtime/src/layer_executor.rs`, `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`.
- Out of bounds: render code; validation; arena path.
- Dispatch: exact field accessors + `schema_version` for the four source types — `SNIPPETS` <=3, 30 lines.
- Context cost: `M`
- Verification: the AC-1 test - FACT from log; `cargo xtask build-guests --check`.
- Exit: every Blackboard tap captures pinned, correctly-named fields.

### Step 5: PostPass whole-print capture

- Task IDs: `TASK-271`
- Objective: add a PostPass capture path that runs the full prefix (all layers -> finalization -> `execute_postpass`, `postpass.rs:87/:135`) and captures the finalized `Vec<LayerCollectionIR>` (LayerFinalization) and `GCodeIR` (GCodeEmit); record whole-print `executed_stage_ids`/`executed_layer_indices` in the manifest; render only selected layers.
- Postcondition: AC-2 passes; GCodeEmit selection triggers emission; ordinary emission behavior unchanged.
- Files allowed to edit (<=3): `crates/slicer-runtime/src/layer_executor.rs`, `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs`. (Only if `execute_postpass` does not already surface the finalized/emitted IRs read-only, add a minimal read hook in `crates/slicer-runtime/src/postpass.rs` and split this step.)
- Out of bounds: changing what G-code is emitted; scheduler edges.
- Context cost: `M`
- Verification: the AC-2 test - FACT from log; `cargo xtask build-guests --check`.
- Exit: PostPass taps capture finalized/emitted IR and record the closure.

### Step 6: Renderer — new-variant geometry, RegionMapping join, mixed units

- Task IDs: `TASK-271`
- Objective: extend `visual_debug_render.rs` to render the new `CapturedIr` geometry variants; implement the RegionMapping `RegionMapIR`->`SliceIR` join on `(global_layer_index, object_id, region_id, variant_chain)` tinted by `RegionPlan` (`config_for()`); project Point2 (100 nm) and f32-mm sources into one shared viewport; wire render dispatch in `visual_debug.rs`.
- Postcondition: AC-4 and the RegionMapping-geometry portion of AC-3 pass; no synthetic-diagram symbol is added.
- Files allowed to edit (<=3): `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`.
- Out of bounds: capture code; coordinate helper crates.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- mixed_unit_shared_viewport --exact 2>&1 | tee target/test-output.log` - FACT; `cargo xtask build-guests --check`.
- Exit: mixed-unit geometry and RegionMapping join render in one correct viewport.

### Step 7: Renderer — seam and LayerPlanning overlays

- Task IDs: `TASK-271`
- Objective: render `SeamPlanIR` seam-point overlays (mm) and expose `LayerPlanIR` sync/non-planar/active-region flags as an opt-in `diagnostic_overlay` annotation on geometry taps; assert the synthetic-diagram render mode does not exist.
- Postcondition: AC-3 passes fully (join + overlay + no synthetic mode).
- Files allowed to edit (<=3): `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`.
- Out of bounds: capture code; a second render mode.
- Context cost: `M`
- Verification: `cargo test -p slicer-runtime --all-targets --test visual_debug_render_tap_tdd -- regionmapping_join_and_layerplanning_overlay --exact 2>&1 | tee target/test-output.log` - FACT.
- Exit: overlays render; no synthetic mode; LayerPlanning has no standalone tap.

### Step 8: Two-phase fail-closed validation and selectors

- Task IDs: `TASK-271`
- Objective: in `validate_request` (:199) add phase-1 rejection of unknown visualization kinds, `diagnostic_overlay` on a G-code source, and `LayerSelector::Name`; add `LayerSelector::Range { start, end }` with `#[serde(deny_unknown_fields)]`; implement phase-2 resolution of `Index`/range/z-only against the schedule (model `LayerPlanIR.global_layers`; gcode parsed `;Z:`) failing closed on no match; add `ValidationError` variants; make the former silent-drop sites (:340/:407/:701-705) unreachable.
- Postcondition: AC-N1..AC-N4 pass; no requested output is ever silently omitted.
- Files allowed to edit (<=3): `crates/pnp-cli/src/visual_debug.rs`, `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`, `crates/pnp-cli/src/visual_debug_gcode.rs` (only if the gcode-branch selector reject moves into the unified validator).
- Out of bounds: renderer; capture; manifest schema ownership.
- Context cost: `M`
- Verification: `cargo test -p pnp-cli --all-targets --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log` - FACT from log.
- Exit: unknown kinds, overlay-on-gcode, `Name`, and malformed ranges are rejected; range/z-only resolve or fail closed.

### Step 9: Agent skill and guide examples

- Task IDs: `TASK-271`
- Objective: add an independent `.claude/skills/visual-debug/SKILL.md` (source selection, request authoring, manifest-first inspection, warnings, resolution cost, fail-closed failure behavior, `debug-pipeline` cross-links) and model-backed + standalone-G-code examples.
- Postcondition: AC-6 and AC-N5 pass without claiming Orca parity, WASM behavior, or coordinate changes.
- Files allowed to edit (<=3): `.claude/skills/visual-debug/SKILL.md`, `.claude/skills/visual-debug/examples/model-backed.md`, `.claude/skills/visual-debug/examples/standalone-gcode.md`.
- Out of bounds: `docs/19_visual_debug.md` beyond Step 1; all code.
- Context cost: `S`
- Verification: the AC-6 and AC-N5 `python3` checks - FACT pass/fail.
- Exit: skill selects visual-debug, is manifest-first, routes non-geometry work to `debug-pipeline`.

### Step 10: Determinism, overhead, and closure gates

- Task IDs: `TASK-271`
- Objective: add byte-determinism tests for model (including one whole-print PostPass tap) and standalone-G-code bundles, and the ordinary-slice no-overhead proof; run the closure gates.
- Postcondition: AC-5 and AC-7 pass; all-target check, clippy, and guest-freshness are clean.
- Files allowed to edit (<=3): `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs`, plus a bounded packet-local fix only for a packet-local failure.
- Out of bounds: ordinary slice production path (observe only).
- Context cost: `M`
- Verification: the AC-5 and AC-7 tests - FACT from log; `cargo xtask build-guests --check`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Exit: both modes byte-deterministic, ordinary slice has no visual-debug signal, gates clean.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Docs reconcile (spec fields + docs/02 SupportGeometryIR). |
| Step 2 | S | Cleanup + TravelMove doc. |
| Step 3 | M | Blackboard capture, SliceIR family. |
| Step 4 | M | Blackboard capture, classification/seam/support/regionmapping. |
| Step 5 | M | PostPass whole-print capture. |
| Step 6 | M | Renderer geometry + join + mixed units. |
| Step 7 | M | Renderer seam/LayerPlanning overlays. |
| Step 8 | M | Fail-closed validation + selectors. |
| Step 9 | S | Agent skill + examples. |
| Step 10 | M | Determinism, overhead, gates. |

Aggregate is `L` (ten steps, no single step above `M`). Per the monolithic decision
the packet stays whole; if `spec-review --preflight` rules it non-atomic, split at
the Blackboard-vs-PostPass seam (Steps 3-4/6-8/9 vs Step 5) before activation.

## Packet Completion Gate

- All ten steps and exits complete; every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` reports fresh; `cargo check`/`clippy
  --workspace --all-targets` clean.
- Update `docs/07_implementation_status.md` (TASK-271) through a worker dispatch,
  never a full backlog read.
- `packet.spec.md` is ready for `status: implemented` only after the independent
  reviewer clears the packet.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run the gated full suite once at closure via `cargo xtask test --summary
  --workspace` (guest-freshness entry point), after all narrow commands pass;
  dispatch it to a sub-agent returning `FACT PASS/FAIL` — never absorb full output.
- Record remaining packet-local risk; confirm context stayed at or below 150k
  standard (or at/below 300k only with a logged swarm ESCALATION).

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and
verification commands must use `--all-targets`.
