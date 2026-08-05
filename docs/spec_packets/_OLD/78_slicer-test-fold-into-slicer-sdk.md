---
status: implemented
packet: 78
task_ids: [TASK-225, TASK-226]
---

# 78_slicer-test-fold-into-slicer-sdk

## Goal

Consolidate every test helper under `slicer_sdk::test_support` behind feature `test`, expose them via a new `slicer_sdk::test_prelude` distinct from `slicer_sdk::prelude`, delete the `slicer-test` crate, update the `pnp_cli module new` scaffold to emit the single feature-gated dev-dep, and migrate `arachne-perimeters` + `rectilinear-infill` as exemplars whose `make_*` helpers prove the consolidated builder API covers the diverse config/region permutations the wider migration in packet 79 will rely on.

## Problem Statement

After packet 77 closes the `#[module_test]` phantom-import contract, two parallel test-support surfaces still exist: `slicer_sdk::test_support` (where the four hook functions live, gated by the new `test` feature) and `crates/slicer-test/` (the standalone crate still housing `MockHost`, `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder`, `square_polygon`, `rect_path`, the three output capture types, and the five `assert_paths_*` helpers). The split is now purely historical — the crate has zero non-self consumers across the workspace's 28 members, its only users are its own internal tests and the `pnp_cli module new` scaffold output, and the documented API surface (`docs/05_module_sdk.md:445-624`) treats `slicer-test` and `slicer-sdk` as one feature anyway. Future agents still see the dual structure and burn time deciding which crate owns what.

This packet executes the architectural decision recorded in ADR-0004 (packet 77): move every source file from `crates/slicer-test/src/*` into `crates/slicer-sdk/src/test_support/` under the existing whole-module `#[cfg(any(test, feature = "test"))]` gate; move every test file under the `test_support_*` prefix into `crates/slicer-sdk/tests/`; introduce a separate `crates/slicer-sdk/src/test_prelude.rs` (whole-module gated, not feature-gated per-item) that re-exports every test helper alongside the existing `crates/slicer-sdk/src/prelude.rs` (which stays test-free); rewrite the `pnp_cli module new` scaffold to emit a single `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` line; remove `crates/slicer-test` from the workspace member list; and delete the `crates/slicer-test/` directory.

To prove the consolidation works under realistic builder API stress before packet 79 commits to migrating 18 core-modules, two exemplar core-modules migrate their hand-rolled `make_*` helpers in this same packet: `arachne-perimeters` (six diverse helpers exercising `ConfigViewBuilder`'s `int` / `float` / `bool` / `string` accessors and `SliceRegionViewBuilder`'s polygon + infill-area permutations) and `rectilinear-infill` (six helpers including bridge-region variants). Either module surfacing a builder gap here is a signal to address it before the bulk packet; clean migrations here let packet 79 follow the same pattern with high confidence.

The `docs/05_module_sdk.md` section heading (`slicer-test Crate` → `Test Support (slicer-sdk feature)`) and `docs/00_project_overview.md` crate-inventory entries also update in this packet because they would otherwise lie about the shipped state.

## Architecture Constraints

- **Whole-module feature gate, not per-item.** `test_prelude.rs` MUST start with `#![cfg(any(test, feature = "test"))]` as line 1 (or line 2 after a doc comment). This makes the prelude either fully present or fully absent — never partial. Per-item gates inside an existing prelude (e.g., adding `#[cfg(...)] pub use ...` lines to `prelude.rs`) is explicitly rejected per the grilling decision because it makes `slicer_sdk::prelude`'s contents build-config-dependent, which breaks IDE jump-to-definition and `cargo doc`'s output.
- **Production guest wasm must NOT pull in `test_support`.** AC-4 enforces this via `cargo check --target wasm32-unknown-unknown` for both exemplar modules, plus a `cargo tree --target wasm32-unknown-unknown` assertion that the `test` feature is not activated. The mechanism: production `[dependencies] slicer-sdk = { path }` has no `features`; only `[dev-dependencies] slicer-sdk = { path, features = ["test"] }` does. Cargo activates `test` only when both dep edges are present in the build graph (i.e., during `cargo test`).
- **AC-N1's `nm`-based check is defense-in-depth.** It catches the case where someone forgets the `#[cfg]` gate on a sub-item of `test_support`. The four production-code `pub fn` hooks added in packet 77 are already inside the gated module, so they shouldn't leak — but the gate must be on the module, not just on individual functions.
- **Migration discipline: builders must preserve every original invariant.** `make_config(density: f64, angle: f64, speed: f64, line_width: f64)` produces a `ConfigView` with exactly those four keys at those values. The replacement `ConfigViewBuilder::new().float("infill_density", density).float("infill_angle", angle).float("infill_speed", speed).float("line_width", line_width).build()` produces an equivalent `ConfigView`. The implementer MUST verify field key names by reading the call sites (e.g., `gyroid-infill/src/lib.rs:14-17` uses `infill_density`, not `density` — that's an exact-match requirement for tests to pass).
- **Test file renames must update bucket aggregator `mod` statements.** `crates/slicer-sdk/tests/` may or may not have a `main.rs` aggregator (recon needed at step time). If it does, every moved test file's `mod` declaration line in `main.rs` must be added.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- **`slicer-sdk` dev-dep behavior under Cargo feature unification**: when `modules/core-modules/X/Cargo.toml` has both `[dependencies] slicer-sdk = { path }` (no features) and `[dev-dependencies] slicer-sdk = { path, features = ["test"] }`, Cargo's build graph treats these as one node. For `cargo build` (production target), only `[dependencies]` is active → `test` feature off. For `cargo test`, both edges are active → features unified → `test` feature on. This is the documented behavior; AC-4's `cargo tree --target wasm32-unknown-unknown` invocation walks only the production dep graph (no `[dev-dependencies]`), so the absence of `feature="test"` in its output is the load-bearing check.
- **`#[cfg(any(test, feature = "test"))]` vs `#[cfg(feature = "test")]`**: the former allows `cargo test -p slicer-sdk` (which adds the `test` cfg flag) to compile test_support even without the explicit feature; the latter would require `cargo test -p slicer-sdk --features test`. The `test` predicate keeps slicer-sdk's own tests ergonomic.
- **Test file renames force module_new_tdd assertion updates**: the existing assertion at `crates/pnp-cli/tests/module_new_tdd.rs:36` asserts `cargo.contains("slicer-test")`. After the rewrite it must assert `cargo.contains("slicer-sdk")` with `features = [\"test\"]`. Similarly at `module_new.rs:545`.
- **`Cargo.lock` regeneration is expected**: the workspace-member removal causes a fresh resolver pass. Lockfile diff is mechanical; commit it in the same commit as the `Cargo.toml` edit to avoid confusing subsequent step diffs.

## Locked Assumptions and Invariants

- **Invariant A**: `crates/slicer-sdk/src/test_support/mod.rs` retains the four hook functions from packet 77 (`reset_global_state`, `install_panic_handler`, `mock_host_setup`, `mock_host_teardown`) byte-identical. This packet's edit to that file is purely additive — `pub mod mock_host;` etc. The macro at `crates/slicer-macros/src/lib.rs:2705-2756` is NOT touched in this packet; its expansion remains `::slicer_sdk::test_support::*` (set in packet 77).
- **Invariant B**: `slicer_sdk::prelude` (the production prelude) does NOT re-export any test helpers. AC-3's verification includes the negative grep `! grep ... crates/slicer-sdk/src/prelude.rs` confirming this.
- **Invariant C**: The exemplar migrations preserve every test's original assertion semantics. A test that previously asserted `assert!((module.density() - 0.2).abs() < 0.001)` continues to make that exact assertion; only the fixture-construction path changes.
- **Invariant D**: The `pnp_cli module new` scaffold's `[dependencies]` line for `slicer-sdk` has NO `features` field. Only the `[dev-dependencies]` line does.

## Risks and Tradeoffs

- **Risk: `Cargo.lock` regeneration produces noisy diffs on unrelated dependencies.** Mitigation: commit `Cargo.lock` in the same commit as the workspace-member edit; subsequent commits will only show domain-relevant diffs. If the lockfile change includes upstream package versions, audit the diff briefly — but accept it as mechanical.
- **Risk: `cargo check --target wasm32-unknown-unknown` may not be a runnable verification command in CI if the wasm32 target isn't installed.** Mitigation: AC-4's command starts with `rustup target list --installed | grep -q wasm32-unknown-unknown` to skip cleanly when the target is missing. CI must install the target before running the gate (the project already builds guests via `cargo xtask build-guests`, which requires the target — so it should always be present).
- **Risk: deleting `crates/slicer-test` between AC-1 and other ACs that grep for `slicer-test` references creates a brief window where the workspace doesn't compile.** Mitigation: per the `Step Completion Expectations` ordering, ALL moves and dev-dep updates land before the directory deletion. Step 4's exit condition includes `cargo check --workspace --all-targets`; if that's not green, deletion is held.
- **Risk: an exemplar migration introduces a subtle bug because the builder defaults differ from the original constructor.** Mitigation: dispatch 3 + 4 extract the exact config keys / region field values the original `make_*` helpers populate, before rewriting; the implementer maps each field to an explicit setter call rather than relying on `ConfigViewBuilder::new().build()` defaults.
- **Tradeoff: keeping `make_*` helper shells (just with builder bodies) vs inlining at call sites.** Accepted: keep the shells when the original used a 4+ parameter constructor — readability wins. Inline when the helper was zero-parameter and only used once.
