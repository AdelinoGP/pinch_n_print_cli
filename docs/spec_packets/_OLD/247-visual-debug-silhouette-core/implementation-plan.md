# Implementation Plan: 247-visual-debug-silhouette-core

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every `cargo test` invocation tees to `target/test-output.log`; read the log, never re-run for output.

## Steps

### Step 1: Schema 1.2.0 gate and the silhouette validation matrix

- Task IDs: `TASK-442`
- Objective: `validate_request` (`crates/pnp-cli/src/visual_debug.rs`) accepts `schema_version: "1.2.0"`, recognizes the `silhouette` kind only under it, and enforces every 247 rejection: `SilhouetteRequiresSchema12` (R1), `SilhouetteMixedWithOtherKinds` (R3), one-view-per-bundle + view value/kind checks via `InvalidSilhouetteView` (R5 + bundle-plane rule), `SilhouettePlateFrameUnsupported` (R4), `SilhouetteUnsupportedForTap` against the new module-local `SILHOUETTE_TAP_STAGE_IDS` const (R2), `SilhouetteUnsupportedOnGcodeSource` (interim), `InvalidColorBy` for `color_by: "tool"` on silhouette (interim), with `VisualizationOptions` gaining `#[serde(default)] pub view: Option<String>` and 1.2.0 requests using the strict typed options parse (which also makes `composited_overlays` fail via `deny_unknown_fields` — AC-N7). An explicit `view` key under a declared pre-1.2 schema is hard-rejected with a message naming `"1.2.0"` (AC-N4 case (c)): under 1.1.0 via the strict-parsed `opts.view.is_some()` check, under 1.0.0 via an `options.get("view")` probe alongside the existing 1.1-option-name hard-reject list — mirroring how the 1.1 options were fail-closed under 1.0 rather than left as tolerated stray keys.
- Precondition: tree at current branch head; `cargo check -p pnp-cli --all-targets` green.
- Postcondition: AC-N1..AC-N10's tests exist and pass; every pre-existing test in `visual_debug_validation_tdd` still passes; no rendering code touched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — the const/`VisualizationOptions`/`ValidationError`/`validate_request` region (approx. the first 700 lines) only
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` — fixture helpers + any test being extended
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §4.2 (D5–D7), §6 only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (renderer arrives in Step 3), `crates/pnp-cli/src/visual_debug_gcode.rs`, all docs
- Blast-radius discipline: no struct field or public constant changes in this step (`VisualizationOptions` is `Deserialize`-only with `#[serde(default)]` on every field — adding `view` breaks no construction site; verified: `VisualizationOptions` literals exist only in `visual_debug.rs` itself, both using FRU or full defaults).
- Expected sub-agent dispatches:
  - Question: run the step verification command and report pass/fail + failing test names; scope: the command below; return: `FACT` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §6 fail-closed matrix (ranged read)
- OrcaSlicer refs: none (PnP-native tool).
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: the ten new negative tests (AC-N1..AC-N10 names as pinned in `packet.spec.md`) pass, and deliberately breaking one rejection (e.g. commenting the mixing-ban check) makes its test fail — proving the tests bind. If any existing validation test fails, the step is wrong, not the test.

### Step 2: 1.2.0 manifest shape with 1.0/1.1 byte-compat pinning

- Task IDs: `TASK-443`
- Objective: change `ImageEntry` (`crates/pnp-cli/src/visual_debug.rs`) to the D7 shape — `layer_index: Option<i64>` (+ `skip_serializing_if = "Option::is_none"`), `layer_z: Option<Option<f64>>` (+ skip; tri-state per design.md: `Some(Some(z))` → number, `Some(None)` → `null`, `None` → absent), new `view: Option<String>` and `layers_rendered: Option<Vec<LayerRangeEntry>>` (+ skip), new `pub struct LayerRangeEntry { pub start: i64, pub end: i64 }` — updating every construction site, and pin 1.0/1.1 serialization byte-compatibility with tests.
- Precondition: Step 1 merged (not functionally required, but keeps `visual_debug.rs` merge-conflict-free).
- Postcondition: all four `ImageEntry` literal sites compile with wrapped values (`layer_index: Some(...)`; model arms `layer_z: Some(Some(capture.layer_z as f64))`; gcode arm `layer_z: Some(image.layer_z)`; new fields `None` everywhere); `cargo check --workspace --all-targets` green; AC-8's test passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `ImageEntry` definition + the four construction sites (the `typed_ir` arm, isolated-overlay arm, geometry arm in `run_model_source`; the gcode arm in `run_visual_debug`)
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` — full (under 400 lines)
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — `ParsedLayer`/`RenderedImage` `layer_z` docs only (the AC-8 null case)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/pnp-cli/src/visual_debug_gcode.rs` (read-only), docs
- Blast-radius discipline (mandatory — struct field change):
  - `ImageEntry` struct-literal sites: exactly the four listed construction sites in `crates/pnp-cli/src/visual_debug.rs`; grounding grep (2026-08-27) found **no** Rust struct literal of `ImageEntry` in any test or other crate (`ImageEntry` matches only in `visual_debug.rs`, `visual_debug_gcode.rs` doc text, and `visual_debug_intermediate_renderer_tdd.rs` comments). Existing tests assert via `serde_json::Value` indexing (`entry["layer_index"]`), which is unaffected because 1.0/1.1 entries still serialize both keys with unchanged values. Re-verify with a `LOCATIONS` dispatch (`rg -n 'ImageEntry' crates/`) before editing; if new sites appeared since, they join this step's edit budget.
  - New tests in `visual_debug_request_bundle_tdd.rs`: `legacy_entries_keep_layer_index_and_null_layer_z_serialization` — renders a 1.0.0 gcode bundle whose fixture layer has `;LAYER_CHANGE` but **no** `;Z:` line, then asserts the entry object contains `"layer_index"` (integer) and `"layer_z"` with `Value::Null`; plus an assertion extending `ac_manifest_serializes_required_index_and_entry_fields` that the 1.0.0 entry's key set is unchanged (no `view`, no `layers_rendered`).
- Expected sub-agent dispatches:
  - Question: any `ImageEntry` literal outside `crates/pnp-cli/src/visual_debug.rs`?; scope: `crates/`; return: `LOCATIONS ≤10`
  - Question: run the verification commands; scope: below; return: `FACT` per command
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D7 only (ranged read; note design.md's tri-state correction supersedes D7's literal `layer_z` text)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_request_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_intermediate_renderer_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — existing manifest-determinism suite unregressed (requires fresh guests: `cargo xtask build-guests --check` first; exit 0 required before blaming this step)
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: AC-8's test passes AND temporarily reverting `layer_z` to plain `Option<f64>` + skip makes it fail (proves the null case is really pinned); the intermediate-renderer suite is green.

### Step 3: Composite silhouette renderer — Slice-family core

- Task IDs: `TASK-444`
- Objective: add to `crates/slicer-runtime/src/visual_debug_render.rs`: `SilhouetteView` (`Front`/`Side`, `name()`, `parse()`), `SilhouetteScheduleSlab`/`SilhouetteSlabSchedule`, the interval-union helper (sorted endpoint sweep, touch merges, exact `f32` comparison), `compute_silhouette_viewport_bounds`, and `render_silhouette_composite` handling `CapturedIr::Slice` captures: one body class from `SliceIR.regions[].polygons` contours (`Point2::to_mm`, min/max on the per-view axis), per-region slabs `[capture.layer_z − region.effective_layer_height, capture.layer_z]`, rectangles emitted ascending layer → class → interval start, drawn via the private `Shape::Fill`/`draw_shapes`/`Canvas` machinery and the shared `Projector`; fail closed `RenderError::MissingGeometryField` when the whole group yields zero rectangles; re-export the new public names from `crates/slicer-runtime/src/lib.rs`.
- Precondition: none beyond a compiling tree (independent of Steps 1–2).
- Postcondition: AC-2, AC-3, AC-6 pass at the renderer level; `render_stage_capture_styled` and all existing render paths untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — module doc, `Projector`/`ViewportBoundsMm`/`Canvas`/`Shape`/`draw_shapes`/`geometry_points_mm`/`slice_shapes` regions only
  - `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs` — `decode_rgb`/`mm_to_px`/fixture helper region only (copy the pattern)
  - `crates/slicer-ir/src/slice_ir.rs` — `SliceIR`/`SlicedRegion` definitions only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (new file — a top-level test binary; no aggregator exists or is needed)
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`, `crates/slicer-runtime/src/layer_executor.rs` (read shapes via the slice_ir read above; captures are constructed directly in tests)
- Blast-radius discipline: no existing struct/constant changes; all additions. New test fixtures constructing `SliceIR`/`SlicedRegion` must use `..Default::default()` FRU or an `// exhaustive:` waiver (both derive `Default`; follow `visual_debug_blackboard_tap_tdd.rs`'s `seeded_slice_ir` pattern).
- Expected sub-agent dispatches:
  - Question: run the verification + `cargo xtask check-literals`; scope: below; return: `FACT` per command
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §2, D1, D2, §7 (ranged read)
  - `docs/08_coordinate_system.md` — mm/unit boundary rules
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p slicer-runtime --test visual_debug_render_tap_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — existing renderer suite unregressed
  - `cargo xtask check-literals` — exit 0
- Exit condition: `region_slab_bottoms_follow_effective_layer_height` (two `SlicedRegion`s with distinct `effective_layer_height` values — the only height field `SlicedRegion` carries; catch-up flags live on `ActiveRegion`, not here — each rectangle bottom at its own `z − effective_layer_height`), `interval_union_holes_islands_and_touching_merge`, and `silhouette_composite_is_deterministic` pass; flipping the union's touch-merge comparison from `<=` to `<` makes the touching-merge test fail (binding check); an empty-group render returns `Err(MissingGeometryField)`, not a blank PNG.

### Step 4: Composite silhouette renderer — support-plan roles, warnings W1/W2

- Task IDs: `TASK-444`
- Objective: extend `render_silhouette_composite` with the `CapturedIr::SupportGeometry` arm: per-`SupportPlanRole` classes from `entry.roles[].regions` (entries with `global_layer_index >= 0` matching the capture layer, sorted `(object_id, region_id)` like `support_geometry_shapes`), slabs from the caller's `SilhouetteSlabSchedule`, the fixed paint order (body → `SUPPORT_RAFT` → `SUPPORT_BASE_INTERFACE` → `SUPPORT_BOTTOM_INTERFACE` → `SUPPORT_INTERFACE` last) with the three new `palette` constants, W1 (negative-index entries: count + `min..max` dropped range), W2 (non-empty coarse `SupportGeometryIR.entries`: count + the design.md-pinned emit-schedule wording), and the occlusion warning (later class overlapping an earlier class's union on the same layer: affected-layer count); warnings deduped per group, ordered W1, W2, occlusion.
- Precondition: Step 3 merged.
- Postcondition: AC-4 and AC-5 pass; Slice-family behavior from Step 3 unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — the Step-3 silhouette region + `support_geometry_shapes` + `palette` only
  - `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR`/`SupportPlanEntry`/`SupportPlanRole`/`SupportPlanRoleRegion`/`SupportGeometryIR`/`SupportGeometryKey` definitions only
  - `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` — `seeded_support_geometry_and_plan` fixture only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`, `crates/slicer-runtime/src/lib.rs` (Step 3 already exported everything public), `crates/slicer-core/**` (W2 evidence is already recorded in design.md)
- Blast-radius discipline: adding `palette` constants is additive (the module is `pub mod palette` of consts; no exhaustive match exists over it). `SupportPlanEntry` has no `Default` — test literals follow the existing `// exhaustive:` waiver convention in `visual_debug_render_tap_tdd.rs`.
- Expected sub-agent dispatches:
  - Question: run the verification + `cargo xtask check-literals`; scope: below; return: `FACT` per command
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D2, D9, §6 W1/W2 (ranged read)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo xtask check-literals` — exit 0
- Exit condition: `support_role_paint_order_and_occlusion_warning` and `raft_and_coarse_entries_skip_with_named_warnings` pass; reversing the class paint order makes the paint-order test fail (binding); a plan with only negative-index entries and no coarse entries yields W1 but not W2.

### Step 5: pnp-cli composite assembly, Z framing, filenames, manifest emission

- Task IDs: `TASK-445`
- Objective: in `run_model_source` (`crates/pnp-cli/src/visual_debug.rs`), add the silhouette branch: resolve the bundle view (default `front`); build the `SilhouetteSlabSchedule` from `ctx.blackboard.layer_plan()` (`z_bottom` = previous `GlobalLayer.z`, `0.0` for index 0); build the model plane extent from `mesh.build_volume` (X-or-Y × Z) before `mesh` moves into `prepare_prepass_context` (mirror `mesh_xy_bounds`'s degenerate handling); compute the bundle viewport once via `compute_silhouette_viewport_bounds`; group captures by tap; call `render_silhouette_composite` per group; emit one `ImageEntry` per group with `view: Some(...)`, `layers_rendered: Some(ranges)` (maximal consecutive runs of the group's capture layer indices), `layer_index: None`, `layer_z: None`, `typed_capture: None`, renderer warnings, shared `world_bounds_mm`, filename `{sanitized_tap}_silhouette_{view}.png`; groups ordered by `STAGE_ORDER` position then tap; author the end-to-end bundle tests over the wedge fixture.
- Precondition: Steps 1–4 merged; `cargo xtask build-guests --check` exits 0 (rebuild without `--check` if 1).
- Postcondition: AC-1, AC-7, AC-9 pass end-to-end; non-silhouette bundles behave exactly as before (the branch is mixing-ban-disjoint).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `run_model_source` + `mesh_xy_bounds` + the render-loop region only
  - `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` — fixture helpers (`wedge_path`/`module_dir`/`write_bounded_config`/`manifest_at`) only
  - `crates/slicer-runtime/src/visual_debug_render.rs` — the new public silhouette signatures only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` (new file — top-level test binary, no aggregator)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (consume the Step-3/4 exports; if a signature does not fit, stop and fix it in a Step-3/4 revisit, never fork a transform here), `crates/pnp-cli/src/visual_debug_gcode.rs`, docs
- Blast-radius discipline: no struct changes (Step 2 owns `ImageEntry`); new-code-only branch plus one new test binary.
- Expected sub-agent dispatches:
  - Question: `cargo xtask build-guests --check` exit code; scope: repo root; return: `FACT` (exit code)
  - Question: run the verification commands; scope: below; return: `FACT` per command + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D3, §5 (ranged read)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_request_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — legacy bundle contract unregressed
  - `cargo xtask check-literals` — exit 0
- Exit condition: `silhouette_bundle_entry_shape_and_default_front_view`, `z_frame_is_model_wide_not_selection_wide`, and `one_image_per_tap_view_group_and_unique_filenames` pass; the entry-shape test additionally re-expands `layers_rendered` and asserts equality with the resolved index set (lossless round-trip); asserting `"layer_index"` absent on the silhouette entry fails if the key is present (binding on the D7 shape).

### Step 6: Docs and the raft deviation row

- Task IDs: `TASK-445`
- Objective: add the "Silhouette Side Views (schema 1.2.0)" section to `docs/19_visual_debug.md` (request/manifest shape with `view`/`layers_rendered`, single-plane-per-bundle rule, `world_bounds_mm` plane semantics, model-wide Z framing, D4 scale guidance containing the AC-10-pinned verbatim phrase `for interface-band inspection on tall models, raise` followed by `resolution_scale` (the Z frame is model-wide, so selecting a band does not zoom — scale is the lever), the fixed paint order with the D2 occlusion caveat, the W1/W2 warnings inventory, supported vs rejected taps, filename scheme); correct `docs/02_ir_schemas.md` IR 9a's "keyed by support-layer index" sentence to the producer-verified model-layer emit-schedule semantics (sentinel wording unchanged); append one open `docs/DEVIATION_LOG.md` row tracking raft side-view rendering (ID = next free `DEV-###`, re-derived at write time via `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; follow the log's existing row format — date, severity Low, affected section, mitigation owner, target close naming the follow-up).
- Precondition: Steps 1–5 merged (the docs describe shipped behavior, never planned behavior).
- Postcondition: AC-10 and AC-11 greps pass; no other docs edited.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — full (232 lines pre-edit)
  - `docs/02_ir_schemas.md` — the "IR 9a — SupportGeometryIR" section only (locate via `rg -n 'IR 9a'`)
  - `docs/DEVIATION_LOG.md` — last 10 rows only (format sample + ID derivation)
- Files allowed to edit (at most 3):
  - `docs/19_visual_debug.md`
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - all `crates/**`, `docs/07_implementation_status.md` (completion-gate dispatch only), every other doc that repeats the IR 9a wording (report via the design.md dispatch; do not mass-edit)
- Blast-radius discipline: not applicable (no struct/constant changes).
- Expected sub-agent dispatches:
  - Question: current highest `DEV-###` row ID; scope: `docs/DEVIATION_LOG.md`; return: `FACT`
  - Question: run the AC-10/AC-11 grep commands; scope: repo root; return: `FACT` per grep
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D2, D4, D9, §9 docs list (ranged read)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'silhouette' docs/19_visual_debug.md && rg -qi 'occlu' docs/19_visual_debug.md && rg -q 'for interface-band inspection on tall models, raise' docs/19_visual_debug.md && rg -q 'model-layer' docs/02_ir_schemas.md && rg -q 'raft side' docs/DEVIATION_LOG.md && echo PASS` — FACT PASS/absent
- Exit condition: the combined grep prints `PASS`; the docs/19 section names the exact manifest field names (`view`, `layers_rendered`) and the exact warning conditions (W1 raft, W2 coarse) so a PNG-reading agent finds the occlusion caveat and the scale guidance without opening the plan.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | validation matrix + ten negative tests |
| Step 2 | M | ImageEntry blast radius (4 sites, one file) + byte-compat tests |
| Step 3 | M | renderer core + new test binary |
| Step 4 | M | support roles + warnings |
| Step 5 | M | assembly + end-to-end wedge tests (guest-WASM dependent) |
| Step 6 | S | docs + deviation row |

Split before activation if aggregate cost exceeds M or any step is L. (Aggregate M: steps share no reads beyond their own ranges; each is independently executable by a fresh worker.)

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (add/close the TASK-442..TASK-445 rows per `task-map.md`), never a full backlog read.
- Reconcile reopened/superseded status transitions: none (no packet superseded).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected: the per-layer support-composite clone cost noted in design.md Risks; the f32 near-touch seam note).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where applicable so the test, bench, and example targets compile (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`).
