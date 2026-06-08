---
status: draft
packet: 92
task_ids: [TASK-242]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 92 — Region-Split Manifest Schema + Host-Filtered Dispatch

## Goal

Add the top-level `[[region_split]]` array section to the module manifest TOML schema (D7), the in-crate `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` registry (D6: `("material", 100)`, `("fuzzy_skin", 200)`, community priorities `>= 1000`), the manifest-load validators (reject duplicate semantics within one manifest, reject `value_type = "scalar"` per D13, reject `priority < 1000` for any semantic name NOT in `CORE_REGION_SPLIT_PRIORITIES`, WARN on tied priorities across manifests), the scheduler-side aggregation that produces a process-wide `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry { priority: u32, value_type: RegionSplitValueType, declaring_modules: Vec<ModuleId> }>` sorted by `(priority, name)` to define the canonical variant-chain order, and the host-filtered dispatch hook in `crates/slicer-runtime/src/layer_executor.rs:494-528` that consults each module's `[[region_split]]` declaration: a module that declares a non-empty `[[region_split]]` set S is invoked only on regions whose `variant_chain` contains at least one `(semantic, _)` pair with `semantic ∈ S`; a module that declares no `[[region_split]]` runs unconditionally (paint-transparent default); a per-layer-region empty-polygon guard skips invocation when `region.polygons.is_empty()` regardless of module declaration. No core module declares `[[region_split]]` in this packet — behavior is preserved end-to-end because no `variant_chain` is yet non-empty (P1c populates it).

## Scope Boundaries

This packet wires the dispatch mechanism and the validator surface but leaves every existing module's manifest untouched. Behavior is preserved because: (a) `aggregated_region_split` is empty in production (no module declares anything yet); (b) every existing module continues to be paint-transparent; (c) no region's `variant_chain` is non-empty until P1c populates them. The packet ships with a synthetic test manifest that exercises the validators and dispatch filter against a fake `[[region_split]]` declaration, so the new code paths have coverage. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) must be `implemented`. Without `RegionKey.variant_chain` the dispatch filter has nothing to inspect.
- Unblocks: P1c (93, RegionMapping cross-product expansion), P3 (95, paint-segmentation port). Core modules begin declaring `[[region_split]]` only in P3.
- Activation blockers: confirmation that packet 91 is `implemented`.

## Acceptance Criteria

### AC-1 — `[[region_split]]` manifest section parses; required fields validated

**Given** the new TOML schema,
**When** `crates/slicer-scheduler/src/manifest.rs` (or equivalent — manifest TOML parser location) is inspected and a test manifest containing `[[region_split]] semantic = "material" priority = 100 value_type = "tool_index"` is parsed,
**Then** the parser succeeds; the resulting in-memory `ManifestEntry` carries a `region_splits: Vec<RegionSplitDeclaration>` field with one entry whose `semantic == "material"`, `priority == 100`, `value_type == RegionSplitValueType::ToolIndex`. Missing required fields (`semantic`, `priority`, `value_type`) cause a structured `ManifestParseError::MissingField` with the field name and the manifest path in the error message.

| `cargo test -p slicer-scheduler region_split_manifest_basic 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [0-9]+ passed; 0 failed'`

### AC-2 — `CORE_REGION_SPLIT_PRIORITIES` registry exists and contains `("material", 100)` + `("fuzzy_skin", 200)`

**Given** the priority registry,
**When** `crates/slicer-schema/src/` is inspected,
**Then** a public constant `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` exists with at least the two core entries `("material", 100)` and `("fuzzy_skin", 200)`; a public constant `COMMUNITY_PRIORITY_FLOOR: u32 = 1000` is defined; both are documented.

| `rg -q 'CORE_REGION_SPLIT_PRIORITIES' crates/slicer-schema/src/ && rg -q '\("material", 100\)' crates/slicer-schema/src/ && rg -q '\("fuzzy_skin", 200\)' crates/slicer-schema/src/ && rg -q 'COMMUNITY_PRIORITY_FLOOR' crates/slicer-schema/src/`

### AC-3 — Validator: duplicate `[[region_split]]` semantic in one manifest is rejected with structured error

**Given** an invalid manifest with two `[[region_split]]` entries naming the same `semantic`,
**When** the manifest loads,
**Then** loading fails with `ManifestParseError::DuplicateRegionSplitSemantic { semantic, manifest_path }`; the error names both line numbers of the duplicate entries in the message.

| `cargo test -p slicer-scheduler region_split_duplicate_semantic_rejected 2>&1 | tee target/test-output.log`

### AC-4 — Validator: `value_type = "scalar"` is rejected at manifest-load time (D13)

**Given** an invalid manifest declaring `value_type = "scalar"` for a `[[region_split]]` entry,
**When** the manifest loads,
**Then** loading fails with `ManifestParseError::ScalarValueTypeNotAllowedInRegionSplit { semantic, manifest_path }`; the error message explicitly references the D13 architectural decision.

| `cargo test -p slicer-scheduler region_split_scalar_rejected 2>&1 | tee target/test-output.log`

### AC-5 — Validator: community semantic with `priority < 1000` is rejected

**Given** an invalid manifest declaring `[[region_split]] semantic = "com.example.foo" priority = 250 ...`,
**When** the manifest loads,
**Then** loading fails with `ManifestParseError::CommunityPriorityBelowFloor { semantic, given_priority, floor: 1000, manifest_path }`. The semantic name is community (not in `CORE_REGION_SPLIT_PRIORITIES`); the priority is below the floor; both facts are surfaced in the error.

| `cargo test -p slicer-scheduler region_split_community_priority_floor 2>&1 | tee target/test-output.log`

### AC-6 — Validator: core semantic with priority mismatch is rejected

**Given** an invalid manifest declaring `[[region_split]] semantic = "material" priority = 100000 ...`,
**When** the manifest loads,
**Then** loading fails with `ManifestParseError::CorePriorityMismatch { semantic, given_priority, expected_priority }`. Core semantics' priorities are fixed by `CORE_REGION_SPLIT_PRIORITIES`; manifests cannot override them.

| `cargo test -p slicer-scheduler region_split_core_priority_mismatch 2>&1 | tee target/test-output.log`

### AC-7 — Cross-manifest aggregation: WARN-level diagnostic on tied priorities across distinct semantics

**Given** two manifests declaring `[[region_split]]` for distinct semantics at the same priority,
**When** the scheduler aggregates them,
**Then** a WARN-level structured diagnostic event is emitted (per `docs/09_progress_events.md` conventions) naming both semantics, both manifest paths, the shared priority, and the lex-tiebreaker order the scheduler chose. The aggregation succeeds (this is a warning, not an error).

| `cargo test -p slicer-scheduler region_split_tied_priority_warn 2>&1 | tee target/test-output.log`

### AC-8 — `aggregated_region_split` BTreeMap is produced and sorted by `(priority, name)`

**Given** two manifests declaring `material` (priority 100) and `fuzzy_skin` (priority 200),
**When** the scheduler aggregates,
**Then** the resulting `aggregated_region_split` BTreeMap, iterated in canonical order (via `iter()` returning sorted-by-key sequence), produces `material` first, `fuzzy_skin` second; a community semantic at priority 1500 appears third. The canonical order is what `RegionKey::variant_chain` and the dispatch filter use as the source of truth.

| `cargo test -p slicer-scheduler region_split_aggregation_canonical_order 2>&1 | tee target/test-output.log`

### AC-9 — Layer-executor dispatch filter routes region-split-declaring modules conditionally

**Given** a synthetic test scenario: one module M_A declares `[[region_split]] semantic = "material"`, one module M_B declares no `[[region_split]]`, and two regions exist on the same layer — region_X with `variant_chain = [("material", ToolIndex(2))]` and region_Y with empty `variant_chain`,
**When** the layer executor dispatches,
**Then** M_A is invoked on region_X (matching variant_chain) and NOT on region_Y (no matching variant); M_B is invoked on BOTH regions (paint-transparent default). The test asserts a recorded invocation log of {(M_A, region_X), (M_B, region_X), (M_B, region_Y)} — three invocations, exact set membership.

| `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 | tee target/test-output.log`

### AC-10 — Empty-polygon guard skips dispatch invocation universally

**Given** a synthetic test scenario: any module M, any region R with `R.polygons.is_empty()`,
**When** the layer executor reaches R,
**Then** M is NOT invoked on R regardless of any `[[region_split]]` declaration; the skip is logged at DEBUG level (one event per skipped invocation) so a future inquiry can audit dispatch decisions.

| `cargo test -p slicer-runtime --test integration empty_polygon_dispatch_guard 2>&1 | tee target/test-output.log`

### AC-11 — Behavior preservation: every existing test passes; no core module declares `[[region_split]]` in this packet

**Given** that no production module changes its manifest in this packet,
**When** the workspace test suite runs,
**Then** every existing test passes; specifically `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode` produces byte-identical g-code to the post-P91 baseline.

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode && sha256sum /tmp/p92-wedge.gcode`

### AC-12 — Guest WASM rebuild clean after the manifest-schema additions

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

### AC-N3 — Invalid TOML with malformed `priority` field produces a structured error pointing at the malformed value

**Given** a manifest with `priority = "not-a-number"` (string where u32 is expected),
**When** loading,
**Then** the error names the field, the actual TOML value, and the expected type; the error variant is `ManifestParseError::TypeMismatch { field, expected, actual }` or equivalent.

| `cargo test -p slicer-scheduler region_split_priority_type_mismatch 2>&1 | tee target/test-output.log`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log` (the manifest + aggregation work lives here)
4. `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log` (dispatch hooks)
5. `cargo xtask build-guests && cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b — Manifest schema + host-filtered dispatch" (~110 lines; read directly).
- `docs/03_wit_and_manifest.md` §"Module Manifest TOML Schema" — current schema shape and validation conventions. Range-read.
- `docs/04_host_scheduler.md` §"Module Dispatch" — current dispatch contract.
- `docs/09_progress_events.md` — structured event conventions for the cross-manifest WARN diagnostic (AC-7).

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
