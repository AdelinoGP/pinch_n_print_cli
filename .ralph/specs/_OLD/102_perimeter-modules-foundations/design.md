# Design: 102_perimeter-modules-foundations

## Controlling Code Paths

- Primary code path: `slicer_core::perimeter_utils` (new) becomes the single source for paint-propagation, seam-candidate, and point-conversion helpers. Both perimeter modules' `run_perimeters` migrates to `use slicer_core::perimeter_utils::{…}` and a new `?`-propagating dispatch through `PerimeterOutputBuilder`.
- Neighboring tests / fixtures: `modules/core-modules/{classic,arachne}-perimeters/tests/boundary_paint_tdd.rs` (existing — must stay green); `crates/slicer-core/tests/perimeter_utils_three_tool_boundary_tdd.rs` (new); `crates/slicer-ir/tests/material_boundary_widening_tdd.rs` (new); `crates/slicer-runtime/tests/contract/per_layer_config_override_tdd.rs` (new); `crates/slicer-runtime/tests/contract/perimeter_builder_capacity_error_tdd.rs` (new); `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` (new).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Schema-version contract: `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps `4.1.0 → 4.2.0` (minor, additive). The migration adapter MUST deserialize the pre-bump `MaterialBoundary { adjacent_tool: u32 }` shape into a single-element `Vec<MaterialBoundarySegment>` (with `near_tool: None`, `far_tool: Some(adjacent_tool)`, and `point_range: 0..1`) so committed test fixtures stay parseable.
- WIT type identity: the `wall-boundary-type` variant must match across `crates/slicer-schema/wit/deps/ir-types.wit` (canonical), the host `bindgen!` consumers, and the guest macro inputs (`#[slicer_module]` via `slicer-macros`). Per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass before declaring Step 2 done.
- Both perimeter modules' `_paint: &PaintRegionLayerView` parameter remains semantically passive in this packet (per T-019). The decision is recorded as "the consumer for paint regions outside `segment_annotations` is Phase 2 work"; the doc-comment must spell this out so the next reader does not believe the unused parameter is an accident.

## Code Change Surface

- Selected approach: extract the seven helpers + `BASE_SPEED` into a single new file `crates/slicer-core/src/perimeter_utils.rs` (per D-14 / roadmap T-010, hosting in `slicer-core` rather than a new crate because `slicer-core` is already the per-layer geometry home per `docs/13` §Out of Scope and a new crate adds Cargo / WIT dependency churn). **Note:** D-14's original `slicer-helpers` placement decision has been reversed by the roadmap-wide `D-ROADMAP-CRATE-PLACEMENT` correction (per `docs/13_slicer_helpers_crate.md` §Out of Scope); `perimeter_utils.rs` lands in `slicer-core`, not `slicer-helpers`, as recorded in §Locked Assumptions below. Both modules `use slicer_core::perimeter_utils::*` at the module head; the local `fn` definitions are deleted line-for-line. `MaterialBoundary` widens via a Rust-side `Vec<MaterialBoundarySegment>` and a WIT-side `list<material-boundary-segment>` variant payload, both schema-bumped together. `?`-propagation sweeps both `lib.rs` files for the literal pattern `let _ = output\.` and rewrites each call site. The `_config` parameter wiring uses the existing `ConfigView::get*` API; no SDK trait change is needed (the parameter is already typed `&ConfigView`, just unused).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/perimeter_utils.rs` — new module with exported helpers.
  - `crates/slicer-core/src/lib.rs` — `pub mod perimeter_utils;` line added.
  - `crates/slicer-ir/src/slice_ir.rs` — `WallBoundaryType::MaterialBoundary` widened; new `MaterialBoundarySegment` struct; `CURRENT_SLICE_IR_SCHEMA_VERSION` bumped to 4.2.0; `serde` migration adapter.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `material-boundary-segment` record + updated `wall-boundary-type` variant.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — delete duplicated helpers, add `use`, propagate `Result`s via `?`, read `_config` for per-layer overrides, document `_paint` disuse.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — same changes as classic.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — manifest defaults aligned with code fallbacks.
  - `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`, `docs/15_config_keys_reference.md` — per Doc Impact Statement.
- Rejected alternatives that were considered and why they were not chosen:
  - Standalone `slicer-perimeter-utils` crate — adds Cargo manifest, dependency wiring, and WIT-side considerations for two helpers that will never have non-perimeter consumers. Rejected: scope creep.
  - In-place dedup via a shared `pub mod` inside one module that the other imports — couples two sibling guests, complicates the WIT-build dependency graph. Rejected: violates module-independence assumption.
  - `MaterialBoundary::MultiSegment { segments: Vec<…> }` as a separate variant (keeping the single-tool variant) — bloats the IR with a backward-compat enum branch that future code has to match on. Rejected: pure widening with a migration adapter is cleaner.

## Files in Scope (read + edit)

Primary edit surface exceeds the 3-file target because the packet bundles 10 roadmap tasks per the user's "as few packets as possible" directive. The four primary files are listed first; each additional file is justified.

- `crates/slicer-core/src/perimeter_utils.rs` — role: new shared module; expected change: create file with ~170 LOC of helper definitions moved from the two perimeter modules.
- `crates/slicer-ir/src/slice_ir.rs` — role: IR widening; expected change: replace `MaterialBoundary { adjacent_tool: u32 }` with `MaterialBoundary { segments: Vec<MaterialBoundarySegment> }`, add `MaterialBoundarySegment` struct, bump schema version, add `serde` migration adapter (~40 LOC delta).
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: consumer migration + Result propagation + per-layer config; expected change: ~100 LOC removed (duplicated helpers), ~10 LOC changed (`use` import, `?` propagation, `_config` reads), ~5 LOC doc-comment for `_paint` disuse.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: same as classic; expected change: mirror of the classic change.
- `crates/slicer-core/src/lib.rs` — role: module declaration; expected change: 1-line `pub mod perimeter_utils;` addition. Justified because it's a trivial extension required for AC-1.
- `crates/slicer-schema/wit/deps/ir-types.wit` — role: WIT-side mirror of the IR widening; expected change: ~10 LOC for the new record + variant payload update. Justified because the WIT type-identity rule requires host and guest schemas to match (per CLAUDE.md).
- `modules/core-modules/{classic,arachne}-perimeters/*.toml` — role: manifest default reconcile; expected change: 3 lines per file (default values). Justified as small mechanical edit.

## Read-Only Context

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read §"Phase 1 — Cross-cutting foundations" (lines ~190–220) and §"Open decision points" rows D-13 / D-14 only — purpose: confirm task scope and IR-widening shape.
- `docs/02_ir_schemas.md` — delegate SUMMARY for the `WallBoundaryType` definition only — purpose: confirm canonical field naming and variant ordering.
- `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist" only (≈ 30 lines) — purpose: comply with type-identity-across-boundaries rule.
- `docs/05_module_sdk.md` — delegate SUMMARY for §"LayerModule trait" + §"PerimeterOutputBuilder" — purpose: confirm `run_perimeters` parameter shape and the existing builder API.
- `CLAUDE.md` — read §"Guest WASM Staleness" and §"WIT/Type Changes Checklist" — purpose: comply with the rebuild and type-identity gates.
- `modules/core-modules/{classic,arachne}-perimeters/tests/boundary_paint_tdd.rs` — read to understand what regression coverage exists; do not edit — purpose: ensure the extraction doesn't break these tests.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, `crates/*/wit-guest/Cargo.lock`, generated bindgen output — never load.
- Vendored deps under `vendor/` or any `deps/` path — never load.
- `crates/slicer-runtime/src/region_partition.rs` — out of scope; this packet does not touch the infill-fill-partition host hook (already landed via Phase 2.0/2.1).
- All other crates not listed in §Files in Scope (e.g., `slicer-core`, `slicer-scheduler`, `slicer-wasm-host` host-side internals, infill modules, support modules, seam-placer) — delegate any fact-check.
- Other `.ralph/specs/<packet>/` directories (P92–P99) — only the `closure-log.md` of P96 is referenced by this packet's preconditions; delegate a FACT if needed.

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace --all-targets`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate compile after Step 1 helper migration.
- "Run `cargo test -p slicer-ir --test material_boundary_widening_tdd`; return FACT (pass/fail) + assertion-line if fail" — purpose: validate Step 2 IR widening.
- "Find all match arms or constructors of `WallBoundaryType::MaterialBoundary` across the workspace; return LOCATIONS (≤ 20 entries)" — purpose: confirm Step 2 covers every call site (including `seam-placer`, `overhang-classifier-default`, GCodeEmit if applicable).
- "Find all occurrences of `let _ = output\.` in `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs`; return LOCATIONS" — purpose: enumerate Step 4 rewrite targets.
- "Summarize `docs/05_module_sdk.md` §'PerimeterOutputBuilder' for the failure-mode contract; return SUMMARY ≤ 200 words" — purpose: confirm Step 4 documentation matches the established failure-mode language.
- "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)" — purpose: gate Step 2 closure on guest-WASM coherence after WIT change.
- "Run `cargo test -p slicer-runtime --test contract per_layer_config_override_tdd`; return FACT (pass/fail)" — purpose: validate Step 3 per-layer config plumbing.

## Data and Contract Notes

- IR or manifest contracts touched: `WallBoundaryType` variant payload widens. Backward-compatible via `#[serde(default)]` migration adapter. Schema version bumps additively. Test fixtures committed before this packet stay parseable; new fixtures must use the new shape.
- WIT boundary considerations: `wall-boundary-type` variant payload changes from a single `u32` to `list<material-boundary-segment>`. The `material-boundary-segment` record is new. Both must be declared in `crates/slicer-schema/wit/deps/ir-types.wit` (the canonical single source — there is no inline copy per CLAUDE.md).
- Determinism or scheduler constraints: none. The shared utils' helpers are pure functions; the dispatch order through `run_perimeters` is unchanged.
- `PerimeterOutputBuilder` failure-mode contract is newly documented: callers MUST propagate `?`. Capacity / contract-violation errors must surface as `ModuleError` rather than be silently discarded. AC-N1 enforces this with a mock-builder fixture.

## Locked Assumptions and Invariants

- The two perimeter modules remain sibling-independent — neither imports the other; both consume the shared utils from `slicer-core`.
- `perimeter_utils.rs` placed in `slicer-core` per docs/13 §Out of Scope (per-layer geometry operations belong in slicer-core, not slicer-helpers). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT` matching the P103 pattern.
- The shared utils' API is **pure** (no I/O, no logging, no state). This invariant is preserved so the helpers can be called from both guest WASM contexts without host-services dependency.
- `WallBoundaryType::MaterialBoundary` semantics: every boundary segment names exactly one transition between two tools (`near_tool` → `far_tool`); polygons with N transitions emit N segments in clockwise order matching the polygon's contour winding.
- `BASE_SPEED = 50.0` (mm/s) remains the outer-wall normalisation reference. Bumped by mutual agreement only — neither manifest defaults nor code fallbacks may change this in isolation.
- Per-layer config reads in `run_perimeters` MUST use `_config.get*` directly each call; caching the `on_print_start` values for re-use across layers is forbidden because it defeats the layer-override mechanism.

## Risks and Tradeoffs

- WIT-type-identity break: editing `ir-types.wit` without rebuilding guest WASM produces silent test failures that look unrelated. Mitigation: explicit `cargo xtask build-guests --check` gate in Step 2's exit condition.
- Schema-bump test-fixture regression: existing committed `SliceIR` JSON fixtures with the old `MaterialBoundary { adjacent_tool: u32 }` shape might not deserialize without the migration adapter. Mitigation: include the migration adapter in Step 2, not Step 5; add a parse-old-shape test in `material_boundary_widening_tdd`.
- Helper-extraction sequencing: extracting helpers and migrating both modules in one step is too large (>3 files / step). Mitigated by Step 1 doing only the `slicer-core::perimeter_utils.rs` creation + `classic-perimeters` migration, leaving `arachne-perimeters` migration as Step 1b (Step 1's second half — see implementation plan).
- Manifest reconcile direction: the roadmap defaults to "manifest is source of truth". If the maintainers prefer the code values, this is a 1-line edit to the manifest instead of the code. Documented as `[FWD]` in §Open Questions.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — IR widening + WIT + serde adapter + new test crosses three crates).
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): "Find all match arms or constructors of `WallBoundaryType::MaterialBoundary`" — MUST return `LOCATIONS` capped at 20 entries. If the call-site count exceeds 20, the implementer halts and re-scopes (likely indicates the widening is more invasive than estimated).

## Open Questions

- `[FWD]` Manifest-vs-code reconcile direction (T-018 / AC-6): roadmap default is "manifest wins, code aligns to manifest". If the implementer discovers a manifest value is itself wrong (e.g., the 30.0 outer_wall_speed doesn't match OrcaSlicer's documented `outer_wall_speed` default), flag and ask. Otherwise apply the default direction without escalating.
- `[FWD]` `_paint` documentation language (T-019): the doc-comment should explicitly say "intentionally unread in this module — consumed by Phase 2 follow-up packet 102". Confirm wording matches the equivalent disuse comments in other modules during implementation; if no precedent exists, use the wording above verbatim.
