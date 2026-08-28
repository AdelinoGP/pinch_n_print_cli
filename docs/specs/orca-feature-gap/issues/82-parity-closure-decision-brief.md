# ISSUE-82 Parity Closure — Decision Brief (grill output, 2026-08-24)

Self-contained input for `/spec-packet-generator`. Produced by an adversarial grilling
session over the 234a residual deviation (calicat qualifies ZERO internal-bridge sites
vs canonical's EXACTLY ONE near Z≈29.45). Every tree/canonical claim below was
dispatch-verified against the working tree and `OrcaSlicerDocumented/` during the
session; nothing is carried forward from memory of prior packets.

Tree basis at authoring: branch `parity/bridges`, HEAD = the merge of `origin/master`
into `parity/bridges` (re-derive HEAD at packet-authoring time; ledger facts rot).
All placement facts below were RE-VERIFIED on that merged tree.

---

## 0. Ground truth established this session (evidence base)

### 0.1 NEW ROOT-CAUSE CANDIDATE: arithmetic inversion (RC-A) — verified verbatim

Canonical `PrintObject::bridge_over_infill` (`OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`)
builds the unsupported-span filter as **the lower-layer fill polygons themselves**:

```cpp
// initially consider the whole layer unsupported, ...
unsupported_area.insert(unsupported_area.end(), fill_polys.begin(), fill_polys.end());
...
unsupported_area = closing(unsupported_area, float(SCALED_EPSILON));
...
unsupported_area   = shrink(unsupported_area, expansion_multiplier * spacing);
unsupported_area   = diff(unsupported_area, lower_layer_solids);
```

Verified: `unsupported_area` is NEVER assigned a complement of the fills anywhere
between initialization and the candidate loop; the only mutations are insert /
closing(SCALED_EPSILON) / shrink(mult·spacing) / diff(grown solids).

Our port, `unsupported_span_areas` + `fill_envelope`
(`crates/slicer-core/src/algos/bridge_over_infill.rs`), instead computes
`difference(bounding_box_envelope(lower_fills), closing_ex(lower_fills))` — the
**complement of the fills inside their bbox** — then shrinks. This is a semantic
inversion: we model air pockets BETWEEN infill islands where canonical models the
sparse-infill-carried zones. After the `expansion_multiplier·spacing` shrink the
complement slivers annihilate, which predicts EXACTLY the measured 234a skip
histogram (9× `unsupported_empty`, 9× `qualified_empty` over the surviving
candidate visits). **RC-A fits the measured evidence at least as well as the
recorded taxonomy explanation (RC-B), likely entirely without it.**

Everything else in the port matches canonical verbatim (see §4).

### 0.2 Canonical gather semantics (all dispatch-verified)

- `fills` = lower layer's `region->fill_expolygons` (fill area INSIDE perimeters),
  pooled across ALL lower-layer regions, unconditionally.
- `solids` = per lower-layer region, every `fill_surfaces` entry whose type
  `!= stInternal`; **when that region's `sparse_infill_density == 100`, ALL of its
  surfaces**. The density check sits INSIDE the per-`LayerRegion` loop → the branch
  is REGION-LOCAL (a fully-solid region and a sparse region in one object are
  handled independently; fills pool, density conditions don't).
- Candidates = CURRENT layer's `stInternalSolid` surfaces ONLY
  (`filter_by_type(stInternalSolid)`; exposed tops are stTop and excluded).
- Result: qualified areas become `stInternalBridge`; `cut_from_infill` is subtracted
  from both `stInternal` infill and `stInternalSolid`.
- Gather uses ONLY the immediately-lower layer; deeper scans exist solely in
  `gather_areas_w_depth` (thick-span anchoring) and the thread-cluster
  filled-polygons-on-lower-layers removal — both F4 territory.
- Orca multiplier rule: `expansion_multiplier = 3` when
  `dont_filter_internal_bridges == ibfDisabled`, else `1`.

### 0.3 Tree facts (dispatch-verified)

- `SlicedRegion` (`crates/slicer-ir/src/slice_ir.rs`) area fields: `polygons`,
  `infill_areas`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`,
  `sparse_infill_area`, `internal_bridge_lines`. **No dense-interior field exists**
  (the RC-B gap). `InfillRegion` carries path lists only
  (`sparse_infill`, `solid_infill`, `ironing`, `internal_bridge_infill`).
- Config keys: `infill_density` (resolved CLI key, fraction f32,
  `crates/slicer-ir/src/resolved_config.rs`), `sparse_infill_density`
  (model/3MF + perimeters-module key). **ABSENT**: `solid_infill_every_layers`,
  `extra_solid_infills`, `internal_solid_infill_density` — our pipeline has NO
  mid-object fully-dense-layer mechanism; the only dense-interior producer possible
  today is the shell band.
- `top_solid_fill` writer: `compute_region_updates`
  (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`), Pass 1 depth-0:
  `apply_opening(difference(region_polys, upper_polys), opening_r)` (opening_r =
  half line width), then Pass 2 shadow propagation through shell layers. NOTE: the
  opening has NO canonical counterpart on stInternalSolid candidates (§4 audit).
- `gate_internal_bridge_sites` (same file) currently: gathers `lower_layer_polygons`
  from lower `region.polygons` (FULL island outlines incl. perimeter band);
  `lower_solids` = top ∪ bottom shell fills pooled across regions; resolves config
  once per timeline via first-entry `RegionKey`; qualifies `top_solid_fill`;
  constructs via `construct_anchored_polygon` on polygon/inset contour stand-ins;
  writes `region.internal_bridge_lines`.
- `LayerStageCommit::InfillPostProcess` arm (`crates/slicer-runtime/src/layer_executor.rs`)
  is a pure emitter over `internal_bridge_lines`.
- Stage order (verified): `Layer::Perimeters` → `PerimetersPostProcess` → `Infill` →
  `InfillPostProcess` → `Support` → `SupportPostProcess` → `PathOptimization`. So
  perimeter walls AND `Layer::Infill` sparse polylines genuinely exist by
  InfillPostProcess time.
- Post-merge re-verification confirmed ALL of the above unchanged; `fill_envelope`'s
  bbox complement intact; `SliceRegionView::from_ir` (`crates/slicer-sdk/src/views.rs`)
  still excludes `internal_bridge_lines` (view DOES carry `is_internal_bridge: bool`).
- Visual-debug (`pnp_cli visual-debug` → `run_visual_debug`
  `crates/pnp-cli/src/visual_debug.rs`; renderer
  `crates/slicer-runtime/src/visual_debug_render.rs`): re-runs slicing internally;
  carrier `StageCapture { ir: CapturedIr }` (`layer_executor.rs`); `CapturedIr::Slice(SliceIR)`
  is serialized VERBATIM into manifest.json `typed_capture` via
  `serde_json::to_value(&capture.ir)`. Therefore ANY net-new `SlicedRegion` field
  (which derives Serialize) leaks into bundle JSON automatically — add
  `#[serde(default)]` for old-bundle compatibility. Only `polygons`/`infill_areas`
  currently RENDER (`slice_shapes`); rendering a new field requires a `slice_shapes`
  arm + a `geometry_points_mm` viewport push (+ optional palette constant).
  `docs/19_visual_debug.md` mentions neither shell classification nor internal bridges.
- NEG-2 golden: `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`,
  compared by `legacy_zero_matches_golden`
  (`crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs`);
  re-bless via env `BLESS_GOLDEN=1`. Currently 0 `;TYPE:Internal Bridge` sections
  (post-234a re-bless; flood-era had 94).
- `enable_extra_bridge_layer` EXISTS in-tree, read by
  `modules/core-modules/rectilinear-infill/src/lib.rs`.

---

## 1. Decisions table (per agenda item)

Format: **Decision** / Rejected alternatives (reasons) / Evidence / Risks.

### Item 1 — Candidate-source modeling (what plays `stInternalSolid`)

- **Decision:** BOTH fixes land unconditionally in the closure packet:
  (a) RC-A arithmetic fix (§4/S1), AND (b) a true dense-interior classification:
  net-new `SlicedRegion.internal_solid_fill: Vec<ExPolygon>`, **WIT-VISIBLE**
  (mirrored through `SliceRegionView`/WIT), authored by the shell-classification
  stage (the prepass — the only sequential, cross-layer-legal home; per-layer
  arenas forbid slice/partition-stage authoring of ceiling proximity). Definition:
  **shell band MINUS the depth-0 exposed seed** (seed recomputable as
  `opening(difference(polygons, upper-layer polygons))` at authoring time) — mirrors
  canonical stInternalSolid-under-stTop and keeps bridge candidates one layer below
  visible ceilings. Naming follows `top_solid_fill` convention. Density==100
  branch: implemented PER-REGION via resolved `infill_density >= 0.999` → that
  region contributes its OWN `infill_areas` to `lower_solids` ("ALL surfaces"
  analog).
- Rejected: arithmetic-first-gated-taxonomy (user chose unconditional); prepass-local
  transient computation (invisible to debug/consumers; repeats 234a log-scraping pain);
  host-only persisted field (insufficient for stated future-proofing goal); module/
  partition-stage authoring (illegal under rayon arena rules); flag-only (loses
  geometry/debug); whole-band definition (qualifies visible ceilings, includes
  stTop which canonical excludes); scheduled-solid-layers definition (needs absent
  config-key family — scope explosion).
- Evidence: §0.1–0.3 dispatches.
- Risks: guest WASM fingerprints change (mandatory `cargo xtask build-guests`
  rebuild — happens anyway due to slicer-core edits); exhaustive-literal updates +
  check-literals watchlist; WIT-change checklist obligations (§7); exposure is
  future-proofing — no module consumer exists YET (named intended consumers:
  infill modules emitting dense interior explicitly, e.g. InternalSolid-role
  fidelity); `apply_opening` may suppress thin candidates (audit, §4).

### Item 2 — Lower-layer input mapping

- **Decision:**
  - `lower_fills` := union of lower-layer regions' `infill_areas` (closest tree
    analog of canonical `fill_expolygons`).
  - Sparse-region (density < 100%) `lower_solids` contribution :=
    `top_solid_fill ∪ bottom_solid_fill ∪ internal_solid_fill ∪ bridge_areas`
    (full non-stInternal analog; canonical non-stInternal includes prior bridge
    surfaces too).
  - Dense-region (density ≥ 0.999) contribution := that region's own `infill_areas`.
  - Semantics: fills pool across all lower regions unconditionally; density
    condition evaluated per LOWER region via THAT region's own `RegionKey` in
    `region_map.config_for(...)` (NOT once-per-timeline like the current flow
    resolution); gating remains per candidate region.
  - Bias accounting (direction, qualitative magnitude): status-quo
    `region.polygons` fills push detection OUTWARD ~one perimeter band
    (~2 line widths + inset) = over-detection near walls; missing dense/bridge
    solids = under-subtraction = further over-detection. Both corrections shrink
    false-positive pressure while RC-A's fix massively ENLARGES legitimate spans —
    net effect must be measured, not assumed.
- Rejected: keep `region.polygons` (documented bias); ad-hoc inset shrink
  (duplicates partition-owned inset logic); keep shells-only solids (leaves
  under-subtraction bias); all-minus-sparse solids (over-broad, suppresses
  legitimate sites).
- Evidence: §0.2 canonical snippet (region-local density check); §0.3 gather facts.
- Risks: per-lower-region config resolution is a NEW resolution site (cost trivial:
  HashMap lookups); mixed-density objects previously untested — synthetic fixture
  mandated (§5).

### Item 3 — Anchoring and construction placement

- **Decision:** Venue split. Prepass keeps cross-layer QUALIFICATION and persists
  qualified polygons; ANCHORED CONSTRUCTION moves downstream to the per-layer
  `LayerStageCommit::InfillPostProcess` arm, consuming REAL perimeter walls +
  `Layer::Infill` sparse polylines as anchors (matches canonical ordering, where
  `bridge_over_infill` runs after walls/clip_fill). This REVERSES 234a's AC-4
  pure-emitter absence check — deliberate, recorded, priced (contract tests
  rewritten; `internal_bridge_lines` carrier retires; replaced by a persisted
  qualified-area carrier, see Approvals #3/#4).
- Rejected: all-in-prepass (walls/true anchor polylines never enter construction;
  permanently caps F4 coverage fidelity regardless of site-selection fixes).
- Evidence: stage-order verification (§0.3); post-merge re-check.
- Risks: wall/polyline reachability inside the arm is UNVERIFIED — S4's first task
  is a reachability probe with STOP-and-report on failure; determinism of
  per-layer parallel construction (angle selection is deterministic given anchors;
  double-slice byte-identity guards it).

### Item 4 — Constants/threshold fidelity audit

- **Decision:** sweep COMPLETE. Matching canonical verbatim (no action): closing
  radius SCALED_EPSILON (both sides close the pooled unsupported set); shrink
  `expansion_multiplier·spacing`; solids shrink `1·spacing` then grow
  `(1+mult)·spacing`; partial gate form (area == surface OR > 9·sp²; our
  `- 1.0`-unit² slack in `partially_supported` is negligible at unit scale);
  worth-clip `expand(4·spacing)`; leftover remerge `sp² < area < sp·120000`
  (= scale_(12.0) = 120,000 units); multiplier 3-default/1-relaxed matches
  ibfDisabled-vs-other. **Sole defect found: the complement initialization (RC-A)** —
  fixed in S1. `enable_extra_bridge_layer`: module-owned today; enters packet scope
  via F4 absorption (emission-semantics parity only).
- Rejected: folding the `apply_opening` question silently into the existing
  taxonomy deviation (hides a potential site suppressor).
- Evidence: side-by-side symbol-level comparison vs verbatim canonical snippets.
- Risks (open, bounded): (a) `apply_opening(half line width)` in
  `compute_region_updates` has no canonical counterpart on candidates and could
  erase thin legitimate surfaces — DECIDED: audit-and-report; measure on calicat
  post-S1 whether the surviving ≈Z≈29.45-class candidate survives the opening; act
  only on measured evidence; otherwise record as micro-deviation. (b) 20mm-box
  steady state UNKNOWN until S1 lands (fixed arithmetic may legitimately create
  sites inside plain boxes near tops — canonical does bridge under top shells over
  sparse interiors); handled by golden policy (§5).

### Item 5 — Empirical arbitration protocol

- **Decision:**
  - PRIMARY assertion surface: visual-debug bundle JSON — run the capture pipeline,
    assert on `typed_capture` payloads containing persisted `internal_solid_fill`
    and the qualified internal-bridge areas per stage (auto-serialization verified,
    §0.3). Requires the S2 render-arm work only for PNG rendering, NOT for JSON
    assertions.
  - SECONDARY consistency: emitted-gcode parsing (extend the existing calicat e2e
    machinery: `;TYPE:Internal Bridge` sections parsed per-Z, length-weighted
    angles).
  - ARBITER BAR (replaces the ≤6-flood-layer proxy): on calicat, EXACTLY ONE
    Internal-Bridge layer within print_z ∈ **[29.15, 29.75]** (canonical ≈29.45
    ± 0.30, generous vs 0.2 layer height), extruded length ∈ **[300, 700] mm**
    (brackets canonical ≈526 mm), ZERO Internal-Bridge extrusion outside that
    window, external Bridge row pins unchanged, double-slice byte-identity holds.
  - Diagnostics: skip-histogram debug logs upgraded to be keyed by print_z
    (currently aggregate-only; the 156/9/9 histogram could not localize WHICH
    visit failed WHICH gate).
  - Regression set the successor packet MUST hold: wedge suite pins (print_z 28.2
    slot-ceiling) pass UNMODIFIED — failure ⇒ STOP-and-report, never re-pin
    in-step; NEG-2 golden under CONDITIONAL re-bless policy (below); mixed-region
    density fixture (NEW): solid slab beside sparse column under one ceiling —
    assert ZERO sites above the solid half, preserved qualification above the
    sparse half; byte-identity double-slice on calicat.
  - GOLDEN POLICY (20mm-box / NEG-2): zero-sites is NOT privileged as "correct".
    Re-bless permitted ONLY with in-comment evidence: full section-count diff
    table, Z-set identity, per-diff-class canonical reasoning (sites predicted by
    the FIXED arithmetic under our defaults are canonical-consistent drift; anything
    else is a bug to fix, not bless).
- Rejected: gcode-parse-only primary (weaker attribution; kept as secondary);
  bundle-JSON-only-without-gcode cross-check; looser 1–2-site windows; byte-match
  against a captured oracle (impossible — `OrcaSlicerDocumented/` is readable-not-
  runnable; no runnable reference gcode exists in-tree).
- Evidence: §0.3 visual-debug dataflow; existing e2e structure.
- Risks: bundle-JSON assertions couple acceptance tests to capture machinery
  (mitigation: secondary gcode checks); the arbiter bar applies at S6 AFTER full
  F4 absorption — canonical's single-site count INCLUDES its deep-harvesting
  machinery, so interim S3 checkpoints assert site PRESENCE in-window, not exact
  counts.

### Item 6 — Scope fence

- **Decision: ABSORB FULL F4.** The closure packet is the ISSUE-82 terminus:
  RC-A arithmetic + dense-interior WIT taxonomy + construction venue move + ALL
  coverage/anchoring parity: expansion zones (`expansion_step = scaled(0.1)`, up to
  5 steps; `expansion_bottom_bridge = shell_width·sqrt(2)`), closing radius from
  frSolidInfill spacing in construction, `gather_areas_w_depth` thick-span downward
  harvesting, thread clustering + filled-polygons-on-lower-layers removal, and
  `enable_extra_bridge_layer` emission semantics.
- Rejected: strictly-site-parity fence (leaves the known ~30–35% extruded-length
  deficit open); middle path (venue + bottom-anchor constant only — muddier fence,
  splits cohesive mechanics).
- Evidence: F4 definition (`docs/specs/bridge-parity-plan.md` §F4) + canonical
  machinery inventory (§0.2).
- Risks: two independently-testable risk classes (selection vs coverage) coupled in
  one packet; largest review surface — mitigations: strict S1→S6 ordering with
  per-step gates; `/spec-review` full closure scope at packet close; `cargo test
  --workspace` ONLY at the acceptance ceremony per AGENTS.md, dispatched to a
  sub-agent with FACT pass/fail.

### Item 7 — Acceptance-criteria draft

Pipe-suffixed = combined output tee'd to `target/test-output.log` per AGENTS.md.
Proposed ACs (final wording belongs to `/spec-packet-generator`):

- **AC-1 (math parity):** updated `bridge_support_gating_tdd` fixtures encode
  fills-as-initial semantics.
  `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd 2>&1 | tee target/test-output.log`
- **AC-2 (arbitration, bundle-primary):** calicat capture run asserts exactly-one
  in-window site, length bracket, zero-outside-window on `typed_capture` JSON.
  (new test; shape: `cargo test -p slicer-runtime --test e2e -- <bundle_arbiter_test> 2>&1 | tee target/test-output.log`)
- **AC-3 (gcode consistency, secondary):** revised
  `calicat_internal_bridge_gating_e2e_tdd` — window/length bars, external row
  90.0°/74 segs/324.6 mm @ Z≈3.2 (±0.25 window) unchanged, byte-identical double
  slice.
- **AC-4 (NEGATIVE criterion, mixed density):** synthetic solid+sparse object —
  zero Internal-Bridge extrusion above the fully-dense half; qualification
  preserved above the sparse half.
- **AC-5 (venue/contract):** rewritten `infill_postprocess_contract_tdd` asserting
  construction consumes committed qualified areas + wall/anchor geometry (records
  the AC-4 reversal).
- **AC-6 (regressions):** wedge suites pass unmodified; NEG-2 golden passes after
  evidence-documented re-bless (`BLESS_GOLDEN=1 cargo test -p slicer-runtime --test e2e -- slicing_precision_integration_tdd::legacy_zero_matches_golden --nocapture 2>&1 | tee target/test-output.log`).
- **Gates:** `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo xtask check-literals`; `cargo xtask build-guests --check` exit 0;
  `cargo build --tests` after the WIT change; `cargo xtask test --workspace`
  ONLY at the acceptance ceremony (AGENTS.md rule).

---

## 2. Successor-packet step skeleton

Ordering principle: each step leaves the tree green on its own gates; S4 starts
with a STOP-capable feasibility probe.

### S1 — Arithmetic fix (RC-A)
- Objective: `unsupported_span_areas` initializes from the pooled lower FILLS
  themselves (closing → shrink mult·spacing → diff grown solids); retire/repurpose
  `fill_envelope`; re-bless unit fixtures to canonical-correct outputs (Test
  Discipline permits fixture updates when the canonical fix changes behaviour).
- Files IN: `crates/slicer-core/src/algos/bridge_over_infill.rs`,
  `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (+ its Cargo.toml [[test]]
  entry only if touched).
- Files OUT: `modules/**`, `crates/slicer-schema/wit/**`, IR, executor.
- Verify: AC-1 command; `cargo check --workspace --all-targets`.
- Interim measurement (feeds §4 audit): calicat probe — does a site appear
  in-window with top_solid_fill candidates alone? Record; do not tune.

### S2 — IR/WIT field `internal_solid_fill`
- Objective: net-new `SlicedRegion.internal_solid_fill: Vec<ExPolygon>`
  (#[serde(default)]) authored by the shell-classification stage as band-minus-
  exposed-seed; WIT type + `SliceRegionView` mirror; renderer arms (`slice_shapes`,
  `geometry_points_mm`, optional palette); production literal in
  `execute_prepass_slice_single_layer_impl` (`crates/slicer-core/src/algos/prepass_slice.rs`)
  extended exhaustively; test literals → FRU.
- Files IN: `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/**`,
  `crates/slicer-sdk/src/views.rs`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  (authoring), `crates/slicer-runtime/src/visual_debug_render.rs`, prepass_slice
  literal, affected test literals.
- Files OUT: module manifests; scheduler stages; `InfillIR` shape.
- Verify: `cargo build --tests`; `cargo xtask build-guests` (fingerprints WILL
  change); `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo xtask check-literals`.
- Obligations: WIT-change checklist (search `wit_host.rs`/`dispatch.rs`/guest WIT
  consumers for the affected type; verify type identity across the boundary).

### S3 — Qualification rewrite (prepass)
- Objective: `gate_internal_bridge_sites` successor consumes `internal_solid_fill`
  as candidates; `lower_fills` from lower `infill_areas`; per-lower-region solids
  composition incl. density≥0.999 branch (per-region `RegionKey` resolution);
  persists qualified polygons to `bridge_areas` extension + net-new host-only
  `SlicedRegion.internal_bridge_areas: Vec<ExPolygon>` (un-mirrored; auto-serializes
  into bundles); skip-histogram logs keyed by print_z. No construction here anymore.
- Files IN: `slice_postprocess_prepass.rs`, `crates/slicer-ir/src/slice_ir.rs`
  (one field), `region_partition` consumers untouched (already read `bridge_areas`),
  related unit/integration tests (`region_partition_tdd`).
- Files OUT: executor arm (S4), modules.
- Verify: narrow crate tests teed; `cargo xtask check-literals`.
- Checkpoint expectation: site PRESENCE in-window on calicat (exact counts deferred
  to S6 — see §5 risk).

### S4 — Construction relocation (InfillPostProcess)
- Objective: FEASIBILITY PROBE FIRST — verify perimeter-wall geometry + Layer::Infill
  polylines are reachable in the `LayerStageCommit::InfillPostProcess` arm
  (STOP-and-report if not). Then: arm becomes constructor — harvest anchors
  (walls + sparse polylines), `determine_bridging_angle` (override respected),
  `construct_anchored_polygon` over `internal_bridge_areas`, emit
  `ExtrusionRole::InternalBridgeInfill` paths; retire `internal_bridge_lines` field
  and prepass construction; rewrite `infill_postprocess_contract_tdd` (AC-4 reversal
  recorded in design).
- Files IN: `crates/slicer-runtime/src/layer_executor.rs`,
  `crates/slicer-ir/src/slice_ir.rs` (field retirement),
  `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`,
  literal sweeps.
- Files OUT: modules; WIT (role variant already exists from 233).
- Verify: contract tests teed; double-slice byte-identity via AC-3 run.
### S5 — F4 coverage/anchoring parity
- Objective: port canonical coverage machinery onto the new venue: expansion zones
  (`expansion_step` scaled(0.1) ·≤5 steps, `expansion_bottom_bridge =
  shell_width·sqrt(2)`), frSolidInfill-spacing closing in construction,
  `gather_areas_w_depth` thick-span harvesting, thread clustering +
  filled-polygons-on-lower-layers removal, `enable_extra_bridge_layer` emission
  semantics (module-side alignment).
- Files IN: `bridge_over_infill.rs` (new pure helpers + tests),
  `layer_executor.rs` (construction site), `modules/core-modules/rectilinear-infill/`
  (extra-layer semantics ONLY — flag any wider module need, STOP-and-report),
  new unit tests.
- Files OUT: other core-modules; WIT beyond what S2 landed.
- Verify: new unit tests teed; wedge suites; AC-3.

### S6 — Arbitration/regression harness + goldens
- Objective: bundle-primary arbiter test (AC-2), gcode secondary (AC-3), mixed-
  density fixture (AC-4), NEG-2 re-bless ceremony with evidence table (AC-6),
  finalize all gates.
- Files IN: `crates/slicer-runtime/tests/e2e/*` (new + revised), golden fixture,
  test doc-comments carrying re-bless evidence.
- Verify: all AC commands; then acceptance ceremony: `cargo xtask test --workspace`
  dispatched to a sub-agent, FACT pass/fail returned.

---

## 3. Open risks register (post-grill)

1. Wall/polyline reachability in the InfillPostProcess arm — unverified; S4 probe
   first, STOP-and-report.
2. `apply_opening` candidate suppression — unknown until S1's calicat probe;
   audit-and-report decision binds.
3. 20mm-box steady state — unknown until S1 lands; golden policy governs.
4. Mega-packet review surface (F4 absorbed) — mitigated by step gates + full
   `/spec-review` closure scope.
5. Bundle-JSON test coupling — mitigated by retained gcode secondary checks.
6. Guest fingerprint churn on every IR touch — expected; freshness gate
   (`cargo xtask build-guests --check`) is authoritative.
7. Interim vs final site counts — canonical single-site expectation includes deep
   harvesting; only the S6 bar is binding.

## 4. Approvals Granted (recorded during the grill, 2026-08-24)

1. Both fixes UNCONDITIONAL in one packet (RC-A arithmetic + dense-interior
   taxonomy).
2. Net-new IR+WIT field `SlicedRegion.internal_solid_fill: Vec<ExPolygon>` —
   WIT-VISIBLE/mirrored, #[serde(default)], render arms included. Justification
   recorded: future-proofing for infill modules (+ future consumers) to emit dense
   interior explicitly; no current module consumer.
3. Net-new HOST-ONLY (un-mirrored) carrier `SlicedRegion.internal_bridge_areas:
   Vec<ExPolygon>` for prepass-qualified polygons (auto-serializes into visual-debug
   bundles).
4. `SlicedRegion.internal_bridge_lines` RETIRED when construction relocates; 234a's
   AC-4 pure-emitter absence check reversed deliberately (recorded deviation from
   234a design, not silent).
5. Dense definition: shell band minus depth-0 exposed seed; naming
   `internal_solid_fill` (snake_case, `top_solid_fill` convention).
6. density==100 branch: per-region, resolved `infill_density >= 0.999`, solids :=
   that region's own `infill_areas`.
7. Input mapping: `lower_fills` := lower `infill_areas` union; sparse-region solids
   := top ∪ bottom ∪ internal_solid_fill ∪ bridge_areas.
8. Construction venue: qualify at prepass, build at InfillPostProcess (walls +
   sparse polylines as anchors).
9. Constants audit closed; opening-radius interplay = audit-and-report (measured
   evidence only; STOP-and-report style).
10. Arbiter: EXACTLY ONE Internal-Bridge layer @ print_z ∈ [29.15, 29.75], length
    ∈ [300, 700] mm, zero elsewhere; external row pins hold; byte-identity holds.
11. Scope fence: ABSORB FULL F4 (expansion zones, depth harvesting, clustering,
    extra-layer semantics) — ISSUE-82 terminus packet.
12. Instrumentation: bundle-JSON PRIMARY; gcode-parse secondary; print_z-keyed
    skip diagnostics.
13. Golden policy: conditional re-bless with in-comment evidence (section-count
    diff table, Z-set identity, per-diff-class canonical reasoning); zero-sites not
    privileged.
14. Step skeleton S1→S6 approved as ordered above; pipe-suffixed verification
    command shapes per AGENTS.md; `cargo xtask test --workspace` reserved for the
    acceptance ceremony.
