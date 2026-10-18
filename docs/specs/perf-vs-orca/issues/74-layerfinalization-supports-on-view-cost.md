# LayerFinalization supports-on whole-print view cost

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed — graduated by the 2026-10-18 base refresh; ticket creation
is not execution approval
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Why do the four whole-print LayerFinalization module brackets jump from
0.30–1.39 s on supports-off to **45.0–48.5 s** on supports-on, when their
combined guest fuel is **5.45 B of 12,964 B (0.042 %)** of the slice?

Fresh base captures ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md),
instrumented observer): classic-on modules 10.11 / 11.61 / 11.07 / 12.25 s
(classifier / part-cooling / skirt-brim / wipe-tower), stage 55.65 s; arachne-on
11.42 / 12.20 / 12.48 / 12.40 s, stage 60.22 s; supports-off controls
0.30–1.39 s total. The `overhang-classifier-default` early-return audit
([Consume-only-when-read annotations](20-consume-only-when-read-annotations.md))
says the matched configs return before reading a stamp, yet its bracket is
10–12 s on supports-on. Ticket 58 already removed the old per-module
whole-print copies (`execute_layer_finalization_with_instrumentation`
(`crates/slicer-runtime/src/layer_finalization.rs`)); the remaining bracket is
therefore expected to be host-side input-view construction
(`push_finalization_layer_view` (`crates/slicer-wasm-host/src/host.rs`),
`dispatch_finalization_call` (`crates/slicer-wasm-host/src/dispatch.rs`)),
guest dispatch, and output merge (`apply_finalization_pushes`
(`crates/slicer-wasm-host/src/dispatch.rs`)) — this is a hypothesis, not a
measured split.

Deliverables:

1. Decompose one supports-on cell's four brackets into host input-view
   construction, guest execution (including the early-return path) and output
   merge; state which term owns the 30–150x support-state jump and why the
   support entity volume is not proportional to the output-byte ratio.
2. Identify and test the cheapest representation-preserving reduction:
   materialize only the entity classes a module declares it reads, skip a
   module whose own liveness predicate is false, share an immutable input view
   where the contract permits, or a bounded delta protocol. Do not change
   output semantics.
3. If a candidate survives, qualify it on the 16-cell output-identity matrix
   against the preserved references and run the standing paired ordinary +
   accelerated A/B on the four pre-declared supports-on cells. Keep/drop is the
   human's.

Constraints: attribution first; no production edit before a measured owner;
output byte-identity is the hard gate; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Acceptance

- A measured bracket split and a named owner for the supports-on jump.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.
