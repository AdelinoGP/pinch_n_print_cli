---
status: implemented
packet: 114_host-services-wit-unification
task_ids: []
---

# 114_host-services-wit-unification

## Goal

Hoist the four byte-identical per-world `host-services` WIT interfaces into one shared `slicer:common/host-services`, remap it (and the already-shared `slicer:common/module-errors`) onto the layer world via `bindgen!`'s `with:`, and collapse the four duplicated host `Host`-trait impls to one each — extending ADR-0002's remap pattern from geometry/config to the `slicer:common` interfaces.

## Problem Statement

The `host-services` WIT interface — eight host functions (`log`, `raycast-z-down`, `surface-normal-at`, `object-bounds`, `clip-polygons`, `offset-polygons`, `simplify-polygon`, `now-us`) plus its `log-level`/`clip-operation`/`offset-join-type` enums — is declared **inline and byte-for-byte identical** in all four world WITs (`world-layer`, `world-prepass`, `world-finalization`, `world-postpass`, each lines 3–18). Because the interface is per-world rather than a shared package interface, `bindgen!` generates four distinct `Host` traits, forcing four identical Rust impls in `host.rs` (`hs::Host` 1682–1803, `phs::Host` 3282+, `fhs::Host` 3549+, `pphs::Host` 4211+) whose bodies differ only by the world's enum namespace. The same shape afflicts `module-errors`: it is already a shared `slicer:common` interface, but is not remapped, so four empty `Host` impls exist (1660–1663).

ADR-0002 unified geometry/config types with exactly the fix this packet applies — a shared interface remapped onto the layer world via `with:` — but stopped at `slicer:types`/`slicer:config`. Extending the same remap to the `slicer:common` interfaces collapses four host-services impls to one and four module-errors impls to one, deleting ~360 LoC of duplicated Rust and removing the four-way drift hazard in the WIT itself.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Layer remains the canonical owner; the other three worlds remap onto `super::layer::slicer::common::{host_services, module_errors}` (ADR-0002 mechanism). `pub mod layer` already precedes the others, so the `super::layer::…` paths resolve.
- The four `bindgen!` invocations stay (count == 4; ADR-0005). This packet edits their `with:` only.
- The import namespace change (`host-services` → `slicer:common/host-services`) is **ABI-visible**: the component import name changes, so every guest must be rebuilt and re-linked. There is no `coord-system` concern (no geometry/mm math).

## Data and Contract Notes

- New world import line (all four): `import slicer:common/host-services;` (replacing `import host-services;`). The interface's `use slicer:types/geometry.{…}` stays — geometry is already shared.
- host.rs `with:` additions (prepass/finalization/postpass blocks), e.g.:
  `"slicer:common/host-services": super::layer::slicer::common::host_services,`
  `"slicer:common/module-errors": super::layer::slicer::common::module_errors,`
- Surviving impls: `impl layer::slicer::common::host_services::Host for HostExecutionContext` and `impl layer::slicer::common::module_errors::Host for HostExecutionContext` (named paths may differ post-remap; the count, not the path, is what AC-3/AC-4 assert).

## Locked Assumptions and Invariants

- The host-services and module-errors interface *content* is unchanged — function set, signatures, and enums are identical to today's four copies. Relocation only.
- The remapped Rust types are identical to layer's (the whole point); the three deleted impls were exact duplicates.
- `bindgen!` count stays 4.

## Risks and Tradeoffs

- **Guest relink** is the central risk: a guest that fails typed instantiation after the import moves indicates a missed rebuild or a path mismatch. AC-7 (freshness) + AC-N1 (roundtrip) bracket it.
- **WIT-checklist breadth**: per CLAUDE.md, search `wit_host.rs`/`dispatch.rs`/`wit_guest` for the affected interface after the change; a stray hardcoded `host-services` path would break linking. Low likelihood (macro/SDK do not hardcode it), but the search is mandatory.
- **Compile-broken window** across Steps 1–3 — accepted; the gate runs only after Step 3.
