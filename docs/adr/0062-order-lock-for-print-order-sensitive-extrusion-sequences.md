# ADR-0062 Order lock for print-order-sensitive extrusion sequences

Status: accepted (landed with packet 244).

## Context

Wave-overhang bridge fill produces paths whose print order and direction are
physically load-bearing: fronts must be deposited anchored-first, and chained
zigzag runs break if reversed. Two downstream stages destroy such sequences
today: the infill linker re-clips, chains, and reverses bridge-role paths; path
optimization nearest-neighbor permutes role groups and may reverse entities.
Adding a dedicated role variant was rejected as one module's need hardcoded into
every consumer's match arms.

## Decision

Add additive `order_lock: Option<u64>` to `ExtrusionPath3D` (WIT
`extrusion-path3d.order-lock`) and project onto `OrderedEntityView`.
None/absent preserves today's behavior exactly. Paths sharing a tag within one
`(layer, object, region)` form an **atomic contiguous sequence**: they stay
adjacent, in authored order and point direction; the block may move as a unit
relative to unrelated entities. Locks protect sequence and geometry (points,
widths); speed/flow side mutations remain legal. The host enforces the invariant
at every mutation point — InfillPostProcess commit, path-optimization proposal
application, finalization merge, and cross-layer tool-cluster rotation —
rejecting violations with a fatal, atomic contract error. G-code emission
bypasses D-P and min-segment pruning for locked paths.

Tags are allocated by the producing module through an SDK allocator type
(invocation-local, from 1, deterministic discovery order; `Some(0)` rejected).
The host remaps local tags to layer-unique global tags (bit 63 set) at every
output boundary; unknown global tags in module output are a contract error. Any
layer producer, InfillPostProcess module, or finalization module may mint locks;
consumers honor the field without knowing the producer.

## Considered options

- New `ExtrusionRole` variant (rejected — role proliferation, scattered match
  arms).
- `Custom("…")` string convention (rejected — invisible typing, per-consumer
  string matching).
- Entity-group wrapper type in InfillIR/LayerCollectionIR (rejected —
  restructures every downstream iteration for a guarantee a field carries).
- Trusting core modules to comply without host enforcement (rejected —
  third-party optimizers and future finalization sorters could silently violate
  the invariant).

## Consequences

- IR/WIT additive change → one-time guest rebuild; host gains tag remapping and
  mutation-point validation; linker gains locked-passthrough + swept carve
  branch (ADR-0026-consistent module-local code); optimizer treats locked blocks
  as single non-reversible candidates; emitter skips simplification for locked
  paths; production literals gain one field.
