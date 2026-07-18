---
status: implemented
packet: 146-arachne-postprocess-order-and-remove-small-simplify
task_ids:
  - none
---

# 146-arachne-postprocess-order-and-remove-small-simplify

## Goal

Fix the `WallToolPaths::generate` post-processing order (N11 — `stitch → removeSmallLines → separateOutInnerContour → simplify → removeEmpty`), port per-line `min_width` in `remove_small_lines` (N12 — minimum junction width over the line; divisor `min_width/2` on top/bottom layers), and port the simplify distance gates (N13 — `smallest_line_segment_squared` / `allowed_error_distance_squared` with `calculateExtrusionAreaDeviationError` as an extra guard on the near-colinear fast path only).

## Problem Statement

Three minor post-processing divergences (N11 + N12 + N13). **N11
(post-processing order):** PNP's `pipeline.rs:350-360` runs
`stitch → simplify → remove_small`. Canonical (`WallToolPaths.cpp:679-699`) is
`stitch (681) → removeSmallLines (683) → separateOutInnerContour (685) →
simplifyToolPaths (687) → removeEmptyToolPaths (689)`. Order swap means PNP
simplify can shorten a line *below* the removal threshold after the removal
decision would have kept it (and vice versa); `separateOutInnerContour`
(inner-surface bookkeeping for infill boundary) has no PNP equivalent. **N12
(`remove_small_lines` threshold):** PNP's `remove_small.rs:40-50` uses a
caller-supplied constant `min_width`, no layer-type branch. Canonical
(`WallToolPaths.cpp:838-856`): `min_width` = minimum junction width over the
line; divisor `min_width/2` on top/bottom layers, `min_width * min_length_factor`
otherwise. PNP under-removes genuinely thin slivers (whose own min width ≪
nominal) and over-removes wide-but-short odd lines. **N13 (`simplify_toolpaths`
gating):** PNP's `simplify.rs:43-121` is an iterative multi-pass sweep gated
**only** by the width-weighted area deviation. Canonical (`ExtrusionLine.cpp:56-243`)
is a single linear pass gated by `smallest_line_segment_squared` /
`allowed_error_distance_squared` (from `meshfix_maximum_resolution`/`_deviation`,
`WallToolPaths.cpp:868-872`) with `calculateExtrusionAreaDeviationError` as an
*extra* guard on the near-colinear fast path only. PNP's iterative area-only
sweep can consume long low-curvature arcs canonical would keep (distance gates
absent), altering wall smoothness. This packet fixes all three — the order
swap, the per-line `min_width`, and the distance-gated simplify.

This packet supersedes `D-112-SIMPLIFY-DP` for the simplify layer (113a's
DP→VW port was an earlier step; E supersedes the iterative area-only sweep with
the canonical distance-gated single pass). E does not change A1/A2/B/C/D's
junction generation, emission, or graph construction — only the post-processing
pipeline.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **E's simplify distance gates may need new config keys** (`meshfix_maximum_resolution`/`_deviation`). The implementer confirms via `docs/15_config_keys_reference.md` whether they are already registered. If not, E adds them to `ArachneParams` + the `arachne-params` WIT record — a WIT record change E must surface in its commit message, not silently absorb. Check `crates/slicer-schema/wit/` for the `arachne-params` record.
- Packet-specific constraint: **E's `separate_out_inner_contour` is a NEW function** (no PNP equivalent); inner-surface bookkeeping for infill boundary. The implementer confirms its exact responsibility via a delegated SUMMARY of `WallToolPaths.cpp:685`'s `separateOutInnerContour`.
- Packet-specific constraint: **E supersedes `D-112-SIMPLIFY-DP`** (113a's DP→VW port) for the simplify layer; the iterative area-only sweep is replaced with the canonical distance-gated single pass. `calculateExtrusionAreaDeviationError` becomes an *extra* guard on the near-colinear fast path only, not the primary gate.
- Packet-specific constraint: **WASM staleness MAY apply** if E adds fields to the `arachne-params` WIT record (which feeds guest WASM). The implementer MUST run `cargo xtask build-guests --check` after any WIT change. If E does NOT touch WIT (distance gates sourced from existing config keys), WASM staleness does not apply. Include the `wasm-staleness` snippet conditionally.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR or manifest contracts touched: **possibly** the `arachne-params` WIT record (if `meshfix_maximum_resolution`/`_deviation` are not already registered and E must add them). The implementer confirms via `docs/15_config_keys_reference.md` + `crates/slicer-schema/wit/`. If a WIT record change is needed, E surfaces it in the commit message (not silently absorbed) and threads through `slicer-sdk`/`slicer-wasm-host`.
- WIT boundary considerations: **conditional**. If E adds fields to the `arachne-params` WIT record, the host boundary (`slicer-sdk/src/host.rs`, `slicer-wasm-host/src/host.rs`) must thread them. This is a WIT record change, not a schema change — but it must be surfaced.
- Determinism: E's changes preserve determinism (the canonical single-pass simplify is deterministic; the per-line `min_width` is a deterministic per-line computation).

## Locked Assumptions and Invariants

- E's post-process order is canonical: `stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`.
- E's per-line `min_width` = minimum junction width over the line; divisor `min_width/2` on top/bottom layers, `min_width * min_length_factor` otherwise; needs `is_initial_layer` (already on `ArachneParams`).
- E's simplify is a single linear pass gated by `smallest_line_segment_squared` / `allowed_error_distance_squared`; `calculateExtrusionAreaDeviationError` is an extra guard on the near-colinear fast path only.
- E keeps N1, N2, N3, N4 red tests GREEN (gated).
- E supersedes `D-112-SIMPLIFY-DP` for the simplify layer.
- E's `separate_out_inner_contour` is a NEW function (no PNP equivalent).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- If E adds WIT record fields, it surfaces the change (not silently absorbed) + runs `cargo xtask build-guests --check`.

## Risks and Tradeoffs

- **The order swap changes when `remove_small` runs relative to `simplify`.** Canonical runs `remove_small` BEFORE `simplify`; PNP runs it AFTER. This means lines that `simplify` would have shortened below the removal threshold are now removed before `simplify` touches them. The regression suite gates this; the `remove_small`/`simplify` fixtures re-baseline.
- **Per-line `min_width` changes the removal threshold for every line.** Lines with a thin junction (slivers) get a smaller threshold (more likely removed); lines with uniform wide junctions get a larger threshold. The N4 red tests (A2's `is_odd` fix) gate that real walls aren't mis-removed.
- **The simplify distance gates need config keys.** If `meshfix_maximum_resolution`/`_deviation` are not registered, E must add them — a WIT record change. Risk is contained by the `rg` check (dispatch listed).
- **`separate_out_inner_contour`'s exact responsibility is unclear without a delegated SUMMARY.** The audit flags its absence but doesn't detail its bookkeeping. E's implementer confirms via the SUMMARY; if the bookkeeping is non-trivial, E may stub it minimally and flag a follow-up.
