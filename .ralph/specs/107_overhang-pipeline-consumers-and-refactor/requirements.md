# Requirements: 107_overhang-pipeline-consumers-and-refactor

## Packet Metadata

- Grouped task IDs:
  - `O-T030` — Confirm `SliceRegionView::overhang_areas()` (added by P104 as stub) now returns non-empty data after P106's `xy_footprint` populator ships
  - `O-T031` — Add `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` accessor (pre-filtered to this region's polygon area)
  - `O-T032` — Mirror accessors on `PaintRegionLayerView` / `SurfaceClassificationView` if applicable; pick naming consistent with `bridge_areas()`
  - `O-T040` — Refactor `overhang-classifier-default` to read `Point3WithWidth.overhang_quartile` from `LayerCollectionView` entities and apply speed factors only
  - `O-T041` — Delete the now-redundant `classify.rs` + `lines_distancer.rs`; register supersession in `DEVIATION_LOG.md`
  - `O-T042` — Update `overhang-classifier-default.toml` manifest: drop wall-geometry `LayerCollectionIR` reads; add narrower `overhang_quartile` read declaration
  - `O-T050` — End-to-end overhang-quartile propagation TDD: overhang ramp → P106 PrePass → perimeter walls → view accessor → finalization speed factors
  - `O-T051` — Pre-vs-post-refactor regression check on benchy / standard fixtures
  - `O-T052` — Update `docs/01_system_architecture.md` (Tier 1 + Tier 3 blocks) and `docs/02_ir_schemas.md` (consumer documentation)
  - `O-T053` — Close perimeter-roadmap deviations D-10, D-12, D-OVERHANG-QUARTILE-NONE; mark T-024 / T-077 unblocked
- Backlog source: `docs/specs/overhang-pipeline-restructuring.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P106 (draft) will land the PrePass-side foundation of the overhang pipeline restructuring — `OverhangRegion.xy_footprint` field already exists in `slice_ir.rs:581` (P106 populates it at runtime), `SurfaceClassificationIR.overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` and `QuartileBand` are P106 FORWARD-DEPs not yet in the tree. Without this consumer-side packet, that data is stranded on the Blackboard: no view accessor exposes the quartile polygons to Tier 2 modules; `overhang-classifier-default` still runs its old per-entity wall-distance computation (the algorithm ADR-0022 — authored by P106 at the next free ADR slot, 0022 — supersedes for classification purposes); and the perimeter-roadmap decisions D-10 / D-12 (in the roadmap) plus the to-be-registered deviation `D-104-OVERHANG-QUARTILE-NONE` stay open. T-077 (`extra_perimeters_on_overhangs`) in P108 cannot transition from its previously-planned no-op pattern to a real consumer until this packet ships the view accessor on top of P106's data.

This packet closes all four concerns. The view accessor + WIT mirror + host populator are mechanical extensions of the patterns established by `bridge_areas()`. The `overhang-classifier-default` refactor shrinks the module from ~100 LOC + 2 helper files to ~50 LOC of pure consumer logic that reads `Point3WithWidth.overhang_quartile` and emits `EntityMutation::SetSpeedFactor`. The end-to-end TDD validates the full data path from mesh through gcode. The regression check confirms the refactor preserves observable behaviour within calibrated tolerances. The closure pass turns three open deviations into resolved/superseded entries and unblocks two perimeter-roadmap tasks.

## In Scope

- `crates/slicer-sdk/src/views.rs`: add `pub fn overhang_quartile_polygons(&self) -> &[QuartileBand]` accessor (consumes `QuartileBand` — FORWARD-DEP on draft P106); add `pub fn overhang_areas(&self) -> &[ExPolygon]` and `pub fn surface_group(&self) -> Option<&SurfaceGroup>` if P104 has not yet shipped them (both are FORWARD-DEPs on draft P104 — neither exists in tree yet).
- `crates/slicer-schema/wit/deps/ir-types.wit`: WIT mirror for the new accessor.
- `crates/slicer-wasm-host/src/host.rs`: `SliceRegionData` field + populator fills the new field at view-construction (pre-filters per-region quartile polygons from `SurfaceClassificationIR.overhang_quartile_polygons` by intersecting with the region's polygon).
- Possibly `PaintRegionLayerView` / `SurfaceClassificationView` mirror per O-T032 — implementer evaluates based on actual consumer needs; default to no mirror unless a downstream consumer is named.
- `modules/core-modules/overhang-classifier-default/src/lib.rs`: refactor to consumer-only (read quartile, apply speed factor).
- `modules/core-modules/overhang-classifier-default/src/classify.rs` and `lines_distancer.rs`: delete.
- `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`: drop broad `LayerCollectionIR` reads; declare narrow `overhang_quartile` read.
- 3 new TDD files: contract test for view accessor non-empty (AC-2), end-to-end pipeline test (AC-5 + AC-N1), regression check (AC-6).
- `docs/DEVIATION_LOG.md`: register new entry `D-104-OVERHANG-QUARTILE-NONE` (ID-conformant with `D-<pkt>-<SLUG>` convention) and mark it closed. **D-10 and D-12 live only in `docs/specs/perimeter-modules-orca-parity-roadmap.md` — not in DEVIATION_LOG.md; do not grep DEVIATION_LOG.md for them.**
- `docs/specs/perimeter-modules-orca-parity-roadmap.md`: update D-10 and D-12 closure notes to reference P107; mark T-024 + T-077 as unblocked.
- `docs/01_system_architecture.md`, `docs/05_module_sdk.md`, `docs/02_ir_schemas.md` per Doc Impact Statement.

## Out of Scope

- Any IR addition or change — all IR work landed in P106.
- The classifier algorithm itself — landed in P106 (`overhang_annotation.rs`).
- Perimeter module source changes — P104's `overhang_quartile = None` shipping code path stays as documented; this packet doesn't rewire it. If a follow-up packet is needed to wire P104's code path to consume `overhang_quartile_polygons()`, it's tracked separately under perimeter-roadmap follow-ups.
- T-077 actual implementation — that's P108 (the renamed P106-special-modes packet), which becomes operational once this packet ships.
- The non-planar wall pipeline (P104 T-074b/c/d) — separate workstream.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/overhang-pipeline-restructuring.md` | ~150 lines | Range-read Phase 3/4/5. |
| `docs/adr/0008-overhang-as-finalization-module.md` | ~30 lines | Read full. |
| `docs/adr/0022-overhang-classification-at-prepass.md` | ~40 lines (from P106 — FORWARD-DEP; does not exist yet; ADR slot 0022 is next free after 0021) | Read full once P106 ships. |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~700 lines | Range-read D-10/D-12 + T-024/T-077 entries. |
| `docs/05_module_sdk.md` | ~500 lines | Delegate SUMMARY for `SliceRegionView` accessor convention. |
| `docs/DEVIATION_LOG.md` | varies | Range-read the three target entries. |
| `CLAUDE.md` | ~600 lines | Range-read §"Guest WASM Staleness" + §"WIT/Type Changes Checklist". |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- None new. P106's `detect_steep_overhang` SUMMARY established the algorithm; this packet's consumer side is workspace-internal.

## Acceptance Summary

- Positive cases: `AC-1` (view accessor added), `AC-2` (P104's stub now returns non-empty), `AC-3` (module refactored + auxiliary files deleted), `AC-4` (manifest narrowed), `AC-5` (end-to-end propagation works), `AC-6` (regression vs pre-refactor within tolerance), `AC-7` (deviation closures + T-024/T-077 unblocked).
- Negative case: `AC-N1` (no overhang → no SetSpeedFactor mutations, quartile stays None).
- Refinements not captured in Given/When/Then:
  - The `≤ 80 LOC` line-count target in AC-3 is a sanity bound, not a perfectionist gate. If the implementer cleanly lands at 85 LOC with no dead code, document and proceed; if it's at 120 LOC there's likely leftover machinery to delete.
  - AC-5's "if P104 propagation still ships None" branch: if encountered, the implementer registers a follow-up task `T-024-WIRE-VIEW-CONSUMER` (or similar) in the perimeter roadmap and notes in the closure log. Do not block this packet on rewiring P104.
- Cross-packet impact: depends on P106 (IR + data) + P104 (stub accessor). Unblocks P108 (T-077) and a perimeter-roadmap follow-up wiring T-024.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after view + WIT additions | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract slice_region_view_overhang_areas_non_empty_tdd` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd` | AC-5 + AC-N1 | FACT pass/fail per case |
| `cargo test -p slicer-runtime --test integration overhang_classifier_refactor_regression_tdd` | AC-6 regression | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence after WIT change | FACT clean / STALE list |
| `[ $(wc -l < modules/core-modules/overhang-classifier-default/src/lib.rs) -le 80 ]` | AC-3 LOC bound | FACT pass/fail |
| `rg -q 'D-10.*(P107\|closed\|resolved)' docs/specs/perimeter-modules-orca-parity-roadmap.md` | AC-7 D-10 closure (in roadmap, not DEVIATION_LOG.md) | FACT pass/fail |
| `rg -q 'D-104-OVERHANG-QUARTILE-NONE.*(closed\|resolved)' docs/DEVIATION_LOG.md` | AC-7 new deviation log entry | FACT pass/fail |
| `rg -q 'T-024.*unblocked\|T-024.*preconditions met' docs/specs/perimeter-modules-orca-parity-roadmap.md` | AC-7 unblock marker | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: existing overhang-classifier-default benchy / standard regression fixtures must stay within calibrated tolerance throughout (AC-6 catches drift). If pre-existing fixtures aren't recorded, Step 4 records them at the start (using the pre-refactor module) before the refactor lands.
- Step ordering rationale: view accessor first (Step 1) so the module refactor (Step 2) can consume it. End-to-end TDD (Step 3) before regression check (Step 4) because the e2e test confirms the pipeline works at all; the regression check confirms it's faithful. Deviation closure (Step 5) last because it depends on all four prior steps being green.
- Shared scratch state: none.

## Context Discipline Notes

- `modules/core-modules/overhang-classifier-default/src/lib.rs` is ≤ 200 LOC pre-refactor; read full.
- `modules/core-modules/overhang-classifier-default/src/classify.rs` — read full once to understand what's being deleted; then DO NOT re-read. The algorithm being deleted is the wall-distance computation; the implementer doesn't need to preserve it.
- `crates/slicer-sdk/src/views.rs` — range-read by `rg -n 'fn (bridge_areas|overhang_areas|surface_group)'` then ±40 lines.
- Likely temptation read: `crates/slicer-core/src/algos/overhang_annotation.rs` (from P106) to verify the data shape. **Delegate FACT** instead — confirm the field signature; don't load the implementation.
- Sub-agent return-format for the heaviest dispatch: the pre-vs-post regression check (AC-6) runs the slicer twice on the same fixtures. If the implementer runs both manually, the output is large; delegate the comparison to a sub-agent that returns FACT (within tolerance / outside tolerance + specific diff).
