---
status: implemented
packet: 113_marshal-boundary-extraction
task_ids: []
---

# 113_marshal-boundary-extraction

## Goal

Consolidate every IR↔WIT translation in `slicer-wasm-host` into one in-process `marshal` module whose origin-attribution rule lives in a single `OriginBucket` that is unit-testable without instantiating a WASM component — implementing ADR-0021.

## Problem Statement

WIT↔IR marshalling in `slicer-wasm-host` is spread across ~40 free functions in two files: the marshal-in projections and marshal-out harvest converters in `host.rs` (5225 LoC) and the postpass converters plus the 230-LoC `deconstruct_layer_ctx` router in `dispatch.rs` (2585 LoC). Tracing one type across the seam means bouncing ~2700 lines inside `host.rs`, then into a second file. Two specific costs motivate this packet:

1. **Stale per-world duplication (mostly).** ADR-0002 unified the four worlds' geometry/config Rust types via `bindgen!`'s `with:` remap, which made the **outbound** converters byte-identical (`ir_to_wit_extrusion_role` ≡ `finalization_role_ir_to_wit` ≡ `convert_postpass_role_to_wit`; `finalization_path_ir_to_wit` ≡ layer; `ir_to_wit_expolygon_prepass` ≡ `ir_to_wit_expolygon`). These are dead copies — ADR-0002 named their removal as a "Deferred" follow-up. **Exception:** the *inbound* role converters `finalization_role_wit_to_ir` and `convert_postpass_role` only *look* identical — unlike layer's `convert_extrusion_role`, they keep `Custom(s) => Custom(s)` instead of recovering `PrimeTower`/`Skirt` from the builtin tags. That is a latent bug (ADR-0021 §Amendment), not dead duplication; it is relocated unchanged here and fixed in packet 115.

2. **The bug-prone logic is untestable in isolation.** The origin-attribution rule — guest output re-bucketed to its source region via `(object_id, region_id)` tuples under an all-or-none tagging contract with finite-float validation — is re-implemented three times inside `convert_infill_output` (135 LoC), `convert_perimeter_output` (194 LoC), and `convert_support_output` (113 LoC), and is exercised only through full wasmtime dispatch. A silent regression in identity preservation cannot be caught by a fast unit test today.

ADR-0021 resolves both: one `marshal` module of flat functions over a shared, unit-testable `OriginBucket`.

## Architecture Constraints

- `marshal` is an **in-process module** (`src/marshal/`), never a crate: the WIT types exist only after `bindgen!` expands inside `slicer-wasm-host` (host-side analogue of ADR-0003). `marshal` must not reference `wasmtime` (AC-2).
- The four `bindgen!` invocations stay in `host.rs`; their count must remain 4 (ADR-0005). This packet does not touch them.
- Per-stage harvest routing stays in `dispatch.rs`, co-located with the stage→export match (ADR-0006). Do **not** move the `match stage_id` into `marshal`.
- Builder methods (`HostInfillOutputBuilder::push_*`, etc.) stay on `HostExecutionContext`; only the `*Collected` data structs move, so no `host ↔ marshal` cycle forms.
- (No `wasm-staleness` constraint: the change surface is host-only Rust and feeds no guest build path. No `coord-system` constraint: converters relocate geometry types but introduce no mm↔unit conversion.)

## Data and Contract Notes

Canonical signatures (from ADR-0021; implement verbatim):

```rust
// marshal/origin.rs
pub struct OriginId { pub object_id: String, pub region_id: u64 } // Clone+Eq+Hash
pub enum MarshalError {
    UntaggedPayload { kind: &'static str, index: usize },
    OriginLengthMismatch { kind: &'static str, origins: usize, payloads: usize },
    NonFiniteFloat { field: &'static str, index: usize },
}
impl From<MarshalError> for String { /* preserves today's Result<_, String> */ }

pub struct OriginBucket<R> { tagged: bool, regions: Vec<(OriginId, R)>, mint: fn(&OriginId) -> R }
impl<R> OriginBucket<R> {
    pub fn new(any_tagged: bool, mint: fn(&OriginId) -> R) -> Self;
    pub fn drain<T>(&mut self, kind: &'static str, payloads: Vec<T>,
                    origins: &[Option<OriginId>], place: impl FnMut(&mut R, T))
        -> Result<(), MarshalError>;
    pub fn into_regions(self) -> Vec<R>;
}
```

`new(false, …)` mints exactly one anonymous region (`OriginId{ "", 0 }`); `drain` then ignores origins. `new(true, …)` mints regions on first sight of each origin, in first-appearance order; an untagged element or length mismatch errors. `convert_infill_output` becomes ~25 LoC: build three leaf-mapped path vecs, OR the three origin slices into `any_tagged`, `OriginBucket::new`, three `drain` calls (`sparse_infill`/`solid_infill`/`ironing` push closures), `into_regions`. `convert_perimeter_output` keeps its rotated-vs-original wall selection *inside* the function (it chooses which `(payloads, origins)` pair to feed `drain`).

## Locked Assumptions and Invariants

- Converter output for valid input is unchanged (AC-6). First-seen bucket ordering must match the existing `Vec::position`-based loop exactly.
- `MarshalError` Display reproduces today's error strings closely enough that any test asserting on message substrings still passes; tests should prefer asserting on the variant.
- `OriginId` equality/hash semantics equal the old `(String, u64)` tuple semantics.
- The inbound finalization/postpass role converters are relocated with **identical (Custom-preserving) behaviour**; this packet does NOT recover `PrimeTower`/`Skirt` (packet 115 does). Folding that fix in here would violate the behaviour-preserving guarantee above.

## Risks and Tradeoffs

- **Ordering drift**: if `OriginBucket` changes bucket order, identity-keyed IR diffs change. Mitigated by AC-6 + the first-seen-order unit test.
- **Large mechanical diff** across two 2000–5000-line files; mitigated by incremental steps each gated on `cargo check` (cross-step invariant).
- **Hidden non-identical "dup"**: a named converter may differ subtly (custom-tag handling). Step 1's delegated diff is the guard; non-identical ones become an [FWD] question, not a blind delete.
