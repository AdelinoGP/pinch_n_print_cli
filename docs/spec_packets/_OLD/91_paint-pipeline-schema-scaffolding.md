---
status: implemented
packet: 91
task_ids: [TASK-241]
---

# 91_paint-pipeline-schema-scaffolding

## Goal

Land the IR type additions, field renames, and schema-version bumps required by design decisions D8, D10, D11 from the paint-pipeline OrcaSlicer-parity roadmap — **without changing any runtime behavior**: add `RegionKey.variant_chain: Vec<(String, PaintValue)>` (empty default), introduce `ConfigId(u32)` newtype and `RegionMapIR.configs: Vec<ResolvedConfig>` interner lookup table (with `RegionPlan.config` migrated to `ConfigId` referencing it), rename `SlicedRegion.boundary_paint` to `segment_annotations`, add `SlicedRegion.variant_chain: Vec<(String, PaintValue)>` (empty default), bump `SliceIR` 1.0.0 → 2.0.0 and `RegionMapIR` 1.0.0 → 2.0.0 across every `BuiltinProducer` constant's `min_ir_schema` / `max_ir_schema` admission, migrate `ResolvedConfig.extensions` from `HashMap` to `BTreeMap` so the type can derive `Hash` (interner prerequisite), derive `Eq + Hash` on `PaintValue` (with `Scalar(f32)` hashing via `to_bits()` and `Custom(String)` via its String), update all 4 production call sites that read `RegionPlan.config` directly (`prepass_slice.rs:275`, `slice_postprocess_prepass.rs:348`/`:390`, `layer_executor.rs:783`, `dispatch.rs:1975`/`:2009`) to route through `region_map.config_for(&key)`, and delete the now-redundant `HashablePaintValue` wrapper in `paint_segmentation.rs:117` (PaintValue is itself Hashable now), so that every existing test in the workspace continues to pass byte-identically (no g-code differs from pre-packet baseline) while the IR has the shape downstream packets 92, 93, 95 will populate.

## Problem Statement

The paint-pipeline OrcaSlicer-parity roadmap reaches three architectural decisions that require IR shape changes before any new behavior can land:

- **D8 (inline polygons into SliceIR)**: each `SlicedRegion` must carry its own `variant_chain` (an ordered sequence of `(paint_semantic_name, PaintValue)` pairs) so the region splitter can emit per-variant `SlicedRegion`s into the existing `SliceIR.regions[*]` vector. Today there is no such field — `PaintRegionIR` carries variant-equivalent information separately, breaking the principle that "a region's paint identity is part of the region", and downstream consumers must cross-reference multiple IR slots.
- **D10 (ConfigId(u32) + interning)**: with region-splitting, the same `ResolvedConfig` is replicated across N painted-variant `RegionPlan`s (a 16-color object can produce 16+ duplicates per region per layer). Inlining the full `ResolvedConfig` (22 f32 fields + extension map) on every plan blows memory accounting. The fix is an interner: `RegionMapIR.configs: Vec<ResolvedConfig>` holds unique configs, `RegionPlan.config: ConfigId(u32)` references the Vec.
- **D11 (`boundary_paint` → `segment_annotations`)**: the field's documented scope was "per-contour-point paint data", and most of its consumers treat it as such. Renaming clarifies (a) its narrowed scope under the new region-splitting model — it carries paint semantics NOT declared `[[region_split]]` — and (b) signals the breaking SliceIR 1.0.0 → 2.0.0 transition.

None of the dependent packets (92 manifest+dispatch, 93 RegionMapping cross-product, 95 paint-segmentation port) can land without these scaffolds. This packet lands them all in one breaking-but-behavior-preserving slice so the IR shape is in place before the first behavior change. The defaults are chosen so every existing test continues to pass byte-identically: empty `variant_chain` everywhere, a single-entry `configs` Vec on every `RegionMapIR`, no rename-induced behavior change (`segment_annotations` has the same map type and content as `boundary_paint`).

The supporting Hash-derivation work on `ResolvedConfig` (D10 prerequisite — Vec<ResolvedConfig> needs Hash to intern by content) and `PaintValue` (D10 prerequisite — `variant_chain` is a Hash key) is included here as the smallest indivisible unit; splitting it further would leave the IR in an uncompilable interim state.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- IR-version bump invariant: `BuiltinProducer.min_ir_schema` stays at `1.0.0` (not bumped to `2.0.0`) — admitting both 1.x and 2.x lets in-flight test guests that still compile against 1.x keep working through the scaffolding window. `max_ir_schema` stays at `4.0.0` (unchanged) so headroom for future bumps remains.
- Hash invariant on `ResolvedConfig`: the implementation MUST hash f32 fields via `to_bits()`, never via float equality. The interner identity check uses the derived Hash; two configs that differ only in a NaN payload's bit pattern would otherwise collide. Document this as "consistent within one process; not portable across architectures with differing NaN bit patterns" in the type's doc-comment.
- Behavior preservation invariant: every existing test passes byte-identically. The interner produces a 1-entry `configs` Vec on legacy single-config flows; `ConfigId(0)` resolves to that single config; `RegionMapIR::config_for` returns the same `&ResolvedConfig` reference layout the previous direct-inline shape did.

## Data and Contract Notes

- IR or manifest contracts touched: `SliceIR` 1.0.0 → 2.0.0; `RegionMapIR` 1.0.0 → 2.0.0. Breaking renames + field additions. All current `BuiltinProducer`s admit 1.x and 2.x simultaneously through the scaffolding window.
- WIT boundary considerations: `slicer-ir` types feed `slicer-macros` via `slicer-schema` → guest bindgen output. Guests must rebuild after this packet; that's why `cargo xtask build-guests --check` is in AC-N2.
- Determinism or scheduler constraints: the BTreeMap migration makes config iteration deterministic (previously the HashMap iteration order varied across runs). Any production code that relied on a *specific* HashMap iteration order is now deterministic; any that relied on iteration-order *variation* would break, but no such code is expected (deliberately-non-deterministic code is anti-pattern).

## Locked Assumptions and Invariants

- **Behavior preservation**: every existing test must pass byte-identically. Any g-code diff against the pre-packet baseline is a bug.
- **Hash invariant**: `ResolvedConfig::hash` produces consistent values within one process. Cross-architecture or cross-process portability is NOT guaranteed (NaN bit patterns differ). The interner is scoped to a single prepass invocation, so this is acceptable.
- **Empty-default invariant**: `RegionKey::default()` and `SlicedRegion::default()` produce empty `variant_chain`. Any code path that constructed these with non-empty variant_chain before this packet did not exist (P1c is the first packet to populate it).
- **Schema admission invariant**: `BuiltinProducer.min_ir_schema` is the LOWEST schema admitted, not the current. Keeping it at 1.0.0 is by design — admits old test guests during the scaffolding window.

## Risks and Tradeoffs

- **Risk: a g-code diff appears against the pre-packet baseline** because BTreeMap iteration order differs from the prior HashMap order in a path that emits g-code. Mitigation: AC-10 captures the baseline before this packet starts and compares post-packet; any diff blocks closure until root-caused. Most likely culprits: `extensions` iteration in a producer or g-code emitter.
- **Risk: a guest module's bindgen output silently keeps the old IR shape** because its `target/` directory wasn't cleaned. Mitigation: AC-N2's `--check` catches this; if STALE: is reported, rebuild without `--check` and re-test.
- **Risk: a downstream call site reads `RegionPlan.config` via a method this packet didn't catch** because the grep dispatch missed a pattern (e.g., `let cfg = &plan.config;` instead of `plan.config.method()`). Mitigation: the `cargo check --workspace --all-targets` gate catches type errors; the change from `ResolvedConfig` to `ConfigId` is the most error-surfacing single edit in the packet.
- **Tradeoff: the linear-scan interner is O(N) per intern call.** For N up to a few thousand distinct configs (per print job), this is fine. A HashSet-based interner could be O(1) per intern but requires `ResolvedConfig: Hash + Eq` and a separate lookup structure. Defer until profiling.
