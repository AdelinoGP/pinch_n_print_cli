# ADR-0018 — Region-Split Priority Registry and Canonical `BTreeMap` Ordering

## Status

Accepted (Packet 92 / TASK-region-split-manifest).

## Context

Packet 92 introduced the `[[region_split]]` manifest array: every module declares which paint semantics it wants the host to expand into separate regions via cross-product on `variant_chain`. The host aggregates entries across all manifests, then drives Packet 93's RegionMapping cross-product from the aggregate.

The aggregate's ordering is non-trivial. Two things depend on it:

1. **Determinism.** RegionMapping uses the aggregate to enumerate `variant_chain` combinations; the order in which semantics are iterated determines the byte representation of the resulting `RegionMapIR` and (transitively) the layer dispatch order, the test-fixture hashes, and the G-code output. A `HashMap` would give different orderings on different runs, breaking byte-identical regression tests.
2. **Per-stage precedence.** When two semantics declare the same priority, a tiebreaker is needed. When a community semantic claims a priority that conflicts with a core one, the rule has to be clear at manifest-load time, not at runtime.

The Packet 92 design needed three things locked in one decision: the storage container, the priority encoding, and the validation rules.

## Decision

**Region-split semantics are aggregated into a canonical `BTreeMap<String, AggregatedRegionSplitEntry>` ordered by `(priority, name)`, with priorities drawn from a fixed registry for core semantics and a floor for community semantics.**

Concretely:

- **Storage:** `BTreeMap`, not `HashMap`. Iteration order is keyed on the `String` semantic name. Tests can rely on the exact order. Determinism is guaranteed by the container.
- **Core priority registry** (locked, in source):

  ```rust
  static CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)] = &[
      ("material",   100),
      ("fuzzy_skin", 200),
  ];
  ```

  Core semantics MUST declare exactly the registered priority. Any other value is a fatal `LoadErrorKind::CoreSemanticPriorityMismatch`.
- **Community floor:** `COMMUNITY_PRIORITY_FLOOR = 1000`. Community semantics (anything not in the core registry) MUST declare `priority >= 1000`. Lower priorities are a fatal `LoadErrorKind::CommunityPriorityBelowFloor`.
- **Tied-priority warning:** if two distinct semantics from different manifests share a priority, the host emits a non-fatal `LoadDiagnostic { level: Warning, ... }` naming both manifests, the shared priority, and the lexicographic tiebreaker. Aggregation continues.
- **Per-manifest validation** rejects: duplicate semantic within one manifest; `value-type = "scalar"` (scalar paints route through `segment_annotations` instead — see `docs/02_ir_schemas.md`).
- **Canonical iteration order in cross-product:** semantics by `BTreeMap` order (effectively name-sorted within priority tier). Within each semantic, `PaintValue` instances are ordered `Flag(false) < Flag(true) < ToolIndex(0) < ToolIndex(1) < … < Custom(s_lex)`. This is the order Packet 93's `enumerate_canonical_chains` produces.

## Consequences

- **Byte-identical regression tests work.** Two consecutive `pnp_cli slice` runs produce the same `RegionMapIR` bytes and the same G-code SHA, because the aggregation is deterministic at every level.
- **Core semantics get short, memorable priorities.** `material = 100`, `fuzzy_skin = 200`. Adding a third core semantic in the future means picking a new slot in the 100–999 range with an ADR.
- **Community semantics have a clear runway.** Anyone shipping a custom module declares `priority >= 1000` and reasons about ordering relative to other community modules without colliding with core slots.
- **Tied priorities are observable, not silent.** The WARN diagnostic surfaces in `pnp_cli dag` output and in module-load logs so authors can adjust before tests start hashing wrong.
- **Variant-chain enumeration order is contract.** Test fixtures and integration tests lock the exact sequence. Reordering breaks every region-split test; any future change requires a coordinated packet that updates the fixtures.

## Rejected alternatives

- **`HashMap` storage.** Non-deterministic iteration; would break byte-identical tests. Rejected.
- **Floating priorities (no registry).** Without a registry, two community modules could claim `priority = 0` and force an unresolvable tie at every load. Rejected.
- **A separate "core / community" namespace prefix instead of a numeric floor.** Adds a string-parsing step at every priority comparison; no real benefit over a numeric floor. Rejected.
- **Allow `value-type = "scalar"`.** Scalar paint values cannot be cross-producted into `variant_chain` (the chain key would carry an `f32`, which is not hashable without `to_bits()` and would explode the cross-product cardinality). Rejected; scalars route through `segment_annotations` instead.

## Future reviewers

- Do not migrate `aggregated_region_split` to `HashMap`. The `BTreeMap` is load-bearing for determinism.
- Do not add a new core semantic without a follow-up ADR. The 100/200/… registry is small on purpose.
- Do not relax the `COMMUNITY_PRIORITY_FLOOR`. If a community module needs to slot near a core semantic, the right move is to add a new core slot via ADR — not to lower the floor.
- Do not change the `PaintValue` ordering in `enumerate_canonical_chains` without updating every region-split test fixture in the same packet.

## Amendment — 2026-11-13 (packet proptest-openrscad_07_pbt-scheduler): priority/name helper, current RegionMapping order, and tied-priority payload

This amendment distinguishes the scheduler's ordering contract from the current RegionMapping implementation and clarifies the diagnostic payload. Where historical descriptions of map iteration, actual cross-product implementation or tie-warning provenance conflict with the clarification below, they are superseded by it. It changes no policy and authorizes no production reordering.

- **Container and scheduler helper.** `aggregate_region_splits` stores entries in a `BTreeMap<String, AggregatedRegionSplitEntry>` keyed and iterated by semantic name. The separate `canonical_variant_chain_order` helper (`crates/slicer-scheduler/src/region_split.rs`) returns the `(priority, name)` sequence specified for canonical priority ordering; raw map iteration is not that sequence in general.
- **Current RegionMapping driver.** `commit_region_mapping_builtin` (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`) passes `plan.aggregated_region_split` to `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`), which passes the map's name-ordered keys to `enumerate_canonical_chains` (`crates/slicer-ir/src/region_split_registry.rs`). The enumerator preserves the supplied axis order; the scheduler helper has no production caller in the current tree. Thus actual cross-product chain axes are name-ordered even where that differs from the scheduler's `(priority, name)` sequence. This source-observed discrepancy is not a decision to replace priority ordering with name ordering; its driver-level audit and any corrective change require separate coordinated work and fixture/compatibility review.
- **Tied-priority diagnostic payload.** One warning is emitted per priority shared by two or more distinct semantics, including semantics declared by the same module. Its `level` is `Warning`, `path` is an empty `PathBuf` sentinel and `field` is `Some("region_split")`. The message names the shared priority, tied semantic names, each semantic's declaring module IDs, and the lexical tiebreaker order; it does not name both manifest paths.
- **Unchanged.** `BTreeMap` storage, `CORE_REGION_SPLIT_PRIORITIES`, `COMMUNITY_PRIORITY_FLOOR`, declaration-only aggregation with no implicit seeds, and the established `PaintValue` ordering remain unchanged. `scan_paint_data` (`crates/slicer-core/src/algos/region_mapping.rs`) supplies values in that order; `enumerate_canonical_chains` preserves, rather than sorts, the caller's value vectors.
