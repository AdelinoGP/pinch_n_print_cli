# Host marshalling unifies WIT types across worlds via bindgen `with:` remap onto the layer world

**Status:** accepted (packet 75, Phase 3 / TASK-218)

The slicer host links four wit-bindgen worlds — layer, prepass, finalization,
postpass — that all `use slicer:types/geometry` and `slicer:config/config-types`
from the same shared WIT packages. Each `bindgen!` invocation previously
regenerated those interfaces as *distinct nominal Rust types*, forcing ~400 lines
of per-world IR↔WIT converters and host-services/config-view impl bodies that
differed only by type namespace.

## Decision

The **layer world is the canonical owner** of the shared geometry and config Rust
types. The prepass/finalization/postpass `bindgen!` blocks remap those interfaces
onto layer's generated modules:

```rust
with: {
    "slicer:types/geometry": super::layer::slicer::types::geometry,
    "slicer:config/config-types": super::layer::slicer::config::config_types,
}
```

This gives the four worlds **one** set of Rust types (genuine type identity, not a
macro generating four copies). The per-world geometry/config `Host`, `HostConfigView`,
and ExPolygon/Point3/BoundingBox3 converters are deleted; the host-services impls
reuse the layer originals via `use super::*`. Net −376 lines in `wit_host.rs`. The
WIT contract and component ABI are unchanged — this is a host-only codegen change,
so guests are not rebuilt (`cargo xtask build-guests --check` stays clean).

`pub mod layer` is declared before the other worlds so the `super::layer::…` paths
resolve.

## Considered and rejected

- **A dedicated `shared` bindgen** generating only `slicer:types`/`slicer:config`,
  with all four worlds (layer included) remapping onto it. Symmetric, but needs a
  synthetic world to bindgen those interfaces and makes layer remap onto it too —
  more moving parts, and it fights the existing re-exports that already canonicalise
  on layer (`wit_host.rs` `pub use layer::…`).
- **A declarative `impl_host_services!` macro** invoked four times. Safe, but
  retains four monomorphic copies (textual dedup, not type identity) — kept only as
  a fallback had `with:` remapping not worked.

## Consequence

A future fifth world must **remap its geometry/config onto the layer world**, not
regenerate them — otherwise its types won't be identical to the others' and the
shared converters/impls won't apply.

Packet 114 extended the remap set: the `slicer:common` interfaces (`host-services`,
`module-errors`) are now also remapped onto the layer world via `bindgen!` `with:`,
alongside the existing geometry/config remaps. Any future world (fifth world) MUST
also remap these `slicer:common` interfaces onto the layer world.

## Deferred

The per-world extrusion-role / extrusion-path / retract-mode converters
(`finalization_role_*`, `finalization_path_*`, `convert_postpass_role`,
`convert_postpass_retract_mode`) still exist. They now operate on the unified types
but have asymmetric layer coverage (layer provides ir→wit role but no wit→ir) and
dedicated tests; their dedup is a smaller follow-up.
