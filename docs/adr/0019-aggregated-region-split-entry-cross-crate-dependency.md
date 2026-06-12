# ADR-0019 — `AggregatedRegionSplitEntry` Cross-Crate Dependency (and Deferred Relocation)

## Status

Accepted (Packet 93 / TASK-region-mapping-cross-product). Status includes a follow-up commitment to relocate the type to `slicer-ir` in a future packet.

## Context

Packet 92 introduced `AggregatedRegionSplitEntry`, the aggregator output that pairs each region-split semantic name with its priority, value type, and the set of manifests that contributed it. The type was defined in `slicer-scheduler` because that's where manifest aggregation happens.

Packet 93 needed to feed the aggregate into `execute_region_mapping`, a pure-algorithm kernel that lives in `slicer-core::algos::region_mapping` (per Packets 84 and 87). That meant `slicer-core` had to depend on `AggregatedRegionSplitEntry` — and the kernel's signature became:

```rust
pub fn execute_region_mapping(
    // ...,
    aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>,
    // ...,
) -> Result<RegionMapIR, RegionMappingError>
```

Two options for resolving the type lookup:

1. **`slicer-core` depends on `slicer-scheduler`** to import the type directly.
2. **Move `AggregatedRegionSplitEntry` to `slicer-ir`** so both `slicer-scheduler` (the producer) and `slicer-core` (the consumer) depend on it through the shared IR crate.

Option 2 is architecturally cleaner — it matches the pattern other shared types follow (`PaintValue`, `RegionKey`, `ConfigDelta` all live in `slicer-ir`). But making the move during Packet 93 would have ballooned the packet's diff into a type-relocation that touched both the scheduler and the kernel. Option 1 was chosen pragmatically with the explicit understanding that the type would be moved later.

## Decision

**`slicer-core` declares a normal dep on `slicer-scheduler` for `AggregatedRegionSplitEntry` access, with an explicit follow-up to relocate the type to `slicer-ir`.**

Concretely:

- `crates/slicer-core/Cargo.toml` gains `slicer-scheduler = { path = "..." }` in `[dependencies]`.
- `slicer-core::algos::region_mapping` imports `AggregatedRegionSplitEntry` from `slicer-scheduler::region_split`.
- A circular-dep check is required before merge: `cargo tree -p slicer-core --edges normal` must show `slicer-scheduler` once but must NOT reach back through `slicer-runtime` or `slicer-wasm-host`. (`slicer-scheduler` is wasmtime-free per ADR-0007 / Packet 85, so the back-edge is structurally impossible.)
- The pure-kernel test path in `slicer-core/tests/` can construct a `BTreeMap<String, AggregatedRegionSplitEntry>` directly without spinning up the scheduler.
- **Follow-up commitment:** relocate `AggregatedRegionSplitEntry` (and any other types in the same boat) to `slicer-ir` in a future packet. `slicer-scheduler` will re-export them for backwards compatibility during the transition. The cross-crate dep on `slicer-core` is removed in the same packet.

## Consequences

- **Packet 93 shipped on schedule.** The kernel got its parameter without forcing a same-packet type-relocation across two crates.
- **The dep graph carries a stylistic blemish.** `slicer-core` is otherwise a pure-geometry/pure-algorithm crate with `slicer-ir` as its only first-party dep. The `slicer-scheduler` edge is the one exception.
- **No circular dep introduced.** `slicer-scheduler` does not depend on `slicer-core` (verified at Packet 93 close). The relationship is `slicer-scheduler → slicer-ir ← slicer-core` plus the new `slicer-core → slicer-scheduler` edge — strictly directional.
- **The follow-up packet has clear acceptance criteria:** (a) move `AggregatedRegionSplitEntry` and any related types to `slicer-ir`; (b) drop the `slicer-scheduler` dep from `slicer-core`; (c) verify `cargo tree -p slicer-core --edges normal` shows only `slicer-ir`, `slicer-helpers`, and leaf utility crates.
- **Future "I need a scheduler type in a kernel" requests should default to "put the type in `slicer-ir`".** This ADR exists specifically to record the one case where that didn't happen and to commit to fixing it.

## Rejected alternatives

- **Inline `AggregatedRegionSplitEntry` into `slicer-core`.** Duplicates the type; `slicer-scheduler` would produce one and `slicer-core` would consume a structurally-identical-but-distinct one. Trait-based bridging gets ugly. Rejected.
- **Pass the aggregate as a flat `&[(String, u32, ValueType)]` instead of the rich struct.** Loses the conversion ergonomics; the kernel would have to re-construct context the scheduler already computed. Rejected.
- **Move the relocation into Packet 93 itself.** Doubled the packet diff and risked cross-crate breakage in a packet that was already mid-scope on the kernel signature. Deferred deliberately.

## Future reviewers

- This ADR is open until the relocation packet lands. After relocation, this ADR should be amended (`Status: Closed — superseded by Packet NN`) and the `slicer-core → slicer-scheduler` dep removed.
- Do not "fix" the dep by inlining the type or by bridging via trait. The agreed fix is type relocation to `slicer-ir`.
- New cross-crate dep requests of the same shape (kernel needs a scheduler type) should default to "put the type in `slicer-ir`", citing this ADR as the precedent worth avoiding.
