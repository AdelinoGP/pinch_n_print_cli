---
status: draft
packet: 237-support-analysis-parity
task_ids:
  - TASK-353
  - TASK-354
  - TASK-355
  - TASK-356
  - TASK-357
  - TASK-358
  - TASK-359
  - TASK-360
  - TASK-361
  - TASK-362
depends_on: 236-support-stabilization
backlog_source: docs/specs/support-parity-gap-register.md
context_cost_estimate: M
---

# Packet Contract: 237-support-analysis-parity

## Goal

Make host support analysis canonical-faithful: give `needs_support` a real producer-to-consumer
signal (gap G-17), route enforcer contacts under auto `support_type` like canonical
`detect_contacts` (divergence 5.2), and implement the five missing `detect_overhangs` stages in
`detect_support_contacts` (divergence 5.3).

## Scope Boundaries

The packet rewires eligibility signal production (`classify_object`, `SliceRegionView`
derivation, both wasm/native marshal legs) and consumption (candidate gating in
`commit_support_analysis_builtin`), fixes the auto/manual enforcer-routing split, and adds the
sharp-tail, bridge-removal, cantilever, and enforce-support-layers stages to
`detect_support_contacts`. Renderer and planner geometric fidelity, rasterizer choice, and the
`buildplate_covered` annotation transport are excluded (see `requirements.md` §Out of Scope).

## Prerequisites and Blockers

- Depends on: `236-support-stabilization` — FORWARD DEPENDENCY on a `status: draft` packet.
  This packet composes with 236's per-region
  `family_assignments` minting in `commit_support_analysis_builtin` — this packet must not
  revert minting to per-candidate. Steps that touch the shared routing/consumer code path
  (Steps 4, 7) are gated: they may author red-first tests against the current tree, but their
  green-light composition checks re-run after 236 reaches `status: implemented`; if 236's
  landed shape diverges from this packet's composition assumptions, Steps 4/7 reconcile
  before closing.
- Unblocks: `238a-support-pattern-config-keys` (declares `bridge_no_support` and
  `support_sharp_tails`, whose consuming behavior
  lands here), `238b`/`238c` (consume the corrected candidate stream and cantilever surfaces).
- Activation blockers: none for authoring/red-test steps. `[FWD]` questions in `design.md`
  are implementer-resolvable or deferred with recorded decisions. The 236 forward dependency
  gates closure of Steps 4 and 7, not activation.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a stack whose upper layer carries a support enforcer over geometry that is
  *not* an angle-thresholded overhang, **when** `commit_support_analysis_builtin` runs with
  `support_type = "normal(auto)"`, **then** the committed `SupportAnalysisIR.candidates`
  include the enforcer-derived contact geometry (canonical `detect_contacts` runs its enforcer
  branch whenever `annotations.enforcers_layers[layer_id]` is non-empty, with no support-type
  gate) alongside any thresholded contacts. The test lives in the producer's in-file
  `#[cfg(test)]` module, reached through the crate's library test target (`--lib`); the
  `tests/unit/` aggregator does NOT mount source modules.
  | `mkdir -p target && cargo test -p slicer-runtime --lib auto_support_type_unions_enforcer_contacts_with_thresholded -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-2. Given** `support_sharp_tails` behavior enabled in `SupportContactParams` and a
  layer-0 sharp-tail profile, **when** `detect_support_contacts` runs, **then** it returns
  non-empty tail contacts for the first layer (canonical `detect_overhangs` sharp-tail branch,
  gated `g_config_support_sharp_tails`, evaluated at `layer_id == 0`).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd sharp_tails_add_first_layer_contacts_when_enabled -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-3. Given** a contact region overlapping `bridge_areas` with bridge removal enabled,
  **when** `detect_support_contacts` runs, **then** the bridge-overlapping area is absent from
  the returned contacts (canonical `SupportMaterialInternal::remove_bridges_from_contacts`,
  gated `bridge_no_support`; the consuming behavior lands here, the key declaration belongs to
  238a per the queue plan — 238a also declares `support_sharp_tails`, AC-2/AC-N1's key).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd bridge_areas_are_removed_from_contacts_under_bridge_no_support -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-4. Given** a leading-layer region with `enforce_support_layers > 0` covering it,
  **when** `detect_support_contacts` runs, **then** the contact equals the full cross-section
  minus the raw lower layer (canonical forces `lower_layer_offset = 0` for
  `layer_id < enforce_support_layers`, ignoring the angle-threshold offset).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd enforce_support_layers_forces_full_contacts_in_leading_layers -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-5. Given** an overhang union whose cantilever span exceeds canonical's 3 mm threshold
  (`dist_max > scale_(3)`; 3 mm via `mm_to_units`), **when** the post-union cantilever pass
  runs, **then** it records cantilever polygons into the new additive
  `SupportAnalysisIR.cantilever_surfaces` map, and
  `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` carries a minor-version bump derived from the
  live constant at activation time (live today: 1.1.0 → additive field ⇒ minor bump, but
  no literal is asserted — tests and docs reference the constant, never a frozen literal).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd cantilever_pass_records_wide_overhang_annotations -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-6. Given** a sliced region whose object has an `OverhangRegion.xy_footprint` disjoint
  from the region's polygons, **when** `SliceRegionView::derive_needs_support` consults
  `SurfaceClassificationIR`, **then** `needs_support()` returns `false` (real signal replaces
  the `from_ir`/`Default` hardcodes; `Default::default()` intentionally stays `true` for
  legacy fixtures).
  | `mkdir -p target && cargo test -p slicer-sdk --test layer_module_tdd derive_needs_support -- 2>&1 | tee target/test-output.log && grep -E "test result: ok.*([1-9][0-9]*) passed" target/test-output.log >/dev/null && echo PASS`
- **AC-7. Given** the eligibility derivation wired on both transport legs, **when** a region
  with disjoint overhang footprint is marshalled for a support-family stage, **then** the
  native leg (`build_native_layer_request`) and the wasm leg (`sliced_region_to_data`) both
  deliver `needs_support == false` (trap T9: neither leg may be skipped). The test module is
  registered in `crates/slicer-wasm-host/tests/contract/main.rs` (aggregator `[[test]]`
  binary `contract` in `crates/slicer-wasm-host/Cargo.toml`).
  | `mkdir -p target && cargo test -p slicer-wasm-host --test contract region_eligibility -- 2>&1 | tee target/test-output.log && grep -E "test result: ok.*([1-9][0-9]*) passed" target/test-output.log >/dev/null && echo PASS`
- **AC-8. Given** a whole-run fixture containing an ineligible region (disjoint overhang
  footprint, no enforcer), **when** the support-analysis stage commits, **then** no
  auto-detected candidate exists for that region while its structured
  `family_assignments` entry (per-region minting, Ruling 1) is still present — producers
  proven to flow, downstream planners proven to see the decline.
  | `mkdir -p target && cargo test -p slicer-runtime --test integration needs_support_false_region_yields_no_auto_candidates -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** sharp-tail behavior disabled (absent key resolves to the in-code default,
  a transitional OFF scaffold until 238a declares `support_sharp_tails` bool-true; canonical
  `g_config_support_sharp_tails` is a developer constant set true),
  **when** `detect_support_contacts` runs on the AC-2 profile, **then** it returns no
  first-layer tail contacts (the stage must be opt-in, mirroring the canonical gate).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd sharp_tails_disabled_by_default_emits_none -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N2. Given** bridge removal disabled, **when** `detect_support_contacts` runs on the
  AC-3 profile, **then** bridge-overlapping contact area survives (behavior is gated, not
  unconditional).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd bridge_removal_disabled_keeps_bridge_contacts -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N3. Given** `enforce_support_layers` at or beyond the region's layer, **when**
  `detect_support_contacts` runs, **then** the forced-full-contact behavior does **not** apply
  and the ordinary angle-thresholded offset governs (boundary guard against over-forcing).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd enforce_support_layers_beyond_model_changes_nothing -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N4. Given** the packet closed, **when** the tree-support renderer suite runs, **then**
  the vacuous test `enforcer_overrides_needs_support_false` remains absent from
  `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` (E1: its replacement is
  the real-signal AC-8, never a resurrection of the empty-assertion original; the sibling
  `planned_region_renders_regardless_of_eligibility_flag` must still exist and pass — 224
  decision 2's renderer inversion stands).
  | `(rg -q "fn enforcer_overrides_needs_support_false" modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs && echo FAIL-VACUOUS-RESURRECTED || echo PASS) && (rg -q "fn planned_region_renders_regardless_of_eligibility_flag" modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs && echo PASS || echo FAIL-INVERSION-LOST)`
- **AC-N5. Given** the manual `support_type` values, **when** the existing manual-routing
  regression tests run after the AC-1 routing change, **then**
  `manual_support_type_emits_no_auto_detected_candidate` and
  `manual_support_type_emits_enforcer_driven_candidate` still pass (manual routing is
  unchanged by the auto-path enforcer union).
  | `mkdir -p target && cargo test -p slicer-runtime --lib manual_support_type -- 2>&1 | tee target/test-output.log && grep -E "test result: ok.*([2-9]|[1-9][0-9]) passed" target/test-output.log >/dev/null && test "$(grep -c '^test .* ok$' target/test-output.log)" -ge 2 && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (exit 0 — this packet's surface feeds guest WASM;
  E4/T4 freshness gate before attributing any guest-facing failure)
- `mkdir -p target && cargo test -p slicer-core --features host-algos --no-fail-fast --test support_overhang_detection_tdd 2>&1 | tee target/test-output.log && grep -cE "test result: (ok|FAILED)" target/test-output.log` (E6/T5: `--features host-algos` mandatory; assert exactly one binary ran)

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §3 rulings, §6 invariants 15–16,
  §7 evidence standards E1–E9, §8 human validation gate, §13 traps T1/T5/T6/T8, §12 brief
  "237-support-analysis-parity" (direct ranged reads at authoring time)
- `docs/specs/support-parity-gap-register.md` - row G-17 (destination updated to this packet)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` -
  divergences 5.2 and 5.3 (delegated SUMMARY; canonical evidence pre-verified and restated in
  `requirements.md`)
- `docs/02_ir_schemas.md` - §IR 2 `needs_support` eligibility semantics (ranged read before
  editing; see Doc Impact Statement)

## Human Validation Gate

Blocking per plan §8: this packet may not flip to `status: implemented` without a dated
sign-off line at the bottom of this section.

Artifacts to produce (all under `tmp/`, which is gitignored — verify by direct listing, trap
T1):

1. Tree G-code: slice the tracked `tmp/SupportTest.stl` with the matched tree profile
   `tmp/support-family-config-tree-matched.json`.
2. Traditional G-code: same fixture with `tmp/support-family-config-normal-matched.json`.
3. Visual-debug bundle for THIS packet's boundary — enforcer/eligibility classification: a
   bundle capturing the enforcer-painted region(s) and the `needs_support`-declined region(s)
   at their decisive layers. If no enforcer-painted fixture variant exists, use a synthetic
   enforcer case (a modifier volume painted `SupportEnforcer` over a non-overhanging slab) and
   record its construction in the evidence file.

Checklist to sign (each item names source, layer, tap, verdict; per E2 this is written
inspection, never a test claim):

- Termination: declined regions produce no support columns reaching the plate beneath them.
- Coverage: enforcer-covered geometry still receives support under auto `support_type`
  (AC-1 behavior visible in the render).
- Collision freedom: forced-full contacts from `enforce_support_layers` do not intersect the
  model on their own layers.
- Interfaces: existing top/bottom interface bands are unchanged where the new stages do not
  apply.
- Block counts vs Orca references: candidate/plan-entry counts for the fixture measured
  against `tmp/SupportTest_Tree_Orca.gcode` / `tmp/SupportTest_Normal_Orca.gcode`, recorded as
  numeric deltas in the evidence file.

Evidence file: `tmp/237-human-validation.md` recording commands run, artifact paths, layer
indices inspected, and the block-count deltas.

Sign-off: `YYYY-MM-DD — <verdict>` (pending).

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "IR 2" (`needs_support` eligibility paragraph: producer now
  sets `false` from overhang-footprint derivation; `derive_needs_support` contract) -
  `rg -q 'derive_needs_support|needs_support' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` section "SupportAnalysisIR" (additive `cantilever_surfaces` map,
  schema minor bump) - `rg -q 'cantilever_surfaces' docs/02_ir_schemas.md`
- `docs/07_implementation_status.md` - TASK-353..362 rows registered by the packet-owned
  closure step (TASK-362), per `task-map.md` - `rg -q 'TASK-362' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_overhangs`: the
  five divergent stages (sharp-tail detection gated `g_config_support_sharp_tails`;
  buildplate-only subtraction of `annotations.buildplate_covered[layer_id]`;
  `remove_bridges_from_contacts` under `bridge_no_support`; the post-union cantilever pass
  recording `layer.cantilevers` when `dist_max > scale_(3)`; `lower_layer_offset = 0` forcing
  under `enforce_support_layers`) plus the tiny-spot filter and
  `support_threshold_overlap` overlap-offset alternative being mirrored already
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_contacts`: the
  enforcer branch running purely on
  `has_enforcer = !annotations.enforcers_layers.empty() && !annotations.enforcers_layers[layer_id].empty()`
  with no `support_type`/auto gate (divergence 5.2's canonical half)
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` —
  `SupportMaterialInternal::remove_bridges_from_contacts`: the bridge-area subtraction the
  AC-3 behavior ports

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
