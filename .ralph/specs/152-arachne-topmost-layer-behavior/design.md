# Design: 152-arachne-topmost-layer-behavior

## Controlling Code Paths

- Primary code paths:
  - WIT record: `crates/slicer-schema/wit/deps/common.wit:26-50` (`record
    arachne-params`, currently has `is-initial-layer` at `:43`, no top/bottom) —
    add `is-bottom-layer` / `is-topmost-layer` bools.
  - Rust mirror: `crates/slicer-core/src/arachne/pipeline.rs` `ArachneParams`
    (`is_initial_layer` at `:144`; no top/bottom fields) + `Default` (`:180-208`).
  - SDK bridge: `crates/slicer-sdk/src/host.rs` — `generate_arachne_walls`
    (`:545`) holds BOTH conversion directions itself: `:551` builds the core
    `ArachneParams` (native path) and `:690` builds the WIT record (wasm path).
    There is NO `ArachneParams` adapter in `slicer-macros` (zero matches; only a
    routing comment at `slicer-macros/src/lib.rs:543`) — do not plan edits there.
  - Host-side service impl: `crates/slicer-wasm-host/src/host.rs:1773-1794` —
    `generate_arachne_walls` maps the incoming WIT record field-by-field into
    the core struct (`is_initial_layer: params.is_initial_layer`, …). This file
    MUST gain the two field mappings or host instantiation of the service
    breaks; it is the only `ArachneParams` reference in `slicer-wasm-host`.
  - removeSmallLines: `crates/slicer-core/src/arachne/remove_small.rs:42-82` —
    `remove_small_lines(lines, min_length_factor, _min_width, is_initial_layer)`
    → add a top/bottom flag; lenient `min_width/2` when top OR bottom.
    PRECISION: the `min_width` used by the threshold (`:75-79`) is a per-line
    local derived from the line's junction widths (`:63-67`, `fold(min)`), NOT
    the `_min_width` parameter (discarded). Keep that per-line derivation — do
    not wire the threshold to the passed param, or strict-path behavior changes
    beyond the G10 fix (the G10 red test's `0.4` arg is annotated "unused by the
    per-line threshold").
  - Pipeline entry: `crates/slicer-core/src/arachne/pipeline.rs:317-321`
    `run_arachne_pipeline(polygons, params, is_initial_layer)` — the top/bottom
    signal rides in `params` (the WIT record already flows through), so the
    threshold reads `params.is_topmost_layer || params.is_bottom_layer`.
  - Module: `modules/core-modules/arachne-perimeters/src/lib.rs:293` (sets
    `params.is_initial_layer = layer_index == 0`) + `:305-306` (the
    `only_one_wall_top` discard) — detect topmost via
    `SliceRegionView::top_shell_index` (no such read exists today), set the WIT
    flags, force single wall on topmost, and run the second pass for G3 part 2.
- Neighboring tests: `arachne_parity_gaps.rs` (G3 `:246-269`, fixture
  `region.set_top_shell_index(Some(0))` at `:252`; G10 `:564-613`, call at
  `:591-596`, assertion at `:598` — G10's CALL adapts, assertion preserved),
  `arachne_parity.rs` (15 locks incl. the
  `arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface`
  source-read lock at `:591`, probing via `include_str!` at `:47-48`).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference
  Obligations (delegate; never load `PerimeterGenerator.cpp:2160-2246`).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **WIT/Type Changes checklist (CLAUDE.md):** editing `common.wit`'s
  `arachne-params` invalidates every guest's bindgen. Search `wit_host.rs`,
  `dispatch.rs`, and `wit_guest`/adapter modules for `arachne-params` /
  `ArachneParams`; verify type identity across the boundary (field order +
  types); run `cargo build --tests` then `cargo xtask build-guests` after the
  WIT edit. This is the ONLY packet of the three touching `common.wit`.
  (`slicer:common` is an unversioned WIT package — `deps/common.wit:1` — and
  docs/03 states no version-bump rule for record field additions; the world
  packages carry `@1.0.0` but the compatibility matrix governs world MAJOR
  loads, not field additions.)
- **ADR-0035 conformance:** `removeSmallLines` is EXPLICITLY in ADR-0035's
  faithful-port function list — the G10 top-or-bottom keying is the
  faithfulness fix that ADR mandates, not a deviation. The second-pass caller
  logic (G3 part 2) lives in the module, outside ADR-0035's named
  `slicer-core/src/arachne` scope, BUT that ADR's recording bar requires
  algorithm divergences to be recorded via an amending/new ADR, not merely a
  DEVIATION_LOG entry — see Open Questions for the top-area-derivation
  consequence. ADR-0033 fixes only the bridge's plain-data calling convention;
  adding two bools conforms.

## Code Change Surface

- Selected approach: carry the top/bottom signal inside the existing
  `arachne-params` record (it already crosses the boundary) rather than adding a
  new host-service parameter — minimal WIT delta, mirrors Orca's
  `WallToolPathsParams.is_top_or_bottom_layer`. Two bools (`is-bottom-layer`,
  `is-topmost-layer`) rather than one `is-top-or-bottom` (user decision) so the
  module can also drive G3's topmost-only single-wall force from the same fields.
- Exact changes:
  - `common.wit`: two bool fields on `arachne-params`.
  - `pipeline.rs`: two `ArachneParams` fields + defaults (`false`);
    `run_arachne_pipeline` passes them to `remove_small_lines`.
  - `remove_small.rs`: threshold keys on `is_bottom || is_topmost`; audit
    `is_initial_layer` remaining consumers before subsuming it. Also gains a NEW
    `#[cfg(test)] mod tests` housing `non_top_layer_strict` (no test module
    exists in the file today; without it the AC-N1 `--lib` filter false-passes
    with "0 tests run").
  - `crates/slicer-sdk/src/host.rs`: the two fields at BOTH conversion sites
    (`:551` native, `:690` WIT). NOT `slicer-macros` — no `ArachneParams` code
    exists there.
  - `crates/slicer-wasm-host/src/host.rs:1773-1794`: the two field mappings in
    the host-side WIT→core conversion.
  - `arachne-perimeters/src/lib.rs`: read `top_shell_index`; set the WIT flags;
    `only_one_wall_top` topmost single-wall force; the second-pass generation
    (PnP already ships `offset2_ex` at `crates/slicer-core/src/polygon_ops.rs:345`,
    mirroring ClipperUtils — no new polygon helper needed).
  - Tests: packet-authored `tests/only_one_wall_top_tdd.rs` in the module
    (houses `only_one_wall_top_second_pass` + `only_one_wall_top_disabled`,
    following the 10 existing native `*_tdd.rs` files that drive
    `ArachnePerimeters::run_perimeters` via `slicer_sdk::traits::LayerModule`;
    standalone ⇒ auto-registered) and `non_top_layer_strict` in
    remove_small.rs's new test mod.
- Rejected alternatives: (a) a single `is-top-or-bottom` bool — rejected per user
  decision (G3 needs to distinguish topmost specifically); (b) a new host-service
  parameter outside the record — rejected (bigger WIT surface, the record already
  flows).

## Files in Scope (read + edit)

Primary:

- `crates/slicer-schema/wit/deps/common.wit` — the two WIT fields.
- `crates/slicer-core/src/arachne/{pipeline.rs,remove_small.rs}` — struct +
  threshold (count as one concern: the pipeline plumbing).
- `modules/core-modules/arachne-perimeters/src/lib.rs` — topmost detection + G3
  behavior (the largest change).

Secondary (mechanical): `crates/slicer-sdk/src/host.rs` (`:551`, `:690`),
`crates/slicer-wasm-host/src/host.rs` (`:1773-1794`). The packet exceeds ≤3
because a WIT record change intrinsically fans out to schema + core + SDK +
wasm-host + module; each is a localized, mirror-the-field edit.

## Read-Only Context

- `crates/slicer-runtime/tests/arachne_parity_gaps.rs` — G3 `:246-269`, G10
  `:564-613` — purpose: exact assertions + the G10 call to adapt.
- `crates/slicer-sdk/src/views.rs:184-210` — `set_top_shell_index` (`:190`) /
  `set_top_solid_fill` (`:205`); backing fields on `SliceRegionView` (`:41`,
  `:45`) and IR `SlicedRegion` (`slice_ir.rs:1298`, `:1304`). Live values come
  from the ShellClassification prepass built-in (entry fn documented at
  `slice_postprocess_prepass.rs:86`; "PrePass::ShellClassification" is
  doc-comment vocabulary, not a literal enum variant in that file) — purpose:
  how the module reads topmost.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs:144-150` — where the
  host applies `top_shell_index`/`top_solid_fill` edits — purpose: confirm the
  field the module keys on is populated on the live path.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; `PerimeterGenerator.cpp:2160-2246` is
  dense — SUMMARY only.
- `target/`, generated `*/wit-guest/` bindgen, `Cargo.lock` — never load.
- `crates/slicer-runtime/tests/arachne_parity.rs` in full — grep only.

## Expected Sub-Agent Dispatches

- "Summarize the second `WallToolPaths` pass algorithm at
  `PerimeterGenerator.cpp:2160-2246`: top-area derivation, bridge exclusion,
  min_width_top_surface filter, offset2_ex, inset_idx renumbering, merge, empty
  fallback. SUMMARY ≤200 words." — G3 part 2.
- "Find all callers of `remove_small_lines` and `run_arachne_pipeline`; return
  LOCATIONS." — confirm the signature change's blast radius.
- "Find all readers of `ArachneParams.is_initial_layer`; return LOCATIONS." —
  before subsuming it.
- "Run `cargo xtask build-guests --check`; FACT clean/STALE." — after WIT edits.
- "Run each gap/packet test; FACT pass/fail or SNIPPETS on fail." — per AC.

## Data and Contract Notes

- WIT boundary: **yes** — `arachne-params` gains two bools; host `bindgen!` and
  every guest `wit_bindgen::generate!` regenerate. Field identity must match
  across `common.wit`, the Rust struct, and the adapter, or instantiation fails.
- IR: `SliceRegionView::top_shell_index` is populated by the host's
  ShellClassification prepass built-in (`slice_postprocess_prepass.rs:86`); the
  module reads it (new read).
- Determinism: threshold + wall-count logic is pure per layer; no scheduler
  impact.

## Locked Assumptions and Invariants

- `arachne-params` WIT record, `ArachneParams` struct, and the adapter stay
  field-identical (1:1) — no catch-all adapter arm.
- The `only_one_wall_top_vs_min_width_top_surface` lock requires the module
  source to keep the string `only_one_wall_top` — the real implementation does
  (it now consumes the key), so the lock stays satisfied.
- G10's assertion `!surviving.is_empty()` is invariant; only the call's argument
  list adapts to the new signature (see requirements §Step Completion).
- Topmost detection keys on `top_shell_index == Some(0)` (the IR's expression of
  Orca's `upper_slices == nullptr`), consistent with the G3 red test's fixture.

## Risks and Tradeoffs

- **WIT change + guest rebuild** is the highest-churn item; isolated to this
  packet on purpose. A stale guest masks BOTH gaps — the `--check` gate is
  mandatory before trusting any module test.
- **G3 part 2 is the largest algorithm port** (uncovered by the red test) — the
  packet-authored `only_one_wall_top_second_pass` test is the only guard; it must
  assert both the single top wall AND the `inset_idx += 1` renumbering, or part 2
  could regress silently.
- **Subsuming `is_initial_layer`** risks a hidden consumer — the LOCATIONS
  dispatch must run before removing it; if consumers exist, keep it alongside the
  new flags.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` — G3 part 2 (second-pass port), kept at M by
  delegating the Orca algorithm SUMMARY and all cargo runs.
- Highest-risk dispatch: the `PerimeterGenerator.cpp:2160-2246` SUMMARY — must be
  ≤200 words / one SNIPPET, never the 86-line region loaded raw.

## Open Questions

- `[FWD]` Does the G3 second pass derive the top area from upper slices (Orca) or
  can it reuse PnP's already-computed `top_solid_fill` on `SliceRegionView`? The
  latter is simpler and avoids re-deriving. Resolution criteria (per ADR-0035's
  recording bar): reuse `top_solid_fill` ONLY if a dispatched comparison confirms
  it matches Orca's `diff_ex(infill_contour, upper_slices_clipped)` semantics for
  the covered cases; any residual divergence must be recorded as a DEVIATION_LOG
  entry AND flagged for an amending/new ADR (ADR-0035 requires algorithm
  divergences be recorded at ADR level, not merely logged). Resolve during the
  G3-part-2 step; does not block activation.
- `[FWD]` Whether `is_initial_layer` is subsumed by `is_bottom_layer` or kept
  distinct — decided by the LOCATIONS dispatch on its consumers.
