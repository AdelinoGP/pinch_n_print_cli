# Implementation Plan: 99_paint-pipeline-doc-sync

## Execution Rules

- One doc file per step (Steps 1-7).
- All `cargo` runs prefixed with `mkdir -p target &&`.
- Per-doc grep gate before moving to next step.

## Steps

### Step 0: Capture pre-packet g-code baselines (regression contract)

- Task IDs: `TASK-249`
- Expected dispatches:
  - "Run wedge slice + sha256sum; FACT".
  - "Run cube_4color slice + sha256sum; FACT".
- Context cost: `S`.
- Exit condition: 2 SHAs recorded.

### Step 1: Verify CONTEXT.md entries; rewrite `docs/01_system_architecture.md` PrePass section

- Task IDs: `TASK-249`
- Objective: AC-1, AC-2, AC-15.
- Files allowed to read:
  - `CONTEXT.md` — full (~150 lines).
  - `docs/01_system_architecture.md` — range-read PrePass section.
- Files allowed to edit (≤ 3):
  - `docs/01_system_architecture.md`.
  - `CONTEXT.md` ONLY if AC-15 verification finds entries missing.
- Expected dispatches:
  - "Run `rg -q 'Variant chain|Painted variant|Region-split semantic|Segment annotation' CONTEXT.md`; FACT".
  - "Open `docs/01_system_architecture.md` PrePass section; return SNIPPETS (≤ 40 lines)".
  - Post-edit: "Run `rg -B2 -A20 'PrePass::MeshSegmentation' docs/01_system_architecture.md`; SNIPPETS to confirm new content".
- Context cost: `S`.
- Exit condition: AC-1, AC-2, AC-15 satisfied.

### Step 2: Edit `docs/02_ir_schemas.md` — bump versions, add/remove types

- Task IDs: `TASK-249`
- Objective: AC-3, AC-4, AC-5, AC-6, AC-7.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — range-read IR sections.
  - `crates/slicer-ir/src/slice_ir.rs` — for current shape facts (range-read).
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`.
- Expected dispatches:
  - "Open `docs/02_ir_schemas.md` and return LOCATIONS for `SliceIR`, `RegionMapIR`, `RegionKey`, `SlicedRegion`, `RegionPlan`, `PaintRegionIR`, `MeshSegmentationIR`, `FacetPaintMark` sections; LOCATIONS".
- Context cost: `M`.
- Exit condition: AC-3, AC-4, AC-5, AC-6, AC-7 satisfied.

### Step 3: Edit `docs/03_wit_and_manifest.md` — add region_split schema, remove mesh-segmentation-output

- Task IDs: `TASK-249`
- Objective: AC-8, AC-9.
- Files allowed to read:
  - `docs/03_wit_and_manifest.md` — range-read manifest section.
- Files allowed to edit (≤ 3):
  - `docs/03_wit_and_manifest.md`.
- Expected dispatches:
  - "Open `docs/03_wit_and_manifest.md` manifest schema section; SNIPPETS (≤ 60 lines)".
- Context cost: `M`.
- Exit condition: AC-8, AC-9 satisfied.

### Step 4: Edit `docs/04_host_scheduler.md` — PrePass table, host-filtered dispatch, empty-polygon guard

- Task IDs: `TASK-249`
- Objective: AC-10, AC-11.
- Files allowed to read:
  - `docs/04_host_scheduler.md` — range-read PrePass + dispatch sections.
- Files allowed to edit (≤ 3):
  - `docs/04_host_scheduler.md`.
- Expected dispatches:
  - "Open `docs/04_host_scheduler.md` PrePass table + dispatch section; SNIPPETS (≤ 60 lines)".
- Context cost: `M`.
- Exit condition: AC-10, AC-11 satisfied.

### Step 5: Edit `docs/07_implementation_status.md` — append TASK-239..TASK-249 entries (DELEGATED)

- Task IDs: `TASK-249`
- Objective: AC-12.
- Files allowed to read: none (delegated).
- Files allowed to edit: none directly.
- Files out-of-bounds: any full-load of docs/07.
- Expected dispatches:
  - "Append TASK-239 through TASK-249 entries to `docs/07_implementation_status.md` as `implemented`, each with a 1-line description matching the packet goal. Also add an explicit 'Deferred follow-ups' section (or extend an existing one) noting: community paint ingestion, PaintValue::Vector, host:raw_slice. Return FACT pass/fail after applying the edit" — purpose: delegated edit.
- Context cost: `S` (dispatch-only).
- Exit condition: AC-12 satisfied.

### Step 6: Edit `docs/08_coordinate_system.md` — add constants conversion table

- Task IDs: `TASK-249`
- Objective: AC-13.
- Files allowed to read:
  - `docs/08_coordinate_system.md` — full (likely small).
  - `docs/specs/orca-paint-segmentation-parity.md` §5 — for the source conversion table.
- Files allowed to edit (≤ 3):
  - `docs/08_coordinate_system.md`.
- Expected dispatches:
  - "Locate §5 'Constants' (or equivalent) in `docs/specs/orca-paint-segmentation-parity.md`; return SNIPPETS (≤ 30 lines)".
- Context cost: `S`.
- Exit condition: AC-13 satisfied.

### Step 7: Flip `docs/specs/orca-paint-segmentation-parity.md` Status line

- Task IDs: `TASK-249`
- Objective: AC-14.
- Files allowed to read:
  - `docs/specs/orca-paint-segmentation-parity.md` — first 30 lines only.
- Files allowed to edit (≤ 3):
  - `docs/specs/orca-paint-segmentation-parity.md`.
- Expected dispatches: none.
- Context cost: `S`.
- Exit condition: AC-14 satisfied.

### Step 8: Per-AC grep verification + negative checks

- Task IDs: `TASK-249`
- Objective: validate every AC.
- Expected dispatches:
  - Run each AC's grep command via sub-agent; return FACT per AC.
  - Run `! rg -q 'boundary_paint' docs/`; FACT (AC-N1).
  - Run `! rg -q 'commit_paint_regions|point_in_paint_region' docs/`; FACT (AC-N2).
  - Run `! rg -q 'core-modules/mesh-segmentation|modules/core-modules/mesh-segmentation' docs/`; FACT (AC-N3).
- Context cost: `S`.
- Exit condition: every grep PASS.

### Step 9: AC-17 byte-identical g-code regression check

- Task IDs: `TASK-249`
- Objective: AC-17.
- Expected dispatches:
  - Wedge slice + sha256sum; compare to Step 0.
  - Cube_4color slice + sha256sum; compare to Step 0.
- Context cost: `S`.
- Exit condition: both SHAs match (this is a doc-only packet; production behavior must be invariant).

### Step 10: Workspace gate

- Task IDs: `TASK-249`
- Expected dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT".
  - "Run `cargo xtask build-guests --check`; FACT".
- Context cost: `S`.
- Exit condition: AC-16 satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost |
| --- | --- |
| Step 0 | S |
| Step 1 | S |
| Step 2 | M |
| Step 3 | M |
| Step 4 | M |
| Step 5 | S |
| Step 6 | S |
| Step 7 | S |
| Step 8 | S |
| Step 9 | S |
| Step 10 | S |

Aggregate: M.

## Packet Completion Gate

- All 11 steps complete.
- AC-1 through AC-17 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: pre/post wedge SHA (match), pre/post cube SHA (match), confirmation of `Status: implemented` on the handoff spec.
- `docs/07_implementation_status.md` carries TASK-239..TASK-249 implemented entries.
- `packet.spec.md` to `status: implemented`.
- **ROADMAP COMPLETE**: all 11 packets (89-99) implemented. The paint pipeline OrcaSlicer-parity work is finished.

## Acceptance Ceremony

- Re-dispatch every AC's grep command; PASS.
- Confirm byte-identical g-code (AC-17 regression contract for doc-only packet).
- Confirm `cargo xtask build-guests --check` clean.
- Peak context usage under 70%.

## Roadmap Closure Statement

This packet closes the paint-pipeline OrcaSlicer-parity roadmap. After this packet:

- All 12 cube_4color RED tests + 12 cube_fuzzy_painted RED tests are GREEN.
- Multi-minute wall-clock reduction in workspace tests (P89, P90).
- OrcaSlicer-parity multi-color slicing including Phase 5 width-limiting + interlocking.
- Mesh-segmentation host kernel wired (P94); WASM surface deleted (P97).
- RegionMapping cross-product expansion (P93).
- Open-string variant-chain extensibility for community paint semantics (P92), with built-in `material` + `fuzzy_skin` matching OrcaSlicer.
- All 6 main `docs/` files synced.

Deferred follow-ups (out of this roadmap):
- 3MF parser extension hook for community paint channels.
- `PaintValue::Vector(Vec<f32>)` IR addition for multi-channel paints (CMYK / RGB).
- Promoting paint-segmentation's internal slicing to `host:raw_slice` if profiling demands.
- Single-pass Voronoi over multi-color sites (option Q from grilling).
