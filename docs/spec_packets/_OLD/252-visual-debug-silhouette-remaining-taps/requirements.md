# Requirements: 252-visual-debug-silhouette-remaining-taps

## Packet Metadata

- Grouped task IDs: `TASK-458`, `TASK-459`, `TASK-460`, `TASK-461` (new rows; crosswalk in `task-map.md`)
- Backlog source: `docs/07_implementation_status.md` (no existing open TASK covers this; the gap is packet 247's `[FWD]` "unowned taps" question, queue row #6 approved 2026-08-27)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 247 built the silhouette composite path but rejected two Z-attributable taps from the plan's D8 whitelist with interim `SilhouetteUnsupportedForTap` errors, and no queue row owned them (247's design.md `[FWD]`, echoed by 249/250/251). `PrePass::RegionMapping` is mechanically ready — its capture (`CapturedIr::RegionMapping`, `crates/slicer-runtime/src/layer_executor.rs`) retains the whole-print `Vec<SliceIR>`, so per-region slabs are self-contained; it needs a determinism rule for dynamic `config_tint` classes. `PrePass::OverhangAnnotation` is the hard half — its capture (`CapturedIr::SurfaceClassification`) carries the per-layer-keyed `overhang_quartile_polygons` bands but **no** per-region heights, and D1 forbids schedule z-diff slabs for SliceIR-height-capable taps (catch-up regions reach below the previous global Z). This packet closes the whitelist with honest slabs for both.

## In Scope

- **RegionMapping silhouette** (renderer): a `CapturedIr::RegionMapping` extraction arm in the composite extraction shared by `render_silhouette_composite` (packet 247's export; after packet 249's refactor the arm lives in the shared internals its styled entry delegates to — behavior-level contract, seam-agnostic). Join semantics mirror `region_mapping_shapes` (`crates/slicer-runtime/src/visual_debug_render.rs`): filter `RegionMapIR.entries` to the capture's layer, sort by the full join key `(object_id, region_id, variant_chain)`, resolve each key against the capture's own retained `slice_ir` by the full tuple; unjoined keys skip. Slab per joined region: `[capture.layer_z − region.effective_layer_height, capture.layer_z]` (plan D1 — the D8 row's "same (joined SliceIR rows)").
- **Tint interval classes**: class key = the `config_tint(region_map.config_for(key))` RGB triple (deterministic FNV-1a content hash — existing symbol); intervals union per (layer, tint); class paint order = ascending `(r, g, b)` lexicographic (a pure function of joined config content — the determinism story 247 flagged). Hash-colliding configs merge into one class (same pixels either way). 247's occlusion warning machinery applies unchanged; a new deterministic warning names the unjoined-entry count when nonzero.
- **OverhangAnnotation silhouette** (renderer): new `render_silhouette_overhang_composite(captures, view, resolution_scale, viewport, height_index)` in `crates/slicer-runtime/src/visual_debug_render.rs`, plus `SilhouetteSliceHeightIndex`/`SilhouetteLayerHeightClass` and builder `build_silhouette_slice_height_index(&[SliceIR])` (per layer: regions grouped by exact `effective_layer_height` bits, classes sorted ascending by height, each carrying its regions' `polygons` as the class footprint). Interval source: `overhang_quartile_polygons.get(&layer_index)` only (keyed lookup — no map iteration); bands sorted ascending by `quartile`; the `per_object` bridge/overhang `xy_footprint`s are **never** drawn (no Z attribution — plan fact 5).
- **Slab attribution**: single height class on the layer → every band polygon spans `[z − h, z]` (no boolean ops). Multiple classes → partition each band polygon by `slicer_core::polygon_ops::intersection` against each class footprint, each piece spanning its class's `[z − h, z]`. Exact: band polygons are subsets of the layer's region polygons by producer construction (`overhang_annotation_producer.rs` diffs footprints derived from committed `SliceIR` region polygons), so the per-class pieces partition each band with no residue.
- **Quartile classes**: four new `palette` constants (Q1–Q4), pairwise distinct from each other, from every color 247 paints on silhouettes, and from `BACKGROUND`; paint order ascending quartile (most severe last, wins projected overlaps); `quartile` outside `1..=4` fails closed with new `RenderError::InvalidQuartile`.
- **Validation lift** (`crates/pnp-cli/src/visual_debug.rs`): `SILHOUETTE_TAP_STAGE_IDS` += `"PrePass::RegionMapping"`, `"PrePass::OverhangAnnotation"`; their `SilhouetteUnsupportedForTap` reason arms removed; `PrePass::MeshAnalysis`/`PrePass::SeamPlanning`/arena taps stay rejected.
- **Assembly wiring** (`run_model_source` silhouette branch, authored by 247): OverhangAnnotation groups build the height index once from `Blackboard::slice_ir` (`crates/slicer-runtime/src/blackboard.rs`) and call the overhang entry point; RegionMapping groups flow through the existing composite call. Filenames from the shared scheme (`PrePass__RegionMapping_silhouette_{view}.png`, `PrePass__OverhangAnnotation_silhouette_{view}.png`); `view`/`layers_rendered`/warnings emission identical to 247's groups. Under `color_by: "tool"` both taps fail with the existing `RenderError::ToolColorUnavailable` (blackboard captures carry no tool assignment — packet 249's per-capture contract, extended to these groups, never silently role-rendered).
- **Test retirement/fallout**: drop the `PrePass::RegionMapping` and `PrePass::OverhangAnnotation` arms from 247's `silhouette_unsupported_taps_rejected_with_reasons` (test survives with its remaining arms — packet 250's AC-N2 re-runs it); add acceptance tests for both taps; extend `visual_debug_silhouette_tdd` and `visual_debug_silhouette_bundle_tdd`.
- **Docs**: two rows in docs/19's silhouette tap table (AC-9 anchors `quartile`, `tint class`).
- New `slicer_runtime` re-exports for the new pub types/functions.

## Out of Scope

- Any `CapturedIr` variant change (rejected — serialized `typed_capture` byte-compat; design.md records the analysis).
- `PrePass::MeshAnalysis` and `PrePass::SeamPlanning` silhouettes (plan §8 — permanent, not interim).
- Arena taps; gcode source; seam overlays; multicolor beyond the tool-rejection contract; raft/coarse support semantics (packets 248–251 and 247's W1/W2 own those).
- Drawing `per_object` bridge/overhang footprints or `prev_layer_boundaries` on silhouettes.
- Any change to top-down renderers (`region_mapping_shapes`, `surface_classification_shapes` stay byte-untouched), to the manifest schema, to `LEGEND_VERSION`, or to packets 248–251's exports.
- Per-quartile coloring of the *top-down* view (it paints uniform `SURFACE_OVERHANG` today; changing it is not this packet's business).

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ranged reads only: fact 5, D1/D2, D8 rows for the two taps, §6–§8, Packet Queue row #6.
- `docs/spec_packets/247-visual-debug-silhouette-core/design.md` + `packet.spec.md` — foundation exports and the AC-N5 test this packet retargets; direct read.
- `docs/19_visual_debug.md` — direct read (edited here).
- `docs/08_coordinate_system.md` — X/Y scaled integers via `Point2::to_mm`, Z mm floats end-to-end.
- `docs/21_data_defaults_and_fixtures.md` — struct-literal churn gate; delegate the watchlist question if a new literal trips `cargo xtask check-literals`.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`. Measurable refinements: AC-2's determinism covers warning-list equality, not just PNG bytes; AC-5 must use footprints whose projected intervals overlap so the test falsifies interval-level (rather than XY-polygon-level) attribution.
- Negative: `AC-N1` through `AC-N5`.
- Cross-packet impact: retargets 247's `silhouette_unsupported_taps_rejected_with_reasons` (AC-N1; the test survives — 250's AC-N2 re-runs it); extends the assembly branch 247 authors and composes with 249's styled-entry refactor and (tap, view, color mode) grouping without changing either packet's exports; 248/251 are untouched (verified: no pin in their AC text names these taps).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | renderer: joined slabs, tint order, quartile order, partition, fail-closed arms (AC-1..6, AC-N3..N5) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | whitelist lift + arm retirement (AC-8, AC-N1) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | end-to-end RegionMapping bundle + tool-mode rejection (AC-7, AC-N2) | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_render_tap_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | existing top-down suite unregressed (palette/RenderError additions) | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_blackboard_tap_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | capture shapes unregressed (no `CapturedIr` change) | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate on new fixtures | exit code |
| `cargo check --workspace --all-targets` | whole-tree compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Steps 1–2 (renderer) require packet 247 implemented (FORWARD-DEP) — they extend its composite machinery and test binary. Step 3 (validation + assembly) additionally interacts with 249's tool contract; Step 4 (bundle + docs) drives the real wedge pipeline and needs fresh guest WASMs: run `cargo xtask build-guests --check` before attributing any Step-4 failure (exit 0 fresh; 1 rebuild; 3 infra error, not clean).
- The wedge fixture's overhang-band presence is unverified (no existing end-to-end OverhangAnnotation render pins it), so end-to-end bundle ACs use `PrePass::RegionMapping` only (its prepass artifacts always commit and always join); all OverhangAnnotation behavior is pinned at the renderer level with direct fixtures, plus validation-level acceptance (AC-8) — mirroring 247's support-tap precedent. An overhang-capable end-to-end run is welcome extra coverage, never a green-gate dependency.
- New test literals of watched types (`SliceIR`, `SlicedRegion`, `RegionMapIR`, `SurfaceClassificationIR`, …) use `..` FRU or the exhaustive waiver, following `visual_debug_blackboard_tap_tdd.rs`'s seeded fixtures.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` and `crates/slicer-runtime/src/visual_debug_render.rs` are both >2000 lines: ranged reads only, targeted at the symbols each step names.
- The plan document is ~820 lines — read only the sections a step cites; delegate everything else.
- Packet 247's five files total ~1000 lines — read `design.md`'s Code Change Surface and Open Questions sections, delegate the rest.
