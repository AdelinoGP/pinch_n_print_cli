---
status: implemented
packet: 77
task_ids: [TASK-223, TASK-224]
closed: 2026-05-31
backlog_source: docs/07_implementation_status.md
---

# Packet 77 — Wire `#[module_test]` and Make `MockHost` a Real Adapter

## Goal

Close the `#[module_test]` phantom-import contract by introducing a feature-gated `slicer-sdk::test_support` module that provides the four hooks the macro names, rewriting the macro to call them fully-qualified, and refactoring `MockHost` into a real `slicer_sdk::host::MeshSource` adapter — so that the documented test-support story in `docs/05_module_sdk.md` is finally implementable without phantom unqualified identifiers.

## Scope Boundaries

This packet is self-contained: no source files move between crates, no core-module tests migrate, and `slicer-test` is **not** deleted. The macro's expansion changes, the `slicer-sdk` crate gains a feature-gated `test_support` module that wraps the existing thread-local seam in `host::test_support`, `MockHost` is rewritten to implement `MeshSource` with builder-style chaining, and the surrounding `docs/05` API fictions (per-arg `set_raycast_z_down`, non-existent `circle_polygon` / `rect_polygon` / `path_length` / `with_logging` / `clip_polygons_call_count`) are corrected so the documentation describes what compiles. ADR-0004 records the decision direction for packets 78–80. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- None — this is the first packet of the 77–80 sequence.
- Does **not** block on guest WASM rebuild for its own acceptance, but the macro change does affect bindgen text, so the closure gate runs `cargo xtask build-guests --check` (and rebuilds if stale) before declaring done.

## Acceptance Criteria

### AC-1 — `slicer-sdk` declares the `test` feature

**Given** a fresh checkout after this packet,
**When** `cat crates/slicer-sdk/Cargo.toml` is inspected,
**Then** it contains a `[features]` section with `test = []` (empty default features list; no other entries unless this packet adds them).

| `grep -A1 '^\[features\]' crates/slicer-sdk/Cargo.toml | grep -qE '^test = \[\]$'`

### AC-2 — `slicer-sdk::test_support` module exists, gated, and exposes the four hooks

**Given** the feature flag from AC-1,
**When** the workspace is built with `cargo check -p slicer-sdk --features test --tests`,
**Then** the path `slicer_sdk::test_support` resolves to a module whose public surface includes exactly the four free functions `reset_global_state()`, `install_panic_handler()`, `mock_host_setup()`, `mock_host_teardown()`, each returning `()`, and the module is `#[cfg(any(test, feature = "test"))]`-gated so it is unreachable from `cargo check -p slicer-sdk` (no features).

| `cargo check -p slicer-sdk --features test --tests 2>&1 | grep -q "error" && exit 1; cargo check -p slicer-sdk 2>&1 | grep -q "test_support" && exit 1; exit 0`

### AC-3 — `#[module_test]` expansion emits fully-qualified `::slicer_sdk::test_support::*` calls

**Given** the macro at `crates/slicer-macros/src/lib.rs:2705-2756`,
**When** the expanded token stream for a no-return-type `#[module_test] fn t() {}` is inspected (via `cargo expand -p slicer-macros --tests` or by reading the rewritten source),
**Then** the expansion contains exactly four call sites — `::slicer_sdk::test_support::reset_global_state()`, `::slicer_sdk::test_support::install_panic_handler()`, `::slicer_sdk::test_support::mock_host_setup()`, and `::slicer_sdk::test_support::mock_host_teardown()` (inside the `Drop` guard) — and contains **zero** occurrences of the bare unqualified identifiers `__slicer_test_reset_global_state`, `__slicer_test_install_panic_handler`, `__slicer_test_mock_host_setup`, or `__slicer_test_mock_host_teardown`.

| `grep -nE 'slicer_test_(reset_global_state|install_panic_handler|mock_host_(setup|teardown))' crates/slicer-macros/src/lib.rs | grep -vE 'slicer_sdk::test_support' && exit 1; exit 0`

### AC-4 — `MockHost` implements `slicer_sdk::host::MeshSource` with builder-style chaining

**Given** the refactored `crates/slicer-test/src/mock_host.rs`,
**When** the file is inspected,
**Then** it contains `impl slicer_sdk::host::MeshSource for MockHost`, the impl defines `raycast_z_down`, `surface_normal_at`, and `object_bounds` returning the configured value (or `None`/`Err` for `object_bounds` when not set), the `MockHost` type carries fields `raycast_hit: Option<f32>`, `normal: Option<slicer_ir::Point3>`, `bounds: Option<slicer_ir::BoundingBox3>`, and exposes consuming builder methods `with_raycast_hit(self, Option<f32>) -> Self`, `with_normal(self, Option<Point3>) -> Self`, `with_object_bounds(self, BoundingBox3) -> Self`, and `install(self)` that calls `slicer_sdk::host::test_support::install_mesh_source(self)`. No `HashMap<(String, f32, f32, f32), _>` field is present.

| `cargo check -p slicer-test --tests && grep -qE 'impl slicer_sdk::host::MeshSource for MockHost' crates/slicer-test/src/mock_host.rs && grep -qE 'fn with_raycast_hit' crates/slicer-test/src/mock_host.rs && grep -qE 'fn install\(self\)' crates/slicer-test/src/mock_host.rs`

### AC-5 — `MockHost` round-trip through `slicer_sdk::host::raycast_z_down`

**Given** the new test file `crates/slicer-test/tests/mock_host_adapter_tdd.rs`,
**When** `cargo test -p slicer-test --test mock_host_adapter_tdd` runs,
**Then** the test `mock_host_install_routes_raycast_through_host_wrapper` passes: it constructs `MockHost::new().with_raycast_hit(Some(4.8)).install()`, calls `slicer_sdk::host::raycast_z_down("obj-x", 1.0, 2.0, 5.0)`, and asserts the return value is exactly `Some(4.8)` (f32 equality on the configured constant; no tolerance needed).

| `cargo test -p slicer-test --test mock_host_adapter_tdd -- mock_host_install_routes_raycast_through_host_wrapper`

### AC-6 — `reset_global_state` clears prior-test mesh source between `#[module_test]`s

**Given** the new test file `crates/slicer-test/tests/mock_host_isolation_tdd.rs`,
**When** `cargo test -p slicer-test --test mock_host_isolation_tdd` runs,
**Then** the test sequence (a first `#[module_test]` installs `MockHost::new().with_raycast_hit(Some(7.0)).install()` and exits without uninstalling; a second `#[module_test]` calls `slicer_sdk::host::raycast_z_down("obj-x", 0.0, 0.0, 0.0)` and asserts the result is `None`) passes, proving that the macro's `reset_global_state` (called at entry of the second test) cleared the prior mesh source via `host::test_support::clear_mesh_source()`.

| `cargo test -p slicer-test --test mock_host_isolation_tdd`

### AC-7 — `install_panic_handler` chains an existing hook and drains captured logs on panic

**Given** the new test file `crates/slicer-test/tests/panic_handler_drains_logs_tdd.rs`,
**When** `cargo test -p slicer-test --test panic_handler_drains_logs_tdd` runs,
**Then** the test `panic_inside_module_test_drains_log_buffer_to_stderr` (which uses `std::panic::catch_unwind` to invoke a closure that emits `host::log_warn("BUG_MARKER_42")` and then panics) passes by asserting that (a) the panic was caught, (b) `host::test_support::take_log_messages()` returns empty (the panic hook drained), AND (c) a prior `std::panic::set_hook` invoked before `install_panic_handler` is still called (verified via a thread-local flag flipped by the prior hook) — proving the chain is preserved.

| `cargo test -p slicer-test --test panic_handler_drains_logs_tdd`

### AC-8 — `slicer-macros` smoke + module_test_tdd compile and pass against the new expansion

**Given** the rewritten macro and the updated `crates/slicer-macros/tests/smoke.rs` + `crates/slicer-macros/tests/module_test_tdd.rs` (which previously defined local `__slicer_test_*` stubs and now depend on `slicer-sdk` with `features = ["test"]`),
**When** `cargo test -p slicer-macros --test smoke --test module_test_tdd` runs,
**Then** all tests pass and the test files contain **zero** definitions of `pub fn __slicer_test_mock_host_setup`, `pub fn __slicer_test_mock_host_teardown`, `pub fn __slicer_test_install_panic_handler`, or `pub fn __slicer_test_reset_global_state` (the local stubs are gone because the macro no longer requires them).

| `cargo test -p slicer-macros --test smoke --test module_test_tdd && grep -qE 'fn __slicer_test_(mock_host_setup|mock_host_teardown|install_panic_handler|reset_global_state)' crates/slicer-macros/tests/smoke.rs crates/slicer-macros/tests/module_test_tdd.rs && exit 1; exit 0`

### AC-9 — `docs/05_module_sdk.md` carries no fictional API references

**Given** the doc cleanup in this packet,
**When** `docs/05_module_sdk.md` is grepped,
**Then** lines 445–624 contain **zero** matches for the strings `circle_polygon`, `rect_polygon`, `path_length(`, `with_logging`, `clip_polygons_call_count`, or `region_id("` (with literal opening double-quote, indicating the string-typed signature). The replacement examples instead use `square_polygon`, `rect_path`, `region_id(42)`, and the new `MockHost::new().with_raycast_hit(Some(4.8))...install()` shape.

| `awk 'NR>=445 && NR<=624' docs/05_module_sdk.md | grep -nE 'circle_polygon|rect_polygon|path_length\(|with_logging|clip_polygons_call_count|region_id\("' && exit 1; exit 0`

### AC-10 — ADR-0004 records the fold decision

**Given** this packet,
**When** `ls docs/adr/0004-test-support-lives-in-slicer-sdk.md` is checked,
**Then** the file exists, contains a `## Status` section reading `Accepted`, a `## Context` section explaining the redundant test seam, a `## Decision` section naming `slicer_sdk::test_support` + `slicer_sdk::test_prelude` (the latter to be created in packet 78), a `## Consequences` section, and a `## Alternatives Considered` section that names at minimum the three alternatives `rename slicer-test to slicer-sdk-test`, `delete slicer-test outright`, and `keep slicer-test as a separate crate but wire the macro to it`, each with a one-line reason for rejection.

| `test -f docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE '^## Status$' docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE 'rename slicer-test to slicer-sdk-test' docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE 'delete slicer-test outright' docs/adr/0004-test-support-lives-in-slicer-sdk.md`

## Negative Test Cases

### AC-N1 — Production `cargo check -p slicer-sdk` (no features) refuses to reference `test_support`

**Given** the feature gate from AC-1/AC-2,
**When** `cargo check -p slicer-sdk` runs without `--features test` and a deliberate one-line probe at the top of `crates/slicer-sdk/src/lib.rs` tries `pub use crate::test_support::reset_global_state as _probe;` (added by the implementer **only** to verify the gate, then removed before merge),
**Then** the compile fails with `error[E0433]: failed to resolve: could not find \`test_support\`` (or equivalent — module not found), proving the gate is real and not paper-thin.

| (Implementer-run during step verification, not in CI. Documented procedure in `implementation-plan.md` step "Verify gate is real".)

### AC-N2 — `MockHost::new().install()` followed by `MockHost::new().install()` does not stack mesh sources

**Given** the install/uninstall contract,
**When** a test installs MockHost A (raycast=Some(1.0)), then without explicit uninstall installs MockHost B (raycast=Some(2.0)), then calls `raycast_z_down`,
**Then** the result is `Some(2.0)` (the second install replaced the first) — verified via a regression test `mock_host_second_install_replaces_first` in `crates/slicer-test/tests/mock_host_adapter_tdd.rs`. This documents that `install_mesh_source` is replace-semantics, not stack-semantics.

| `cargo test -p slicer-test --test mock_host_adapter_tdd -- mock_host_second_install_replaces_first`

## Verification (gate commands only)

Closure of this packet requires:

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-test -p slicer-macros -p slicer-sdk`

Full per-AC verification matrix and delegation hints live in `requirements.md`.

## Authoritative Docs

- `docs/05_module_sdk.md` — the `slicer-test` and `#[module_test]` sections being repaired (lines 445–624).
- `docs/01_system_architecture.md` — module-host boundary; SDK as the module-authoring crate.
- `CLAUDE.md` — project-root test discipline (narrow tests; `cargo xtask build-guests --check` after WIT/macro changes).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (created by this packet) — fold decision.

## Doc Impact Statement

`docs/05_module_sdk.md` lines 445–624 lose every fictional API reference and gain examples that match the new `MockHost` builder API. `docs/adr/0004-test-support-lives-in-slicer-sdk.md` is created. `docs/00_project_overview.md` is **not** modified by this packet (the crate row for `slicer-test` is removed in packet 78 when the crate is deleted).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Implementation Notes

Recorded at closure (2026-05-31). Two erratta against the spec text, neither changing the contract:

- **AC-6 restructure.** The spec's two-`#[module_test]` structure (first test installs and exits dirty; second test asserts `None`) was found tautological during implementation. Rust's test runner pools threads, and `MESH_SOURCE` is a per-thread `RefCell` thread-local; a dirty exit on thread A is invisible to thread B regardless of which test runs first and regardless of whether `reset_global_state` fires. The second test would observe `None` even if `reset_global_state` were a no-op. `mock_host_isolation_tdd.rs` was therefore restructured: a single test installs a `MockHost`, calls `slicer_sdk::test_support::reset_global_state()` inside its own body, then asserts the post-reset `raycast_z_down` returns `None`. This strictly strengthens the verification — it now exercises `reset_global_state`'s `clear_mesh_source()` call directly rather than relying on cross-test thread-local leakage that cannot occur. The contract being locked ("at every `#[module_test]` entry, the mesh source is cleared regardless of prior state") is preserved; the assertion is sharper.
- **AC-N1 error code.** The probe captured `error[E0432]: unresolved import` rather than the spec text's `error[E0433]: failed to resolve`. Both prove the same thing (the gate is real — `test_support` is not reachable from non-feature code). The AC text itself admits "or equivalent — module not found".

One scope expansion not in the Step-4 EDIT list:

- `crates/slicer-test/tests/smoke.rs` had to be updated in Step 4 because an existing smoke test called the deleted pre-Step-4 `MockHost` API (`enable_logging`, two-arg `log_contains(LogLevel, &str)`, `mock_host::LogLevel` re-export). Without the update, `cargo check -p slicer-test --tests` (Step 4's gating verify) could not pass. Patch was minimal: install the log capture sink and switch to the new single-arg associated-fn `MockHost::log_contains(needle)`.
