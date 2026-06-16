# Design — Packet 77

## Controlling Code Paths

The packet touches three crates plus two docs files. The seam being closed lives across all three crates simultaneously — none of them in isolation makes the documented contract honest.

- **`crates/slicer-macros/src/lib.rs:2685-2756`** — the `#[module_test]` proc-macro and its two `quote! { ... }` branches (with-return-type vs unit). Today they emit unqualified `__slicer_test_*` identifiers. The change rewrites both branches to emit `::slicer_sdk::test_support::*` fully-qualified paths. This is the single behavior change in `slicer-macros`.
- **`crates/slicer-sdk/src/host.rs:108-343`** — already provides the primitive thread-local seam (`LOG_CAPTURE`, `MESH_SOURCE`, `test_support::{install_log_capture, take_log_messages, install_mesh_source, clear_mesh_source}`). This packet does NOT modify any of this; it adds a sibling module that consumes these primitives.
- **`crates/slicer-sdk/src/lib.rs:20-31`** — module list. Adds `pub mod test_support;` under `#[cfg(any(test, feature = "test"))]`.
- **`crates/slicer-sdk/Cargo.toml`** — gains `[features] test = []`.
- **`crates/slicer-test/src/mock_host.rs`** — full rewrite (≈ 150 LoC delta net). The current 148-line file becomes a real `MeshSource` adapter with builder chaining.

The macro-expanded test body, after this packet, looks like:

```rust
#[test]
fn t() {
    struct __SlicerTestGuard;
    impl Drop for __SlicerTestGuard {
        fn drop(&mut self) {
            ::slicer_sdk::test_support::mock_host_teardown();
        }
    }
    ::slicer_sdk::test_support::reset_global_state();
    ::slicer_sdk::test_support::install_panic_handler();
    ::slicer_sdk::test_support::mock_host_setup();
    let _guard = __SlicerTestGuard;
    /* user body */
}
```

The new `test_support` module wraps the primitives:

```rust
// crates/slicer-sdk/src/test_support/mod.rs
#![cfg(any(test, feature = "test"))]
use crate::host;

pub fn reset_global_state() {
    let _ = host::test_support::take_log_messages();
    host::test_support::clear_mesh_source();
}

pub fn install_panic_handler() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let drained = host::test_support::take_log_messages();
        for (level, msg) in drained {
            eprintln!("[captured {}] {}", level.as_str(), msg);
        }
        prev(info);
    }));
}

pub fn mock_host_setup() { host::test_support::install_log_capture(); }
pub fn mock_host_teardown() {
    let _ = host::test_support::take_log_messages();
    host::test_support::clear_mesh_source();
}
```

`MockHost` becomes:

```rust
#[derive(Debug, Default)]
pub struct MockHost {
    raycast_hit: Option<f32>,
    normal: Option<slicer_ir::Point3>,
    bounds: Option<slicer_ir::BoundingBox3>,
    call_counts: std::collections::HashMap<String, usize>,
}

impl MockHost {
    pub fn new() -> Self { Self::default() }
    pub fn with_raycast_hit(mut self, v: Option<f32>) -> Self { self.raycast_hit = v; self }
    pub fn with_normal(mut self, v: Option<slicer_ir::Point3>) -> Self { self.normal = v; self }
    pub fn with_object_bounds(mut self, v: slicer_ir::BoundingBox3) -> Self { self.bounds = Some(v); self }
    pub fn install(self) { slicer_sdk::host::test_support::install_mesh_source(self) }
    pub fn uninstall() { slicer_sdk::host::test_support::clear_mesh_source() }
    /* record_call / call_count / assert_call_count stay */
    /* log_warn / log_contains route through host::log_warn and take_log_messages */
}

impl slicer_sdk::host::MeshSource for MockHost {
    fn raycast_z_down(&self, _o: &str, _x: f32, _y: f32, _sz: f32) -> Option<f32> { self.raycast_hit }
    fn surface_normal_at(&self, _o: &str, _x: f32, _y: f32, _z: f32) -> Option<slicer_ir::Point3> { self.normal }
    fn object_bounds(&self, _o: &str) -> Option<slicer_ir::BoundingBox3> { self.bounds }
}
```

## Architecture Constraints

- **Feature gating is structural, not decorative.** `test_support` MUST be `#[cfg(any(test, feature = "test"))]`. The negative case (AC-N1) deliberately probes the gate. A symbol leak into the non-feature build would break the slicer-sdk-in-every-guest-wasm invariant covered in packet 78.
- **`MeshSource: Send + Sync + 'static`.** `host::test_support::install_mesh_source` takes `S: MeshSource`, which is bound `Send + Sync + 'static`. `MockHost` must satisfy this — the fields chosen (`Option<f32>`, `Option<Point3>`, `Option<BoundingBox3>`, `HashMap<String, usize>`) all do.
- **Panic-hook chaining must preserve the previous hook.** Tests rely on `cargo test`'s default panic banner + backtrace. `take_hook` + `set_hook` with delegation preserves it. A bare `set_hook` would silently swallow the standard test output and break debugging.
- **Macro hook call order is `reset → install_panic → setup → (body) → teardown`.** Reset must run first so a leftover hook from a prior test (in case the prior test installed one without removing it) is cleared before the new chain. Setup runs after panic-handler install so any panic in `mock_host_setup` itself drains correctly.
- **`MockHost::install` consumes `self`.** Once installed, the value is in the thread-local; you can't keep mutating it after install. Tests that need varied answers construct a fresh `MockHost` and re-`install()`. This is documented behavior and the AC-N2 regression locks the "second install replaces first" semantics.
- **`f32: !Hash`.** Per-arg-keyed mocking (the `HashMap<(String, f32, f32, f32), Option<f32>>` shape implied by the doc fiction) is structurally rejected; one-answer-per-query stays the contract.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

Primary edits (≤ 3 conceptual surfaces; multiple files only because the seam crosses crates):

1. **`crates/slicer-sdk/src/test_support/mod.rs`** (new file, ≈ 40 LoC) + `crates/slicer-sdk/src/lib.rs` (+1 line) + `crates/slicer-sdk/Cargo.toml` (+3 lines for `[features] test = []`).
2. **`crates/slicer-macros/src/lib.rs:2705-2756`** — macro expansion rewrite (≈ 8 path tokens changed per branch × 2 branches).
3. **`crates/slicer-test/src/mock_host.rs`** — full rewrite (≈ 150 LoC delta). Plus `crates/slicer-test/Cargo.toml` (1 line: `features = ["test"]` on `slicer-sdk` dep) and `crates/slicer-test/src/lib.rs` (+ ~15 lines for `prelude` re-exports).

Secondary edits (mechanical follow-on from the primary changes):

4. `crates/slicer-test/tests/mock_host_adapter_tdd.rs`, `mock_host_isolation_tdd.rs`, `log_capture_round_trip_tdd.rs`, `panic_handler_drains_logs_tdd.rs` — four new test files (each ≈ 30-60 LoC).
5. `crates/slicer-macros/Cargo.toml` (+1 dev-dep line) + `crates/slicer-macros/tests/smoke.rs`, `module_test_tdd.rs` (delete stub functions; replace tracking-flag assertions with observable-behavior ones — net negative LoC).
6. `docs/05_module_sdk.md:445-624` — text edits only; no new sections.
7. `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — new file (≈ 60 LoC).

## Files in Scope (read+edit)

Edit-allowed:
- `crates/slicer-sdk/Cargo.toml`
- `crates/slicer-sdk/src/lib.rs`
- `crates/slicer-sdk/src/test_support/mod.rs` (new)
- `crates/slicer-macros/src/lib.rs` (only lines ≈ 2680-2760)
- `crates/slicer-macros/Cargo.toml`
- `crates/slicer-macros/tests/smoke.rs`
- `crates/slicer-macros/tests/module_test_tdd.rs`
- `crates/slicer-test/Cargo.toml`
- `crates/slicer-test/src/lib.rs`
- `crates/slicer-test/src/mock_host.rs`
- `crates/slicer-test/tests/mock_host_adapter_tdd.rs` (new)
- `crates/slicer-test/tests/mock_host_isolation_tdd.rs` (new)
- `crates/slicer-test/tests/log_capture_round_trip_tdd.rs` (new)
- `crates/slicer-test/tests/panic_handler_drains_logs_tdd.rs` (new)
- `docs/05_module_sdk.md` (only lines 445-624)
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (new)

## Read-Only Context

- `crates/slicer-sdk/src/host.rs` lines 108-343 — `LOG_CAPTURE` / `MESH_SOURCE` / `MeshSource` trait / `test_support` mod. Read once to confirm signatures; do not re-read.
- `crates/slicer-sdk/tests/host_wrappers_tdd.rs` lines 67-100 — `StubMesh` precedent that `MockHost`'s new shape mirrors.
- `crates/slicer-test/src/mock_host.rs` lines 1-148 — existing implementation (to identify what stays vs goes).
- `crates/slicer-test/src/fixtures.rs`, `capture.rs`, `assert_paths.rs` — read only to confirm what the new `prelude` should re-export. Do not edit.
- `docs/05_module_sdk.md` lines 445-624 only.
- `docs/adr/0001*.md`, `0002*.md`, `0003*.md` — read for style/format of new ADR. Each ≤ 50 lines per recon; combined ≤ 150 lines — safe to load.

## Out-of-Bounds Files

- `crates/slicer-sdk/src/builders.rs`, `coords.rs`, `error.rs`, `layer_collection_builder.rs`, `postpass_*`, `prelude.rs`, `prepass_*`, `traits.rs`, `views.rs` — out of scope; this packet only touches `host.rs` (read-only) and `lib.rs` (one-line addition).
- `crates/slicer-runtime/**` — out of scope entirely.
- `modules/core-modules/**` — out of scope (packets 79, 80).
- `crates/slicer-ir/**`, `crates/slicer-core/**`, `crates/slicer-schema/**`, `crates/slicer-helpers/**`, `crates/pnp-cli/**`, `xtask/**` — out of scope.
- `OrcaSlicerDocumented/**` — never load. No OrcaSlicer parity in this packet.
- All `target/`, lockfile, and generated wasm artifacts.

## Expected Sub-Agent Dispatches

The implementer should plan for these delegations (each with the listed return-format):

1. **Macro pre-edit context** — `Question: what is the current expansion of #[module_test] at crates/slicer-macros/src/lib.rs:2705-2756 — show only the two quote! { ... } blocks and the surrounding signature. Scope: that file, lines 2680-2760. Return: SNIPPETS (≤ 1 snippet, ≤ 60 lines).`
2. **`host::test_support` signature confirmation** — `Question: what are the exact public function signatures of slicer_sdk::host::test_support (install_log_capture, take_log_messages, install_mesh_source, clear_mesh_source)? Scope: crates/slicer-sdk/src/host.rs lines 320-343. Return: FACT (≤ 5 lines).`
3. **Pre-cleanup doc fiction inventory** — `Question: list every line number in docs/05_module_sdk.md:445-624 that references one of: circle_polygon, rect_polygon, path_length(, with_logging, clip_polygons_call_count, region_id(". Scope: that file, that range. Return: LOCATIONS (line:context, ≤ 20 entries).`
4. **Post-edit clippy delegate** — `Question: does cargo clippy --workspace --all-targets -- -D warnings pass? Scope: workspace. Return: FACT: clean / first violation with file:line.`
5. **Guest staleness recheck after macro change** — `Question: does cargo xtask build-guests --check pass after the macro edit? Scope: xtask. Return: FACT: clean / list of STALE guests.`
6. **AC-9 grep check** — `Question: does grepping docs/05_module_sdk.md:445-624 for the six forbidden tokens return any matches? Scope: that file, that range. Return: FACT: clean / list of remaining matches.`
7. **Bench-touchpoint sanity** — `Question: does cargo bench --no-run -p slicer-sdk compile after the test_support addition? Scope: slicer-sdk. Return: FACT: pass/fail.` (Defensive — ensures no `Default` derive collision in the test-only module.)

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

## Context Cost Estimate

- **Aggregate**: M (sum of 7 steps S/M/L below).
- **Largest single step**: Step 4 (`MockHost` rewrite + 4 new TDDs) — M.
- **Highest-risk dispatch**: dispatch 3 (doc-fiction inventory) — depends on accurate awk-range grep; verify the line range still matches by reading a 5-line window at the boundaries before grepping.

## Open Questions

None — every design decision was resolved during the grilling session before generation. The plan file at `C:\Users\agpen\.claude\plans\hidden-discovering-lollipop.md` records the resolution chain.
