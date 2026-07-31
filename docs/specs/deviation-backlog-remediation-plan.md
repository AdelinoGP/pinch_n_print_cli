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

Environment note: **OrcaSlicer canonical is NOT vendored in this repo, and no path to it may be
assumed.** An earlier revision of this note claimed the gitignored `OrcaSlicerDocumented/` at the
repo root was canonical and instructed every packet to verify against it. That instruction was
wrong and actively dangerous: per `CLAUDE.md`, an `OrcaSlicerDocumented/` tree may have been reduced
to serve as a Pinch 'n Print GUI, with **whole functions missing**. A parity check against a reduced
tree silently reads a truncated file and returns a confidently wrong answer, and `.gitignore` means
nothing in the repo warns you.

Every packet MUST verify its port against a **full** OrcaSlicer checkout:

- The checkout's location is **machine-local and deliberately not recorded here.** Ask the
  maintainer where it lives; do not guess, and do not fall back to whatever
  `OrcaSlicerDocumented/` happens to be on disk.
- **Sanity-check fullness before trusting any result.** Compare against a known-full reference:
  `src/libslic3r/PrintObject.cpp` is ~4,880 lines with 9 occurrences of
  `discover_horizontal_shells`; a reduced tree measures ~3,240 lines and 6. Confirm the specific
  function you need is present and complete, not merely that the file exists.
- If no full checkout is available, record the canonical claim as `[BLOCK]` in the packet's
  `design.md` and keep the packet `draft`. **Never ground a parity claim in an unverified tree.**

Files a packet in this plan will typically need: `PerimeterGenerator.cpp`,
`SkeletalTrapezoidation.cpp`, `VariableWidth.cpp`, `FillConcentric.cpp`, `PrintConfig.cpp`,
`WallToolPaths.cpp`, `Flow.cpp`, `FillBase.cpp`.

Cite canonical by **function name**, never line number — line numbers are pinned to whatever
upstream revision their author had open and are unverifiable for anyone else.

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

**Preflight verdicts are deliberately not stored in this table.** A gate verdict is a ledger fact: it is true only of
the packet revision that was gated, and it rots the moment the packet is edited. This column previously carried a
frozen `**PREFLIGHT PASS**` on every row — including rows whose packets had never been gated, and including one row
whose packet fails `S0` outright. Re-derive a verdict by running `/spec-review --preflight <packet>` at the moment you
need it; never read one out of this table.

| # | Packet dir | Deviations | Tranche | Depends on | Status |
|---|---|---|---|---|---|
| 1 | `.ralph/specs/181-dispatch-missing-component-handling` | DEV-087 | T1 | — | generated · draft · TASK-297 |
| 2 | `.ralph/specs/182-gcode-header-width-defaults` | D-165 | T1 | — | generated · draft · TASK-295 |
| 3 | `.ralph/specs/183-arachne-voronoi-panic-diagnosis` | D-167 (diagnosis spike), DEV-098 remediation | T1 | — | generated · active · TASK-296 (reopened) |
| 4 | `.ralph/specs/184-classic-perimeter-flow-parity` | D-164-classic, D-105-classic, D-152-classic | T2 | — | generated · draft · TASK-303 |
| 5 | `.ralph/specs/185-arachne-width-bridge-parity` | D-164-arachne, D-168, D-163 | T2 | — | generated · draft · TASK-304 |
| 6 | `<tbd>-arachne-discretize-point-point` | D-154 | T3 | #3 (D-167 verdict gates design) | **pending — dependency-blocked** (see amendment 2026-07-25a) |
| 7 | `<tbd>-concentric-infill-arachne` | D-104f | T3 | #5, #6 | **pending — dependency-blocked** (see amendment 2026-07-25a) |
| 8a | `.ralph/specs/186-custom-gcode-placeholder-engine` | DEV-085 (engine half) | T3 | — | generated · draft · TASK-305 |
| 8b | `.ralph/specs/187-custom-gcode-injection-registry` | DEV-085 (layer-scoped points) | T3 | #8a | generated · draft · TASK-306 |
| 8c | `.ralph/specs/188-custom-gcode-conditional-points` | DEV-085 (tool/role-scoped points + residuals) | T3 | #8b | generated · draft · TASK-307 |
| 9a | `.ralph/specs/189-per-point-speed-factor-carrier` | DEV-009 (carrier prerequisite) | T3 | — | generated · draft · TASK-308 |
| 9a2 | `.ralph/specs/193-overhang-distance-prepass-carrier` | DEV-009 (overhang-distance carrier prerequisite) | T3 | — | generated · draft (see amendment 2026-07-25d) |
| 9b | `.ralph/specs/190-smoothed-overhang-speed` | DEV-009 (smoothed-speed half) | T3 | #9a, #9a2 | generated · draft · TASK-309 · **maintainer ruled option (C)** (see amendment 2026-07-25d) |
| 9c | `.ralph/specs/191-overhang-add-intersections` | DEV-009 (ADD_INTERSECTIONS half) | T3 | #9a2, #9b | generated · draft · TASK-310 (not schedulable until #9b resolves) |
| 10 | `.ralph/specs/192-infill-linker-anchor-length` | DEV-089 | T3 | — | generated · draft · TASK-311 |

**Queue amendment (2026-07-25a): rows 6 and 7 are dependency-blocked, not merely ordered.** Row 6 (D-154) depends on row 3, and row 3's packet
`.ralph/specs/183-arachne-voronoi-panic-diagnosis` was `status: draft` — verified at the time of this amendment, along with 181, 182, 184 and 185,
all of which were also `draft`. Packet 183 has since been implemented and is now **reopened** for the DEV-098 workspace-test remediation while
retaining its D-167 verdict. Row 6 remains dependent on packet 183's recorded verdict; the remediation does not reopen that dependency or change
the D-154 design gate. Row 7 (D-104f) depends on both row 5 and row 6, so it inherits the block. **Both rows stay `pending`.**

**Queue amendment (2026-07-25b): rows 8 and 9 were each decomposed into three packets. Neither may ship at aggregate `L`.**
The plan rates both `L`, and the Batch Protocol forbids shipping a packet at aggregate `L`; the remedy is decomposition, not scope compression.
Grounding then moved the seams away from where this plan predicted them:

- **Row 8 (DEV-085) → 8a/8b/8c.** The row's own headline counts are **wrong and must not be quoted**: canonical `PrintConfigDef::init_fff_params`
  registers **16** custom-G-code injection points (13 `coString` + 3 `coStrings`, the latter per-filament vectors resolved via `get_at(filament_id)`),
  not 15; the row's enumerated unimplemented list contains 14 names; and its claim that the extrusion-role family appears in
  `docs/ORCA_CONFIG_REFERENCE.md` is false (zero occurrences of `filament_change_extrusion_role_gcode` / `process_change_extrusion_role_gcode`).
  The split seam is the one the code actually has: **8a** fixes the substitution engine itself (the `bytes[i] as char` mojibake, the unknown-placeholder
  policy, and the placeholder *value* keys `docs/15` advertised but the manifest never declared) before any new injection point is added; **8b** builds the
  injection-point registry and lands the layer-scoped points; **8c** lands the tool- and role-scoped points. `time_lapse_gcode` moved 8c→8b (it is
  layer-scoped and shares 8b's `;LAYER_CHANGE` walk). `file_start_gcode` moved 8b→8c **as a recorded residual, not an implementation**:
  `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) writes `serialize_header_block` before it iterates
  `gcode_ir.commands`, so nothing a `PostPass::GCodePostProcess` module emits can precede the header. Four further points
  (`wrapping_detection_gcode`, `machine_pause_gcode`, `template_custom_gcode`, `printing_by_object_gcode`) are likewise recorded as residuals with
  measured unreachability evidence rather than faked. **Note that canonical has no injection-point abstraction to mirror** — the same block is
  hand-inlined 20+ times across `GCode::_do_export`, `GCode::process_layer`, `GCode::set_extruder` and `GCode::_extrude`, and its one table
  (`s_CustomGcodeSpecificPlaceholders`) is validation-only and already drifted. The registry is an improvement over canonical, not parity with it.
- **Row 9 (DEV-009) → 9a/9b/9c.** This plan describes DEV-009 as "two features in `crates/slicer-gcode/src/emit.rs` (`resolve_feedrate`)" that are
  independent. **All three parts of that are false.** `resolve_feedrate` (on `DefaultGCodeEmitter`) contains no quantized lookup — it is a flat per-role
  table times a clamped `speed_factor`. The quantization lives in `modules/core-modules/overhang-classifier-default/src/lib.rs`
  (`quartile_for_distance`, `BAND_BOUNDARY_MULTIPLIERS`). And PnP carries **one `speed_factor` per entity**, not per point — the classifier takes a
  whole-entity `max()` of overhang quartiles — whereas canonical's `calculate_speed` (in `ExtrusionQualityEstimator::estimate_extrusion_quality`)
  interpolates per point. So a per-point speed carrier is a **prerequisite this plan never identified**, and it becomes **9a**. The two features are
  also coupled, one-directionally: canonical feeds `ADD_INTERSECTIONS`' inserted vertices into `calculate_speed`, so ordering is fixed —
  carrier (9a) → smoothed interpolation (9b) → mid-segment insertion (9c). The six-band schedule remains an accepted permanent deviation and is out
  of scope in all three; note the distinction that makes this coherent — the accepted deviation is `annotate_overhangs`' four *quartile bands*, while
  canonical's six *overlap levels* `{90, 75, 50, 25, 13, 0}` build a different table (`speed_sections`), which is what 9b ports.

**Queue amendment (2026-07-25c): row 9b is BLOCKED pending a maintainer decision — it is not an authoring defect.**
*(Superseded by amendment 2026-07-25d below, which records the ruling. Retained for the reasoning.)*
`190-smoothed-overhang-speed` cleared most mechanical axes — though **not** `S0`: it has no `task-map.md`, so it never
"reached PREFLIGHT PASS on every mechanical axis" as an earlier revision of this line claimed. Porting canonical's smoothed
overhang-speed interpolation **reverses three recorded decisions**, so the packet records four `[BLOCK]` items in its
`design.md` §Open Questions and refuses to activate until they are answered. The three reversed decisions, each verified
verbatim against the artifact:

- **ADR-0032** (Accepted, unsuperseded) requires curl distance to bucket through the *same* `BAND_BOUNDARY_MULTIPLIERS`
  and merge via `max(overhang_quartile, curl_quartile)`, and states "**No new config keys.** … If that independent
  control is ever needed, it is **new scope**." The port deletes the merge and adds `slowdown_for_curled_perimeters`.
- **ADR-0031** (Accepted, unsuperseded) shrinks `overhang-classifier-default` to "a pure finalization-tier consumer",
  drops the cross-layer wall-distance code as "redundant", and notes that walls are "merely an inset-by-`line_width/2`
  proxy". The port reintroduces cross-layer wall-distance scanning and removes `EntityMutation::SetSpeedFactor`.
  (ADR-0031's own in-body Amendment preserves the contested clauses, so the conflict stands; ADR-0008 is cross-referenced
  by ADR-0031 as still standing on the same decision and is **not** examined by the packet.)
- **`crates/slicer-core/src/algos/overhang_annotation.rs`**'s accepted deviation records PnP's **4** bands at pre-pass
  time against OrcaSlicer's **6** overlap levels `{90, 75, 50, 25, 13, 0}` at emission time. The port restores both halves.

The options, **in the packet's own lettering — an earlier revision of this line swapped (A) and (B) against
`190/design.md`, which would have made a recorded ruling ambiguous**: **(A)** conform, dropping the smoothed-speed half
of DEV-009; **(B)** supersede the two ADRs with amendment rows; **(C)** add a continuous `overhang_distance_mm` beside `overhang_quartile` on
`Point3WithWidth`, stamped by the same prepass — which *minimises* the supersession rather than avoiding it (AC-6 removes
`SetSpeedFactor` under every option), at the cost of a further packet ahead of 9b whose struct-literal blast radius is
`L` and must be split. **Row 9c (191) inherits the block** and additionally departs from ADR-0031 in a direction 9b never
raises — it makes the module a *geometry* mutator rewriting `path.points` — which the maintainer's ruling should be
understood to cover.

Grounding settled the one question that could have dissolved the fork: a `PostPass::LayerFinalization` guest **cannot**
reach `SliceRegionView`. `world-finalization.wit` does not import the package declaring it, `layer-collection-view`
exposes six methods, and `host-services` exposes fifteen — none region-, surface- or quartile-related. The module must
therefore derive overhang distance in-module against the previous layer's `OuterWall` centreline, which is a
half-line-width proxy for canonical's slice boundary. That bias is filed as its own deviation row rather than hidden.

**Queue amendment (2026-07-25d): the maintainer ruled option (C). Row 9b is unblocked; new row 9a2 is its prerequisite.**
The ruling was taken after a `--preflight` gate run over rows 1–10 verified 9b's characterisation of the conflict rather
than taking it on trust. Three things that verification changed:

- **9b's quotations of ADR-0031 and ADR-0032 are verbatim and clean**, its `[BLOCK-3]` WIT-reachability measurement is
  exactly right (world-finalization imports no `slicer:ir-handles`; `layer-collection-view` has six methods;
  `host-services` has fifteen functions, of which **five** — not four, as 9b's prose says — are `*-batch` forms), and
  ADR-0031's in-body amendment does preserve the contested clauses. The fork was real and correctly described.
- **9b's own cost estimate for option (C) was inflated against itself.** Its `design.md` claimed the previously frozen
  figure understated the sweep by "roughly 15-20 %". Re-derived at ruling time, the two figures were materially the same.
  Do not quote either one: re-derive with `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask` and sum.
- **Option (C) is not ADR-conforming and was not chosen as if it were.** `AC-6` removes `EntityMutation::SetSpeedFactor`
  under every option, and ADR-0008's finalization-tier speed-factor decision is implicated under (C) exactly as under (B).
  (C) narrows the supersession; it does not avoid it.

Consequences: a new prerequisite packet (row 9a2) adds a continuous `overhang_distance_mm` beside `overhang_quartile` on
`Point3WithWidth`, stamped by the prepass that already stamps the quartile, with the matching `point3-with-width` WIT
record field and the exhaustive struct-literal sweep split into `M` steps. **The carrier's signedness and
`+ boundary_offset` normalisation are defined once, in row 9a2, and referenced by 9b and 9c** — rows 9b and 9c previously
disagreed on this (9b specified an *unsigned* point-to-segment minimum while every predicate 9c ports reads canonical's
*signed* distance plus `boundary_offset`, which would have made 9c's proximity test half-unreachable and degenerated its
crossing test). `ADR-0053` records the ruling and amends ADR-0031, ADR-0032 and ADR-0008; it is written to cover **9c's
geometry mutation as well as 9b's interpolation**, because an ADR scoped to 9b alone would not reach it.

**Packet numbers and TASK IDs were allocated centrally by the batch orchestrator, not re-derived per packet.** With authors running in parallel,
per-author re-derivation is precisely the collision that made packet 181's first allocation clash with 178's `TASK-294`. Allocation: 186→TASK-305,
187→TASK-306, 188→TASK-307, 189→TASK-308, 190→TASK-309, 191→TASK-310, 192→TASK-311. `DEV-###` IDs are **not** allocated here — every packet
re-derives its own at the moment of writing. Each packet carries an explicit "register TASK-### in `docs/07_implementation_status.md`" Doc Impact
obligation with a verification grep; packets 181–183 allocated IDs without one, which reads as fabrication at preflight.

**Queue amendment (2026-07-24, post-generation):** row 10 was appended after this plan was written. DEV-089 was registered later the same day, from the `infill-linker` containment work (ADR-0025's 2026-07-24 amendment): canonical's per-arc anchor-length rule — whole arc under `anchor_length_max`, otherwise an `anchor_length` stub off each end via `take_limited`, candidates consumed shortest-first — is not ported, and PnP applies a single 10 × spacing gate with no stub mode. T3 with no dependency: it is a quality divergence rather than a containment one, since the connectors are contour geometry either way, and it needs new config keys plus the lerped partial segment.

**Queue amendment (2026-07-24, at generation time):** row 3 was authored as a D-167 **diagnosis spike only**. D-154 was split out to new row 6 because grounding confirmed `is_secondary` does **not** exist on `HalfEdge` (`crates/slicer-core/src/voronoi.rs`) and must be added and populated from boostvoronoi — a struct-field change whose blast radius, bundled with the spike, would have made row 3 context-cost `L`. This matches the plan's own tranche text, which already placed the spike in T1 and the discretize port in T3.

Non-packet cleanup (log hygiene, do separately): DEV-070 doc-close, DEV-026 defer-annotate,
D-110 flip-to-Closed.
