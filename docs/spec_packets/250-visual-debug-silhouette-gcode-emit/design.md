# Design: 250-visual-debug-silhouette-gcode-emit

## Controlling Code Paths

- Primary code path: `validate_request` → `run_model_source` → `run_postpass_taps` (`crates/pnp-cli/src/visual_debug.rs`; tiers 2–4 via `execute_postpass_with_capture` with the `PostPassCapture` sink, captures via packet 249's `postpass_stage_captures` `WholePrint` `GCodeEmit` arm) → packet 247's silhouette assembly branch → **new** `render_gcode_emit_silhouette` (`crates/slicer-runtime/src/visual_debug_render.rs`) → `Projector` + `Canvas`.
- Neighboring tests/fixtures: packet 249's `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs` (whole-print capture fixtures, wedge postpass harness), packet 247's `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (decoded-pixel composite assertions — this packet's renderer tests live there), `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` (top-down `PostPass::GCodeEmit` two-run byte harness), `crates/slicer-gcode/tests/gcode_emit_per_role_tolerance_tdd.rs` (`with_resolved_config` fixture pattern for the AC-2 round-trip).
- OrcaSlicer comparison: none — PnP-native tool. The only Orca-facing fact is D16's rectangular-vs-stadium docs caveat, already written by packet 248; this packet appends a GCodeEmit mention.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `GCodeCommand::Move`'s `x`/`y`/`z`/`e` are millimeter `f32` `Option`s end-to-end; no `Point2`/`mm_to_units` appears in the new code.
- Projector single-owner rule (archived spec, binding): rectangle corners go through `Projector::project(x_or_y_mm, z_mm)`; no new transform.
- Fact-9 lock (corrected 2026-08-27): `Move.e` is the **accumulated** E position — `DefaultGCodeEmitter` (`crates/slicer-gcode/src/emit.rs`) does `e_position += e_delta` and emits `Some(e_position)` iff `e_delta != 0.0`; negative deltas (wipe-tower `generate_purge_paths` inline retracts) flow through `Move.e` by deliberate emitter design; typed `Retract`/`Unretract` never touch `e_position`. Any inversion that reads `Move.e` as a per-move delta is wrong and must not survive review.
- One rectangular flow formula in the workspace (248's `[FWD]`): the closed form moves to `slicer-runtime`; `crates/pnp-cli/src/visual_debug_gcode.rs::silhouette_segment_width_mm` keeps its pub signature and delegates. Never fork a second union or a second formula.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): fixture literals of watched types (`GCodeIR` has `Default`; `LayerCollectionIR` uses `..Default::default()`; `PrintEntity` has no `Default` — existing suites' `// exhaustive:` waiver) follow the established patterns.

## Code Change Surface

- Selected approach: a **dedicated** GCodeEmit composite entry point rather than a `CapturedIr::GCodeEmit` arm inside `render_silhouette_composite_styled`. This deviates from packet 249's `[FWD]` suggestion, deliberately: the inversion needs the resolved `filament_diameter`, which the styled signature does not carry — threading it would churn 247/249's pinned signatures and their `RenderStyle::default()` byte-equivalence AC. The dedicated entry reuses the same private union/class-order/rectangle-draw machinery in-module, so paint order and determinism rules stay single-sourced. Fallout check: no 247–249 AC pins the styled entry point handling `GCodeEmit` (249's AC-N1 pins the tap's *rejection*, which this packet retires per its `[FWD]`).

- Exact surface, per file:
  - `crates/slicer-runtime/src/visual_debug_render.rs`
    - `pub fn silhouette_flow_width_mm(e_delta_mm: f64, length_mm: f64, slab_height_mm: f64, filament_diameter_mm: f64) -> f64` — packet 248's closed form, promoted verbatim (`Δe × π(d/2)² / (L × h)`).
    - `pub struct GcodeEmitSegment { pub tool: u32, pub role: slicer_ir::ExtrusionRole, pub slab_index: u32, pub h0_mm: f32, pub h1_mm: f32, pub width_mm: f32 }` and `pub fn gcode_emit_silhouette_segments(g: &slicer_ir::GCodeIR, view: SilhouetteView, schedule: &SilhouetteSlabSchedule, filament_diameter_mm: f32) -> (Vec<GcodeEmitSegment>, Vec<String>)` — the pub inversion walk (directly unit-pinnable, AC-2/AC-3): carries `(x, y, z)` position (each `Some` coordinate updates the carry; `None` retains), last `Some(e)` value, and current tool (0 initial, updated by `ToolChange { to }`); an extruding move is a `Move` with `e: Some(cur)` where `Δe = cur − last_e > 0` and 3D length `> 0`; width via `silhouette_flow_width_mm` with `h` from the bucketed slab; `h0`/`h1` are the segment endpoints projected on the view's horizontal axis; travel/negative-delta/zero-length moves produce no segment; `last_e` updates on **every** `Some(e)` (including negative deltas — the position stream is authoritative). Bucketing: containing slab (`z_bottom < z ≤ z_top` over the full schedule); no containing slab → nearest slab (minimum distance to `[z_bottom, z_top]`, ties → lower index) + one W4 warning per distinct affected Z (`gcode emit: extruding move at z={z:.3} outside every schedule slab; drawn at nearest slab [{b:.3}, {t:.3}]`), distinct Zs sorted ascending, capped at 8 with a `+N more` tail.
    - `pub fn render_gcode_emit_silhouette(g: &slicer_ir::GCodeIR, view: SilhouetteView, resolution_scale: u32, viewport: ViewportBoundsMm, schedule: &SilhouetteSlabSchedule, style: &RenderStyle, filament_diameter_mm: f32) -> Result<(RenderedImage, Vec<String>), RenderError>` — scale check as the existing entry points; segments → intervals `[min(h0,h1) − w/2, max(h0,h1) + w/2]` unioned per (slab, class) via `union_silhouette_intervals`; class = role (packet 249's role rank order) or `tool_index` ascending (`ColorBy::Tool`); rectangles emitted ascending slab index → class order → interval start; zero rectangles across the group → `RenderError::MissingGeometryField` (247's rule); returns W4 warnings (deduped, deterministic order).
  - `crates/slicer-runtime/src/lib.rs` — re-export `render_gcode_emit_silhouette`, `gcode_emit_silhouette_segments`, `GcodeEmitSegment`, `silhouette_flow_width_mm` beside the 247–249 re-exports.
  - `crates/pnp-cli/src/visual_debug.rs`
    - `run_postpass_taps`: emitter gains `.with_resolved_config((*ctx.default_resolved_config).clone())` (the fidelity fix — one line, AC-9); return type extended to carry the sorted finalized `(u32, f32)` layer schedule (tuple or small struct; one call site in `run_model_source`).
    - Silhouette assembly branch: the `SilhouetteSlabSchedule` for **both** postpass tap groups is built from the plumbed finalized schedule filtered to the resolved selection (if packet 249 landed a capture-payload-derived construction, refactor it onto this source — same data, byte-identical output, proven by re-running 249's suite); the `PostPass::GCodeEmit` group calls `render_gcode_emit_silhouette` with `ctx.default_resolved_config.filament_diameter` and the request's `RenderStyle`; `layers_rendered` = selection ∩ schedule indices as maximal `LayerRangeEntry` ranges; filenames `PostPass__GCodeEmit_silhouette_{view}[_tool].png` via `sanitize_path_component`; entry fields exactly per 247/249 (`view`, `layers_rendered`, `color_by`/`tool_color_source`/`tool_palette` on tool groups, no `layer_index`/`layer_z`, `typed_capture: None`).
    - `validate_request`: `SILHOUETTE_TAP_STAGE_IDS` += `"PostPass::GCodeEmit"`; nothing else changes (gcode-source tap rejection, R4/R5, mixing ban untouched).
  - `crates/pnp-cli/src/visual_debug_gcode.rs` — `silhouette_segment_width_mm` body becomes a one-line delegation to `slicer_runtime::silhouette_flow_width_mm` (pub signature unchanged; 248's AC-2 pin stays green).
  - Tests: new `crates/pnp-cli/tests/visual_debug_gcode_emit_silhouette_tdd.rs`; edits to `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (renderer ACs) and `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (AC-N2 retire/retarget, AC-N3 re-run).
  - Docs: `docs/19_visual_debug.md`.

- Rejected alternatives and reasons:
  - `CapturedIr::GCodeEmit` arm inside `render_silhouette_composite_styled` — needs a diameter parameter the pinned signature lacks (see Selected approach).
  - Deriving slabs by text-parsing the `Raw { text: ";Z:..." }` / `";HEIGHT:..."` comments in the typed stream — a second text parser over a typed IR, exactly the fragility D11's "second implementation" cost warning is about; the finalized-layer schedule is typed and already computed.
  - Inverting with `ResolvedConfig::default()`'s 1.75 mm diameter instead of fixing the emitter config — the image would claim widths derived from a diameter the request never set (the confidently-wrong-image failure mode), and the captured IR would keep misrepresenting the real pipeline.
  - Bucketing by nearest slab for **all** moves (no containment test) — turns the honest common case into an approximation; containment first, nearest only as the warned fallback.
  - Resetting the carried E position on `e: None` moves — falsified by the emitter: zero-delta moves emit `None` without touching `e_position`, so a reset would fabricate a giant `Δe` at the next `Some`.
  - Skipping negative-delta moves *without* updating `last_e` — wrong: the position stream is cumulative, so the post-retract extrusion's `Δe` must difference against the retracted position, not the pre-retract one.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/visual_debug_render.rs` — inversion walk, promoted width formula, dedicated render entry; the packet's largest edit.
- `crates/pnp-cli/src/visual_debug.rs` — emitter-config fix, schedule plumbing, assembly branch, tap lift.
- `crates/slicer-runtime/src/lib.rs` + `crates/pnp-cli/src/visual_debug_gcode.rs` — re-exports and the one-line width delegation.
- Tests: new `crates/pnp-cli/tests/visual_debug_gcode_emit_silhouette_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`; `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`.
- Docs: `docs/19_visual_debug.md`.

Justification for exceeding three primaries: the same two-crate split packets 247–249 established (CLI assembly vs runtime pixel math); the lib.rs/gcode.rs edits are one-liners; each step stays ≤3 edits.

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` — the `emit` accumulation region only (`e_position`, `filament_area`, the `Move` push, travel/z-hop emission) — purpose: fact-9 semantics and fixture design; never edited.
- `crates/slicer-ir/src/slice_ir.rs` — `GCodeCommand`/`GCodeIR`/`PrintMetadata` shapes only — never edited.
- `crates/slicer-runtime/src/postpass.rs` — `PostPassCapture` fields only — never edited.
- `crates/slicer-ir/src/resolved_config.rs` — `filament_diameter` row only (schema default 1.75) — never edited.
- Packets 247/248/249 `packet.spec.md` + `design.md` — export contracts; never edit those directories.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `crates/slicer-gcode/src/serialize.rs` and the serializer path — text serialization is packet 248's inversion target, not this one's.
- `modules/core-modules/**` sources — guest artifacts matter only as prebuilt test dependencies (`cargo xtask build-guests --check` before blaming e2e failures).
- Packet directories 247/248/249/251 (beyond the read-only spec/design files) — never create or modify files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate.

## Expected Sub-Agent Dispatches

- Question: exact landed names/signatures of 249's `postpass_stage_captures`, `run_postpass_taps` shape parameter, and the silhouette branch's schedule construction; scope: `crates/pnp-cli/src/visual_debug.rs`; return: `SNIPPETS ≤2×30 lines`; purpose: Steps 3–4.
- Question: landed name of 248's promoted union helper and 247's class-order internals in `crates/slicer-runtime/src/visual_debug_render.rs`; return: `FACT`; purpose: Step 1–2 reuse.
- Question: run the step's test binary tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step.
- Question: `cargo xtask build-guests --check` exit code (0/1/3) before wedge e2e steps; scope: repo root; return: `FACT`; purpose: Steps 5–6.
- Question: `cargo xtask check-literals` exit code after new fixtures; return: `FACT`; purpose: Steps 2/5.
- Question: does any test outside `visual_debug_agent_determinism_tdd` pin absolute bytes/values of a model-source postpass `typed_capture` or its rendered PNG?; scope: `crates/pnp-cli/tests/`, `crates/slicer-runtime/tests/`; return: `LOCATIONS ≤10`; purpose: Step 3 fallout sweep for the emitter-config fix.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — assembly + end-to-end bundle tests over the wedge postpass pipeline)
- Highest-risk dispatch and required return format: Step 5/6 wedge e2e runs (guest-WASM-dependent); `FACT pass/fail` preceded by the `build-guests --check` exit code (0/1/3).

## Open Questions

- `[FWD to packet 251]` No exports here affect seam overlays; if your composited glyphs land on a GCodeEmit-tool-colored base, the glyph-visibility note 249 forwarded to you applies to this tap's palette too.
- `[FWD to the batch orchestrator]` 247's unowned-tap question (`PrePass::RegionMapping`/`PrePass::OverhangAnnotation`) remains unowned after this packet — the queue's last silhouette-capable-tap packet. Recommend a follow-up packet decision at batch close.
- `[FWD to implementer]` If packet 249 landed `run_postpass_taps`'s schedule differently than assumed (e.g. already returning finalized `(index, z)` pairs), reuse the landed shape — the contract is one schedule source for both postpass groups, not a specific plumbing vessel.
- No `[BLOCK]` items.
