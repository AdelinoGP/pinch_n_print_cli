# ADR-0003: WIT↔IR conversions stay generated per-world inside `#[slicer_module]`

Status: accepted

ADR-0002 unified cross-world WIT type *identity on the host* (`wit_host.rs`) by
remapping each world's `bindgen!` onto the layer world's `slicer:types/geometry`,
deleting the duplicate host-side converters. This ADR records the analogous —
but opposite — decision for the **guest/macro** side.

The `#[slicer_module]` proc macro emits WIT↔IR/SDK conversions
(`ExtrusionRole`, `RetractMode`, `ExtrusionPath3D`, `ExPolygon`, gcode-command
drains) for each of the four worlds. A 2026 architecture depth review flagged
these as duplicated across worlds and proposed extracting them into a shared
plain-Rust crate of free functions.

**Decision:** keep the conversions generated *per world*. Where a conversion
shape recurs across worlds, generate it from a single macro-level emitter (one
source of truth, emitted as `From`/`Into` impls per world); do **not** move them
into a shared crate of plain functions.

**Why a shared crate is not viable (unlike the host side):** the host can use
`bindgen!`'s `with:` remap to make all worlds share one set of generated types,
because the host expands all four worlds in one crate. Guests cannot: each guest
runs its *own* `wit_bindgen::generate!` inside a private
`mod __slicer_<world>_world_export` (see `crates/slicer-macros/src/lib.rs`), and
a given guest only links the world it implements. The generated WIT types
therefore exist only inside each guest's per-world module — distinct Rust types
that do not exist in any shared crate's namespace until macro-expansion time. A
plain shared function cannot name them in its signature. The IR/SDK side is
concrete and shared, but every conversion bridges at least one per-world WIT
type, so none can be fully hoisted.

**Consequence:** these conversions cannot be unit-tested as ordinary Rust
functions; they are covered by the guest round-trip contract tests
(`crates/slicer-runtime/tests/contract/macro_*_roundtrip_tdd.rs`), which compile
a guest and dispatch through wasmtime, and any macro edit requires
`cargo xtask build-guests` before those tests are trustworthy. Recorded so
future architecture reviews do not re-propose the guest-side shared-crate
extraction.
