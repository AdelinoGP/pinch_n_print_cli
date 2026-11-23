# `PrePass::SupportAnalysis` attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Why does `PrePass::SupportAnalysis` (`292 / 290 ms` = `10.4 / 9.6 %` of `benchy` supports-on, `2,853 / 2,892 ms` = `14.5 / 14.4 %` of `base` supports-on, `12.0–18.1×` floor) run at only `2.95–3.00×` concurrency on `benchy` (vs `6.47–7.44×` on `base`), how much is serial `collect_print_plane_occupancy` (`~85 ms` `benchy` / `~745 ms` `base` single-threaded `slice_mesh_ex` spotted in `t181`) vs `host:support_analysis` `offset` (`84 %` of built-in thread-time: `0.61 s` `benchy`, `15.54–18.11 s` `base`), and what exact (zero-drift vs `t182/reference.json`) bundle clears the standing `t137` 4-cell planner gate?**

## Scope

- Substage attribution inside `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (`model_occupancy` assembly, `collect_print_plane_occupancy`, overhang/contact detection `offset` + `clip_polygons`, termination surfaces, modifier-region handling).
- Screen exact mechanisms: parallelizing `collect_print_plane_occupancy` (or sharing/reusing mesh slicing), improving layer/region parallelism in `detect_overhangs`, and eliminating redundant/unnecessary `offset` calls (`401` `benchy` / `1,231` `base`) with `100 %` bit-identical IR vs `t182/reference.json`.
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Supports-off is structurally inert (`0 ms` when `enable_support: false`), so any graduated implementation candidate qualifies for `t137`'s standing 4-cell planner gate (`>= 0.80 %` paired wall saving with CPU corroborating on all 4 supports-on cells).

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears `t137`).
