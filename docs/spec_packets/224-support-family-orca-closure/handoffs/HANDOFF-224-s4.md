# HANDOFF-224-s4 — Packet 224 remediation session 4 (2026-08-20) [condensed]

Session-4 record at HEAD `ed62090d`, packet status `draft`.

## Critical recovery state (was live during session 4, since resolved)

The Step 3b RC-15 port lived ONLY in `stash@{0}` ("224-s4-bisect-wip3": sampling + Q3 fix + narrowing
revert + DBG diagnostic) — the final stash pop never ran. Working tree held only temporary DBG
instrumentation (code-9999 blocks + eprintln taps in tree-support-planner lib.rs, to_buildplate_tdd.rs,
tree_family_tdd.rs) to be deleted before any measurement. Parked stashes w6/w6b ("unverified orphan
attempts") must never be dropped. Any full-crate run taken in the instrumented state ("only RC-C red")
is INVALID — it measured HEAD's old triangle-centroid sampling plus noise, not the port. The port's real
planner state was 9 failures across 5 binaries.

## Decisions resolved with the human (goal: canonical feature parity)

1. **Collision-gate narrowing REVERTED, bisect-confirmed.** HEAD closure 12/12 (251.6 s); HEAD +
   narrowing-only closure **9/12** with exactly the WIP's 3 failures (`fixture_invariants`,
   `support_never_intersects_model_at_exact_z`, `final_gcode_roles`); HEAD + RC-15-sampling-only
   closure 12/12 (250.1 s). Canonical `TreeSupport::drop_nodes` (`TreeSupport.cpp`) checks ALL nodes
   uniformly — no roof/interface role exemption — so the narrowing had no canonical counterpart.
2. **Eligibility-test inversion KEEP AND ROUTE.** `868508ba`'s
   `planned_region_renders_regardless_of_eligibility_flag` matches canonical (the toolpath generator
   prints what was planned; the planner owns eligibility). File the `needs_support` gap row (hardcoded
   `true` in `classify_object`, `crates/slicer-core/src/algos/mesh_analysis.rs`; and in
   `SliceRegionView`'s `Default`/`from_ir`, `crates/slicer-sdk/src/views.rs`); delete or retarget the
   now-vacuous `enforcer_overrides_needs_support_false`.

## Canonical verification of the port — `sample_contact_points` vs `TreeSupport::generate_contact_points` (`TreeSupport.cpp`)

Q1 corner stream (normalized dot > −0.7, contour only) MATCH; Q2 arc walk (start 0, closing edge,
contour+holes, step = branch_distance) MATCH; Q3 interior grid DIVERGENCE FIXED in stash — canonical
grids once over the whole-object bbox rotated 22° about the object center, port had gridded the
overhang bbox; fix adds `grid_bounds: Option<(f32,f32,f32,f32)>` param, `Some(...)` from both call
sites, overhang-bbox fallback; Q4 dedup (cell = base_radius, first-in-bucket) MATCH (`div_euclid` vs
C++ truncation fine); Q5 `base_radius = max(0.4, branch_diameter/2)` MATCH; Q6 default max_bridge_length
10 mm, `sample_step = max(point_spread, max_bridge_length/2)` MATCH (module-local
`DEFAULT_MAX_BRIDGE_LENGTH_MM = 10.0` stands in for an undeclared key — known rough edge); Q7 input =
per-layer overhang ExPolygons MATCH; Q8 collision checks all nodes, no exemption.

## Triage of the 9 planner failures (port + Q3 fix, narrowing reverted): fixture/config mismatches, no assertion weakened

RC-C pre-existing red. Fixes: shrink lone-contact fixtures to ~0.2×0.2 mm so samples collapse into one
dedup bucket (cell ≥ base_radius 0.4 mm) → 1 contact; spread cap_overflow tiles (≥2.4 mm pitch) so the
union stays 1100 disjoint polygons → >1024 contacts → code 1001 fires with dropped_count=76
(assertions unchanged); tile `multi_overhang_grid` at 4.0 so corner contacts land in separate buckets →
MST edges → propagation clamps → code 1002 fires; give `distributed_contacts` its own config
(`tree_support_branch_diameter = 1.0`) so contour-class arc points survive dedup (assertion unchanged);
`radius_aware_collision` body-radius failure undiagnosed — ask the human before weakening any floor.

## Remaining work

Finish Step 3b (closure 12/12 + planner only-RC-C-red; commit port checkpoint; one-time
deposited-material/XY-path re-measurement per `tree-density-diagnosis.md`, anchored to each G-code's own
filament footer; replace stale design.md baseline — never quote 486.33/1538.36/852.02/1158.87/31.6%);
Step 6 visual-debug inspection gate; Step 7 paperwork (gap row, register, stubs 224a/225/226/227, AC
amendments); Step 8 close (golden LAST renamed off `orca_parity` with provenance header, gates,
acceptance ceremony).
Measurement rules: `--no-fail-fast` on the planner crate (8 binaries); `-- <name> --exact` for bare
closure tests; `cargo xtask build-guests --check` before EVERY measurement; guest bakes working tree
(VERIFY pops actually happened — session 4 lost the port into a stash this way); delegate canonical
reads, G-code parsing, PNG inspection, and workspace test runs to subagents.
