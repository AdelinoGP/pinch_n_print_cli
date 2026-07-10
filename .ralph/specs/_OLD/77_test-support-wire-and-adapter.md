---
status: implemented
packet: 77
task_ids: [TASK-223, TASK-224]
---

# 77_test-support-wire-and-adapter

## Goal

Close the `#[module_test]` phantom-import contract by introducing a feature-gated `slicer-sdk::test_support` module that provides the four hooks the macro names, rewriting the macro to call them fully-qualified, and refactoring `MockHost` into a real `slicer_sdk::host::MeshSource` adapter — so that the documented test-support story in `docs/05_module_sdk.md` is finally implementable without phantom unqualified identifiers.

## Problem Statement

The project's documented module-testing story is fictional. `docs/05_module_sdk.md:582-624` describes `#[module_test]` as a wrapper that "automatically sets up the mock host, installs the SDK's test panic handler, and resets global state between tests" — but the macro at `crates/slicer-macros/src/lib.rs:2705-2756` emits **unqualified** calls to four `__slicer_test_*` identifiers that nothing in the workspace defines. The only places those names exist are local stub functions inside the macro's own test files (`crates/slicer-macros/tests/{smoke,module_test_tdd}.rs`), which exist precisely to make the macro compile in isolation. Any module that tried to use `#[module_test]` for real would fail to compile.

Meanwhile, `slicer-sdk` already owns a working primitive test seam — `crates/slicer-sdk/src/host.rs:320-343` exposes `pub mod test_support` with `install_log_capture` / `take_log_messages` / `install_mesh_source` / `clear_mesh_source`, backed by two thread-locals (`LOG_CAPTURE`, `MESH_SOURCE`). It is genuinely used by `crates/slicer-sdk/tests/host_wrappers_tdd.rs` and one runtime executor test (`crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs:29,461-518`). But `MockHost` in `crates/slicer-test/src/mock_host.rs` knows nothing about either thread-local; it carries its own parallel `logs: Vec<LogEntry>` and `call_counts: HashMap<String, usize>` fields, divorced from the real host wrapper code.

The documentation also describes APIs that don't exist (`set_raycast_z_down` with per-arg signature, `with_logging`, `clip_polygons_call_count`, `circle_polygon`, `rect_polygon`, `path_length`, `slicer_test::prelude`). Future agents reading the docs see a richer test surface than the codebase provides, then either work around the gap or push tests up into `slicer-runtime/tests/` where the full host machinery is available (which is exactly what happened with `wipe_tower_bed_bounds.rs` and `prepass_support_generation_orca_parity_tdd.rs` — relocated in packet 80).

This packet closes the gap **without moving any files**. The macro's expansion changes to fully-qualified `::slicer_sdk::test_support::*` calls. The SDK gains a feature-gated `test_support` mod that provides the four hooks the macro names, routing through the existing `host::test_support` thread-locals. `MockHost` is rewritten as a real `MeshSource` adapter with builder-style chaining matching the in-tree `StubMesh` precedent (`host_wrappers_tdd.rs:67-87`). Doc fictions are excised. ADR-0004 records the convergence direction so packets 78–80 don't re-litigate.

This packet keeps the `slicer-test` crate alive on purpose. The fold (move source files into `slicer-sdk`, delete the crate, expose via `slicer_sdk::test_prelude`) is packet 78's responsibility; doing both in one packet would couple two reversible changes into one large blast radius.

## Architecture Constraints

- **Feature gating is structural, not decorative.** `test_support` MUST be `#[cfg(any(test, feature = "test"))]`. The negative case (AC-N1) deliberately probes the gate. A symbol leak into the non-feature build would break the slicer-sdk-in-every-guest-wasm invariant covered in packet 78.
- **`MeshSource: Send + Sync + 'static`.** `host::test_support::install_mesh_source` takes `S: MeshSource`, which is bound `Send + Sync + 'static`. `MockHost` must satisfy this — the fields chosen (`Option<f32>`, `Option<Point3>`, `Option<BoundingBox3>`, `HashMap<String, usize>`) all do.
- **Panic-hook chaining must preserve the previous hook.** Tests rely on `cargo test`'s default panic banner + backtrace. `take_hook` + `set_hook` with delegation preserves it. A bare `set_hook` would silently swallow the standard test output and break debugging.
- **Macro hook call order is `reset → install_panic → setup → (body) → teardown`.** Reset must run first so a leftover hook from a prior test (in case the prior test installed one without removing it) is cleared before the new chain. Setup runs after panic-handler install so any panic in `mock_host_setup` itself drains correctly.
- **`MockHost::install` consumes `self`.** Once installed, the value is in the thread-local; you can't keep mutating it after install. Tests that need varied answers construct a fresh `MockHost` and re-`install()`. This is documented behavior and the AC-N2 regression locks the "second install replaces first" semantics.
- **`f32: !Hash`.** Per-arg-keyed mocking (the `HashMap<(String, f32, f32, f32), Option<f32>>` shape implied by the doc fiction) is structurally rejected; one-answer-per-query stays the contract.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- `host::test_support::install_mesh_source` has **replace semantics** (it writes `*cell.borrow_mut() = Some(Box::new(source))`). The AC-N2 regression test pins this.
- `host::test_support::take_log_messages` returns `Vec<(LogLevel, String)>` and **uninstalls the sink** (the source code at `host.rs:330` does `cell.borrow_mut().take()`). The packet's `reset_global_state` and `mock_host_teardown` both use it; the second call inside the same test would return empty — that is expected and AC-6 verifies the consequence.
- `MockHost::log_warn` after the refactor calls `slicer_sdk::host::log_warn(message)`, which routes through `LOG_CAPTURE` if the capture is installed. `mock_host_setup` installs the capture, so during a `#[module_test]` body, `MockHost::log_warn` is observable via `MockHost::log_contains` (which calls `take_log_messages` and inspects the returned vec). **Side effect**: calling `log_contains` drains the buffer. Document this in the rustdoc comment.
- The `__SlicerTestGuard` symbol in the macro expansion stays — it's a local struct inside the function body, not a public API. No collision risk.

## Locked Assumptions and Invariants

- **Invariant A**: `host::test_support` thread-locals are per-thread. Rust's default test runner runs tests on multiple threads, so cross-thread leakage is impossible by construction. `#[module_test]` does not need explicit serialization.
- **Invariant B**: Within a single `#[module_test]`, the hook order `reset → install_panic → setup → (body) → teardown` MUST NOT change. Tests rely on the body seeing a clean state and the panic hook being live for the body's execution. Reordering is a behavior change that would invalidate AC-6 and AC-7.
- **Invariant C**: `MeshSource` is a `Send + Sync + 'static` trait. `MockHost` must remain auto-`Send + Sync` (no `Rc`, no `RefCell` exposed). All fields chosen satisfy this.
- **Invariant D**: The macro must still expand to a function that the test framework recognizes as a `#[test]`. Both `quote!` branches preserve `#[test]` on the outer function.

## Risks and Tradeoffs

- **Risk: macro expansion bug surfaces only at downstream use, not in `slicer-macros` tests.** Mitigation: `crates/slicer-macros/tests/{smoke,module_test_tdd}.rs` now depend on `slicer-sdk` with `features = ["test"]`, so they exercise the fully-qualified path. If those compile and pass, the expansion is correct.
- **Risk: `install_panic_handler` chaining captures the test framework's hook permanently.** Tests don't typically uninstall hooks at end. Mitigation: `mock_host_teardown` runs in the `Drop` guard; we don't restore the prior hook on teardown (Rust offers no `restore_hook` API for the chain we built). The next `install_panic_handler` call takes the previous hook (which is our own chained one) and re-chains, potentially building a stack. This is acceptable per-process but should not leak across `cargo test` invocations (which start fresh processes). **Document this** in the `install_panic_handler` rustdoc as a known characteristic; not a bug.
- **Risk: doc-fiction cleanup leaves dangling examples that someone tries to copy-paste.** Mitigation: AC-9 grep gate prevents the strings from staying. The replacement examples MUST compile against the new MockHost API.
- **Tradeoff: per-arg-keyed mocking would be more expressive but `f32: !Hash`.** Accepted: one-answer-per-query.
