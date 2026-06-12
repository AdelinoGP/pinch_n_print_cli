# ADR-0014 — `xtask` Guest Discovery Uses a Validated Filesystem Walk, Not `cargo_metadata`

## Status

Accepted (Packet 70 / TASK-216-guest-builder).

## Context

`cargo xtask build-guests` builds every guest WASM artefact in the workspace (core-modules + test-guests). It needs to discover the guests, check their freshness against tracked sources, and rebuild stale ones. The obvious approach — read `cargo_metadata` to enumerate workspace members — does not work in this repository.

Each guest `Cargo.toml` declares its own `[workspace]` sentinel so that the parent workspace's `cargo` invocations do not accidentally pull guest deps into the host build graph. The sentinel is load-bearing: removing it would re-introduce `wit-bindgen` and friends into the parent `Cargo.lock`. The side-effect is that `cargo metadata` from the parent workspace reports zero guest members; the guests are invisible to it.

Additionally, the xtask is invoked from the agentic build hook. Hook latency matters: anything pulling `wasmtime`, `slicer-runtime`, or `pyo3` into the xtask compile graph adds seconds to every cold rebuild. The xtask must stay lean.

## Decision

**Guest discovery is a validated filesystem walk over two tree-roots, with no `cargo_metadata` involvement.** Concretely:

- `xtask/src/build_guests.rs` walks `modules/core-modules/*/wit-guest/` and `crates/slicer-wasm-host/test-guests/*/` using `walkdir`.
- For each candidate `Cargo.toml`, the discoverer parses it via `toml` and validates a per-tree shape predicate:
  - **Core-modules wit-guests:** `[lib].crate-type = ["cdylib"]` + `[workspace]` sentinel + path-dep on the parent core-module crate.
  - **Test-guests:** `[lib].crate-type = ["cdylib"]` + `[workspace]` sentinel + dep on `wit-bindgen`.
- Candidates that fail validation surface as `SKIP:` lines (informational, not fatal) so authors learn early when a guest stops matching the contract.
- Artefact output paths are per-tree conventions, stable across the packet lifetime:
  - core-modules: `modules/core-modules/<dir>/<dir>.wasm`
  - test-guests:  `crates/slicer-wasm-host/test-guests/<crate-name>.component.wasm`
- Freshness is computed by comparing the artefact's mtime against the latest mtime of: `crates/slicer-schema/wit/**/*.wit` + `crates/slicer-{macros,sdk,ir,schema}/{src,Cargo.toml}` + per-guest sources. `slicer-core` and `slicer-helpers` are explicitly NOT tracked (the former is optional per guest; the latter is host-only).
- The xtask crate's `Cargo.toml` declares only `walkdir` and `toml` as deps. It MUST NOT depend on `slicer-runtime`, `slicer-wasm-host`, `wasmtime`, `pyo3`, `truck-stepio`, or `meshopt`.

## Consequences

- **Discovery works correctly with the `[workspace]` sentinel pattern.** The pattern stays in place; the xtask sees the guests; nothing leaks into the parent build graph.
- **Hook latency stays low.** Cold xtask builds finish in single digits of seconds.
- **Freshness is precise without being conservative.** Touching `slicer-core` does not trigger a guest rebuild storm; touching the WIT files does.
- **Layout changes require xtask updates.** Moving the test-guests directory or renaming a tree-root requires editing `build_guests.rs`. This is a deliberate trade — `cargo_metadata` would be more flexible but unusable.
- **A path drift exists between the original packet 70 spec and the on-disk layout.** The packet spec referenced a top-level `test-guests/` tree-root that was never materialised; the actual location is `crates/slicer-wasm-host/test-guests/`. The discrepancy is documented in `DEVIATION_LOG.md` DEV-072; the on-disk layout is authoritative.

## Rejected alternatives

- **Use `cargo_metadata`.** Returns zero guests because of the `[workspace]` sentinel. Removing the sentinel would pull `wit-bindgen` into the host build graph. Rejected.
- **Drop the `[workspace]` sentinel and accept the dep-leak cost.** The cost is large (cold host rebuilds get materially slower; wit-bindgen ABI churn becomes a host-build risk). Rejected.
- **Hard-code the guest list in `build_guests.rs`.** Authors adding a new guest would have to remember to register it; mistakes would surface as silent skip. Rejected in favour of automatic discovery with validation feedback.

## Future reviewers

- Do not add `slicer-runtime`, `slicer-wasm-host`, `wasmtime`, or any heavy crate as an `xtask` dep. Verify with `cargo tree -p xtask --edges normal`.
- Do not migrate discovery back to `cargo_metadata` without first removing the `[workspace]` sentinels and measuring the impact on host build times.
- If a new tree-root is added for a third class of guest (e.g. example modules), add a new shape predicate alongside the existing two — do not relax the existing ones.
