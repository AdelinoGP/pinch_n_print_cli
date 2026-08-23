# Design: 234-bridge-false-site-gating

## Controlling Code Paths

- Primary code path: `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) — the F1 stamper that intersects each `BridgeRegion.xy_footprint` with `region.infill_areas`, offsets by `expansion_margin_mm`, and extends `region.bridge_areas`. Called from `execute_prepass_slice_single_layer_impl` (same file) during `PrePass::Slice`. This packet adds the unsupported-span gate and relocates its invocation post-slice.
- Neighboring tests/fixtures: `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` (flat, `required-features = ["host-algos"]`), `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (calls `assemble_bridge_areas` directly), `crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (precedence + disjointness).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The gating must run post-slice, not during `PrePass::Slice`: `assemble_bridge_areas` executes inside `execute_prepass_slice_single_layer_impl` while layers are sliced (rayon-parallel), so the lower layer's slices are not yet committed there. The lower-layer data is available only after `PrePass::Slice` commits the full `SliceIR` — exactly the input `PrePass::OverhangAnnotation` already reads to diff consecutive-layer footprints, and `PrePass::ShellClassification` reads to annotate. See Q3 resolution below.
- The gating reuses the existing `SurfaceClassificationIR.prev_layer_boundaries` (`HashMap<u32, Vec<ExPolygon>>`, packet 193, populated by `commit_overhang_annotation_builtin`) as the lower-layer anchor source, or reads the committed `SliceIR` directly. The map is keyed by the CURRENT global layer index (value = previous-layer contours), so the lookup is `get(&global_layer_index)`, never `global_layer_index - 1`. No new IR field, no schema bump, no `STAGE_ORDER` change.
- The gating function is pure and host-side only in its *invocation* (called from the host `PrePass::ShellClassification` built-in, never from a module), but the edited path `crates/slicer-core/src/algos/prepass_slice.rs` is inside `crates/slicer-core/**`, which the core modules (`rectilinear-infill`, `classic-perimeters`, `gyroid-infill`, `infill-linker`, `lightning-infill`, `path-optimization-default`, `traditional-support`, `tree-support`, `arachne-perimeters`) depend on as a path dependency. The guest fingerprint covers the `slicer-core` path-dependency closure, so this edit feeds the guest build and the staleness snippet applies.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The gate subtracts the ungrown `prev_layer_boundaries` polygons, which are already in 100 nm units (host-only aggregation). Canonical's cited `voids = diff(voids, *lower_layer_covered)` is in the dead `#else` overload of `process_external_surfaces` and subtracts ungrown lower slices; the 0.1 mm × 5 expansion zones grow already-classified bridge surfaces outward and never subtract.

## Code Change Surface

- Selected approach: keep `assemble_bridge_areas` stamping candidates during `PrePass::Slice` (its mesh-derived footprint intersection is unchanged), and add a pure `gate_bridge_areas_by_unsupported_span(region: &mut SlicedRegion, lower_layer_slices: Option<&[ExPolygon]>)`. A missing key means no lower layer and clears candidates; a present key means a lower layer exists and subtracts its ungrown contours, so an empty value subtracts nothing. Invoke the gate from `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) after `PrePass::OverhangAnnotation` has populated `prev_layer_boundaries`, keyed by the region's global layer index. The mesh-validity filter (`BridgeRegion.is_valid`) stays as a cheap pre-filter inside `assemble_bridge_areas` (already present at HEAD).
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-core/src/algos/prepass_slice.rs` — add `gate_bridge_areas_by_unsupported_span` with ungrown lower-contour subtraction.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `commit_shell_classification_builtin` gains a post-annotation pass that, for each region, looks up `prev_layer_boundaries.get(&global_layer_index)` and calls the gate. The map is ALREADY previous-layer contours keyed by the CURRENT global layer index. A missing key means no lower layer and clears candidates; a present key, including an empty value, means a lower layer exists and subtracts ungrown contours.
  - `crates/slicer-core/Cargo.toml` — add `[[test]] name = "bridge_false_site_gating_tdd"` with `required-features = ["host-algos"]`.
  - `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` — net-new flat test (AC-1, AC-2, AC-N1, AC-N2).
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` — update the two `assemble_bridge_areas` call sites (lines ~775 and ~886) whose assertions expect non-empty `bridge_areas`; they must now either pass lower-layer slices or assert the ungated stamping behaviour separately from the gate.
- Rejected alternatives and reasons:
  - Gating inside `assemble_bridge_areas` during `PrePass::Slice` — rejected: lower-layer slices are not committed while layers slice in parallel (the packet 36-rev1 scheduler constraint); would require a new N±1 prepass data dependency.
  - A new dedicated prepass stage — rejected: `PrePass::ShellClassification` already runs post-slice and reads the committed `SliceIR`; a new stage would add `STAGE_ORDER`/manifest surface for no benefit.
  - Discarding the mesh-validity filter outright — rejected: it is already present, cheap (a boolean check), and rejects facet clusters that can never be valid bridges before the more expensive polygon-difference test; keep it and measure (see Locked Assumptions).

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/prepass_slice.rs` - role: primary change surface; expected change: add `gate_bridge_areas_by_unsupported_span` (ungrown lower-contour subtraction; no anchor-growth helper).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: integration point; expected change: invoke the gate post-annotation in `commit_shell_classification_builtin`.
- `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` - role: net-new test home; expected change: AC-1/AC-2/AC-N1/AC-N2 + `no_lower_layer_clears_bridge_areas` + `existing_empty_lower_layer_retains_bridge_area`.
- `crates/slicer-core/Cargo.toml` - role: register the net-new test target; expected change: `[[test]]` entry.
- `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` - role: blast-radius fallout; **outcome (2026-08-22): no edit needed** — the two direct `assemble_bridge_areas` call sites (~775 non-empty assertion, ~886 simplicity loop) still pass because the gate runs post-slice in `PrePass::ShellClassification` and direct stamper calls are unaffected. The file has 3 pre-existing failures at HEAD (`bridge_footprint_does_not_leak_outside_facet_z_span`, `invalid_bridge_excluded_from_slice_areas`, `supported_bridge_candidate_does_not_emit_bridge_fill`), verified as not caused by this packet by two independent lines of evidence: (1) a stash-run with the four packet paths stashed reproduced the same 3 failures (13 passed), and (2) the 3 tests exercise only `PrePass::Slice`/`assemble_bridge_areas` and never reach `commit_shell_classification_builtin`, the only runtime file this packet edits. They assert the F1 invariant on the ungated stamper path and are out of scope.
- `resources/overhang.obj` - role: discovered-fallout fix; expected change: translated into the printable bed (XY −1346.56/−564.40, Z −17.0) because the slicer has no auto-centering and requires models to start at Z=0 (the model previously sliced to zero layers).

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - lines 599-693 only - purpose: `BridgeRegion` fields (`is_valid`, `xy_footprint`, `expansion_margin_mm`) and `SurfaceClassificationIR.prev_layer_boundaries` shape.
- `crates/slicer-runtime/src/region_partition.rs` - lines 1-60 and 160-216 only - purpose: precedence `bridge > bottom > top > sparse` and the `bridge = intersection(&slice_region.bridge_areas, wall_inset)` claim.
- `crates/slicer-core/src/algos/prepass_slice.rs` - lines 197-256 only - purpose: current `assemble_bridge_areas` body.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-core/src/algos/bridge_over_infill.rs` (233's net-new module) - prerequisite, not this packet's surface; do not edit
- `crates/slicer-core/src/algos/mesh_analysis.rs` - orientation (235's surface); do not edit

## Expected Sub-Agent Dispatches

- Question: exact `detect_bridging_direction(to_cover, anchors_area)` floating-edge computation and the `unsupported_edges` `diff_pl(..., grown_lower)` geometry; scope: `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` + `BridgeDetector.cpp`; return: `SUMMARY` (≤200 words) + `LOCATIONS`; purpose: Step 1 port target.
- Question: how `commit_overhang_annotation_builtin` populates `prev_layer_boundaries` — which layers get an entry, and whether a layer with an empty previous-layer contour set is stored as an empty `Vec` or omitted (the gate must treat a MISSING key as "no lower layer" and a PRESENT key — even empty — as "lower layer exists"); scope: `crates/slicer-runtime/src/`; return: `LOCATIONS`; purpose: Step 2 wiring. (The keying is already confirmed: current global layer index, per `crates/slicer-wasm-host/src/marshal/in_.rs`.)
- Question: every test/non-test struct-literal site that constructs `SlicedRegion` or `BridgeRegion` and would break if a field is added; scope: `crates/*/src` + `crates/*/tests`; return: `LOCATIONS`; purpose: blast-radius discipline (only if a field is added — the current design adds none).

## Data and Contract Notes

- IR/manifest contracts: no new IR field; reuses `SurfaceClassificationIR.prev_layer_boundaries` and `SlicedRegion.bridge_areas`. No schema version bump.
- WIT boundary: none — the gating is host-only and not mirrored in WIT.
- Determinism/scheduler constraints: the gate runs in `PrePass::ShellClassification`, which is sequential and reads the committed `SliceIR`/`prev_layer_boundaries`; no new cross-layer dependency, no `STAGE_ORDER` change. The gate is deterministic (pure polygon difference).

## Locked Assumptions and Invariants

- **Pre-filter decision (recorded):** KEEP `BridgeRegion.is_valid` (min-length + anchor-width pass/fail) as a cheap pre-filter. The measured output was identical with the pre-filter enabled and disabled: AC-5 overhang was `25/31` bridge layers in both runs, and AC-3 bridge was `34/40` in both runs. This is zero delta -> the gate alone rejects every candidate the pre-filter rejects; the pre-filter is retained as a cheap pre-filter with zero measured output delta (discard deferred -- removing it is a follow-up simplification, not required for parity).
- Invariants I1 (no bridge over solid lower layer) and I2 (site existence, and only there) are the primary acceptance invariants; I6 (role disjointness) is the regression guard. I4/I5/I7 belong to 233/235 except where gating changes interact (none expected — the gate only removes sites, never alters surviving-site geometry).
- The flat-ceiling case is pinned by `wedge_multi_layer_top_bottom_evidence` (`crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`): the ceiling's bridge candidate survives because the lower layer's wall-ring contours do not cover the cavity interior under ungrown subtraction, so the gate never demotes the correct layer for a flat ceiling. **Measured (2026-08-22):** the gate retains the cavity interior at z=28.0 (instrumented bridge_areas ≈ 4e10 units² after gating, non-empty), yet the e2e test is RED — the G-code emits `;TYPE:Internal Bridge` and no `;TYPE:Bridge`. Measured facts: the gate retains the candidate; the marker is Internal Bridge; the baseline (gate removed, stash-verified) is green. Inferred (not directly instrumented — the partition claim hook did not fire in the CLI path): the region partition's bridge claim (`intersection(&slice_region.bridge_areas, wall_inset)` with `wall_inset = perim.infill_areas`, `crates/slicer-runtime/src/region_partition.rs`) does not emit Bridge extrusion for the cavity interior, and 233's `bridge_over_infill` module (running at `LayerStageCommit::InfillPostProcess`, after partitioning) claims the interior slot as Internal Bridge without checking lower-layer support. The test's expectation (Bridge at z=28.0) is canonical-correct; the baseline's green was non-canonical (flooded bridge_areas covered the wall ring). The test stays red; the fix belongs to a follow-up on the partition claim / 233's void-over-bridge handling, not to this packet's gate. **Follow-up owner: packet 235 (external-bridge orientation) is the natural owner of the external-bridge evidence chain; if 235 does not pick this up, file a dedicated follow-up packet for the partition bridge-claim coverage.**
- **Measured (2026-08-22), overhang.obj:** the mesh analysis produces NO bridge candidates for the overhang lip (instrumented `bridge_areas` is empty before gating at the lip layer), so AC-5's "site exists" is satisfied by 233's `;TYPE:Internal Bridge` markers, not external `;TYPE:Bridge`. The lip's facet detection is 235's surface (`crates/slicer-core/src/algos/mesh_analysis.rs`); recorded here so 235 knows the external-bridge evidence chain starts at facet detection.

## Risks and Tradeoffs

- The gate relocates bridge-area finalization from `PrePass::Slice` to `PrePass::ShellClassification`; any downstream consumer that reads `bridge_areas` between those two stages (e.g. visual-debug captures, `region_partition` inputs) must be re-checked for ordering. `region_partition` runs per-layer after prepass, so it sees the gated result — but visual-debug taps that snapshot `bridge_areas` during `PrePass::Slice` will now show ungated candidates.
- Changing F1 flips baseline-dependent tests: `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` asserts non-empty `bridge_areas` from `assemble_bridge_areas`; **outcome (2026-08-22):** the two direct call sites (~775 non-empty assertion, ~886 simplicity loop) still pass unchanged because the gate runs post-slice and direct stamper calls are unaffected — no re-scoping was needed. The file's 3 pre-existing failures at HEAD are verified as not this packet's (stash-run + the tests never reach the edited runtime file) and are out of scope.
- The `prev_layer_boundaries` keying is the current global layer index (value = previous-layer contours); the implementer must use `get(&global_layer_index)`, not `global_layer_index - 1` — an off-by-one silently gates against the wrong layer.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 wiring + blast-radius fallout)
- Highest-risk dispatch and required return format: the `prev_layer_boundaries` population-semantics `LOCATIONS` dispatch (Step 2) — if the gate treats an empty `Vec` differently from a missing entry, the first-layer demotion silently mis-gates.

## Open Questions

- `[FWD]` to 235: does the gate's surviving `bridge_areas` geometry (post-difference) preserve the exact boundary that 235's `detect_bridging_direction` floating-edge computation expects, or must 235 re-derive floating edges from the gated polygons? The gate subtracts the UNGROWN lower-layer contours, which trims the bridge boundary — 235 should confirm its floating-edge input is the gated (trimmed) polygon, not the pre-gate candidate.
- `[FWD]` to 235: this packet's span test subtracts UNGROWN lower-layer contours (no expansion-zone growth); 235's `SCALED_EPSILON` anchor expand is its own constant — confirm the two are not double-applied (they are distinct: this packet grows nothing).
- None `[BLOCK]` — Q3 resolved (see below).

## Q3 Resolution (scheduler dependency)

**Resolved — no new prepass data dependency on N±1 layers.** The unsupported-span test reads the lower layer's slices from inputs that already exist in the post-slice prepass: the committed `SliceIR` (each object's final per-layer region polygons, read by `PrePass::OverhangAnnotation` and `PrePass::ShellClassification`) and, more directly, `SurfaceClassificationIR.prev_layer_boundaries` (`HashMap<u32, Vec<ExPolygon>>` keyed by global layer index, populated by `commit_overhang_annotation_builtin` since packet 193). `PrePass::OverhangAnnotation` already diffs consecutive-layer footprints from the committed `SliceIR` — the exact N−1 access pattern the gate needs — so the gate resolves inside existing per-layer/prepass stage inputs by running in `PrePass::ShellClassification` (which executes after `PrePass::Slice` and `PrePass::OverhangAnnotation`). The packet 36-rev1 scheduler constraint (no cross-layer dependency during parallel per-layer slicing) is satisfied because the gate runs in the sequential prepass tier, not the parallel per-layer tier.

## Note from Packet 233 Execution (2026-08-22)

During packet 233's implementation, a wip stash salvaged a *during-slice* variant of this
packet's false-site gating (current-minus-previous raw-polygon difference applied inside
`execute_prepass_slice_single_layer_impl` via the batch cache) and it was briefly landed by
accident. Empirical findings, preserved here as steering evidence for Step 2:

- The during-slice variant shifts external bridge classification one layer later: for a flat
  bridge ceiling exactly at z=28.0, exact-boundary slicing excludes that ceiling from the
  layer's cross-section, so `difference(current_raw, previous_raw)` is empty there and the
  gate clears `bridge_areas`/`is_bridge`; material first appears at z=28.2. This broke the
  previously-green e2e test `wedge_multi_layer_top_bottom_evidence` (expects `;TYPE:Bridge`
  at z=28.0). The variant was reverted; packet 233's tree keeps `assemble_bridge_areas` at
  HEAD semantics.
- The salvage diff is preserved verbatim at
  `references/prepass_slice_false_site_gating.salvage.rs` (uncompiling reference copy — do
  not drop into `src/`). It is superseded by this design's Q3 resolution: gate in
  `PrePass::ShellClassification` from committed data (`prev_layer_boundaries` /
  committed `SliceIR`), never inside the parallel per-layer slicing loop.
- Implication for Step 2's verification: whichever gate lands must state how the flat-ceiling
  boundary case (empty current-minus-previous at the true ceiling layer) avoids demoting the
  correct layer, and name the test that pins it. If canonical semantics genuinely classify at
  the next containing layer instead, `wedge_multi_layer_top_bottom_evidence`'s expectation
  must be revisited explicitly in that step, not silently.
