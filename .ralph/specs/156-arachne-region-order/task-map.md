# Task Map: 156-arachne-region-order

## Backlog mapping

This packet's `task_ids` is `none` (declared in `packet.spec.md`
frontmatter). The backlog source is the round-3 Arachne parity audit
appended to `docs/18_arachne_parity_audit.md` (commit `54536d57`).
The packet covers:

- **G12** — `WallToolPaths::getRegionOrder` (Algorithm) —
  `docs/18_arachne_parity_audit.md` lines 296-323 + 415-421

The gap is also tracked (under the gap ID only — no
`docs/07` task ID was ever created) in the M2 "Real Arachne"
section of `docs/07_implementation_status.md:312-330` as part of
the cross-cutting N1–N13 closure chain (P141–P147 + P152 + P153).

## Step ↔ gap mapping

| Step | Gap(s) closed | Verification command |
| --- | --- | --- |
| Step 1 (SparsePointGrid utility) | G12 (foundation utility for getRegionOrder) | `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_get_nearby_returns_only_nearby_points --exact sparse_point_grid_single_insert_get_nearby_self --exact` |
| Step 2 (getRegionOrder port) | G12 (constraint-set builder) | `cargo test -p slicer-core --test region_order_tdd -- region_order_get_emits_adjacent_constraints --exact region_order_empty_input_returns_empty --exact region_order_single_line_preserved --exact region_order_no_adjacency_falls_back_to_nearest_neighbor --exact region_order_zero_max_line_width_returns_no_constraints --exact` |
| Step 3 (topological_walk + wrapper) | G12 (greedy emission order) | `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_respects_constraints --exact` |
| Step 4a (pipeline integration) | G12 (new `ArachneParams` field + reorder call) | `cargo check --workspace --all-targets` |
| Step 4b (module `outer_to_inner` derivation) | G12 (wall_sequence plumbed with correct polarity; AC-7/AC-8 locks) | `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing` + `cargo test -p slicer-core --test wall_sequence_reorder_tdd` + `cargo test -p slicer-runtime --test arachne_parity` |
| Step 5 (doc + final gates) | G12 (close documentation) | doc greps + `cargo xtask build-guests --check` + `cargo clippy --workspace --all-targets -- -D warnings` |

## Cross-packet relationships

- **Depends on:** `155-arachne-beading-simplify-parity` (packet A)
  should be `status: implemented` before this packet is
  activated (per the packet prerequisites in
  `packet.spec.md`). The two packets are technically
  independent (no shared symbol), but landing them together
  in the same sprint risks overlapping work in
  `crates/slicer-core`.
- **Unblocks:** none (no open packet reads the
  `reorder_by_region_order` helper or the `outer_to_inner`
  field this packet introduces).
- **Adjacent packets (not in this packet's scope):** packets
  148/149/150/151/152/153/154 (the earlier M2 + perimeter-parity
  closure chain). They are all `status: implemented` per
  `docs/07_implementation_status.md:312-330` and their changes
  are already in the tree at `parity/arachne @ 34ce576e`.
- **Audit doc updates (this packet's responsibility):**
  - Mark G12 closed in the Gap summary table (line 291).
  - Update the G12 detailed-gap "PnP status" entry
    (line 297) to "closed (this packet)".
  - Drop the G12 row from the "Open gaps" list (line 48-187).

## Wall sequencing ownership (ADR-0011 cross-reference)

This packet's `reorder_by_region_order` pass operates at the
**slicer-core** level, on the `Vec<ExtrusionLine>` returned by
`run_arachne_pipeline` — which the `arachne-perimeters` module itself calls.
ADR-0011 locks that "`PerimeterRegion.walls: Vec<WallLoop>` is committed in
**final print order**" and that the perimeter module owns wall-sequence
reordering. The G12 pass is a *pre-`WallLoop`* reorder performed inside the
module's own call chain, so the module still owns the committed order — no
ADR-0011 amendment is required. State this conformance argument in D-157.

**Two pre-existing reorders stay in place and must not be duplicated or
cancelled by the new pass (AC-8):**

- `slicer_core::perimeter_utils::wall_sequence_reorder`
  (`crates/slicer-core/src/perimeter_utils.rs:723`, tested by
  `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs`) — the **post-module**
  `Vec<WallLoop>` wall-sequence reorder. The earlier draft of this packet did
  not mention it at all. If the module's `WallLoop` order derives from the
  pipeline's now-reordered `ExtrusionLine` order, a second direction flip here
  would cancel the first — the implementer must verify, not assume.
- the module's per-region `sort_by_key(perimeter_index)`
  (`arachne-perimeters/src/lib.rs:614`) — a stable secondary sort on
  `Vec<WallLoop>`.

## OrcaSlicer parity surface

Line numbers re-resolved against the real `OrcaSlicerDocumented/` tree on
2026-07-14. **The earlier draft's refs were all wrong** — it cited
`WallToolPaths.cpp:809-893`, `WallToolPaths.hpp:104`, and
`PerimeterGenerator.cpp:2270-2360`, and additionally asserted a
`radius / sqrt(2)` grid cell size and a `Point::Zero()` initial cursor, neither
of which exists in OrcaSlicer.

- `WallToolPaths.hpp:211` — `getRegionOrder` declaration.
- `WallToolPaths.cpp:973-1058` — `getRegionOrder` impl; cell size = `searching_radius` (`:1022`); direction-independent `is_odd` constraint branch (`:1044-1054`).
- `SparsePointGrid.hpp:31-38` + `SparseGrid.hpp:106,137-146` — grid ctor (cell size stored verbatim) and `getNearby`.
- `PerimeterGenerator.cpp:2781-2857` — the topological walk; cursor init at `:2798-2799`; open-before-closed candidate sort at `:2815-2818`.
- `PerimeterGenerator.cpp:2761-2766` + `PrintConfig.hpp:187-192` + `PrintConfig.cpp:2084` — `outer_to_inner` polarity (default **false**).

All OrcaSlicer reads MUST be delegated to a sub-agent per the
`orca-delegation` snippet.
