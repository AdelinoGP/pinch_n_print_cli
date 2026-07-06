---
status: draft
packet: 120-arachne-postprocess-order-and-remove-small-simplify
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 120-arachne-postprocess-order-and-remove-small-simplify

## Goal

Fix the `WallToolPaths::generate` post-processing order (N11 — `stitch → removeSmallLines → separateOutInnerContour → simplify → removeEmpty`), port per-line `min_width` in `remove_small_lines` (N12 — minimum junction width over the line; divisor `min_width/2` on top/bottom layers), and port the simplify distance gates (N13 — `smallest_line_segment_squared` / `allowed_error_distance_squared` with `calculateExtrusionAreaDeviationError` as an extra guard on the near-colinear fast path only).

## Scope Boundaries

Reorder the post-processing pipeline in `arachne/pipeline.rs:360-375`, add `separateOutInnerContour` + `removeEmptyToolPaths`, rewrite `remove_small_lines` to use per-line `min_width`, and rewrite `simplify_toolpaths` to use distance gates + the area guard on the near-colinear fast path. Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `119-arachne-local-maxima-and-construction-epilogue` (D — E's `removeSmallLines` interacts with D's `is_odd = true` micro-loops; D must land first so the `is_odd` semantics are canonical).
- Unblocks: `121-arachne-cross-cutting-closure` (F — F's e2e closure gate depends on E's post-process order being canonical).
- Activation blockers: none.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** `run_arachne_pipeline`'s post-processing pipeline (`pipeline.rs:360-375`), **when** the pipeline runs, **then** the stage order is `stitch → remove_small_lines → separate_out_inner_contour → simplify_toolpaths → remove_empty_toolpaths` (matching `WallToolPaths.cpp:679-699`), NOT PNP's current `stitch → simplify → remove_small`. `separateOutInnerContour` (inner-surface bookkeeping for infill boundary) and `removeEmptyToolPaths` are present.
  | `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order --nocapture 2>&1 | tee target/test-output-e-ac1.log`
- **AC-2. Given** a line whose minimum junction width is `min_w` (much smaller than the nominal width), **when** `remove_small_lines` runs, **then** the line's removal threshold is `min_w` (per-line, not the caller-supplied constant) — on top/bottom layers the divisor is `min_w/2`, otherwise `min_w * min_length_factor` (matching `WallToolPaths.cpp:838-856`).
  | `cargo test -p slicer-core --features host-algos --test arachne_remove_small_per_line_min_width --nocapture 2>&1 | tee target/test-output-e-ac2.log`
- **AC-3. Given** a long low-curvature arc that PNP's iterative area-only sweep would consume, **when** `simplify_toolpaths` runs, **then** the arc survives because the distance gates (`smallest_line_segment_squared` / `allowed_error_distance_squared` from `meshfix_maximum_resolution`/`_deviation`) reject the collapse, with `calculateExtrusionAreaDeviationError` as an extra guard on the near-colinear fast path only (matching `ExtrusionLine.cpp:56-243`).
  | `cargo test -p slicer-core --features host-algos --test arachne_simplify_distance_gates --nocapture 2>&1 | tee target/test-output-e-ac3.log`

## Negative Test Cases

- **AC-N1. Given** the post-process order is canonical, **when** `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast` runs, **then** N1 red tests stay GREEN — the order swap + per-line `min_width` don't regress A1's junction placement.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-e-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order --test arachne_remove_small_per_line_min_width --test arachne_simplify_distance_gates --no-fail-fast 2>&1 | tee target/test-output-e-gate.log`

## Authoritative Docs

- `docs/15_config_keys_reference.md` — `min_length_factor` (0.5), `meshfix_maximum_resolution`/`_deviation` (for the simplify distance gates). Read directly.
- `docs/DEVIATION_LOG.md` `D-112-SIMPLIFY-DP` + `D-116B-CONNECTJUNCTIONS-EMISSION` entries — read full; substrate + A2's addendum.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-120-POSTPROCESS-ORDER` documenting the N11+N12+N13 fix, with an addendum on `D-112-SIMPLIFY-DP` noting E supersedes the iterative area-only sweep with the canonical distance-gated single pass. Supersession pattern.
  - `rg -q 'D-120-POSTPROCESS-ORDER' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:679-699` — canonical post-process order (`stitch → removeSmallLines → separateOutInnerContour → simplifyToolPaths → removeEmptyToolPaths`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — `removeSmallLines` per-line `min_width` + layer-type divisor (`min_width/2` top/bottom, `min_width * min_length_factor` otherwise).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/ExtrusionLine.cpp:56-243` — `simplifyToolpaths` distance gates (`smallest_line_segment_squared` / `allowed_error_distance_squared`) + `calculateExtrusionAreaDeviationError` as extra guard on near-colinear fast path only.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:868-872` — `meshfix_maximum_resolution`/`_deviation` sourcing for the distance gates.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.