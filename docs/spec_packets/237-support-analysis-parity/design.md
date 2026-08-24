# Design: 237-support-analysis-parity

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` — `detect_support_contacts` (div 5.3
    stages) + `SupportContactParams` extension; "Not modelled" doc list at the function's
    header is the authoritative inventory of what is missing.
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` —
    `commit_support_analysis_builtin` routing (`enforcer_contacts` currently gated on
    `!support_type.is_auto()` in the per-region `filter_map`), G-17 consumer gating,
    cantilever map population.
  - `crates/slicer-core/src/algos/mesh_analysis.rs` — `classify_object` hardcodes
    `needs_support: true` when synthesizing `OverhangRegion`s (G-17 producer).
  - `crates/slicer-sdk/src/views.rs` — `SliceRegionView::Default`/`from_ir` hardcode the flag;
    new `derive_needs_support` lands here (G-17 view derivation).
- Neighboring tests/fixtures:
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` — existing harness
    (`rect`, `params`, `pillar_then_cap`, `sweep` helpers); host-algos gated via
    `required-features` in `crates/slicer-core/Cargo.toml`.
  - Producer tests: in-file `#[cfg(test)]` module of `support_analysis_producer.rs`
    (`candidates_for`, `config_with_support_type`, `overhang_stack`,
    `manual_support_type_emits_no_auto_detected_candidate`,
    `manual_support_type_emits_enforcer_driven_candidate`), reachable ONLY through the
    crate's `--lib` test target — the `tests/unit/` aggregator does not mount source modules.
  - Both-legs contract home: `crates/slicer-wasm-host/tests/contract/main.rs` aggregator.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not
  repeat delegation rules.

## Architecture Constraints

- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This packet edits `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-core/**`, `crates/slicer-macros/**` — all inside the snippet's applicability list.)
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. (Concretely here: canonical's cantilever threshold `dist_max > scale_(3)` becomes a 3 mm constant compared against mm-space spans, or its `mm_to_units` equivalent — never a raw `3_0000` literal without provenance.)
- Schema/version locking: `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` receives a
  minor-version bump in the step that adds `SupportAnalysisIR.cantilever_surfaces` (live
  constant today: 1.1.0; additive field ⇒ minor bump derived at activation). No artifact freezes a literal:
  tests and docs reference the constant; the expectation is derived from the live value at
  activation time. The bump's test fallout is owned by that step, not deferred to acceptance
  (see Blast Radius below).

## Code Change Surface

- Selected approach:
  - **Div 5.3** — extend `SupportContactParams` with the stage inputs (sharp-tail enable,
    bridge polygons, enforce-layer count, first-layer flag) so `detect_support_contacts`
    stays a pure function; implement each canonical stage inline at its canonical position in
    the existing step sequence (sharp-tails before the diff; bridge removal after blockers;
    enforce-forcing replacing `lower_layer_offset_mm`; cantilever after `union_ex`, returning
    annotations alongside contacts via a small result struct or out-param).
  - **Div 5.2** — restructure the routing branch: enforcer branch condition becomes
    "enforcers non-empty" (any support_type); auto additionally runs the thresholded branch;
    union both geometries; keep `blocked`/`enforced` flags computed against the region and
    surviving geometry as today.
  - **G-17 producer** — `classify_object`: `needs_support` derives from whether the overhang
    facet cluster exists at all (an `OverhangRegion` is only synthesized when overhang facets
    exist, so the flag stays `true` there but the *absence* of any region now means
    ineligible — the real signal is region-level presence plus footprint overlap).
  - **G-17 view derivation** — add
    `SliceRegionView::derive_needs_support(Option<&SurfaceClassificationIR>)`; `from_ir`
    callers (native marshal, wasm marshal, macro shim keeps forwarding the accessor) pass the
    committed classification through `LayerStageInput.surface_classification`. A region whose
    polygons are disjoint from every overhang `xy_footprint` of its object → false.
  - **G-17 consumer** — in the producer's contact loop, skip thresholded candidates whose
    source region derived ineligible-without-enforcer; record the decline implicitly by still
    minting the per-region family assignment (Ruling 1 compliance).
  - **Doc comment correction** — `SupportType::NormalAuto` / `is_auto` docs
    (`crates/slicer-ir/src/slice_ir.rs`): enforcers apply under ALL support types; only the
    angle-thresholded branch is auto-gated.
- Exact functions, traits, manifests, tests, fixtures:
  - `detect_support_contacts`, `SupportContactParams`, `lower_layer_offset_mm`
    (`overhang_annotation.rs`)
  - `commit_support_analysis_builtin`, `enforcer_contacts`, `resolve_contact_params`
    (`support_analysis_producer.rs`)
  - `classify_object` (`mesh_analysis.rs`)
  - `SliceRegionView::{default, from_ir, derive_needs_support}` (`views.rs`)
  - `sliced_region_to_data` (`in_.rs`), `build_native_layer_request` (`native.rs`),
    macro adapter (`slicer-macros/src/lib.rs`)
  - `SupportAnalysisIR`, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` (`slice_ir.rs`),
    `SupportType` doc comments (`slice_ir.rs`)
  - Tests: `support_overhang_detection_tdd.rs` (new names AC-2/3/4/5, N1/N2/N3),
    producer unit module (AC-1, N5), `layer_module_tdd.rs` (AC-6 family), wasm-host
    `contract` module (AC-7), runtime `integration` (AC-8)
- Rejected alternatives and reasons:
  - *Planner-side eligibility re-litigation* (renderer checks `needs_support` again): rejected
    — this is exactly the pre-224 defect; decision 2 stands (AC-N4).
  - *WIT schema change to transport eligibility per-candidate*: rejected —
    `slice-region-view.needs-support` already exists in `deps/ir-types.wit` and both legs
    marshal it; only the value derivation was missing.
  - *Implementing buildplate_covered with a synthetic host-side approximation*: rejected —
    invents data the pipeline does not produce; recorded `[FWD]` instead.
  - *Separate `support_contact_canonical_stages_tdd` binary*: dropped during self-review —
    extending the existing gated `support_overhang_detection_tdd` avoids a Cargo.toml edit and
    binary-count churn (E6 reconciliation stays trivial).

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/overhang_annotation.rs` — div 5.3 stages; expected change:
  params struct + four stage implementations + updated "Not modelled" doc list.
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` — div 5.2 routing, G-17
  consumer gating, cantilever population, param plumbing; expected change: routing branch +
  candidate filter + result mapping.
- `crates/slicer-sdk/src/views.rs` — G-17 view derivation; expected change: one method +
  from_ir signature/callers.
Justified extras (each is a thin mechanical surface, split across steps):
- `crates/slicer-core/src/algos/mesh_analysis.rs` — producer hardcode removal (one expression).
- `crates/slicer-wasm-host/src/marshal/native.rs`, `crates/slicer-wasm-host/src/marshal/in_.rs`,
  `crates/slicer-macros/src/lib.rs` — leg wiring (pass-through calls only).
- `crates/slicer-ir/src/slice_ir.rs` — additive field + version constant + doc comments.
- Test files listed under Neighboring tests/fixtures.

## Read-Only Context

- `docs/specs/support-families-anchored-entities-plan.md` — lines 82–120 (rulings), 142–196
  (invariants/evidence), 342–374 (237 brief) only — authority for behavior decisions.
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` — 5.2/5.3
  rows only.
- `modules/core-modules/traditional-support-planner/src/lib.rs` — consumer shape reference
  (`plan_for_object` filters candidates by family; `plan_candidate` declines on
  `candidate.blocked`) — informs
  AC-8 expectations, do not edit.
- `crates/scheduler`-adjacent: none. `crates/slicer-scheduler/src/validation.rs` is READ-ONLY
  if consulted at all (236-owned).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load (E7/T1).
- `target/`, `Cargo.lock`, generated code, vendored dependencies, guest build artifacts under
  `modules/core-modules/*/wit-guest/target/` — never load.
- `crates/slicer-scheduler/src/validation.rs` write-conflict logic — 236-owned; read-only.
- Other packet directories under `docs/spec_packets/` — never modify (Packet Safety).
- `docs/15_config_keys_reference.md` and any module manifest `.toml` — 238a owns key
  declarations; this packet adds none.
- Renderer modules (`modules/core-modules/tree-support/src/lib.rs`,
  `traditional-support`) — their inversion comments stay as-is; AC-N4 only greps them.

## Expected Sub-Agent Dispatches

- Question: confirm how source-module `#[cfg(test)]` tests of
  `support_analysis_producer.rs` are reached from a narrow cargo invocation (`--lib` vs any
  aggregator); scope: `crates/slicer-runtime/tests/unit/main.rs` + Cargo.toml test targets;
  return: `LOCATIONS`; purpose: AC-1/N5 command correctness before authoring Step 4.
- Question: enumerate struct-literal sites compiling against `SupportAnalysisIR` outside
  slicer-ir (fixture builders, marshalling, tests); scope: `crates/ --include *.rs`; return:
  `LOCATIONS` (≤20); purpose: blast-radius list for the schema-bump step.
- Question: verify `LayerStageInput.surface_classification` reaches both marshal call sites
  unchanged; scope: `crates/slicer-wasm-host/src/{binding,dispatch,marshal/native}.rs`;
  return:
  `FACT`; purpose: de-risk the leg wiring steps. Known baseline: the native leg
  (`build_native_layer_request`) does not read the field yet — this packet threads it.
- Question: locate `remove_bridges_from_contacts` semantics (inputs, offset magnitudes,
  return) in canonical; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`;
  return: `SNIPPETS` (≤30 lines); purpose: faithful port in Step 2.

## Data and Contract Notes

- IR/manifest contracts: `SupportAnalysisIR.cantilever_surfaces` mirrors
  `model_occupancy`'s key type (`SupportGeometryKey`) and is host-only like
  `overhang_quartile_polygons` (serde default keeps old fixtures loading; the additive minor
  bump follows the established pattern, cf. `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`
  history). No WIT change in this packet: `cantilever_surfaces` projection is deferred to
  238c, following the `prev_layer_boundaries` precedent.
- WIT boundary: untouched. `slice-region-view.needs-support`
  (`crates/slicer-schema/wit/deps/ir-types.wit`) already exists; guest artifacts remain fresh
  because WIT text is unchanged (fingerprint may still flip on dependency-closure changes —
  hence the staleness gate).
- Determinism/scheduler constraints: candidate stream must stay byte-stable —
  `contact_work.sort_by` ordering and rayon collect-order guarantees are preserved; the new
  enforcer-union must not reorder candidates (union geometry replaces the thresholded-only
  geometry in place). Per-region `family_assignments` minting order and content are
  236-owned: this packet composes, never reverts to per-candidate minting (Ruling 1).
  Suppressed candidates still yield their region's structured assignment entry.

## Locked Assumptions and Invariants

- Invariant 15 (Ruling 1): every RegionMap region keeps exactly one attributed plan-entry
  source; suppression produces a declined record, never silence.
- Invariant 16 / T2: every verification command asserts non-zero matched tests.
- E1/T6: the replacement for the deleted vacuous test asserts real produced signal (AC-8);
  `enforcer_overrides_needs_support_false` stays deleted.
- E6/T5: every slicer-core test command carries `--features host-algos`.
- 224 decision 2 renderer inversion preserved (AC-N4 greps both directions).
- Config keys touched by behavior here (`bridge_no_support`, `enforce_support_layers`,
  `support_sharp_tails`) are NOT declared in manifests by this packet (238a owns
  declarations; E9
  silent-default mechanism acknowledged — the in-code sharp-tails default is a transitional
  OFF scaffold until 238a declares the key canonical-true).

## Risks and Tradeoffs

- The enforcer-union under auto changes candidate geometry for every auto-configured run with
  enforcers — golden reblessing may be requested downstream (238b tree goldens); classify any
  drift per E3, never silently regenerate.
- `derive_needs_support` misclassification risk (footprint-vs-polygon disjointness is a
  conservative proxy for facet-level classification): mitigated by AC-8's whole-run fixture
  and the human gate's coverage check; the proxy errs toward *not* suppressing.
- Schema bump ripples into serde fixtures/marshalling literals beyond slicer-ir (blast-radius
  dispatch pre-bakes the list into the owning step).
- Bridge-removal port needs exact canonical offset magnitudes; wrong scaling violates E8 —
  the SNIPPETS dispatch pins the constants before implementation.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — five-stage implementation touches the densest file)
- Highest-risk dispatch and required return format: canonical
  `remove_bridges_from_contacts` semantics — `SNIPPETS` ≤30 lines.

## Open Questions

- `[FWD]` Buildplate-covered subtraction transport: no host-side producer of
  buildplate annotations exists (`SurfaceClassificationIR` carries none; SliceIR carries
  none; WIT carries none). Recommendation: resolve alongside 242 (which already owns an
  annotation-channel decision for the rasterizer) or log a reasoned DEVIATION then. This
  packet ships four of five divergence-5.3 stages and records the fifth as forwarded —
  explicitly not a silent scope-cut.
- `[FWD]` `cantilever_surfaces` WIT projection timing: host-only-first following
  `prev_layer_boundaries` precedent; 238c decides projection shape when it consumes renderer
  flow. Implementer may proceed host-only without blocking.
- `[FWD]→238a` Confirm `bridge_no_support` manifest spelling at declaration time; this
  packet's typed parameter name (`bridge_no_support: bool`) anticipates it.
- Implementer-resolvable: whether the enforcer-union sets `enforced = true` on candidates
  whose geometry partially originates from the enforcer branch (recommended: yes — the flag
  describes policy retention, and enforcer-derived area is retained by policy). Record the
  choice in the Step 4 commit message; no packet change needed.
