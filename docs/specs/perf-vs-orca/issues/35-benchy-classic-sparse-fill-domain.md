# Benchy classic sparse-fill domain at middle layers

Type: task
Status: resolved
Blocked by: 28
Assignee: wayfinder session (ses_f14767b8cffePrly8sT2Su6Hrv), 2026-09-29

## Question

Why does matched Benchy/classic/supports-off emit a large sparse-infill path
where PNP Arachne emits almost none and Orca classic/arachne emit none? At
global layer 104 (Z 21 mm), [Classic output-volume surplus](28-classic-output-volume-surplus.md)
measured 2,562 PNP classic sparse XY extrusion segments versus one PNP
Arachne segment; both historical Orca runs recorded zero sparse segments there.
The mismatch persists across nearby layers and dominates PNP's classic-vs-
Arachne Benchy output delta. This is a fill-geometry/classification question,
**not** permission to delete paths or imitate the shorter generator.

Compare the same layer's `SliceIR` top/bottom/bridge/sparse claims before and
after `sync_perimeter_infill_areas_into_slice`, the two generators'
`PerimeterIR.infill_areas`, and the gyroid input/output. Find the first
divergent ownership or perimeter-inset boundary; use a real-layer oracle
(the user-supplied `tmp/3dbenchy.stl` stays gitignored) and a committed small
fixture if the root cause is reproducible without that model. Check whether
Orca classifies the disputed interior as solid, wall, void or sparse before
claiming canonical correctness. If a safe correction exists, measure it under
the map's output-disclosure and ordinary+accelerated paired gate; measured
keep/drop returns to the human. Do not weaken canonical parity to reduce
bytes. Source counts, layer CSVs and scripts:
`evidence/t28-output-volume/FINDINGS.md`.

## Answer

**Resolved 2026-09-29: the divergence is in the *linker's commit protocol*, not in
the fill domain. The infill linker clips the raw gyroid waves to nothing on these
layers, and the host reads that empty output as "committed nothing" and preserves
the raw, unclipped waves instead.**

Reproduced on the unmodified committed tree (`cargo xtask build-guests --check`
exit 0, release host), then localized with a temporary in-guest probe
(`evidence/t35-fill-domain/probe.patch`, removed before close):

1. `GyroidInfill::fill_expolygon` (`modules/core-modules/gyroid-infill/src/lib.rs`)
   emits raw waves over the gyroid bbox expanded by `10 × spacing_mm` — "Emit raw
   — no clipping (ADR-0025 degraded-not-failed)". Overshoot is by design; the
   linker owns clipping.
2. `RoleBoundaries::for_role` / `link_paths_without_offset`
   (`modules/core-modules/infill-linker/src/orchestrate.rs`) is supposed to remove
   it. On classic layer 104 the probe measured **56 raw waves / 1,741 mm in and
   0 paths kept** against both the offset boundary and the raw fallback. Arachne
   layer 104: 56 waves / 1,782 mm in, **1 path / 2 mm kept**.
3. `deconstruct_layer_ctx` (`crates/slicer-wasm-host/src/dispatch.rs`) returns
   `Ok(None)` when the invocation's buckets are all empty, and `layer_executor`'s
   apply treats `None` as "committed nothing" — so the prior `InfillIR` from
   `Layer::Infill` survives (ADR-0028 §Amendment Change 3). That preserved IR is
   the raw envelope, which then prints.

**Impact measured.** Classic: 60 layers with zero linker output carry **54,324 of
79,940 printed sparse mm (68.0%)** and 72,135 of 106,442 printed sparse segments
(3,054,585 of 4,486,477 sparse bytes). All 37 of its >1,000-segment layers are in
that set. Arachne: 16 layers, 7,566/36,219 mm (20.9%). At layer 104, **88.7% of
the revived 1,741 mm path lies outside the part cross-section** (1,545.0 mm
outside, 196.1 mm inside, segment-midpoint polygon test against the layer's own
`SliceIR` cross-section). The printed sparse envelope is 59.0 × 53.1 mm against a
36.8 × 30.2 mm wall bbox.

**Base fixture is consistent.** Neither generator logged a single all-empty
linker layer on base (492/493 layers), and base classic prints *fewer* sparse mm
than base Arachne (164,905 vs 174,242) — matching ticket 28's finding. The
mechanism explains the fixture divergence: base's cross-sections rarely make the
sparse clip vanish entirely; Benchy's do at these layers.

**This is a containment hole of the class ADR-0025's 2026-07-24 amendment closed
inside the linker.** The amendment is explicit that a known-empty role boundary
is not an unknown one ("`Some(empty)` means … the role's paths have nowhere legal
to go and clip away"). The linker honours it; the empty-output protocol overrides
it one stage later, making a *verdict* indistinguishable from an *absence*.

The arithmetic makes the containment claim independent of any fill-pattern
disagreement: layer 104's sparse claim is 17.7 mm², so even at 100% density a
0.4 mm path can cover at most 44.2 mm, while 1,741.2 mm prints — **39× the
absolute upper bound**.

No fix was made: changing the empty-output/preservation protocol is a
runtime-contract change (ADR-0028 Change 3, ADR-0025 containment contract) that
needs its own packet and the map's paired A/B + output-disclosure gates. A
candidate shape is recorded in the evidence, not recommended. Geometry
correctness of Arachne/Orca is not adjudicated, no speed claim is made, and
`ERR_MALFORMED_LAYER_MARKER` warnings were not investigated.

Evidence: [`evidence/t35-fill-domain/FINDINGS.md`](../evidence/t35-fill-domain/FINDINGS.md),
`linker-vs-printed.csv`, `probe.patch`.
