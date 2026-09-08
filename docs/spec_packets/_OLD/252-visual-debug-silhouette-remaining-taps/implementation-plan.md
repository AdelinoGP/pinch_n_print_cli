# Implementation Plan: 252-visual-debug-silhouette-remaining-taps

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: RegionMapping silhouette extraction arm (renderer)

- Task IDs: `TASK-458`
- Objective: add the `CapturedIr::RegionMapping` arm to the silhouette composite extraction — render-time join against the capture's own retained `slice_ir` (full-tuple key, `(object_id, region_id, variant_chain)` sort, skip-on-miss with a count), per-joined-region slabs `[capture.layer_z − effective_layer_height, capture.layer_z]`, tint classes keyed by `config_tint` RGB with ascending-RGB paint order, and the new unjoined-entry warning — pinned by AC-1/AC-2/AC-3 renderer tests.
- Precondition: packet 247 implemented (FORWARD-DEP — `render_silhouette_composite`, the interval-union/rectangle-emission machinery, the occlusion-warning slot, and `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` all exist). Dispatch the seam check first: if packet 249's `render_silhouette_composite_styled` delegation has landed, the arm goes into the shared internals both entry points use.
- Postcondition: `region_mapping_slabs_follow_joined_effective_layer_height`, `region_mapping_tint_class_order_and_determinism`, and `region_mapping_unjoined_entries_warn_and_skip` pass; every pre-existing test in the binary still passes (no behavior change for Slice/Support groups).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — ranged: the composite renderer region (247's), `region_mapping_shapes`, `config_tint`, `palette`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` — ranged: fixture helpers + one existing composite test as the pattern
  - `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` — ranged: `seeded_region_map`, `seeded_slice_ir` fixtures only
  - `crates/slicer-ir/src/slice_ir.rs` — ranged: `RegionMapIR`/`RegionKey`/`RegionPlan`/`config_for`, `SlicedRegion`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**` (validation/assembly is Step 3), `crates/slicer-runtime/src/layer_executor.rs` (read-only capture shapes), packet dirs 247–251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step; the arm is a new match case plus module-private helpers. No literal blast radius.
- Expected sub-agent dispatches:
  - Question: does `render_silhouette_composite_styled` exist in `crates/slicer-runtime/src/visual_debug_render.rs` (249 landed)?; scope: that file; return: `FACT` yes/no + the extraction seam's function name
  - Question: run the step verification command; scope: one cargo test invocation; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D1, D2, D8 RegionMapping row, §7 only
  - `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — Code Change Surface section only
- OrcaSlicer refs:
  - none (PnP-native)
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail (whole binary: new tests green, prior tests unregressed)
  - `cargo xtask check-literals` — exit code (new `RegionMapIR`/`SliceIR` fixtures)
- Exit condition: the three AC tests pass, the binary is green, and a falsifying check holds — deleting the per-region slab lookup (uniform slab) makes `region_mapping_slabs_follow_joined_effective_layer_height` fail on the catch-up bottom pixel.

### Step 2: Overhang height index + `render_silhouette_overhang_composite`

- Task IDs: `TASK-459`
- Objective: add `SilhouetteLayerHeightClass`, `SilhouetteSliceHeightIndex`, `build_silhouette_slice_height_index`, the quartile palette constants, `RenderError::InvalidQuartile` (+ Display arm), and `render_silhouette_overhang_composite` — keyed band lookup, ascending-quartile paint order, single-height fast path, mixed-height `polygon_ops::intersection` partition, fail-closed arms (invalid quartile, empty bands, missing height-index layer) — plus `slicer_runtime` re-exports; pinned by AC-4/AC-5/AC-6/AC-N3/AC-N4/AC-N5.
- Precondition: packet 247 implemented (FORWARD-DEP — `SilhouetteView`, viewport type, rectangle emission machinery, the test binary). Step 1 not required (independent halves; may run in a parallel worker).
- Postcondition: `overhang_bands_single_height_slabs_and_quartile_order`, `overhang_bands_partition_across_mixed_height_classes`, `overhang_composite_is_deterministic`, `overhang_invalid_quartile_fails_closed`, `overhang_empty_bands_fail_closed`, `overhang_missing_height_index_layer_fails_closed` pass; existing suites unregressed.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — ranged: `RenderError` + Display, `palette`, `surface_classification_shapes`, the 247 composite region
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` — ranged: fixture helpers
  - `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` — ranged: `seeded_surface_classification` only
  - `crates/slicer-ir/src/slice_ir.rs` — ranged: `SurfaceClassificationIR`/`QuartileBand`, `SlicedRegion`
  - `crates/slicer-core/src/polygon_ops.rs` — ranged: `intersection` signature and doc comment only
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — module doc comment only (band⊆footprint, merge-by-quartile invariants)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**` (Step 3), `crates/slicer-runtime/src/layer_executor.rs` and `blackboard.rs` beyond read-only signatures, packet dirs 247–251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `RenderError` gains a variant, not a field: design.md records the verified radius (variant-matched only in its own Display impl; pnp-cli consumes via error conversion). Re-verify at implementation time with the dispatch below before editing. New pub structs are new types — no existing literal sites exist by definition.
- Expected sub-agent dispatches:
  - Question: list every variant-`match` site of slicer-runtime's `RenderError` outside its Display impl; scope: `crates/`; return: `LOCATIONS ≤10` (expected: none)
  - Question: run the step verification command; scope: one cargo test invocation; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — fact 5, D1, D2, D8 OverhangAnnotation row, §7 only
- OrcaSlicer refs:
  - none (PnP-native)
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p slicer-runtime --test visual_debug_render_tap_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail (palette/RenderError additions must not disturb the top-down suite)
  - `cargo xtask check-literals` — exit code
- Exit condition: all six AC tests pass and a falsifying check holds — replacing the mixed-height partition with the layer's first height class makes `overhang_bands_partition_across_mixed_height_classes` fail on the second class's bottom pixel.

### Step 3: Validation lift, assembly routing, arm retirement

- Task IDs: `TASK-460`
- Objective: `SILHOUETTE_TAP_STAGE_IDS` += the two taps (reason arms deleted); `run_model_source` silhouette branch routes OverhangAnnotation groups through `build_silhouette_slice_height_index(ctx.blackboard.slice_ir())` + `render_silhouette_overhang_composite` and RegionMapping groups through the existing composite call; tool-mode groups for either tap fail with `RenderError::ToolColorUnavailable` at the same seam 249 rejects `CapturedIr::Slice`; retire the two arms of `silhouette_unsupported_taps_rejected_with_reasons` (remaining arms — arena, MeshAnalysis, SeamPlanning — keep passing); add `silhouette_region_mapping_and_overhang_taps_accepted` (AC-N2's render-level tool test lives in the bundle binary — Step 4, which drives the real pipeline).
- Precondition: Steps 1–2 complete; packets 247 and 249 implemented (FORWARD-DEP — the assembly branch, `SILHOUETTE_TAP_STAGE_IDS`, and 249's per-capture tool contract exist; the swarm executes queue order so both precede this packet).
- Postcondition: AC-8 and AC-N1 tests pass; the retired arms are gone; no other validation behavior changes (AC-N1's remaining-arms run is the falsifier for over-deletion).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — ranged: `validate_request`'s silhouette region, `SILHOUETTE_TAP_STAGE_IDS`, the silhouette assembly branch
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` — ranged: `silhouette_unsupported_taps_rejected_with_reasons` + the library-call harness helpers
  - `crates/slicer-runtime/src/blackboard.rs` — ranged: `slice_ir()` accessor only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` (Steps 1–2 froze the renderer surface), docs (Step 4), packet dirs 247–251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant added; the whitelist is a const slice extension. The deleted reason arms are the only removals — AC-N1's test run is the regression gate.
- Expected sub-agent dispatches:
  - Question: run the step verification command; scope: one cargo test invocation; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §6 R2 row and §8 only
  - `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` — AC-N5 text only (the arms being retired)
- OrcaSlicer refs:
  - none (PnP-native)
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail (retirement + acceptance + remaining arms in one binary)
- Exit condition: the whole validation binary is green; grepping the AC-N5 test body for the two tap names finds them only in acceptance tests, never in its rejected-arm list; a falsifying check holds — reverting the whitelist addition makes `silhouette_region_mapping_and_overhang_taps_accepted` fail with `SilhouetteUnsupportedForTap`.

### Step 4: End-to-end bundle coverage + docs

- Task IDs: `TASK-461`
- Objective: AC-7's end-to-end RegionMapping bundle test over `resources/regression_wedge.stl` (entry shape, filename `PrePass__RegionMapping_silhouette_front.png`, subset-vs-all-layers `world_bounds_mm` byte-identity); AC-N2's `silhouette_tool_on_remaining_taps_fails_tool_color_unavailable` (tool-mode request for each new tap fails with `RenderError::ToolColorUnavailable` before any geometry is read — deterministic regardless of the wedge's overhang-band content); extend docs/19's silhouette tap table with the two rows (anchors `quartile`, `tint class`).
- Precondition: Steps 1–3 complete; guest WASMs fresh (`cargo xtask build-guests --check` exit 0 — exit 1 rebuild first, exit 3 is infra error, not clean).
- Postcondition: AC-7, AC-N2, and AC-9 pass; the bundle binary is green end to end.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` — ranged: harness helpers + one existing bundle test as the pattern
  - `docs/19_visual_debug.md` — direct read (232-line scale; re-derive length at read time)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs`
  - `docs/19_visual_debug.md`
- Files explicitly out of bounds:
  - `crates/**/src/**` (production surface frozen after Step 3), `docs/07_implementation_status.md` (completion-gate dispatch only), packet dirs 247–251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - None — test + docs only.
- Expected sub-agent dispatches:
  - Question: `cargo xtask build-guests --check` exit code; scope: repo root; return: `FACT` exit code
  - Question: run the step verification command; scope: one cargo test invocation; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` — edited here; the 247-authored silhouette section is the insertion point
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D3 (model-wide framing) only
- OrcaSlicer refs:
  - none (PnP-native)
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `rg -q 'quartile' docs/19_visual_debug.md && rg -q 'tint class' docs/19_visual_debug.md && echo PASS` — PASS printed
- Exit condition: AC-7's test passes against the real pipeline, both doc anchors resolve, and the docs rows state the occlusion-relevant facts (tint/quartile paint order) rather than restating the plan.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | RegionMapping arm + tint classes + 3 renderer tests |
| Step 2 | M | Height index + overhang entry point + 6 renderer tests |
| Step 3 | S | Whitelist lift + assembly routing + arm retirement |
| Step 4 | M | End-to-end wedge bundle + docs rows |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (adds TASK-458..461 rows per `task-map.md`).
- Reconcile reopened/superseded status transitions (none expected; this packet retires no packet-level status).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected: the tool-rejection seam location if 249's refactor moved between authoring and implementation).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
