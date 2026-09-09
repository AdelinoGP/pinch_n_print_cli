---
status: implemented
packet: 250-visual-debug-silhouette-gcode-emit
task_ids:
  - TASK-452
  - TASK-453
  - TASK-454
---

# 250-visual-debug-silhouette-gcode-emit

## Goal

Extend the `silhouette` visualization kind (packets 247/249, schema 1.2.0) to `PostPass::GCodeEmit`: recover per-move extrusion widths from the typed `GCodeIR` command stream by consecutive-`Some(e)` position differencing (plan fact 9 corrected + D11 — `Move.e` is the accumulated E position; `Δe < 0` inline purge retracts are non-extruding and skipped; `e: None` travels carry no interval and never reset the carried position), bucket each extruding move by Z-containment into finalized-layer schedule slabs with the W4 nearest-slab out-of-slab warning, render roles or tools through a dedicated GCodeEmit composite entry point on the D10 single whole-print capture, fix `run_postpass_taps` to configure its `DefaultGCodeEmitter` with the model source's resolved config (grounding-surfaced: today it emits with `ResolvedConfig::default()`, so the captured stream's E values ignore the request's `filament_diameter`), and lift packet 249's interim `SilhouetteUnsupportedForTap` rejection for this tap.

## Problem Statement

`PostPass::GCodeEmit` is the only D8-whitelisted silhouette tap still rejected after packets 247–249, and it is the only view of the typed G-code stream **before** `PostPass::GCodePostProcess` rewrites it — a defect visible here but absent from the final `.gcode` localizes to the postprocess modules. Unlike every other tap, `GCodeCommand::Move` carries no width and no layer index (plan fact 10), so widths must be recovered by inverting the emitter's own rectangular flow formula over consecutive accumulated `Move.e` positions (fact 9 corrected, D11) and Z must be bucketed into the finalized-layer schedule. Grounding surfaced a real fidelity gap this packet also owns: `run_postpass_taps` (`crates/pnp-cli/src/visual_debug.rs`) builds its `DefaultGCodeEmitter` without `with_resolved_config`, so captured streams are emitted with `ResolvedConfig::default()` (`filament_diameter` 1.75, default feedrates/simplification) regardless of the request's config — the inversion cannot be exact, and the captured IR misrepresents the real pipeline, until that is fixed.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `GCodeCommand::Move`'s `x`/`y`/`z`/`e` are millimeter `f32` `Option`s end-to-end; no `Point2`/`mm_to_units` appears in the new code.
- Projector single-owner rule (archived spec, binding): rectangle corners go through `Projector::project(x_or_y_mm, z_mm)`; no new transform.
- Fact-9 lock (corrected 2026-08-27): `Move.e` is the **accumulated** E position — `DefaultGCodeEmitter` (`crates/slicer-gcode/src/emit.rs`) does `e_position += e_delta` and emits `Some(e_position)` iff `e_delta != 0.0`; negative deltas (wipe-tower `generate_purge_paths` inline retracts) flow through `Move.e` by deliberate emitter design; typed `Retract`/`Unretract` never touch `e_position`. Any inversion that reads `Move.e` as a per-move delta is wrong and must not survive review.
- One rectangular flow formula in the workspace (248's `[FWD]`): the closed form moves to `slicer-runtime`; `crates/pnp-cli/src/visual_debug_gcode.rs::silhouette_segment_width_mm` keeps its pub signature and delegates. Never fork a second union or a second formula.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): fixture literals of watched types (`GCodeIR` has `Default`; `LayerCollectionIR` uses `..Default::default()`; `PrintEntity` has no `Default` — existing suites' `// exhaustive:` waiver) follow the established patterns.

## Data and Contract Notes

- IR/manifest contracts: no IR/WIT/schema change — `GCodeIR` is consumed as-is; manifest deltas are 247/249's existing fields on new entries. 1.0/1.1 bundles byte-frozen (unchanged code paths). The emitter-config fix changes **captured** postpass `typed_capture` content (E/F values) for model-source requests whose config differs from defaults — a fidelity correction, not a schema change; AC-N4 + the fallout sweep own it.
- WIT boundary: none.
- Determinism/scheduler constraints: the command stream is a `Vec` walked once in order; segments inherit stream order; rectangle emission ascending slab index → class order (249's role ranks / ascending tool) → interval start; W4 warnings sorted by Z ascending, deduped, capped at 8 + `+N more`; group order per 247/249. No `HashMap` iteration anywhere on this path.
- Exactness bound: differencing f32 accumulated positions recovers `Δe` to ~ulp of the running `e_position`; AC-2's `1e-3` mm width tolerance covers it. The recovered width is the **deposited** width (`× flow_factor`) — 248's documented caveat, extended to GCodeEmit in docs.

## Locked Assumptions and Invariants

- `Move.e` is the accumulated position; `Some` iff the move's delta ≠ 0; negative deltas flow through it; typed `Retract`/`Unretract` never appear in the position stream. The inversion differences consecutive `Some` values, carries across `None`, updates the carry on every `Some` (negative included), draws only `Δe > 0`.
- Slabs are the finalized-layer schedule z-diffs (first from 0), the same source as packet 249's LayerFinalization slabs — one schedule authority per bundle.
- Out-of-slab material is never silently dropped: containment first, nearest slab + W4 second; contained-but-unselected draws nothing and warns nothing (selection, not loss).
- `render_silhouette_composite`/`render_silhouette_composite_styled` signatures and byte behavior are frozen; GCodeEmit renders only through the dedicated entry.
- The gcode-source silhouette (`visual_debug_gcode.rs`) and its D14 fallback are untouched; the model-source GCodeEmit path has no width fallback (the resolved config always supplies the diameter).
- Filename scheme `PostPass__GCodeEmit_silhouette_{view}[_tool].png`; one silhouette plane per bundle (247 invariant, inherited).

## Risks and Tradeoffs

- The emitter-config fix alters captured postpass streams for non-default configs (E values, feedrates, `min_segment_length` simplification). Any self-captured baseline pinning the old default-config stream must be re-baselined to canonical-correct output (Test Discipline rule); the Step-3 LOCATIONS sweep plus AC-N4 bound the blast radius. The real `run_slice` pipeline is unaffected (it always configured its emitter).
- This is a second E-inversion implementation (vs packet 248's parser-based one), testable mainly against itself — accepted by the user in D11; the AC-2 emitter round-trip is the strongest available external anchor, and the shared width formula removes one axis of divergence.
- Whole-print execution cost for any postpass tap is inherent (plan facts 8/15) — D10 removed the clone multiplication (249), not the execution.
- Nearest-slab placement on genuinely nonplanar prints draws approximated Z; W4's per-Z text keeps the image trustworthy (plan §10 item 4).
