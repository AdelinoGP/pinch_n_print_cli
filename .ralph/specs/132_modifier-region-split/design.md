# Design: 132_modifier-region-split

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/region_partition.rs` — the wall-inset
  partition at `Layer::Perimeters` commit is where the four fill polygons exist and where
  ADR-0030 pins the split. The modifier cross-section (sliced per layer from
  `ObjectMesh.modifier_volumes`) intersects the partitioned polygons here; sub-region
  identities are minted here.
- Config binding path: `crates/slicer-core/src/algos/region_mapping.rs` —
  `stamp_modifier_config_deltas` (~lines 269-314) currently stamps object-wide under
  `ModifierScope::AllFeatures`; gains a geometric scope variant binding the delta to the
  sub-region `RegionKey`.
- Modifier slicing: reuse the `slice_mesh_ex` path (`crates/slicer-core/src/algos/
  prepass_slice.rs:516` idiom — `slice_mesh_ex(&object_world_mesh(object), &zs)`) applied to
  the modifier mesh — where this call lives (prepass vs partition-time lazy slice) is Step 1's
  discovery output.
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/executor/` (new
  `modifier_region_split_tdd.rs`); `crates/slicer-runtime/tests/contract/` (AC-4 composition
  test reusing the 131 echo guest); wedge e2e SHA guard.
- OrcaSlicer comparison surface: none directly — the reference behavior (one wall set, split
  fill) is recorded in ADR-0030 §Context; no OrcaSlicer code is ported.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Host-only: no WIT file changes and no guest-source changes are expected. If Step 1
  discovery finds a guest-feeding file on the change surface after all (e.g. `slicer-ir`
  struct addition), the wasm-staleness rule from `CLAUDE.md` applies in full: run
  `cargo xtask build-guests --check` and rebuild before attributing any failure.
- Walls stay merged: nothing in this packet may cause `Layer::Perimeters` to receive a
  modifier sub-region as a wall-generating region (ADR-0030 Future-Reviewer note).
- Partition precedence `bridge > bottom > top > sparse` is composed with, never re-derived:
  the split applies to the already-partitioned polygons.

## Code Change Surface

- Selected approach: partition-time splitting. Modifier cross-sections are intersected with
  the base region's four partitioned polygons; each non-degenerate intersection mints a
  sub-region (id derivation `[FWD]`, pattern the paint `paint_variant_region_id` synthesis)
  whose fill polygons replace the overlapped area of the base's (base keeps the difference).
  `wall_source_region_id = Some(base)` flows to the packet-130 view-building site. Config
  binding via a geometric `ModifierScope` variant that `stamp_modifier_config_deltas` uses to
  target the sub-region `RegionKey` instead of the object config.
- Exact changes: `region_partition.rs` (split + sub-region minting + wall-source),
  `region_mapping.rs` (`ModifierScope` variant + targeted stamping), the modifier-mesh
  per-layer slicing call site (Step 1 decides prepass-cached vs lazy), the 130 dispatch site
  (extend the wall-source predicate's modifier arm), new tests.
- Rejected alternatives: (a) full SlicedRegion splits following the paint pipeline —
  rejected: paint regions get their own walls, exactly what ADR-0030 forbids here, and the
  paint pipeline's Voronoi machinery is massively oversized for polygon intersection;
  (b) config-only spatial evaluation inside infill modules (module tests point-in-modifier
  per scan line) — rejected: pushes geometry into every module, violates the shallow-module
  goal (ADR-0025); (c) doing this pre-partition on the raw slice polygon — rejected: the
  four role polygons don't exist yet, so precedence would need re-derivation.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/region_partition.rs` — role: the split; expected change: split
  fn + sub-region minting + wall-source arm.
- `crates/slicer-core/src/algos/region_mapping.rs` — role: config binding; expected change:
  `ModifierScope` variant + targeted stamp (~60 lines).
- Modifier slicing call site (Step 1 names it; expected `prepass_slice.rs` or
  `region_partition.rs`-local) — role: per-layer modifier cross-sections.
- `crates/slicer-runtime/tests/executor/modifier_region_split_tdd.rs` (new) +
  `crates/slicer-runtime/tests/contract/` AC-4 test — role: TDD.
- `docs/02_ir_schemas.md` — role: Doc Impact; expected change: modifier sub-region
  subsection.

## Read-Only Context

- `crates/slicer-core/src/algos/prepass_slice.rs` — lines 505-520 only — `slice_mesh_ex`
  idiom.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — the `paint_variant_region_id`
  synthesis lines only (rg first) — region-id minting pattern.
- `crates/slicer-ir/src/slice_ir.rs` — `SlicedRegion` + `RegionMapIR` regions only.
- `crates/slicer-model-io/src/loader.rs` — lines 547-628 only — `ModifierVolume` shape (the
  full construction incl. `priority` + `applies_to: ModifierScope::AllFeatures` is at 618-626;
  the `ModifierVolume` struct itself is defined in `slicer-ir`, re-exported via `loader.rs:18`).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — never load (reference behavior already recorded in ADR-0030).
- The paint-segmentation pipeline beyond the id-minting lines — delegate any comparison.
- `modules/core-modules/**` — untouched by this packet.
- `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- Step 1 discovery: "Trace where a region's identity + partitioned polygons travel from
  `region_partition.rs` to `Layer::Infill`/`Layer::InfillPostProcess` dispatch (which
  structs/maps carry them); return LOCATIONS ≤15 (struct/fn + one-line role)".
- Step 1 discovery: "How is `RegionKey` constructed for a paint-variant region (fields +
  where variant ids come from)? FACT ≤5 lines" — sub-region `RegionKey` derivation.
- "Run `cargo test -p slicer-runtime --test executor -- modifier_split 2>&1 | tee
  target/test-output.log | grep '^test result'`; FACT + counts; SNIPPETS ≤20 on failure".
- "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N1 guard.
- "Run `cargo check --workspace --all-targets`; FACT or LOCATIONS ≤30".

## Data and Contract Notes

- IR contracts: whether sub-regions surface as new `SlicedRegion`-level entries or
  partition-map entries is Step 1's decision — either way NO schema-version bump is expected
  unless a struct gains a field (then: minor bump + record in closure log).
- WIT boundary: none touched (fields shipped in 130; accessor in 131).
- Determinism: sub-region ids must be deterministic across runs (derive from base region_id +
  modifier index/hash, never from iteration order of a hash map).

## Locked Assumptions and Invariants

- Modifier sub-regions NEVER receive their own walls (ADR-0030; AC-3 pins it).
- `wall_source_region_id` semantics from packet 130 are reused verbatim — `Some(base)` means
  "shares base walls"; the modifier arm may not overload it.
- Split conservation: base-remainder ∪ sub-region polygons == pre-split polygons (AC-1's 1%
  area tolerance covers Clipper rounding only).
- No-modifier objects are byte-identical end-to-end (AC-N1).
- Empty/degenerate modifier cross-section ⇒ no split, base config (AC-N2) — never a panic.

## Risks and Tradeoffs

- Open/self-intersecting modifier meshes: reuse the solid-mesh slicing repair path; on
  irrecoverable geometry fall back to no-split + a structured warning (never abort the
  slice).
- Region-count growth multiplies per-region work downstream — bounded by modifier count per
  object; no mitigation needed at expected counts (1-4 modifiers).
- The Step-1 discovery could reveal that sub-regions must surface earlier than partition time
  (e.g. region mapping needs them at prepass). If so, the packet's approach section is
  amended as a recorded deviation BEFORE implementation proceeds — not silently.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (the split implementation)
- Highest-risk dispatch: the Step-1 trace — must return a bounded LOCATIONS memo, not a
  region-pipeline tour.

## Open Questions

- `[FWD]` Sub-region `RegionKey`/`region_id` derivation — pattern the paint variant-id
  synthesis; Step 1 resolves before coding.
- `[FWD]` Modifier-mesh slicing site: prepass-cached (slice all modifier meshes once,
  alongside solids) vs partition-time lazy — decided by where layer-Z context is cheapest in
  Step 1's trace; either satisfies the ACs.
- `[FWD]` Overlapping-modifier semantics: mirror the existing stamping priority order; if the
  existing order is undefined for spatial overlap, split by document order and record a
  deviation.
