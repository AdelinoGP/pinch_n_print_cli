# Requirements: 92_region-split-manifest-and-dispatch

## Packet Metadata

- Grouped task IDs:
  - `TASK-242` — Manifest `[[region_split]]` schema + priority registry + host-filtered dispatch hook.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b — Manifest schema + host-filtered dispatch"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After packet 91 ships the IR scaffolding for region-splitting, the IR can *represent* a non-empty `variant_chain` but the host doesn't know what to do with one. Three machinery pieces are missing:

1. **Module manifests have no way to opt in to region-splitting.** D7 chose a top-level `[[region_split]]` TOML array section: a module declares which paint semantics it cares about, each with a `priority` (determining variant-chain canonical order) and a `value_type` (`flag` / `tool_index` / `custom_string`; `scalar` is rejected per D13). Without this section, the manifest loader (`crates/slicer-scheduler/src/manifest.rs`, type `LoadedModule` at line 29) has no place for the metadata and the scheduler has no way to know which modules are paint-aware.
2. **There is no canonical variant-chain order.** D6 chose fixed core priorities (`material = 100`, `fuzzy_skin = 200`) and a community floor of 1000. Without a registered priority registry and a cross-manifest aggregation step, modules can't agree on the order in which their semantics appear inside `variant_chain`, and `RegionKey` equality breaks (two regions with the same paint values in different orders would be treated as distinct).
3. **The layer executor invokes every paint-declaring module on every layer regardless of paint content.** D9 specifies host-filtered dispatch. Current dispatch in `execute_single_layer_inner` (`crates/slicer-runtime/src/layer_executor.rs`) is **per-(module × layer)** — line 394 invokes `runner.run_stage(stage_id, layer, &live_module, input)` once per module per layer, and modules iterate regions internally. The per-layer host filter is wedged into the same per-module loop at line 362, before `instrumentation.on_module_start` so skipped modules are absent from instrumentation and audit records. A module declaring `[[region_split]]` set S should be **skipped on layer L** if no region's `variant_chain` on L matches any semantic in S; a module with no `[[region_split]]` is paint-transparent (always invoked). Granularity is per-(module × layer), NOT per-(module × region) — true per-region host dispatch would require refactoring every module's invocation contract and is out of scope.

Side concern (now descoped): a "universal empty-polygon dispatch guard" was originally planned here, but the codebase has no per-region host invocation site at which it could fire (per-region iteration happens inside each module). The guard's new home is either P93 (P1c — RegionMapping cross-product expansion at IR construction, preferred — overrides D15 partially so empty `RegionPlan` entries are never emitted) or P95 (P3 — paint-segmentation port, if P93 keeps D15 intact). See `.ralph/specs/93_region-mapping-cross-product/design.md` §"Empty-Polygon Filter Decision (from P92 audit)".

This packet lands the manifest + aggregation + per-layer dispatch filter machinery. No core module declares `[[region_split]]` in this packet — that begins in P3 when `paint-segmentation` is wired. Behavior is preserved because every existing module is paint-transparent (filter never excludes anyone) and no region's `variant_chain` is non-empty until P1c populates them.

## In Scope

- Add top-level `[[region_split]]` TOML array section to the module manifest schema.
- Define `RegionSplitDeclaration { semantic: String, priority: u32, value_type: RegionSplitValueType }` and `RegionSplitValueType { Flag, ToolIndex, CustomString }` in `crates/slicer-scheduler/src/manifest.rs` (alongside `LoadedModule`).
- Add `LoadedModule.region_splits: Vec<RegionSplitDeclaration>` (default-empty; new field on the struct at `crates/slicer-scheduler/src/manifest.rs:29`).
- Add per-manifest validators (four new variants on the existing `LoadErrorKind` enum at `crates/slicer-scheduler/src/manifest.rs:450`, reusing the established `LoadError { kind, path, field, message }` shape):
  - duplicate semantic in one manifest → `LoadErrorKind::DuplicateRegionSplitSemantic`.
  - `value_type = "scalar"` → `LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`.
  - community semantic with `priority < 1000` → `LoadErrorKind::CommunityPriorityBelowFloor`.
  - core semantic with wrong priority → `LoadErrorKind::CorePriorityMismatch`.
  - missing required field (`semantic`/`priority`/`value_type`) → existing `LoadErrorKind::Schema` (no new variant; populate `field`).
  - malformed field types (e.g. `priority = "abc"`) → existing `LoadErrorKind::TomlParse` (no new variant; the toml-deserializer message already surfaces field/expected/actual).
- Add `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` and `COMMUNITY_PRIORITY_FLOOR: u32 = 1000` constants in `crates/slicer-schema/src/lib.rs`.
- Add scheduler-side aggregation in a new module `crates/slicer-scheduler/src/region_split.rs`: `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry>` produced at module-load time from all loaded modules' `region_splits`. Each entry carries `{ priority: u32, value_type: RegionSplitValueType, declaring_modules: Vec<ModuleId> }`. The aggregator accepts `&mut Vec<LoadDiagnostic>` and pushes WARN-level diagnostics into it (existing pattern; not the runtime `ProgressEvent` channel).
- Cross-manifest WARN `LoadDiagnostic` on tied priorities (different semantic names with equal priority) — `level: DiagnosticLevel::Warning`.
- Lex tiebreaker for tied priorities (`(priority, name)` sort).
- **Per-layer host-filtered dispatch guard** at `crates/slicer-runtime/src/layer_executor.rs:362` inside `execute_single_layer_inner` (filter call site; the whole guard block is lines 357-364; the per-module loop's `runner.run_stage(...)` call is at line 394). For each `(module, layer)` pair: skip invocation if the module declares a non-empty `[[region_split]]` set S AND no region on the layer has a `variant_chain` containing `(semantic, _)` with `semantic ∈ S`. The per-module descriptor in scope at the filter site is `&CompiledModuleStatic`, which carries the `region_split_semantics: HashSet<String>` propagated from `LoadedModule` at plan-build time (D-92-5).
- Cache `region_splits` as a `HashSet<String>` on `LoadedModule` at module-load to keep filter cost at O(|regions| × |S|) per dispatch decision.
- Synthetic test manifests under `crates/slicer-scheduler/tests/fixtures/region_split_manifests/` exercising every validator.
- Synthetic two-layer integration test under `crates/slicer-runtime/tests/integration/` constructing non-empty `variant_chain` regions (no production code path produces them yet).

## Out of Scope

- Any core module's manifest gaining a `[[region_split]]` section — P3 (95) territory.
- Populating `variant_chain` on any region — P1c (93).
- Paint-segmentation port — P3.
- 3MF parser changes for community paint channels (deferred per roadmap).
- Doc updates to `docs/03_wit_and_manifest.md` or `docs/04_host_scheduler.md` — P5c (99).
- Any change to `pnp_cli` g-code emission.
- **Universal empty-polygon dispatch guard** (was AC-10) — no per-region host invocation site exists; deferred to P93 (preferred: filter at RegionMapping cross-product step, overriding D15 partially) or P95 (fallback). See `.ralph/specs/93_region-mapping-cross-product/design.md` §"Empty-Polygon Filter Decision (from P92 audit)".
- **Per-region host dispatch refactor** — refactoring `runner.run_stage` from per-(module × layer) to per-(module × layer × region) is materially larger than P92's M budget; deferred to a candidate follow-up packet decided at P95 closure if needed.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b" (~110 lines; read directly). Carries the empty-polygon-guard descope note added at packet refinement.
- `docs/03_wit_and_manifest.md` §"Module Manifest TOML Schema" — current schema; range-read.
- `docs/04_host_scheduler.md` §"Module Dispatch" — current dispatch shape; range-read.
- `crates/slicer-scheduler/src/manifest.rs` — confirmed location of `LoadedModule` (line 29), `DiagnosticLevel` (line 413), `LoadDiagnostic` (line 424), `LoadError` (line 437), `LoadErrorKind` (line 450), and `ingest_manifest` (line 532). The AC-7 WARN plugs into the `LoadDiagnostic { level: DiagnosticLevel::Warning, ... }` pattern used at `manifest.rs:493`, `validation.rs:333,342`, `execution_plan.rs:223`. The runtime `docs/09_progress_events.md` channel is NOT used here.
- `crates/slicer-runtime/src/layer_executor.rs` — the per-layer dispatch site (range-read lines 355-400; filter call site at line 362, before `on_module_start`; the per-module loop's `runner.run_stage(...)` call is at line 394).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — cross-product expansion source; SUMMARY confirms per-object metadata model (validates our per-manifest declaration architecture).

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11` (the original AC-10 empty-polygon guard was removed at refinement; the former AC-11 byte-identical baseline was renumbered to AC-10, and the former AC-12 guest-WASM rebuild was renumbered to AC-11 to close the gap). Refinements:
  - `aggregated_region_split` must be `BTreeMap` (sorted ordering); `HashMap` would lose determinism critical to AC-8.
  - The host filter's "at least one (semantic, _) match" semantics (AC-9) deliberately allow OR-of-multiple-semantics at the LAYER level. A module declaring `[[region_split]] material` AND `[[region_split]] fuzzy_skin` runs on any layer containing at least one region with either painted variant — never both AND-ed (per D9).
  - Granularity is per-(module × layer). Per-region filtering remains module-internal (modules iterate regions themselves once invoked on a layer).
  - AC-10 byte-identical baseline relies on Step 0 having captured `P91_BASELINE_SHA=<hex>` into `closure-log.md`; the shell command in AC-10 compares the new SHA against that file.
- Negative cases: `AC-N1` (no core module declares yet), `AC-N2` (empty aggregation default), `AC-N3` (priority type-mismatch handled by reused `LoadErrorKind::TomlParse`).
- Cross-packet impact: unblocks P1c (93), P3 (95). Hands off empty-polygon guard to P93/P95 (P93 design.md has the open question added at refinement).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No new lint warnings | FACT pass/fail |
| `cargo test -p slicer-scheduler region_split 2>&1 \| tee target/test-output.log` | AC-1, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-N2, AC-N3 | FACT pass/fail with per-test breakdown |
| `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 \| tee target/test-output.log` | AC-9 — per-layer host filter | FACT pass/fail |
| Baseline-compare shell command (see AC-10) | AC-10 — byte-identical g-code vs P91 baseline | FACT exit 0 on match |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-11 — guest WASM clean | FACT pass/fail |
| `! rg -q '\[\[region_split\]\]' modules/core-modules/` | AC-N1 — no core module declares | FACT pass/fail |
| `rg -q 'CORE_REGION_SPLIT_PRIORITIES' crates/slicer-schema/src/ && rg -q 'COMMUNITY_PRIORITY_FLOOR' crates/slicer-schema/src/` | AC-2 — registry exists | FACT pass/fail |

## Step Completion Expectations

- The `CORE_REGION_SPLIT_PRIORITIES` constant must land before the validators (Step 3) — validators reference the registry. Order matters within Step 2.
- The synthetic test manifests (Step 5) must exercise every validator branch (AC-3 through AC-6, AC-N3); skipping a branch leaves a quietly broken validator on packet close.
- The host-filter guard (Step 6) is a single insertion at `execute_single_layer_inner` line 362 (the whole guard block is lines 357-364), placed BEFORE `instrumentation.on_module_start` and the per-module loop's `runner.run_stage(...)` call at line 394. It is per-(module × layer) by construction (one `runner.run_stage` call per pair). Do NOT attempt to make it per-region — the host has no per-region invocation site, and per-region filtering would require refactoring `run_stage` (out of scope per §Out of Scope).
- AC-10 byte-identical check (Step 7) confirms behavior preservation: with no core module declaring `[[region_split]]` and every existing region having empty `variant_chain`, the per-layer filter never excludes a module. Step 0 captures the baseline SHA into `closure-log.md` so Step 7's command can compare.

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is 1323 lines (post-edit). The filter call site is at line 362 inside the per-module loop; the `runner.run_stage` call is at line 394. Read with `Read offset: 355, limit: 50`; do NOT load the full file.
- `crates/slicer-scheduler/src/manifest.rs` is moderately sized. Confirmed targets: `LoadedModule` at line 29, `DiagnosticLevel` at 413, `LoadDiagnostic` at 424, `LoadError` at 437, `LoadErrorKind` at 450, `ingest_manifest` at 532. Range-read each as needed.
- The `serde` + `toml` derives for the new types are straightforward; for `RegionSplitValueType` use `#[serde(rename_all = "snake_case")]` so TOML's `value_type = "flag"` deserializes to `RegionSplitValueType::Flag`.
- The four new `LoadErrorKind` variants live in the existing enum at `manifest.rs:450`. Do NOT introduce a separate error type. Reuse `LoadErrorKind::Schema` for missing-field and `LoadErrorKind::TomlParse` for type-mismatch (do NOT add `MissingField` or `TypeMismatch` variants — the existing surface already covers them).
