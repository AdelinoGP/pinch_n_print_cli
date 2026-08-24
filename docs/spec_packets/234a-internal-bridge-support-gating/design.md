# Design: 234a-internal-bridge-support-gating (closure edition)

## Controlling Code Paths

- Primary math: `unsupported_span_areas`, `qualify_internal_bridge_surface`,
  `construct_anchored_polygon`, `determine_bridging_angle` in
  `crates/slicer-core/src/algos/bridge_over_infill.rs`; `fill_envelope` (bbox complement)
  is the RC-A defect and leaves the dataflow.
- Qualification: successor of `gate_internal_bridge_sites` in
  `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, still ordered AFTER the
  shell-classification passes and strictly after 234's `gate_bridge_areas_by_unsupported_span`;
  candidates now come from net-new `SlicedRegion.internal_solid_fill`.
- Construction: `LayerStageCommit::InfillPostProcess` arm in
  `crates/slicer-runtime/src/layer_executor.rs` becomes the anchored constructor; today it is
  a pure emitter over `internal_bridge_lines`, which this packet retires.
- Classification authoring: `compute_region_updates` (same prepass file) gains the
  band-minus-exposed-seed derivation writing `internal_solid_fill` beside its existing
  `top_solid_fill` Pass-1/Pass-2 machinery.
- Neighbouring tests/fixtures:
  `crates/slicer-core/tests/bridge_support_gating_tdd.rs`,
  `crates/slicer-runtime/tests/integration/region_partition_tdd.rs`,
  `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`,
  `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs`,
  `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` (NEG-2 golden),
  `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`,
  e2e aggregator `main.rs` registrations,
  golden `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`.

## Architecture Constraints

- **Venue split legality:** per-layer stage arms run under rayon with private arenas;
  cross-layer reads are forbidden outside the sequential prepass. Qualification (needs L-1
  committed state) therefore stays prepass; construction needs only same-layer committed
  state (`internal_bridge_areas` + walls + sparse polylines) so it may move to
  InfillPostProcess. S4 opens with an explicit reachability probe; failure = STOP-and-report.
- **Ordering lock:** support qualification runs strictly after 234's false-site gate within
  ShellClassification; external-bridge orientation from 235 is untouched; partition's
  `sparse_infill_area = difference(wall_inset, bridge ∪ bottom ∪ top)` continues consuming the
  extended `bridge_areas` unchanged.
- **Per-region config resolution:** density branch resolves `infill_density` via EACH lower
  region's own `RegionKey` through `region_map.config_for(...)` — a different resolution site
  than the current first-entry-per-timeline flow lookup; keys stay snake_case.
- **Visibility contract:** `internal_solid_fill` is WIT-MIRRORED (future-proofing for infill
  modules); `internal_bridge_areas` is host-only/un-mirrored but auto-serializes into
  visual-debug bundles because `SliceIR` derives Serialize; both new fields carry
  `#[serde(default)]`; `internal_bridge_lines` disappears tree-wide in S4.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Any ported C++ function (S5: expansion zones, `gather_areas_w_depth`, clustering) carries
  the standard header from `docs/ORCASLICER_ATTRIBUTION.md` at the top of the new file/section.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): every watched-type
  literal touched by S2/S4 uses a `..` rest or an `// exhaustive:` waiver; production literals
  stay exhaustive; `cargo xtask check-literals` runs inside each affected step.

## Code Change Surface

- Selected approach: six-step closure (see implementation-plan). Math initialization flips to
  fills-as-initial; dense-interior becomes first-class WIT-visible data authored where the
  band is already computed; qualification persists polygons instead of centerlines; the
  per-layer arm regains construction fed by real anchors; F4 machinery ports onto that venue;
  arbitration moves to persisted-bundle assertions with gcode as secondary.
- Exact functions/types touched:
  - `unsupported_span_areas` (rewrite init; drop `fill_envelope`),
    `qualify_internal_bridge_surface` (unchanged math),
    `construct_anchored_polygon` (gains expansion-zone parameters in S5a),
    `determine_bridging_angle` (unchanged),
    net-new pure helpers for expansion/harvesting/clustering (S5).
  - `compute_region_updates`: author `internal_solid_fill` (band minus exposed seed; seed =
    opening(difference(region_polys, upper_polys)) reusing Pass-1 geometry).
  - `gate_internal_bridge_sites` successor: candidates from `internal_solid_fill`;
    `lower_fills` from `infill_areas`; per-lower-region solids incl. `infill_density >= 0.999`
    branch; writes `internal_bridge_areas` + extends `bridge_areas`; print_z-keyed skip logs.
  - InfillPostProcess arm: anchor harvest (perimeter paths + Layer::Infill polylines),
    angle selection, strip construction, `ExtrusionRole::InternalBridgeInfill` emission.
  - `SlicedRegion`: add `internal_solid_fill`, add `internal_bridge_areas`, remove
    `internal_bridge_lines`; `SliceRegionView::from_ir` mirrors ONLY `internal_solid_fill`;
    canonical WIT region type gains `internal_solid_fill` only.
  - Renderer: `slice_shapes` arm + `geometry_points_mm` viewport push for BOTH new fields
    (bundle JSON needs no arm; PNG rendering does).
  - Module alignment: `modules/core-modules/rectilinear-infill/src/lib.rs`
    `enable_extra_bridge_layer` semantics matched to canonical extra-layer behaviour.
- Rejected alternatives (with reasons, from the frozen brief): arithmetic-first-gated
  taxonomy (both land unconditionally by decision); prepass-local transient classification
  (invisible downstream, repeats log-scraping pain); host-only-only field (fails stated
  future-proofing goal); slice/partition-stage authoring (illegal under arena rules);
  whole-band candidates (qualifies visible ceilings; canonical excludes stTop);
  keep-`region.polygons` fills (outward wall-band bias); shells-only solids
  (under-subtraction bias); all-minus-sparse solids (suppresses legitimate sites);
  all-construction-in-prepass (anchors stay stand-ins forever); gcode-only arbitration
  (weaker attribution; retained as secondary).

## Files in Scope (read + edit)

Multi-crate contract change; extras beyond 3 primary are intrinsic and accepted.

- `crates/slicer-core/src/algos/bridge_over_infill.rs` - role: S1 init rewrite + S5 helpers
- `crates/slicer-core/tests/bridge_support_gating_tdd.rs` - role: AC-1 fixtures re-bless
- `crates/slicer-ir/src/slice_ir.rs` - role: two fields added, one removed (S2/S3/S4)
- `crates/slicer-schema/wit/**` - role: WIT region type gains `internal_solid_fill` (S2)
- `crates/slicer-sdk/src/views.rs` - role: mirror `internal_solid_fill` only (S2)
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: classification authoring +
  qualification rewrite (S2/S3)
- `crates/slicer-runtime/src/layer_executor.rs` - role: constructor relocation (S4)
- `crates/slicer-runtime/src/visual_debug_render.rs` - role: render arms (S2)
- `crates/slicer-core/src/algos/prepass_slice.rs` - role: exhaustive production literal (S2/S4)
- `modules/core-modules/rectilinear-infill/src/lib.rs` - role: extra-layer semantics (S5b)
- tests listed under Controlling Code Paths + net-new e2e files + bucket aggregator mains

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - `SlicedRegion` declaration block and
  `InfillRegion` only (file >300 lines; locate by symbol, ranged reads)
- `crates/slicer-runtime/src/layer_executor.rs` - `LayerStageCommit::InfillPostProcess` arm
  plus `SUPPORTED_TAP_STAGE_IDS` order constant only (ranged reads)
- `docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md` - full read;
  authoritative decisions/approvals
- `docs/specs/bridge-parity-plan.md` - sections F3/F4 only

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- All `modules/**` EXCEPT the scoped `rectilinear-infill` extra-layer alignment; any wider
  module need = STOP-and-report
- Other packet directories under `docs/spec_packets/**`
- `target/`, `Cargo.lock`, generated code, vendored dependencies
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: exact arithmetic + verbatim snippets of canonical expansion-zone application and
  `gather_areas_w_depth` (constants `expansion_step`, `expansion_bottom_bridge`, closing
  radius source); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return:
  SNIPPETS ≤3x30 lines; purpose: S5 fidelity.
- Question: which struct-literal sites compile against `SlicedRegion` today (production +
  test) and which hard-assert on `internal_bridge_lines`; scope: `crates/**`; return:
  LOCATIONS ≤20; purpose: S2/S4 blast radius (re-derive at step time; never trust stale pins).
- Question: are perimeter-wall paths and Layer::Infill polylines reachable from the
  InfillPostProcess commit arm (name the payload/accessor symbols); scope:
  `crates/slicer-runtime/src/layer_executor.rs` + `crates/slicer-ir/src/stage_io.rs`; return:
  FACT ≤5 lines; purpose: S4 feasibility probe.
- Question: thread-cluster + filled-polygons-on-lower-layers removal semantics; scope:
  `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: SUMMARY ≤200 words; purpose: S5b.

## Data and Contract Notes

- IR/manifest contracts: `InfillRegion.internal_bridge_infill` keeps type/role from 233; no
  manifest changes; no config-key additions (`infill_density` already resolved).
- WIT boundary: one added field on the region type (`internal_solid_fill`); run the WIT-change
  checklist (search `wit_host.rs`/dispatch/guest consumers; verify type identity across the
  component boundary; `cargo build --tests` after edits).
- Determinism/scheduler: prepass sequential over sorted timelines; construction deterministic
  given committed anchors; AC-6 byte-identity is the guard.

## Locked Assumptions and Invariants

- Arbiter bar frozen: EXACTLY ONE Internal-Bridge layer, print_z ∈ [29.15, 29.75], extruded
  length ∈ [300, 700] mm, zero elsewhere; external Bridge row @ Z≈3.2 keeps [85°, 95°].
- Multiplier mapping: `dont_filter_internal_bridges == ibfDisabled` ⇒ 3, else 1.
- Dense definition locked: shell band MINUS depth-0 exposed seed; threshold fraction >= 0.999.
- Golden policy locked: conditional re-bless with section-count diff table, Z-set identity,
  per-diff-class canonical reasoning; zero-sites not privileged; wedge suites pass unmodified
  or STOP.
- Bundle-JSON primary / gcode secondary instrumentation split.

## Risks and Tradeoffs

- Wall/polyline reachability in the InfillPostProcess arm is UNVERIFIED until the S4 probe;
  failure path is STOP-and-report with measurements (packet stays draft on redesign).
- `apply_opening(half line width)` on candidates has no canonical counterpart; audit-and-
  report after S1: if the surviving calicat candidate cannot clear the opening, surface the
  measurement before any change.
- 20mm-box steady state unknown post-S1 (canonical legitimately bridges under top shells over
  sparse interiors); governed by the golden policy, not by zero-site privilege.
- Mega-packet review surface (F4 absorbed): mitigated by step gates and full `/spec-review`
  closure scope; `cargo xtask test --workspace` reserved for the acceptance ceremony.
- Interim site counts before S5 completes may differ from the final arbiter bar; only S6 bars
  bind.
- Guest fingerprint churn on every IR/WIT touch is expected; freshness gate is authoritative.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (S5 split into S5a/S5b keeps every step ≤ M)
- Highest-risk dispatch and required return format: S4 reachability probe, FACT ≤5 lines.

## Open Questions

- `[FWD→S1]` Does a calicat site appear in-window with `top_solid_fill` candidates alone once
  RC-A is fixed? Probe-and-record; result tunes nothing silently — taxonomy lands regardless.
- `[FWD→S1]` Does the surviving candidate clear `apply_opening`? Audit-and-report; act only
  on measured evidence, else record micro-deviation in bridge-parity plan F3 addendum.
- `[FWD→S4]` Reachability probe outcome governs whether S4 proceeds or the packet stops for
  redesign approval.
- None `[BLOCK]`.
