---
status: implemented
packet: 105_classic-spacing-fill-mmu
task_ids:
  - T-050
  - T-051
  - T-052
  - T-053
  - T-054
  - T-054b
  - T-054c
  - T-060
  - T-061
  - T-062
  - T-062b
  - T-063
  - T-064
  - T-065
  - T-P96-A0
  - T-P96-B
---

# 105_classic-spacing-fill-mmu

## Goal

Land the OrcaSlicer-parity wall-emission geometry stack — distinct outer/inner extrusion widths with `ext_perimeter_spacing2` arithmetic, all three `wall_sequence` modes including `InnerOuterInner` sandwich, thin-wall detection with `LoopType::ThinWall` emission, gap-fill emission via the new `LoopType::GapFill`/`ExtrusionRole::GapFill` variants, and OrcaSlicer-parity MMU per-color outer-wall fragmentation (**Model A — partition / both-trace**, per the source-grounded rewrite of ADR-0013): remove the `external_contour` union-trace consumption from **both** perimeter modules so each per-color `SlicedRegion` traces its own outer wall independently. There is **no skip mask** — `bisector_edge_skip_mask` is NOT introduced; the prior skip-mask draft is removed (see D-105-MMU-MODEL-PIVOT, D-105-BISECTOR-MASK-DROPPED). (`variable-width-perimeters` never ships — see D-110-DROP-VARIABLE-WIDTH; fake-Arachne module deleted under P108. T-P96-C0/C1/C2 are dropped — Model A needs no per-cell mask consumer.)

## Problem Statement

`classic-perimeters` currently emits walls with a single configurable `line_width` (not distinguishing outer from inner), with a constant inter-wall spacing that ignores OrcaSlicer's `ext_perimeter_spacing2 vs perimeter_spacing` distinction, no thin-wall detection, no gap-fill, no `wall_sequence` modes, and an MMU mechanism (`external_contour` from P96) that union-traces the model perimeter once per painted object — diverging from OrcaSlicer's per-color outer-wall fragmentation. The four defects compound: incorrect spacing on multi-width prints, missing thin features, gap-filled by infill or left as voids, single-color MMU wall regardless of paint, and an unparsable single sequence of walls per region (no sandwich mode, no inner-first option). (`variable-width-perimeters` never ships per D-110-DROP-VARIABLE-WIDTH; the fake-Arachne module is deleted under P108.)

This packet lands the entire wall-emission geometry stack in one coordinated change because the four workstreams touch the same `lib.rs` files (the perimeter modules), the same IR (`SlicedRegion`, `LoopType`, `ExtrusionRole`), and the same host-side surface (`paint_segmentation`). Splitting would require three sequential touches of the same files, each with its own compile-cycle and AC churn. The MMU foundation (T-P96-A0/B) folds in because T-P96-B modifies the same per-cell wall-trace loop that the wall_sequence + thin-wall + gap-fill code paths rewrite — coupling at the LOC level, not just at the conceptual level. T-P96-A0 lands first as a doc-only investigation step so the Model A decision is grounded in OrcaSlicer source (confirmed: each per-color region traces its own outer wall independently; no shared-bisector skip mask used).

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
