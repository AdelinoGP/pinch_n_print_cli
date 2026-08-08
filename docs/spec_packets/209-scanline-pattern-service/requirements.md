# Requirements: 209-scanline-pattern-service

## Re-scope 2026-08-07 — read this first

This packet was originally authored around a **shared `slicer_core::scanline_fill` kernel**: a new
`crates/slicer-core/src/scanline_fill.rs` exporting `scanline_fill`, `ScanlineFillParams`,
`FillSegment` and a promoted `adjust_solid_spacing`, with both callers rewired onto it.

**That approach is forbidden and has been removed in full.**
`docs/adr/0026-infill-linking-algorithms-in-linker-module.md` records that a
`slicer-core::infill_ops` module bundling exactly these helpers was proposed at the 2026-07-01
infill-parity grilling and **rejected by the project owner**. `slicer_core::scanline_fill` is that
same proposal renamed. ADR-0026 §Consequences states that `slicer-core` gains **only**
`clip_polylines`, because that is generic geometry with no domain logic — and a scan-line *fill*
kernel is fill-pattern logic. Its decisive argument is the multi-language module promise: a C++ or
Zig infill component must not have to link a Rust helper. Its §Future-Reviewer Notes say
"Do not re-suggest `slicer_core::infill_ops`." The user confirmed on 2026-08-07 that this recipe has
already been tried and failed.

Removed from this packet, everywhere (`packet.spec.md`, this file, `design.md`,
`implementation-plan.md`, `task-map.md`): `crates/slicer-core/src/scanline_fill.rs`,
`crates/slicer-core/tests/scanline_fill_tdd.rs`, the symbols `scanline_fill`, `ScanlineFillParams`,
`FillSegment`, `ScanIntersection`, `IntersectionKind`, the promotion of `adjust_solid_spacing` out of
`rectilinear-infill`, the twelve-divergence enumeration that only existed to justify one kernel, the
canonical classification/dedup pass as a required implementation, and the old AC-1..AC-15 /
AC-N1..AC-N4 set.

Also removed: **all ADR-0009 amendment work.** With no extraction there is nothing to amend, so the
user-approved 2026-08-07 override of ADR-0009's Future-Reviewer Note ("Do not re-suggest extracting
patterns to `slicer_core::patterns`") **lapses unused**. It was granted and is now unnecessary; it is
recorded here so a future reader does not mistake its absence for an oversight or re-litigate it. The
deviation row `D-209-ADR-0009-AMENDED` is deleted, as are the `## Amendment — packet 209` section,
the `### Retired clause` / `### What stands` / `### Replacement text` subsections, and every
reference to the `D-285-ADR-0051-AMENDED` precedent. `D-209-INCLUSIVE-SCAN-GRID` and
`D-209-IRONING-SCANLINE-COPY` are also deleted — the first because the inclusive grid is now
*corrected* rather than deferred, the second because the ironing copy is now *in scope*.

`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE` is **kept**: it still describes a real behaviour change under
the new scope (scan start moves to the rotated bbox min, the vertex test changes, the centroid
fallback is deleted, four density fixtures are corrected).

**New scope: reconcile in place, no extraction.** Each of the three copies keeps its own
implementation. The packet's job is to make them agree on one canonical behaviour, removing the
correctness divergence while leaving the duplication to a future WIT pattern-services packet.

## Packet Metadata

- Grouped task IDs: `TASK-325`
- Backlog source: `docs/07_implementation_status.md` (Workstream 3 — Benchy parity and missing
  OrcaSlicer behavior). `TASK-325` is assigned by `docs/specs/deviation-remediation-206-212-plan.md`
  and is absent from `docs/07` at authoring time; re-derive the highest live `TASK-###` at the moment
  the row is written rather than trusting this sentence.
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

There are **three** live copies of the same scan-line fill skeleton — flatten edges to
`(i64,i64,i64,i64)`, take a y-bbox, walk `scan_y`, sort x-intersections, pair them, emit
`ExtrusionPath3D`:

- `scan_expolygon` (`modules/core-modules/rectilinear-infill/src/lib.rs`) — rotates by −angle about a
  per-ExPolygon unrotated-bbox centre, applies an `x_shift`, uses `adjust_solid_spacing` for solid
  roles, a half-open vertex test (`scan_y < lo || scan_y >= hi`), an **inclusive** grid
  (`scan_y = rmin_y; while scan_y <= rmax_y`), a sub-spacing bail, a zero-length-span drop, and a
  top-boundary post-pass over `rotated_contour`.
- `TraditionalSupport::fill_expolygon` (`modules/core-modules/traditional-support/src/lib.rs`) —
  rotates about the world origin, uses a **strictly-between** test
  (`scan_y > edge_min_y && scan_y < edge_max_y`), starts at `min_y + line_spacing`, has **no**
  zero-length-span drop, and has a centroid fallback with no canonical analog.
- `SupportSurfaceIroning::fill_expolygon`
  (`modules/core-modules/support-surface-ironing/src/lib.rs`) — a real third copy with its own
  `collect_edges`, the same strictly-between test and the same `min_y + line_spacing` start, no
  zero-length-span drop, no rotation at all (axis-aligned scan) and no centroid fallback. DEV-127's
  row names only the first two. It is **in scope now**: reconciling without it leaves the divergence
  half-fixed.

**The real correctness bug is the strictly-between test.** At a vertex lying exactly on a scan line
with one neighbour above and one below — a true crossing — it drops **both** incident edges' events,
losing a crossing. The row's intersection list then pairs incorrectly and inside/outside inverts for
the remainder of that scan row. Support and ironing both carry it. Canonical
`slice_region_by_vertical_lines` (`FillRectilinear.cpp`) keeps a true crossing **exactly once** (two
raw events, collapsed to one by the post-sort compaction) and drops a tangential touch entirely.

The half-open test in `rectilinear-infill` already reproduces the canonical **crossing** count
exactly (the lower edge's event is included at `lo`, the upper edge's is excluded at `hi`). It
differs from canonical only at a tangential touch, where it yields two coincident events that its
zero-length-span rule annihilates — coverage-identical to canonical, but splitting one span into two
abutting spans when the touch lies inside a span. That residual is filed, not fixed
(`D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`).

Two further axes diverge without either side being canonical. The **rotation reference** (bbox centre
vs world origin) is geometrically inert here, because both anchor the scan grid at the rotated-space
bbox extreme; the difference is that bbox-centre rotation is *exactly* translation-equivariant while
world-origin rotation drifts by up to ~1 unit (100 nm) with plate position through `rotate_point`'s
`round()`. Canonical rotates about the origin but derives its phase from `align_to_grid` against
`_infill_direction`'s object-bbox anchor, which PnP ports in neither copy. The **scan grid** is
worse: `rectilinear-infill` is inclusive (`floor(h/s)+1` lines), support and ironing are exclusive
from `min + s` (`scan_y = min_y + line_spacing; while scan_y < max_y`, which emits `ceil(h/s) - 1`
lines — *not* `floor((h-s)/s)+1`; that expression reduces to `floor(h/s)` and is off by one at exact
multiples, e.g. it predicts 5 lines for the 10 mm / 2 mm fixture case where the code emits 4), and
canonical is half-open from the bbox min (`ceil(h/s)`) — **no copy matches canonical.**

Existing coverage is stale and thin. `traditional-support` and `support-surface-ironing` have **zero**
geometric-invariant tests: nothing covers the vertex test, the scan start, translation invariance or
zero-length spans. `docs/adr/0009-raft-as-layer-infill-role.md` acknowledges the duplication and
defers it, but all three of its pointers have rotted (see §In Scope).

This is one coherent slice because the three copies cannot be made to agree one axis at a time
without leaving intermediate states where two of them disagree with the third.

## In Scope

### 1. The four reconciled axes

The decision table (with canonical evidence and per-copy consequence) lives in `packet.spec.md`
§"Canonical Decisions (the four axes)" and is not restated here. Its four rows are: vertex/edge
inclusion, rotation reference, scan start, scan grid extent. The rationale for each decision is in
`design.md` §"Per-axis rationale".

### 2. Consequential deletions

- `rectilinear-infill`: the top-boundary post-pass over `rotated_contour` (it existed only because the
  half-open test excluded `rmax_y`, and canonical has no line at the bbox max at all), the separate
  `contour_edges` / `rotated_contour` vectors, and the `rmax_y - rmin_y < effective_spacing`
  sub-spacing bail (canonical's `ceil(w/s) >= 1` never bails).
- `traditional-support`: the centroid fallback. It has no canonical analog, and the new bbox-min scan
  line supersedes it — every non-degenerate region now receives at least one real scan line, so the
  fallback is dead code rather than a lost behaviour.
- Neither module's `line_spacing <= 0` guard nor its degenerate-bbox guard may be lost; those are
  different conditions from the sub-spacing bail (AC-N1, AC-N2).

### 3. Fixture re-baselining (canonical corrections, owned)

Two self-captured `rectilinear-infill` fixtures encode the non-canonical inclusive grid and are
re-baselined to canonical-correct output, per `CLAUDE.md` §Test Discipline. The full statement is in
`packet.spec.md` §"Shipped infill geometry moves"; in brief,
`square_10mm_density_20_emits_n_raw_segments` goes from `floor(bb_h/spacing)+1` (6) to
`ceil(bb_h/spacing)` (5), and `very_small_polygon_emits_no_paths_without_panic` is renamed to
`very_small_polygon_emits_one_scan_row_without_panic` and asserts one path instead of zero. **No
assertion may be weakened, narrowed or deleted in either file** — only these two counts change, and
each is justified by a quoted canonical formula.

`rectilinear_raw_emit_tdd.rs`'s `half_open_vertex_test_no_double_count` is renamed to
`vertex_event_test_no_double_count` at **both** occurrences (the module doc-comment index at the top
of the file and the `fn`; re-derive the count with `rg -n`) and **strengthened by addition only**: the
original assertion is retained with its message byte-identical
(`AC-N1: expected 9 segments for triangle with apex on scan line, got {}`), and two assertions are
added whose messages begin `AC-3 crossing vertex:` and `AC-3 tangential touch:` so AC-3 can verify
them by grep. No test function is added or removed, so the binary still reports `7 passed`.

### 4. `adjust_solid_spacing` attribution, corrected in place

The function **stays private in `rectilinear-infill`** — ADR-0026's 2026-08-05 amendment explicitly
places it in the rectilinear emitter, and promoting it is exactly the move ADR-0026 forbids. Its
arithmetic is preserved byte-for-byte (changing it would move solid-infill geometry, an axis this
packet did not decide). Only its doc comment changes: `/// Ported from OrcaSlicer
FillBase.cpp::adjust_solid_spacing` is **false** and is replaced by text naming
`D-209-ADJUST-SOLID-SPACING-DIVERGENCE` and the three divergences from canonical
`Fill::_adjust_solid_spacing` (`FillBase.cpp`), enumerated in AC-5. **Exactly three.** Canonical's
`number_of_intervals == 0 → return distance` guard is **not** a fourth axis: PnP's opening
`let count = width / distance; if count < 1 { return distance; }` is that same guard for `width >= 0`
and both sides return `distance`. Recording a fourth would write a fabricated canonical claim into a
permanent artifact.

### 5. Density fixture correction in `traditional_support_tdd.rs`

All **four** `make_config(true, 0.2, …)` call sites — re-derived with
`rg -n 'make_config\(true, 0\.2,'`: `extrusion_role_is_support_material`, `speed_factor_from_config`,
`alternating_angle` and `empty_regions_no_output` — change the density argument to `20.0`. Per-site
justification and the diff guard are in AC-8. `run_support` reads `support_density` as a **percent**,
so `0.2` gives `line_spacing = 0.4 / 0.002 = 200 mm` against a 10 mm square: after this packet those
fixtures produce exactly one degenerate boundary scan line. Widening the fixture is arithmetically
impossible; the density argument is the only lever. **No assertion in the file may change.**

### 6. New geometric-invariant coverage

- `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs` — **five** `#[test]`
  fns: `scan_starts_at_rotated_bbox_min`, `crossing_vertex_contributes_one_intersection`,
  `fill_phase_is_translation_invariant`, `zero_length_span_is_dropped`,
  `non_positive_spacing_yields_no_paths`.
- `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs` — **three**
  `#[test]` fns: `scan_starts_at_bbox_min`, `crossing_vertex_contributes_one_intersection`,
  `zero_length_span_is_dropped`.

Both files drive their module through its public entry point (`run_support` /
`run_support_postprocess`); `fill_expolygon` is private in both modules, so a test that tries to call
it directly will not compile. Fixture constraints: the crossing vertex must lie on a line the
half-open grid actually visits (`rmin_y + k * line_spacing`), or the test passes for the wrong reason;
`fill_phase_is_translation_invariant` must use a non-zero fill angle and a non-axis-aligned
translation, or invariance holds trivially. AC-7 and AC-10 pin the counts because a newly authored
file with zero `#[test]` fns prints `ok. 0 passed` and exits 0.

### 7. Ledger and docs

- DEV-127 stays **Open**. The duplication is not removed, so it cannot close as "duplication gone",
  and its target close (the WIT pattern-services packet) is unchanged. Its body gains a dated
  reconciliation note, names the third copy, and has `SupportFiller` corrected to
  `TraditionalSupport` at both occurrences — **there is no `SupportFiller` type in this tree**.
- Five new rows: `D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE`, `D-209-IRONING-FILL-BEHAVIOUR-CHANGE`,
  `D-209-HALF-OPEN-SCAN-GRID-ADOPTED`, `D-209-ADJUST-SOLID-SPACING-DIVERGENCE`,
  `D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`. Contents are specified in AC-14. The `D-` counter is a ledger
  fact — re-derive at the moment of writing.
- `docs/adr/0009-raft-as-layer-infill-role.md`: **three rotted pointers corrected in place, nothing
  else.** The symbol `fill_expolygon_multi` → `scan_expolygon` (2 occurrences), the reused id
  `TASK-270` → `DEV-127` (2 occurrences; `docs/07` shows TASK-270 closed as the packet-160
  visual-debug G-code renderer), and the dead path `docs/specs/support-modules-orca-port.md`
  (4 occurrences — Status line, `raft-default` module description, trade-off bullet, References list;
  it now exists only under `docs/specs/_OLD/`). **No `## Amendment` section. No normative content
  changes.** The `**Trade-offs we explicitly accept**:` bullet stating the duplication is NOT
  addressed remains true and must stay, as must the `slicer_core::patterns` Future-Reviewer Note.
- `docs/07_implementation_status.md` Workstream 3: the `TASK-325` row, via worker dispatch.

## Out of Scope

- **Any extraction into `slicer-core`.** Forbidden by ADR-0026 and by ADR-0009's Future-Reviewer Note.
  No new `slicer-core` module, no promoted helper, no `slicer_core::patterns` / `infill_ops` /
  `scanline_fill`. AC-2 enforces this mechanically.
- **Any amendment to ADR-0009, ADR-0026 or any other ADR.** Only ADR-0009's three factual pointers are
  corrected. The 2026-08-07 override lapses unused (see §Re-scope above).
- **De-duplication.** Three copies remain. No artifact may claim the duplication is gone.
- A real WIT pattern service (guest→guest algorithm invocation) — the fix DEV-127 and ADR-0009 both
  name. It does not exist in the tree in any form: every interface under
  `crates/slicer-schema/wit/deps/` is a host-calls-guest stage export, and the only guest→host import
  surface is `host-services` / `module-errors` / `profiling` / `config-types` / `ir-handles`, which
  contains no pattern function. Designing it is a separate architectural workstream.
- The ADR-0033 host-service bridge as a mechanism. It exists for algorithms a guest **cannot** link
  (rayon, boostvoronoi); scan-line fill links fine in a guest. See also DEV-094: an SDK wrapper
  without a `wasm32` arm silently runs the native path, which for a pure-math function is
  undetectable by any test.
- Any WIT, IR, `ResolvedConfig`, manifest or schema-version change.
- Canonical's `align_to_grid` / `_infill_direction` anchor and the `full_infill()`
  `(line_spacing + SCALED_EPSILON) / 2` half-spacing inset. Both are real parity gaps, recorded in
  `D-209-HALF-OPEN-SCAN-GRID-ADOPTED`, not ported. The **inset** is demonstrably separable from the
  half-open bound: `make_fill_lines` (the free function `fill_surface_by_multilines` delegates to)
  runs the same `ceil(w/s)` bound from `bounding_box.min.x()` with no inset, the inset being applied
  only by `fill_surface_by_lines` under `params.full_infill()`. **`align_to_grid` is not shown
  separable by that citation** — `make_fill_lines` merges `align_to_grid` into the bbox *before*
  reading `min.x()`, so it governs the grid **origin**, which this packet instead anchors at the
  surface bbox min (canonical's own behaviour when `align_to_grid` is skipped). Porting either moves *all*
  infill phase globally and would swamp this packet.
- Canonical's neighbour-product tangency filter as literal code. Implementing it requires each copy to
  retain contour adjacency (previous/next vertex per edge), which the shared
  `Vec<(i64,i64,i64,i64)>` flattening destroys in all three — a restructure in three places for a
  segment-count-only difference. Recorded as `D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`.
- `adjust_solid_spacing`'s arithmetic (attribution only), `pattern_shift` (infill-only, not a shared
  axis), density-unit reconciliation (infill 0–1 ratio, support 0–100 percent), role / line width /
  output-sink / input-polygon selection, and non-rectilinear patterns (gyroid, lightning, honeycomb).
- Adding rotation to `support-surface-ironing`. It scans axis-aligned by design; giving it a
  `rotate_point` would move ironing geometry on an axis this packet did not decide for it. AC-9 asserts
  no `fn rotate_point` appears there.
- `tree-support::fill_expolygon_tree` and `gyroid-infill::fill_expolygon`. Checked and rejected as
  copies: the former is a grid-sample + Prim-tree generator (`point_in_expolygon` sampling,
  `nearest_boundary_point`), not a scan-line kernel; the latter is a different pattern entirely.

## Authoritative Docs

- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — short; **read whole first**. The
  prohibition that re-scoped this packet.
- `docs/adr/0009-raft-as-layer-infill-role.md` — short; direct read whole. Only its pointers change.
  Do not pin a line count.
- `docs/08_coordinate_system.md` — delegate a SUMMARY. Unit rule for all integer scan-line math.
- `docs/DEVIATION_LOG.md` — very large; grep-only for `DEV-127` and `DEV-094`. Never read whole.
- `docs/07_implementation_status.md` — very large; worker dispatch only, for the TASK-325 row.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `fill_surface_by_lines` supplies the grid decision (`n_vlines = ceil(w/s)`, `x0 = bounding_box.min(0)`, the `full_infill()`-only inset); the free function `make_fill_lines` — which `fill_surface_by_multilines` delegates to, and which owns the bbox, `n_vlines` and `x0` on that path — confirms the bound is separable from the inset while showing `align_to_grid` still sets the grid origin; `slice_region_by_vertical_lines` supplies the vertex-event counts (tangential touch → 0, true crossing → 1) and the `align_to_grid` / `_infill_direction` anchor this packet does not port; `ExPolygonWithOffset`'s constructor supplies the rotation origin.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::_adjust_solid_spacing` is the three-axis comparison behind AC-5.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `fill_expolygons_generate_paths` sets `fill_params.dont_adjust = true` and builds its filler with `Fill::new_from_type`, i.e. canonical drives support and infill through the same `FillRectilinear`. This is the evidence that support gets no solid-spacing adjustment and that one behaviour, not two, is the correct target.

## Acceptance Summary

Criteria are stated only in `packet.spec.md`.

- Positive: `AC-1` through `AC-15`. Negative: `AC-N1` through `AC-N4`.
- Every AC command pins an explicit passed count (`ok\. 1 passed`, `ok\. 2 passed`, `ok\. 3 passed`,
  `ok\. 5 passed`, `ok\. 7 passed`, `ok\. 8 passed`, `ok\. 9 passed`, `ok\. 11 passed`). Measured
  against this harness: a name filter that matches nothing still prints
  `test result: ok. 0 passed; … N filtered out` and exits 0, so a bare `rg -q '^test result: ok'` is
  a vacuous pass the moment the binary exists. **No exemption**, least of all for the two newly
  authored binaries. Do not relax these.
- Every AC command was run against the pre-implementation tree. **Seventeen of the nineteen are
  falsifying today** — they exit non-zero on the unmodified tree and can only pass once the work
  lands (AC-6, AC-9 and AC-N2 re-measured after the guard-coverage strengthening: all three still
  exit 1). AC-2 folds the ADR-0026 no-extraction guard into a compound command with falsifying clauses
  rather than standing alone, because a standalone
  `[ ! -f crates/slicer-core/src/scanline_fill.rs ]` would pass vacuously.
- **Two are deliberate regression guards and pass (rc=0) today. That is by design, not a defect —
  but neither may be counted as evidence that the work happened.**
  - **AC-11** — measured rc=0: `ironing_tdd` already reports `ok. 11 passed`, and the
    `git merge-base` diff of `ironing_tdd.rs` is already empty. Its job is the *conjunction*: after
    the reconciliation the eleven tests must still pass **while** the file stays byte-unchanged. It
    is legitimate because the only way to make a failing ironing test green is to edit its
    assertions, and the diff clause forecloses exactly that.
  - **AC-12** — measured rc=0: all four pinned binaries are green today (9 / 7 / 4 / 9). Its job is
    to prove the reconciliation did **not** move role, density, bridge-orientation or solid-role
    spacing behaviour in the untouched callers. A guard against unintended movement is only
    meaningful if it is green before the change; requiring it to be red would be incoherent.
  - Neither guard may be relaxed, and no closure report may cite AC-11 or AC-12 alone as proof that
    a step was implemented.
- Cross-packet impact: none. No other packet in `docs/specs/deviation-remediation-206-212-plan.md`
  reads or writes `scan_expolygon`, either `fill_expolygon`, ADR-0009 or DEV-127. This packet touches
  no shared crate, so no other packet's guest build is affected — but the three edited modules are
  guests, so `cargo xtask build-guests --check` still applies after each module edit.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd` (pin `ok. 7 passed`) | AC-1, AC-3: the grid re-baseline and the vertex-test rename/strengthen | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p rectilinear-infill --test rectilinear_infill_edge_cases_tdd` (pin `ok. 2 passed`) | AC-4, AC-N2: sub-spacing bail removed, degenerate guard kept | FACT pass/fail |
| `cargo test -p rectilinear-infill --test rectilinear_infill_tdd` (pin `ok. 9 passed`) | AC-12: density/angle/alternation regression | FACT pass/fail |
| `cargo test -p rectilinear-infill --test top_bottom_fill_tdd` (pin `ok. 7 passed`) | AC-12: solid-role spacing adjust still fires per role | FACT pass/fail |
| `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd` (pin `ok. 4 passed`) | AC-12: bridge orientation path unaffected | FACT pass/fail |
| `cargo test -p traditional-support --test traditional_support_tdd` (pin `ok. 8 passed`) | AC-8: eight pre-existing behaviours, four density literals | FACT pass/fail |
| `cargo test -p traditional-support --test support_fill_geometry_tdd` (pin `ok. 5 passed`) | AC-7, AC-N1, AC-N3, AC-N4: the five new support invariants | FACT pass/fail |
| `cargo test -p traditional-support --test enforcer_blocker_tdd` (pin `ok. 9 passed`) | AC-12: paint-policy gating unaffected | FACT pass/fail |
| `cargo test -p support-surface-ironing --test ironing_scanline_parity_tdd` (pin `ok. 3 passed`) | AC-10, AC-N3, AC-N4: the three new ironing invariants | FACT pass/fail |
| `cargo test -p support-surface-ironing --test ironing_tdd` (pin `ok. 11 passed`) | AC-11: eleven pre-existing ironing behaviours, file byte-unchanged | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness after each module edit | FACT: clean or `STALE:` list |
| `cargo xtask check-deviations --check` | Generated Open Deviation Map agrees with the log | FACT pass/fail (`--check` is load-bearing) |
| `cargo check --workspace --all-targets` | Whole-tree compile including test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

`cargo test --workspace` is **not** required for closure and must not be run for this packet; the
targeted matrix above plus `cargo check --workspace --all-targets` is the gate.

## Step Completion Expectations

- `rectilinear-infill` is reconciled first (Step 1). It has by far the strongest test corpus
  (7 + 9 + 7 + 4 + 2 = 29 tests across five binaries), so it is the best detector of a mistaken
  canonical reading before the same decision is applied to two modules with no geometric coverage.
- Support (Step 2) before ironing (Step 3). Support carries more of the reconciliation (rotation
  reference, centroid fallback) and its `traditional_support_tdd.rs` gives partial cover; ironing's
  change is a strict subset of support's minus the rotation.
- `cargo xtask build-guests --check` must be run — and a rebuild performed if it reports `STALE:` —
  after **each** of Steps 1, 2 and 3, before any module test failure is attributed to the
  reconciliation. All three edited modules are guest crates.
- Steps 1, 2 and 3 all change emitted geometry. Capture each step's failing-test list before starting
  the next; `target/test-output.log` is overwritten on every run.
- Ledger and doc edits (Step 4) land last, so nothing is recorded against unproven code.
- **No step may create a file under `crates/slicer-core/`.** If a step's work seems to want a shared
  helper, that is the ADR-0026-forbidden shape reasserting itself — stop and re-read
  `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` §Future-Reviewer Notes.

## Context Discipline Notes

- All three module `src/lib.rs` files are mid-sized (each well under 600 lines) and may be read whole;
  `rectilinear-infill`'s shrinks during this packet, so do not pin its line count.
- `docs/DEVIATION_LOG.md` and `docs/07_implementation_status.md` are grep/dispatch-only. Reading
  either whole will blow the budget on its own.
- Do not open `OrcaSlicerDocumented/` directly. The canonical facts this packet needs are already
  stated in `packet.spec.md` §"Canonical Decisions" and `design.md`; re-verify by delegation if
  challenged.
- Resist reading `crates/slicer-schema/wit/deps/common.wit` and anything under
  `crates/slicer-core/src/`. This packet adds no WIT and creates no `slicer-core` module; opening
  either invites drift back toward the forbidden design.
- Where this packet writes `Point2` or `ExPolygon` in prose it means **`slicer_ir::Point2`** and
  **`slicer_ir::ExPolygon`** (`crates/slicer-ir/src/slice_ir.rs`). Keep them crate-qualified:
  `slicer-core` defines an unrelated `Point2` in
  `crates/slicer-core/src/arachne/sparse_point_grid.rs`, and all three modules depend on
  `slicer-core`, so a bare `Point2` is ambiguous to a reader even though no code in this packet lands
  in that crate.
