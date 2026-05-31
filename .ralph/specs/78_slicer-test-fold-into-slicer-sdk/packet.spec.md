---
status: draft
packet: 78
task_ids: [TASK-225, TASK-226]
requires: [77]
backlog_source: docs/07_implementation_status.md
---

# Packet 78 — Fold `slicer-test` into `slicer-sdk`, Delete the Crate, Migrate Two Exemplar Modules

## Goal

Consolidate every test helper under `slicer_sdk::test_support` behind feature `test`, expose them via a new `slicer_sdk::test_prelude` distinct from `slicer_sdk::prelude`, delete the `slicer-test` crate, update the `pnp_cli module new` scaffold to emit the single feature-gated dev-dep, and migrate `arachne-perimeters` + `rectilinear-infill` as exemplars whose `make_*` helpers prove the consolidated builder API covers the diverse config/region permutations the wider migration in packet 79 will rely on.

## Scope Boundaries

This packet executes the file-level fold and the crate deletion. Source moves from `crates/slicer-test/src/*` into `crates/slicer-sdk/src/test_support/`, tests follow with renamed file prefixes to avoid collisions in the destination directory, and the workspace member list shrinks by one. A new whole-module-gated `crates/slicer-sdk/src/test_prelude.rs` becomes the canonical import path for module-author tests; `slicer_sdk::prelude` remains feature-independent for production code. Two exemplar core-modules migrate their hand-rolled `make_*` fixtures to the consolidated builders — chosen for diverse builder API coverage (six helpers each, exercising `ConfigViewBuilder`'s four primitive accessors and `SliceRegionViewBuilder`'s polygon + infill-area permutations) so any builder gap surfaces here before packet 79 attempts 18 modules. The `docs/05` section heading rewrite (`slicer-test Crate` → `Test Support`) lands here; the `docs/00` crate-inventory update also lands. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 77 implemented**. `slicer-sdk::test_support` must already exist (gated, with the four hooks routing through `host::test_support`), and `MockHost` must already be a real `MeshSource` adapter. Without 77's foundation, the move in this packet has nowhere to land that wouldn't double-implement the seam.
- Closure requires `cargo xtask build-guests --check` clean (rebuild if stale) at the gate — the test support fold does not directly change bindgen text, but the workspace-member removal forces a fresh resolver pass.

## Acceptance Criteria

### AC-1 — `crates/slicer-test/` no longer exists

**Given** the fold,
**When** the working tree is inspected,
**Then** the directory `crates/slicer-test/` does not exist, the workspace `Cargo.toml` member list does not contain `"crates/slicer-test"`, and `cargo metadata --format-version=1 --no-deps` lists exactly 27 workspace members (was 28).

| `test ! -d crates/slicer-test && ! grep -q '"crates/slicer-test"' Cargo.toml && [ "$(cargo metadata --format-version=1 --no-deps 2>/dev/null | python -c 'import sys,json; print(len(json.load(sys.stdin)[\"workspace_members\"]))')" = "27" ]`

### AC-2 — `slicer_sdk::test_support` owns all four source modules + the four hook functions from packet 77

**Given** the source moves,
**When** `crates/slicer-sdk/src/test_support/` is listed,
**Then** it contains `mod.rs` (with the four hook functions from packet 77 still present), `mock_host.rs`, `capture.rs`, `fixtures.rs`, `assert_paths.rs`, AND the whole `test_support` module is gated `#[cfg(any(test, feature = "test"))]` at its declaration in `crates/slicer-sdk/src/lib.rs`.

| `for f in mod.rs mock_host.rs capture.rs fixtures.rs assert_paths.rs; do test -f crates/slicer-sdk/src/test_support/$f || exit 1; done && grep -qE 'cfg\(any\(test, feature = "test"\)\)\]\s*pub mod test_support' crates/slicer-sdk/src/lib.rs`

### AC-3 — `crates/slicer-sdk/src/test_prelude.rs` exists as a separate feature-gated module

**Given** the new prelude,
**When** the file is inspected,
**Then** it exists, its first non-comment line is `#![cfg(any(test, feature = "test"))]` (whole-module gate, not per-item gates), it re-exports at minimum `MockHost`, `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder`, `square_polygon`, `rect_path`, `InfillOutputCapture`, `PerimeterOutputCapture`, `SupportOutputCapture`, and at least five `assert_paths_*` functions (`assert_paths_planar`, `assert_max_segment_length`, `assert_extrusion_width_range`, `assert_paths_inside_polygon`, `assert_no_path_intersections`). `crates/slicer-sdk/src/prelude.rs` does NOT re-export any of these (production prelude stays test-free).

| `test -f crates/slicer-sdk/src/test_prelude.rs && head -3 crates/slicer-sdk/src/test_prelude.rs | grep -qE '^\#!\[cfg\(any\(test, feature = "test"\)\)\]' && for sym in MockHost ConfigViewBuilder SliceRegionViewBuilder PerimeterRegionViewBuilder square_polygon rect_path InfillOutputCapture PerimeterOutputCapture SupportOutputCapture assert_paths_planar assert_max_segment_length assert_extrusion_width_range assert_paths_inside_polygon assert_no_path_intersections; do grep -qE "pub use .*::$sym\b|::\{[^}]*\b$sym\b" crates/slicer-sdk/src/test_prelude.rs || exit 1; done && grep -qE 'MockHost|ConfigViewBuilder|SliceRegionViewBuilder|InfillOutputCapture' crates/slicer-sdk/src/prelude.rs && exit 1; exit 0`

### AC-4 — Production `cargo check --target wasm32-unknown-unknown` for migrated modules does NOT pull in `test_support`

**Given** the feature gate from AC-2,
**When** `cargo check --target wasm32-unknown-unknown -p arachne-perimeters` and `cargo check --target wasm32-unknown-unknown -p rectilinear-infill` run,
**Then** both pass cleanly AND a `cargo tree -p arachne-perimeters --target wasm32-unknown-unknown --no-default-features 2>&1 | grep -E 'slicer-sdk.*feature="test"'` returns empty (confirming the test feature is NOT activated in the production guest wasm build).

| `rustup target list --installed | grep -q wasm32-unknown-unknown && cargo check --target wasm32-unknown-unknown -p arachne-perimeters && cargo check --target wasm32-unknown-unknown -p rectilinear-infill && ! (cargo tree -p arachne-perimeters --target wasm32-unknown-unknown --no-default-features 2>&1 | grep -qE 'slicer-sdk.*feature="test"')`

### AC-5 — `cargo check -p slicer-sdk` (no features) does NOT compile `test_support`

**Given** the whole-module gate at `crates/slicer-sdk/src/lib.rs`,
**When** `cargo check -p slicer-sdk` runs without `--features test`,
**Then** the build succeeds AND a deliberate one-line probe (`pub use crate::test_support::MockHost as _probe;`) added temporarily to `crates/slicer-sdk/src/lib.rs` produces `error[E0433]: failed to resolve: could not find \`test_support\`` (or equivalent). This is the same gate-is-real check as packet 77's AC-N1, applied to the post-fold layout.

| (Implementer-run during step verification, documented in `implementation-plan.md` step "Verify test_support gate post-fold". Not CI-gated.)

### AC-6 — `pnp_cli module new` scaffold emits exactly one `slicer-sdk` dev-dep with `features = ["test"]`

**Given** the scaffold update at `crates/pnp-cli/src/module_new.rs:188-207`,
**When** `pnp_cli module new <name> <stage>` runs and produces the new module's `Cargo.toml`,
**Then** the generated `[dev-dependencies]` section contains exactly one line referencing `slicer-sdk` with `features = ["test"]`, no line contains `slicer-test`, and the `[dependencies]` section's `slicer-sdk` line has no `features` field (production build does not activate `test`).

| `cargo test -p pnp-cli --test module_new_tdd && grep -qE 'slicer-sdk = .*features = \[.*"test".*\]' crates/pnp-cli/src/module_new.rs && ! grep -qE 'slicer-test' crates/pnp-cli/src/module_new.rs`

### AC-7 — `arachne-perimeters` tests use `slicer_sdk::test_prelude` and pass with no `make_*` helpers retained from the original 6

**Given** the exemplar migration,
**When** `cargo test -p arachne-perimeters` runs,
**Then** all original test assertions pass AND `grep -nE '^fn (make_square|make_narrow_rect|make_config|make_config_full|make_region_from_poly|make_region)\b' modules/core-modules/arachne-perimeters/tests/*.rs` returns empty (those six helpers are gone — their bodies now inline as `ConfigViewBuilder` / `SliceRegionViewBuilder` / `square_polygon` chains, or the helper function shells remain as named shorthands but their bodies are single-expression builder chains, not the original multi-line constructions).

| `cargo test -p arachne-perimeters && grep -rnE '^\s*fn (make_square|make_narrow_rect|make_config|make_config_full|make_region_from_poly|make_region)\s*\(' modules/core-modules/arachne-perimeters/tests/ | while read line; do file=$(echo "$line" | cut -d: -f1); fn=$(echo "$line" | grep -oE 'make_\w+'); echo "$fn in $file"; awk "/^fn $fn/,/^}/" "$file" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done`

### AC-8 — `rectilinear-infill` tests use `slicer_sdk::test_prelude` and pass with no original `make_*` helpers retained

**Given** the exemplar migration,
**When** `cargo test -p rectilinear-infill` runs,
**Then** all original test assertions pass AND `grep -nE '^fn (make_square_expolygon|make_test_region|make_config|make_square_region|make_bridge_region)\b' modules/core-modules/rectilinear-infill/tests/*.rs` returns no multi-line definitions (same shorthand-or-gone rule as AC-7).

| `cargo test -p rectilinear-infill && grep -rnE '^\s*fn (make_square_expolygon|make_test_region|make_config|make_square_region|make_bridge_region)\s*\(' modules/core-modules/rectilinear-infill/tests/ | while read line; do file=$(echo "$line" | cut -d: -f1); fn=$(echo "$line" | grep -oE 'make_\w+'); awk "/^fn $fn/,/^}/" "$file" | wc -l | awk '{if ($1 > 4) exit 1}' || exit 1; done`

### AC-9 — Both exemplar modules' `Cargo.toml` carries `slicer-sdk = { ..., features = ["test"] }` in `[dev-dependencies]`

**Given** the migration,
**When** the two modules' `Cargo.toml` files are inspected,
**Then** each contains a `[dev-dependencies]` section with a `slicer-sdk` line that includes `features = ["test"]`, AND neither contains a `slicer-test` line.

| `for m in arachne-perimeters rectilinear-infill; do grep -A5 '^\[dev-dependencies\]' modules/core-modules/$m/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]' || exit 1; grep -qE 'slicer-test' modules/core-modules/$m/Cargo.toml && exit 1; done; exit 0`

### AC-10 — Structural rewrite of `docs/05_module_sdk.md` removes the `slicer-test Crate` section header

**Given** the doc rewrite,
**When** `docs/05_module_sdk.md` is grepped,
**Then** it contains no heading `## slicer-test Crate` (line 445 was the previous location), instead contains `## Test Support (slicer-sdk feature)` (or equivalent — the new section title), every `use slicer_test::*` in examples is replaced by `use slicer_sdk::test_prelude::*` (paired with `use slicer_sdk::prelude::*` in test examples), and the section opens with a one-line note documenting the `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` scaffold convention.

| `! grep -qE '^## slicer-test Crate' docs/05_module_sdk.md && grep -qE '^## Test Support' docs/05_module_sdk.md && ! grep -qE 'use slicer_test::' docs/05_module_sdk.md && grep -qE 'use slicer_sdk::test_prelude' docs/05_module_sdk.md`

### AC-11 — `docs/00_project_overview.md` no longer lists `slicer-test` in the crate inventory

**Given** the crate deletion,
**When** `docs/00_project_overview.md` is grepped,
**Then** it contains no row with `slicer-test` in the crate table at lines ≈ 122-156, no entry `slicer-test/` in the `crates/` directory tree, and `grep -c 'slicer-test' docs/00_project_overview.md` returns 0.

| `! grep -qE 'slicer-test' docs/00_project_overview.md`

## Negative Test Cases

### AC-N1 — Production `cargo build` for the workspace does NOT compile `test_support` source files

**Given** the whole-module gate,
**When** `cargo build --workspace` runs (production profile, no `--features` flag — `test` is not a default feature, and `[dependencies]` references to `slicer-sdk` in core-modules have no `features = ["test"]`),
**Then** the compile succeeds AND inspecting the `target/release/` artifacts via `nm` / `cargo bloat` (or equivalent — implementer's choice) confirms no symbols matching `slicer_sdk::test_support::*` appear in the production output. This is a defensive check ensuring the cargo dev-dep / dep split actually works.

| `cargo build --workspace --release && for sym in MockHost ConfigViewBuilder; do ! find target/release -name '*.rlib' -exec nm {} \; 2>/dev/null | grep -qE "test_support.*$sym" || exit 1; done; exit 0`

### AC-N2 — Removing `[dev-dependencies] slicer-sdk = { features = ["test"] }` from a migrated module fails the module's tests with a clear error

**Given** the dev-dep wiring,
**When** the implementer temporarily removes the `features = ["test"]` from `modules/core-modules/arachne-perimeters/Cargo.toml`'s dev-dep entry (then restores it),
**Then** `cargo test -p arachne-perimeters` fails with an error containing `MockHost` or `ConfigViewBuilder` or `test_prelude` not found / unresolved (proving the gate is the actual mechanism, not paper-thin). This is documented in `implementation-plan.md` step "Verify dev-dep is load-bearing".

| (Implementer-run during step verification. Not CI-gated.)

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo check --target wasm32-unknown-unknown -p arachne-perimeters` AND `cargo check --target wasm32-unknown-unknown -p rectilinear-infill`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p slicer-sdk -p arachne-perimeters -p rectilinear-infill -p pnp-cli --test module_new_tdd`

Full per-AC matrix and delegation hints live in `requirements.md`.

## Authoritative Docs

- `docs/05_module_sdk.md` — the `slicer-test Crate` section is being structurally rewritten (renamed and re-targeted).
- `docs/00_project_overview.md` — the crate inventory loses the `slicer-test` row.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (from packet 77) — the decision this packet executes; quote its `## Decision` section in the rewritten `docs/05` test-support section as the source of truth.
- `CLAUDE.md` (project root) — `slicer-test` references must be scrubbed in step 12.

## Doc Impact Statement

`docs/05_module_sdk.md` loses its `slicer-test Crate` section heading and gains a `Test Support (slicer-sdk feature)` section. `docs/00_project_overview.md` loses one row in the crate table and one line in the directory tree (both at lines ≈ 122-156). Project root `CLAUDE.md` loses any `slicer-test` references (search-and-update). No ADR is created in this packet — ADR-0004 (created in packet 77) is the relevant decision record.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
