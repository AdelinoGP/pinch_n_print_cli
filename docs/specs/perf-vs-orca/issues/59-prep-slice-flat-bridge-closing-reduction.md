# PrePass::Slice flat-bridge closing reduction: representation-identical analysis

Type: task
Label: wayfinder:task
Status: resolved (2026-10-14; verdict (b) — no representation-safe reduction; artifact-bound gate; no production edit, no timing campaign; see ## Answer)
Assignee: 2026-10-14 OpenCode wayfinder session (map resumption; first frontier ticket in order)
Graduated from the map's fog by the 2026-10-09 perf refresh
([evidence](../evidence/perf-refresh-2026-10-09/FINDINGS.md)).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
builds the flat-bridge mask with
`closed = closing_ex(&support, FLAT_BRIDGE_ENCLOSURE_RADIUS_MM, closing_join)`
and `refilled = difference(closed, support)`. Fresh attribution at HEAD
(2026-10-09 refresh) puts `polygon_ops::closing_ex` under `host:slice` as the
largest host scope on the base cells — **384–389 s accumulated thread-time**
(470 calls, ~0.82 s/call) on base classic supports-on/off and ~24–25 s (240
calls) benchy — with the same cluster carrying `offset` 127.6 s and
`clip_polygons` 48.3 s accumulated; `PrePass::Slice`'s own stage wall is 47–56 s
on base (ticket 27 probe, uninstrumented).

The prior candidate was falsified on a real layer, not on algebra: avoiding the
full-layer `difference` via the set identity changed `SliceIR.bridge_areas`
(105 candidate polygons vs 104 legacy on benchy layer 1 — see
`evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md`). Representation is the
binding constraint.

Deliverable — representation analysis first:

1. Enumerate what actually consumes the bridge mask / closing result and what
   representation each consumer needs (`derive_needs_support`'s bbox prefilter +
   exact intersection, bridge assembly, the visual-debug taps — per ticket 27's
   premise audit), on real layers.
2. Either (a) a candidate reduction proven representation-identical against a
   real-layer IR differential oracle — the prior attempt's lesson: the synthetic
   fixture is not an oracle — or (b) a documented **no safe reduction** verdict
   with the evidence.

Only after (a) may the standing paired A/B run.

## Acceptance

For (a): representation identity on real layers first, then the paired ordinary
+ accelerated A/B per the standing metric; measured keep/drop to the human; no
auto-commit. For (b): the verdict is a resolution with the representation
evidence attached; no code change.

## Notes

- This ticket carries the map's previous "flat-bridge reduction inside
  `PrePass::Slice`, representation analysis first" fog patch; the `apply_opening`
  geometry-parity question stays separate and out of scope for this map.
- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md` (host table),
  `evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md`,
  `evidence/t27-serial-floor/FINDINGS-SUBSTAGE-2.md` (probe-2 stage fuel),
   `evidence/t27-serial-floor/PREMISE-AUDIT.md` (consumer audit).

## Comments

### Benchy premise refresh — 2026-10-13 (no take started)

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
confirms `host:slice` still leads accumulated native work across both generators
and support states. `closing_ex` is 23.069–25.128 s accumulated thread-time
(240 calls in each profile), not slice wall. Profiling-off Arachne stage
brackets are 2.773/2.609 s off/on. The real-layer representation constraint and
this ticket's question remain unchanged; no replacement or timing gate was
started. Evidence: [fresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md).

### Claim — 2026-10-14 (map resumption; claim recorded before any work)

The frontier was re-derived on resume: the open front is this ticket, [60],
and [20] (parked below-fold); per the t63/t64 convention this session took the
first frontier ticket in order. Take plan, per this ticket's own acceptance:
(1) enumerate the real-layer consumers of the flat-bridge mask / closing
result and the representation each consumer needs; (2) attempt a reduction
shape against a real-layer IR differential oracle (the prior attempt's lesson:
the synthetic fixture is not an oracle) — either a candidate proven
representation-identical (then the standing paired ordinary + accelerated A/B,
keep/drop to the human, no auto-commit) or (3) a documented no-safe-reduction
verdict with the differential evidence. No production edit and no timing
campaign are authorized by the claim itself.

## Answer — resolution comment, 2026-10-14

**Verdict (b): no representation-safe reduction is established; the enclosure
closing call is irreducible without changing output. No production edit, no
code change, no timing campaign (the standing paired A/B's precondition —
"only after (a) may the standing paired A/B run" — is not met).**

Evidence: [t59 findings](../evidence/t59-flat-bridge-closing/FINDINGS.md),
`reduced/` (aggregates + drilled layers), `reduce_t59.py`, `verify.py`
(exits 0 against the in-repo reduced set), `probe.patch` (the temporary
harness, removed from the tree).

**Deliverable 1 — consumers/representation (real layers).** The closing result
is consumed only as the per-component ≥10 % overlap gate inside
`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`);
the reported geometry is the untouched component. The gate's output flows,
with exact geometry, to `gate_bridge_areas_by_unsupported_span` /
`update_external_bridge_orientation`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`),
`sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`; `sparse_infill_area`
material exclusion), the guest consumers (`wave-overhangs`,
`classic-perimeters` per-vertex `is_bridge`, `gyroid-infill`) and the
visual-debug SliceIR capture. Representation identity at the region level is
therefore binding.

**Deliverable 2 — the candidate family is refuted by measurement.**

1. **Local-closing candidate** (support clipped to the flat_bridge bbox padded
   by `2R + 1 mm`, then the same gate) **flips verdicts on 92/239 benchy
   layers (38.5 %) and 320/469 base layers (68.2 %)** — not representation
   identical. Flips are structural, not jitter: components move between
   0.99→0.05 and 0.0→1.0.
2. **Mechanism, drilled on both fixtures** (`reduced/mechanism.json`):
   `support = difference(region.polygons, unsupported_region)` is **one large
   piece plus ~72–74 degenerate sliver paths** (areas ≈1e-5 mm², max extent
   ~1.9 × 0.1 mm). Closing the raw path list vs the same point set re-emitted
   by `intersection`/`union_ex`: benchy layer 161 — 325.304 mm² vs 215.4 /
   215.381 mm² (large piece alone 127.902); base layer 8 — 31.536 mm² vs
   72.1 / 72.053 mm² (large piece alone 73.170, support itself 72.1). On base
   layer 8 the raw closing is even **smaller than its own input point set**:
   mitered offset artifacts from the slivers (miter limit
   `ORCA_MORPH_MITER_LIMIT` = 3.0, `morph_passes`/`closing_ex`,
   `crates/slicer-core/src/polygon_ops.rs`) reshape the NonZero cleanup
   decisively. **Any change to the input path list, call sequence, or offset
   configuration moves the artifact field and flips gate verdicts.**
3. **The skip family is empty.** Far-rejection guards have zero hits (0/3,266
   benchy and 0/11,685 base components have bbox⊕R disjoint from the support
   bbox), corroborating ticket 27's record. The baseline gate itself is
   knife-edge (min |fraction − 0.1| = 4e-5 / 2.4e-4; 13/63 components within
   0.02). Prior shapes stay refuted (refilled-elimination t27; collinearity
   cleanup ~2 %; bbox guard 0 hits).
4. **Cost ceiling of the refuted shape, measured anyway:** the local shape is
   only ~12 % cheaper in accumulated thread-time (benchy 17.109 s vs
   19.416 s; base 353.676 s vs 403.713 s) because the flat-bridge bbox covers
   ~half the support bbox (mean ratio 0.519 / 0.494). No gap-closing scale even
   if exact.

**Qualification/disclosures.** Reconstruction fidelity against
`execute_prepass_slice_single_layer_with_cache` was byte-asserted on sampled
layers and both drilled layers; the harness reproduces production inputs
exactly (bed placement, `layer_top_zs` schedule — 240/495 layers,
`slice_closing_radius` 0.049, miter join). Timings are single-run in-process
accumulated thread-time (attribution only, never wall). Host-stage coverage
argument for both generators and support states is stated in the findings.
Interruption/load context: single machine, background load not controlled; no
uninstrumented campaign ran.

**Route:** nothing graduates from this take (the family is closed; the
artifact-bound gate is recorded in the findings as current canonical behaviour
this stage consumes, not as route work). Frontier after this resolution:
[host:native attribution](60-host-native-attribution.md) first,
[consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
still parked below-fold.
