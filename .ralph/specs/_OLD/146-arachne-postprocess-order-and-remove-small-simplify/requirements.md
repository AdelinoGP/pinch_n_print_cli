# Requirements: 146-arachne-postprocess-order-and-remove-small-simplify

## Packet Metadata

- Grouped task IDs: **none** (provenanced by the second-pass Arachne parity
  audit `target/arachne_parity_audit_20260706_020657.md` findings N11, N12, N13;
  no `docs/07` `TASK-###` exists for N1–N13).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `S`

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

## In Scope

- **Post-process order swap** in `crates/slicer-core/src/arachne/pipeline.rs:350-360`:
  change `stitch → simplify → remove_small` to
  `stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`.
  Add `separate_out_inner_contour` (inner-surface bookkeeping for infill
  boundary — NEW function in `arachne/separate_inner_contour.rs` or inline in
  `pipeline.rs`) and `remove_empty_toolpaths` (filter out empty `ExtrusionLine`s
  after simplify).
- **Per-line `min_width` in `remove_small_lines`** in
  `crates/slicer-core/src/arachne/remove_small.rs:40-50`: `min_width` = minimum
  junction width over the line (not the caller-supplied constant). Divisor
  `min_width/2` on top/bottom layers, `min_width * min_length_factor` otherwise.
  The layer-type branch needs `is_initial_layer` (already on `ArachneParams`).
- **Simplify distance gates** in `crates/slicer-core/src/arachne/simplify.rs:43-121`:
  replace the iterative multi-pass area-only sweep with a single linear pass
  gated by `smallest_line_segment_squared` / `allowed_error_distance_squared`
  (from `meshfix_maximum_resolution`/`_deviation`). `calculateExtrusionAreaDeviationError`
  becomes an *extra* guard on the near-colinear fast path only, not the primary
  gate. The distance gates need new config keys (`meshfix_maximum_resolution`/
  `_deviation`) threaded through `ArachneParams` (or reuse existing keys if
  registered — the implementer confirms via `docs/15_config_keys_reference.md`).
- **New tests**: `arachne_postprocess_order.rs` (AC-1), `arachne_remove_small_per_line_min_width.rs`
  (AC-2), `arachne_simplify_distance_gates.rs` (AC-3).
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/stitch_*.json`,
  `remove_small_*.json` (if they exist), `simplify_*.json` (if they exist) —
  re-record via self-capture (E changes all three post-process stages). The
  `perimeter_parity/*` fixtures (slicer-runtime) are Packet F's scope.
- **Deviation-log entry**: `D-146-POSTPROCESS-ORDER` (new ID, addendum on
  `D-112-SIMPLIFY-DP`, supersession pattern — E supersedes the iterative
  area-only sweep).

## Out of Scope

- **N1–N10** — Packets A1, A2, B, C, D. E reads their output but does not change
  them.
- **`cube_4color.3mf` e2e closure gate** — record-only across E; Packet F blocks.
- **`cargo test --workspace`** — only at Packet F's closure ceremony.
- **New WIT/IR schema changes** — E's surface is `slicer-core`-internal; no
  WIT/IR change. E may add config keys (`meshfix_maximum_resolution`/
  `_deviation`) to `ArachneParams` if not already registered — that's a
  manifest/config-key addition, not a WIT schema change (the `arachne-params`
  WIT record already carries the params; adding fields is a WIT record change
  that E must surface, not silently absorb — check `crates/slicer-schema/wit/`
  for the `arachne-params` record).
- **`OrcaSlicerDocumented/` C++ oracle build** — declined.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — `min_length_factor` (0.5),
  `meshfix_maximum_resolution`/`_deviation` (for the simplify distance gates —
  the implementer confirms whether these are already registered or need adding).
- `docs/DEVIATION_LOG.md` `D-112-SIMPLIFY-DP` + `D-142-CONNECTJUNCTIONS-EMISSION`
  entries — read full; substrate + A2's addendum.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — E
  carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:679-699` — canonical post-process order.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — `removeSmallLines` per-line `min_width` + layer-type divisor.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/ExtrusionLine.cpp:56-243` — `simplifyToolpaths` distance gates + `calculateExtrusionAreaDeviationError` as extra guard.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:868-872` — `meshfix_maximum_resolution`/`_deviation` sourcing.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (canonical post-process order), `AC-2` (per-line
  `min_width`), `AC-3` (simplify distance gates) from `packet.spec.md`.
- Negative cases: `AC-N1` (N1 red tests stay green).
- Cross-packet impact: unblocks `147` (F — F's e2e closure gate depends on E's
  post-process order being canonical).
- Refinements not captured in Given/When/Then:
  - E's `separate_out_inner_contour` is a NEW function (no PNP equivalent);
    inner-surface bookkeeping for infill boundary. The implementer confirms its
    exact responsibility via a delegated SUMMARY of `WallToolPaths.cpp:685`'s
    `separateOutInnerContour`.
  - E's `remove_empty_toolpaths` is a simple filter (remove `ExtrusionLine`s
    with empty `junctions` after simplify).
  - E's per-line `min_width` needs `is_initial_layer` (already on `ArachneParams`)
    for the top/bottom layer divisor.
  - E's simplify distance gates may need new config keys
    (`meshfix_maximum_resolution`/`_deviation`); the implementer confirms via
    `docs/15_config_keys_reference.md` whether they are already registered. If
    not, E adds them to `ArachneParams` + the `arachne-params` WIT record — a
    WIT record change E must surface, not silently absorb.
  - E supersedes `D-112-SIMPLIFY-DP` (113a's DP→VW port) for the simplify layer;
    the iterative area-only sweep is replaced with the canonical distance-gated
    single pass.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order -- --nocapture 2>&1 \| tee target/test-output-e-ac1.log` | AC-1: canonical post-process order | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_remove_small_per_line_min_width -- --nocapture 2>&1 \| tee target/test-output-e-ac2.log` | AC-2: per-line min_width | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_simplify_distance_gates -- --nocapture 2>&1 \| tee target/test-output-e-ac3.log` | AC-3: simplify distance gates | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-e-stays-green.log` | N1/N2/N4/N3 stay green (E doesn't regress A1/A2/B) | FACT pass (expected) |
| `cargo test -p slicer-core --features host-algos --test stitch --test simplify --test remove_small 2>&1 \| tee target/test-output-e-regression.log` | post-process regression (fixtures re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/e-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-e-e2e.log` | e2e closure delta (record-only per cross-cutting policy; E records the failure count in its commit msg, does NOT block on green) | FACT pass/fail + summary line (record-only) |
| `rg -q 'D-146-POSTPROCESS-ORDER' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (E's surface is `slicer-core`-internal; no guest feed expected, but the gate must run clean before blaming any guest-test failure) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **E must keep N1, N2, N3, N4 red tests GREEN.** E changes only the
  post-processing pipeline; regressing A1/A2/B's junction/geometry output means
  backing out.
- **E's `separate_out_inner_contour` is a NEW function** (no PNP equivalent);
  the implementer confirms its exact responsibility via a delegated SUMMARY.
- **E's per-line `min_width` needs `is_initial_layer`** (already on
  `ArachneParams`) for the top/bottom layer divisor.
- **E's simplify distance gates may need new config keys.** The implementer
  confirms via `docs/15_config_keys_reference.md` whether
  `meshfix_maximum_resolution`/`_deviation` are already registered. If not, E
  adds them to `ArachneParams` + the `arachne-params` WIT record — a WIT record
  change E must surface in its commit message, not silently absorb.
- **E supersedes `D-112-SIMPLIFY-DP`** (113a's DP→VW port) for the simplify
  layer; the iterative area-only sweep is replaced with the canonical
  distance-gated single pass.
- **Fixture re-baseline is atomic per fixture and records rationale.**
- **Deviation-log correction uses the supersession pattern** — new
  `D-146-POSTPROCESS-ORDER` + addendum on `D-112-SIMPLIFY-DP`.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/arachne/pipeline.rs:350-360` is the primary edit
  target for the order swap — range-read this block only (the rest of
  `pipeline.rs` is A1/A2/B/C's scope).
- `crates/slicer-core/src/arachne/remove_small.rs` (~57 LOC per the audit) is
  the primary edit target for N12 — full-read (small file).
- `crates/slicer-core/src/arachne/simplify.rs:43-121` is the primary edit target
  for N13 — range-read the multi-pass sweep + the area-gate; the file may be
  larger, range-read only the relevant block.
- `crates/slicer-core/src/arachne/stitch.rs` — read-only for E (the stitch stage
  is unchanged; E only reorders it in the pipeline).
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (delegate),
  `modules/core-modules/arachne-perimeters/` (E's surface is `slicer-core`-
  internal), `slicer-sdk`/`slicer-wasm-host` (no WIT change unless the
  `arachne-params` record needs new fields for the distance gates).
- Sub-agent return-format hints for the heaviest dispatches: the
  `removeSmallLines` SUMMARY (`WallToolPaths.cpp:838-856`) should request the
  per-line `min_width` computation + the layer-type divisor explicitly. The
  `simplifyToolpaths` SUMMARY (`ExtrusionLine.cpp:56-243`) should request the
  distance-gate thresholds + the near-colinear fast-path guard explicitly.