# Requirements: 91_paint-pipeline-schema-scaffolding

## Packet Metadata

- Grouped task IDs:
  - `TASK-241` — Paint-pipeline schema scaffolding (IR additions, renames, version bumps; behavior-preserving).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1a — Schema scaffolding"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The paint-pipeline OrcaSlicer-parity roadmap reaches three architectural decisions that require IR shape changes before any new behavior can land:

- **D8 (inline polygons into SliceIR)**: each `SlicedRegion` must carry its own `variant_chain` (an ordered sequence of `(paint_semantic_name, PaintValue)` pairs) so the region splitter can emit per-variant `SlicedRegion`s into the existing `SliceIR.regions[*]` vector. Today there is no such field — `PaintRegionIR` carries variant-equivalent information separately, breaking the principle that "a region's paint identity is part of the region", and downstream consumers must cross-reference multiple IR slots.
- **D10 (ConfigId(u32) + interning)**: with region-splitting, the same `ResolvedConfig` is replicated across N painted-variant `RegionPlan`s (a 16-color object can produce 16+ duplicates per region per layer). Inlining the full `ResolvedConfig` (22 f32 fields + extension map) on every plan blows memory accounting. The fix is an interner: `RegionMapIR.configs: Vec<ResolvedConfig>` holds unique configs, `RegionPlan.config: ConfigId(u32)` references the Vec.
- **D11 (`boundary_paint` → `segment_annotations`)**: the field's documented scope was "per-contour-point paint data", and most of its consumers treat it as such. Renaming clarifies (a) its narrowed scope under the new region-splitting model — it carries paint semantics NOT declared `[[region_split]]` — and (b) signals the breaking SliceIR 1.0.0 → 2.0.0 transition.

None of the dependent packets (92 manifest+dispatch, 93 RegionMapping cross-product, 95 paint-segmentation port) can land without these scaffolds. This packet lands them all in one breaking-but-behavior-preserving slice so the IR shape is in place before the first behavior change. The defaults are chosen so every existing test continues to pass byte-identically: empty `variant_chain` everywhere, a single-entry `configs` Vec on every `RegionMapIR`, no rename-induced behavior change (`segment_annotations` has the same map type and content as `boundary_paint`).

The supporting Hash-derivation work on `ResolvedConfig` (D10 prerequisite — Vec<ResolvedConfig> needs Hash to intern by content) and `PaintValue` (D10 prerequisite — `variant_chain` is a Hash key) is included here as the smallest indivisible unit; splitting it further would leave the IR in an uncompilable interim state.

## In Scope

- Add `RegionKey.variant_chain: Vec<(String, PaintValue)>` (empty default).
- Add `pub struct ConfigId(pub u32);` newtype in `slicer-ir` with `Copy + Clone + Debug + Hash + Eq + PartialEq`.
- Add `RegionMapIR.configs: Vec<ResolvedConfig>` interner Vec.
- Change `RegionPlan.config` from `ResolvedConfig` to `ConfigId`.
- Add `RegionMapIR::config_for(&self, key: &RegionKey) -> &ResolvedConfig` and `RegionMapIR::intern_config(&mut self, rc: ResolvedConfig) -> ConfigId` helpers.
- Rename `SlicedRegion.boundary_paint` → `SlicedRegion.segment_annotations` (same map type).
- Add `SlicedRegion.variant_chain: Vec<(String, PaintValue)>` (empty default).
- Bump `SliceIR` schema 1.0.0 → 2.0.0.
- Bump `RegionMapIR` schema 1.0.0 → 2.0.0.
- Update every `BuiltinProducer` constant's `min_ir_schema`/`max_ir_schema` admission to admit 2.x.
- Migrate `ResolvedConfig.extensions: HashMap → BTreeMap` (Hash prerequisite).
- Derive `Hash` on `ResolvedConfig` (with `to_bits()` for f32 fields, manual impl if needed).
- Derive (or manually impl) `Eq + Hash` on `PaintValue` (Scalar via `to_bits()`, Custom via String).
- Update production call sites that read `RegionPlan.config` directly: `crates/slicer-runtime/src/prepass_slice.rs:275`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs:348` and `:390`, `crates/slicer-runtime/src/layer_executor.rs:783`, `crates/slicer-runtime/src/dispatch.rs:1975` and `:2009` — route through `region_map.config_for(&key)`.
- Delete `HashablePaintValue` wrapper in `crates/slicer-core/src/algos/paint_segmentation.rs:117` (PaintValue is Hashable now).
- Mechanical sed across ~20 test files that name `boundary_paint`.
- Rebuild guests; confirm `cargo xtask build-guests --check` clean.

## Out of Scope

- Manifest `[[region_split]]` schema — P1b (92).
- Host-filtered dispatch in layer-executor — P1b (92).
- RegionMapping cross-product expansion (populating non-empty `variant_chain`) — P1c (93).
- Mesh-segmentation host kernel wiring — P2 (94).
- Paint-segmentation kernel port — P3 (95).
- Population of `segment_annotations` for any new semantic — P3 (95).
- Deletion of `PaintRegionIR` — P3 (95).
- Doc updates to `docs/02_ir_schemas.md` etc. — P5c (99).
- Any change to `pnp_cli` g-code emission paths.
- Any change to the test fixtures themselves (P0a/P0b territory).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1a — Schema scaffolding" (~80 lines; read directly).
- `docs/02_ir_schemas.md` — current shapes for `SliceIR`, `RegionMapIR`, `RegionKey`, `RegionPlan`, `SlicedRegion`, `ResolvedConfig`, `PaintValue`. Range-read only the sections naming those types. File length variable; delegate if > 300 lines.
- `docs/05_module_sdk.md` — `BuiltinProducer` constant shape, `min_ir_schema` semantics. Range-read.
- `docs/specs/orca-paint-segmentation-parity.md` — context on the region-splitting model. Read §"IR types" only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion` / `FuzzySkinPaintedRegion` shape; confirm via SUMMARY that the structure uses a parent region pointer + paint discriminator (matches our `variant_chain` design).

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11` from `packet.spec.md`. Refinements:
  - The new `RegionMapIR::config_for` helper MUST handle the legacy single-entry case (one interned ResolvedConfig, every key maps to it) so callers don't need conditional logic during the scaffolding window.
  - The `Hash` derive on `ResolvedConfig` may require a manual impl if any f32 field cannot be wrapped — document the chosen approach in the closure log.
- Negative cases: `AC-N1` (no `RegionPlan` constructed with old shape), `AC-N2` (guest WASM clean), `AC-N3` (`boundary_paint` gone from implementation tree, doc tree exempt until P5c).
- Cross-packet impact: unblocks P1b, P1c, P2, P3.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles after IR changes | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-N2 — guest WASM up to date | FACT pass/fail |
| `cargo test -p slicer-ir 2>&1 \| tee target/test-output.log` | AC-1, AC-2, AC-4, AC-7 — IR type tests pass | FACT pass/fail |
| `cargo test -p slicer-core 2>&1 \| tee target/test-output.log` | AC-9 — slicer-core tests pass (HashablePaintValue removal) | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e 2>&1 \| tee target/test-output.log` | AC-10 — e2e tests pass byte-identically | FACT pass/fail with overall count |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p91-wedge.gcode && sha256sum /tmp/p91-wedge.gcode` | AC-10 — byte-identical g-code | FACT (single sha256 hash); compare to pre-packet baseline in closure log |
| `cargo test --workspace 2>&1 \| tee target/test-output.log` | AC-11 — workspace gate (required at close) | FACT pass/fail with count; dispatch to sub-agent (CLAUDE.md §Test Discipline) |
| `! rg -n --glob '!.ralph/specs/91_paint-pipeline-schema-scaffolding/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' 'boundary_paint' crates/ modules/` | AC-N3 — implementation tree clean | FACT pass/fail |

`cargo test --workspace` is justified for this packet's final gate per `CLAUDE.md` §Test Discipline rule 2 — the schema bump touches enough crates that the workspace-wide gate is the only reliable confirmation of byte-identical preservation. All other verifications use targeted per-crate commands.

## Step Completion Expectations

- The `BTreeMap` migration for `ResolvedConfig.extensions` (Step 1's sub-step) MUST land before `Hash` is derived (Step 2). HashMap is not Hash-deriveable.
- The `Hash for PaintValue` work MUST land before `HashablePaintValue` deletion (Step 5). Reverse order leaves the workspace uncompilable.
- The schema-version bump (Step 4) is purely a constant change and may land in any order relative to AC-1/2/3/4 field additions, but the `BuiltinProducer` admission update must accompany it in the same Step so guest WASM bindgen sees a consistent version pair.
- Step 7 (production call-site migration) requires ALL prior IR-shape steps complete; otherwise the migrated call sites won't compile.
- AC-10's byte-identical g-code check is a CONTRACT: any g-code diff must be investigated before declaring the step complete. The diff is allowed only if the closure log documents an unambiguous root cause (e.g., a known integer-rounding interaction with the interner that the implementer can prove is semantically equivalent) — never silently accepted.

## Context Discipline Notes

- `crates/slicer-ir/src/slice_ir.rs` is the primary edit site. If it exceeds 600 lines, range-read by symbol name (locate `pub struct RegionKey`, `pub struct RegionPlan`, `pub struct SlicedRegion`, `pub struct RegionMapIR`, `pub struct PaintValue`) rather than full-read.
- `crates/slicer-ir/src/resolved_config.rs` may be > 400 lines (22 f32 fields, lots of helpers). Range-read the `pub struct ResolvedConfig` block + the `impl ResolvedConfig` block. Do not load auxiliary helpers unless they break compile.
- `crates/slicer-runtime/src/dispatch.rs` is large (likely > 2000 lines). The two call sites (1975 and 2009) are narrow; use ranged `Read` with `offset: 1960, limit: 60` and `offset: 1995, limit: 60`. Do not load the whole file.
- The seven `BuiltinProducer` files under `crates/slicer-runtime/src/builtins/` are each tiny (≤ 60 lines). They can each be read in full.
- The cube `.3mf` fixtures from P0a are NOT consumed by this packet (no test fixture additions); avoid temptation to investigate paint distribution.
