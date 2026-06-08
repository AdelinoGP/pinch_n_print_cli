---
status: draft
packet: 91
task_ids: [TASK-241]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 91 — Paint Pipeline Schema Scaffolding (behavior-preserving)

## Goal

Land the IR type additions, field renames, and schema-version bumps required by design decisions D8, D10, D11 from the paint-pipeline OrcaSlicer-parity roadmap — **without changing any runtime behavior**: add `RegionKey.variant_chain: Vec<(String, PaintValue)>` (empty default), introduce `ConfigId(u32)` newtype and `RegionMapIR.configs: Vec<ResolvedConfig>` interner lookup table (with `RegionPlan.config` migrated to `ConfigId` referencing it), rename `SlicedRegion.boundary_paint` to `segment_annotations`, add `SlicedRegion.variant_chain: Vec<(String, PaintValue)>` (empty default), bump `SliceIR` 1.0.0 → 2.0.0 and `RegionMapIR` 1.0.0 → 2.0.0 across every `BuiltinProducer` constant's `min_ir_schema` / `max_ir_schema` admission, migrate `ResolvedConfig.extensions` from `HashMap` to `BTreeMap` so the type can derive `Hash` (interner prerequisite), derive `Eq + Hash` on `PaintValue` (with `Scalar(f32)` hashing via `to_bits()` and `Custom(String)` via its String), update all 4 production call sites that read `RegionPlan.config` directly (`prepass_slice.rs:275`, `slice_postprocess_prepass.rs:348`/`:390`, `layer_executor.rs:783`, `dispatch.rs:1975`/`:2009`) to route through `region_map.config_for(&key)`, and delete the now-redundant `HashablePaintValue` wrapper in `paint_segmentation.rs:117` (PaintValue is itself Hashable now), so that every existing test in the workspace continues to pass byte-identically (no g-code differs from pre-packet baseline) while the IR has the shape downstream packets 92, 93, 95 will populate.

## Scope Boundaries

This packet is a pure schema scaffold: every new field has a default that preserves current behavior (empty `variant_chain`, `ConfigId(0)` pointing at the first/only interner entry, single-entry `configs` Vec), every rename is mechanical (`boundary_paint` → `segment_annotations` is a `replace_all` across ~20 test files plus production sites), and every schema-version bump is a constant update across the seven host `BuiltinProducer` definitions. The packet does NOT add any new dispatch logic, does NOT change which modules run where, and does NOT populate `variant_chain` with non-empty content. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packets 89 and 90 are independent — this packet can land before, after, or alongside them.
- Unblocks: P1b (92, manifest + dispatch), P1c (93, RegionMapping cross-product), P3 (95, paint-segmentation port). None of those can land before this packet.
- Activation blockers: confirmation that no other packet is currently `active`. If packet 89 is still `active`, this packet stays `draft` until 89 closes.

## Acceptance Criteria

### AC-1 — `RegionKey` carries `variant_chain: Vec<(String, PaintValue)>` defaulted to empty; `Hash + Eq` derived

**Given** the IR change,
**When** `crates/slicer-ir/src/slice_ir.rs` is inspected,
**Then** `RegionKey` has a field `variant_chain: Vec<(String, PaintValue)>` (in addition to the existing `global_layer_index`, `object_id`, `region_id`); the `Hash` and `Eq` derives still apply (cover `PaintValue`'s new manual `Hash` impl); and `RegionKey::default()` produces an empty `variant_chain`.

| `rg -q 'variant_chain: Vec<\(String, PaintValue\)>' crates/slicer-ir/src/slice_ir.rs && cargo check -p slicer-ir 2>&1 | tee target/test-output.log`

### AC-2 — `ConfigId(u32)` newtype introduced; `RegionMapIR.configs: Vec<ResolvedConfig>` added; `RegionPlan.config: ConfigId` (replaces inline `ResolvedConfig`)

**Given** the interning model in D10,
**When** `crates/slicer-ir/src/slice_ir.rs` is inspected,
**Then** `pub struct ConfigId(pub u32);` exists with `Copy + Clone + Debug + Hash + Eq + PartialEq` derived; `RegionMapIR` has a new field `configs: Vec<ResolvedConfig>` (or equivalent — `Vec<Arc<ResolvedConfig>>` is acceptable if memory-locality is preserved); `RegionPlan.config` is `ConfigId`, not `ResolvedConfig`; a convenience accessor `RegionMapIR::config_for(&self, key: &RegionKey) -> &ResolvedConfig` exists and is documented.

| `rg -q 'pub struct ConfigId\(' crates/slicer-ir/src/slice_ir.rs && rg -q 'configs: Vec<(Arc<)?ResolvedConfig' crates/slicer-ir/src/slice_ir.rs && rg -q 'config: ConfigId' crates/slicer-ir/src/slice_ir.rs && rg -q 'fn config_for' crates/slicer-ir/src/slice_ir.rs && cargo check -p slicer-ir 2>&1 | tee target/test-output.log`

### AC-3 — `SlicedRegion.boundary_paint` renamed to `segment_annotations`; documentation updated

**Given** the rename in D11,
**When** `crates/slicer-ir/src/slice_ir.rs` is inspected,
**Then** `SlicedRegion` has a field `segment_annotations` of the SAME map type as the former `boundary_paint` (`HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>` or equivalent); no field named `boundary_paint` survives anywhere under `crates/slicer-ir/src/`; the field's doc-comment explicitly states "populated only for paint semantics NOT declared `[[region_split]]` in a module manifest".

| `rg -q 'segment_annotations:' crates/slicer-ir/src/slice_ir.rs && ! rg -q 'boundary_paint' crates/slicer-ir/src/`

### AC-4 — `SlicedRegion` gains `variant_chain: Vec<(String, PaintValue)>` (empty default)

**Given** the per-variant SliceIR shape,
**When** `SlicedRegion` is inspected,
**Then** it has a field `variant_chain: Vec<(String, PaintValue)>`; `SlicedRegion::default()` produces an empty `variant_chain`; the field is `pub`.

| `rg -A 2 'pub struct SlicedRegion' crates/slicer-ir/src/slice_ir.rs | rg -q 'variant_chain: Vec<\(String, PaintValue\)>'`

### AC-5 — `SliceIR` schema 1.0.0 → 2.0.0; `RegionMapIR` schema 1.0.0 → 2.0.0; `BuiltinProducer` `min/max_ir_schema` admit 2.x

**Given** the breaking shape changes,
**When** the schema constants are inspected,
**Then** the `SCHEMA_VERSION` (or equivalent) constants for `SliceIR` and `RegionMapIR` in `crates/slicer-ir/src/slice_ir.rs` both report `SemVer { major: 2, minor: 0, patch: 0 }`; every `BuiltinProducer` struct in `crates/slicer-runtime/src/builtins/*.rs` whose `ir_reads` or `ir_writes` includes `SliceIR` or `RegionMapIR` has `min_ir_schema: SemVer { major: 1, minor: 0, patch: 0 }` (admits 1.x and 2.x) and `max_ir_schema: SemVer { major: 4, minor: 0, patch: 0 }` (unchanged headroom).

| `rg -nE 'major:\s*2\b' crates/slicer-ir/src/slice_ir.rs | rg -q SliceIR && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`

### AC-6 — `ResolvedConfig.extensions` migrated from `HashMap` to `BTreeMap`; `Hash` derived

**Given** the interner prerequisite,
**When** `crates/slicer-ir/src/resolved_config.rs` is inspected,
**Then** the `extensions` field is `BTreeMap<...>` (not `HashMap<...>`); `ResolvedConfig` derives `Hash` (with `f32` fields hashed via `to_bits()` — either explicitly via a manual `Hash` impl or via a wrapper newtype that implements `Hash` for each `f32`-bearing field); the `Hash` impl is documented as "consistent within one process; not portable across architectures with differing NaN bit patterns".

| `rg -q 'extensions: BTreeMap' crates/slicer-ir/src/resolved_config.rs && rg -q '(impl Hash for ResolvedConfig|#\[derive\([^)]*Hash[^)]*\)\] *\npub struct ResolvedConfig)' crates/slicer-ir/src/resolved_config.rs && cargo check -p slicer-ir 2>&1 | tee target/test-output.log`

### AC-7 — `PaintValue` derives / impls `Eq + Hash`; all four variants hashable

**Given** PaintValue is used inside `variant_chain` (Hash key),
**When** `crates/slicer-ir/src/slice_ir.rs` (or wherever `PaintValue` is defined) is inspected,
**Then** `PaintValue` has `Eq + Hash` available; `Scalar(f32)` hashes via `to_bits()`; `Custom(String)` hashes via its `String`; `Flag` and `ToolIndex` use their default discriminant-based hash. A unit test asserts that two `PaintValue::Scalar(1.5)` hash to the same value.

| `rg -q '(impl Hash for PaintValue|#\[derive\([^)]*Hash[^)]*\)\] *\n[^;]*PaintValue)' crates/slicer-ir/src/ && cargo test -p slicer-ir paint_value_hash 2>&1 | tee target/test-output.log`

### AC-8 — Four production call sites use `region_map.config_for(&key)` instead of `plan.config` direct read

**Given** the `RegionPlan.config: ConfigId` change,
**When** `crates/slicer-runtime/src/prepass_slice.rs`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `crates/slicer-runtime/src/layer_executor.rs`, and `crates/slicer-runtime/src/dispatch.rs` are inspected,
**Then** every read of `plan.config` (line ranges noted in the roadmap: 275 / 348+390 / 783 / 1975+2009) is replaced with `region_map.config_for(&key)` (or an equivalent helper); no path under `crates/slicer-runtime/src/` constructs a `ResolvedConfig` by cloning `plan.config` directly.

| `! rg -q '(plan\.config|&plan\.config|plan\.config\.clone\(\))(?!.*ConfigId)' crates/slicer-runtime/src/ && rg -q 'config_for\(&' crates/slicer-runtime/src/`

### AC-9 — `HashablePaintValue` wrapper deleted; `paint_segmentation.rs:117` uses `PaintValue` directly

**Given** AC-7's `Hash for PaintValue` removes the need for the wrapper,
**When** `crates/slicer-core/src/algos/paint_segmentation.rs` is inspected,
**Then** no `struct HashablePaintValue` definition exists anywhere under `crates/slicer-core/src/` and no `use ... HashablePaintValue` statement survives; the previous wrapper call sites use `PaintValue` directly.

| `! rg -q 'HashablePaintValue' crates/slicer-core/src/`

### AC-10 — Behavior preservation: every existing test in the workspace passes byte-identically

**Given** the goal of "behavior-preserving schema scaffold",
**When** the workspace test suite runs and the produced g-code is compared to a pre-packet baseline,
**Then** every test that passed before this packet still passes; specifically, `cargo test -p slicer-runtime --test e2e` returns the same overall pass count as the pre-packet baseline, and a default `pnp_cli slice` run on `resources/regression_wedge.stl` produces byte-identical g-code to the pre-packet output (captured via `sha256sum` in the closure log).

| `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p91-wedge.gcode && sha256sum /tmp/p91-wedge.gcode`

### AC-11 — `cargo test --workspace` passes (final gate for an IR-version-bumping packet)

**Given** the schema bump touches many crates,
**When** the full workspace test suite runs (final gate; dispatched per `CLAUDE.md` §Test Discipline),
**Then** the suite completes with zero new failures vs the pre-packet baseline.

| `cargo test --workspace 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

## Negative Test Cases

### AC-N1 — No production code constructs a `RegionPlan` with `config: ResolvedConfig` (old shape)

**Given** the migration,
**When** `crates/` is grepped for the old shape,
**Then** no source under `crates/*/src/` constructs `RegionPlan { config: ResolvedConfig { ... }, ... }`; every construction routes through the interner (`region_map.intern_config(rc)` or equivalent).

| `! rg -nE 'RegionPlan\s*\{[^}]*config:\s*ResolvedConfig\s*\{' crates/`

### AC-N2 — `cargo xtask build-guests --check` reports clean

**Given** the IR change feeds guest WASM bindgen,
**When** `cargo xtask build-guests` runs (rebuild) followed by `--check`,
**Then** `--check` reports zero `STALE:` entries — every guest's bindgen reflects the new IR shape.

| `cargo xtask build-guests && cargo xtask build-guests --check`

### AC-N3 — `boundary_paint` is GONE everywhere (not just `slicer-ir`)

**Given** the rename in AC-3,
**When** the full workspace is grepped (excluding this packet's directory and the roadmap doc itself, which records the historical name),
**Then** zero source files under `crates/` or `modules/` mention `boundary_paint`. Doc files under `docs/` may transiently retain the name until packet 99 (P5c — Doc updates); the implementation tree must be clean.

| `! rg -n --glob '!.ralph/specs/91_paint-pipeline-schema-scaffolding/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' 'boundary_paint' crates/ modules/`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests && cargo xtask build-guests --check`
4. `cargo test --workspace 2>&1 | tee target/test-output.log` (the schema bump touches enough crates that the workspace gate is required at close)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1a — Schema scaffolding" — primary scope (~80 lines; read directly).
- `docs/02_ir_schemas.md` — current IR shapes; range-read sections describing `SliceIR`, `RegionMapIR`, `RegionKey`, `RegionPlan`, `SlicedRegion`, `ResolvedConfig`, `PaintValue` only. Delegate other sections.
- `docs/05_module_sdk.md` — `BuiltinProducer` constant shape (range-read).
- `crates/slicer-ir/src/slice_ir.rs` — primary edit site. Read in full only when the change touches > 50% of the file; otherwise range-read.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `crates/slicer-ir/src/slice_ir.rs` doc-comments for `RegionKey`, `RegionPlan`, `SlicedRegion`, `RegionMapIR`, `ConfigId`, `PaintValue` — verified via `rg -q 'variant_chain' crates/slicer-ir/src/slice_ir.rs` and `rg -q 'ConfigId' crates/slicer-ir/src/slice_ir.rs`.

`docs/02_ir_schemas.md` is updated by packet 99 (P5c — Doc updates), NOT this packet. The IR source carries authoritative shape; the doc lags by one packet by design (the roadmap defers doc sync to the final packet of the batch). Self-consistent because P1a does not yet *use* the new fields meaningfully — they are scaffolding.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion` / `FuzzySkinPaintedRegion` structure that this packet's `variant_chain` model maps to. Delegate a SUMMARY confirming the field shape (parent region pointer + paint discriminator). Do NOT load the file.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
