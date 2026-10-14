# Infill-linker batch-execute experiment

Type: task
Label: wayfinder:task
Status: resolved (2026-10-13; human KEPT the batch-execute candidate; see
## Answer)
Assignee: claimed and resolved by the 2026-10-13 OpenCode wayfinder session
(human's continue-the-map invocation; claim recorded before any work)
Graduated from
[Infill-linker post-hoist residual and exact clip-work reduction](62-infill-linker-post-hoist-residual.md)
(2026-10-13); execution handed off separately
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 62 (resolved)

## Question

Does batching each `link_paths_without_offset` invocation's paths into one
clipper engine + one execute — with per-subject output attribution re-derived
exactly — preserve byte-identical output on the matched cells and pass the
standing paired ordinary + accelerated uninstrumented wall/CPU gate?

The attribution is complete
([evidence](../evidence/t62-linker-post-hoist/FINDINGS.md)): per-path
execute + engine_add are 90.38% of linker fuel (85.96% + 4.42%, arachne supports-off
ordinary; classic 83.99% + 5.44%), the prepared universe is re-fed 57.3×
(arachne) / 49.5× (classic) per preparation, and the per-call cost is
universe-dominated. Assume-cost ceilings: linker fuel → ≈12.58% of current
(uniform-by-invocation; layer-aware bound ≈10.1%), slice guest share
44.28% → ≈5.57% on the primary cell. These are estimates, not measurements.

## Scope and decision evidence

1. Implement the batched candidate in
   `PreparedPolylineClip`/`link_paths_without_offset` only. Preserve:
   per-path output identity/ordering, `remove_short_polylines` filtering,
   `point_with_metadata`, `source_segments`/`majority_owner`, the
   [ran-and-empty replacement contract](37-infillpostprocess-empty-output-protocol.md),
   and the raw-boundary fallback arm (each arm is an ordinary invocation).
2. Re-derive per-subject attribution exactly and prove it: the vendored
   `clipper2-rust` 1.1.0 `Clipper64` exposes no owner tags on open-path
   solutions; each output open path is a contiguous clipped span of exactly
   one subject. The mechanism needs an independent replay oracle on real
   inputs (ticket 62's scope item 3) before any timing claim. Subject–subject
   adjacency (touching gyroid spans) is the one behavioural risk to test.
3. Qualify fresh clean outputs in both modes on the full benchy matrix before
   timing; the ceiling arithmetic is not a measurement.
4. Run the standing paired ordinary + accelerated uninstrumented wall/CPU
   gate (6 pairs/cell on the pre-declared linker-bearing cells; cpu/wall
   ratios quoted per sample, starved samples excluded per §10.3).
5. Native rerouting stays prohibited unless its own take measures native
   work; displaced guest fuel proves nothing about wall.

## Acceptance and boundaries

Measured keep/drop to the human; candidates are never auto-committed. The
linker's own wall opportunity is inherited evidence (t61's profiling-off
layer-1 dispatch brackets 3.182/3.191 s), not a predicted saving; fuel→wall
transfer is the paired A/B's question. Independent of
[low-layer dispatch granularity](63-low-layer-dispatch-granularity.md): their
timing campaigns must not overlap, and a linker-cost change requires a fresh
scheduling baseline for any later scheduling diagnosis.

Evidence: [t65 campaign findings](../evidence/t65-linker-batch/FINDINGS.md)
([post-hoist attribution findings](../evidence/t62-linker-post-hoist/FINDINGS.md)).

## Answer

**Resolved (2026-10-13) — output preservation proven exactly; the standing
gate recorded GATE NOT MET on one wash cell and the human KEPT the
candidate anyway, explicitly.** Evidence:
[evidence/t65-linker-batch/FINDINGS.md](../evidence/t65-linker-batch/FINDINGS.md)
(`probe-candidate-combined.patch` (probe, removed from the tree before any
measured build), `qualification.json`, `ab-reduction.json`, `oracle.json`,
`reduce_ab.py`).

Mechanism (scope items 1-2): `PreparedPolylineClip::clip_batched`
(`crates/slicer-core/src/polygon_ops.rs`) runs one `Clipper64` + one
execute per `link_paths_without_offset` invocation and re-derives per-subject
attribution by tolerant segment incidence, with per-subject `clip` repair
for ambiguous subjects (junction kisses) and whole-call fallback reserved to
unattributable segments; every downstream step keeps its exact prior
semantics. 10 differential tests (`crates/slicer-core/tests/polygon_ops_clip_batched_tdd.rs`);
in-guest replay oracle (probe build) **PASS on all 11 captured cells in
both modes — 0 mismatches over 12,022–19,290 subjects**, with 2,037–4,284
subjects repaired per job and 1 whole-call fallback total; measured
batch-vs-replay clip fuel −43.6…−65.7%.

Qualification (scope item 3): fresh 16-cell matrix, **16/16 PASS**,
byte-identical between arms and to the preserved t58 production references.
Paired gate (scope item 4), 8 pre-declared arachne cells (justified in the
findings' pre-declaration): **6/6 pairs retained in every cell, zero
starvation exclusions; benchy arachne wall −20.01/−21.08 % supports-off and
−12.24/−10.84 % supports-on (CPU corroborating, −2.97…−4.77 %); base cells a
wash (−0.07/−0.52/−1.65 %, and ordinary-base-arachne-on +0.43 % — a median
loss inside overlapping ranges, CPU corroborating)**. GATE NOT MET per the
both-clauses/both-modes rule; measured KEEP/DROP returned to the human.

**Decision: the human KEPT the candidate 2026-10-13**, accepting the wash
cell explicitly on the t27 precedent; the committed tree now carries the
batch (production artifacts already match it; both guest-freshness gates
green). Scope item 5 held: native rerouting was not touched. Standing note
for follow-ons: this is a linker-cost change — any later scheduling
diagnosis needs a fresh scheduling baseline per the acceptance boundaries.