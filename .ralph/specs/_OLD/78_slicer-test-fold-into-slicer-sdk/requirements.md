# Requirements — Packet 78

## Packet Metadata

- **Packet**: 78
- **Slug**: `78_slicer-test-fold-into-slicer-sdk`
- **Status**: draft
- **Task IDs**: TASK-225, TASK-226
- **Requires**: 77
- **Backlog source**: `docs/07_implementation_status.md`

## Problem Statement

After packet 77 closes the `#[module_test]` phantom-import contract, two parallel test-support surfaces still exist: `slicer_sdk::test_support` (where the four hook functions live, gated by the new `test` feature) and `crates/slicer-test/` (the standalone crate still housing `MockHost`, `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder`, `square_polygon`, `rect_path`, the three output capture types, and the five `assert_paths_*` helpers). The split is now purely historical — the crate has zero non-self consumers across the workspace's 28 members, its only users are its own internal tests and the `pnp_cli module new` scaffold output, and the documented API surface (`docs/05_module_sdk.md:445-624`) treats `slicer-test` and `slicer-sdk` as one feature anyway. Future agents still see the dual structure and burn time deciding which crate owns what.

This packet executes the architectural decision recorded in ADR-0004 (packet 77): move every source file from `crates/slicer-test/src/*` into `crates/slicer-sdk/src/test_support/` under the existing whole-module `#[cfg(any(test, feature = "test"))]` gate; move every test file under the `test_support_*` prefix into `crates/slicer-sdk/tests/`; introduce a separate `crates/slicer-sdk/src/test_prelude.rs` (whole-module gated, not feature-gated per-item) that re-exports every test helper alongside the existing `crates/slicer-sdk/src/prelude.rs` (which stays test-free); rewrite the `pnp_cli module new` scaffold to emit a single `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` line; remove `crates/slicer-test` from the workspace member list; and delete the `crates/slicer-test/` directory.

To prove the consolidation works under realistic builder API stress before packet 79 commits to migrating 18 core-modules, two exemplar core-modules migrate their hand-rolled `make_*` helpers in this same packet: `arachne-perimeters` (six diverse helpers exercising `ConfigViewBuilder`'s `int` / `float` / `bool` / `string` accessors and `SliceRegionViewBuilder`'s polygon + infill-area permutations) and `rectilinear-infill` (six helpers including bridge-region variants). Either module surfacing a builder gap here is a signal to address it before the bulk packet; clean migrations here let packet 79 follow the same pattern with high confidence.

The `docs/05_module_sdk.md` section heading (`slicer-test Crate` → `Test Support (slicer-sdk feature)`) and `docs/00_project_overview.md` crate-inventory entries also update in this packet because they would otherwise lie about the shipped state.

## In Scope

- **Source-file moves** (4 files):
  - `crates/slicer-test/src/mock_host.rs` → `crates/slicer-sdk/src/test_support/mock_host.rs`
  - `crates/slicer-test/src/capture.rs` → `crates/slicer-sdk/src/test_support/capture.rs`
  - `crates/slicer-test/src/fixtures.rs` → `crates/slicer-sdk/src/test_support/fixtures.rs`
  - `crates/slicer-test/src/assert_paths.rs` → `crates/slicer-sdk/src/test_support/assert_paths.rs`
- **Test-file moves** (renamed to avoid collision in destination directory):
  - `crates/slicer-test/tests/assert_paths_tdd.rs` → `crates/slicer-sdk/tests/test_support_assert_paths_tdd.rs`
  - `crates/slicer-test/tests/config_view_builder_tdd.rs` → `crates/slicer-sdk/tests/test_support_config_view_builder_tdd.rs`
  - `crates/slicer-test/tests/output_capture_tdd.rs` → `crates/slicer-sdk/tests/test_support_output_capture_tdd.rs`
  - `crates/slicer-test/tests/perimeter_region_view_builder_tdd.rs` → `crates/slicer-sdk/tests/test_support_perimeter_region_view_builder_tdd.rs`
  - `crates/slicer-test/tests/slice_region_view_builder_tdd.rs` → `crates/slicer-sdk/tests/test_support_slice_region_view_builder_tdd.rs`
  - The four tests added in packet 77 (`mock_host_adapter_tdd.rs`, `mock_host_isolation_tdd.rs`, `log_capture_round_trip_tdd.rs`, `panic_handler_drains_logs_tdd.rs`) → `crates/slicer-sdk/tests/test_support_*_tdd.rs` (same renaming pattern).
  - `crates/slicer-test/tests/smoke.rs` — absorbed into the existing `crates/slicer-sdk/tests/smoke.rs`.
- **New file**: `crates/slicer-sdk/src/test_prelude.rs` — whole-module-gated `#[cfg(any(test, feature = "test"))]` re-export of `MockHost`, `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder`, `square_polygon`, `rect_path`, the three `*OutputCapture` types, and the five `assert_paths_*` helpers.
- **`crates/slicer-sdk/src/lib.rs`**: add `pub mod test_prelude;` under the same feature gate.
- **`crates/slicer-sdk/src/test_support/mod.rs`**: extend to declare the four newly-moved submodules (`pub mod mock_host;`, `pub mod capture;`, `pub mod fixtures;`, `pub mod assert_paths;`).
- **`crates/pnp-cli/src/module_new.rs:188-207`**: rewrite `generate_cargo_toml` so the emitted scaffold has `[dev-dependencies] slicer-sdk = { path = "../../crates/slicer-sdk", features = ["test"] }` (and no `slicer-test` line). Production `[dependencies] slicer-sdk` line stays without `features`.
- **`crates/pnp-cli/tests/module_new_tdd.rs:36`** and **`crates/pnp-cli/src/module_new.rs:545`**: update test assertions to verify the new scaffold shape.
- **Workspace `Cargo.toml:10`**: remove `"crates/slicer-test",` from the members list.
- **Delete `crates/slicer-test/`** entirely after all moves are confirmed.
- **`modules/core-modules/arachne-perimeters/Cargo.toml`**: add `slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }` to `[dev-dependencies]`.
- **`modules/core-modules/arachne-perimeters/tests/*.rs`**: replace bodies of `make_square`, `make_narrow_rect`, `make_config`, `make_config_full`, `make_region_from_poly`, `make_region` with `ConfigViewBuilder` / `SliceRegionViewBuilder` / `square_polygon` / `rect_path` chains, preserving every original assertion verbatim.
- **`modules/core-modules/rectilinear-infill/Cargo.toml`**: same dev-dep addition.
- **`modules/core-modules/rectilinear-infill/tests/*.rs`**: replace bodies of `make_square_expolygon`, `make_test_region`, `make_config`, `make_square_region`, and the two `make_bridge_region` variants similarly.
- **`docs/05_module_sdk.md:445-624`**: structural rewrite — rename `## slicer-test Crate` → `## Test Support (slicer-sdk feature)`; replace every `use slicer_test::*` with `use slicer_sdk::test_prelude::*` (paired with `use slicer_sdk::prelude::*` in test examples); open the section with the `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` convention note.
- **`docs/00_project_overview.md:122-156`**: remove the `slicer-test` row from the crate table and the corresponding `slicer-test/` line from the directory tree.
- **Project root `CLAUDE.md`**: scan and remove any `slicer-test` references; the docs no longer describe such a crate.

## Out of Scope

- Migrating any core-module beyond the two exemplars (deferred to packet 79).
- Extending builders for `WallLoop` / `SeamCandidate` / `PrintEntity` / `LayerCollectionIR` / `ToolChange` (packet 79).
- Relocating any runtime-located module test (packet 80).
- Modifying `crates/slicer-sdk/src/host.rs` thread-locals (unchanged since well before packet 77).
- Adding new `test_support` capabilities. This is a fold + delete + exemplar migration; no API additions beyond the existing `slicer-test` surface.
- Touching `OrcaSlicerDocumented/`. No parity concerns.
- Touching `crates/slicer-runtime/test-guests/` (those test guests share infrastructure but live in a separate target dir; not affected by the `slicer-test` deletion since none of them depend on it).
- Updating `crates/slicer-sdk/src/prelude.rs` to re-export test items — explicitly rejected per the grilling decision; production prelude stays test-free, `test_prelude` is separate.

## Authoritative Docs

- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (created in packet 77) — the decision this packet implements. **Quote its `## Decision` paragraph verbatim** in the rewritten `docs/05` test-support section.
- `docs/05_module_sdk.md:445-624` — subject of the structural rewrite. > 600 line file; read only this range via offset loads.
- `docs/00_project_overview.md:122-156` — subject of crate-inventory updates. ≈ 165 lines total; safe to read directly.
- `CLAUDE.md` (project root) — scan-and-update for `slicer-test` mentions.
- `crates/slicer-sdk/src/lib.rs` — module list (≈ 49 lines as of packet 77's end; small file, direct read).
- `crates/slicer-test/src/lib.rs` (≈ 13 lines) — confirm what is currently exported before moving.

## OrcaSlicer Reference Obligations

None. This packet does not borrow or check parity against any OrcaSlicer code. The fold is an internal SDK reorganization.

## Acceptance Summary

Acceptance Criteria are defined in `packet.spec.md` and referenced by ID. Measurable refinements:

- **AC-1 refinement**: the 30→29 member count is the corrected baseline (an earlier recon recorded 28; spec-review re-verified 30 current members on 2026-05-31). The verification command counts entries via `awk` + `grep` on `Cargo.toml`'s `members = [...]` block — no Python required. If a future packet adds or removes members between this refinement and execution, the implementer must recount and update the literal in AC-1's command in the same commit.
- **AC-2 refinement**: `crates/slicer-sdk/src/test_support/mod.rs` already exists from packet 77 with the four hook functions. This packet's edit ADDS submodule declarations (`pub mod mock_host;` etc.) without removing the four hook functions. AC-2's verification implicitly relies on packet 77's prior outcome.
- **AC-3 refinement**: `test_prelude.rs`'s first line is a whole-module gate (`#![cfg(any(test, feature = "test"))]`), NOT per-item gates inside an existing prelude — this is a deliberate ergonomic choice per the grilling decision (preludes whose contents depend on build config break IDE jump-to-definition).
- **AC-6 refinement**: the test for scaffold shape (`cargo test -p pnp-cli --test module_new_tdd`) must include at least one assertion that the generated `[dependencies]` line has no `features` field, OR that production `cargo check --target wasm32-unknown-unknown` against a freshly scaffolded module succeeds. Either is acceptable; the latter is stronger.
- **AC-7 / AC-8 refinement**: "no `make_*` helpers retained" means the `fn make_<name>(...)` shells may remain (for call-site readability — `make_config(0.2, 30.0)` is more readable than the equivalent builder chain inline 12 times) but the body must be a single-expression builder chain (`ConfigViewBuilder::new().float("infill_density", density).float("infill_angle", angle).build()`), NOT the original multi-line `let mut fields = HashMap::new(); ...` construction. The verification command's `wc -l` threshold (`> 4`) catches multi-line bodies.
- **AC-10 refinement**: the `## Test Support` section's opening lines must include a one-sentence reference to ADR-0004 so future readers can find the decision rationale.

## Verification Commands

| AC | Command | Delegation hint |
|---|---|---|
| AC-1 | `bash -c 'test ! -d crates/slicer-test && ! grep -q "\"crates/slicer-test\"" Cargo.toml && [ "$(awk ... Cargo.toml | grep -cE "^[[:space:]]*\"[^\"]+\"")" = "29" ]'` | Bash-only: count via `awk` + `grep` of the `members = [...]` block (no Python). Expected count 29 (was 30). |
| AC-2 | `for f in mod.rs mock_host.rs capture.rs fixtures.rs assert_paths.rs; do test -f crates/slicer-sdk/src/test_support/$f; done && grep -qE 'cfg.*pub mod test_support' crates/slicer-sdk/src/lib.rs` | Direct file checks. |
| AC-3 | (full command in `packet.spec.md`) | Direct head+grep loops; expect both negative grep (prelude.rs doesn't carry test items) and positive grep (every required symbol present in test_prelude.rs). |
| AC-4 | `rustup target list --installed \| grep -q wasm32 && cargo check --target wasm32-unknown-unknown -p arachne-perimeters -p rectilinear-infill && cargo tree --target wasm32-unknown-unknown -p arachne-perimeters` | Delegate cargo invocations; assert the `cargo tree` output contains `slicer-sdk` WITHOUT `feature="test"`. |
| AC-5 | `bash -c 'set -e; BAK=$(mktemp); cp .../lib.rs "$BAK"; trap ... EXIT; printf "...use ::MockHost as _gate_probe;" >> .../lib.rs; cargo check -p slicer-sdk 2>&1 \| grep -qE "E0433\|could not find .test_support."'` | Scripted probe with `mktemp` backup + `trap` restore. Dispatch as FACT pass/fail. Replaces the prior manual-only marker. |
| AC-6 | `cargo test -p pnp-cli --test module_new_tdd && grep -qE 'slicer-sdk = .*features = \[.*"test".*\]' crates/pnp-cli/src/module_new.rs && ! grep -qE 'slicer-test' crates/pnp-cli/src/module_new.rs` | Delegate test run; grep direct. |
| AC-7 | (compound command in `packet.spec.md`) | Delegate `cargo test`; the grep+awk multi-line-body check runs locally. |
| AC-8 | (compound command in `packet.spec.md`) | Same pattern. |
| AC-9 | `for m in arachne-perimeters rectilinear-infill; do grep -A5 '\[dev-dependencies\]' modules/core-modules/$m/Cargo.toml \| grep -qE 'slicer-sdk.*features = \[.*"test".*\]'; done` | Direct. |
| AC-10 | `! grep -qE '^## slicer-test Crate' docs/05_module_sdk.md && grep -qE '^## Test Support' docs/05_module_sdk.md && ! grep -qE 'use slicer_test::' docs/05_module_sdk.md && grep -qE 'use slicer_sdk::test_prelude' docs/05_module_sdk.md` | Direct. |
| AC-11 | `! grep -qE 'slicer-test' docs/00_project_overview.md` | Direct. |
| AC-12 | `bash -c '[ "$(grep -c slicer-test CLAUDE.md)" = "0" ]'` | Direct. |
| AC-N1 | `bash -c 'cargo build --target wasm32-unknown-unknown --release -p arachne-perimeters && cargo build --target wasm32-unknown-unknown --release -p rectilinear-infill && ! grep -aE "test_support::(mock_host\|capture\|fixtures\|assert_paths)" target/wasm32-unknown-unknown/release/arachne_perimeters.wasm target/wasm32-unknown-unknown/release/rectilinear_infill.wasm'` | Replaces the prior `nm` approach (binutils not on Windows by default). `grep -a` scans the wasm artifact's embedded symbol strings. |
| AC-N2 | `bash -c 'set -e; BAK=$(mktemp); cp .../Cargo.toml "$BAK"; trap ... EXIT; sed -i.tmp "s/, *features = \[\"test\"\]//g" .../Cargo.toml; cargo test -p arachne-perimeters 2>&1 \| grep -qE "MockHost\|ConfigViewBuilder\|test_prelude\|unresolved\|E0432\|E0433\|cannot find"'` | Scripted mutation with `mktemp` backup + `trap` restore. Replaces the prior manual-only marker. |
| Closure: workspace check | `cargo check --workspace --all-targets` | Delegate. |
| Closure: clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Delegate. |
| Closure: targeted test sweep | `cargo test -p slicer-sdk -p arachne-perimeters -p rectilinear-infill -p pnp-cli --test module_new_tdd` | Delegate; expect 4 package green. |
| Closure: guest staleness | `cargo xtask build-guests --check` then rebuild if STALE | Delegate; the workspace-member removal may trigger a fresh resolver pass that re-touches guest source paths. |

## Step Completion Expectations

The packet has a load-bearing ordering constraint that `implementation-plan.md`'s per-step preconditions cannot fully express: **all source moves (Step 3) MUST land green before Step 4 (delete `crates/slicer-test`) begins, AND Step 4 MUST land green before Step 9 (exemplar migrations) begins**. Reordering would leave the `slicer-test` crate exposed to inconsistent state — for example, deleting it before all consumers (including `pnp_cli` scaffold) are updated to reference `slicer-sdk::test_prelude` instead would produce a transient broken commit. The `implementation-plan.md` per-step exit conditions enforce this serialization; do not parallelize steps 3-9.

## Context Discipline Notes

Packet-specific cautions (workspace-wide discipline lives in the `context-discipline` snippet in `packet.spec.md`):

- `docs/05_module_sdk.md` is large (> 600 lines); read only lines 445-624 directly. The structural rewrite in this packet replaces a ≈ 180-line section — the read scope is bounded.
- The two exemplar core-modules' test directories may contain multiple `.rs` files (recon at packet generation showed both modules have multiple TDD files). Read each file's `make_*` helper definitions and ALL call sites of those helpers before rewriting the helper body — a builder chain that omits a setter the original constructor populated is an assertion-weakening bug.
- The `pnp_cli module new` scaffold change touches both the source string template (`crates/pnp-cli/src/module_new.rs:188-207`) and the assertions (`crates/pnp-cli/src/module_new.rs:545` + `crates/pnp-cli/tests/module_new_tdd.rs:36`). Both edit sites must change in lockstep — verify by running `cargo test -p pnp-cli --test module_new_tdd` after the production edit, observing the assertion failure, then updating the assertion.
- The workspace-member removal (Step 4) forces a `Cargo.lock` regeneration. Commit the lockfile change in the same commit as the member-list edit to avoid spurious diffs in subsequent steps.
- Guest WASM staleness: this packet does not directly touch WIT or macro text, but the workspace-member change may re-trigger the build script's input-set computation. Run `cargo xtask build-guests --check` at the closure gate and rebuild if stale.
