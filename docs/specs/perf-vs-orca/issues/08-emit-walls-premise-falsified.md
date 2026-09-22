# emit_walls premise falsified and classic re-attribution

Type: task
Status: resolved

## Question

Does classic `ClassicPerimeters::emit_walls`
(`modules/core-modules/classic-perimeters/src/lib.rs`) inset clone/store cost
enough to justify retaining only the first inset (the then-ranked lead 1) — and
after falsification, where does `com.core.classic-perimeters`' cost actually
sit?

## Answer

Premise **falsified** 2026-09-22 (`docs/specs/perf-vs-orca/evidence/perf-emit-walls/FINDINGS.md`; grilling
decisions Q1–Q15 in the session record). Supports-off benchy, classic config,
guest fuel (temporary ADR-0055 user scopes, removed after):

- `emit_walls`' inset clone/store: **0.0014%** of module fuel — lead closed
  for time (memory-shape idea only; peak RSS unmeasurable, DEV-026). The share
  is mode-independent, so the falsification stands in any build mode.
- `expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`)
  per-vertex `overhang_quartile` / `signed_distance_to_boundary` queries:
  **69.8%** of total guest fuel (2,107 rings) — legacy linear scans in
  ordinary builds; the spatial acceleration is injected only by the controlled
  rustc driver (packet 254, `docs/23_controlled_perimeter_builds.md`).
- `PerimeterSpatialContext::is_bridge` per-point scan: **4.2%**.
- `polygon_ops::offset2_ex` (`crates/slicer-core/src/polygon_ops.rs`): **15.2%**
  — 960 calls: 720 from the `only_one_wall_top` `min_width_top` shrink/expand
  after `split_top_surfaces`, 240 from the gap-fill width band.
- `build_wall_flags`: negligible — the ticket-07 fast path works on this
  workload.

Re-ranked leads (nothing authorized or implemented): (1) accelerated
perimeter-spatial adoption (the measured 74% path3d+bridge scans), (2)
`offset2_ex` call-count reduction in the shrink/expand, (3) consume-only-when-
read per-vertex annotations, (4) verify `fold_marks`
(`crates/slicer-wasm-host/src/profiling.rs`) thread handling before acting on
`host:slice` `closing_ex` span-vs-wall contradiction. Ordinary-mode shares are
only ordinary-valid — see [Accelerated-mode pair](09-accelerated-mode-pair.md).
