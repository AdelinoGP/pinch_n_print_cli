# Requirements: 156-arachne-region-order

## Metadata

- Packet status: `draft`
- Backlog source: G12 in `docs/18_arachne_parity_audit.md`
- Task IDs: none

## Problem

The previous G12 implementation added a partial core reorder but did not
faithfully port Orca's constraint construction, applied it before final line
post-processing, collapsed `wall_sequence` into a boolean, dropped that value
at the WASM boundary, and allowed the perimeter module and path optimizer to
override the result. A green direct-core fixture therefore did not establish
production parity.

## Requirements

1. `get_region_order` must reproduce canonical pair exclusions and unique
   precedence relation semantics before the walk consumes it.
2. `SparsePointGrid` must remain a sparse candidate index, not an independent
   geometry-policy filter.
3. The topological walk must consume only canonical acyclic constraints and
   match Orca's finalized-line candidate behavior. Remove PnP-only recovery.
4. Region order must occur after all Arachne line post-processing.
5. The existing three `wall_sequence` configuration values must be represented
   as a three-state boundary value, carried through WIT/SDK/host unchanged,
   and resolved only by the perimeter module.
6. The module must commit a sequence-aware `WallLoop` order, and the optimizer
   must preserve it rather than apply a contradictory role grouping.
7. Tests must prove core, guest-host, committed-wall, and end-to-end behavior
   for `InnerOuter`, `OuterInner`, and `InnerOuterInner` on layer 0 and later
   layers where the modes differ.
8. Documentation must accurately describe the canonical stage, references,
   WIT change, ownership boundary, and any remaining intentional deviation.

## Acceptance Summary

The acceptance criteria in `packet.spec.md` are the canonical executable
contract. No criterion is satisfied solely by a direct `run_arachne_pipeline`
test; mode propagation and committed output require real module/WASM evidence.

## Verification Commands

Use the AC commands in `packet.spec.md`, then:

```text
cargo xtask build-guests --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p slicer-core
```

## Out of Scope

G11, G15, G20, unrelated path-optimization travel heuristics, and new config
keys. WIT/SDK/host changes needed to preserve the existing config are in scope.
