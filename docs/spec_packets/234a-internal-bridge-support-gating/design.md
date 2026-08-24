# Design: 234a-internal-bridge-support-gating

## Controlling Code Paths

- Current construction site (to be emptied): the `LayerStageCommit::InfillPostProcess` arm in `crates/slicer-runtime/src/layer_executor.rs` — anchors gathered from `region.sparse_infill` paths plus perimeter walls and inset contours; `candidate_voids = slicer_core::difference(&slice_region.sparse_infill_area, &slice_region.bridge_areas)`; sliver-guard filter keyed on `dont_filter_internal_bridges`; subtraction from `sparse_infill_area`; population of `region.internal_bridge_infill` with `ExtrusionRole::InternalBridgeInfill` paths.
- New home (this packet, Option-A revision approved 2026-08-24): inside `commit_shell_classification_builtin`'s stage in `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, ordered AFTER the shell-classification passes (so `top_solid_fill`/`bottom_solid_fill` exist for every layer) and strictly AFTER 234's `gate_bridge_areas_by_unsupported_span` invocation, walking region timelines so each region's layer L is qualified against committed layer L-1. The pass resolves flow settings via `region_map.config_for(&key)` → `&ResolvedConfig` (the prepass's existing config channel — no ConfigView is threaded into builtins), qualifies upper-layer `top_solid_fill` surfaces with the Step-1 math (`lower_fills` = lower-layer `region.polygons`; `lower_solids` = lower-layer top+bottom solid fills), constructs anchored lines via `construct_anchored_polygon` (anchors: current-region `polygons`/`infill_areas` contours — perimeter walls do not exist until Layer::Perimeters), writes the centerline polylines into net-new host-only `SlicedRegion.internal_bridge_lines: Vec<Vec<Point2>>`, and extends `region.bridge_areas` with the qualified polygons so the existing partition dataflow suppresses module sparse infill there. The SAME-LAYER InfillPostProcess arm in `crates/slicer-runtime/src/layer_executor.rs` is reduced to a pure emitter: map `internal_bridge_lines` → `ExtrusionRole::InternalBridgeInfill` `ExtrusionPath3D`s (z from `slice.z`, width from `flow.thread_diameter_mm` via `ctx.config_view`) into `InfillRegion.internal_bridge_infill`. Rationale for the carrier field: per-layer stages run under `rayon::par_iter` with private per-layer arenas; cross-layer arena reads are forbidden and `blackboard.infill` is empty throughout PrePass, so the prepass cannot populate `InfillIR` directly.
- Pure math home: `crates/slicer-core/src/algos/bridge_over_infill.rs`, beside the existing `determine_bridging_angle` and `construct_anchored_polygon` (both stay unchanged in algorithm; the latter gains a caller at the prepass).
- Neighbouring tests: `crates/slicer-core/tests/bridge_over_infill_tdd.rs` (233's suite), `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (234's), `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` (wedge pins), `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` (slot-ceiling assertions at print_z 28.2).

## Architecture Constraints

- **Placement supersession (explicit):** 233's AC-N2 recorded "prepass stays free of internal-bridge logic". This packet reverses that placement. Rationale: canonical qualification needs committed lower-layer geometry; layers slice in parallel so InfillPostProcess cannot legally read layer L-1 — the identical constraint that moved 234's false-site gate to ShellClassification. The reversal is a designed decision recorded here, not a silent deviation; no DEVIATION_LOG row is required because the original constraint was a design choice, not an invariant.
- **Candidate source (revised 2026-08-24):** candidates are the upper layer's `top_solid_fill` surfaces — the dense ceiling interfaces. `bottom_solid_fill` is RULED OUT by writer evidence (`slice_postprocess_prepass.rs` Pass 1 depth-0: `apply_opening(difference(region_polys, lower_polys))` — created only where the lower layer is ABSENT, the exposed-floor shell, the opposite of an internal interface). `top_solid_fill` is available at prepass once the shell-classification passes complete; Step 2 must verify its population ordering before wiring (if it turns out to be computed post-prepass, STOP and report). AC-5's frozen calicat bar arbitrates empirically: correct gating must yield ≤6 internal-bridge layers with ~1 site near Z≈29.4. ZERO qualifying sites on calicat = STOP-and-report with measurements, never a silent candidate re-pick.
- **Material exclusion (verified dataflow):** do NOT subtract from `sparse_infill_area` at prepass — it does not exist yet there and would be overwritten: `region_partition.rs` derives it at Perimeters commit as `difference(wall_inset, bridge ∪ bottom ∪ top)`. The lever is `bridge_areas`: extend it with the qualified polygons after 234's gate; partition then excludes them from sparse, the rectilinear module emits only over `sparse_infill_area`, so no double-printing occurs. Precedent: 234's gate already mutates `bridge_areas` at prepass and partition consumes the gated value.
- **Arm reduction:** today's InfillPostProcess subtraction of `sparse_infill_area` (layer_executor.rs L2439-2440) is post-hoc dead code (the module emitted at `Layer::Infill`, one stage earlier; nothing downstream re-reads the field) — delete it with the rest of the construction block.
- **Canonical arithmetic [port contract]:** `unsupported_area = closing(lower_fills, spacing) shrunk by expansion_multiplier*spacing` minus `(lower_solids shrunk 1*spacing) expanded (1+mult)*spacing`, with `expansion_multiplier = 3` default, `1` when the filter key selects limited filtering. Per surface `s`: `unsupported = intersection(s, unsupported_area)`; qualify iff non-empty AND (`area(unsupported) == area(s)` OR `area(unsupported) > 9*spacing^2`). Bridge polygon = `expand(unsupported, 4*spacing)` clipped to `s`; leftovers with `spacing^2 < area < 12*spacing^2` remerge into neighbours.
- **Config mapping:** `dont_filter_internal_bridges=false` → full gates (canonical `ibfDisabled` default); `true` → bypass area/partial gate (`ibfNofilter`). The multiplier parameter carries `ibfLimited`. Key string stays snake_case; manifest untouched. At the prepass, settings resolve via `region_map.config_for(&key)` → `&ResolvedConfig` (pattern: prepass lines ~508/~551); the reduced arm keeps `ctx.config_view` for `flow.thread_diameter_mm`.
- **Ordering:** support gating runs strictly after 234's false-site gate within the same stage; external-site orientation ordering from 235 is untouched.
- Schema/version constants: none touched. No new IR field, no WIT change, no scheduler stage addition.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach (Option-A revision): add pure functions to `bridge_over_infill.rs` — `unsupported_span_areas(lower_fills, lower_solids, spacing_mm, expansion_multiplier)` and `qualify_internal_bridge_surface(surface, unsupported, spacing_mm, nofilter)` returning the clipped bridge polygon or None (LANDED, Step 1 green) — then a prepass pass in `slice_postprocess_prepass.rs` that iterates region timelines, qualifies `top_solid_fill` surfaces against committed L-1 with flow/spacing resolved via `region_map.config_for` (`bridge_line_width`, `internal_bridge_flow`, `nozzle_diameter`, `internal_bridge_angle`, `dont_filter_internal_bridges`), constructs anchored lines via `construct_anchored_polygon`, writes net-new `SlicedRegion.internal_bridge_lines: Vec<Vec<Point2>>`, and extends `region.bridge_areas`. Reduce the InfillPostProcess arm to the pure `ExtrusionPath3D` emitter over that field.
- Exact files:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` — the two pure functions (+ private closing/shrink/grow helpers). DONE in Step 1.
  - `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (net-new) + `crates/slicer-core/Cargo.toml` [[test]] entry with `required-features = ["host-algos"]`. DONE in Step 1.
  - `crates/slicer-ir/src/slice_ir.rs` — ONE field on `SlicedRegion` (~1477-1530; derives Default): `pub internal_bridge_lines: Vec<Vec<Point2>>`, host-only, un-mirrored.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — the relocated pass + update of the ONE production exhaustive `SlicedRegion` literal (`crates/slicer-core/src/algos/prepass_slice.rs:1085`, inside `execute_prepass_slice_single_layer_impl`) to carry the new field explicitly (production literals stay exhaustive per docs/21).
  - `crates/slicer-runtime/src/layer_executor.rs` — reduce the InfillPostProcess arm to the emitter; delete anchor-gathering, candidate_voids, `construct_anchored_polygon`/`determine_bridging_angle` calls, the sliver/dont-filter gate logic, and the post-hoc sparse subtraction.
  - Test literals broken by the field: `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` :746/:856/:1088 → convert to `..Default::default()` FRU (docs/21 gate requires it anyway); sweep any others via `cargo check --workspace --all-targets`.
  - `resources/calicat.stl` (imported binary fixture) + `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (net-new; registered through the e2e aggregator main.rs).
- Rejected alternatives:
  - Gating in place at InfillPostProcess using only current-layer data — rejected twice over: layers run under `rayon::par_iter` with private arenas, so lower-layer committed data is legally unreachable from any stage arm; any current-layer proxy diverges from canonical (the measured flood IS that divergence).
  - Populating `InfillIR` from the prepass — impossible: `InfillIR` does not exist until the per-layer phase (`arena.set_infill` only in Layer arms); `blackboard.infill` is empty throughout PrePass.
  - Prepass subtraction from `sparse_infill_area` — overwritten by `region_partition.rs` at Perimeters commit; the lever is `bridge_areas`.
  - A dedicated dense-fill IR classification (full `stInternalSolid` modelling) — deferred: `top_solid_fill` is the available dense-ceiling carrier; AC-5 arbitrates.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/bridge_over_infill.rs` - role: pure math port (Step 1, landed)
- `crates/slicer-ir/src/slice_ir.rs` - role: ONE host-only field on `SlicedRegion` (`internal_bridge_lines`)
- `crates/slicer-core/src/algos/prepass_slice.rs` - role: update the single exhaustive `SlicedRegion` literal for the new field
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: relocated sequential pass
- `crates/slicer-runtime/src/layer_executor.rs` - role: reduce old arm to the pure emitter
- `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (net-new, landed); `crates/slicer-core/Cargo.toml` ([[test]] entry, landed)
- `resources/calicat.stl` (imported); `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (net-new); `crates/slicer-runtime/tests/e2e/main.rs` (aggregator registration)
- Test-literal FRU fixes where the new field breaks exhaustive literals (e.g., `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`)

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - lines ~1486-1535 and ~1720-1735 only - purpose: SliceRegion field inventory (`bottom_solid_fill`, `sparse_infill_area`, `bridge_areas`, `internal_bridge_infill`) and the region-view mirror.
- `docs/specs/bridge-parity-plan.md` - §3/F3 and §6 invariant list only.
- `tmp/cmp_after.log`, `tmp/cmp_dontfilter.log` - authoring-session measurement evidence (session-local; do not commit).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `modules/core-modules/**` - host-only relocation; if a module edit ever seems required, STOP and report
- `crates/slicer-schema/wit/**`, `crates/slicer-schema/**` (no WIT/view changes — `SliceRegionView::from_ir` must NOT copy the new field), `crates/slicer-ir/src/slice_ir.rs` beyond the single `SlicedRegion` field addition (no other IR edits, no `InfillRegion`/`InfillIR` changes)
- `target/`, `Cargo.lock`, generated code, vendored dependencies

## Expected Sub-Agent Dispatches

- Question: exact arithmetic of the gather lambda in canonical `PrintObject.cpp::bridge_over_infill` (closing radius source, shrink/grow order, solid-expansion constants, leftover thresholds); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: SNIPPETS ≤30 lines; purpose: Step-1 fidelity.
- Question: who writes `SliceRegion::bottom_solid_fill` and under what condition (is it the internal-solid interface?); scope: `crates/slicer-runtime/src/**` + `modules/core-modules/rectilinear-infill/src/lib.rs` writers; return: LOCATIONS ≤12; purpose: Q1 resolution.
- Question: do sparse-infill extrusion PATHS regenerate after ShellClassification commits, or are they final at Slice-stage dispatch? scope: gcode-emission consumers of `sparse_infill` paths + marshal ordering; return: LOCATIONS ≤12; purpose: Q2 mechanism decision (area-subtract vs path-replace).

## Data and Contract Notes

- IR/manifest contracts: unchanged. `InfillRegion.internal_bridge_infill` keeps type/role from 233.
- Determinism: the prepass runs sequentially over sorted timelines with total-order tie-breaks inherited from `determine_bridging_angle`; AC-5 double-slice byte-identity guards this.
- Watched types: literals constructing `SliceRegion`/`BridgeRegion` in new tests need `..` rest or `// exhaustive:` waiver per docs/21; `cargo xtask check-literals` is a listed gate.

## Locked Assumptions and Invariants

- I2 (no flooding): AC-5's frozen bar (≤6 internal-bridge layers on calicat; ≤5000 mm total).
- I3/I7 external surfaces untouched: AC-5 Z≈3.2 angle window [85°,95°] + uniform-feedrate expectations carried by existing suites.
- 234's gate precedes support gating within ShellClassification; the support pass also runs AFTER the shell-classification passes (`top_solid_fill` must be populated for every layer before qualification — Step 2 verifies ordering; if violated, STOP and report).
- The new `SlicedRegion.internal_bridge_lines` field is host-only: never copied into `SliceRegionView`, never surfaced to modules; `bridge_areas` extension is the only module-visible signal.
- Frozen baselines from the authoring session (2026-08-24): baseline-after-series 148 layers / 86675.76 mm / ratio 91.184; probe proved flag inertness. These numbers motivate but do not constrain beyond AC-5's stated bar.

## Risks and Tradeoffs

- Wedge e2e assertions (print_z 28.2 slot-ceiling) may have been calibrated against the UNFILTERED internal-bridge behaviour; if AC-6 fails after relocation, STOP and report — assertion re-pins are a post-packet decision with measured justification, never an in-step edit.
- `no_linker_module_degraded_raw_output_tdd` threshold 28.0 was calibrated today against flood-era output; recalibrate again ONLY with freshly measured both-sides numbers documented in-comment.
- If `top_solid_fill` turns out to be populated only after the prepass (ordering check in Step 2), or calicat gates to ZERO qualifying sites, the packet STOPS and reports with measurements — candidate re-picks are post-packet decisions, never an in-step edit.
- Adding the `SlicedRegion` field breaks exhaustive literals: exactly one production site known (`crates/slicer-core/src/algos/prepass_slice.rs:1085` — extend explicitly, production literals stay exhaustive) plus test literals (`crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` :746/:856/:1088 → FRU); sweep with `cargo check --workspace --all-targets` and `cargo xtask check-literals`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1 math fidelity; Step 2 relocation mechanics)

## Open Questions

- `[FWD→Step 1]` Q1 candidate-field resolution: **ANSWERED (2026-08-24):** `bottom_solid_fill` ruled out by writer evidence (exposed-bottom shell — `apply_opening(difference(region_polys, lower_polys))`, the opposite of an internal interface). Candidate = upper-layer `top_solid_fill`; AC-5's frozen calicat bar arbitrates; zero qualifying sites on calicat = STOP-and-report with measurements. **MEASURED OUTCOME (implementation, 2026-08-24):** the relocated pass runs and finds candidates (18 top-fill layer-visits on calicat's 174), but canonical gates reject all of them — skip histogram: 156× `top_solid_fill_empty`, 9× `unsupported_empty`, 9× `qualified_empty`; qualifying sites = **0** vs canonical's ~1 near Z≈29.4. AC-5's frozen bars all pass (0 ≤ 6 layers; ~0 mm ≤ 5000 mm; byte-identity; external row exactly 90.0°/74 segs/324.6 mm). Interpretation: our IR has no dense-interior (`stInternalSolid`) surface taxonomy, so `top_solid_fill` is a narrower candidate than canonical's; under-detection (not flooding) is the residual divergence. Recorded as a known deviation; coverage/anchoring parity follow-up stays under ISSUE-82. No silent candidate re-pick was performed. Corroborating artifact: the NEG-2 byte-golden (`precision_legacy_20mmbox.gcode`) contained 94 flood-era `Internal Bridge` sections from 233's seam; post-relocation it contains 0 (all other section counts identical, 100-layer Z-set unchanged) — re-blessed 2026-08-24 as canonical-correct drift per Test Discipline.
- `[FWD→Step 1]` Q2 sparse-path regeneration timing: **ANSWERED (2026-08-24), superseded by a sharper fact:** sparse-infill PATHS are authored at per-layer `Layer::Infill` into `InfillIR` and never regenerate; but the decisive discovery is that `sparse_infill_area` itself is DERIVED at Perimeters commit (`region_partition.rs`: `difference(wall_inset, bridge ∪ bottom ∪ top)`) AFTER the prepass — so material exclusion flows through extended `bridge_areas`, never through prepass sparse mutation. Today's arm subtraction at `layer_executor.rs` L2439-2440 is post-hoc dead code.
- `[RESOLVED 2026-08-24, Option A]` Former stage-boundary blocker: the committed infill artifact is NOT reachable at ShellClassification (`InfillIR` authored only in the rayon-parallel per-layer phase with private arenas; cross-layer arena reads forbidden). Resolution approved by the user: net-new host-only `SlicedRegion.internal_bridge_lines: Vec<Vec<Point2>>` carrier field written by the prepass pass, consumed by the same-layer reduced InfillPostProcess arm. No WIT/SDK/mirror changes required (`SliceRegionView::from_ir` copies selected fields only).
- None `[BLOCK]`.
