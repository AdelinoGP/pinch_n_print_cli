# Requirements: 234a-internal-bridge-support-gating (closure edition)

## Motivation

The landed 234a edition ported canonical support qualification and relocated it into the
ShellClassification prepass; every frozen flood-era bar passed, but the recorded deviation
stands: calicat qualifies ZERO internal-bridge sites where canonical produces EXACTLY ONE
near Z≈29.45 (skip histogram over 174 layer-visits: 156x `top_solid_fill_empty`, 9x
`unsupported_empty`, 9x `qualified_empty`). The grilling session
(`docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md`) established a NEW,
dispatch-verified root cause candidate (RC-A): `unsupported_span_areas`
(`crates/slicer-core/src/algos/bridge_over_infill.rs`) initializes the unsupported carrier as a
BOUNDING-BOX COMPLEMENT of the lower fills (`fill_envelope`), while canonical
`PrintObject::bridge_over_infill` initializes it to the FILL POLYGONS THEMSELVES
("initially consider the whole layer unsupported"), then closes/shrinks/diffs grown solids.
RC-A predicts the measured histogram exactly. Independently, our IR lacks any dense-interior
(`stInternalSolid`) taxonomy and construction anchors on polygon stand-ins instead of walls.
This packet closes the deviation AND absorbs F4 (coverage/anchoring ~30–35% extruded-length
deficit) so ISSUE-82 reaches terminus.

Relation to prior work: builds on landed 233/234/235 and the original 234a edition; revises
THIS packet directory in place (pre-revision text in git history); no other packet directory
is modified or superseded.

## Authoritative Full Scope

1. **S1 Arithmetic fix:** `unsupported_span_areas` takes pooled lower fills as the initial
   unsupported set: `shrink(closing(fills), mult*spacing)` minus `expand(shrink(solids,
   1*spacing), (1+mult)*spacing)`; `fill_envelope` deleted from the dataflow; unit fixtures
   re-blessed to canonical-correct outputs.
2. **S2 Dense-interior taxonomy:** net-new `SlicedRegion.internal_solid_fill: Vec<ExPolygon>`
   (`#[serde(default)]`), authored by `compute_region_updates`' shell-classification stage as
   the shell band MINUS the depth-0 exposed seed; mirrored through `SliceRegionView::from_ir`
   and the canonical WIT region type; render arm in `slice_shapes` + viewport push in
   `geometry_points_mm`.
3. **S3 Qualification rewrite:** prepass pass consumes `internal_solid_fill` candidates;
   `lower_fills` = union of lower regions' `infill_areas`; per-lower-region solids =
   `top_solid_fill ∪ bottom_solid_fill ∪ internal_solid_fill ∪ bridge_areas`, OR that region's
   own `infill_areas` when its resolved `infill_density >= 0.999` (per-region condition via
   each lower region's own `RegionKey` in `region_map.config_for`); qualified polygons extend
   `bridge_areas` AND persist to net-new host-only `SlicedRegion.internal_bridge_areas`;
   skip-histogram logs keyed by print_z.
4. **S4 Venue split:** feasibility probe (walls + Layer::Infill polylines reachable in the
   InfillPostProcess arm; STOP-and-report on failure), then the arm constructs via
   `construct_anchored_polygon` + `determine_bridging_angle` from committed
   `internal_bridge_areas`; `internal_bridge_lines` retired tree-wide;
   `infill_postprocess_contract_tdd.rs` rewritten (234a AC-4 pure-emitter check reversed).
5. **S5 F4 coverage parity:** expansion zones (`expansion_step = scaled(0.1)`, up to 5 steps;
   `expansion_bottom_bridge = shell_width*sqrt(2)`); frSolidInfill-spacing closing radius in
   construction; `gather_areas_w_depth` downward harvesting; thread clustering +
   filled-polygons-on-lower-layers removal; `enable_extra_bridge_layer` emission-semantics
   alignment in `modules/core-modules/rectilinear-infill`. New ports carry the attribution
   header per `docs/ORCASLICER_ATTRIBUTION.md`.
6. **S6 Arbitration harness:** bundle-primary calicat arbiter (AC-5), revised gcode e2e
   (AC-6), mixed-density fixture (AC-N1), golden re-bless ceremony with evidence table
   (AC-N2), wedge tripwire (AC-N3).

Out of scope: F5/F6/F7 rows of the bridge-parity plan (flow spacing/speed coupling, sparse
alternation); runnable OrcaSlicer oracle claims; scheduler stage-set changes; modules beyond
the scoped rectilinear-infill alignment.

## AC-ID Summary

| AC | Artifact under test | Binary driving it |
| --- | --- | --- |
| AC-1 | `unsupported_span_areas` semantics | slicer-core `bridge_support_gating_tdd` |
| AC-2 | IR/WIT/view mirror + serde default | static greps (views.rs, wit/, slice_ir.rs) |
| AC-3 | Prepass qualification persistence | slicer-runtime `integration` |
| AC-4 | InfillPostProcess constructor + field retirement | slicer-runtime `contract` |
| AC-5 | Bundle-primary one-site arbiter | slicer-runtime `e2e` (net-new) |
| AC-6 | G-code secondary consistency bars | slicer-runtime `e2e` (revised) |
| AC-N1 | Mixed-density rejection (negative) | slicer-runtime `e2e` (net-new) |
| AC-N2 | Golden equality after evidence-documented re-bless | slicer-runtime `e2e` |
| AC-N3 | Wedge regression tripwire | slicer-runtime `e2e` |

## Verification Matrix

Repeat of every pipe-suffixed command (authoritative strings live in `packet.spec.md`):

1. AC-1: `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- fills_are_the_initial_unsupported_carrier --nocapture 2>&1 | tee target/test-output.log`
2. AC-2: `rg -q 'internal_solid_fill' crates/slicer-sdk/src/views.rs && rg -q 'internal_solid_fill' crates/slicer-schema/wit && ! rg -q 'internal_bridge_areas' crates/slicer-schema/wit && rg -q -U '#\[serde\(default\)\]\s+(pub )?internal_solid_fill' crates/slicer-ir/src/slice_ir.rs && rg -q -U '#\[serde\(default\)\]\s+(pub )?internal_bridge_areas' crates/slicer-ir/src/slice_ir.rs`
3. AC-3: `cargo test -p slicer-runtime --test integration -- internal_bridge_qualification_writes_gated_areas --nocapture 2>&1 | tee target/test-output.log`
4. AC-4: `cargo test -p slicer-runtime --test contract -- infill_postprocess_constructs_anchored_paths --nocapture 2>&1 | tee target/test-output.log && ! rg -q 'internal_bridge_lines' crates/ modules/`
5. AC-5: `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_arbiter_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
6. AC-6: `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_gating_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
7. AC-N1: `cargo test -p slicer-runtime --test e2e -- mixed_density_internal_bridge_rejection_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
8. AC-N2: `cargo test -p slicer-runtime --test e2e -- slicing_precision_integration_tdd::legacy_zero_matches_golden --nocapture 2>&1 | tee target/test-output.log`
9. AC-N3: `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report_tdd --nocapture 2>&1 | tee target/test-output.log`
10. Gates: `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check` (exit 0).
11. Ceremony only: `cargo xtask test --workspace` via sub-agent FACT pass/fail.

## Cross-Step Expectations

- Guest WASM freshness gate after every step that touches slicer-ir/slicer-core/sdk/wit
  (snippet rule in design.md); fingerprints WILL change in S2/S3/S4.
- Struct-literal sweep (`cargo check --workspace --all-targets` + `cargo xtask
  check-literals`) inside S2 (field add) and S4 (field retirement); FRU conversions for test
  literals; production literals stay exhaustive per `docs/21_data_defaults_and_fixtures.md`.
- Determinism: prepass stays sequential over sorted timelines; per-layer construction is
  deterministic given committed anchors; AC-6 byte-identity guards both.
- Config keys remain snake_case; `infill_density` resolved per lower-region `RegionKey`.
- Doc Impact execution (F3/F4 rows) lands with S6, after measured numbers exist.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — gather initialization ("initially consider the whole layer unsupported"), `filter_by_type(stInternalSolid)` candidate loop, `stInternalBridge` subtraction from stInternal/stInternalSolid, density==100 supporter branch, `generate_sparse_infill_polylines_for_anchoring`, `gather_areas_w_depth`, thread clustering + `filled_polyons_on_lower_layers`, expansion zones (`expansion_step`, `expansion_bottom_bridge`); `discover_horizontal_shells` `extra_solid_infills` deliberately NOT borrowed
