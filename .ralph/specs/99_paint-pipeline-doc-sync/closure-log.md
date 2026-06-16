# Closure Log: 99_paint-pipeline-doc-sync

**Packet:** 99
**Slug:** `99_paint-pipeline-doc-sync`
**Status at closure:** `draft` → `implemented` (status flip in second commit)
**Closure date:** 2026-06-15
**Branch:** `master`
**Commits:** 2 (this packet's audit-driven fixes + status flip)

## Roadmap Closure

Packet 99 closes the 11-packet paint-pipeline OrcaSlicer-parity roadmap:

| Packet | Title | Closure |
|---|---|---|
| 89 | Benchy 3MF retirement | 2026-06-08 |
| 90 | Regression wedge STL swap | 2026-06-08 |
| 91 | Paint-pipeline schema scaffolding | 2026-06-08 |
| 92 | Region-split manifest + dispatch | 2026-06-08 |
| 93 | Region-mapping cross-product | 2026-06-08 |
| 94 | Host mesh-segmentation wiring | 2026-06-10 |
| 95 | Paint-segmentation Orca-port | 2026-06-12 |
| 96 | Paint-segmentation Phase 5 width-limit | 2026-06-13 |
| 97 | WASM mesh-segmentation deletion | 2026-06-13 |
| 98 | Loader paint-channel symmetry | 2026-06-15 |
| **99** | **Doc sync (this packet)** | **2026-06-15** |

**ROADMAP COMPLETE.** After this packet, all 6 main `docs/` files reflect the post-roadmap state; the 1021-line parity spec is preserved as the algorithmic blueprint; the 3 deferred follow-ups are recorded in `docs/07_implementation_status.md` under "## Known parity gaps (post-roadmap work)".

## Per-AC Outcome (post-fix)

| AC | Status | Notes |
|----|--------|-------|
| AC-1 | PASS | `PrePass::MeshAnalysis` adjacent to retired `PrePass::MeshSegmentation` mention; no obsolete markers |
| AC-2 | PASS | `variant_chain` documented in docs/01 "Variant-Chain Region Splitting" sub-section |
| AC-3 | PASS | SliceIR 2.0.0 + RegionMapIR 2.0.0 |
| AC-4 | PASS | `variant_chain` + `segment_annotations` documented |
| AC-5 | PASS | `ConfigId` + `configs: Vec<ResolvedConfig>` interner documented |
| AC-6 | PASS | Deleted types fully removed from docs/02 |
| AC-7 | PASS | `PaintValue::Vector(Vec<f32>)` deferred variant added (`#[doc(hidden)]`) |
| AC-8 | PASS | All 4 sub-checks: `[[region_split]]` + priority registry + `value_type` + cross-manifest aggregation |
| AC-9 | PASS | `mesh-segmentation-output` WIT resource removed |
| AC-10.1 | **INTENT-MET (grep-FAIL by design)** | The `PrePass::MeshSegmentation` row was deleted from the PrePass stage-prerequisites table; the stage is retired per P94r + P97. The grep gate is no longer applicable. See META-1 in packet.spec.md |
| AC-10.2 | PASS | `PrePass::PaintSegmentation` row updated to `SliceIR`, `RegionMap`; produces split `SliceIR` via `replace_slice_ir` |
| AC-10.3 | PASS | `guard-based fallback contract` sentence removed |
| AC-11 | PASS | Host-Filtered Dispatch + Universal Empty-Polygon Dispatch Guard sub-sections added |
| AC-12 | PASS | All 11 TASK IDs (TASK-239..TASK-249) present in docs/07 |
| AC-13 | PASS | 100 nm + 14-row OrcaSlicer Constant Conversion Table |
| AC-14 | PASS | `Status: implemented` line in `docs/specs/orca-paint-segmentation-parity.md` (was: `awaiting Slice Rework (blocked)`) |
| AC-15 | PASS | All 4 paint-vocab entries present in `CONTEXT.md` (verified, no edit needed) |
| AC-16 | PASS | `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo xtask build-guests --check` clean (31 guests) |
| AC-17 | PASS | wedge SHA `AA4DA2FA…EF1E3B` + cube_4color SHA `AD0245C3…AB54EBF` byte-identical vs pre-packet baseline |
| AC-N1 | LIVE-DOC-PARTIAL | 4 residual live-doc matches: docs/02:785, 811 (rename-history notes), docs/05:486 (WIT SDK accessor — documented in DEV-1), docs/07:198 (historical log — documented in DEV-1) |
| AC-N2 | LIVE-DOC-PASS | Zero matches in live doc set |
| AC-N3 | LIVE-DOC-PASS | Zero matches in live doc set |

## Audit-Driven Fixes (post-implementation)

The session-scoped spec-audit flagged three live-doc defects that were fixed before status flip:

1. **`docs/02_ir_schemas.md:954`** — doc comment `SlicedRegion.boundary_paint` → `SlicedRegion.segment_annotations`. The `boundary_paint`→`segment_annotations` rename was a packet-91 contract change; this was a missed live-doc residual.
2. **`docs/04_host_scheduler.md:1003`** — `PrePass::MeshSegmentation` table row DELETED entirely. The stage is retired per P94r + P97; a retired stage does not belong in a live-stage prerequisites table. The row's previous content (1) contradicted docs/01 in the same packet's edit set, (2) misassigned `SurfaceClassificationIR` (which is `MeshAnalysis`'s output), and (3) misassigned the output relative to AC-10.1's own prose.
3. **`docs/01_system_architecture.md:75`** — "The nine prepass stages execute in this order:" → "The six prepass stages execute in this order:"; new follow-up paragraph added explaining that `host:slice` and `host:shell_classification` are Layer-stage host calls (not `PrePass::*` enum variants) that run between `MeshAnalysis` (stage 2) and `PaintSegmentation` (stage 5). See DEV-4 in packet.spec.md for the packet-text defect resolution.

## Meta-Deviation (packet-text defect, see META-1)

AC-10.1's "Then" clause specified an output (`produces MeshIR via replace_mesh`) for a stage that is retired per P94r + P97. The grep gate (`rg -q 'PrePass::MeshSegmentation.*replace_mesh'`) was too narrow to catch the cross-doc inconsistency. The initial implementation satisfied the gate by writing a sentence that contradicted the rest of the doc set; the audit-driven fix deleted the row entirely, leaving the gate INTENTIONALLY-FAIL but INTENT-MET.

**Lesson for future retro-sync packets:**

- Phrase retired-stage rows with no output claim.
- AC grep gates for retro-sync packets should include cross-doc consistency checks (e.g. `rg 'PrePass::MeshSegmentation' docs/` should return only matches consistent with 'retired' framing; 'produces ...' sentences on retired-stage rows should fail the gate).
- AC-10.1's "Then" clause for the MeshSegmentation row is itself a packet-text defect (it claims the stage produces MeshIR, but the stage is retired). The grep gate inherited the defect.

See **META-1** in `packet.spec.md` ## Deviations section for the full entry.

## Deferred Follow-Ups (out of this roadmap)

Recorded under `docs/07_implementation_status.md` "## Known parity gaps (post-roadmap work)":

1. **Community paint ingestion (3MF parser extension hook)** — parser currently handles only the four OrcaSlicer paint channels (`paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin`); community-contributed channels are silently ignored until the extension hook lands.
2. **`PaintValue::Vector(Vec<f32>)` IR addition** — for multi-channel paints (CMYK / RGB); deferred to a future IR-breaking change. The variant is already reserved in `docs/02_ir_schemas.md` as `#[doc(hidden)]` to avoid module authors producing it accidentally.
3. **Promoting paint-segmentation's internal slicing to `host:raw_slice`** — performance/reusability refactor; deferred until profiling data justifies it. The internal slice-plane intersection loop is currently private to the paint-segmentation kernel.
4. **Single-pass Voronoi over multi-color sites (option Q from grilling)** — algorithmic improvement, deferred.

## File Inventory (this session)

Modified:

- `docs/01_system_architecture.md` — PrePass section rewritten to 6 stages; "Variant-Chain Region Splitting" sub-section added; 7+ live-stale renames (boundary_paint→segment_annotations, PaintRegionIR→PaintRegionLayerView, etc.)
- `docs/02_ir_schemas.md` — SliceIR 4.1.0→2.0.0 + RegionMapIR 2.0.0; IR 4 (PaintRegionIR/LayerPaintMap/SemanticRegion/PaintRegionRTreeIndex) section purged; renumbered IR 5→IR 4; `PaintValue::Vector(Vec<f32>)` variant added; line 954 stale-ref fix
- `docs/03_wit_and_manifest.md` — `reads = ["SliceIR.regions.boundary_paint"]` → `["SliceIR.regions.segment_annotations"]`
- `docs/04_host_scheduler.md` — `PrePass::MeshSegmentation` row deleted (retired stage); `PrePass::PaintSegmentation` row updated to `SliceIR, RegionMap; produces split SliceIR via replace_slice_ir`; Host-Filtered Dispatch sub-section added; Universal Empty-Polygon Dispatch Guard sub-section added; `guard-based fallback contract` sentence removed; `point_in_paint_region conflict check as defence-in-depth` sentence removed
- `docs/05_module_sdk.md` — `boundary_paint()` accessor description rephrased; `host::point_in_paint_region` / `host::segment_in_paint_region` code example replaced with `paint_view.get_regions()` + WIT accessor example
- `docs/07_implementation_status.md` — TASK-245 + TASK-249 entries; 3 deferred follow-ups recorded under "## Known parity gaps (post-roadmap work)"
- `docs/08_coordinate_system.md` — 14-row Constant Conversion Table added (sourced from orca-paint-segmentation-parity.md §5)
- `docs/10_scenario_traces.md` — `SlicedRegion.boundary_paint` → `SlicedRegion.segment_annotations`
- `docs/specs/orca-paint-segmentation-parity.md` — Status line flipped: `awaiting Slice Rework (blocked)` → `implemented`
- `.ralph/specs/99_paint-pipeline-doc-sync/packet.spec.md` — DEV-1 (AC-N1/N2/N3 strict-grep deviation) + DEV-4 (count interpretation) + META-1 (AC-10.1 narrow-grep meta-deviation) entries; status flipped to `implemented`

Created:

- `.ralph/specs/99_paint-pipeline-doc-sync/closure-log.md` — this file

**Production code: untouched** (packet is doc-only; AC-17 byte-identical g-code confirms).

## Verification Artifacts

- **wedge g-code SHA256:** `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B` (pre = post = **MATCH**)
- **cube_4color g-code SHA256:** `AD0245C3463174606718D13675B1F9B4F1C09B6AF5FDF13F3C2EC791DAB54EBF` (pre = post = **MATCH**)
- **`cargo clippy --workspace --all-targets -- -D warnings`:** exit 0 (only pre-existing nom/quick-xml future-incompat warnings remain)
- **`cargo xtask build-guests --check`:** exit 0, all 31 guests fresh

## Status Decision

Per user's explicit finalization request, packet status is flipped from `draft` to `implemented` in a separate commit (commit 2 of 2). The acceptance ceremony is green under the documented deviations (DEV-1, DEV-4, META-1).
