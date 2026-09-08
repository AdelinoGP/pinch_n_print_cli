# Requirements: 250-visual-debug-silhouette-gcode-emit

## Packet Metadata

- Grouped task IDs: `TASK-452`, `TASK-453`, `TASK-454`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`PostPass::GCodeEmit` is the only D8-whitelisted silhouette tap still rejected after packets 247–249, and it is the only view of the typed G-code stream **before** `PostPass::GCodePostProcess` rewrites it — a defect visible here but absent from the final `.gcode` localizes to the postprocess modules. Unlike every other tap, `GCodeCommand::Move` carries no width and no layer index (plan fact 10), so widths must be recovered by inverting the emitter's own rectangular flow formula over consecutive accumulated `Move.e` positions (fact 9 corrected, D11) and Z must be bucketed into the finalized-layer schedule. Grounding surfaced a real fidelity gap this packet also owns: `run_postpass_taps` (`crates/pnp-cli/src/visual_debug.rs`) builds its `DefaultGCodeEmitter` without `with_resolved_config`, so captured streams are emitted with `ResolvedConfig::default()` (`filament_diameter` 1.75, default feedrates/simplification) regardless of the request's config — the inversion cannot be exact, and the captured IR misrepresents the real pipeline, until that is fixed.

## In Scope

- E-delta recovery over typed `GCodeIR.commands`: walk carrying the last seen `Some(e)` and the current position (`x`/`y`/`z` fields carried across `None`); `Δe` by differencing consecutive `Some(e)`; `e: None` moves are travel (no interval, carried E position NOT reset); `Δe < 0` (inline wipe-tower purge retracts — the emitter deliberately routes them through `Move.e`) and zero-length moves are non-extruding, skipped. Typed `Retract`/`Unretract` commands do not perturb the `Move.e` position stream (verified: the emitter's `e_position` accumulator excludes them) — the walk ignores them.
- Width per extruding move: `w = Δe × π(filament_diameter/2)² / (L₃D × h)` with `L₃D` the 3D move length (matching the emitter's distance), `h` the containing slab's height, and `filament_diameter` from the model source's resolved config (`ctx.default_resolved_config.filament_diameter`). The closed form is packet 248's — promoted into `slicer-runtime` so both crates share one formula (248's pnp-cli `silhouette_segment_width_mm` becomes a delegating wrapper; its pinned tests stay green).
- Z-containment bucketing: slab containing the move's current Z (`z_bottom < z ≤ z_top`); slabs are the finalized-layer schedule z-diffs (D8 slab-source note). Out-of-slab Z (nonplanar) draws at the nearest slab with the W4 warning naming the affected Z values; contained-but-unselected slabs draw nothing (D3 selection semantics, no warning).
- Schedule plumbing: `run_postpass_taps` additionally returns the sorted finalized `(global_layer_index, z)` schedule so the silhouette assembly can build `SilhouetteSlabSchedule` for a GCodeEmit-only bundle (and share one schedule source with packet 249's LayerFinalization groups).
- Emitter-config fidelity fix: `run_postpass_taps` configures its emitter `with_resolved_config((*ctx.default_resolved_config).clone())`, with the test fallout owned here (AC-9, AC-N4).
- Dedicated renderer entry `render_gcode_emit_silhouette` (roles or tools per 249's `RenderStyle`; tool tracked from `ToolChange`, tool 0 initial) reusing 247's union/paint-order/draw machinery — deviation from 249's `[FWD]` suggestion recorded in `design.md`.
- Validation lift: `SILHOUETTE_TAP_STAGE_IDS` += `"PostPass::GCodeEmit"`; retire 249's `gcode_emit_silhouette_still_rejected`; retarget 247's `silhouette_unsupported_taps_rejected_with_reasons` (drop its GCodeEmit arm).
- Filenames `PostPass__GCodeEmit_silhouette_{view}[_tool].png`; entry shape per 247 (`view`, `layers_rendered`, no `layer_index`/`layer_z`); grouping key (tap, view, color mode) per 249.
- Docs: `docs/19_visual_debug.md` GCodeEmit paragraph (AC-10).

## Out of Scope

- The standalone gcode source's silhouette (packet 248 — its parser-based inversion and `;Z:` slabs are untouched).
- `PostPass::LayerFinalization` rendering (packet 249 — its extraction arm and `postpass_stage_captures` shape are consumed, not modified beyond the shared schedule refactor).
- Seam overlays / `composited_overlays` (packet 251).
- Any change to `render_silhouette_composite` / `render_silhouette_composite_styled` signatures (their byte-equivalence pins stay intact).
- `gcode_line_width_mm` fallback semantics — that is a gcode-source (D14) concept; the model source always has a resolved `filament_diameter`, so GCodeEmit widths are always derivable and no fallback path exists here.
- Per-tool emitter configs (`with_tool_configs`) — `PrepassContext` exposes only the global resolved config; per-tool diameters affect estimator metadata only, not `Move.e` (verified), so they are out of scope.
- `GCodePostProcess`-stage taps, arena taps, plate frame, view mixing (existing rejections unchanged).

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ~811 lines; ranged reads only (facts 9/10/12, D8 GCodeEmit row, D10/D11/D16/D17, §6–§9).
- Packets 247/248/249 `packet.spec.md` + `design.md` — read-only export contracts.
- `docs/19_visual_debug.md` — range-read post-249.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-10`. AC-2/AC-3 pin the corrected fact-9 semantics (position differencing, travel carry, negative-delta skip); AC-4/AC-5 pin bucketing + W4 both directions; AC-9 pins the emitter-config fix.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: retires 249's `gcode_emit_silhouette_still_rejected` (AC-N2); retargets 247's `silhouette_unsupported_taps_rejected_with_reasons` (AC-N2); preserves 248's gcode-source tap rejection (AC-N3) and its `silhouette_segment_width_mm` pin via delegation; the emitter-config fix may shift any test pinning absolute captured-GCodeIR bytes — AC-N4 plus Step 3's regression sweep own that fallout (re-baseline to canonical-correct output if a self-captured baseline pinned the default-config stream).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | inversion, bucketing, W4, tool classes, determinism, fail-closed (AC-2..6, 8, N1) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | bundle shape, filenames, emitter-config fix (AC-1, 7, 9) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | tap lift + retirement/retarget (AC-N2, N3) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | top-down postpass byte-determinism after the emitter-config fix (AC-N4) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | packet 248's suite still green after the width-formula promotion | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate on new fixtures | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` | closure gates | FACT pass/fail |

## Step Completion Expectations

- Step 1's promoted width helper must land before Step 2's inversion consumes it; Step 3's emitter-config fix must land before Step 5's end-to-end ACs (AC-2/AC-9 are unfalsifiable against a default-config stream).
- The schedule returned by `run_postpass_taps` (Step 4) is the single slab source for **both** postpass tap groups after this packet; if packet 249 landed a capture-payload-derived schedule, Step 4 refactors it onto the plumbed source and re-runs 249's suite to prove byte-identical output.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` and `crates/slicer-runtime/src/visual_debug_render.rs` are ~2.2k lines each (pre-247) — ranged reads anchored on symbols only (`run_postpass_taps`, the silhouette branch, `gcode_shapes`).
- `crates/slicer-gcode/src/emit.rs` — read only the `emit` accumulation region (the `e_position`/`filament_area` block) to reconfirm fact 9; never load the whole file.
- The wedge e2e tests need fresh guest WASMs: `cargo xtask build-guests --check` before blaming failures.
