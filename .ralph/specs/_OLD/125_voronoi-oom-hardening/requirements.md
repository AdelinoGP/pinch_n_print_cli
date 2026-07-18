# Requirements: 125_voronoi-oom-hardening

## Problem Statement

A painted/MMU model (`cube_fuzzyPainted.3mf`) crashed the slicer with a 9.9 GiB allocation. The
diagnose session pinned the chain: `RegionKey.region_id` is **dual-purpose** — both a region IDENTITY
(a 64-bit `PaintValue` hash, e.g. `0x3E8281949ECA9508`) and the slot the resolved TOOL index is stored
into. A painted region's identity (`as u32 = 2_664_076_552`) leaked through the tool slot into
`slicer-gcode/src/emit.rs`, which sized a dense `vec![0.0f32; max_tool + 1]` ≈ 9.92 GiB.

The original packet 125 *bounded* the crash (a `DEFAULT_TOOL=0` resolver floor, a `MAX_PLAUSIBLE_TOOLS`
emit guard, and a `>1 GiB` allocator tripwire) but **deferred the real fix** as "a separate refactor"
and left three executor-bucket tests intentionally red. The full-bucket acceptance ceremony falsified
that scoping: `region_id` is read for **opposite** purposes by different consumers (emit/path-opt as a
tool; postpass back-refs as an identity), so the conflation cannot be fixed in one field without
breaking a consumer. The deferred split therefore had to be done — and once `tool_index` is
first-class, the clean axes it unlocks (D14 fuzzy routing, per-tool config) and the remaining
boostvoronoi failure modes were folded into the same coherent slice (per user direction).

This matters because: (1) painted/MMU models cannot slice at all until the split lands; (2) the
`region_id`-as-identity back-reference that postpasses depend on must be restored; (3) the painted
FuzzySkin path and the boostvoronoi builder both have latent aborts on real painted geometry.

## Task IDs

None (`task_ids: []`). Diagnose-driven bug-fix slice, expanded in place per user direction; the
deviation is recorded as `D-125-TOOL-IDENTITY-SPLIT` in `docs/DEVIATION_LOG.md`.

## In Scope

- **Part A — `region_id`↔tool split:** add `PrintEntity.tool_index: u32` (additive, schema 1.0→1.1);
  carry the tool to the path-opt guest via `ordered-entity-view.tool-index` + the host
  `dispatch::OrderedEntityView` intermediate + SDK `OrderedEntityView`; carry it to finalization via
  explicit `tool-index` params on `push-entity-to-layer`/`-with-priority`/`insert-entity-at` (+
  `print-entity-view.tool-index` for the finalization input deep-copy reconstruction); flip emit and
  the path-opt guest to read `tool_index`; restore `region_key.region_id` to a pure identity in
  assembly. Fix all `PrintEntity` construction sites.
- **Part B — D14 fuzzy routing:** stop synthesizing FuzzySkin into `SlicedRegion.segment_annotations`;
  expose `slice-region-view.variant-chain()` over WIT; thread a `variant_fuzzy` flag into
  `build_wall_flags` in both perimeter guests (arachne + classic).
- **Part C — per-tool config:** `tool_config:<idx>:<key>` resolved by `resolve_per_tool_configs`;
  consumed at emit (per-tool `retract_length`) AND at `RegionMapping` for painted tools (per-tool
  geometry overlay at highest precedence).
- **Part D — boostvoronoi input guard:** `MAX_VORONOI_SEGMENTS` cap + typed `InputTooLarge` before the
  builder.
- **Part E — fpv-panic containment:** `catch_unwind` around the builder converting the
  `fpv.is_finite()` panic into a typed `MmuGraphError::PredicatePanic`.
- Retain the packet-125 safety nets (`DEFAULT_TOOL=0`, emit guard, tripwire).
- Doc edits: `docs/02`, `docs/03`, `docs/DEVIATION_LOG.md`.

## Out of Scope

- Per-tool **geometry** for **non-painted** tools (spatial / modifier-extruder / `DEFAULT_TOOL`
  fallback) — those tools are resolved *after* perimeter generation (`layer_executor.rs:597,747-751`);
  delivering it needs a pipeline-ordering change.
- An upstream boostvoronoi `discretize` loop patch/fork — Part D contains it via an input cap instead.
- Per-tool nozzle-diameter→extrusion-width cascade (OrcaSlicer's native per-extruder width mechanism);
  our `tool_config:<n>:line_width` is a deliberate superset, but the nozzle-derived path is not built.
- `print-entity-view` finalization-input reconstruction is the ONLY reason that record carries
  `tool-index`; no finalization guest reads a tool from it for logic.

## Authoritative Docs

| Doc | Why | Size / delegation |
|---|---|---|
| `docs/02_ir_schemas.md` | `PrintEntity.tool_index`, schema bump, `tool_config:` namespace + precedence | Large — delegate a SUMMARY of the named sections; do not load whole |
| `docs/03_wit_and_manifest.md` | WIT record/method changes (4 surfaces) | Large — delegate LOCATIONS for the named records |
| `CLAUDE.md` | Guest WASM staleness + test discipline | Small — load directly |
| `docs/DEVIATION_LOG.md` | `D-125-TOOL-IDENTITY-SPLIT` rationale | Targeted grep for the entry id |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — filament-preset overrides applied LAST (highest); grounds the `per_tool` = highest precedence choice (Part C).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `region_config_from_model_volume` precedence (print < object < modifier < material < layer-range).
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` / `PerimeterGenerator.cpp` — width derives from the region's extruder `nozzle_diameter`; documents the absence of per-filament line-width in OrcaSlicer.

## Acceptance Summary

Acceptance is defined by `packet.spec.md` AC-1 … AC-14 + AC-N1 … AC-N2 (referenced by ID; not copied).
Measurable refinements that did not fit Given/When/Then:

- **AC-1** uses the captured crash value `region_id = 0x3E8281949ECA9508` as the leak probe; both the
  tool-slot-is-0 and the identity-is-preserved assertions must hold in the same entity.
- **AC-7** is host-IR-only (asserts on `execute_paint_segmentation` output); **AC-8** is the guest-side
  jitter confirmation that the WIT `variant-chain` projection actually reaches `build_wall_flags`.
- **AC-11** precedence is `global < per_object < per_paint_semantic < per_tool` (per-tool highest), and
  the overlay fires only for painted/material chains; non-painted regions are unaffected (the
  region-mapping bucket stays behavior-neutral when no `tool_config:` keys are set).
- **Whole-packet invariant:** the full executor bucket is **167 passed / 0 failed** (was 164/3), and
  the workspace `--no-fail-fast` suite is **2307/0**. The packet-125 floor/guard/tripwire remain in
  place (AC-N1, AC-N2 prove no regression re-opens the OOM).

## Verification Commands (full matrix)

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `cargo test -p slicer-runtime --lib tool_fallback_never_leaks_region_identity` | FACT pass/fail |
| AC-2 | `cargo test -p slicer-runtime --test executor ordered_entities_assembled_with_preserved_region_identity` | FACT |
| AC-3 | `cargo test -p slicer-runtime --test executor layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops` | FACT |
| AC-4 | `cargo test -p slicer-runtime --test executor extruder_synthetic_t0_t1_emission` | FACT |
| AC-5 | `cargo test -p slicer-runtime --test unit cross_object_ordering_resequences_entities_by_travel_cost` | FACT (live WASM guest) |
| AC-6 | `cargo test -p slicer-ir --test ir_tests slice_ir_schema_version_is_one_one_zero` | FACT |
| AC-7 | `cargo test -p slicer-runtime --test executor paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain` | FACT |
| AC-8 | `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter` | FACT (ignore `fpv` stderr noise) |
| AC-9 | `cargo test -p slicer-scheduler --test scheduler_integration resolver_per_tool` | FACT (2 tests) |
| AC-10 | `cargo test -p slicer-gcode per_tool_config_overrides_retract_length` | FACT |
| AC-11 | `cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd region_mapping_applies_per_tool_config_overlay_to_painted_tool` | FACT (needs `--features host-algos`) |
| AC-12 | `cargo test -p classic-perimeters --test classic_perimeters_tdd per_region_line_width_sets_emitted_wall_width` | FACT |
| AC-13 | `cargo test -p slicer-core --features host-algos oversized_input_returns_input_too_large` | FACT (~1.5s) |
| AC-14 | `cargo test -p slicer-core --features host-algos collinear_overlapping_segments_do_not_panic_the_builder` | FACT |
| AC-N1 | `cargo test -p slicer-gcode emit_rejects_out_of_range_tool_id` | FACT |
| AC-N2 | `cargo test -p slicer-runtime --test executor -- mmu_no_oversized_alloc_repeat` | FACT (no exit 173) |
| Gate | `cargo test -p slicer-runtime --test executor` | FACT 167/0 (tee to `target/test-output.log`, read the file) |
| Gate | `cargo clippy --workspace --all-targets -- -D warnings` | FACT clean |
| Gate | `cargo xtask build-guests --check` | FACT clean (rebuild without `--check` if `STALE:`) |

## Step Completion Expectations (cross-step invariants only)

- After ANY guest-dep edit (slicer-ir / slicer-sdk / slicer-core / slicer-schema WIT / a guest's src),
  `cargo xtask build-guests --check` must be run and the guests rebuilt if `STALE:` BEFORE any
  guest/executor test result is trusted. A stale guest produces failures that look unrelated.
- The full `cargo test -p slicer-runtime --test executor` bucket must be run after each Part, not only
  at the end — the subset-green / bucket-red gap is exactly what the original packet missed. Two of
  this packet's own regressions lived in `--lib` and `unit` targets invisible to the executor bucket
  and only surfaced under a workspace `--no-fail-fast` sweep; closure must include that sweep.

## Context Discipline Notes (packet-specific)

- The blast radius is wide (two WIT worlds, host bridge, SDK, macro drain, emitter, 4 guests, ~43
  `PrintEntity` construction sites). Use the missing `Default` on `PrintEntity` as the compiler's
  construction-site checklist; do not hand-enumerate by reading.
- Delegate every `cargo test`/`clippy`/`build-guests` run with a FACT pass/fail return; never absorb
  full test output. `target/test-output.log` holds the last run — read it, do not re-run.
