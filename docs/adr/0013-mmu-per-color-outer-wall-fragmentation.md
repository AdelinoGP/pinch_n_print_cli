# ADR-0013 — MMU Multi-Color Perimeters Fragment Per-Color, Not Union-Traced

## Status

Accepted

## Context

Packet 96 closed `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` (AC-22b) by introducing `SlicedRegion.external_contour: Option<Vec<ExPolygon>>` — a host-side `union_ex` of all sibling painted regions of the same object, which perimeter modules then trace **once per painted object** rather than per cell. This satisfied the literal AC text ("outer wall count matches unpainted baseline") and was registered as deviation `D-96-AC22-EXTERNAL-CONTOUR`.

P96's mechanism was a pragmatic close-out under two constraints documented at the time:
1. **WASM guest boolean ops are no-ops** for the originally-drafted per-edge `bisector_edge_skip_mask`, so the guest could not apply the mask itself.
2. **Arachne medial-axis walls don't map 1:1 onto original cell-boundary edges**, breaking per-edge dedup at the wall level.

OrcaSlicer's MMU does **not** union-trace. It produces **per-color outer-wall fragments with tool changes at color transitions** (`T<N>` G-code between adjacent-color wall segments). The "fragmentation" P96's mechanism eliminated is the parity-correct behavior; the single-trace simplification is the divergence. Production impact: multi-color models currently print as if monochrome at the outer-wall layer, with tool color determined arbitrarily by `dominant_tool_index()`.

The constraints that justified P96's shortcut are not absolute:
- **Constraint 1 is overcome by computing the mask host-side**, exactly as `external_contour` is computed host-side. Guest only consumes a per-edge skip mask via an indexed read, not a boolean operation.
- **Constraint 2 is real for Arachne but applies at the WALL level**; the dedup can happen at the BOUNDARY level (preprocessing the per-color input contour Arachne receives), which is structurally different from the per-edge wall mask classic uses.

## Decision

**Multi-color models emit per-color outer-wall fragments with deterministic per-edge bisector ownership.**

- Each painted `SlicedRegion` traces its **own** outer perimeter, per-cell.
- At each shared edge between two adjacent same-object painted cells of different colors (a **bisector edge**), exactly one side owns the wall trace. The other side skips that edge segment.
- Ownership is determined by a **deterministic tie-break rule** (D-13 in the perimeter roadmap, to be closed by an OrcaSlicer-source investigation: T-P96-A0). Default if OrcaSlicer's rule is opaque or non-deterministic: **lower color-ID owns the bisector edge**.
- The mechanism carrying the skip information from host to guest is a **per-region edge mask** computed host-side (D-14 in the perimeter roadmap, defaulting to the resurrected `bisector_edge_skip_mask: Vec<bool>` shape with per-edge indexing aligned to `SlicedRegion.polygons`).
- **Arachne handles dedup at the boundary level** (preprocessing per-color input contour before SkeletalTrapezoidation), not at the wall level (D-15 in the perimeter roadmap). This is structurally different from classic and is addressed in M2 alongside the real Arachne port (T-P96-E).
- `SlicedRegion.external_contour` is deleted after the per-color reshape lands green (T-P96-D in the perimeter roadmap). Its plumbing through host, WIT, and per-layer arena (~5 files) is purely cleanup once unused.
- Tool-change G-code (`T<N>`) is emitted before each color-fragment outer wall via the existing `RegionKey.region_id → ToolChange` pipeline (packet 50b). No new emit code path; the existing path simply sees more boundaries.
- The `cube_4color_gcode_output_tdd.rs` AC-22b assertion is reshaped from "outer wall count ≈ unpainted baseline" to "per-color fragmentation with tool changes" (T-P96-A); the test is renamed to reflect the new shape.

`D-96-AC22-EXTERNAL-CONTOUR` is superseded by `D-<packet>-AC22-PARITY-RESHAPE`, registered at packet close (T-P96-F).

## Consequences

- **Multi-color models now match OrcaSlicer MMU output** at the outer-wall layer. Per-layer outer-wall extrusion sequences ≈ number of distinct colors present on that layer. Each fragment is preceded by `T<N>` matching its `ToolIndex`.
- **`SlicedRegion.external_contour` is removed.** Cascades through `slicer-ir`, WIT schema (`crates/slicer-schema/wit/deps/ir-types.wit`), `slicer-wasm-host` populator, `slicer-sdk` view accessor, and the two perimeter modules' consumption sites. ~5 files touched; one schema version bump on `SliceIR`.
- **Deterministic tie-break is baked into the host computation** of the per-edge mask. Both perimeter modules see the same masks for the same input; behavior is reproducible across runs.
- **Arachne's M2 design diverges from classic's M1 design** for this dedup. Classic uses a per-edge wall mask; Arachne preprocesses its input contour. Cited investigation (T-P96-E acceptance) anchors the Arachne approach to OrcaSlicer source.
- **One-time test churn**: `cube_4color_gcode_output_tdd.rs` test name, assertion shape, and expected SHA change. Re-baselined as `P<packet>_CUBE_4COLOR_PARITY_SHA` at close (T-P96-F).
- **No effect on single-color models** at any IR or G-code layer.

## Rejected alternatives

- **Keep `external_contour` (P96 mechanism).** Rejected: fails the OrcaSlicer parity goal stated for this roadmap. Documented as deliberate-but-known divergence in P96; this ADR retires that divergence.
- **Per-edge boolean mask applied in the guest.** Rejected in P96 because WASM-component boolean ops on `ExPolygon` lists are no-ops in the guest sandbox. Resurrected here in a different shape — the host computes the mask; the guest applies it per-edge via indexed read (no boolean op needed).
- **Recompute the mask in the guest from boundary intersection.** Rejected: duplicates host work, slower, and re-introduces the non-determinism the host-side computation removes.
- **Single fragmentation logic shared between classic and Arachne.** Rejected: Arachne's medial-axis walls don't map 1:1 onto original edges (P96 worker proof). The dedup must happen at boundary level for Arachne, wall level for classic. These are structurally different mechanisms; forcing them to share code would couple two algorithms that operate on different inputs.

## Future reviewers

- Do **not** re-suggest the union-trace simplification ("just trace the outer contour once"); it fails MMU parity and was deliberately retired. If a non-parity simplified mode is wanted for compatibility or testing, expose it as an opt-in config gate, not as the default.
- Do **not** widen the per-edge mask into the guest. The mask is host-computed by design (constraint 1 above). Future Arachne or classic implementations should consume the mask as input, not recompute it.
- The tie-break rule (D-13) is locked in this ADR via the investigation citation. If a future change wants to alter the tie-break, write a new ADR — silently flipping it would change the wall-owning-side per-layer and break parity test SHAs.
- The Arachne boundary-level dedup (D-15) is intentionally a different mechanism from classic's per-edge mask. They are not unified. Future reviewers proposing unification should consult T-P96-E's investigation citation before doing so.
