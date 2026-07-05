# ADR-0033 — Host-Service Bridge Pattern for Host-Only Algorithms

## Status

Accepted (2026-07-05). Formalizes a pattern that already shipped twice without a decision record: `medial-axis` (pre-existing) and `generate-arachne-walls` (packet 112, `D-112-HOSTSVC-BRIDGE`). Written retroactively — see "Context" for why.

## Context

Some algorithms a WASM guest module needs (Voronoi construction via `boostvoronoi`, parallelism via `rayon`) are gated behind `slicer-core`'s `host-algos` Cargo feature (`crates/slicer-core/Cargo.toml`: `host-algos = ["dep:rayon", "dep:boostvoronoi"]`) because those dependencies do not compile to `wasm32` at all. A guest module (compiled to `wasm32-wasip1` or similar) can never link `slicer-core` with `host-algos` enabled — the dependency graph simply does not build for that target.

Packet 112 (Arachne extrusion generation) needed exactly this: `arachne-perimeters` is a WASM guest module, but the real wall-generation pipeline (`slicer_core::arachne::pipeline::run_arachne_pipeline`) depends on `SkeletalTrapezoidationGraph::from_polygons`, which depends on `boostvoronoi`. Packet 112's original design assumed an in-guest call chain (`arachne-perimeters` calling `slicer-core` functions directly); this was infeasible for the reason above, discovered during implementation and recorded as `D-112-HOSTSVC-BRIDGE` in `docs/DEVIATION_LOG.md`. The fix mirrored an existing, undocumented instance of the same shape: `medial-axis` (`crates/slicer-schema/wit/deps/common.wit:21`, host impl at `crates/slicer-wasm-host/src/host.rs:1738`, guest-callable SDK wrapper at `crates/slicer-sdk/src/host.rs:310`) already bridges host-only geometry work (also gated by `host-algos`) across the WIT boundary the same way. Neither instance had an ADR; this one exists so a third instance doesn't repeat the pattern without a citable rationale.

## Decision

**When a guest module needs an algorithm gated behind a host-only Cargo feature (`host-algos` or equivalent), add a new WIT host-service function rather than attempting to link the host-only code into the guest.** The shape, established by `medial-axis` and followed by `generate-arachne-walls`:

1. **WIT declaration** in `crates/slicer-schema/wit/deps/common.wit`'s `host-services` interface: a `func` taking plain data (polygons, params — no opaque handles) and returning `result<T, string>`. `generate-arachne-walls: func(polygons: list<ex-polygon>, params: arachne-params) -> result<list<extrusion-line>, string>;` (`common.wit:50`) mirrors `medial-axis: func(input: ex-polygon, min-width: f32, max-width: f32) -> result<list<thick-polyline>, string>;` (`common.wit:21`) exactly in this respect.
2. **Host-side implementation** in `crates/slicer-wasm-host/src/host.rs`: the WIT trait impl (`generate_arachne_walls` at line 1767, mirroring `medial_axis` at line 1738) delegates directly to the native `slicer-core` function (`slicer_core::arachne::pipeline::run_arachne_pipeline`) and marshals the result back to WIT types. This file always builds with `host-algos` enabled (`crates/slicer-wasm-host/Cargo.toml` enables it unconditionally) since the host process is never itself `wasm32`.
3. **Guest-callable SDK wrapper** in `crates/slicer-sdk/src/host.rs` (`generate_arachne_walls` at line 536, mirroring `medial_axis` at line 310): a `cfg`-split function — the native/non-`wasm32` branch calls the `slicer-core` function **directly** (no WIT round-trip needed when running natively, e.g. in host-side tests), and the `wasm32` branch marshals the call across the WIT import binding. `slicer-sdk`'s own `Cargo.toml` only enables `host-algos` under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, so Cargo feature unification guarantees a `wasm32` guest build never pulls in `rayon`/`boostvoronoi` even transitively.
4. **Guest module** calls only the SDK wrapper (`modules/core-modules/arachne-perimeters/src/lib.rs` calls `slicer_sdk::host::generate_arachne_walls`, not `slicer_core` directly) and, per this project's existing WASM-boundary convention, does not depend on `slicer-core` at all when it only needs bridged algorithms — `arachne-perimeters`'s own `Cargo.toml` has no `slicer-core` dependency.

This keeps the guest module's build graph clean (no dead `host-algos`-only deps ever considered for the `wasm32` target) while giving it access to host-only computation through the same claim/execution model every other host service (`clip-polygons`, `offset-polygons`, mesh queries) already uses.

## Rejected alternatives

- **Vendor/reimplement a WASM-portable subset of the algorithm inside the guest.** Rejected for `generate-arachne-walls`: `boostvoronoi`'s segment-Voronoi construction (see ADR-0023) is exactly the kind of numerically delicate computational-geometry code a from-scratch WASM port would risk subtly breaking, and no WASM-portable equivalent crate was identified.
- **Drop the `wasm32` guest target for `arachne-perimeters` and make it a host-native-only module.** Rejected: violates this project's module-execution model, in which every `core-module` (including `arachne-perimeters`) is a WASM component loaded uniformly by the scheduler; carving out a native-only exception for one module would require a second module-loading code path.
- **Pass a `slicer-core`-internal handle/token across the WIT boundary instead of plain data.** Rejected: WIT host-service functions in this codebase (`clip-polygons`, `offset-polygons`, `medial-axis`) already establish the convention of passing plain geometry data by value across the boundary, not opaque handles; breaking that convention for one new service would fragment the host-services interface's calling style for no benefit (the plain-data payloads here are not large enough to justify a handle-based indirection).

## Consequences

- Any future guest-side need for `host-algos`-gated computation (or any other host-only dependency) should follow this same four-layer shape (WIT func → host impl delegating to the native crate → SDK `cfg`-split wrapper → guest calls the wrapper only) rather than re-deriving an ad-hoc bridge.
- `docs/03_wit_and_manifest.md`'s `host-services` interface listing now documents both `medial-axis` and `generate-arachne-walls` (previously undocumented for both — this ADR's own trigger).
- `D-112-HOSTSVC-BRIDGE` in `docs/DEVIATION_LOG.md` should be read alongside this ADR: the deviation records *that* the in-guest design was infeasible and the bridge was the fix; this ADR records the *reusable pattern* the fix established.
- No `Cargo.toml` changes are required by this ADR — the feature-gating structure it documents (`host-algos` unconditional in `slicer-wasm-host`, conditional-on-native in `slicer-sdk`, absent from guest modules that only need bridged access) already exists exactly as described.

## Future reviewers

- If a third host-service bridge is added, point its own deviation/design doc at this ADR instead of re-explaining the four-layer shape from scratch.
- If the WIT host-services calling convention ever moves away from plain-data payloads (e.g. to opaque resource handles), update this ADR's "Decision" and "Rejected alternatives" sections rather than letting new services silently diverge from what's documented here.
