# Requirements — Packet 79

## Packet Metadata

- **Packet**: 79
- **Slug**: `79_core-modules-test-migration-and-builder-extension`
- **Status**: draft
- **Task IDs**: TASK-227, TASK-228
- **Requires**: 78
- **Backlog source**: `docs/07_implementation_status.md`

## Problem Statement

After packet 78 lands, `slicer_sdk::test_support` owns the canonical module-testing fixture API behind a `test` feature, exposed via `slicer_sdk::test_prelude`. Two exemplar modules (`arachne-perimeters`, `rectilinear-infill`, joined by the P78 packet's continuity check on `gyroid-infill`) prove the consolidated builders cover diverse `ConfigViewBuilder` / `SliceRegionViewBuilder` shapes. But 13 other core-modules with tests still carry hand-rolled `make_*` fixture helpers, and 4 of those (the "Group B" modules in this packet's classification) construct IR types the existing builders don't yet cover: `path-optimization-default` and `seam-placer` build variant `WallLoop` shapes; `skirt-brim` builds `LayerCollectionIR` with `PrintEntity` lists; `wipe-tower` builds `LayerCollectionIR` with `ToolChange` entries. Recon (in this packet's generation phase) confirmed that `crates/slicer-sdk/src/layer_collection_builder.rs` — the production builder — covers a different concern (entity ordering for finalization) and is not the right surface to extend; this packet adds a parallel `LayerCollectionFixtureBuilder` to `test_support/fixtures.rs` instead.

This packet does two things in sequence. **Half one** extends the test_support builders behind the existing feature gate: new `LayerCollectionFixtureBuilder`, freestanding `print_entity(...)`, `tool_change(...)`, `seam_candidate(...)` helpers, and one new method on the existing `PerimeterRegionViewBuilder` (`add_outer_wall_with_flags`) to cover seam-placer's specialized wall-loop shape. Each new fixture surface lands with a `crates/slicer-sdk/tests/test_support_*_tdd.rs` round-trip test before any consumer touches it. **Half two** migrates the 13 modules. Group A (7 modules — `layer-planner-default`, `lightning-infill`, `mesh-segmentation`, `traditional-support`, `tree-support`, `classic-perimeters`, plus `gyroid-infill` verified from P78) maps cleanly to the existing builders. Group B (4 modules — `path-optimization-default`, `seam-placer`, `skirt-brim`, `wipe-tower`) requires the half-one extensions. Group C (3 modules — `fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`) have tests but no `make_*` helpers; verify they still pass with the post-78 test_prelude available, and add the import only when it shortens the file. 4 modules with no tests (`machine-gcode-emit`, `part-cooling`, `seam-planner-default`, `support-planner`) are skipped — support-planner gains its first test in packet 80 via the relocation of `prepass_support_generation_orca_parity_tdd.rs` from runtime.

The migration discipline is strict: every original assertion must survive the migration verbatim (looser tolerances or skipped checks are rejected). The implementer captures a pre/post assertion snapshot for one representative test per migrated module as a human-readable audit trail (AC-N1).

Without this packet, the consolidation that packets 77 and 78 begin remains half-finished: the builders exist but most modules don't use them; documentation describes a single test surface but the codebase still has 13 modules independently reinventing it.

## In Scope

### Half one — `test_support` builder extensions

- **`crates/slicer-sdk/src/test_support/fixtures.rs`** — under the existing `test_support` feature umbrella, add:
  - `pub fn print_entity(entity_id: u64, role: ExtrusionRole, points: Vec<Point3WithWidth>, region_key: RegionKey, topo_order: u32) -> PrintEntity`
  - `pub fn tool_change(after_entity_index: u32, tool_index: u32) -> ToolChange` (or a wider signature accepting at least those two fields plus any others required to construct a non-default `ToolChange` per `docs/02_ir_schemas.md`)
  - `pub fn seam_candidate(position: Point3WithWidth, score: f32, reason: SeamReason) -> SeamCandidate`
  - `pub struct LayerCollectionFixtureBuilder { ... }` with `new`, `global_layer_index(u32)`, `z(f32)`, `add_entity(PrintEntity)`, `add_tool_change(ToolChange)`, `build() -> LayerCollectionIR`. Internally uses `..Default::default()` for fields not explicitly set (relies on the `Default` derive on `LayerCollectionIR` from TASK-200b/200c).
  - `impl PerimeterRegionViewBuilder { pub fn add_outer_wall_with_flags(self, path: ExtrusionPath3D, feature_flags: Vec<WallFeatureFlag>, boundary_type: WallBoundaryType) -> Self }` — covers `seam-placer::wall_at_z`'s non-empty-flags + `ExteriorSurface`-boundary shape.
- **`crates/slicer-sdk/src/test_prelude.rs`** — re-export the 5 new surfaces under the existing whole-module gate.
- **`crates/slicer-sdk/tests/test_support_print_entity_tdd.rs`** — new TDD; round-trip assertion.
- **`crates/slicer-sdk/tests/test_support_tool_change_tdd.rs`** — new TDD.
- **`crates/slicer-sdk/tests/test_support_seam_candidate_tdd.rs`** — new TDD.
- **`crates/slicer-sdk/tests/test_support_layer_collection_fixture_builder_tdd.rs`** — new TDD; covers `add_entity` + `add_tool_change` + `global_layer_index` + `z` + `build`.
- **`crates/slicer-sdk/tests/test_support_wall_loop_with_flags_tdd.rs`** — new TDD; exercises `add_outer_wall_with_flags`.
- **`docs/05_module_sdk.md` §Test Support** — one paragraph appending the new helpers to the documented surface. **No structural rewrite** — packet 78 did that.

### Half two — Module migrations (13 modules touched + 1 verified)

- **Group A** (existing builders cover; replace `make_*` bodies with single-expression builder chains):
  - `modules/core-modules/layer-planner-default/tests/*.rs` — 2 helpers (`make_config`, `make_config_with_per_object_lh`)
  - `modules/core-modules/lightning-infill/tests/*.rs` — 2 helpers (`make_config`, `make_square_region`)
  - `modules/core-modules/mesh-segmentation/tests/*.rs` — 2 helpers (`config_with`, `object_view`)
  - `modules/core-modules/traditional-support/tests/*.rs` — 2 helpers (`make_config`, `make_square_region`)
  - `modules/core-modules/tree-support/tests/*.rs` — 2 helpers (`make_config`, `make_square_region`)
  - `modules/core-modules/classic-perimeters/tests/*.rs` — 7 helpers (`make_square`, `make_config`, `make_config_with_speeds`, `make_region`, `square_polygon`, `config_1_wall`, `config_2_walls`)
  - **Verify-only**: `modules/core-modules/gyroid-infill/tests/*.rs` — migrated in P78; this packet runs `cargo test -p gyroid-infill` as a regression check, no edits.
- **Group B** (require half-one extensions; replace `make_*` bodies with chains using the new fixtures):
  - `modules/core-modules/path-optimization-default/tests/*.rs` — 3 `make_wall_loop` variants. Recon confirmed all three use `feature_flags: vec![]` + `boundary_type: Interior` — the existing `PerimeterRegionViewBuilder::add_outer_wall` already produces this shape, so the migration here does NOT actually need the new `add_outer_wall_with_flags`. The helpers can collapse to direct `add_outer_wall(...)` calls in the test bodies.
  - `modules/core-modules/seam-placer/tests/*.rs` — 2 helpers (`candidate`, `wall_at_z`). `candidate` migrates to `seam_candidate(...)`; `wall_at_z` migrates to `add_outer_wall_with_flags(...)` because it uses non-empty flags + `ExteriorSurface`.
  - `modules/core-modules/skirt-brim/tests/*.rs` — 4 helpers across two test files: `make_entity_at` × 2 (one in `skirt_brim_tdd.rs`, one in `finalization_live_tdd.rs` with different signature), `make_layer_with_entities`, `make_layer`. All migrate to `print_entity(...)` + `LayerCollectionFixtureBuilder`.
  - `modules/core-modules/wipe-tower/tests/*.rs` — 2 `make_layer` variants (one each in `wipe_tower_tdd.rs` and `finalization_live_tdd.rs`). Both migrate to `LayerCollectionFixtureBuilder::new().z(...).add_tool_change(tool_change(...))....build()`. The `dummy_entity` helper they call internally can stay or migrate to `print_entity(...)` at the implementer's discretion.
- **Group C** (no `make_*` helpers; cosmetic-only decisions):
  - `modules/core-modules/fuzzy-skin/tests/*.rs`
  - `modules/core-modules/support-surface-ironing/tests/*.rs`
  - `modules/core-modules/top-surface-ironing/tests/*.rs`
  - For each: if adding `use slicer_sdk::test_prelude::*;` enables a meaningful collapse of polygon literals to `square_polygon`/`rect_path` calls AND the file gets shorter, migrate. Otherwise leave untouched. Document the decision per module in the implementation log.
- **`Cargo.toml` of every migrated Group-A and Group-B module** — add `slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` to `[dev-dependencies]`. Group-C modules get the dev-dep only if they migrate per the cosmetic-collapse rule.

## Out of Scope

- The 4 modules with no tests (`machine-gcode-emit`, `part-cooling`, `seam-planner-default`, `support-planner`). Support-planner gains its first test in packet 80.
- Relocating runtime-located module tests (packet 80).
- Extending `slicer-sdk::host` or `slicer-sdk::test_support`'s hook functions (frozen since packet 77).
- Extending the production `LayerCollectionBuilder` at `crates/slicer-sdk/src/layer_collection_builder.rs`. The new `LayerCollectionFixtureBuilder` is a separate test-side type to avoid collision.
- Touching `OrcaSlicerDocumented/`. No parity concerns.
- Touching `crates/slicer-runtime/test-guests/`.
- Adding new test coverage to migrated modules. Migration is verbatim — same assertions, same inputs.
- Switching any `#[test]` to `#[module_test]`. None of the migrated modules' tests reach into `host::*` (per packet 77's grilling-phase survey; reconfirmed by classic-perimeters recon in this packet's generation).
- Changing the `LayerCollectionIR`, `PrintEntity`, `ToolChange`, `WallLoop`, `SeamCandidate` IR shapes. The fixtures construct these — they do not modify the IR.

## Authoritative Docs

- `docs/05_module_sdk.md` (post-packet-78 state) — §Test Support. Read only the section being appended to (≈ lines 445-560 post-rewrite; the exact range depends on packet 78's final edit).
- `docs/02_ir_schemas.md` — IR field definitions for `PrintEntity` (IR 9), `LayerCollectionIR` (IR 12), `ToolChange` (subsection of IR 12), `WallLoop` (subsection of IR 6), `SeamCandidate`. Read only the field surfaces being populated. **Size note**: > 600 lines — never load whole; offset-load with line ranges per IR.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — no change; quoted only as the prior decision record.
- `CLAUDE.md` (project root) — §Test Discipline (this is the packet that activates the workspace-test escape clause; the closure ceremony runs `cargo test --workspace`).
- `crates/slicer-sdk/src/layer_collection_builder.rs` — 97 lines per recon; read once to confirm it covers entity-ordering only and is not the right target to extend.
- `crates/slicer-sdk/src/test_support/fixtures.rs` (post-packet-78 location) — the file to extend.

## OrcaSlicer Reference Obligations

None. This packet does not borrow or check parity against any OrcaSlicer code.

## Acceptance Summary

ACs are defined in `packet.spec.md` and referenced by ID. Measurable refinements:

- **AC-1 / -2 / -3 / -4 / -5 refinement**: each builder TDD MUST be written before the corresponding production helper code (red-then-green). The red phase confirms the TDD genuinely depends on the new helper; the green phase locks behavior. AC-N2 is the self-enforcement mechanism (the cargo test invocation fails until the TDD file exists).
- **AC-3 refinement**: `LayerCollectionFixtureBuilder` MUST coexist with the production `LayerCollectionBuilder` without name collision. They live in different modules (`test_support::fixtures` vs `layer_collection_builder`); both can be `use`-imported in the same file if needed (though no current consumer does so).
- **AC-6 refinement**: classic-perimeters has 7 helpers spanning two test files (`classic_perimeters_tdd.rs` and `boundary_paint_tdd.rs`). Both files migrate.
- **AC-7 refinement**: `path-optimization-default`'s 3 `make_wall_loop` variants are functionally identical (per recon — same body, parameter list varies only in whether `width` is configurable); the migration collapses all three call sites to direct `PerimeterRegionViewBuilder::add_outer_wall(rect_path(...))`-style calls. The helper function shells may disappear entirely; if they stay, the body is one line.
- **AC-7 refinement (skirt-brim)**: the four `make_entity_at` / `make_layer*` helpers span two test files with slightly different parameter lists. Each migration preserves the original parameter list (so call sites don't need rewriting), only the body changes to a builder chain.
- **AC-8 refinement**: "cosmetic-only changes are rejected" means a Group-C migration that adds `use slicer_sdk::test_prelude::*;` but doesn't actually use any of the imports is rejected. Either use the prelude productively (replacing inline polygon literals with `square_polygon`/`rect_path`, etc.), or don't import it.
- **AC-9 refinement**: the dev-dep path is relative — `../../../crates/slicer-sdk` per the prior P78 scaffold convention.
- **AC-9 fragility note**: the verification uses `grep -A5 '^\[dev-dependencies\]' | grep -qE 'slicer-sdk.*features = \[.*"test".*\]'`, which assumes `slicer-sdk` appears within the first 5 lines following `[dev-dependencies]`. All current Group-A/B module dev-dep sections are short enough that this holds. If a future module's dev-dep section grows beyond that window, the grep silently false-fails and AC-9 reports "missing in $m" when the entry is actually present — switch to `cargo metadata --format-version=1 --no-deps` parsing (or `cargo tree -e features` with the C-1 pattern from AC-10) at that point.
- **AC-11 refinement**: the workspace test sweep is the gate. The implementation log MUST capture the total test count (e.g., "1985 passed / 0 failed / 0 ignored") so future audits can detect regressions.
- **AC-N1 refinement**: the implementer picks ONE representative test per migrated module (10 modules in Groups A+B; 11 if classic-perimeters' two test files each get a snapshot — implementer's discretion), records the pre-migration assertion line(s) verbatim in the implementation log, then after migration confirms the same lines still exist (the file may have moved them up/down by line number, but the literal assertion text — `assert!(...)`, `assert_eq!(...)`, etc. — is identical).

## Verification Commands

| AC | Command | Delegation hint |
|---|---|---|
| AC-1 | `grep -qE 'pub fn print_entity' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_print_entity_tdd` | Delegate cargo test; grep direct. |
| AC-2 | `grep -qE 'pub fn tool_change' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_tool_change_tdd` | Same. |
| AC-3 | (compound; see `packet.spec.md`) | Delegate cargo test; grep + line-count direct. |
| AC-4 | `grep -qE 'pub fn add_outer_wall_with_flags' ... && cargo test -p slicer-sdk --test test_support_wall_loop_with_flags_tdd` | Same. |
| AC-5 | `grep -qE 'pub fn seam_candidate' ... && cargo test -p slicer-sdk --test test_support_seam_candidate_tdd` | Same. |
| AC-6 | (compound 7-package sweep + helper-body check) | Delegate 7-package `cargo test`; the `awk` body-length loop is local. |
| AC-7 | (compound 4-package sweep + helper-body check) | Same. |
| AC-8 | `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing` | Delegate. |
| AC-9 | (compound `Cargo.toml` grep loop) | Direct. |
| AC-10 | (compound wasm-target + cargo-tree) | Delegate cargo invocations. |
| AC-11 | `cargo test --workspace` | Delegate; return per-package counts. **Project-wide exception activated** — this is the bulk-migration acceptance ceremony. |
| AC-N1 | Manual snapshot — implementer ceremony documented in `implementation-plan.md`. | Not CI-gated. |
| AC-N2 | Implicit in AC-1..AC-5 (the cargo test invocation fails when the TDD file is missing). | Self-enforcing. |
| Closure: workspace check | `cargo check --workspace --all-targets` | Delegate. |
| Closure: clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Delegate. |
| Closure: guest staleness | `cargo xtask build-guests --check` then rebuild if STALE | Delegate; expect STALE due to feature-flag activations on new dev-deps in 10 module Cargo.tomls. |

## Step Completion Expectations

This packet has a **hard sequencing constraint** that per-step preconditions cannot fully express: **all five builder extensions (steps 2-6) and their TDD coverage MUST be green before any Group-B migration begins (steps 11-14)**. A Group-B migration that consumes a builder method that doesn't yet exist would compile-fail; the implementer must hold the migration until the builder lands. The per-step preconditions in `implementation-plan.md` encode this serialization explicitly via `Precondition: steps X-Y complete`. Group-A migrations (steps 8-10) can proceed in parallel with the second half of the builder extensions (steps 4-6) since they use only existing APIs, but the implementation plan documents the steps as sequential for clarity.

## Context Discipline Notes

- This packet has 13 module migrations + 5 builder extensions + 5 TDD files + 3 Group-C decisions + 11 assertion snapshots + doc updates. **Aggregate context cost is L**, with natural handoff boundaries between half one (steps 1-7: extensions) and half two (steps 8-onwards: migrations).
- Per-module migrations are small individually but the cumulative read load (every module's test files + production source for config-key strings) can balloon. Use focused dispatches per module: "list the config keys used by `<module>::on_print_start`" returns FACT in ≤ 5 lines, vs reading the full module source.
- The recon at packet generation time captured verbatim helper bodies for Group-B modules — the implementer should NOT re-read those bodies from the source files; they are in this packet's `design.md` §Data and Contract Notes for direct reference.
- `cargo test --workspace` (AC-11) takes ≥ 11 minutes per project `CLAUDE.md`. Dispatch with `run_in_background: true` if implementing in a context-constrained environment; return only the summary line.
- Guest WASM rebuild is a precaution, not an expected outcome: dev-dependencies do not enter the guest's production dependency closure, so `Cargo.toml` dev-dep-only additions should not invalidate the guest build-script input hashes. Run `cargo xtask build-guests --check` anyway as standard hygiene per `CLAUDE.md` §Guest WASM Staleness; rebuild (drop `--check`) only if `STALE:` is actually reported.
