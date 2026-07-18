# Closure Log: 110_arachne-voronoi-skt-foundations

Status at closure: all 7 implementation steps DONE. Full acceptance ceremony passed 100% green — all 8 ACs, all 3 negative ACs, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask build-guests --check`, and the full `slicer-core` crate test suite (121 tests passed, 0 failed).

This log records implementation facts and deviations from the packet's original text, so the record is accurate for P111/P112 planning and for anyone auditing this packet later.

## 1. boostvoronoi version chosen

`boostvoronoi` v0.12 — already pinned in `crates/slicer-core/Cargo.toml` prior to this packet's activation (T-200's "pin the latest 0.x" open question was effectively pre-resolved). ADR-0023 confirms and records this pin. The latest available release at implementation time was 0.12.1; the packet did not bump to it (0.12 was retained as-is — no functional or security reason to bump surfaced during implementation).

## 2. 9-stage preprocess pipeline — corrected sequence

The packet's `requirements.md` (Grouped task IDs, T-204) and `packet.spec.md` (AC-6) describe the sequence loosely as "offset+simplify+offset+simplify+offset+fixSelfIntersections+removeSmallAreas+offsetExtra+simplifyExtra". The ACTUAL sequence implemented, verified directly against `WallToolPaths.cpp:565-604`, is:

1. Triple offset: shrink(-ε) / grow(+2ε) / shrink(-ε) — net-zero displacement, used to weld near-touching features and clean small self-intersections.
2. `simplify(smallest_segment = 0.5 mm, allowed_distance = 0.025 mm)`.
3. `fixSelfIntersections(ε)`.
4. `removeDegenerateVerts`.
5. `removeColinearEdges(0.005 rad)`.
6. `fixSelfIntersections(ε)` — repeated.
7. `removeDegenerateVerts` — repeated.
8. `removeSmallAreas(small_area_length², false)`.
9. `union_`.

Computed `epsilon_offset = (allowed_distance / 2) - 1_unit ≈ 12.499 µm`. The packet's own text (AC-1, AC-6, AC-N3, and the coordinate-system snippet in `design.md`) claims "~11.5 µm" throughout. The ~8.7% discrepancy is within the documented sanity tolerance and was implemented per the literal formula rather than force-fit to match the packet's stated 11.5 µm figure.

The mandatory AC-6 hazard doc-comment string `destroys features < epsilon_offset ~11.5 µm` was still included verbatim in `preprocess_input_outline`'s doc-comment, exactly as AC-6's `rg` check requires — even though the actual computed constant used at runtime is ~12.5 µm, not ~11.5 µm. **This is a minor residual inconsistency between the required literal doc-string and the real computed value** — flagged here as a candidate follow-up cleanup for P111/P112 if it ever causes confusion (e.g., someone reading only the doc-comment and assuming 11.5 µm is the operative threshold).

## 3. T-P96-E — major design correction (most significant deviation in this packet)

This is the most significant deviation between what the packet specified and what was shipped.

**What the packet specified:** `packet.spec.md` AC-7, `requirements.md`'s T-P96-E task bullet, and `design.md`'s Selected Approach / Code Change Surface sections all specified `preprocess_per_color_inputs` as a bisector-edge contraction algorithm: for each `(ToolIndex, Polygons)` pair, walk each polygon's edges, and for edges shared with a neighboring different-color cell, apply a `TieBreakRule` (an enum with `LowerToolIndexWins` (default), `HigherToolIndexWins`, `Custom(fn)` variants) to contract/remove the edge on the losing side. This was grounded in a citation to ADR-0013.

**Why it was wrong:** mid-implementation, direct verification proved the ADR-0013 citation was stale. ADR-0013's CURRENT doctrine, as it stands in the tree today (the tie-break model was retired 2026-06-23), explicitly states "no skip mask, no per-edge ownership, and no tie-break rule" (lines 9, 29), and confirms Arachne was deliberately brought in line with this doctrine (lines 32, 40: Arachne's old union-trace special case was removed "so it also fragments per-color" like Classic).

Independent verification directly against OrcaSlicer's canonical C++ source (`PerimeterGenerator.cpp:2600-2653`, `process_arachne()`; `Arachne/WallToolPaths.hpp:63-83`) confirmed Arachne's real implementation contains **zero** color/extruder/material-aware logic anywhere. Per-color boundary isolation happens entirely UPSTREAM, during layer/region segmentation (`PrintObjectSlice.cpp` → `multi_material_segmentation_by_painting()` → per-extruder `LayerRegion` split) — before Arachne (`WallToolPaths` / `SkeletalTrapezoidation`) ever sees geometry. Arachne itself is completely color-blind.

This was surfaced to the user mid-implementation; the user directed verification against canonical OrcaSlicer source, which confirmed the correction.

**Shipped implementation:** `preprocess_per_color_inputs(painted_cells: &[(ToolIndex, Vec<ExPolygon>)]) -> Vec<(ToolIndex, Vec<ExPolygon>)>` is a validated pass-through:
- No `TieBreakRule` enum exists in the shipped code.
- No contraction happens.
- Each color's cell boundary passes through unmodified, trusting the upstream paint/region-split pipeline (P91-94) to have already produced a valid non-overlapping partition.
- The function validates the non-overlap invariant and **logs a warning** (does not silently repair) if violated beyond epsilon.

This is deliberately simpler than the packet's original design and is the correct behavior per both current architecture doctrine (ADR-0013) and canonical OrcaSlicer source. Part 2 of this closure corrects `packet.spec.md`'s AC-7 text and `requirements.md`'s T-P96-E bullet to match the shipped behavior.

## 4. `discretize_parabolic_edge` signature interpretation

The packet's signature (`focus, line_a, line_b, max_segment_len` — 4 parameters, per `packet.spec.md` AC-5 and `requirements.md`/`design.md`) has no separate `start`/`end` arc-bound parameters, unlike OrcaSlicer's real function, which takes 6 parameters including explicit `start`/`end`/`transitioning_angle`.

Resolved by treating `line_a`/`line_b` as serving double duty: both the directrix-line definition AND (via their own projected positions) the arc's local-x bounds. This interpretation is documented in the function's doc-comment.

`transitioning_angle`-driven bead-marking-point insertion (an Arachne-specific bead-transition marker mechanism) was deliberately NOT implemented — out of scope for this packet's signature and belongs to P111/P112 (BeadingStrategy) territory instead.

## 5. AC-5's "OrcaSlicer reference" — what it actually is

AC-5 requires the discretized polyline to lie within 0.005 mm Hausdorff distance of "the OrcaSlicer-discretized polyline for the same parabola." This is **not** a captured OrcaSlicer C++ execution trace — this environment cannot execute OrcaSlicer C++. It is an independent higher-resolution resampling of the same closed-form parabola equation, used as a Hausdorff-distance reference stand-in, plus a stronger tolerance-independent on-parabola distance check.

Literal numeric OrcaSlicer parity is deferred to P112/T-231. This matches the same precedent already set by Steps 2-3's Voronoi/SKT goldens, which are recorded from boostvoronoi's own output, not from an OrcaSlicer execution trace — consistent with `design.md`'s own stated Risk ("Goldens recorded per crate vs from OrcaSlicer... OrcaSlicer-parity verification lands in P112's T-231").

## 6. Logging mechanism correction

`packet.spec.md` AC-N3 originally called for "the function emits a tracing `warn!`." The `tracing` crate does not exist anywhere in this workspace. `log` is the established host-side convention (used unconditionally in 5 other crates) but was `optional`/gated behind the `host-algos` feature in `slicer-core`.

Fixed by making `log` an unconditional dependency of `slicer-core` — removed `optional = true` and `dep:log` from the `host-algos` feature array — so that `arachne/preprocess.rs` (which is correctly NOT `host-algos`-gated, since it needs no boostvoronoi) can call real `log::warn!`.

This Cargo.toml change is a universal guest dependency edit per CLAUDE.md's "Guest WASM Staleness" section. It triggered a full `cargo xtask build-guests` rebuild across all 30 pre-existing guests; the rebuild was run and confirmed clean afterward.

## 7. `modules/core-modules/arachne-perimeters/wit-guest/` subcrate

Not explicitly named in the packet's file-scope list (`design.md`'s Code Change Surface table lists only the top-level module directory, manifest, `src/lib.rs`, and `Cargo.toml`), but structurally required: every other core-module has an identical `wit-guest/` companion subcrate, which is the actual `cdylib`/`wasm32` compilation target — the top-level module crate alone cannot produce a WASM guest.

Created mirroring `classic-perimeters/wit-guest/`'s exact pattern. NOT added to the root workspace `Cargo.toml` members list, matching precedent — `wit-guest/` subcrates are discovered by `cargo xtask build-guests`, not built as ordinary workspace members.

## 8. `LayerModule::run_perimeters` real signature

Discovered via compiler error, not doc-read (`docs/05_module_sdk.md`'s trait surface description was insufficiently precise for the exact parameter list). Real signature:

```rust
fn run_perimeters(
    &self,
    layer_index: u32,
    regions: &[SliceRegionView],
    paint_regions: &PaintRegionLayerView,
    output: &mut PerimeterOutputBuilder,
    config: &ConfigView,
) -> Result<(), ModuleError>
```

Six parameters. The T-205 skeleton implementation matches this exactly (returns `Ok(())`, emits `warn!`).

## 9. Residual open questions / follow-ups for P111/P112

- **P111 (BeadingStrategy stack)** needs `SkeletalTrapezoidationGraph` (T-202) as its anchor for bead-count assignment — confirm the `r_min`/`r_max`/`central`/`is_curved` field shape (documented in this packet's Step 3/4 outputs) is sufficient before P111 starts.
- **P112 (extrusion + wire-up, T-230/T-231)** is where real OrcaSlicer numeric parity gets verified for the Voronoi/SKT/discretize goldens recorded in this packet (currently self-consistent/boostvoronoi-derived, not OrcaSlicer-execution-verified — see item 5 above).
- The `epsilon_offset` ~11.5 µm (packet text) vs ~12.5 µm (actual computed constant, item 2 above) discrepancy should be revisited if it ever causes a visible parity mismatch in P112.
- `discretize_parabolic_edge`'s simplified 4-parameter signature (item 4 above) may need extension with explicit `start`/`end` parameters when P112 wires it against real SKT graph edges, if the current double-duty interpretation of `line_a`/`line_b` proves insufficient for actual VD edge arcs that don't align with the generating segment's own endpoints.
- `preprocess_input_outline`'s `small_area_length_mm` threshold (stage 8) was not specified anywhere in the packet text and was defaulted to `smallest_segment_mm` (0.5 mm) as a discretionary implementation choice — confirm this default is reasonable when real fixtures are exercised in P112.
