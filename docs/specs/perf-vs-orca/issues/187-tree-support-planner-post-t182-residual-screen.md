# `PrePass::SupportGeometry` remainder (`com.core.tree-support-planner`) attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Inside the surviving `PrePass::SupportGeometry` stage (`825 / 831 ms` = `29.5 / 27.5 %` of `benchy` supports-on, `6,512 / 6,111 ms` = `33.0 / 30.4 %` of `base` supports-on — the ONLY single stage able to close binding `benchy-classic-on` alone at `→ 0.849x`), where do the remaining `785 / 791 ms` (`benchy`) and `6,296 / 5,884 ms` (`base`) of `com.core.tree-support-planner` go across guest CPU (`5.85 / 48.56 B` fuel, now #2 in supports-on fuel), `host:native` `offset` (`1.43 / 17.49 s`), and remaining `host:native` `clip_polygons` (`1.72 / 14.82 s`), and which exact (zero-drift) mechanism(s) clear the standing `t137` 4-cell planner gate?**

## Scope

- Substage attribution inside `modules/core-modules/tree-support-planner/src/lib.rs` (drop/influence loop, occupancy offset batching, phase-1 tiled carve, phase-2, phase-3 residual after `t182`, phase-4 simplify/emit, WIT marshal/decode, and `5.85 / 48.56 B` guest fuel hotspots).
- Offline/probe replay of exact (0-drift vs `t182/reference.json`) candidate mechanisms across `benchy` and `base` supports-on (re-checking classic ≡ arachne invariance).
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Any graduated implementation candidate must be priced against `t137`'s standing 4-cell planner gate (`>= 0.80 %` paired wall saving with CPU corroborating in all 4 supports-on cells, structurally inert on supports-off).

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears `t137`).
