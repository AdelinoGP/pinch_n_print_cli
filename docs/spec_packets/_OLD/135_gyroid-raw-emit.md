---
status: implemented
packet: 135_gyroid-raw-emit
task_ids: [TASK-260]
---

# 135_gyroid-raw-emit

## Goal

Bring `gyroid-infill` to raw-emit parity: fix the rotation order (rotate the polygon first, per FillGyroid.cpp:300-376), delete the broken per-vertex clipping, add `align_to_grid` phase coherence and the 10× expand factor, and make the module multi-role by adding the three solid claims to its manifest (ADR-0027 / gyroid multi-role deviation).

## Problem Statement

Gyroid emitted through ray-cast per-vertex clipping (misclassifies segments whose boundary crossing falls between sample points), rotated around the bbox center (wrong ordering), expanded only 4× spacing (waves could fall short of the generation region), and its manifest held only `claim:sparse-fill` — so the existing top/bottom/bridge emission code in `emit_polys` was dormant. ADR-0027 made multi-role a real opt-in: the manifest gains the three solid claims; defaults stay rectilinear for solid roles (matching OrcaSlicer's sparse-only gyroid).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm). Rotation-order fix per FillGyroid.cpp:300-376: rotate ExPolygon by `-(base + correction)`, generate waves in the rotated bbox, rotate points back at emission.
- Raw emit: clipping, short-filtering, and chaining LEAVE the module (the packet-133 linker owns them); the wave-generation core (`gyroid_f`, `make_one_period`, `make_wave`, orientation choice, `DENSITY_ADJUST = 2.44`, `CORRECTION_ANGLE_DEG = -45.0`, `PATTERN_TOLERANCE = 0.2`) is verified correct and stays untouched — byte-identical.
- Manifest claim addition (`claim:top-fill` / `bottom-fill` / `bridge-fill` alongside `claim:sparse-fill`) is opt-in capability, NOT a default change: fill-holder defaults stay `rectilinear-infill` for all four roles; gyroid is not referenced in `resolved_config.rs` defaults.

## Data and Contract Notes

- Deleted: `clip_polyline_to_expolygon`, `point_in_expolygon`, `point_in_polygon` (linker clips), `polygon_bbox_mm` (replaced by the rotated-polygon bbox), the rotation-around-bbox-center code, and the short-segment filter (linker filters). 4 broken clipper helpers deleted.
- Added: `align_to_grid` helper (snaps `bb.min` to a multiple of `2π×scale_factor` — phase coherence across layers); expand 4.0 → 10.0×spacing (`lib.rs:259`).
- Raw waves in world space may extend beyond the source polygon but stay within the expanded generation bbox — the delivered test asserts bbox containment, NOT polygon containment (the old `no_emitted_point_outside_partitioned_polygon` assertion contradicted the raw-wave/no-clipping contract and was replaced).
- Multi-role opt-in guard regression: `default_holders_gyroid_sparse_only` — even with four declared claims, default config does not route top/bottom/bridge to gyroid.
- The gyroid multi-role divergence row is recorded in `docs/DEVIATION_LOG.md` (DEV-115) and realized by this packet; ADR-0027 status moved to Accepted.
- Follow-up (2026-07-19): per-region `infill_density` / `line_width` (packet 131 accessor) wired via `slicer_sdk::config_resolution::resolve_float`; new tests `per_region_density_overrides_module_global` in both gyroid and rectilinear suites, plus `rotated_square_45_per_point_correspondence_within_2mm` (strict per-point counter-proof of the rotation fix).

## Locked Assumptions and Invariants

- Wave core byte-identical (`gyroid_f`, `make_one_period`, `make_wave` unchanged).
- The four-role emission structure stays (ADR-0027 makes it live); `solid_fill_role` mapping stays.
- Rotation inverse-equivalence: 45° output rotated by −45° matches 0° output within 2 units per point (AC-2).
- Default config must NOT route solid roles to gyroid — divergence is opt-in only.

## Risks and Tradeoffs

- Gyroid solid shells are not 100% dense (wavy surface on top/bottom) — user's choice when opting in (ADR-0027 trade-off, documented).
- Raw waves unclipped until the linker lands (degraded-not-failed per ADR-0025; packet 136 AC-N1 pins at e2e).

## Implementation Deviations (recorded at close)

The packet's Doc Impact statement claimed `none` and referenced a pre-existing deviation row; the row was absent from `docs/DEVIATION_LOG.md` and was filed as DEV-115 by the 2026-08-05 doc audit. No other deviations.
