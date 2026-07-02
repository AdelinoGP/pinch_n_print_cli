# Requirements: 132_modifier-region-split

## Packet Metadata

- Grouped task IDs:
  - `TASK-257`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Infill-modifier volumes — the standard way users raise infill density locally for stiffness —
do nothing spatial in PnP: the loader ingests them (`loader.rs:547-622`), but
`stamp_modifier_config_deltas` applies their config to the whole object
(`region_mapping.rs:266-268` — the code itself documents "no bbox/polygon overlap check";
only `ModifierScope::AllFeatures` is in use), and `prepass_slice.rs:286` slices only the
solid mesh. With packet 131's per-region config delivery in place, the missing half is
geometry: sub-regions must exist for the config to bind to. This packet creates them —
wall-less, wall-sharing, fill-only — per ADR-0030, which also makes them the first mainstream
population of the wall-sharing groups that packet 133's linker connects along.

## In Scope

- Slice each `ObjectMesh.modifier_volumes` mesh at the layer Z (reuse the existing
  `slice_mesh_ex` path; empty cross-section ⇒ no split on that layer).
- Intersect modifier cross-sections with the owning region's four partitioned fill polygons
  (`sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) at region
  partition; emit wall-less sub-regions with their own `region_id`.
- `ModifierScope` extension beyond `AllFeatures` so `stamp_modifier_config_deltas` binds the
  modifier's config delta to the sub-region's `RegionKey` (not the whole object).
- Populate `wall_source_region_id = Some(base)` for modifier sub-regions at the packet-130
  view-building site.
- Ensure `Layer::Perimeters` generates no walls for sub-regions (they are not dispatched as
  perimeter regions / carry no own wall geometry).
- Executor + contract tests per AC-1…AC-5, AC-N1, AC-N2 (programmatic object+modifier
  construction; no new 3MF fixture).
- Doc Impact: `docs/02_ir_schemas.md` modifier sub-region semantics.

## Out of Scope

- E2e 3MF modifier fixture + visual verification — M3, packet 136.
- Any linking behavior (133), module algorithm changes (134/135).
- Multiple overlapping modifiers on one region beyond last-wins/priority semantics already
  defined for config deltas — if overlap semantics surface as ambiguous, record a deviation
  and mirror the existing `stamp_modifier_config_deltas` priority order.
- Negative-part volumes (already handled elsewhere — TASK-192b) and support
  enforcer/blocker volumes.

## Authoritative Docs

- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — binding; full read (short).
- `docs/specs/modifier-region-infill.md` §Phase M1 — full read (short).
- `docs/02_ir_schemas.md` — delegate; `SlicedRegion`, `RegionMapIR`, partition sections only.
- `docs/04_host_scheduler.md` — delegate; region dispatch section only if Step 1 discovery
  needs it.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`. Refinements: AC-1's conservation check is
  area-based (union of split == pre-split within 1%) — it catches both overlap and gap bugs;
  AC-4 is the integration proof that 131+132 compose (density 0.40/0.15 mirrors the packet-131
  AC-1 values deliberately).
- Negative cases: `AC-N1` (no-modifier byte-identity incl. wedge SHA), `AC-N2` (degenerate
  slice → no split, no panic).
- Cross-packet impact: produces the first mainstream wall-sharing-group fixtures for packet
  133; packet 136's M3 fixture depends on this packet's semantics being exactly ADR-0030's.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test executor -- modifier_split 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1/2/3/5/N2 | FACT + counts |
| `cargo test -p slicer-runtime --test contract -- modifier_split_subregion_density 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-4 (131+132 composition) | FACT |
| `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 byte-identity | FACT |
| `cargo check --workspace --all-targets` | compile gate | FACT |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT |
| `rg -q 'modifier sub-region' docs/02_ir_schemas.md && echo HIT` | Doc Impact grep | FACT |

## Step Completion Expectations

- Step ordering rationale: Step 1 (discovery) exists because ADR-0030 pins the semantics but
  not the exact struct plumbing (which types carry a sub-region between partition and
  dispatch). Its output contract is a bounded decision memo, not code — no later step may
  begin until the memo names the touched structs and the sub-region `RegionKey` derivation.
- Cross-step invariant: `wall_source_region_id` population for PAINT virtual-variants
  (packet 130 behavior) must not regress while adding the modifier arm — the 130 contract
  test `infill_postprocess_wall_source` stays green through every step.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-core/src/algos/region_mapping.rs` (read only the `ModifierScope` +
  `stamp_modifier_config_deltas` regions, ~lines 260-320 and 600-640),
  `crates/slicer-core/src/algos/prepass_slice.rs` (slicing entry ~line 286 region),
  `crates/slicer-runtime/src/region_partition.rs` (full file allowed — it is the primary
  surface and small enough), `docs/02_ir_schemas.md` (sections only).
- Likely temptation reads: the paint-segmentation pipeline (how paint splits regions) — do
  NOT open it; modifier sub-regions deliberately do NOT follow the paint path (they are
  partition-time, wall-less). If a comparison fact is needed, delegate a SUMMARY.
- Sub-agent return-format hints: Step 1 discovery dispatches return LOCATIONS (struct/fn +
  one-line role, ≤15 entries); cargo gates return FACT.
