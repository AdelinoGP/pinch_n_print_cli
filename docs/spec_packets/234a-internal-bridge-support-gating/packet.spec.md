---
status: implemented
packet: 234a-internal-bridge-support-gating
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md; decisions frozen in docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md
context_cost_estimate: M
---

# Packet Contract: 234a-internal-bridge-support-gating

**Closure edition (in-place revision, 2026-08-25).** The original edition's landed work
(Step-1 support math, ShellClassification relocation, pure-emitter reduction) stays; this
revision extends the SAME packet to deliver the canonical-faithful machinery and to absorb
F4 coverage/anchoring machinery. Residual low-z qualification and coverage breadth remain
documented divergences owned by other tracks; the pre-revision artifact text remains
recoverable in git history.

> Closure exception (2026-08-25): acceptance ceremony 353/354 binaries green; sole failure `wit_verify::built_core_module_components_embed_canonical_world` (support-planner guest WIT-world drift vs canonical `slicer:prepass-support-geometry`) is pre-existing support-track debt (last touched by packet-221/TASK-332 commits), not this packet's surface — tracked for the support family.

## Goal

Make internal-bridge-over-infill site selection, anchoring construction, and coverage
canonically faithful machinery delivered end-to-end: restore the fills-as-initial unsupported-span semantics,
author a WIT-visible dense-interior classification (`internal_solid_fill`), split venue so
qualification stays in the prepass while anchored construction consumes real walls and
sparse-infill anchors at InfillPostProcess, port F4's expansion/harvesting/clustering
machinery, and pin the documented matched-oracle baseline set via bundle-primary arbitration.
Residual low-z qualification and coverage breadth diverge from canonical and are owned
elsewhere by the shell-classification, infill, and support tracks (DEV-149/DEV-150), as
restated for closure on 2026-08-25.

## Scope Boundaries

In: arithmetic correction in `crates/slicer-core/src/algos/bridge_over_infill.rs`; net-new
`SlicedRegion.internal_solid_fill` (WIT-mirrored) and `SlicedRegion.internal_bridge_areas`
(host-only) with `internal_bridge_lines` retired; per-region density>=100% supporter branch;
prepass qualification rewrite; InfillPostProcess constructor; F4 expansion zones,
`gather_areas_w_depth` harvesting, thread clustering, `enable_extra_bridge_layer` emission
semantics (HOST-side, carrier-free: prepass-authored duplicates appended to the upper layer's
existing `internal_bridge_areas`, constructed by the existing InfillPostProcess path; no module
changes, no IR/WIT surface change); arbitration/regression harness incl. golden policy. Out:
F5/F6/F7 flow-speed and alternation rows, all modules, scheduler stage set, runnable-oracle
claims.

## Prerequisites and Blockers

- Depends on: landed 233 (construction seam, angle port), 234 (false-site gate),
  235 (external orientation); the original 234a edition (in substance on this tree —
  support math, ShellClassification relocation, pure-emitter reduction are all present;
  the packet directory itself reopened as `draft` for this closure revision).
- Unblocks: ISSUE-82 closure; F4 row of `docs/specs/bridge-parity-plan.md`.
- Activation blockers: none known at emission; S4 begins with a feasibility probe whose
  failure path is STOP-and-report, not silent redesign.

## Acceptance Criteria

- **AC-1. Given** lower-layer fill polygons F and solid polygons S, **when**
  `unsupported_span_areas` runs, **then** the result equals `shrink(closing(F), mult*spacing)`
  minus `expand(shrink(S, 1*spacing), (1+mult)*spacing)` — F itself is the initial unsupported
  carrier and no bounding-box complement is computed (`fill_envelope` removed from the dataflow).
  | `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- fills_are_the_initial_unsupported_carrier --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the net-new IR/WIT surface, **when** `SliceRegionView::from_ir` mirrors a
  region, **then** `internal_solid_fill` is copied into the view and declared in the canonical
  WIT region type, while `internal_bridge_areas` stays host-only (absent from the view and
  WIT) and both new `SlicedRegion` fields carry `#[serde(default)]`.
  | `rg -q 'internal_solid_fill' crates/slicer-sdk/src/views.rs && rg -q 'internal_solid_fill' crates/slicer-schema/wit && ! rg -q 'internal_bridge_areas' crates/slicer-schema/wit && rg -q -U '#\[serde\(default\)\]\s+(pub )?internal_solid_fill' crates/slicer-ir/src/slice_ir.rs && rg -q -U '#\[serde\(default\)\]\s+(pub )?internal_bridge_areas' crates/slicer-ir/src/slice_ir.rs`
- **AC-3. Given** committed region timelines with a dense-band candidate over sparse lower
  fills, **when** the prepass qualification pass runs, **then** qualified polygons land in BOTH
  `region.bridge_areas` and `region.internal_bridge_areas`, and every skip log line carries the
  visited layer's `print_z`. | `cargo test -p slicer-runtime --test integration -- internal_bridge_qualification_writes_gated_areas --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a committed `internal_bridge_areas` polygon set, **when** the
  `LayerStageCommit::InfillPostProcess` arm executes, **then** it constructs anchor-snapped
  strips via `construct_anchored_polygon` using perimeter-wall geometry and `Layer::Infill`
  sparse polylines as anchors, emits `ExtrusionRole::InternalBridgeInfill` paths, and
  `internal_bridge_lines` no longer exists anywhere in the tree.
  | `cargo test -p slicer-runtime --test contract -- infill_postprocess_constructs_anchored_paths --nocapture 2>&1 | tee target/test-output.log && ! rg -q 'internal_bridge_lines' crates/ modules/`
 - **AC-5. Given** a visual-debug capture of a calicat slice under the matched-oracle profile (`tmp/calicat_orcaSlicer.gcode` header: layer_height 0.2, first_layer 0.25, line_width 0.525, bridge_flow 0.95, infill_density 25%, supports on), **when** the bundle-primary arbiter inspects `typed_capture` payloads, **then** the canonical-faithful machinery pins the documented matched-profile baseline set {≈(4.45, 23.2 mm²), ≈(18.45, 8.4 mm²), ≈(29.45, 143.2 mm²)} within ±10% area tolerance; residual low-z qualification and cavity-coverage breadth diverge from canonical and are owned by the shell-classification / infill / support tracks (DEV-149/DEV-150, closure restatement 2026-08-25). | `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_arbiter_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
 - **AC-6. Given** emitted calicat G-code from the matched-oracle profile, **when** the secondary consistency parser runs, **then** the measured labeled-section truth is zero `;TYPE:Internal Bridge` sections and zero labeled extrusion (the fresh 2026-08-25 slice found geometry under other role labels); the external Bridge row nearest Z=3.2 keeps length-weighted dominant angle in [85, 95] degrees, and two consecutive slices are byte-identical. The label-role gap is DEV-153 and is owned by the infill/emitter track; the matched-profile measurement is pinned here rather than tuned toward the canonical bar. | `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_gating_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
 - **AC-7. Given** `enable_extra_bridge_layer` resolved off/default, **when** the pipeline emits G-code, **then** bytes are identical to the same slice with the feature absent (byte-stability verified over serialized SliceIR output as proxy); **when** it resolves on, **then** each qualifying internal-bridge layer gains one duplicated bridge layer directly above (dense-interior-overlap condition), constructed by the existing InfillPostProcess path. | `cargo test -p slicer-runtime --test integration -- extra_bridge_layer_emission_semantics --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** an object with one fully-dense region (resolved `infill_density >= 0.999`)
  beside one sparse region under a shared ceiling, **when** the pipeline slices it, **then**
  ZERO internal-bridge geometry qualifies above the dense half while qualification above the
  sparse half is preserved. | `cargo test -p slicer-runtime --test e2e -- mixed_density_internal_bridge_rejection_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the legacy-precision golden flow, **when**
  `legacy_zero_matches_golden` runs after the re-bless ceremony, **then** emitted bytes equal
  `precision_legacy_20mmbox.gcode`, whose re-bless evidence table (section-count diff,
  Z-set identity, per-diff-class reasoning) is recorded in the test's documentation comment
  and referenced from the deviation log / parity plan, not inside the fixture bytes (the BLESS
  flow regenerates the fixture from raw slicer output) (zero-sites is not
  privileged). | `cargo test -p slicer-runtime --test e2e -- slicing_precision_integration_tdd::legacy_zero_matches_golden --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** the wedge regression set, **when** the existing wedge suites run UNMODIFIED,
  **then** all pins hold; any failure is STOP-and-report with measurements, never an in-step
  re-pin. | `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report_tdd --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_arbiter_e2e_tdd --nocapture 2>&1 | tee target/test-output.log`

At acceptance ceremony ONLY: `cargo xtask test --workspace` dispatched to a sub-agent
returning FACT pass/fail (AGENTS.md gate; guest freshness preflight included).

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md` - direct read; authoritative decisions/approvals record
- `docs/specs/bridge-parity-plan.md` - sections F3/F4 only (delegated SUMMARY acceptable)
- `docs/04_host_scheduler.md` - delegated SUMMARY of ShellClassification/InfillPostProcess commit payloads
- `docs/21_data_defaults_and_fixtures.md` - struct-literal gate rules (ranged read)
- `docs/19_visual_debug.md` - tap classes/bundle guarantees (delegated SUMMARY)
- `docs/ORCASLICER_ATTRIBUTION.md` - porting-header obligation for S5 ports
- `docs/08_coordinate_system.md` - conversion checklist (snippet below carries the factor)

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/specs/bridge-parity-plan.md` section "F3" - replace the 234a addendum's residual-
  divergence text with the closure outcome - `rg -q '### F3 — HIGH' docs/specs/bridge-parity-plan.md`
- `docs/specs/bridge-parity-plan.md` section "F4" - mark coverage/anchoring closed by this
  packet with measured numbers - `rg -q '### F4 — HIGH' docs/specs/bridge-parity-plan.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `bridge_over_infill` gather initialization (fills-as-initial), candidate `filter_by_type(stInternalSolid)` loop, `stInternalBridge` subtraction block; `discover_horizontal_shells` `extra_solid_infills` (deliberately NOT borrowed); `generate_sparse_infill_polylines_for_anchoring`; `gather_areas_w_depth` downward harvesting; thread clustering and `filled_polyons_on_lower_layers` removal; expansion-zone application (`expansion_step`, `expansion_bottom_bridge`)
- `OrcaSlicerDocumented/src/libslic3r/Fill.hpp` — filler/spacing constants used by anchored construction spacing, if referenced by S5 snippets

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
