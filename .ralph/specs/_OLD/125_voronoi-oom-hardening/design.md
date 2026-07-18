# Design: 125_voronoi-oom-hardening (region_id↔tool split + D14 fuzzy + per-tool config + voronoi hardening)

> Implementation-complete record. Sections describe the **shipped** shape so a reviewer can map each AC
> to the exact surface that satisfies it.

## Controlling Code Paths

- **Split core / conflation site:** `crates/slicer-runtime/src/layer_executor.rs` —
  `assemble_ordered_entities`. The loop already separated `base_key` (identity → `config_for`) from
  `entity_key` (the transport slot that held the tool); the split makes that explicit by setting
  `PrintEntity.tool_index = resolved_tool` and `region_key.region_id = region.region_id`.
- **Tool transport to path-opt guest:** `PrintEntity.tool_index` → host `dispatch::OrderedEntityView`
  → WIT `ordered-entity-view.tool-index` (`crates/slicer-wasm-host/src/host.rs`,
  `crates/slicer-wasm-host/src/dispatch.rs`) → SDK `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`)
  → guest `tool_index_of` (`modules/core-modules/path-optimization-default/src/lib.rs`).
- **Tool transport to finalization:** WIT `push-entity-to-layer`/`-with-priority`/`insert-entity-at`
  `tool-index` params + `print-entity-view.tool-index`
  (`crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`), host reconstruction
  (`host.rs` finalization `staged` build), SDK `FinalizationOutputBuilder` + macro drain
  (`crates/slicer-macros/src/lib.rs`).
- **Emit (host-side tool reader):** `crates/slicer-gcode/src/emit.rs` reads `entity.tool_index`;
  retains the `MAX_PLAUSIBLE_TOOLS` guard + `GCodeEmitError::ToolIndexOutOfRange`.
- **D14 fuzzy:** `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (drop segment_annotations
  synthesis) → `slice-region-view.variant-chain` WIT accessor (`ir-types.wit`, host `SliceRegionData`
  + `marshal/in_.rs`, SDK `SliceRegionView`) → `crates/slicer-core/src/perimeter_utils.rs`
  `build_wall_flags(variant_fuzzy)` ← both perimeter guests.
- **Per-tool config:** `crates/slicer-scheduler/src/config_resolution.rs` `resolve_per_tool_configs`;
  emit consumer (`emit.rs` `retract_length_for_tool` + `with_tool_configs`, wired in
  `crates/slicer-runtime/src/run.rs`); painted-geometry overlay in
  `crates/slicer-core/src/algos/region_mapping.rs` `execute_region_mapping_inner`, threaded via
  `crates/slicer-runtime/src/prepass.rs` → `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.
- **Voronoi hardening:** `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`
  `from_colored_lines` (input cap + `catch_unwind`), `MmuGraphError` variants.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- **No struct `Default` on `PrintEntity`** — the missing derive is the deliberate compiler checklist for
  the ~43 construction sites; `#[serde(default)]` on the field provides deserialization back-compat
  without a struct `Default`.
- **Schema bump is additive:** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` 1.0.0 → 1.1.0; older
  serialized `PrintEntity`s deserialize (field defaults to `0`).
- **Behavior-neutral when unused:** the per-tool config axis (both emit + region-mapping) is a no-op
  when no `tool_config:` keys are present, so the default-config golden output is unchanged.

## Selected Approach (one; alternatives rejected)

For each of two forks the **principled/full** option was taken (per user):

1. **Fuzzy transport** — chosen: a real `slice-region-view.variant-chain` WIT accessor the perimeter
   guest reads. *Rejected:* host-side synthesis of FuzzySkin back into the WIT `segment_annotations`
   projection (smaller, but re-conflates the channel D14 reserves for modifier volumes).
2. **Finalization tool channel** — chosen: explicit `tool-index` params on the push/insert WIT methods
   (guests pass the tool). *Rejected:* host derives the tool from the pushed `region_id` (smaller, but
   keeps a bounded local `region_id`-as-tool convention, against the packet's purpose).
3. **Per-tool precedence** — chosen: per-tool overlay is **highest** (`global < per_object <
   per_paint_semantic < per_tool`), mirroring OrcaSlicer filament-override-last (`PrintApply.cpp`).
   *Rejected:* per-tool as a low base (the earlier draft) — contradicts OrcaSlicer's apply order.

## Code Change Surface (shipped)

Primary (the split + axes):
- `crates/slicer-ir/src/slice_ir.rs` — `PrintEntity.tool_index`; schema bump.
- `crates/slicer-runtime/src/layer_executor.rs` — assembly sets `tool_index` / restores identity.
- `crates/slicer-gcode/src/emit.rs` — emit reads `tool_index`; per-tool `retract_length` consumer.
- `crates/slicer-core/src/algos/region_mapping.rs` — painted per-tool overlay (highest precedence).

Supporting (transport + plumbing): `crates/slicer-schema/wit/deps/ir-types.wit` &
`…/world-finalization/world-finalization.wit`; `crates/slicer-wasm-host/src/{host.rs,dispatch.rs,marshal/in_.rs}`;
`crates/slicer-sdk/src/{views.rs,traits.rs}`; `crates/slicer-macros/src/lib.rs`;
`crates/slicer-core/src/{perimeter_utils.rs, algos/paint_segmentation/mod.rs, …/voronoi_graph.rs}`;
`crates/slicer-scheduler/src/config_resolution.rs` (+ lib re-exports);
`crates/slicer-runtime/src/{run.rs,prepass.rs,builtins/region_mapping_producer.rs,lib.rs}`;
guests `path-optimization-default`, `arachne-perimeters`, `classic-perimeters`, `skirt-brim`,
`wipe-tower`; ~43 `PrintEntity` test/fixture construction sites.

Docs: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md`.

## Read-Only Context

- `CLAUDE.md` §"Guest WASM Staleness", §"Test Discipline", §"Coordinate System Hazard" — small.
- `docs/02_ir_schemas.md` §IR 10 (PrintEntity), §Config Key Namespaces — large; read named ranges only.
- `docs/03_wit_and_manifest.md` — large; LOCATIONS-delegate the four touched records.

## Out-of-Bounds Files (do not load)

- `OrcaSlicerDocumented/**` — delegate per the OrcaSlicer obligations.
- `target/**`, lockfiles, generated bindgen output, guest `.wasm` artifacts.
- Unrelated crates not in the change surface above.

## Expected Sub-Agent Dispatches

- `cargo test`/`clippy`/`build-guests` runs → FACT pass/fail (+ failing assertion + ≤20 lines on fail).
- `docs/02`/`docs/03` fact-checks → LOCATIONS (the named record/section only).
- OrcaSlicer precedence/width → SUMMARY (≤200 words) or LOCATIONS — never load the C++ directly.

## Data and Contract Notes

- WIT `region-id` is a `string`; SDK `RegionId = u64`. The host serializes u64→string and the SDK
  parses back — that round-trip (not a numeric WIT field) is why the guest casts work.
- The finalization-input deep-copy (macro drain) reconstructs full `PrintEntity`s from
  `print-entity-view`, which is the sole reason that record carries `tool-index`.
- `overlay_resolved` writes only fields differing from `ResolvedConfig::default()`; the per-tool
  overlay reuses the global-based `resolve_per_tool_configs` and is applied like the existing paint
  overlay (correct in the common case where global geometry is default).

## Locked Assumptions and Invariants

- `DEFAULT_TOOL = 0` floor, emit `MAX_PLAUSIBLE_TOOLS` guard, and `>1 GiB` allocator tripwire are
  PERMANENT belt-and-suspenders — must not be removed even though the split makes the leak structurally
  impossible (AC-N1, AC-N2 lock this).
- `region_key.region_id` is a PURE region identity post-split; no consumer may store a tool there.
- D14: `SlicedRegion.segment_annotations` is modifier-volume-only; FuzzySkin rides `variant_chain`.

## Risks and Tradeoffs

- Wide blast radius across guests → mitigated by the `Default`-less compiler checklist + a guest
  rebuild + the full bucket after each Part.
- Per-tool overlay reuses the paint-overlay's "global-based" merge → has the same pre-existing
  global-clobber quirk when a user sets a GLOBAL geometry value AND a per-region override; documented,
  consistent with paint, and irrelevant in the common (default-global) case.

## Context Cost Estimate

Aggregate L (this is why the original packet deferred it). Largest single step: Part A2 (~43
construction sites) — mechanical, compiler-driven, delegable. Highest-risk dispatch: the full executor
bucket FACT after Part A (the subset-green/bucket-red trap).

## Open Questions

None. All forks resolved (see Selected Approach); all ACs green; workspace 2307/0.
