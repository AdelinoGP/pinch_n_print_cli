# Deviation Backlog Remediation — Plan

> Approved batch plan for closing the open, uncovered rows in `docs/DEVIATION_LOG.md`.
> Packets are authored via `/spec-packet-generator` (draft) and executed via `/swarm`.
> This document is the batch home; the `## Packet Queue` at the bottom is the resume ledger.

## Context

`docs/DEVIATION_LOG.md` is the single source of truth for registered divergences from
OrcaSlicer canonical / the architecture docs. Its own rule: a row is **open** unless its
`Status` begins with "Closed." 91 rows → **21 open**. Cross-referencing every open row
against active/draft/implemented spec packets under `.ralph/specs/` shows most open rows
have **no packet that fixes them** — they were filed by now-archived packets or explicitly
marked "file, don't fix." This plan proposes a small set of themed spec packets to close the
genuinely-actionable, uncovered ones, sequenced by dependency and impact.

Scope decisions:
- **"Covered by a spec" = a packet actually _fixes_ it.** Mere mention (e.g. DEV-026/085/087
  flagged out-of-scope by draft packets 162–165) does **not** count as covered.
- **Plan genuine bugs only.** Accepted intentional divergences and executed-decision rows are
  excluded from fix-planning (listed at the end so nothing is silently dropped).
- Package as a **few themed packets**; sequence into **prioritized tranches**.

Environment note: **OrcaSlicer canonical is vendored in-repo** at `OrcaSlicerDocumented/src/libslic3r/`
(`PerimeterGenerator.cpp`, `SkeletalTrapezoidation.cpp`, `VariableWidth.cpp`, `FillConcentric.cpp`,
`PrintConfig.cpp`, `WallToolPaths.cpp`). Every packet verifies its port against this vendored
source — cite by **function name**, never line number.

## Target set

Two rows changed classification once verified against source:

- **DEV-070 → DOC-CLOSE (stale).** The `wall_sequence` field/parse and the config-driven
  `role_group` branch have **already been removed** from `PathOptimizationDefault`
  (`modules/core-modules/path-optimization-default/src/lib.rs`); `role_group` is now a fixed
  role→priority match documented as ADR-0011-owned, pinned by
  `committed_wall_sequence_is_not_reordered_by_role_priority`. Ownership moved to
  `classic-perimeters::emit_walls` (`wall_sequence_reorder`). Action: `git blame` to date the
  removal, then reconcile the log row + doc07 TASK-054 to Closed. **No code fix.**
- **DEV-026 → DEFER (environmental).** `DagValidationPass::HostVersionCompatibility` (gap 1) is
  already implemented in `slicer-scheduler::validation`. The remaining peak-RSS/500-layer half
  needs OS-level RSS sampling of a `pnp_cli slice` subprocess — the `AccountingAllocator`
  structurally cannot see WASM linear memory — and is already deferred under TASK-156. Not a
  parity fix; pursue only as a bench-style workstream.

That leaves **12 actionable deviations**, grouped into 8 packets below.

## Proposed packets

Names below are slugs; the assigned packet numbers are in the Packet Queue.

### P-CLASSIC-FLOW — classic-perimeter flow & width parity  · Cluster A · bundled
One file (`modules/core-modules/classic-perimeters/src/lib.rs`), one classic `perimeter_parity`
re-record. Internal order: D-164-classic → D-105-classic → D-152-classic.
- **D-164 (classic half)** — retype `outer_wall_line_width`/`inner_wall_line_width` in
  `classic-perimeters.toml` to float-or-percent, default `0` (auto-from-nozzle). `FloatOrPercent`/
  `Percent` types already exist (`crates/slicer-schema`). Wire `0 → nozzle_diameter` at the
  `on_print_start` read sites. Canonical: `PrintConfig.cpp` `coFloatOrPercent`. M.
- **D-105-FLOW-NOT-WIRED (classic half, T-052)** — replace width-average `(outer+inner)/2` spacing
  in `inset_polygons` / `emit_gap_fill` with `slicer_core::flow::line_width_to_spacing`
  (`ext_perimeter_spacing2`); propagate its fallible `Result`. Canonical: `PerimeterGenerator::process_classic`. M.
- **D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER** — stop discarding `min_width_top_surface`; gate
  the `only_one_wall_top` single-wall collapse on per-loop width ≥ `min_width_top_surface`.
  Canonical: `PerimeterGenerator.cpp` only_one_wall_top logic. M.

### P-ARACHNE-FLOW — arachne width & bridge parity  · Cluster B · bundled
`modules/core-modules/arachne-perimeters/src/lib.rs` + `crates/slicer-core/src/arachne/pipeline.rs`;
one arachne `arachne_parity` re-record. D-164-arachne and D-168 both edit `arachne_params_from_config`.
- **D-164 (arachne half)** — same float-or-percent retype + auto-resolution at the
  `arachne_params_from_config` read sites. M.
- **D-168-ARACHNE-SIMPLIFY-FALLBACKS** — fix `ArachneParams::default` fallback constants
  (`smallest_line_segment_squared`, `allowed_error_distance_squared`) from 0.05/0.005 mm to canonical
  0.5/0.025 mm (squared: 0.25 / 0.000625). Guard: `manifest_default_reconcile_tdd`. S code / M fixtures.
- **D-163-ARACHNE-BRIDGE-ROLE-CONVERSION-EXEMPTION** — in `build_walls`, skip `flow_to_width` for
  `is_bridge` vertices and substitute bridge flow width wholesale, matching canonical's
  `erOverhangPerimeter && flow.bridge()` exemption. Canonical: `VariableWidth.cpp::thick_polyline_to_multi_path`.
  S + design (first per-vertex role exemption).

### P-GCODE-HEADER — G-code header width truth  · standalone · trivial
- **D-165-GCODE-HEADER-WIDTH-DEFAULTS-LIE** — in `crates/slicer-gcode/src/serialize.rs`, change the
  header-comment defaults from 0.42/0.45 to the governing 0.4/0.4 and delete the stale
  removed-`config_schema.rs` citation. S.

### P-ARACHNE-GEOM — Voronoi/skeletal geometry  · Cluster C · split into two packets
`crates/slicer-core/src/skeletal_trapezoidation/graph.rs` + the `voronoi` path. Grounding split this
into a T1 spike and a T3 port (queue rows 3 and 6); see the queue amendment.

- **D-167-BOOSTVORONOI-ROBUST-FPT-PANICS (diagnosis-first spike, T1)** — grounding found the
  structural cause: of the three boostvoronoi call sites in `slicer-core`, `medial_axis.rs` and
  `algos/paint_segmentation/voronoi_graph.rs` both wrap the builder in
  `catch_unwind(AssertUnwindSafe(...))` (the former's comment names `assertion failed: fpv.is_finite()`
  at `robust_fpt.rs`), but `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) has **no
  guard** — only `map_err(map_bv_error)`. A `robust_fpt` failure is an `assert!` panic, not a
  `Result::Err`, so `map_err`/`?` cannot observe it, and the skeletal path is the one entry point with
  no backstop. The spike adds the missing guard (which is also the instrumentation), captures the
  degenerate inputs, measures the wall-loop delta, and records a verdict: close as inert, or narrow to
  a successor owning `preprocess_input_outline` hardening (ADR-0023 assigns pre-snapping to the
  caller). Gates D-154. S–M.
- **D-154-DISCRETIZE-POINT-POINT-CASE (T3)** — port canonical `SkeletalTrapezoidation::discretize`'s
  3-branch dispatch into `discretize_edge`: its single `!is_curved` early return of `vec![start, end]`
  conflates canonical branch 1 (seg-seg / secondary) with branch 3 (point-point, which canonical still
  subdivides by `discretization_step_size`). `contains_point` and `source_point_of` are already
  available in scope, but **`is_secondary` is confirmed absent** from PnP's `HalfEdge` — it must be
  added and populated from boostvoronoi's `e.is_secondary()` in `voronoi_from_segments`, a struct-field
  change with its own blast radius. M–L; that is why it is not bundled with the spike.

### P-CONCENTRIC — concentric infill through Arachne  · feature · after P-ARACHNE-FLOW + P-ARACHNE-GEOM
- **D-104f-CONCENTRIC-INFILL-NO-ARACHNE** — route concentric infill through `run_arachne_pipeline`/
  `WallToolPaths` in `crates/slicer-runtime/src/run.rs` per canonical `FillConcentric.cpp`. Replace the
  `#[ignore]`d source-string test with a real geometric assertion. L.

### P-HOST-DISPATCH — dispatch MissingComponent handling  · standalone · **resolved to option (B)**
- **DEV-087** — `crates/slicer-wasm-host/src/dispatch.rs` has five (not four — the row is stale)
  `MissingComponent → Ok(success)` arms. The row offered "(A) prove `None` is unreachable for a real
  module, then narrow the laundering to an explicit placeholder marker; (B) if reachable, make it
  fatal." **Grounding selected (B):** `None` *is* reachable for a real module —
  `compile_module_component` returns it with only a `Warning` when `fs::read` or `compile_component`
  fails. Two further findings shaped the packet: `placeholder_wasm` is not a manifest declaration but
  an ≤8-byte file-size heuristic (`is_placeholder_wasm`), and no module in the tree qualifies (smallest
  real core-module `.wasm` is 68,495 bytes), so the skip path has zero users. The packet therefore
  **retires the placeholder-skip capability** rather than conditioning it behind a marker: absent
  component is fatal at load, at the six executor fallbacks, and at all five arms. This avoids widening
  the `wasm_handles` side-table (~21 signatures). It contradicts ADR-0020 §Decision item 1 and so files
  `D-181-ADR-0020-AMENDED`. Refs ADR-0015, ADR-0020, ADR-0045. M.

### P-CUSTOM-GCODE — machine custom-gcode injection points  · large feature · standalone
- **DEV-085** — `modules/core-modules/machine-gcode-emit/src/lib.rs` reads only 2 of 15 injection
  points. Build a real injection-point registry, implement the missing points, harden
  `substitute_placeholders` (unknown-key error vs passthrough; fix the `bytes[i] as char` mojibake).
  Refs `docs/15_config_keys_reference.md`, `docs/ORCA_CONFIG_REFERENCE.md`, packet 59. L.

### P-SPEED — smoothed-speed + ADD_INTERSECTIONS  · large feature · standalone
- **DEV-009** — two features in `crates/slicer-gcode/src/emit.rs` (`resolve_feedrate`): (a) smoothed-speed
  interpolation replacing the flat quantized lookup; (b) `ADD_INTERSECTIONS` mid-segment vertex
  insertion at overhang-quartile band crossings. Six-band schedule stays an accepted permanent
  deviation (out of scope). L.

## Coupling graph

- **Cluster A — `classic-perimeters/src/lib.rs`:** D-164-classic, D-105-classic, D-152-classic edit one
  file, re-baseline the same classic `perimeter_parity` goldens → one packet. D-164 first.
- **Cluster B — `arachne-perimeters/src/lib.rs` + `arachne/pipeline.rs`:** D-164-arachne and D-168 both
  edit `arachne_params_from_config`; D-163 shares file + fixtures → one packet.
- **D-164 splits by generator** across A and B so each generator's fixtures re-record once. D-165 must
  agree on the same 0.4 default (weak coupling).
- **Cluster C:** D-167 diagnosis precedes D-154 (shared graph path). D-104f is downstream of the whole
  arachne pipeline.
- **Independent / parallelizable:** DEV-087, DEV-085, DEV-009.

## Tranches

- **T1 — quick correctness + unblocking diagnosis:** P-HOST-DISPATCH (DEV-087) · P-GCODE-HEADER (D-165) ·
  P-ARACHNE-GEOM step 1 (D-167 spike, gates D-154).
- **T2 — flow/config parity:** P-CLASSIC-FLOW · P-ARACHNE-FLOW.
- **T3 — deeper geometry + big features:** P-ARACHNE-GEOM step 2 (D-154) → P-CONCENTRIC (D-104f);
  P-CUSTOM-GCODE (DEV-085) and P-SPEED (DEV-009) in parallel.

## Excluded (recorded so nothing is silently dropped)

- **Accepted divergences / decision records:** D-109-SELF-CAPTURED-FIXTURES, D-152-TOP-AREA-SOURCE,
  DEV-039, DEV-009's six-band-schedule portion, D-110-DROP-VARIABLE-WIDTH (recommend flipping its
  stale-open row to Closed).
- **Already owned by an implemented packet:** D-173-THUMBNAIL-SINGLE-PNG (packet 173),
  D-283-ADR-0046-AMENDED (packet 180).
- **Reclassified during verification:** DEV-070 (doc-close), DEV-026 (defer).

## Packet Queue

Dependency-ordered. Resume at the first `pending` row whose dependencies are `generated`. Update each
row immediately on generation/closure. **T1 packets (181–183) commit together.**

| # | Packet dir | Deviations | Tranche | Depends on | Status |
|---|---|---|---|---|---|
| 1 | `.ralph/specs/181-dispatch-missing-component-handling` | DEV-087 | T1 | — | generated · draft · TASK-297 · **PREFLIGHT PASS** |
| 2 | `.ralph/specs/182-gcode-header-width-defaults` | D-165 | T1 | — | generated · draft · TASK-295 · **PREFLIGHT PASS** |
| 3 | `.ralph/specs/183-arachne-voronoi-panic-diagnosis` | D-167 (diagnosis spike) | T1 | — | generated · draft · TASK-296 · **PREFLIGHT PASS** |
| 4 | `<tbd>-classic-perimeter-flow-parity` | D-164-classic, D-105-classic, D-152-classic | T2 | — | pending |
| 5 | `<tbd>-arachne-width-bridge-parity` | D-164-arachne, D-168, D-163 | T2 | — | pending |
| 6 | `<tbd>-arachne-discretize-point-point` | D-154 | T3 | #3 (D-167 verdict gates design) | pending |
| 7 | `<tbd>-concentric-infill-arachne` | D-104f | T3 | #5, #6 | pending |
| 8 | `<tbd>-machine-custom-gcode-injection` | DEV-085 | T3 | — | pending |
| 9 | `<tbd>-gcode-smoothed-speed-add-intersections` | DEV-009 | T3 | — | pending |

**Queue amendment (2026-07-24, at generation time):** row 3 was authored as a D-167 **diagnosis spike only**. D-154 was split out to new row 6 because grounding confirmed `is_secondary` does **not** exist on `HalfEdge` (`crates/slicer-core/src/voronoi.rs`) and must be added and populated from boostvoronoi — a struct-field change whose blast radius, bundled with the spike, would have made row 3 context-cost `L`. This matches the plan's own tranche text, which already placed the spike in T1 and the discretize port in T3.

Non-packet cleanup (log hygiene, do separately): DEV-070 doc-close, DEV-026 defer-annotate,
D-110 flip-to-Closed.
