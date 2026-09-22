# Wall-flags annotation-free fast path

Type: task
Status: resolved

## Question

Should `build_wall_flags` (`crates/slicer-core/src/perimeter_utils.rs`) skip
wall-paint reprojection and return the normal boundary fallback when no
effective material or fuzzy annotation can affect the wall?

## Answer

**KEEP** — committed ("perf: skip wall paint reprojection without effective
annotations"), 2026-09-07 (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §7a/§13, `docs/specs/perf-vs-orca/evidence/perf-flags/`).

Implementation: `build_wall_flags` seeds `variant_fuzzy` and returns the
outer/inner boundary fallback when no effective annotation can affect the wall;
material `ToolIndex` and fuzzy `Flag(true)` keep the original path. The
predicate preserves the original selection rules (index mode: selected polygon;
reprojection mode: all reachable originals). Callers, geometry tolerances, and
host/WIT contracts unchanged. One documented defensive behavior change:
ineffective annotations with an empty reprojection ring and positive point
count return defaults rather than panicking (production callers reject empty
wall paths) — not equivalence for malformed input.

Evidence and its limits: interleaved tree-support process-CPU medians improved
(Benchy classic 171.6 → 165.4 s, Arachne 98.6 → 94.6 s; base classic 2846.8 →
2560.9 s, Arachne 1376.3 → 1342.5 s), but the quiet follow-up showed
**overlapping wall ranges and slightly higher wall medians — no reliable wall
improvement**. Acceptance was explicitly revised for this candidate only
(repeatable process-CPU savings justify the contained change); it is **not**
blanket authorization to relax acceptance later. Output: Arachne matched;
Classic has known same-binary nondeterminism so coarse G-code checks are smoke
evidence only — focused exact-output tests in
`inner_wall_material_boundary_tdd.rs` carry the invariants.

Follow-up audit caught a harness regression: the quiet scripts under
`docs/specs/perf-vs-orca/evidence/perf-flags/quiet-validation/` reused Classic configuration while labelling
with their generator loop variable, so files labelled Arachne were Classic
(`; wall_generator = Classic`). Those Arachne-specific conclusions are invalid
(raw captures preserved); earlier interleaved Arachne captures remain valid.
The harness now uses explicit per-generator configs with `-ExpectedGenerator`
validation, and `docs/specs/perf-vs-orca/evidence/alloc-bench/regression-check-generator-validation.ps1`
passes (11 cases including the exact failure mode). Every future generator run
must validate actual dispatch from stderr and G-code — labels prove nothing.
