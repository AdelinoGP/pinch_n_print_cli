# Requirements: 99_paint-pipeline-doc-sync

## Packet Metadata

- Grouped task IDs:
  - `TASK-249` — Doc sync after the paint-pipeline OrcaSlicer-parity roadmap (packets 89-98).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5c — Doc updates"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packets 89-98 reshape the paint pipeline end-to-end: new IR shapes (P91), manifest schema + dispatch (P92), region-mapping cross-product (P93), mesh-segmentation host wiring (P94), paint-segmentation port (P95), Phase 5 width-limiting (P96), WASM mesh-segmentation deletion (P97), loader symmetry (P98). The implementation is now the source of truth, but `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/07_implementation_status.md`, and `docs/08_coordinate_system.md` are stale. They reference deleted types (`PaintRegionIR`, `MeshSegmentationIR`), wrong prepass orders, missing manifest schemas, and obsolete WIT resources.

The roadmap intentionally deferred doc sync to a single closing packet to avoid mid-roadmap doc churn (every intermediate state would have required a doc update; consolidating saves effort and avoids transient incorrect documentation). This packet finishes that sync.

The work is entirely doc edits — no production code changes, no fixture changes, no test changes. Verification is grep-based: each AC names a phrase to find or NOT find.

## In Scope

- `docs/01_system_architecture.md`:
  - Rewrite prepass-order section to the post-roadmap 9-stage sequence.
  - Add variant-chain region-splitting model description (concise; reference docs/02 + docs/03 for details).
  - Remove the obsolete "PrePass::MeshSegmentation [new — runs first]" wired-flag block.
- `docs/02_ir_schemas.md`:
  - Bump SliceIR + RegionMapIR to 2.0.0.
  - Document `variant_chain` on `RegionKey` + `SlicedRegion`.
  - Document `segment_annotations` (replaces `boundary_paint`).
  - Document `ConfigId(u32)` + `configs: Vec<ResolvedConfig>` interner.
  - REMOVE `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintRegionRTreeIndex`, `MeshSegmentationIR`, `FacetPaintMark`.
  - Note `PaintValue::Vector(Vec<f32>)` as deferred follow-up.
- `docs/03_wit_and_manifest.md`:
  - Add `[[region_split]]` manifest schema section.
  - Add priority registry section (`CORE_REGION_SPLIT_PRIORITIES`, community floor 1000, value_type enum).
  - Add cross-manifest aggregation behavior section (WARN on tied priorities, lex tiebreaker).
  - REMOVE `mesh-segmentation-output` WIT resource documentation.
- `docs/04_host_scheduler.md`:
  - Update PrePass stage-prerequisites table (`PrePass::MeshSegmentation` → no prereqs + replace_mesh; `PrePass::PaintSegmentation` → SliceIR + RegionMapIR + replace_slice_ir).
  - Document host-filtered dispatch contract (the layer-executor hook from P92).
  - Document the universal empty-polygon dispatch guard.
  - REMOVE the "guard-based fallback contract" sentence for paint-segmentation (the guest path is deleted in P97).
- `docs/07_implementation_status.md`:
  - Add TASK-239 through TASK-249 entries as `implemented`.
  - Record 3 deferred follow-ups (community paint ingestion, PaintValue::Vector, host:raw_slice).
- `docs/08_coordinate_system.md`:
  - Add the constant-conversion table from `docs/specs/orca-paint-segmentation-parity.md` §5 (every OrcaSlicer constant divides by 100).
- `docs/specs/orca-paint-segmentation-parity.md`:
  - Flip `Status:` from `awaiting Slice Rework` to `implemented`. KEEP the file as historical record (don't delete; the 1021-line spec stays as the algorithmic blueprint reference).
- `CONTEXT.md`:
  - Verify `Variant chain`, `Painted variant`, `Region-split semantic`, `Segment annotation` entries are present (added during planning).

## Out of Scope

- Any production code changes.
- Any fixture or test changes.
- Schema/manifest/WIT additions beyond what packets 89-98 already shipped.
- New deferred items not in the roadmap's deferred-list.
- Doc edits to files not in the In Scope list (e.g., `docs/05_module_sdk.md` is technically affected by P92's dispatch contract, but its existing content is generic enough to stay accurate; explicit update deferred unless an inconsistency surfaces).

## Authoritative Docs

This packet EDITS docs. The source-of-truth for the content is the implementation that landed in packets 89-98 + the planning docs:

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — comprehensive plan (referenced for content shape).
- `docs/specs/orca-paint-segmentation-parity.md` — normative algorithm spec; flipped at packet close.
- The actual source code under `crates/` — authoritative for IR shapes and producer/dispatch behavior.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- None. The doc sync reflects pinch_n_print's implementation; OrcaSlicer parity content is in the doc edits via the source material (`docs/specs/orca-paint-segmentation-parity.md`), already cited.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-17`. Refinements:
  - AC-12's task-ID check assumes the implementer adds entries TASK-239 through TASK-249. Numbering follows the existing convention; if any prior task is already in docs/07 (e.g., TASK-237 / TASK-238 from packets 87/88), the new entries continue the count.
  - AC-15's CONTEXT.md verification is a check — the entries were added during planning (per the conversation summary). If absent, this packet re-adds them.
  - AC-17 byte-identical g-code is the regression contract: this is a doc-only packet, so production behavior is invariant.
- Negative cases: `AC-N1` (no `boundary_paint`), `AC-N2` (no deleted accessor refs), `AC-N3` (no WASM mesh-segmentation refs).
- Cross-packet impact: closes the roadmap. All paint-pipeline-related docs reflect the post-P98 state.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace still compiles (sanity; no source changes expected) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest clean | FACT pass/fail |
| Per-AC `rg -q` and `rg -nE` checks | Doc-content gates (each AC has its own grep) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p99-wedge.gcode && sha256sum /tmp/p99-wedge.gcode` | AC-17 wedge byte-identical | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p99-cube.gcode && sha256sum /tmp/p99-cube.gcode` | AC-17 cube byte-identical | FACT (sha256) |
| `! rg -q 'boundary_paint' docs/` | AC-N1 | FACT pass/fail |
| `! rg -q 'commit_paint_regions\|point_in_paint_region' docs/` | AC-N2 | FACT pass/fail |
| `! rg -q 'core-modules/mesh-segmentation' docs/` | AC-N3 | FACT pass/fail |

## Step Completion Expectations

- Each doc file is edited in a single step (or split as multi-commit if the doc is large and the edits are unrelated). Doc edits are independent across files; ordering within Steps 1-7 is flexible.
- AC-17 byte-identical is the regression-guard: this packet edits docs only; production must not change. If g-code SHA differs, escalate IMMEDIATELY — something unintended is in the diff.
- AC-12's docs/07 update is delegated per CLAUDE.md (never load full backlog into implementer context).

## Context Discipline Notes

- `docs/02_ir_schemas.md` may be > 600 lines. Range-read by IR type section.
- `docs/04_host_scheduler.md` may be > 400 lines. Range-read by section.
- `docs/07_implementation_status.md` is large and grows monotonically. Delegate any edit to a sub-agent — never load the full backlog into context.
- `docs/specs/orca-paint-segmentation-parity.md` is 1021 lines. The flip is a one-line frontmatter edit; only read the first ~10 lines.
- `CONTEXT.md` is small (~142 lines per current state); read in full.
