---
status: implemented
packet: 125_voronoi-oom-hardening
task_ids: []   # none — bug-fix slice from the 2026-06-24 diagnose session, expanded in place.
backlog_source: docs/07_implementation_status.md
context_cost_estimate: L
implemented: 2026-06-25   # all ACs green; executor bucket 167/0; workspace 2307/0.
---

# Packet Contract: 125_voronoi-oom-hardening

> **Implementation status: COMPLETE (2026-06-25).** Every AC below is green; the full workspace is
> 2307 passed / 0 failed. This rewrite supersedes the original "bound the OOM, defer the split"
> contract: the deferred `region_id`↔tool split was implemented here, plus D14 fuzzy routing, a
> per-tool config axis (incl. painted per-tool geometry), and voronoi hardening. The packet now serves
> as the **verification record** — a reviewer runs the AC commands to confirm each shipped change.
> `HANDOFF.md` in this directory is the historical deferral note and is superseded by this packet.

## Goal

Eliminate the painted-model OOM at its root by separating the dual-purpose `RegionKey.region_id` into
a first-class `PrintEntity.tool_index` (pure tool selector) and a pure region identity, then build the
clean axes the split unlocks: D14-correct fuzzy-skin routing, a per-tool config overlay (emit-time
settings + painted-tool geometry), and deterministic containment of the boostvoronoi failure modes.

## Scope Boundaries

This packet splits the `region_id`/tool conflation across the IR, two WIT worlds (`ir-handles`,
`world-finalization`), the host bridge, the SDK, the macro drain, the emitter, and the perimeter/
finalization guests; routes painted FuzzySkin through a new `slice-region-view.variant-chain` WIT
accessor (D14); adds the `tool_config:<idx>:<key>` config axis (emit-time + painted-tool RegionMapping
overlay); and bounds the boostvoronoi builder (input cap + `catch_unwind`). It retains every packet-125
safety net (`DEFAULT_TOOL=0` floor, emit `MAX_PLAUSIBLE_TOOLS` guard, `>1 GiB` allocator tripwire). It
does **not** deliver per-tool geometry for *non-painted* tools (resolved post-perimeter), an upstream
boostvoronoi loop patch, or per-tool nozzle-diameter→width cascade — all explicitly out of scope below.

## Acceptance Criteria

### Part A — `region_id`↔tool split

- **AC-1. Given** an assembled wall/infill entity whose four tool resolvers all return `None` (region
  carries the captured paint-variant identity `0x3E8281949ECA9508`), **when** `assemble_ordered_entities`
  runs, **then** `PrintEntity.tool_index == DEFAULT_TOOL (0)` (the identity never reaches the tool slot)
  AND `PrintEntity.region_key.region_id` equals the source identity `0x3E8281949ECA9508` (the identity
  is preserved, no longer overwritten by the tool). | `cargo test -p slicer-runtime --lib tool_fallback_never_leaks_region_identity`
- **AC-2. Given** a perimeter+infill+support layer, **when** entities are assembled, **then** each
  `region_key.region_id` is the source region identity (e.g. `1`, `2`, `0`), not the resolved tool. |
  `cargo test -p slicer-runtime --test executor ordered_entities_assembled_with_preserved_region_identity`
- **AC-3. Given** a layer-world deep-copy commit with seeded region ids `11`/`22`, **when** committed,
  **then** entity `region_key.region_id` round-trips as `11`/`22` and tool changes + z-hops survive. |
  `cargo test -p slicer-runtime --test executor layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops`
- **AC-4. Given** two regions whose tool is supplied only via `RegionPlan.config.extensions["extruder"]`
  (`0` and `1`), **when** assembled, **then** the resolved tools appear in `PrintEntity.tool_index`
  (`{0,1}`), not in `region_key.region_id`. | `cargo test -p slicer-runtime --test executor extruder_synthetic_t0_t1_emission`
- **AC-5. Given** a cross-object layer whose two regions carry material `ToolIndex(1)`/`ToolIndex(2)`
  and raw order `[A,A,B,B]`, **when** the live path-optimization guest runs, **then** entities are
  grouped by `tool_index` → `x = [0.0, 0.0, 1.0, 1.0]` (the guest reads `OrderedEntityView.tool_index`,
  not `region_id`). | `cargo test -p slicer-runtime --test unit cross_object_ordering_resequences_entities_by_travel_cost`
- **AC-6. Given** the additive `PrintEntity.tool_index` field, **when** the IR schema version is
  checked, **then** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION == 1.1.0`. | `cargo test -p slicer-ir --test ir_tests slice_ir_schema_version_is_one_one_zero`

### Part B — D14 fuzzy-skin routing

- **AC-7. Given** `cube_fuzzyPainted.3mf`, **when** paint segmentation runs, **then** FuzzySkin reaches
  `SlicedRegion.variant_chain` as `("fuzzy_skin", Flag(true))` AND **no** region's `segment_annotations`
  contains the `FuzzySkin` key (D14: that channel is modifier-volume-only). | `cargo test -p slicer-runtime --test executor paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain`
- **AC-8. Given** the same fixture sliced through the executor, **when** perimeters generate, **then**
  the painted face still jitters (`painted_face_pts > 2 × unpainted_face_pts`) — fuzzy reached the
  perimeter guest via the new `slice-region-view.variant-chain()` WIT accessor, not `segment_annotations`. |
  `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter`

### Part C — per-tool config (`tool_config:<idx>:<key>`)

- **AC-9. Given** a config source with `tool_config:1:retract_length=5.5` and a non-numeric
  `tool_config:bogus:…` key, **when** `resolve_per_tool_configs` runs, **then** the result map has
  `[1].retract_length == 5.5`, omits tool `0` (no override), and skips the non-numeric index (empty for
  that input) — never erroring the resolution. | `cargo test -p slicer-scheduler --test scheduler_integration resolver_per_tool`
- **AC-10. Given** a `DefaultGCodeEmitter` carrying `tool_configs = {1: retract_length 6.5}` with global
  `retract_length 2.0`, **when** retract length is resolved, **then** `retract_length_for_tool(1) == 6.5`
  and `retract_length_for_tool(0) == 2.0` (per-tool overrides global at emit). | `cargo test -p slicer-gcode per_tool_config_overrides_retract_length`
- **AC-11. Given** an object painted material `ToolIndex(1)`/`ToolIndex(2)` and `tool_configs = {1:
  line_width = default+0.2}`, **when** `execute_region_mapping_inner` runs, **then** the `RegionPlan` for
  the `("material", ToolIndex(1))` chain carries the overridden `line_width`, while the `ToolIndex(2)`
  and base chains keep the default (per-tool overlay applied at highest precedence, painted tools only). |
  `cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd region_mapping_applies_per_tool_config_overlay_to_painted_tool`
- **AC-12. Given** the classic perimeter module, **when** `run_perimeters` runs with per-region
  `line_width` `0.4` then `0.8`, **then** the emitted outer-wall extrusion width equals the configured
  `line_width` exactly (closing the previously-untested config→geometry link; with AC-11 this is per-tool
  `line_width` end-to-end). | `cargo test -p classic-perimeters --test classic_perimeters_tdd per_region_line_width_sets_emitted_wall_width`

### Part D/E — voronoi hardening

- **AC-13. Given** an input of `MAX_VORONOI_SEGMENTS + 1` distinct segments, **when**
  `MMU_Graph::from_colored_lines` runs, **then** it returns `Err(MmuGraphError::InputTooLarge { cap })`
  **before** invoking the boostvoronoi builder — no hang, no panic (bounds the latent `discretize`
  loop). | `cargo test -p slicer-core --features host-algos oversized_input_returns_input_too_large`
- **AC-14. Given** the minimal collinear-overlapping degenerate fixture, **when**
  `from_colored_lines` runs, **then** it completes with `Ok` (the `merge_collinear_overlapping`
  precondition + the `catch_unwind` backstop convert any `fpv.is_finite()` panic into a typed
  `Result` instead of aborting the process). | `cargo test -p slicer-core --features host-algos collinear_overlapping_segments_do_not_panic_the_builder`

## Negative Test Cases

- **AC-N1. Given** a synthetic entity whose tool index is an out-of-range value (e.g.
  `2_664_076_552`), **when** `slicer-gcode/src/emit.rs` sizes the per-tool buffer, **then** it returns
  `GCodeEmitError::ToolIndexOutOfRange` instead of allocating `vec![0.0f32; id + 1]` (no >1 GiB
  allocation). | `cargo test -p slicer-gcode emit_rejects_out_of_range_tool_id`
- **AC-N2. Given** the guarded `>1 GiB` allocator active in the executor bucket, **when** the painted
  path runs 10× in a loop, **then** no single allocation exceeds 1 GiB (the tripwire never fires; exit
  code is not 173). | `cargo test -p slicer-runtime --test executor -- mmu_no_oversized_alloc_repeat`

## Verification

Gate commands (closure check runs these; the full matrix lives in `requirements.md`):

- `cargo test -p slicer-runtime --test executor`  (expect **167 passed / 0 failed**; was 164/3)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`  (must be clean — this packet touches universal guest deps)

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PrintEntity.tool_index`, `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`,
  Config Key Namespaces (`tool_config:` + precedence). Large — read only the named sections.
- `docs/03_wit_and_manifest.md` — `ordered-entity-view.tool-index`, `print-entity-view.tool-index`,
  `slice-region-view.variant-chain`, finalization push `tool-index` params. Large — named records only.
- `CLAUDE.md` §"Guest WASM Staleness", §"Test Discipline" — small, load directly.
- `docs/DEVIATION_LOG.md` — entry `D-125-TOOL-IDENTITY-SPLIT` records the in-place scope expansion.

## Doc Impact Statement (Required)

**NOT `none`.** The split is a contract change. `docs/02_ir_schemas.md` documents `PrintEntity.tool_index`
and the layer-collection schema bump (1.0.0→1.1.0) plus the `tool_config:<idx>:<key>` namespace and
revised precedence. `docs/03_wit_and_manifest.md` documents the new WIT fields/accessors
(`ordered-entity-view.tool-index`, `print-entity-view.tool-index`, `slice-region-view.variant-chain`)
and the finalization `tool-index` params. `docs/DEVIATION_LOG.md` records `D-125-TOOL-IDENTITY-SPLIT`
(the original packet's `none` Doc Impact + "separate refactor" scoping were falsified by the full-bucket
acceptance ceremony). All four doc edits are landed.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — filament-preset overrides are applied LAST (highest precedence); informs the `per_tool` precedence decision (Part C).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `region_config_from_model_volume` layering order (print < object < modifier < material < layer-range); the baseline our `tool_config` axis extends.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` / `PerimeterGenerator.cpp` — extrusion width derives from the region's extruder `nozzle_diameter` (a base, % widths); documents why OrcaSlicer has no per-filament line-width and ours is a deliberate superset.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
