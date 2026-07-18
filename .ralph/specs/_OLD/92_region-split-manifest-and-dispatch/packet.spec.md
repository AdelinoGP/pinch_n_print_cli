---
status: implemented
packet: 92
task_ids: [TASK-242]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 92 — Region-Split Manifest Schema + Host-Filtered Dispatch

## Goal

Add the top-level `[[region_split]]` array section to the module manifest TOML schema (D7), the in-crate `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` registry (D6: `("material", 100)`, `("fuzzy_skin", 200)`, community priorities `>= 1000`), the manifest-load validators (reject duplicate semantics within one manifest, reject `value_type = "scalar"` per D13, reject `priority < 1000` for any semantic name NOT in `CORE_REGION_SPLIT_PRIORITIES`, WARN on tied priorities across manifests), the scheduler-side aggregation that produces a process-wide `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry { priority: u32, value_type: RegionSplitValueType, declaring_modules: Vec<ModuleId> }>` sorted by `(priority, name)` to define the canonical variant-chain order, and the host-filtered **per-layer** dispatch guard at `crates/slicer-runtime/src/layer_executor.rs:362` (filter call site; the whole guard block is lines 357-364) inside `execute_single_layer_inner` — placed before `instrumentation.on_module_start` and the existing `runner.run_stage(&stage.stage_id, layer, &live_module, input)` call at line 394 so skipped modules are absent from instrumentation and audit records that consults each module's `[[region_split]]` declaration: a module declaring a non-empty `[[region_split]]` set S is **skipped on a layer L** if no region in `L.regions` has a `variant_chain` containing `(semantic, _)` with `semantic ∈ S`; a module that declares no `[[region_split]]` runs unconditionally on every layer (paint-transparent default). Filter granularity is per-(module × layer), NOT per-(module × region) — per-region filtering remains internal to each module, unchanged from today. No core module declares `[[region_split]]` in this packet — behavior is preserved end-to-end because no `variant_chain` is yet non-empty (P1c populates it).

## Scope Boundaries

This packet wires the per-layer dispatch filter and the validator surface but leaves every existing module's manifest untouched. Behavior is preserved because: (a) `aggregated_region_split` is empty in production (no module declares anything yet); (b) every existing module continues to be paint-transparent; (c) no region's `variant_chain` is non-empty until P1c populates them. The packet ships with a synthetic test manifest that exercises the validators and dispatch filter against a fake `[[region_split]]` declaration, so the new code paths have coverage. **Descoped from this packet:** the universal empty-polygon dispatch guard (originally AC-10). The host has no per-region invocation site to hang the guard on; per-region iteration is module-internal. The guard's new home is either P93 (P1c — RegionMapping cross-product expansion at IR construction time, preferred) or P95 (P3 — paint-segmentation port, if P93 keeps D15's "emit empty unconditionally" invariant). See `requirements.md` §Out of Scope and the roadmap §P1b descope note. **Also descoped:** per-region host dispatch refactor (out of scope; deferred to a future packet decided at P95 closure if needed). Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) must be `implemented`. Without `RegionKey.variant_chain` the dispatch filter has nothing to inspect.
- Unblocks: P1c (93, RegionMapping cross-product expansion), P3 (95, paint-segmentation port). Core modules begin declaring `[[region_split]]` only in P3.
- Activation blockers: confirmation that packet 91 is `implemented`.

## Acceptance Criteria

### AC-1 — `[[region_split]]` manifest section parses; required fields validated

**Given** the new TOML schema,
**When** `crates/slicer-scheduler/src/manifest.rs` (real location of `ingest_manifest`/`LoadedModule`) is inspected and a test manifest containing `[[region_split]] semantic = "material" priority = 100 value_type = "tool_index"` is parsed,
**Then** the parser succeeds; the resulting in-memory `LoadedModule` carries a `region_splits: Vec<RegionSplitDeclaration>` field with one entry whose `semantic == "material"`, `priority == 100`, `value_type == RegionSplitValueType::ToolIndex`. Missing required fields (`semantic`, `priority`, `value_type`) cause a `LoadError` whose `kind == LoadErrorKind::Schema` (reused; no new variant) with `field` set to the missing field name and `path` set to the manifest path.

| `cargo test -p slicer-scheduler region_split_manifest_basic 2>&1 | tee target/test-output.log`

### AC-2 — `CORE_REGION_SPLIT_PRIORITIES` registry exists and contains `("material", 100)` + `("fuzzy_skin", 200)`

**Given** the priority registry,
**When** `crates/slicer-schema/src/` is inspected,
**Then** a public constant `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` exists with at least the two core entries `("material", 100)` and `("fuzzy_skin", 200)`; a public constant `COMMUNITY_PRIORITY_FLOOR: u32 = 1000` is defined; both are documented.

| `rg -q 'CORE_REGION_SPLIT_PRIORITIES' crates/slicer-schema/src/ && rg -q '\("material", 100\)' crates/slicer-schema/src/ && rg -q '\("fuzzy_skin", 200\)' crates/slicer-schema/src/ && rg -q 'COMMUNITY_PRIORITY_FLOOR' crates/slicer-schema/src/`

### AC-3 — Validator: duplicate `[[region_split]]` semantic in one manifest is rejected with structured error

**Given** an invalid manifest with two `[[region_split]]` entries naming the same `semantic`,
**When** the manifest loads,
**Then** loading fails with `LoadErrorKind::DuplicateRegionSplitSemantic { semantic, manifest_path }`; the error names both line numbers of the duplicate entries in the message.

| `cargo test -p slicer-scheduler region_split_duplicate_semantic_rejected 2>&1 | tee target/test-output.log`

### AC-4 — Validator: `value_type = "scalar"` is rejected at manifest-load time (D13)

**Given** an invalid manifest declaring `value_type = "scalar"` for a `[[region_split]]` entry,
**When** the manifest loads,
**Then** loading fails with `LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit { semantic, manifest_path }`; the error message explicitly references the D13 architectural decision.

| `cargo test -p slicer-scheduler region_split_scalar_rejected 2>&1 | tee target/test-output.log`

### AC-5 — Validator: community semantic with `priority < 1000` is rejected

**Given** an invalid manifest declaring `[[region_split]] semantic = "com.example.foo" priority = 250 ...`,
**When** the manifest loads,
**Then** loading fails with `LoadErrorKind::CommunityPriorityBelowFloor { semantic, given_priority, floor: 1000, manifest_path }`. The semantic name is community (not in `CORE_REGION_SPLIT_PRIORITIES`); the priority is below the floor; both facts are surfaced in the error.

| `cargo test -p slicer-scheduler region_split_community_priority_floor 2>&1 | tee target/test-output.log`

### AC-6 — Validator: core semantic with priority mismatch is rejected

**Given** an invalid manifest declaring `[[region_split]] semantic = "material" priority = 100000 ...`,
**When** the manifest loads,
**Then** loading fails with `LoadErrorKind::CorePriorityMismatch { semantic, given_priority, expected_priority }`. Core semantics' priorities are fixed by `CORE_REGION_SPLIT_PRIORITIES`; manifests cannot override them.

| `cargo test -p slicer-scheduler region_split_core_priority_mismatch 2>&1 | tee target/test-output.log`

### AC-7 — Cross-manifest aggregation: WARN-level `LoadDiagnostic` on tied priorities across distinct semantics

**Given** two manifests declaring `[[region_split]]` for distinct semantics at the same priority,
**When** the scheduler aggregator runs and a caller-provided `&mut Vec<LoadDiagnostic>` is in scope,
**Then** a `LoadDiagnostic { level: DiagnosticLevel::Warning, path, field, message }` is pushed onto that vec following the existing pattern (`crates/slicer-scheduler/src/manifest.rs:493`, `validation.rs:333,342`, `execution_plan.rs:223`). The message names both semantics, both manifest paths, the shared priority, and the lex-tiebreaker order the scheduler chose. The aggregation succeeds (this is a warning, not an error). This is the scheduler load-time diagnostic channel — NOT the runtime `ProgressEvent` channel from `docs/09_progress_events.md`.

| `cargo test -p slicer-scheduler region_split_tied_priority_warn 2>&1 | tee target/test-output.log`

### AC-8 — `aggregated_region_split` BTreeMap is produced and sorted by `(priority, name)`

**Given** two manifests declaring `material` (priority 100) and `fuzzy_skin` (priority 200),
**When** the scheduler aggregates,
**Then** the resulting `aggregated_region_split` BTreeMap, iterated in canonical order (via `iter()` returning sorted-by-key sequence), produces `material` first, `fuzzy_skin` second; a community semantic at priority 1500 appears third. The canonical order is what `RegionKey::variant_chain` and the dispatch filter use as the source of truth.

| `cargo test -p slicer-scheduler region_split_aggregation_canonical_order 2>&1 | tee target/test-output.log`

### AC-9 — Per-layer dispatch filter routes region-split-declaring modules conditionally

**Given** the per-layer filter predicate `pub fn module_invocation_allowed_on_layer(declared: &HashSet<String>, slice: Option<&SliceIR>) -> bool` exposed by `slicer_runtime::layer_executor` and a synthetic test matrix:
- Modules: M_A with declared semantics `{"material"}`; M_B with declared semantics `{}` (paint-transparent).
- Layers (synthetic `SliceIR`s): Layer_1 with two regions — region_X carrying `variant_chain = [("material".into(), PaintValue::ToolIndex(2))]` and region_Y with empty `variant_chain`; Layer_2 with only region_Y.

**When** the predicate is evaluated for every `(module, layer)` cell of the (declared × layer) matrix,
**Then** it returns `true` for `(M_A, Layer_1)`, `(M_B, Layer_1)`, and `(M_B, Layer_2)`, and returns `false` for `(M_A, Layer_2)` — the allowed set is exactly `{(M_A, Layer_1), (M_B, Layer_1), (M_B, Layer_2)}`. A separate edge-case test asserts conservative-allow when `slice == None`. The test fixture constructs non-empty `variant_chain` programmatically because no production code path populates `variant_chain` today (P1c / packet 93 owns population). The dispatch-loop wiring that consults this predicate — the `if !module_invocation_allowed_on_layer(...) { continue; }` guard at `crates/slicer-runtime/src/layer_executor.rs:362`, immediately before `instrumentation.on_module_start` (and well before the `runner.run_stage(...)` call at line 394) — is verified by code inspection rather than by driving `execute_single_layer_inner` through a mock `LayerStageRunner`; see D-92-6 in §Deviations for the trade-off.

| `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`

### AC-10 — Behavior preservation: byte-identical g-code against the P91 baseline

**Given** that no production module changes its manifest in this packet, that `aggregated_region_split` is empty (AC-N1), and that no region's `variant_chain` is non-empty in production,
**When** `pnp_cli slice` runs against the regression-wedge fixture,
**Then** the produced g-code is byte-identical to the post-P91 baseline captured in Step 0 (recorded as `P91_BASELINE_SHA=<hex>` in `.ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md`). The comparison shell command exits 0 only on match.

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode && test "$(sha256sum /tmp/p92-wedge.gcode | awk '{print $1}')" = "$(grep -oE 'P91_BASELINE_SHA=[a-f0-9]+' .ralph/specs/92_region-split-manifest-and-dispatch/closure-log.md | head -1 | cut -d= -f2)"`

### AC-11 — Guest WASM rebuild clean after the manifest-schema additions

**Given** the manifest TOML grammar widens (a new optional top-level array),
**When** `cargo xtask build-guests` runs (rebuild) followed by `--check`,
**Then** `--check` reports zero `STALE:` entries.

| `cargo xtask build-guests && cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — No core module's `<name>.toml` manifest contains a `[[region_split]]` section in this packet

**Given** the no-behavior-change invariant,
**When** every `modules/core-modules/*/<name>.toml` is grepped,
**Then** none contains a `[[region_split]]` header. (Core modules begin declaring `[[region_split]]` only in P3.)

| `! rg -q '\[\[region_split\]\]' modules/core-modules/`

### AC-N2 — `aggregated_region_split` is empty when no module declares `[[region_split]]`

**Given** the default core-modules directory,
**When** the scheduler aggregates,
**Then** `aggregated_region_split.is_empty()` returns true; the canonical variant-chain order is the empty sequence.

| `cargo test -p slicer-scheduler region_split_aggregation_empty_default 2>&1 | tee target/test-output.log`

### AC-N3 — Invalid TOML with malformed `priority` field produces a structured `LoadErrorKind::TomlParse` error

**Given** a manifest with `priority = "not-a-number"` (string where u32 is expected),
**When** loading,
**Then** the error is a `LoadError` whose `kind == LoadErrorKind::TomlParse` (existing variant; no new variant added). The toml-deserializer's structured message — already surfaced through `LoadErrorKind::TomlParse` today — names the field, the actual TOML value, and the expected type. The error carries the manifest path.

| `cargo test -p slicer-scheduler region_split_priority_type_mismatch 2>&1 | tee target/test-output.log`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log` (the manifest + aggregation work lives here)
4. `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log` (the per-layer dispatch filter)
5. AC-10 baseline-compare command (see AC-10) — must exit 0
6. `cargo xtask build-guests && cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b — Manifest schema + host-filtered dispatch" (~110 lines; read directly). The P1b section also carries the empty-polygon-guard descope note from this packet's audit.
- `docs/03_wit_and_manifest.md` §"Module Manifest TOML Schema" — current schema shape and validation conventions. Range-read.
- `docs/04_host_scheduler.md` §"Module Dispatch" — current dispatch contract.
- `crates/slicer-scheduler/src/manifest.rs:413-450` — `DiagnosticLevel` / `LoadDiagnostic` / `LoadError`+`LoadErrorKind` definitions; the WARN channel for AC-7 plugs into this surface. (No `docs/09_progress_events.md` reference: that channel is for runtime slice events, not manifest-load diagnostics.)

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `crates/slicer-scheduler/src/manifest.rs` doc-comments for the new `RegionSplitDeclaration` struct and validation errors — `rg -q 'RegionSplitDeclaration' crates/slicer-scheduler/src/`.
- `crates/slicer-schema/src/lib.rs` (or equivalent) doc-comments for `CORE_REGION_SPLIT_PRIORITIES` and `COMMUNITY_PRIORITY_FLOOR` — `rg -q 'CORE_REGION_SPLIT_PRIORITIES' crates/slicer-schema/src/`.

`docs/03_wit_and_manifest.md` and `docs/04_host_scheduler.md` updates are deferred to packet 99 (P5c — Doc updates). Acceptable because no consumer of those docs needs the new section content until a core module declares `[[region_split]]` (P3 and later).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — the cross-product expansion `PrintApply::apply` reads `(painting_extruders × volume_regions)` from per-object metadata. Delegate a SUMMARY to confirm OrcaSlicer's expansion uses object-local metadata, not a global registry — which validates our per-manifest declaration approach. Do NOT load.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

Recorded during implementation (spec-audit-session, 2026-06-08). All LOW severity; `closure-log.md` has fuller rationale.

- D-92-1 [design.md §Code Change Surface, fixture list] — Specified: `tied_priorities/manifest_a.toml`, `tied_priorities/manifest_b.toml` | Implemented: `tests/fixtures/region_split_manifests/aggregation/{tied_alpha.toml, tied_beta.toml}` plus `a/b/c.toml` for canonical-order tests | Reason: W4 grouped aggregation fixtures; test intent fully covered.
- D-92-2 [design.md §Code Change Surface, dispatch hook pseudocode] — Specified: `module_invocation_allowed_on_layer(module: &LoadedModule, chains: impl Iterator<...>) -> bool` | Implemented: `module_invocation_allowed_on_layer(declared: &HashSet<String>, slice: Option<&SliceIR>) -> bool` at `layer_executor.rs:1326` | Reason: takes pre-cached HashSet directly; `Option<&SliceIR>` cleanly handles conservative-allow when no slice IR is available.
- D-92-3 [pre-audit design.md / requirements.md / packet.spec.md — dispatch insertion line] — Specified: filter inserted "immediately before line 385's `runner.run_stage(...)`" | Implemented: filter inserted at `layer_executor.rs:362`, BEFORE `on_module_start`; real `run_stage` is now at line 394 | Reason: skipped modules absent from instrumentation/audit log; in-code comment records intent. Improvement. Packet docs updated post-audit to reference the corrected line numbers; the deviation captures the pre-audit gap.
- D-92-4 [implementation-plan.md §Step 3 / §Step 9] — Specified: no `#![allow]` attributes mentioned | Implemented: `#![allow(clippy::result_large_err)]` at `manifest.rs:7` | Reason: the 4 new String-bearing `LoadErrorKind` variants pushed `LoadError` past clippy's 128-byte threshold; allow documented in code; boxing rejected as out-of-scope.
- D-92-5 [pre-audit design.md §Locked Assumptions and Invariants — "no separate ModuleMetadata"] — Specified: the runtime descriptor at the dispatch site IS `&LoadedModule` | Implemented: the runtime descriptor is `&CompiledModuleStatic` (in `execution_plan.rs`), extended with `region_split_semantics: HashSet<String>` propagated from `LoadedModule.region_split_semantics` at plan-build (line 741) | Reason: documentation gap, not a behavioral gap; the plan-build step is the materialization surface design didn't name.
- D-92-6 [pre-audit packet.spec.md AC-9 acceptance language] — Specified: "asserts a recorded invocation log of {(M_A, region_X), (M_B, region_X), (M_B, region_Y)}", implying a real `LayerStageRunner` invocation record. | Implemented: 4 `#[test]` functions in `tests/integration/region_split_dispatch_filter.rs` evaluate `module_invocation_allowed_on_layer` directly against the (declared × layer) matrix; the dispatch wiring at `layer_executor.rs:362` is verified by code inspection only. AC-9 has been rewritten post-audit to match the predicate-against-matrix approach. | Reason: avoids mock-runner infrastructure cost; helper is exhaustively unit-tested (4 cases including None-slice edge); call-site is a single guarded `continue` whose correctness is reviewable at a glance. Mock-runner integration test deferred indefinitely per user direction (cost not justified for 4-line regression surface).
