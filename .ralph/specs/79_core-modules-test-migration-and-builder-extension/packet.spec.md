---
status: draft
packet: 79
task_ids: [TASK-227, TASK-228]
requires: [78]
backlog_source: docs/07_implementation_status.md
---

# Packet 79 — Migrate Remaining Core-Module Tests + Extend `test_support` Builders

## Goal

Make every core-module test under `modules/core-modules/*/tests/` that can use shared builders do so — extend `slicer_sdk::test_support` builders with the four IR-shape gaps the existing fixtures don't cover (`LayerCollectionIR`, `PrintEntity`, `ToolChange`, variant `WallLoop` flag combos / `SeamCandidate`), then bulk-migrate 13 modules using the consolidated `slicer_sdk::test_prelude` so that no core-module retains hand-rolled `make_*` fixture helpers when a shared builder covers the shape.

## Scope Boundaries

This packet has two halves with a hard sequencing constraint between them. **Half one** extends `slicer_sdk::test_support` builders behind the existing `test` feature: adds a new `LayerCollectionFixtureBuilder` (the production `LayerCollectionBuilder` at `crates/slicer-sdk/src/layer_collection_builder.rs` covers a different concern — entity ordering for finalization — and is left alone), adds freestanding `print_entity(...)`, `tool_change(...)`, `seam_candidate(...)` fixture helpers, and extends `PerimeterRegionViewBuilder` with `add_outer_wall_with_flags` covering the one `seam-placer::wall_at_z` case the existing API doesn't reach. Each new fixture surface lands with a TDD file under `crates/slicer-sdk/tests/test_support_*_tdd.rs` before any consumer migrates. **Half two** migrates 13 core-modules — 7 Group-A (existing builders cover); 4 Group-B (require the half-one extensions); 3 Group-C (no `make_*` helpers; verify dev-dep + import only if it shortens). 4 modules with no tests at all are skipped. The 3 P78-migrated modules (gyroid, arachne, rectilinear) are verified-only. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 78 implemented**. `slicer_sdk::test_support` (mod + the four hook functions from packet 77) and `slicer_sdk::test_prelude` MUST already exist; the `slicer-test` crate MUST be deleted; the `pnp_cli module new` scaffold MUST already emit `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }`.
- Closure requires `cargo xtask build-guests --check` clean (rebuild if stale) at the gate. No WIT or macro changes in this packet, but the bulk of touched files means a fresh resolver pass is plausible.

## Acceptance Criteria

### AC-1 — `print_entity` fixture helper exists and TDD-locked

**Given** the builder extension,
**When** `crates/slicer-sdk/src/test_support/fixtures.rs` is inspected,
**Then** it contains a `pub fn print_entity(entity_id: u64, role: ExtrusionRole, points: Vec<Point3WithWidth>, region_key: RegionKey, topo_order: u32) -> PrintEntity` (or equivalent signature accepting at minimum those five inputs) gated by the existing `test_support` feature umbrella; `crates/slicer-sdk/tests/test_support_print_entity_tdd.rs` exists and `cargo test -p slicer-sdk --test test_support_print_entity_tdd` passes a round-trip test that constructs an entity, asserts every field matches the input.

| `grep -qE 'pub fn print_entity' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_print_entity_tdd`

### AC-2 — `tool_change` fixture helper exists and TDD-locked

**Given** the builder extension,
**When** `crates/slicer-sdk/src/test_support/fixtures.rs` is inspected,
**Then** it contains a `pub fn tool_change(after_entity_index: u32, tool_index: u32) -> ToolChange` (or equivalent signature accepting at minimum those two inputs); `crates/slicer-sdk/tests/test_support_tool_change_tdd.rs` exists and `cargo test -p slicer-sdk --test test_support_tool_change_tdd` passes.

| `grep -qE 'pub fn tool_change' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_tool_change_tdd`

### AC-3 — `LayerCollectionFixtureBuilder` exists and TDD-locked; does NOT collide with production `LayerCollectionBuilder`

**Given** the new fixture builder,
**When** `crates/slicer-sdk/src/test_support/fixtures.rs` is inspected,
**Then** it contains a `pub struct LayerCollectionFixtureBuilder` with at minimum `pub fn new() -> Self`, `pub fn global_layer_index(self, idx: u32) -> Self`, `pub fn z(self, z: f32) -> Self`, `pub fn add_entity(self, e: PrintEntity) -> Self`, `pub fn add_tool_change(self, tc: ToolChange) -> Self`, `pub fn build(self) -> LayerCollectionIR`. The production `crates/slicer-sdk/src/layer_collection_builder.rs::LayerCollectionBuilder` (entity-ordering surface) is unchanged. `crates/slicer-sdk/tests/test_support_layer_collection_fixture_builder_tdd.rs` exists and passes a test that constructs a layer with two entities + one tool change and asserts every field of the resulting `LayerCollectionIR`.

| `grep -qE 'pub struct LayerCollectionFixtureBuilder' crates/slicer-sdk/src/test_support/fixtures.rs && grep -qE 'pub fn (add_entity|add_tool_change|global_layer_index|z|build)' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_layer_collection_fixture_builder_tdd && [ "$(grep -c 'pub fn' crates/slicer-sdk/src/layer_collection_builder.rs)" = "$(grep -c 'pub fn' crates/slicer-sdk/src/layer_collection_builder.rs)" ]`

### AC-4 — `PerimeterRegionViewBuilder::add_outer_wall_with_flags` exists and TDD-locked

**Given** the builder extension,
**When** `crates/slicer-sdk/src/test_support/fixtures.rs` is inspected,
**Then** `impl PerimeterRegionViewBuilder` contains a method matching `pub fn add_outer_wall_with_flags(self, path: ExtrusionPath3D, feature_flags: Vec<WallFeatureFlag>, boundary_type: WallBoundaryType) -> Self` (or equivalent with at minimum those three inputs); `crates/slicer-sdk/tests/test_support_wall_loop_with_flags_tdd.rs` exists and passes, exercising the `seam-placer::wall_at_z` shape (non-empty `feature_flags`, `boundary_type: WallBoundaryType::ExteriorSurface`).

| `grep -qE 'pub fn add_outer_wall_with_flags' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_wall_loop_with_flags_tdd`

### AC-5 — `seam_candidate` fixture helper exists and TDD-locked

**Given** the builder extension,
**When** `crates/slicer-sdk/src/test_support/fixtures.rs` is inspected,
**Then** it contains a `pub fn seam_candidate(position: Point3WithWidth, score: f32, reason: SeamReason) -> SeamCandidate` (or equivalent accepting at minimum those three inputs); `crates/slicer-sdk/tests/test_support_seam_candidate_tdd.rs` exists and passes.

| `grep -qE 'pub fn seam_candidate' crates/slicer-sdk/src/test_support/fixtures.rs && cargo test -p slicer-sdk --test test_support_seam_candidate_tdd`

### AC-6 — All 7 Group-A modules pass tests via `slicer_sdk::test_prelude` and have no multi-line `make_*` helpers from their pre-packet list

**Given** the Group-A migration (Group A: `layer-planner-default`, `lightning-infill`, `mesh-segmentation`, `traditional-support`, `tree-support`, `classic-perimeters`, plus `gyroid-infill` verified from packet 78),
**When** `cargo test -p layer-planner-default -p lightning-infill -p mesh-segmentation -p traditional-support -p tree-support -p classic-perimeters -p gyroid-infill` runs,
**Then** all 7 packages pass AND for each package the test files (a) declare `use slicer_sdk::test_prelude::*;` near the top, and (b) any retained `fn make_*` shells have bodies ≤ 4 lines (single-expression builder chains; the verification command's `awk` body-length probe catches multi-line constructions).

| `cargo test -p layer-planner-default -p lightning-infill -p mesh-segmentation -p traditional-support -p tree-support -p classic-perimeters -p gyroid-infill && for m in layer-planner-default lightning-infill mesh-segmentation traditional-support tree-support classic-perimeters gyroid-infill; do grep -qE 'use slicer_sdk::test_prelude' modules/core-modules/$m/tests/*.rs || exit 1; for fn in $(grep -hoE '^fn make_\w+' modules/core-modules/$m/tests/*.rs | cut -d' ' -f2); do for f in modules/core-modules/$m/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done; done`

### AC-7 — All 4 Group-B modules pass tests via the new builder extensions

**Given** the Group-B migration (Group B: `path-optimization-default`, `seam-placer`, `skirt-brim`, `wipe-tower`),
**When** `cargo test -p path-optimization-default -p seam-placer -p skirt-brim -p wipe-tower` runs,
**Then** all 4 packages pass AND each test file declares `use slicer_sdk::test_prelude::*;` AND for the previously-noted `make_*` helpers (per-module list in `requirements.md`) bodies are ≤ 4 lines (single-expression builder chains using the new fixtures from AC-1/-2/-3/-4/-5).

| `cargo test -p path-optimization-default -p seam-placer -p skirt-brim -p wipe-tower && for m in path-optimization-default seam-placer skirt-brim wipe-tower; do grep -qE 'use slicer_sdk::test_prelude' modules/core-modules/$m/tests/*.rs || exit 1; for fn in $(grep -hoE '^fn (make_wall_loop|candidate|wall_at_z|make_entity_at|make_layer_with_entities|make_layer)' modules/core-modules/$m/tests/*.rs | awk '{print $2}'); do for f in modules/core-modules/$m/tests/*.rs; do awk "/^fn $fn/,/^}/" "$f" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done; done; done`

### AC-8 — Group-C modules (no `make_*` helpers) compile and test with `slicer_sdk::test_prelude` import added only if the file becomes shorter

**Given** the Group-C verification (Group C: `fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`),
**When** `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing` runs,
**Then** all 3 packages pass AND for each module either (a) the test file declares `use slicer_sdk::test_prelude::*;` AND has fewer total lines than before this packet, OR (b) the test file is untouched (no `slicer-sdk` dev-dep added). The choice per module is documented in the implementation log; cosmetic-only changes are rejected.

| `cargo test -p fuzzy-skin -p support-surface-ironing -p top-surface-ironing`

### AC-9 — `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` is present in every migrated module's Cargo.toml

**Given** the migration,
**When** the `Cargo.toml` of every Group-A and Group-B module is inspected,
**Then** each contains a `[dev-dependencies]` section with a `slicer-sdk` line that includes `features = ["test"]`. Group-C modules that opted into the prelude (per AC-8) also satisfy this; Group-C modules that opted out are exempt.

| `for m in layer-planner-default lightning-infill mesh-segmentation traditional-support tree-support classic-perimeters path-optimization-default seam-placer skirt-brim wipe-tower; do grep -A5 '^\[dev-dependencies\]' modules/core-modules/$m/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]' || { echo "missing in $m"; exit 1; }; done`

### AC-10 — Wasm-target gate: production guest builds for representative migrated modules do not pull in `test_support`

**Given** the feature-gate discipline from packet 78 (`[dependencies] slicer-sdk` has no `features`; only `[dev-dependencies]` activates `test`),
**When** `cargo check --target wasm32-unknown-unknown` runs against three representative migrated modules (`skirt-brim` from Group B because it uses the new `LayerCollectionFixtureBuilder` heavily; `seam-placer` from Group B because it uses the new wall-loop-with-flags; `classic-perimeters` from Group A as a sanity baseline),
**Then** each builds cleanly AND `cargo tree --target wasm32-unknown-unknown -p skirt-brim 2>&1 | grep -E 'slicer-sdk.*feature="test"'` returns empty (no `test` feature activation in the production guest build).

| `rustup target list --installed | grep -q wasm32-unknown-unknown && for m in skirt-brim seam-placer classic-perimeters; do cargo check --target wasm32-unknown-unknown -p $m || exit 1; done && ! (cargo tree -p skirt-brim --target wasm32-unknown-unknown 2>&1 | grep -qE 'slicer-sdk.*feature="test"')`

### AC-11 — Bulk acceptance ceremony: `cargo test --workspace` passes

**Given** all 13 module migrations + 3 verifications + 5 builder extensions,
**When** `cargo test --workspace` runs (this is the rare packet justifying workspace-wide testing per project `CLAUDE.md` — bulk migration touching every core-module's tests; the narrower per-module runs in AC-6/-7/-8 cover the work but the workspace gate is the closure verification),
**Then** all tests pass with zero regressions. The implementation log records the test count and per-package pass/fail summary.

| `cargo test --workspace`

## Negative Test Cases

### AC-N1 — Assertion preservation regression: a single canonical pre/post snapshot

**Given** the assertion-preservation discipline (no assertion may weaken during migration),
**When** the implementer records, in the implementation notes, the verbatim assertion lines from one representative test in each Group-A and Group-B module BEFORE the migration begins, then after the migration confirms the same assertion line exists in the migrated file,
**Then** every pre-migration assertion has an identical post-migration counterpart. A test where `assert!((module.density() - 0.2).abs() < 0.001)` becomes `assert!((module.density() - 0.2).abs() < 0.01)` (looser tolerance) is rejected and reverted. The implementation log records the 11 pre/post snapshots.

| (Manual implementer ceremony — documented in `implementation-plan.md` step "Assertion preservation snapshot". Not CI-gated.)

### AC-N2 — Adding a new builder/helper without TDD coverage is rejected

**Given** the discipline that builder extensions MUST land with TDD coverage (AC-1, AC-2, AC-3, AC-4, AC-5 each require a `test_support_*_tdd.rs` file),
**When** the implementer's first attempt adds the new helpers to `fixtures.rs` without TDD files,
**Then** the verification commands for AC-1 through AC-5 fail because the corresponding `cargo test -p slicer-sdk --test test_support_*_tdd` invocations fail with "no test named ..." — the failing test invocation is itself the signal that TDD coverage is missing, and the step must back-fill before continuing.

| (Implicit in AC-1 through AC-5 verification; the test-file existence check makes this self-enforcing.)

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo check --target wasm32-unknown-unknown -p skirt-brim` AND `cargo check --target wasm32-unknown-unknown -p seam-placer` (confirms feature gate holds for both new-builder consumers)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace` (acceptance ceremony; see project `CLAUDE.md` §Test Discipline for the workspace-test escape clause this packet activates)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/05_module_sdk.md` (post-packet-78) — `## Test Support (slicer-sdk feature)` section is the canonical doc for the builder surface; this packet extends what it describes (with one-line additions to the helper list, NOT a structural rewrite).
- `docs/02_ir_schemas.md` — `PrintEntity`, `LayerCollectionIR`, `ToolChange`, `WallLoop`, `SeamCandidate` IR shapes. Read only the field-name authority sections for each shape being constructed.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — fold decision rationale; no change in this packet.
- `CLAUDE.md` project root — §Test Discipline (test-bucket policy, workspace-test escape clause), §Guest WASM Staleness.

## Doc Impact Statement

`docs/05_module_sdk.md` §Test Support gains one or two lines listing the new fixture functions (`LayerCollectionFixtureBuilder`, `print_entity`, `tool_change`, `seam_candidate`, `add_outer_wall_with_flags`). No structural rewrite — packet 78 did that work. No ADR created. `docs/07_implementation_status.md` rows for TASK-227 and TASK-228 flip from `[ ]` to `[x]` at closure.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
