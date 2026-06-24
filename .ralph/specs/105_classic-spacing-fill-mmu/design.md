# Design: 105_classic-spacing-fill-mmu

## Controlling Code Paths

- Primary code path: `slicer_core::flow::line_width_to_spacing` (new) drives the spacing arithmetic in both perimeter modules. `slicer_core::perimeter_utils::wall_sequence_reorder` (new) takes the generated `Vec<WallLoop>` + in-module wall tree and reorders per the configured `WallSequence`. Both perimeter modules' `run_perimeters` is rewritten to (a) compute outer/inner widths separately from config, (b) run thin-wall detection via `medial_axis`, (c) collect gaps per-inset and emit gap-fill via `medial_axis`, (d) invoke `wall_sequence_reorder` before commit. `external_contour` union-trace consumption is removed from both modules — each per-color `SlicedRegion` traces its own outer wall independently via `offset_ex(-ext_perimeter_width/2)` per region (Model A). The `external_contour` IR field stays (deleted in P107 T-P96-D).
- Neighboring tests / fixtures: 6 new TDD files. Existing `boundary_paint_tdd.rs`, `arachne_perimeters_tdd.rs`, and `classic_perimeters_tdd.rs` regression tests must stay green. The 4-color cube fixture is reshaped in this packet: protected executor test renamed `cube_4color_per_layer_per_color_fragmentation_with_tool_changes`; G-code SHA re-baselined as `P105_CUBE_4COLOR_PARITY_SHA`.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- ADR-0011 invariant: `PerimeterRegion.walls` is committed in final print order; wall tree is in-module scaffolding only and never crosses the module boundary. `wall_sequence_reorder` operates on the tree IN-process and discards it after producing the final flat `Vec<WallLoop>`.
- ADR-0013 invariant (Model A): `external_contour` union-trace consumption is removed from both perimeter modules; each per-color `SlicedRegion` runs an independent perimeter pass. No `bisector_edge_skip_mask`, no skip-mask carrier, no tie-break rule needed. Source-confirmed against OrcaSlicer (see rewritten ADR-0013 and T-P96-A0 one-pager).
- Schema-version contract: bump from the live `CURRENT_SLICE_IR_SCHEMA_VERSION` value (`4.3.0` as of branch head) to `4.4.0` (additive — new `GapFill` enum variants). Computed at activation; do not hardcode. Existing fixtures stay parseable via `#[non_exhaustive]` on enums.
- WIT type identity: `wall-loop-type` (in `ir-types.wit`) and `extrusion-role` (in `types.wit`) both gain a `gap-fill` arm. Per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass after WIT edit.
- `LoopType::GapFill` and `ExtrusionRole::GapFill` add match arms in every consumer that exhaustively matches the enum. The `ir_to_wit_extrusion_role` function in `crates/slicer-wasm-host/src/marshal/leaf.rs:183` is an EXHAUSTIVE match — adding `ExtrusionRole::GapFill` to the IR enum breaks the build unless the WIT `extrusion-role` variant (`gap-fill`) AND the `leaf.rs` match arm land in the SAME atomic sub-step (Step 2a). The implementer enumerates ALL exhaustive-match consumers via a delegated LOCATIONS dispatch (Step 2 in implementation plan) and adds them in 2b.
- Per-layer config rule (carries from P102): all new config keys (`outer_wall_line_width`, `inner_wall_line_width`, `precise_outer_wall`, `wall_sequence`, `detect_thin_wall`, `gap_infill_speed`, `filter_out_gap_fill`) MUST be read via `_config.get*` per `run_perimeters` invocation, not cached at `on_print_start`.

## Code Change Surface

- Selected approach: bundle the four workstreams in one packet because they share the same `lib.rs` editing surface in both perimeter modules; splitting forces three sequential touches of the same file with inter-packet AC churn. Pipeline within each module's `run_perimeters` becomes: read configs → compute outer/inner widths → build wall geometry (with new spacing) → run thin-wall detection (medial_axis) → run gap collection + gap-fill emission → reorder via wall_sequence → commit. Each phase is a discrete pure-function call to `slicer-core`; the module orchestrates. T-P96-A0 produces the doc one-pager that confirms Model A (partition/both-trace; no skip mask), grounding ADR-0013.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/flow.rs` (NEW) — `pub fn line_width_to_spacing(width, layer_height, nozzle_diameter) -> f32`; `pub fn flow_to_width(spacing, layer_height, nozzle_diameter) -> f32`.
  - `crates/slicer-core/src/perimeter_utils.rs` — extend with `pub fn wall_sequence_reorder(&mut Vec<WallLoop>, WallSequence, &[PolygonTreeNode])`; add `pub enum WallSequence { OuterInner, InnerOuter, InnerOuterInner }` (NET-NEW in `slicer-core`; the existing `WallSequence` in `modules/core-modules/path-optimization-default/src/lib.rs:46` has only `InnerOuter`/`OuterInner` — no `InnerOuterInner`, and is local to that module; the ADR-0011-compliant home is `slicer_core::perimeter_utils`).
  - `crates/slicer-core/src/lib.rs` — `pub mod flow;` declaration.
  - `crates/slicer-ir/src/slice_ir.rs` — add `LoopType::GapFill`; add `ExtrusionRole::GapFill`; mark both `#[non_exhaustive]`; bump `CURRENT_SLICE_IR_SCHEMA_VERSION` from live `4.3.0` to `4.4.0`. NOTE: `variable_width` is defined here at `slice_ir.rs:1627` and re-exported from `crates/slicer-ir/src/lib.rs:160` — it is a `slicer-ir` function, NOT `slicer-core`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `wall-loop-type` enum gains `gap-fill` arm. `crates/slicer-schema/wit/deps/types.wit` — `extrusion-role` variant gains `gap-fill` arm. (Two files, separate WIT locations — keep as two edits in sub-step 2a.)
  - `modules/core-modules/classic-perimeters/src/lib.rs` — full `run_perimeters` rewrite per the pipeline above; remove `external_contour` consumption (classic already correct — verify only); per-color outer-wall tracing confirmed independent.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — mirror; delete `by_object` shared-boundary branch so arachne also uses per-cell `emit_outer=true` (Model A).
  - `modules/core-modules/{classic,arachne}-perimeters/*.toml` — register 7 config keys.
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` — deregister `wall_sequence`.
  - `modules/core-modules/path-optimization-default/src/lib.rs` — migrate `WallSequence` usages: the module-local `WallSequence` enum (lines 46-51, variants `InnerOuter`/`OuterInner`) and its config read (lines ~276-295) are replaced by consuming `slicer_core::perimeter_utils::WallSequence` which adds `InnerOuterInner`. All call sites (struct field line 143, match lines 161-163, config-read parse lines 278-279) migrate. The local enum definition is removed.
  - `modules/core-modules/part-cooling/src/lib.rs` — `ExtrusionRole::GapFill` match arm (fan dispatch).
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` or host GCodeEmit role priority table — `ExtrusionRole::GapFill` match arm.
  - 6 new TDD files.
  - `docs/specs/orca-mmu-perimeter-investigation.md` (NEW) from T-P96-A0.
  - 4 doc edits per Doc Impact Statement.
- Rejected alternatives that were considered and why they were not chosen:
  - Split spacing/thin-walls/gap-fill/MMU into 4 packets: rejected — same-file edits forced 4× compile cycles and AC churn for no architectural benefit.
  - Store the wall tree in `PerimeterRegion`: rejected per ADR-0011 — IR stays flat.
  - Reuse `LoopType::ThinWall` for gap-fill geometry: rejected per ADR-0013 / D-8 closure — `GapFill` is structurally distinct (different semantics, different downstream role-priority bucket).
  - Model B (bisector skip mask, host computes mask per-edge): rejected — source-confirmed OrcaSlicer uses Model A (partition/both-trace); mask approach was a misread. See ADR-0013 (rewritten) and D-105-BISECTOR-MASK-DROPPED.

## Files in Scope (read + edit)

Primary edit surface lists ~13 files because the packet bundles 17 tasks per the user's "as few packets as logically possible" directive. The **three highest-LOC-delta** files are listed first; the rest are justified as small mechanical additions.

- `modules/core-modules/classic-perimeters/src/lib.rs` — role: `run_perimeters` rewrite; expected change: ~250 LOC delta (new spacing + thin-wall + gap-fill + wall_sequence reorder; external_contour consumption verified absent).
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: mirror of classic + delete `by_object` shared-boundary branch; expected change: ~250 LOC delta.
- `crates/slicer-core/src/perimeter_utils.rs` — role: `wall_sequence_reorder` + `WallSequence` enum; expected change: ~120 LOC added.
- `crates/slicer-core/src/flow.rs` (NEW) — role: Flow math; expected change: ~80 LOC.
- `crates/slicer-ir/src/slice_ir.rs` — role: enum variants + schema bump; expected change: ~20 LOC.
- `crates/slicer-schema/wit/deps/ir-types.wit` — role: WIT mirrors; expected change: ~10 LOC.
- `crates/slicer-wasm-host/src/host.rs` + `crates/slicer-sdk/src/views.rs` — role: GapFill match arm additions in role-dispatch; expected change: ~5 LOC each.
- `modules/core-modules/{classic,arachne}-perimeters/*.toml` — 7 config keys each; ~30 LOC each.
- `modules/core-modules/path-optimization-default/path-optimization-default.toml` — deregister 1 key.
- `modules/core-modules/part-cooling/src/lib.rs`, `machine-gcode-emit/src/lib.rs` (or host) — 1-3 line match arm additions.
- `docs/specs/orca-mmu-perimeter-investigation.md` (NEW), 4 other docs.

## Read-Only Context

- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — read full — purpose: confirm IR-flat-list invariant and `wall_sequence` ownership.
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — read full — purpose: confirm Model A (partition/both-trace; no mask; no tie-break).
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read Phase 5, Phase 6, and "Inherited from P96" sections.
- `docs/02_ir_schemas.md` — delegate SUMMARY for `LoopType`, `ExtrusionRole`, `SlicedRegion`, schema-version contract.
- `docs/01_system_architecture.md` — read §"Crate Boundaries" full — purpose: align new `flow` module + `perimeter_utils` extension with crate placement convention.
- `docs/15_config_keys_reference.md` — range-read §"Walls" and §"Quality".
- `CLAUDE.md` — §"Guest WASM Staleness" + §"WIT/Type Changes Checklist".

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `crates/slicer-core/src/algos/mesh_analysis.rs` — out of scope (BridgeRegion / OverhangRegion handling belongs to other packets).
- `crates/slicer-core/src/algos/prepass_slice.rs` — out of scope (no `bisector_edge_skip_mask` initializer to add; field was dropped per Model A pivot).
- `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs` — out of scope (no `compute_bisector_edge_skip_mask` to add; Model A needs no host-side mask computation; see D-105-BISECTOR-MASK-DROPPED).
- `modules/core-modules/seam-placer/src/lib.rs` — explicitly out of scope (Phase 8 work, P106).
- All other `modules/core-modules/*/src/lib.rs` except the two perimeter modules + part-cooling + machine-gcode-emit role-arm consumers — out of scope.
- All other `crates/slicer-runtime/src/` files — out of scope.
- Other `.ralph/specs/<packet>/` directories — only P102/P103/P104 are referenced as preconditions; delegate FACT if needed.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp + PerimeterGenerator.cpp per-color branches for the MMU outer-wall fragmentation; confirm Model A (each per-color region traces its own independent outer wall via per-region offset, no bisector skip mask); cite file:line. Return SUMMARY ≤ 200 words." — Step 1 (T-P96-A0 deliverable).
- "Summarize OrcaSlicerDocumented/src/libslic3r/Flow.cpp for `Flow::new_from_width_height` math; return SUMMARY ≤ 100 words." — Step 4.
- "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644 for ext_perimeter_spacing2 + precise_outer_wall gating; return SUMMARY ≤ 150 words." — Step 4.
- "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913 for wall_sequence reorder including InnerOuterInner sandwich; return SUMMARY ≤ 200 words, no code." — Step 5.
- "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609 + 1665-1670,1930-1958 for thin-wall + gap-fill cascades; return SUMMARY ≤ 200 words." — Step 6.
- "Find all exhaustive matches on `LoopType` or `ExtrusionRole` across the workspace; return LOCATIONS ≤ 20 entries each." — Step 2 (post-`#[non_exhaustive]` add — confirms which consumers need a new `GapFill` arm).
- "Run `cargo check --workspace --all-targets` after each step; return FACT pass/fail + SNIPPETS ≤ 20 lines on fail."
- "Run targeted test per AC; return FACT pass/fail per case."
- "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list ≤ 5 entries)." — Step 2 closure gate.

## Data and Contract Notes

- IR or manifest contracts touched: `LoopType` + `ExtrusionRole` enums gain a `GapFill` variant via additive bumps; schema version → `4.4.0`. WIT mirrors: `wall-loop-type` in `ir-types.wit` + `extrusion-role` in `types.wit` gain `gap-fill`. `CURRENT_SLICE_IR_SCHEMA_VERSION` → `4.4.0` (live value as of branch head: `4.3.0`; bump computed at activation).
- WIT boundary considerations: enum variant additions are backward-compatible only if downstream code is exhaustive-match-tolerant. The `#[non_exhaustive]` attribute on both enums is the contractual guarantee. Per CLAUDE.md, after WIT edit run `cargo build --tests --workspace` to catch type identity break.
- Determinism constraint: all new perimeter paths MUST be deterministic across runs for the same input. `wall_sequence_reorder` is a pure function: same `Vec<WallLoop>` + same `mode` + same tree → same output. No randomness, no global state.
- `external_contour` IR field stays in `SlicedRegion` after this packet — only the **consumption** in both perimeter modules is removed. Field deletion is T-P96-D in P107 after the new mechanism is green in production.
- The 4-color cube fixture executor test is reshaped in this packet: renamed `cube_4color_per_layer_per_color_fragmentation_with_tool_changes`; G-code SHA re-baselined as `P105_CUBE_4COLOR_PARITY_SHA`.

## Locked Assumptions and Invariants

- `WallSequence::OuterInner` reverses the canonical `[Outer, Inner_0, Inner_1, …]` order to `[…, Inner_1, Inner_0, Outer]`. `InnerOuter` is canonical. `InnerOuterInner` (per outer contour): `[Inner_0, Outer, Inner_1, …]` — first inner, then outer, then remaining inner walls.
- **MMU is Model A** — each per-color `SlicedRegion` runs an independent perimeter pass; no skip mask, no tie-break rule, no shared-bisector ownership. Source-confirmed against OrcaSlicer (T-P96-A0); see rewritten ADR-0013 and D-105-BISECTOR-MASK-DROPPED.
- `ext_perimeter_spacing2 = (outer_wall_line_width + inner_wall_line_width) / 2` (the OrcaSlicer formula). Documented in `flow.rs` doc-comment.
- `wall_sequence_reorder` is a pure function: same `Vec<WallLoop>` + same `mode` + same tree → same output. No randomness, no global state.
- `1 unit = 100 nm` invariant preserved in all new spacing arithmetic. Every mm↔unit boundary uses `from_mm` / `units_to_mm` helpers; raw `* 10_000.0` is forbidden.
- `flow.rs` and `perimeter_utils/wall_sequence.rs` placed in `slicer-core` per docs/13 §Out of Scope. Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.

## Risks and Tradeoffs

- Packet size (17 tasks across 4 workstreams) is at the upper limit of single-Ralph-run usability. Mitigation: 8 explicit steps (7 source + 1 doc-impact landing, with Step 3 dropped) with files-to-edit ≤ 3 each; every AC verifiable in isolation. If the implementer's context approaches 70% during Step 4 (the largest), they halt and resume in a fresh agent for Step 5 onward.
- `wall_sequence` deregistration from `path-optimization-default` is a small mechanical change but touches a module not otherwise in scope. Verified: the key is consumed-nowhere in path-optimization (it was registered there as a vestige per ADR-0011); deregistration is a manifest-only edit, no source changes.
- Adding `#[non_exhaustive]` to `LoopType` and `ExtrusionRole` is a one-time backward-compat improvement but forces every exhaustive `match` on these enums to add a wildcard or new arm. The Step 2 LOCATIONS dispatch enumerates these for the implementer.
- Schema bump (live `4.3.0` → `4.4.0`) races with any other in-flight packet that might bump to `4.4.0` first. The actual target version is computed at activation from the live constant; the implementer records the actual bump chosen in the closure log. No hardcoded version in ACs — the AC asserts variant presence, not a literal version string.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (large M — at the edge; consider this packet a risk-flagged M).
- Largest single step: `M` (Step 4 — module-side spacing model + width plumbing + new TDD).
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): OrcaSlicer `wall_sequence` SUMMARY (≤ 200 words). The sandwich-mode algorithm is structurally complex; if the SUMMARY returns code, re-dispatch with explicit "no code, behavioral description only" cap.

## Open Questions

- `[FWD]` `WallSequence` enum location: `slicer_core::perimeter_utils` is the assumed home (per existing T-054 row in roadmap). If a more canonical home exists (`slicer-ir`?), the implementer can relocate; cross-roadmap impact is negligible.
- `[FWD]` `Flow::new_from_width_height` parity: the minimal port should be sufficient for `line_width_to_spacing(width, layer_height, nozzle_diameter) -> f32`. If the implementer finds the formula needs an additional `bridge_flow_ratio` parameter or similar, document and add — but only if a test demands it.
