# Requirements: 92_region-split-manifest-and-dispatch

## Packet Metadata

- Grouped task IDs:
  - `TASK-242` — Manifest `[[region_split]]` schema + priority registry + host-filtered dispatch hook.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b — Manifest schema + host-filtered dispatch"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After packet 91 ships the IR scaffolding for region-splitting, the IR can *represent* a non-empty `variant_chain` but the host doesn't know what to do with one. Three machinery pieces are missing:

1. **Module manifests have no way to opt in to region-splitting.** D7 chose a top-level `[[region_split]]` TOML array section: a module declares which paint semantics it cares about, each with a `priority` (determining variant-chain canonical order) and a `value_type` (`flag` / `tool_index` / `custom_string`; `scalar` is rejected per D13). Without this section, the manifest loader has no place for the metadata and the scheduler has no way to know which modules are paint-aware.
2. **There is no canonical variant-chain order.** D6 chose fixed core priorities (`material = 100`, `fuzzy_skin = 200`) and a community floor of 1000. Without a registered priority registry and a cross-manifest aggregation step, modules can't agree on the order in which their semantics appear inside `variant_chain`, and `RegionKey` equality breaks (two regions with the same paint values in different orders would be treated as distinct).
3. **The layer executor invokes every module on every region.** D9 chose host-filtered dispatch: a module declaring `[[region_split]]` with set S is invoked only on regions whose `variant_chain` matches at least one semantic in S; a module with no `[[region_split]]` is paint-transparent (invoked unconditionally). The executor currently has no notion of this filter.

Side concern: empty-polygon `RegionPlan` entries land in P1c per D15 (emit-unconditionally). Without a universal empty-polygon dispatch guard, every module gets invoked on every empty region — wasted work and a guaranteed source of bugs (modules that don't check `polygons.is_empty()` first). The empty-polygon guard belongs in dispatch, not in each module, so it ships in this packet alongside the host filter.

This packet lands the machinery. No core module declares `[[region_split]]` in this packet — that begins in P3 when `paint-segmentation` is wired. Behavior is preserved because every existing module continues to be invoked on every region, exactly as before.

## In Scope

- Add top-level `[[region_split]]` TOML array section to the module manifest schema.
- Define `RegionSplitDeclaration { semantic: String, priority: u32, value_type: RegionSplitValueType }` and `RegionSplitValueType { Flag, ToolIndex, CustomString }` in `crates/slicer-scheduler/src/` (or the manifest types crate).
- Add `ManifestEntry.region_splits: Vec<RegionSplitDeclaration>` (or equivalent — wherever parsed manifests are stored).
- Add per-manifest validators:
  - duplicate semantic in one manifest → `ManifestParseError::DuplicateRegionSplitSemantic`.
  - `value_type = "scalar"` → `ManifestParseError::ScalarValueTypeNotAllowedInRegionSplit`.
  - community semantic with `priority < 1000` → `ManifestParseError::CommunityPriorityBelowFloor`.
  - core semantic with wrong priority → `ManifestParseError::CorePriorityMismatch`.
  - malformed field types → existing `ManifestParseError::TypeMismatch` (no new variant needed).
- Add `CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)]` and `COMMUNITY_PRIORITY_FLOOR: u32 = 1000` constants in `crates/slicer-schema/src/`.
- Add scheduler-side aggregation: `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry>` produced at startup from all loaded modules' `region_splits`. Each entry carries `{ priority: u32, value_type: RegionSplitValueType, declaring_modules: Vec<ModuleId> }`.
- Cross-manifest WARN diagnostic on tied priorities (different semantic names with equal priority).
- Lex tiebreaker for tied priorities (`(priority, name)` sort).
- Host-filtered dispatch hook at `crates/slicer-runtime/src/layer_executor.rs:494-528` — read each module's `region_splits` (via scheduler-provided metadata) and conditionally invoke based on the dispatched region's `variant_chain`.
- Universal empty-polygon dispatch guard: skip module invocation if `region.polygons.is_empty()`.
- DEBUG-level event when the empty-polygon guard fires (auditable trace).
- Synthetic test manifests under `crates/slicer-scheduler/tests/fixtures/` exercising every validator + the dispatch filter + the empty-polygon guard.

## Out of Scope

- Any core module's manifest gaining a `[[region_split]]` section — P3 (95) territory.
- Populating `variant_chain` on any region — P1c (93).
- Paint-segmentation port — P3.
- 3MF parser changes for community paint channels (deferred per roadmap).
- Doc updates to `docs/03_wit_and_manifest.md` or `docs/04_host_scheduler.md` — P5c (99).
- Any change to `pnp_cli` g-code emission.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1b" (~110 lines; read directly).
- `docs/03_wit_and_manifest.md` §"Module Manifest TOML Schema" — current schema; range-read.
- `docs/04_host_scheduler.md` §"Module Dispatch" — current dispatch shape; range-read.
- `docs/09_progress_events.md` — structured event conventions for AC-7 WARN.
- `crates/slicer-scheduler/src/manifest.rs` (or equivalent file — locate via Glob if not at this path).
- `crates/slicer-runtime/src/layer_executor.rs` — the dispatch hook target (range-read around line 494).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — cross-product expansion source; SUMMARY confirms per-object metadata model (validates our per-manifest declaration architecture).

## Acceptance Summary

- Positive cases: `AC-1` through `AC-12`. Refinements:
  - `aggregated_region_split` must be `BTreeMap` (sorted ordering); `HashMap` would lose determinism critical to AC-8.
  - The host filter's "at least one (semantic, _) match" semantics (AC-9) deliberately allow OR-of-multiple-semantics. A module declaring `[[region_split]] material` AND `[[region_split]] fuzzy_skin` runs on any region with either painted variant — never both AND-ed (per D9).
  - The empty-polygon guard runs BEFORE the host filter (AC-10): even modules with no `[[region_split]]` skip empty regions.
- Negative cases: `AC-N1` (no core module declares yet), `AC-N2` (empty aggregation default), `AC-N3` (priority type-mismatch).
- Cross-packet impact: unblocks P1c, P3.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No new lint warnings | FACT pass/fail |
| `cargo test -p slicer-scheduler region_split 2>&1 \| tee target/test-output.log` | AC-1, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-N2, AC-N3 | FACT pass/fail with per-test breakdown |
| `cargo test -p slicer-runtime --test integration region_split_dispatch_filter 2>&1 \| tee target/test-output.log` | AC-9 — host filter | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration empty_polygon_dispatch_guard 2>&1 \| tee target/test-output.log` | AC-10 — empty-polygon guard | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p92-wedge.gcode && sha256sum /tmp/p92-wedge.gcode` | AC-11 — byte-identical g-code | FACT (single sha256); compare to post-P91 baseline |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-12 — guest WASM clean | FACT pass/fail |
| `! rg -q '\[\[region_split\]\]' modules/core-modules/` | AC-N1 — no core module declares | FACT pass/fail |
| `rg -q 'CORE_REGION_SPLIT_PRIORITIES' crates/slicer-schema/src/ && rg -q 'COMMUNITY_PRIORITY_FLOOR' crates/slicer-schema/src/` | AC-2 — registry exists | FACT pass/fail |

## Step Completion Expectations

- The `CORE_REGION_SPLIT_PRIORITIES` constant must land before the validators (Step 3) — validators reference the registry. Order matters within Step 2.
- The synthetic test manifests (Step 5) must exercise every validator branch (AC-3 through AC-6, AC-N3); skipping a branch leaves a quietly broken validator on packet close.
- The host-filter hook (Step 6) is the most subtle change. It runs INSIDE the existing module-dispatch loop, not as a pre-filter pass — the loop iterates each `ActiveRegion`, and within the loop the filter inspects each module's declaration before invoking. Reordering the filter into a pre-pass would break the per-region iteration assumptions.
- AC-11 byte-identical check (Step 7) confirms behavior preservation: with no core module declaring `[[region_split]]` and every existing region having empty `variant_chain`, the host filter never excludes a module, and the empty-polygon guard only fires for genuinely-empty regions (which is a correctness improvement that should not change emitted g-code — empty-polygon regions emit no g-code).

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is likely > 600 lines. The dispatch hook is in a narrow 35-line window (494-528). Read with `Read offset: 480, limit: 60`; do NOT load the full file.
- `crates/slicer-scheduler/src/manifest.rs` (or equivalent) may be sizeable. Locate the existing manifest-parse function first via `Grep`, then range-read.
- The `serde` + `toml` derives for the new types are straightforward; if `RegionSplitValueType` is an enum, use `#[serde(rename_all = "snake_case")]` so TOML's `value_type = "flag"` deserializes to `RegionSplitValueType::Flag`.
- The four new `ManifestParseError` variants live in the existing error enum (locate via `Grep`); do NOT introduce a new error type if the existing one suffices.
