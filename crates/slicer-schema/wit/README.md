# Canonical WIT Contract

This directory (`crates/slicer-schema/wit/`) is the **single canonical WIT contract** for
the ModularSlicer pipeline. Both the host and the guest macro read these exact files — there
are no flattened copies, no inline literals elsewhere.

## Layout

```
wit/
  root.wit                      # package slicer:root@1.0.0 (umbrella anchor)
  deps/
    types.wit                   # package slicer:types       — interface geometry
    config.wit                  # package slicer:config      — interface config-types
    ir-types.wit                # package slicer:ir-handles  — interface ir-handles
    common.wit                  # package slicer:common      — interface module-errors
    world-layer/world-layer.wit           # package slicer:world-layer@1.0.0
    world-prepass/world-prepass.wit       # package slicer:world-prepass@1.0.0
    world-postpass/world-postpass.wit     # package slicer:world-postpass@1.0.0
    world-finalization/world-finalization.wit  # package slicer:world-finalization@1.0.0
```

Dep packages (`slicer:types`, `slicer:config`, etc.) are **unversioned** — required for
cross-package resolution with `wit_parser`. World packages carry `@1.0.0`.

## How each consumer reads these files

**Host** (`crates/slicer-runtime/src/wit_host.rs`):
```rust
wasmtime::component::bindgen!{
    path: "../slicer-schema/wit",
    world: "slicer:world-layer/layer-module@1.0.0",
    // with: { "slicer:config/config-types.config-view" => ..., ... }
}
```
One `bindgen!` call per world, all pointing at this directory. No inline WIT.

**Guest proc-macro** (`crates/slicer-macros/src/lib.rs`):
The `#[slicer_module]` macro reads the dep files via `include_str!`, wraps each
`package x;` declaration in nested-package braces `package x { … }`, concatenates with
the world file, and feeds the result to `wit_bindgen::generate!{ inline: … }`. Both sides
ultimately parse the same bytes from `deps/*.wit`.

## Naming rule

The geometry path type is **`extrusion-path3d`**. The digit is fused directly to
`path` (one segment), giving the ABI-stable single-segment form that the host's
hand-written bindings already use. Any hyphen-before-digit spelling such as
`extrusion-path-<n>` (where `<n>` is a numeral) is **avoided** because it would
produce a different ABI symbol and break the host↔guest linking contract — even
though wit-parser 0.247 technically permits such labels. The canonical name is
therefore `extrusion-path3d` for consistency, not legality.

## Drift trap — never add a flattened copy

If you copy these files to a flat location and edit them, the guest and host may diverge
silently. Both consumers **must** read these files directly. The conformance tests in
`wit_single_source_tdd.rs` assert:
- No `*-flat*` files exist here (`no_flat_copies`).
- World files cross-reference dep packages (`worlds_are_not_self_contained`).
- Each shared interface is defined exactly once (`shared_interface_defined_once`).
