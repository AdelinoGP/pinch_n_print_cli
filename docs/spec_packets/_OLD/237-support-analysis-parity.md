---
status: implemented
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
---

# 237-support-analysis-parity

## Goal

Make host support analysis canonical-faithful: give `needs_support` a real producer-to-consumer
signal (gap G-17), route enforcer contacts under auto `support_type` like canonical
`detect_contacts` (divergence 5.2), and implement the five missing `detect_overhangs` stages in
`detect_support_contacts` (divergence 5.3).

## Motivation

Host support analysis is not canonical-faithful in three ways, each measured and registered:

1. **G-17 — `needs_support` carries no signal.** `classify_object`
   (`crates/slicer-core/src/algos/mesh_analysis.rs`) hardcodes
   `needs_support: true` on every synthesized `OverhangRegion`, and
   `SliceRegionView::Default`/`from_ir` (`crates/slicer-sdk/src/views.rs`) hardcode the flag
   `true` as well. No producer ever sets it false, so planner- and renderer-side consumers of
   the documented eligibility precedence (`crates/slicer-sdk/src/traits.rs` `run_support`
   doc-comment) consume a constant. Packet 224 decision 2 kept the renderer-side inversion and
   deleted the vacuous test `enforcer_overrides_needs_support_false`; this packet restores the
   signal by real classification (E1), never by resurrecting that test.
2. **Divergence 5.2 — enforcers ignored under auto.**
   `commit_support_analysis_builtin`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) calls `enforcer_contacts`
   only when `!support_type.is_auto()`. Canonical `detect_contacts`
   (`SupportMaterial.cpp`) runs its enforcer branch whenever `has_enforcer`
   (`annotations.enforcers_layers[layer_id]` non-empty) with **no** support-type gate; the
   auto gate applies only to `detect_overhangs`' angle-thresholded branch.
3. **Divergence 5.3 — five missing stages.** `detect_support_contacts`
   (`crates/slicer-core/src/algos/overhang_annotation.rs`) implements diff → expand-back →
   blockers → tiny-spot filter → XY expansion → union and self-documents a "Not modelled"
   list: sharp-tail detection, buildplate-only subtraction, bridge removal, the cantilever
   pass, and `enforce_support_layers` forcing.

## Architecture Constraints

- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This packet edits `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-core/**`, `crates/slicer-macros/**` — all inside the snippet's applicability list.)
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. (Concretely here: canonical's cantilever threshold `dist_max > scale_(3)` becomes a 3 mm constant compared against mm-space spans, or its `mm_to_units` equivalent — never a raw `3_0000` literal without provenance.)
- Schema/version locking: `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` receives a
  minor-version bump in the step that adds `SupportAnalysisIR.cantilever_surfaces` (live
  constant today: 1.1.0; additive field ⇒ minor bump derived at activation). No artifact freezes a literal:
  tests and docs reference the constant; the expectation is derived from the live value at
  activation time. The bump's test fallout is owned by that step, not deferred to acceptance
  (see Blast Radius below).

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
