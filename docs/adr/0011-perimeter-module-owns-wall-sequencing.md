# ADR-0011 — The Perimeter Module Owns Wall Sequencing

## Status

Accepted

## Context

`wall_sequence` controls the order in which perimeter loops are printed within one region. OrcaSlicer supports three modes — `OuterInner`, `InnerOuter`, and `InnerOuterInner` (sandwich). `OuterInner` reverses the loop list; `InnerOuter` is the canonical order; `InnerOuterInner` interleaves per outer contour ("first inner → outer → remaining inner"), which requires knowing the parent/child relationship of each loop within the wall tree.

In OrcaSlicer the reordering happens inside `process_classic()` (`OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` lines 1801–1913), operating on the perimeter tree the generator just built. In this codebase, `wall_sequence` was preregistered as a `path-optimization-default` config key in `docs/15_config_keys_reference.md` before the perimeter modules had a parity roadmap — a vestige, not a designed decision.

The perimeter-modules parity roadmap (`docs/specs/perimeter-modules-orca-parity-roadmap.md`) surfaced the question explicitly as decision D-1: should reordering live in the perimeter module (matching Orca), in `path-optimization-default` (matching the current config-key location), or split across both (perimeter emits a flat list with `parent_loop_index` so path-optimization can group)?

The grilling weighed:
- Whether `wall_sequence` is a geometry-coupled concern (sandwich mode interacts with sharp corners, `overhang_reverse`, and seam scoring) — **yes**, all geometry-time.
- Whether path-optimization-default has access to the data sandwich mode needs (per-region wall tree, sibling grouping) — **no**, it sees a flat `Vec<LayerCollectionIR>` of entities.
- Whether keeping the algorithm where Orca has it eases the port — **yes**, in this specific case it aligns clean.
- The cost of moving the config-key registration — **paperwork, not architecture**.

## Decision

**The perimeter module owns wall-sequence reordering.**

- `PerimeterRegion.walls: Vec<WallLoop>` is committed in **final print order**. This includes the Arachne perimeter module after its finalized-line region ordering. Downstream consumers (seam-placer, path-optimization-default, GCodeEmit) may treat the sequence as authoritative; path optimization may optimize permitted travel but preserves the committed wall subsequence and does not reorder walls within a region.
- The `wall_sequence` config key is owned by `classic-perimeters` and (pre-P108) `arachne-perimeters`. The config-key registration migrates from `path-optimization-default` accordingly. **Amendment (2026-06-19, D-110-DROP-VARIABLE-WIDTH):** `variable-width-perimeters` was never shipped; the iterative-inset module is deleted under P108 and real Arachne is introduced fresh under P110+P112. **Amendment (P108):** the fake `arachne-perimeters` module (iterative-inset approximation) was deleted; `classic-perimeters` is the sole perimeter generator until real Arachne lands under P110+P112 and re-registers `wall_sequence` under the same module id.
- Reordering logic — including `InnerOuterInner` sandwich grouping — lives in the shared `slicer-perimeter-utils` (the new shared crate from roadmap T-010), so every perimeter module calls one implementation.
- The wall tree (hole/contour nesting) is in-module scaffolding only. It is built during generation, used for sandwich-mode grouping, and discarded before commit. The tree never crosses the module boundary; no `parent_loop_index` field is added to `WallLoop`.
- `path-optimization-default` deregisters `wall_sequence` from its config schema. Its scope remains travel/retract/Z-hop and nearest-neighbour ordering of entities; intra-region wall sequencing is out of its mandate.

## Consequences

- **IR stays flat.** `WallLoop` carries no parent/child field. Decision D-6 in the perimeter-modules roadmap closes as "flat list, final-print-order". Downstream IR readers see a simpler contract.
- **Cross-module placement aligns with the algorithm's data needs.** Sandwich mode needs the wall tree; the perimeter module has it; no view extensions or IR additions are required.
- **Future advanced wall-sequencing modes** (overhang-coupled sequencing, non-planar walls, vertical-shell-thickness-driven sequencing) inherit the geometry context they need without further refactor.
- **Real Arachne (M2) integrates naturally.** `Arachne::WallToolPaths::generate()` already groups walls by `inset_idx` internally. When M2 lands, Arachne's sequencing fits the same boundary; no architecture fight.
- **One-time migration cost:** the roadmap gains tasks to (a) deregister `wall_sequence` from `path-optimization-default`'s config schema, (b) register it in the perimeter manifests, (c) implement the three modes in `slicer-perimeter-utils`. Roadmap task T-076 is split accordingly.
- **General principle recorded for future decisions:** match OrcaSlicer pipeline placement when the modular split allows it. When the modular split forces a deviation, document it explicitly. Wall sequencing is one of the cases where it does align — but it would not be the rule in every case (e.g. seam placement, fuzzy skin, and overhang speed quartiles deliberately split out of the perimeter module here, where Orca consolidates them in `process_classic()`).

## Rejected alternatives

- **`path-optimization-default` owns reordering.** Rejected: path-optimization sees a flat entity list, has no per-region wall tree, and would need either an IR-level `parent_loop_index` field or a guarantee that perimeter modules commit "one PerimeterRegion per outer contour" — neither is free, and both leak geometry concerns into the optimization stage.
- **Split — perimeter emits flat by `perimeter_index`, path-optimization handles OuterInner-only, sandwich requires IR additions.** Rejected: introduces a per-mode boundary that's hard to reason about, doubles the surface where a wall_sequence bug could live, and still requires the IR addition.

## Future reviewers

- Do **not** re-suggest moving `wall_sequence` back to `path-optimization-default` for "config key locality" — locality must follow the algorithm, not the other way around.
- Do **not** add `parent_loop_index` to `WallLoop` without a separate ADR — the wall tree is deliberately in-module scaffolding.
- If a new wall-sequencing mode arrives that genuinely requires cross-module coordination, write a new ADR rather than silently extending `WallLoop`.
