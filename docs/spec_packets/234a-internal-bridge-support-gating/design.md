# Design: 234a-internal-bridge-support-gating

## Controlling Code Paths

- Current construction site (to be emptied): the `LayerStageCommit::InfillPostProcess` arm in `crates/slicer-runtime/src/layer_executor.rs` — anchors gathered from `region.sparse_infill` paths plus perimeter walls and inset contours; `candidate_voids = slicer_core::difference(&slice_region.sparse_infill_area, &slice_region.bridge_areas)`; sliver-guard filter keyed on `dont_filter_internal_bridges`; subtraction from `sparse_infill_area`; population of `region.internal_bridge_infill` with `ExtrusionRole::InternalBridgeInfill` paths.
- New home (this packet): inside `commit_shell_classification_builtin`'s stage in `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, strictly AFTER the invocation of 234's `gate_bridge_areas_by_unsupported_span`, walking `build_region_timelines(slices: &[SliceIR])` so each region's layer L is qualified against committed layer L-1. The stage already holds every committed `SliceIR` (guarded by `ShellClassificationError::SliceIRNotCommitted`) — the same multi-layer access 234's gate and 235's orientation use. Field attribution: `internal_bridge_infill` is an `InfillRegion` field (`crates/slicer-ir/src/slice_ir.rs`), not `SliceRegion`; Step 2 must first confirm the committed infill artifact is reachable at ShellClassification (the executor commits it via `arena.set_infill(ir)`) and mirror the existing commit-key pattern when reading it.
- Pure math home: `crates/slicer-core/src/algos/bridge_over_infill.rs`, beside the existing `determine_bridging_angle` and `construct_anchored_polygon` (both stay unchanged in algorithm; the latter gains a caller at the prepass).
- Neighbouring tests: `crates/slicer-core/tests/bridge_over_infill_tdd.rs` (233's suite), `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (234's), `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` (wedge pins), `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs` (slot-ceiling assertions at print_z 28.2).

## Architecture Constraints

- **Placement supersession (explicit):** 233's AC-N2 recorded "prepass stays free of internal-bridge logic". This packet reverses that placement. Rationale: canonical qualification needs committed lower-layer geometry; layers slice in parallel so InfillPostProcess cannot legally read layer L-1 — the identical constraint that moved 234's false-site gate to ShellClassification. The reversal is a designed decision recorded here, not a silent deviation; no DEVIATION_LOG row is required because the original constraint was a design choice, not an invariant.
- **Candidate source:** candidates are the upper layer's internal-solid interface surfaces — NOT sparse infill. Our IR has no dedicated internal-solid field; Step 1 resolves which existing representation plays that role (leading suspect: `SliceRegion.bottom_solid_fill`, `crates/slicer-ir/src/slice_ir.rs` ~line 1510) by checking its writers and by gating calicat: the correct source must yield ~1 qualifying site near Z≈29.4 under canonical math. If no existing field qualifies, STOP and report — adding one is a scope change requiring re-approval.
- **Canonical arithmetic [port contract]:** `unsupported_area = closing(lower_fills, spacing) shrunk by expansion_multiplier*spacing` minus `(lower_solids shrunk 1*spacing) expanded (1+mult)*spacing`, with `expansion_multiplier = 3` default, `1` when the filter key selects limited filtering. Per surface `s`: `unsupported = intersection(s, unsupported_area)`; qualify iff non-empty AND (`area(unsupported) == area(s)` OR `area(unsupported) > 9*spacing^2`). Bridge polygon = `expand(unsupported, 4*spacing)` clipped to `s`; leftovers with `spacing^2 < area < 12*spacing^2` remerge into neighbours.
- **Config mapping:** `dont_filter_internal_bridges=false` → full gates (canonical `ibfDisabled` default); `true` → bypass area/partial gate (`ibfNofilter`). The multiplier parameter carries `ibfLimited`. Key string stays snake_case; manifest untouched.
- **Ordering:** support gating runs strictly after 234's false-site gate within the same stage; external-site orientation ordering from 235 is untouched.
- Schema/version constants: none touched. No new IR field, no WIT change, no scheduler stage addition.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add pure functions to `bridge_over_infill.rs` — `unsupported_span_areas(lower_fills, lower_solids, spacing_mm, expansion_multiplier)` and `qualify_internal_bridge_surface(surface, unsupported, spacing_mm, nofilter)` returning the clipped bridge polygon or None — then a prepass pass `gate_internal_bridge_sites(...)` (name-resolution-equivalent spellings accepted) in `slice_postprocess_prepass.rs` that iterates region timelines, applies qualification with flow values resolved from the same ConfigView the current arm uses (`bridge_line_width`, `internal_bridge_flow`, `nozzle_diameter`, `internal_bridge_angle`), constructs anchored lines via `construct_anchored_polygon`, populates `InfillRegion.internal_bridge_infill`, and performs whichever sparse-material removal mechanism Q2 dictates. Delete the InfillPostProcess construction block from `layer_executor.rs`.
- Exact files:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` — add the two pure functions (+ private closing/shrink/grow helpers).
  - `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (net-new) + `crates/slicer-core/Cargo.toml` [[test]] entry with `required-features = ["host-algos"]`.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — the relocated pass.
  - `crates/slicer-runtime/src/layer_executor.rs` — remove the old block (keep anchor-gathering ONLY if the prepass cannot reach perimeter data; otherwise delete wholesale — decide via Q2/Q1 evidence).
  - `resources/calicat.stl` (imported binary fixture) + `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (net-new; registered through the e2e aggregator main.rs).
- Rejected alternatives:
  - Gating in place at InfillPostProcess using only current-layer data — rejected: no legal lower-layer access under parallel slicing; any current-layer proxy diverges from canonical (the measured flood IS that divergence).
  - Adding a dedicated `internal_solid_fill` IR field — rejected for this packet: prefer resolving Q1 with existing fields; a field addition is a schema-touching change needing its own blast-radius pass.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/bridge_over_infill.rs` - role: pure math port
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: relocated sequential pass
- `crates/slicer-runtime/src/layer_executor.rs` - role: remove old arm
- `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (net-new); `crates/slicer-core/Cargo.toml` ([[test]] entry)
- `resources/calicat.stl` (imported); `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (net-new)

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - lines ~1486-1535 and ~1720-1735 only - purpose: SliceRegion field inventory (`bottom_solid_fill`, `sparse_infill_area`, `bridge_areas`, `internal_bridge_infill`) and the region-view mirror.
- `docs/specs/bridge-parity-plan.md` - §3/F3 and §6 invariant list only.
- `tmp/cmp_after.log`, `tmp/cmp_dontfilter.log` - authoring-session measurement evidence (session-local; do not commit).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `modules/core-modules/**` - host-only relocation; if a module edit ever seems required, STOP and report
- `crates/slicer-schema/wit/**`, `crates/slicer-ir/**` (no field additions)
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
- 234's gate precedes support gating within ShellClassification.
- Frozen baselines from the authoring session (2026-08-24): baseline-after-series 148 layers / 86675.76 mm / ratio 91.184; probe proved flag inertness. These numbers motivate but do not constrain beyond AC-5's stated bar.

## Risks and Tradeoffs

- Wedge e2e assertions (print_z 28.2 slot-ceiling) may have been calibrated against the UNFILTERED internal-bridge behaviour; if AC-6 fails after relocation, re-pin assertions to post-fix reality with justification comments — never weaken to green blindly.
- `no_linker_module_degraded_raw_output_tdd` threshold 28.0 was calibrated today against flood-era output; recalibrate again ONLY with freshly measured both-sides numbers documented in-comment.
- If Q1 resolves to "no existing field represents internal-solid interfaces", the packet STOPS before Step 2 edits and reports — candidate-source invention is out of bounds here.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1 math fidelity; Step 2 relocation mechanics)

## Open Questions

- `[FWD→Step 1]` Q1 candidate-field resolution (bottom_solid_fill vs alternative): answered by the LOCATIONS dispatch above; recorded here before Step 2 edits.
- `[FWD→Step 1]` Q2 sparse-path regeneration timing: answered by the LOCATIONS dispatch above; determines subtraction vs path-replacement mechanism; recorded here before Step 2 edits.
- None `[BLOCK]`.
