# ADR-0021: The IR↔WIT marshalling boundary is a `marshal` module of flat functions over a shared `OriginBucket`, not a world-parameterized trait

## Status

Accepted (architecture-review session, 2026-06-16).

## Context

WIT↔IR marshalling in `slicer-wasm-host` is spread across ~40 free functions
in two files: the IR→WIT (marshal-in) projections and the WIT→IR (marshal-out)
harvest converters in `host.rs` (5225 LoC), plus the postpass converters and
the 230-LoC `deconstruct_layer_ctx` harvest router in `dispatch.rs` (2585 LoC).
Tracing one type across the seam means bouncing ~2700 lines within `host.rs`
and then into a second file; the two directions of one concept live far apart.

Two forces compound the friction:

1. **Stale per-world duplication.** Because ADR-0002 unified the four worlds'
   geometry/config Rust types via `bindgen!`'s `with:` remap, several
   "per-world" converters are now byte-identical: `ir_to_wit_extrusion_role` ≡
   `finalization_role_ir_to_wit`; `convert_extrusion_role` ≡
   `finalization_role_wit_to_ir` ≡ `convert_postpass_role`;
   `ir_to_wit_expolygon_prepass` ≡ `ir_to_wit_expolygon`. These are the
   "Deferred" follow-up ADR-0002 named, not live variation.

2. **The bug-prone logic is untestable in isolation.** The origin-attribution
   rule — guest output is re-bucketed to its source **region** via origin
   tuples `(object_id, region_id)`, under an all-or-none tagging contract with
   finite-float validation — is re-implemented inside `convert_infill_output`
   (135 LoC), `convert_perimeter_output` (194 LoC), and `convert_support_output`
   (113 LoC). It is exercised only through full wasmtime dispatch.

A "Design It Twice" interface exploration produced three shapes: minimal
(generic `marshal_out<IntoIr>`), common-caller (a single deep
`harvest_layer_commit` swallowing the stage match), and flexibility (a
`ToWit<World>` / `FromWit` / `Harvest` / `Project` trait family with world ZST
tags). The flexibility design justified its `ToWit<World>` seam on
`ExtrusionRole` "already crossing two worlds" — but that duplication is exactly
what the unification cleanup deletes. The trait would price a seam the
unification removes, and it cuts against ADR-0003 / ADR-0005, which both reject
abstractions introduced only to make a move work.

## Decision

Introduce an in-process `marshal` module (`crates/slicer-wasm-host/src/marshal/`,
`pub(crate)`) that owns the entire IR↔WIT boundary in both directions — the
**Marshalling boundary** in `CONTEXT.md`. It cannot be a separate crate: the WIT
types exist only after `bindgen!` expands inside this crate (the host-side
analogue of ADR-0003's guest-side reasoning).

Concrete shape:

- **Flat functions, one per concept, both directions co-located.** Leaf maps
  (`role`, `expolygon`, `point3`, `paint`, `wall`, `gcode`, `retract`, `path`)
  and marshal-in projections (`slice_region`, `perimeter_region`, the prepass
  `*_view`s) are plain named functions. **No `ToWit<World>` / `FromWit` /
  `Project` trait and no `world::*` ZST tags** — after ADR-0002 unification
  there is one Rust type per concept and therefore one converter per direction;
  a world parameter would encode variation that does not exist.

- **A shared `OriginBucket` (`marshal/origin.rs`) is the single home of the
  all-or-none origin-attribution + bucketing rule**, generic over the IR region
  accumulator. The three big output converters route through it. This is the
  module's deepest part and the unit-test surface: the rule becomes testable
  with plain `Vec`s and no component instantiated.

- **A structured `MarshalError`** (mixed/untagged origin, origin-length
  mismatch, non-finite float) carried internally, with `From<MarshalError> for
  String` so the crate boundary keeps today's `Result<_, String>` while tests
  assert on the variant.

- **`OriginId { object_id: String, region_id: u64 }`** replaces the two
  identical aliases `PerimeterRegionOrigin` / `SliceRegionOrigin`.

- **The `*Collected` accumulator structs move into `marshal`**
  (`marshal/accumulators.rs`); their builder *methods* stay on
  `HostExecutionContext`, so no `host ↔ marshal` cycle forms.

- **Per-stage harvest routing stays in `dispatch.rs`, co-located with the
  stage→export match (ADR-0006).** `deconstruct_layer_ctx` shrinks to a thin
  router that calls `marshal::convert_*`; the deep
  `harvest_layer_commit(stage_id, …)` form is rejected because it splits the
  stage taxonomy across two files with no compile-time guard keeping the harvest
  match in sync with the export match.

A precondition cleanup (sequenced first) deletes the stale per-world converter
copies from §Context.1 — a pure, behaviour-preserving deletion — before the
relocation into `marshal`.

## Consequences

- The origin-attribution rule and finite-float guards are tested once, in
  `marshal` unit tests, without instantiating a WASM component. This is the
  primary win.
- `dispatch.rs` keeps all wasmtime mechanics (pool/linker/store, export-name
  match, `DispatchError`) and the thin stage router; it gains a dep on
  `marshal`. `host.rs` builder impls write into `marshal::accumulators`.
- `marshal` depends only on `slicer-ir`, `slicer-core`, and the in-crate
  bindgen types — no `wasmtime`, no I/O.
- A future fifth world adds no marshalling ceremony: unified types mean its
  converters are the existing ones. If a genuinely world-divergent type ever
  appears (a type *not* remapped onto layer), that — and only that — would
  reopen the world-parameter question; this ADR is scoped to the
  post-unification reality where no such type exists.

## Alternatives considered

- **`ToWit<World>` / `FromWit` / `Harvest` / `Project` trait family with
  `world::*` ZST tags.** Rejected: its load-bearing justification
  (`ExtrusionRole` across two worlds) is deleted by the unification cleanup;
  one adapter per concept is a hypothetical seam, not a real one (ADR-0005's
  "two adapters = real seam" test fails). Adds turbofish ceremony at every leaf
  call site for compile-time world-correctness the unified types already
  guarantee.
- **A single deep `harvest_layer_commit(stage_id, &LayerOutputCollectors)`
  swallowing the stage match into `marshal`.** Rejected: splits the stage
  taxonomy — export routing in `dispatch.rs`, harvest routing in `marshal` —
  with no compile-time guard, the desync hazard the design flagged against
  itself. Keeping both matches in `dispatch.rs` is the honest locality.
- **Leave `*Collected` in `host.rs`** (the minimal design's choice to dodge a
  cycle). Rejected: the cycle is illusory because only the structs move, not
  the builder methods; data belongs with its transform.

## Verification

- `marshal` has no `wasmtime` reference:
  `! grep -rE 'wasmtime' crates/slicer-wasm-host/src/marshal/`.
- The all-or-none + bucketing rule appears once:
  `grep -rl 'any_tagged\|OriginBucket' crates/slicer-wasm-host/src/marshal/origin.rs`
  matches, and no `any_tagged` survives in `host.rs` / `dispatch.rs`.
- The stale duplicates are gone:
  `! grep -rE 'finalization_role_(ir_to_wit|wit_to_ir)|convert_postpass_role\b|ir_to_wit_expolygon_prepass' crates/slicer-wasm-host/src`.
- `grep -c 'bindgen!' crates/slicer-wasm-host/src/host.rs` stays 4 (ADR-0005
  untouched).
- `marshal` unit tests assert origin attribution / all-or-none / finite-float
  on `MarshalError` variants without instantiating a component.

## Cross-references

- ADR-0002 (WIT marshalling type unification) — the enabling decision; this ADR
  completes its "Deferred" per-world converter cleanup and relies on its
  one-Rust-type-per-concept invariant.
- ADR-0003 (per-world conversions stay generated in the guest macro) — the
  guest-side mirror; together they bracket the seam: guests generate per-world,
  the host marshals through unified types.
- ADR-0005 (runner traits in slicer-wasm-host) — untouched; `marshal` sits
  beneath the four runner traits, which keep their IR-typed seams.
- ADR-0006 (export-for-stage-id sole lookup) — the reason per-stage harvest
  routing stays beside export routing in `dispatch.rs`.

## Amendment (2026-06-16): the inbound role converter's per-world divergence was a latent bug, not legitimate variation

Implementing packet 113 surfaced a case that looks like a counterexample to
"one converter per concept": the WIT→IR `extrusion-role` converter is **not**
identical across worlds. The layer-world `convert_extrusion_role` recovers the
reserved builtin roles from their tags (`Custom(prime_tower_tag) => PrimeTower`,
`Custom(skirt_tag) => Skirt`), but the finalization copy (`finalization_role_wit_to_ir`) — a genuine WIT→IR
converter on the commit path — does **not**: it keeps `Custom(s) => Custom(s)`.
(The *outbound* IR→WIT converters are identical across all worlds.)

This is a **latent bug, not a real seam.** A `PrimeTower`/`Skirt` entity that
round-trips through a finalization guest returns as `Custom("…/skirt@1")`; the
immediately following `GCODE_EMIT` stage then misclassifies it (feedrate falls
back to `outer_wall_speed`, `;TYPE` becomes `Custom`, the skirt-travel filter
misses it). The pre-existing tests pin only the outbound encoding, so the lossy
round-trip was undetected.

**The postpass path differs (clarified during the 115 implementation):**
`convert_postpass_role` was a WIT→WIT field-identity cast — the postpass role
type *is* the layer role type post-remap — not a WIT→IR converter. The postpass
WIT→IR recovery always occurred downstream at `marshal/out.rs:539` via
`convert_extrusion_role`, so postpass never lost the typed role;
`convert_postpass_role` was a redundant cast. Packet 115 deletes it for
consistency (one recovering converter), with no postpass behaviour change.

The flat-function decision therefore **stands**: the correct end-state is a
single recovering `convert_extrusion_role`. The fix is **packet 115**, which
collapses the inbound converters to the recovering form and adds round-trip
regression tests. **Packet 113 (the behaviour-preserving extraction) does not
touch the divergence**: it relocates the two inbound converters into `marshal`
verbatim and excludes them from its dead-duplicate deletion set, so no
behaviour changes inside the refactor.

**Update (packet 115, landed):** the fix shipped. The two lossy variants
(`finalization_role_wit_to_ir`, `convert_postpass_role`) were deleted from
`marshal/leaf.rs`; the finalization (`host.rs`) and postpass call sites now route
through the single recovering `convert_extrusion_role` (the postpass path recovers
downstream at `marshal/out.rs:539`). Regression coverage added: a `marshal::leaf`
round-trip unit test and a finalization dispatch contract test
(`finalization_role_round_trip`) that was confirmed RED before the fix and GREEN
after.
