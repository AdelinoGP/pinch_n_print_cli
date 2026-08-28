# Implementation Plan: 248-visual-debug-silhouette-gcode-source

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Steps 1–2 are independent of packet 247. Steps 3–7 require packet 247 **implemented** (they consume `SilhouetteView`, the interval-union helper, `LayerRangeEntry`, and the 1.2.0 validation surface).

## Steps

### Step 1: Parser groundwork — `e_delta_mm`, `G92 E`, `M200`, `filament_diameter`

- Task IDs: `TASK-446`
- Objective: `Segment` carries the signed per-move E delta; `ParsedGcode` carries `filament_diameters_mm` and `volumetric_extrusion_line`; `parse_gcode` handles `G92 E<val>` resets (correctness fix) and records `M200` without an unsupported warning.
- Precondition: clean tree; `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd` green.
- Postcondition: new parser tests green; the full existing gcode-renderer suite green (the G92 fix must not break — or must re-baseline to canonical-correct — any pinned render).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — the `Segment`/`ParsedGcode` structs and `parse_gcode` body (~lines 350–760)
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` — fixture/decoding helper region only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs`; `crates/slicer-gcode/**`; packet dirs 247/249–251
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `Segment` literal sites: exactly one — the `segments.push(Segment { … })` in `parse_gcode`; zero test literals (LOCATIONS-verified 2026-08-27 via `rg -n 'Segment \{' crates/pnp-cli`). `ParsedGcode` literal sites: exactly one — the return expression of `parse_gcode`. Both edits land in this step; `cargo xtask check-literals` gates.
  - Dispatch a `LOCATIONS` re-check for both structs before editing; cite the result in the commit message.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd` then `--test visual_debug_gcode_renderer_tdd`, tee'd; scope: those two binaries; return: `FACT pass/fail` + failing names
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §3 fact 13, §4.4 D13/D14 (E-mode, missing-datum inventory) only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- g92_e_reset_synchronizes_e_position 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo xtask check-literals`
- Exit condition: AC-7's test passes; the existing gcode-renderer suite is green; a run with the `G92 E` arm commented out makes AC-7's test fail (falsifies the pin).

### Step 2: Slab derivation (D12/W3) and width derivation (D13/D16) helpers

- Task IDs: `TASK-446`
- Objective: `gcode_silhouette_slabs` (per-layer `[prev marker, z]`, first `[0, z]`, W3 warnings, skip-no-guess) and `silhouette_segment_width_mm` (rectangular closed form) exist with direct unit pins.
- Precondition: Step 1 complete.
- Postcondition: helper-level unit tests green — `silhouette_width_formula_closed_form` (the closed-form halves of AC-2, absolute and `M83` relative fixtures) and `slab_derivation_w3_cases` (AC-6's W3 triple: duplicate, non-monotonic, markerless). AC-2's full pipe-suffixed test adds its pixel half in Step 4; AC-6's completes at the bundle level in Step 6.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — parser output structs + the new helpers' insertion point only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs`; `crates/slicer-runtime/**`
- Expected sub-agent dispatches:
  - Question: run the step's test binary; scope: `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §4.4 D12/D13/D16 and fact 9's formula paragraph only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- silhouette_width_formula_closed_form 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- slab_derivation_w3_cases 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: the width helper recovers the authored `0.5` in both E modes; the W3 fixture yields exactly three warnings naming layer indices/values; a slab is never emitted for a warned layer (assert map absence).

### Step 3: Shared interval union — promote packet 247's helper

- Task IDs: `TASK-447`
- Objective: `pub fn union_silhouette_intervals` exists in `crates/slicer-runtime/src/visual_debug_render.rs`, re-exported from `slicer-runtime`; 247's composite renderer delegates to it (byte-equal behavior).
- Precondition: **packet 247 implemented**; dispatch first: FACT on the landed helper's name/visibility. If 247 landed it already-public, this step is rename/re-export only.
- Postcondition: 247's silhouette suite green unchanged; the helper callable from `pnp-cli`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — the union helper + silhouette composite region only
  - `crates/slicer-runtime/src/lib.rs` — the visual_debug_render re-export block only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**` (next step); all packet dirs
- Expected sub-agent dispatches:
  - Question: exact name/visibility of 247's interval-union helper; scope: `crates/slicer-runtime/src/visual_debug_render.rs`; return: `FACT`
  - Question: run `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd`; return: `FACT pass/fail`
- Context cost: `S`
- Authoritative docs:
  - `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — interval-union bullet only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `rg -q 'union_silhouette_intervals' crates/slicer-runtime/src/lib.rs || rg -q 'pub use.*union' crates/slicer-runtime/src/lib.rs`
- Exit condition: 247's suite green; exactly one union implementation exists in the tree (`rg -c` for the sweep's merge comparison returns one production site).

### Step 4: `render_gcode_silhouette` composite renderer

- Task IDs: `TASK-447`
- Objective: the composite entry point renders one PNG per (view, color mode) with per-(layer, class) unions, unclassified-first paint order, fallback/R8 width policy, selection-independent framing, and deterministic output.
- Precondition: Steps 1–3 complete; packet 247 implemented (`SilhouetteView` import).
- Postcondition: AC-2 (pixel half), AC-3, AC-4, AC-5, AC-8, AC-10 (renderer half), AC-N1/N2 (error values from the renderer) green at the module level.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — rasterization region (`viewport_bounds`, `draw_thick_line`, `encode_png`) + the new code
  - `crates/slicer-runtime/src/visual_debug_style.rs` — `gcode_role_color`/`ToolColors` region only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs` (Steps 5–6); `crates/slicer-runtime/src/**` (frozen after Step 3)
- Expected sub-agent dispatches:
  - Question: run the step's test binary; scope: `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd`; return: `FACT pass/fail` + failing names
  - Question: `cargo xtask check-literals` exit code; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §4.4, D15, §7 determinism rules only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- adaptive_z_markers_derive_per_layer_slabs 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- unclassified_class_paints_first_and_warns 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: decoded-pixel assertions pass for slabs, inflation, paint order, fallback, and tool classes; repeat render byte-identical; swapping the paint order in a scratch build flips AC-5 (falsifiability check, then revert).

### Step 5: Validation staging (interim removal, taps rejection, tool narrowing, new error)

- Task IDs: `TASK-448`
- Objective: `SilhouetteUnsupportedOnGcodeSource` removed with its Display arm and interim test; gcode silhouette accepted; non-empty `taps` + gcode silhouette → `SilhouetteUnsupportedForTap`; blanket silhouette tool-coloring rejection narrowed to model source; `VisualDebugError::SilhouetteWidthUnderivable` added and mapped.
- Precondition: packet 247 implemented; Step 4 complete (error type referenced by `map_gcode_error`).
- Postcondition: AC-N4, AC-N5, AC-N6, AC-N7, AC-N3 green; 247's model-source validation tests untouched and green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `ValidationError`/`VisualDebugError` enums, `validate_request` silhouette region, `map_gcode_error` only
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` — the silhouette test region only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug_gcode.rs` (frozen after Step 4); `crates/slicer-runtime/**`
- Expected sub-agent dispatches:
  - Question: does `silhouette_tool_coloring_rejected_role_accepted` use a model-source request?; scope: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`; return: `FACT`
  - Question: run `cargo test -p pnp-cli --test visual_debug_validation_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `M`
- Authoritative docs:
  - `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — `[FWD to packet 248]` bullet only
- OrcaSlicer refs: none
- Verification:
  - `! rg -q 'SilhouetteUnsupportedOnGcodeSource' crates/`
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: the variant is absent from `crates/`, the acceptance test passes, and every other 247 validation pin still passes (a binary-count/test-count drop versus the pre-step run must be exactly the one removed interim test).

### Step 6: Gcode-arm bundle assembly and end-to-end manifest tests

- Task IDs: `TASK-448`
- Objective: the gcode arm of `run_visual_debug` grows the silhouette branch — one entry per (view, color mode) group, `gcode_silhouette_{view}[_tool].png` filenames, `tap: ""`, `layers_rendered` maximal ranges, absent `layer_index`/`layer_z`, `tool_palette` for tool groups.
- Precondition: Steps 4–5 complete.
- Postcondition: AC-1, AC-6, AC-8 (bundle level), AC-9, AC-10 (bundle half), AC-N1/N2 (command level, no bundle written) green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — the gcode arm (~lines 1780–2040) and 247's model silhouette branch as the pattern
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug_gcode.rs`; `crates/slicer-runtime/**`
- Expected sub-agent dispatches:
  - Question: run the step's test binary; scope: `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd`; return: `FACT pass/fail` + failing names
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §5 manifest sketch + D3 framing only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- gcode_silhouette_bundle_entry_shape 2>&1 | tee target/test-output.log | grep -E "^test result"`
  - `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- width_underivable_without_diameter_fails_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`
- Exit condition: the manifest entry matches AC-1 field-for-field; an R8 failure leaves no bundle content on disk (asserted); duplicate silhouette specs collapse to one image per (view, mode).

### Step 7: docs/19 gcode-silhouette subsection

- Task IDs: `TASK-448`
- Objective: `docs/19_visual_debug.md` documents the flow-derived width rule vs `filled_areas`, the rectangular-model and deposited-width caveats, the D14 fallback, W3, and palette-only tool coloring.
- Precondition: Steps 1–6 complete (document what shipped, not what was planned).
- Postcondition: AC-11's greps pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — full (direct read; under 300 lines pre-247, confirm post-247 length and range-read if grown past 300)
- Files allowed to edit (at most 3):
  - `docs/19_visual_debug.md`
- Files explicitly out of bounds:
  - all `crates/**`; `docs/07_implementation_status.md` (completion-gate dispatch only)
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §4.4 caveat paragraphs only
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'flow-derived' docs/19_visual_debug.md && rg -q 'deposited' docs/19_visual_debug.md && rg -q 'W3' docs/19_visual_debug.md && echo PASS`
- Exit condition: all three greps pass and the section explicitly states the silhouette must not be cited as a width-measurement tool.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Parser fields + G92 fix + regression sweep |
| Step 2 | S | Pure helpers + unit pins |
| Step 3 | S | Visibility promotion; 247-dependent |
| Step 4 | M | Composite renderer + pixel tests |
| Step 5 | M | Validation staging + 247 test surgery |
| Step 6 | M | Bundle assembly + e2e manifest tests |
| Step 7 | S | Docs |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` (add TASK-446..448 rows) through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: none (nothing superseded; 247's `[FWD]` obligations discharged).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected: the multi-tool diameter clamp and lazy-R8 selection sensitivity, both documented).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
