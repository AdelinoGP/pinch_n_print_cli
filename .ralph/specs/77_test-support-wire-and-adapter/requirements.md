# Requirements — Packet 77

## Packet Metadata

- **Packet**: 77
- **Slug**: `77_test-support-wire-and-adapter`
- **Status**: draft
- **Task IDs**: TASK-223, TASK-224
- **Backlog source**: `docs/07_implementation_status.md`

## Problem Statement

The project's documented module-testing story is fictional. `docs/05_module_sdk.md:582-624` describes `#[module_test]` as a wrapper that "automatically sets up the mock host, installs the SDK's test panic handler, and resets global state between tests" — but the macro at `crates/slicer-macros/src/lib.rs:2705-2756` emits **unqualified** calls to four `__slicer_test_*` identifiers that nothing in the workspace defines. The only places those names exist are local stub functions inside the macro's own test files (`crates/slicer-macros/tests/{smoke,module_test_tdd}.rs`), which exist precisely to make the macro compile in isolation. Any module that tried to use `#[module_test]` for real would fail to compile.

Meanwhile, `slicer-sdk` already owns a working primitive test seam — `crates/slicer-sdk/src/host.rs:320-343` exposes `pub mod test_support` with `install_log_capture` / `take_log_messages` / `install_mesh_source` / `clear_mesh_source`, backed by two thread-locals (`LOG_CAPTURE`, `MESH_SOURCE`). It is genuinely used by `crates/slicer-sdk/tests/host_wrappers_tdd.rs` and one runtime executor test (`crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs:29,461-518`). But `MockHost` in `crates/slicer-test/src/mock_host.rs` knows nothing about either thread-local; it carries its own parallel `logs: Vec<LogEntry>` and `call_counts: HashMap<String, usize>` fields, divorced from the real host wrapper code.

The documentation also describes APIs that don't exist (`set_raycast_z_down` with per-arg signature, `with_logging`, `clip_polygons_call_count`, `circle_polygon`, `rect_polygon`, `path_length`, `slicer_test::prelude`). Future agents reading the docs see a richer test surface than the codebase provides, then either work around the gap or push tests up into `slicer-runtime/tests/` where the full host machinery is available (which is exactly what happened with `wipe_tower_bed_bounds.rs` and `prepass_support_generation_orca_parity_tdd.rs` — relocated in packet 80).

This packet closes the gap **without moving any files**. The macro's expansion changes to fully-qualified `::slicer_sdk::test_support::*` calls. The SDK gains a feature-gated `test_support` mod that provides the four hooks the macro names, routing through the existing `host::test_support` thread-locals. `MockHost` is rewritten as a real `MeshSource` adapter with builder-style chaining matching the in-tree `StubMesh` precedent (`host_wrappers_tdd.rs:67-87`). Doc fictions are excised. ADR-0004 records the convergence direction so packets 78–80 don't re-litigate.

This packet keeps the `slicer-test` crate alive on purpose. The fold (move source files into `slicer-sdk`, delete the crate, expose via `slicer_sdk::test_prelude`) is packet 78's responsibility; doing both in one packet would couple two reversible changes into one large blast radius.

## In Scope

- `crates/slicer-sdk/Cargo.toml` — add `[features] test = []`.
- `crates/slicer-sdk/src/test_support/mod.rs` — new file, gated `#[cfg(any(test, feature = "test"))]`. Provides `reset_global_state`, `install_panic_handler`, `mock_host_setup`, `mock_host_teardown`.
- `crates/slicer-sdk/src/lib.rs:20-31` — register the new submodule under the feature gate.
- `crates/slicer-macros/src/lib.rs:2705-2756` — rewrite `#[module_test]` expansion to emit fully-qualified `::slicer_sdk::test_support::*` calls.
- `crates/slicer-test/src/mock_host.rs` — rewrite `MockHost` as a `MeshSource` adapter with builder-style chaining; add `install` / `uninstall` methods; route `log_warn` / `log_contains` through `host::log_warn` / `host::test_support::take_log_messages`.
- `crates/slicer-test/src/lib.rs` — add `pub mod prelude` re-exporting `MockHost`, `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder`, `square_polygon`, `rect_path`, `InfillOutputCapture`, `PerimeterOutputCapture`, `SupportOutputCapture`, all `assert_paths_*`.
- `crates/slicer-test/Cargo.toml` — change `slicer-sdk` dep to `features = ["test"]`.
- `crates/slicer-test/tests/mock_host_adapter_tdd.rs` — new file.
- `crates/slicer-test/tests/mock_host_isolation_tdd.rs` — new file.
- `crates/slicer-test/tests/log_capture_round_trip_tdd.rs` — new file.
- `crates/slicer-test/tests/panic_handler_drains_logs_tdd.rs` — new file.
- `crates/slicer-macros/Cargo.toml` — add `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }` to `[dev-dependencies]`.
- `crates/slicer-macros/tests/smoke.rs` (lines 16-28) — delete the local `__slicer_test_*` stub functions; replace tracking-flag assertions with observable-behavior assertions.
- `crates/slicer-macros/tests/module_test_tdd.rs` (lines 38-59 + related body assertions) — same.
- `docs/05_module_sdk.md` (lines 445-624) — remove fictional API references; update examples to match the new `MockHost` builder API. Defer the structural section rename (`slicer-test Crate` → `Test Support`) to packet 78.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — new ADR; status `Accepted`.

## Out of Scope

- Moving any source files between crates (deferred to packet 78).
- Renaming `slicer-test` (rejected alternative; ADR-0004 records why).
- Deleting `slicer-test` (deferred to packet 78).
- Adding `slicer_sdk::test_prelude` (introduced in packet 78).
- Migrating any core-module test (packet 79).
- Relocating any runtime-located module tests (packet 80).
- Touching `crates/slicer-sdk/src/host.rs` thread-locals — they already work and are reused as-is.
- Adding per-call-arg keyed mocking. The `MockHost` API is one-answer-per-query (matches the `StubMesh` precedent at `crates/slicer-sdk/tests/host_wrappers_tdd.rs:67-87`); per-arg keying is rejected because `f32: !Hash`.
- Removing `MockHost::record_call` / `call_count` / `assert_call_count` — kept (independent of host state; "was X called" is a useful axis).
- Changing the `host::test_support` API surface (its functions stay byte-identical; this packet only adds a layer above them).
- Updating `docs/00_project_overview.md` crate inventory — done in packet 78 when the crate is deleted.

## Authoritative Docs

- `docs/05_module_sdk.md` — §`slicer-test Crate` and §`#[module_test]` (lines 445-624) are the subject of the cleanup. **Size note**: file > 600 lines; delegate any cross-reference reads with a tight line range.
- `docs/01_system_architecture.md` — module-host boundary context (lines 1-200 sufficient).
- `docs/adr/0001-prepass-builtins-commit-in-stage.md`, `0002-wit-marshalling-type-unification.md`, `0003-macro-per-world-wit-conversions.md` — read only to match the ADR-0004 format/style. **Delegate** if total size exceeds 300 lines.
- `CLAUDE.md` (project root) — §Test Discipline and §Guest WASM Staleness.

## Acceptance Summary

Acceptance Criteria are defined in `packet.spec.md` and referenced here by ID only. Measurable refinements that didn't fit the Given/When/Then templates:

- **AC-3 refinement**: the macro file at `crates/slicer-macros/src/lib.rs` post-packet contains **exactly two** `quote! { ... }` blocks for `#[module_test]` (the `has_return_type` branch and the unit branch), each emitting the four fully-qualified calls in the same order: `reset_global_state` first, then `install_panic_handler`, then `mock_host_setup`, then `mock_host_teardown` (in the `Drop`). Order is contract — `reset_global_state` must run before `install_panic_handler` so a leftover panic hook from a prior test is cleared before the new chain is installed.
- **AC-4 refinement**: `MockHost` must remain `Default`-derivable (or have a `Default` impl returning the all-`None` state) and must remain `Send + Sync` (because `host::test_support::install_mesh_source` requires `MeshSource: Send + Sync + 'static`).
- **AC-7 refinement**: the panic-hook chain implementation must use `std::panic::take_hook()` before `std::panic::set_hook(Box::new(...))` so the previous hook is preserved and called after the log-drain prints. If the previous hook is the Rust default test hook (the most common case), this preserves the standard panic banner and backtrace output.
- **AC-10 refinement**: ADR-0004's `## Status` section must include the date of acceptance (today's date as committed) so future audits can sequence it against the other ADRs.

## Verification Commands

Full per-AC matrix. Delegation hints assume the implementer is dispatching `cargo` calls to sub-agents returning `FACT: pass/fail + ≤ 5 lines of context` rather than absorbing stdout.

| AC | Command | Delegation hint |
|---|---|---|
| AC-1 | `grep -A1 '^\[features\]' crates/slicer-sdk/Cargo.toml \| grep -qE '^test = \[\]$'` | Direct; output is exit code only. |
| AC-2 | `cargo check -p slicer-sdk --features test --tests && cargo check -p slicer-sdk` | Delegate both `cargo check` runs; return `FACT: both clean / which failed`. |
| AC-3 | `grep -nE 'slicer_test_(reset_global_state\|install_panic_handler\|mock_host_(setup\|teardown))' crates/slicer-macros/src/lib.rs \| grep -vE 'slicer_sdk::test_support'` | Direct grep; expect empty output. |
| AC-4 | `cargo check -p slicer-test --tests && grep -qE 'impl slicer_sdk::host::MeshSource for MockHost' crates/slicer-test/src/mock_host.rs` | Delegate `cargo check`. |
| AC-5 | `cargo test -p slicer-test --test mock_host_adapter_tdd -- mock_host_install_routes_raycast_through_host_wrapper` | Delegate; return failing assertion if red. |
| AC-6 | `cargo test -p slicer-test --test mock_host_isolation_tdd` | Delegate; this file has exactly two `#[module_test]` functions wired to verify isolation. |
| AC-7 | `cargo test -p slicer-test --test panic_handler_drains_logs_tdd` | Delegate; uses `catch_unwind` so doesn't actually fail the test runner. |
| AC-8 | `cargo test -p slicer-macros --test smoke --test module_test_tdd && grep -qE 'fn __slicer_test_' crates/slicer-macros/tests/{smoke,module_test_tdd}.rs && exit 1; exit 0` | Delegate `cargo test`; grep direct. |
| AC-9 | `awk 'NR>=445 && NR<=624' docs/05_module_sdk.md \| grep -nE 'circle_polygon\|rect_polygon\|path_length\(\|with_logging\|clip_polygons_call_count\|region_id\("'` | Direct; expect empty. |
| AC-10 | `test -f docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE '^## Status$' ... && grep -qE 'rename slicer-test to slicer-sdk-test\|delete slicer-test outright' ...` | Direct; sequential grep. |
| AC-N1 | Manual implementer step — see `implementation-plan.md` Step "Verify gate is real". Not CI-gated. | Implementer documents the temporary probe + result. |
| AC-N2 | `cargo test -p slicer-test --test mock_host_adapter_tdd -- mock_host_second_install_replaces_first` | Delegate. |
| Closure: workspace check | `cargo check --workspace --all-targets` | Delegate; return `FACT: clean / first error`. |
| Closure: workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Delegate; return `FACT: clean / first violation`. |
| Closure: targeted test sweep | `cargo test -p slicer-test -p slicer-macros -p slicer-sdk` | Delegate; return `FACT: counts pass/fail per package`. |
| Closure: guest staleness | `cargo xtask build-guests --check` then rebuild (without `--check`) if `STALE:` is reported | Delegate; macro change affects bindgen text so guests will be stale and **must** be rebuilt before the test sweep. |

## Step Completion Expectations

None. The packet is small enough that per-step preconditions and exit conditions in `implementation-plan.md` cover everything; no cross-step invariants are needed here.

## Context Discipline Notes

Packet-specific cautions (workspace-wide discipline lives in the `context-discipline` snippet in `packet.spec.md`):

- `docs/05_module_sdk.md` is large (> 600 lines). Read **only** the line range relevant to each cleanup edit (445-624 for the §`slicer-test` block; 582-624 for the `#[module_test]` block). Do not load the whole file.
- `crates/slicer-macros/src/lib.rs` is large (> 2750 lines). The edit target is lines 2705-2756. Never load the whole file; jump straight to that range using offset reads.
- `crates/slicer-sdk/src/host.rs` (line 320 onward defines the existing `test_support` mod) need not be edited; read only to confirm function signatures before wrapping them.
- The macro change affects bindgen-generated text for every guest. After step 3, run `cargo xtask build-guests --check`; rebuild if stale. Do not skip this — guest test failures otherwise look unrelated.
