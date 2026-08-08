# Design: 209-scanline-pattern-service

## ADR conformance check (required reading before any edit)

This packet was re-scoped on 2026-08-07 because its previous approach violated a standing ADR. The
replacement approach is checked against both governing ADRs here. If a downstream agent finds any
residual conflict, it must be recorded as `[BLOCK]` in §Open Questions rather than worked around.

### ADR-0026 — infill linking algorithms live in the linker module, not `slicer-core`

**Verdict: NO CONFLICT. The new scope actively conforms.**

- ADR-0026 §Context records that `slicer-core::infill_ops` — a module bundling `connect_infill`,
  `chain_or_connect_infill`, `BoundaryInfillGraph`, `infill_direction`, `ExPolygonWithOffset`,
  `adjust_solid_spacing`, `remove_short_polylines` — was proposed at the 2026-07-01 infill-parity
  grilling and rejected by the project owner. §Decision places the algorithms **inside** the owning
  module. §Consequences states `slicer-core` gains **only** `clip_polylines`, because that is generic
  geometry with no domain logic. The previous approach (`crates/slicer-core/src/scanline_fill.rs`
  owning a scan-line fill kernel plus a promoted `adjust_solid_spacing`) is that same proposal
  renamed: a scan-line *fill* kernel is fill-pattern logic, not generic geometry.
- ADR-0026's decisive argument — the multi-language module promise, "a C++ or Zig component must not
  need to link a Rust helper" — applies verbatim to a scan-line kernel, which any third-party fill
  module would otherwise be pushed to depend on.
- §Future-Reviewer Notes says "Do not re-suggest `slicer_core::infill_ops`" and "Do not extract … to
  `slicer-core` 'for reuse' without a second concrete consumer."
- **This packet creates no `slicer-core` file, promotes nothing, and adds no dependency.** Each copy
  keeps its own implementation; only *behaviour* converges. AC-2's guard clause
  (`[ ! -f crates/slicer-core/src/scanline_fill.rs ]` plus the `^pub mod (scanline_fill|infill_ops|patterns);`
  negative grep) enforces this mechanically at closure.
- ADR-0026's **2026-08-05 amendment** independently confirms the placement this packet keeps:
  `infill_direction` and `adjust_solid_spacing` live in the **rectilinear emitter**
  (`modules/core-modules/rectilinear-infill/src/lib.rs`) because they are scan-line geometry
  generation. This packet leaves `adjust_solid_spacing` exactly there and changes only its doc
  comment (AC-5).

### ADR-0009 — raft rendering reuses the `Layer::Infill` role/claim pattern

**Verdict: NO CONFLICT. Its normative content is untouched and remains true.**

- ADR-0009's §Decision is about raft dispatch (`ExtrusionRole::RaftInfill`, `claim:raft-fill`,
  `raft-default` as a synthesizer). This packet touches none of it.
- Its §Future-Reviewer Note "**Do not re-suggest extracting patterns to `slicer_core::patterns`**"
  is **not triggered**, because nothing is extracted. The user-approved 2026-08-07 override of that
  note therefore **lapses unused** — it was granted, and with the new scope there is nothing to
  override. Recorded in `requirements.md` §Re-scope so a future reader does not read its absence as
  an oversight. **Do not re-litigate and do not amend.**
- Its §"Trade-offs we explicitly accept" bullet — "Existing duplication between
  `rectilinear-infill::fill_expolygon_multi` and `traditional-support::fill_expolygon` is NOT
  addressed by this decision. Its proper fix (WIT-interface pattern services) is a separate
  architectural conversation" — **remains literally true after this packet**, because the duplication
  is not removed. It must stay in the file. Only the rotted *pointers* inside it change (the symbol
  name, the reused TASK id, the dead spec path); the claim does not.
- No `## Amendment` section is added. AC-15 asserts its absence.

### Other ADRs checked

- **ADR-0033** (host-service bridge): not engaged. No WIT surface, no host service, no SDK wrapper.
  Its scope is algorithms a guest cannot link (rayon, boostvoronoi); scan-line fill links fine.
- **ADR-0025** (infill linker as raw-emit post-pass): not engaged. This packet changes what raw
  segments the emitters produce, not how they are linked. `rectilinear-infill` still raw-emits.
- Every other `docs/adr/*.md` was checked for the strings `slicer-core`, `scan`, `fill` and `support`
  during re-scoping; none constrains in-place reconciliation of module-private geometry.

## Controlling Code Paths

- Primary code paths, all three edited in place, none merged:
  - `scan_expolygon` and its private `rotate_point` / `adjust_solid_spacing`
    (`modules/core-modules/rectilinear-infill/src/lib.rs`)
  - `TraditionalSupport::fill_expolygon` with its free `collect_edges` / `rotate_point`
    (`modules/core-modules/traditional-support/src/lib.rs`)
  - `SupportSurfaceIroning::fill_expolygon` with its own free `collect_edges`
    (`modules/core-modules/support-surface-ironing/src/lib.rs`)
- The DEV-127 row and the 206-212 plan both call the support side `SupportFiller::fill_expolygon`.
  **There is no `SupportFiller` type in this tree**; the struct is `TraditionalSupport` and
  `fill_expolygon` is an inherent method on it. This packet uses `TraditionalSupport::fill_expolygon`
  everywhere, and AC-13 requires the stale name corrected in the DEV-127 row itself (both
  occurrences: row body and Affected section).
- Neighboring tests/fixtures: `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`
  is the strongest existing pin on scan-line geometry
  (`square_10mm_density_20_emits_n_raw_segments`, `half_open_vertex_test_no_double_count`,
  `solid_spacing_adjusted_for_solid_role`, `pattern_shift_interleaves_layers`,
  `polygon_with_hole_segments_split_around_hole`,
  `angle_45_rotated_output_matches_unrotated_after_inverse`,
  `two_disjoint_expolygons_independent_scan_conversion`).
  `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` pins only
  role/speed/density/alternation and `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs`
  only role/z/width/flow/relative-density — **neither has any geometric-invariant coverage**, which
  is why Steps 2 and 3 each author one.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat
  delegation rules.

## Per-axis rationale

The decision table is in `packet.spec.md` §"Canonical Decisions (the four axes)". This section gives
the reasoning that table's cells compress, and states plainly where a decision is **not** backed by
canonical.

### Vertex/edge inclusion — canonical wins, approximated deliberately

Canonical `slice_region_by_vertical_lines` (`FillRectilinear.cpp`) does two things. It drops a vertex
whose two scan-axis neighbours lie on the **same** side (a tangential touch → **0** intersections),
and it lets a true crossing generate two raw events that a post-sort compaction collapses to **1**.
Counted per vertex case:

| vertex case | canonical | half-open (`lo <= scan_y < hi`) + zero-length drop | strictly-between |
| --- | --- | --- | --- |
| true crossing (one neighbour above, one below) | 1 | 1 (lower edge included at `lo`, upper excluded at `hi`) | **0 — the defect** |
| tangential touch, local minimum | 0 | 2 coincident → zero-length span → dropped | 0 |
| tangential touch, local maximum | 0 | 0 (both edges excluded at `hi`) | 0 |
| scan-parallel edge on the line | edge skipped; its two endpoints enter via their non-parallel neighbours (product 0) | edge skipped; both corner events enter at `lo` | edge skipped; both corners dropped |

So the half-open predicate reproduces canonical's crossing count **exactly**, and reduces a tangential
touch to a zero-length span — coverage-identical to canonical once the zero-length span is dropped.
Its only residual is that a tangential touch strictly *inside* a span splits one span into two
abutting spans: same covered geometry, one extra path. Filed as
`D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`, not fixed.

**Why not port the neighbour-product filter literally.** It needs each edge's originating contour plus
its previous/next vertex. All three copies flatten to a bare `Vec<(i64,i64,i64,i64)>`, which destroys
adjacency; porting the filter means restructuring edge collection **and** adding a classification/dedup
pass in **three** places, for a segment-count-only difference. That is a large change with a small
payoff, and — importantly — it is the kind of change that argues for a shared kernel, which ADR-0026
forbids. The half-open predicate gets the correctness fix (the crossing count) at three one-line edits.

Consequence: `rectilinear-infill` does **not** change on this axis. Support and ironing swap
strictly-between for half-open and gain the zero-length-span drop. AC-N3 and AC-N4 pin both halves.

### Rotation reference — **not decided by canonical; decided on exactness and blast radius**

Canonical `ExPolygonWithOffset`'s constructor rotates about the **world origin**
(`contour.rotate(angle)` with no centre), and `fill_surface_by_lines` takes the bbox **after**
rotation. But canonical's *phase* does not come from the rotation origin: in the sparse branch
`align_to_grid(bounding_box.min, Point(line_spacing, line_spacing), refpt)` snaps the grid to
`_infill_direction`'s reference point, which is the **object** bounding-box centre, deliberately
making sparse infill translation-**dependent** so it aligns across layers. In the solid branch
(`full_infill() && !dont_adjust`) `align_to_grid` is skipped entirely and the grid anchors to the
surface bbox min, which is translation-equivariant.

PnP ports **neither** anchor model. So there is no canonical winner available on this axis without
porting `align_to_grid`, which is explicitly out of scope. **State this plainly; do not claim
canonical authority for the choice.** The decision is bbox-centre, for three reasons:

1. It reproduces canonical's **solid** branch exactly — bbox-centre rotation plus a bbox-min grid is
   the same set of physical scan lines as origin rotation plus a bbox-min grid, which is what
   canonical does when `align_to_grid` is skipped.
2. It is **exactly** translation-equivariant. `refpt` is an integer computed from the input, so
   translating the input by an integer vector leaves the rotated coordinates bit-identical.
   World-origin rotation is equivariant only up to `rotate_point`'s `round()`, i.e. it drifts by up to
   ~1 unit (100 nm) with plate position. That is the *only* respect in which support's current form is
   plate-dependent — it is a rounding-scale effect, **not** a user-visible defect, and no artifact may
   describe it as one.
3. It moves no `rectilinear-infill` geometry, and the change is geometrically inert for support:
   both frames anchor the scan grid at the rotated-space bbox extreme, so the physical scan lines are
   the same up to that ±1-unit rounding.

`support-surface-ironing` never rotates. The axis is **inapplicable** to it; AC-9 asserts it gains no
`rotate_point`, because adding one would move ironing geometry on an axis this packet did not decide
for it.

### Scan start — canonical wins outright

`fill_surface_by_lines` sets `x0 = bounding_box.min(0)`. The second canonical entry point,
`fill_surface_by_multilines`, computes **no** bbox, `n_vlines` or `x0` of its own — it delegates to
the free function `make_fill_lines` (`FillRectilinear.cpp`), which owns all three and passes
`bounding_box.min.x()` straight through to `slice_region_by_vertical_lines`. The first scan line is at
the bbox min. `rectilinear-infill`
already does this; support and ironing start a full spacing later and therefore drop the first line of
every region. They move to the bbox min. **This gains one line per region and shifts every line by one
spacing** — a real, owned support/ironing geometry change
(`D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE`, `D-209-IRONING-FILL-BEHAVIOUR-CHANGE`).

The `full_infill()` inset `x0 += (line_spacing + SCALED_EPSILON) / 2` is applied by canonical **only**
under `params.full_infill()`. PnP applies it nowhere. That remains an unported gap, recorded in
`D-209-HALF-OPEN-SCAN-GRID-ADOPTED`.

### Scan grid extent — canonical wins; shipped infill geometry moves

Canonical is half-open: `n_vlines = (bbox.max(0) - bbox.min(0) + line_spacing - 1) / line_spacing`
(= `ceil(w/s)`), with lines at `x0 + i * line_spacing` for `i` in `[0, n_vlines)`. No line is ever
emitted at the bbox max, even when the width divides evenly. **No PnP copy matches this**:
`rectilinear-infill` is inclusive (`floor(h/s)+1`), support and ironing are exclusive from `min + s`
(`scan_y = min_y + line_spacing; while scan_y < max_y` emits `ceil(h/s) - 1` lines — **not**
`floor((h-s)/s)+1`, which reduces to `floor(h/s)` and is off by one at exact multiples: for
`h = 10 mm`, `s = 2 mm` the code emits **4** lines, not 5). Since the three must agree and none is
canonical, the only defensible target is
canonical.

`ceil(w/s) >= 1` for any `w > 0`, so canonical **never** returns nothing for a sub-spacing region.
That retires two non-canonical behaviours at once: `rectilinear-infill`'s
`rmax_y - rmin_y < effective_spacing` bail and `traditional-support`'s centroid fallback (whose whole
purpose was to produce *something* for a region the bail would have skipped — the bbox-min line now
does that, and does it on the polygon rather than through its centroid).

It also retires `rectilinear-infill`'s top-boundary post-pass, which existed solely because the
half-open vertex test excluded `rmax_y`. Under canonical there is no scan line at `rmax_y` at all, so
the post-pass would emit an **extra**, non-canonical segment. Delete it together with the
`contour_edges` / `rotated_contour` vectors that feed it.

**This moves shipped `rectilinear-infill` output.** Two self-captured fixtures are re-baselined; the
statement, the arithmetic and the "no assertion may be weakened" rule are in `packet.spec.md`
§"Shipped infill geometry moves". The two struck justifications for deferring the grid instead —
"the bound is inseparable from the `full_infill()` inset **and** the `align_to_grid` anchor" and
"re-baselining would weaken an AC" (**inverts** `CLAUDE.md` §Test Discipline) — must not reappear in
any artifact, deviation row or commit message.

The first of those two needs stating precisely, because only half of it is false, and the corrected
form is what must be quoted into `D-209-HALF-OPEN-SCAN-GRID-ADOPTED`:

- **Inseparable from the `full_infill()` inset — refuted.** `make_fill_lines` (`FillRectilinear.cpp`)
  runs the identical `ceil(w/s)` bound and passes `bounding_box.min.x()` with **no** inset; the inset
  is applied only by `fill_surface_by_lines` under `params.full_infill()`. The half-open bound
  therefore stands on its own without the inset, which is why this packet can adopt the bound and
  leave the inset an Open, unported gap.
- **Inseparable from `align_to_grid` — NOT refuted; the citation in fact confirms the coupling.**
  `make_fill_lines` executes `bounding_box.merge(Slic3r::align_to_grid(bounding_box.min,
  Point(line_spacing, line_spacing), refpt))` **before** reading `bounding_box.min.x()`, so its
  `min.x()` is the *post-align* min, not a bare source-bbox min. In canonical, the grid **origin** is
  set by `align_to_grid` on every path except `fill_surface_by_lines`' solid branch
  (`full_infill() && !dont_adjust`), which skips it.

This does not change the decision: PnP adopts canonical's half-open **extent** (`ceil(w/s)` lines from
the grid origin, none at the bbox max) while anchoring that origin at the surface bbox min — exactly
what canonical does in the branch where `align_to_grid` is skipped. The `align_to_grid` anchor remains
an Open, unported parity gap, and no artifact may claim canonical refutes its relevance to the grid
origin.

## Architecture Constraints

- **No file may be created under `crates/slicer-core/`, and no existing `slicer-core` file may be
  edited.** This is the ADR-0026 boundary and the reason this packet exists in its current form. If an
  implementation step starts to want a shared helper, stop and re-read ADR-0026 §Future-Reviewer
  Notes rather than negotiating with it.
- No WIT, IR, manifest or schema-version change. Every edit is inside a module's private geometry, or
  in a module's own test file, or in docs. No struct-literal blast radius and no version-constant
  fallout follows from this packet.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Unit-boundary rule specific to this packet: every scan-line computation stays in **integer units**.
  `slicer_ir::ExPolygon` points are `slicer_ir::Point2 { x: i64, y: i64 }` in units (note: **not**
  `slicer_core::arachne::sparse_point_grid::Point2`, an unrelated same-named type in a crate all three
  modules depend on — keep both names crate-qualified in prose and code). The `mm_to_units` conversion
  of `line_width / density` and the `units_to_mm` conversion into `ExtrusionPath3D` points already sit
  at each module's boundary and stay there. No mm value may be introduced into the scan loop; that is
  the single largest source of porting error in a scan-line routine.

## Code Change Surface

Selected approach — **reconcile in place; each copy keeps its own implementation.**

### `modules/core-modules/rectilinear-infill/src/lib.rs`

- `scan_expolygon`: change the loop bound `while scan_y <= rmax_y` → `while scan_y < rmax_y`.
- `scan_expolygon`: delete the `rmax_y - rmin_y < effective_spacing` early return.
- `scan_expolygon`: delete the top-boundary post-pass (the `for &(rx1, ry1, rx2, ry2) in &rotated_contour`
  loop and its two `Point3WithWidth` constructions) and the `contour_edges` / `rotated_contour`
  vectors that exist only to feed it, including the `contour_edges.push(...)` inside the contour-edge
  collection loop.
- `scan_expolygon`: update the doc comment — it currently advertises "The half-open vertex test
  (include at min_y, exclude at max_y) prevents double-counting at polygon vertices (AC-N1)". Keep the
  half-open description (it is still true and is now the reconciled contract) and add that the grid is
  canonical half-open `ceil(h/s)`.
- `adjust_solid_spacing`: **body byte-for-byte unchanged**; doc comment replaced per AC-5.
- `rotate_point` and everything else: unchanged.

### `modules/core-modules/rectilinear-infill/tests/`

- `rectilinear_raw_emit_tdd.rs`: re-baseline `square_10mm_density_20_emits_n_raw_segments`'s expected
  expression from `(bb_h_mm / spacing_mm).floor() as usize + 1` to `(bb_h_mm / spacing_mm).ceil() as usize`
  and update the two comment lines and the assertion message that name the formula; rename
  `half_open_vertex_test_no_double_count` → `vertex_event_test_no_double_count` at **both** occurrences
  (the numbered doc-comment index at the top of the file and the `fn`); add the two assertions
  required by AC-3. No other assertion changes; no test added or removed.
- `rectilinear_infill_edge_cases_tdd.rs`: rename `very_small_polygon_emits_no_paths_without_panic` →
  `very_small_polygon_emits_one_scan_row_without_panic`, change its assertion from "empty" to
  "exactly one path", and update its explanatory comment (which currently states the sub-spacing bail
  as the reason). Keep the `.expect("run_infill must not panic on a sub-spacing polygon")`.

### `modules/core-modules/traditional-support/src/lib.rs`

- `TraditionalSupport::fill_expolygon`: compute an unrotated-space bbox and a
  `refpt = (min_x + (max_x - min_x) / 2, min_y + (max_y - min_y) / 2)`; add the degenerate guard
  `if min_x >= max_x || min_y >= max_y { return Vec::new(); }`; rotate edge endpoints about `refpt`
  (translate by `-refpt`, then `rotate_point(.., cos_a, -sin_a)`); add `refpt` back after the inverse
  rotation, exactly as `scan_expolygon` does.
- Scan start `let mut scan_y = min_y + line_spacing;` → the rotated-space bbox min; loop bound stays
  exclusive (`while scan_y < rmax_y`).
- Replace `if scan_y > edge_min_y && scan_y < edge_max_y` with a scan-parallel skip (`ry1 == ry2`)
  plus the half-open test (`scan_y >= lo && scan_y < hi`).
- Add the zero-length-span drop (`if x_start == x_end { i += 2; continue; }`).
- Delete the whole centroid fallback block (`if paths.is_empty() { … centroid_y … }`), which carries
  its own copy of the strictly-between shape under a different identifier
  (`centroid_y > edge_min_y && centroid_y < edge_max_y`). Re-derive with `rg -n`: the literal string
  `scan_y > edge_min_y` occurs **once** in this file, in the scan loop; the fallback copy does not
  match it.
- `collect_edges` and `rotate_point` stay. They are private to this module and ADR-0026 requires them
  to.

### `modules/core-modules/traditional-support/tests/`

- `traditional_support_tdd.rs`: **fixture-only**, four `0.2` → `20.0` density literals (8 changed diff
  lines), zero assertion edits.
- `support_fill_geometry_tdd.rs`: **created**, five tests (AC-7).

### `modules/core-modules/support-surface-ironing/src/lib.rs`

- `SupportSurfaceIroning::fill_expolygon`: scan start `min_y + line_spacing` → `min_y`; loop bound
  stays `< max_y`; replace the strictly-between test with the scan-parallel skip plus the half-open
  test; add the zero-length-span drop; add the degenerate guard on the x axis as well as y
  (`min_x >= max_x`) to match the other two.
- **No rotation is added.** Ironing scans axis-aligned by design (AC-9).
- `collect_edges` stays.

### `modules/core-modules/support-surface-ironing/tests/`

- `ironing_scanline_parity_tdd.rs`: **created**, three tests (AC-10).
- `ironing_tdd.rs`: **not edited**. AC-11 asserts it is byte-unchanged against the merge-base.

### Docs

- `docs/DEVIATION_LOG.md` — DEV-127 amended (stays Open), five `D-209-*` rows added.
- `docs/adr/0009-raft-as-layer-infill-role.md` — three rotted pointers only; no normative change.
- `docs/07_implementation_status.md` — Workstream 3 TASK-325 row, via dispatch.

### Rejected alternatives

- **A shared `slicer_core::scanline_fill` kernel** (the packet's own previous design). Rejected: it is
  `slicer-core::infill_ops` renamed, and ADR-0026 rejected that at the 2026-07-01 grilling on the
  multi-language module promise. See §ADR conformance check.
- **The ADR-0033 host-service bridge** (a `scanline-fill` func in `host-services`). Rejected: the
  bridge exists for algorithms a guest **cannot** link (rayon, boostvoronoi per
  `docs/03_wit_and_manifest.md`); scan-line fill links fine in a guest, so the bridge buys a WIT
  surface, a host impl and an SDK wrapper for zero capability. It would also create the DEV-094 shape:
  an SDK wrapper whose `wasm32` arm is missing silently falls through to the native path, and for a
  pure-math function that fallback *works*, making the phantom bridge undetectable by any test.
- **Real WIT pattern services** (guest→guest algorithm invocation) — what DEV-127 and ADR-0009 both
  name as the proper fix. Out of scope: it does not exist in any form. Every interface under
  `crates/slicer-schema/wit/deps/` is a host-calls-guest stage export; there is no import of a stage
  interface by any world, no dispatch-by-name func, and no handle representing another module.
- **Reconciling only support and ironing, leaving `rectilinear-infill` untouched.** Rejected: it would
  converge two copies onto a grid none of them shares with canonical, and would leave the third
  disagreeing — the exact state DEV-127 describes, with extra churn.
- **Deferring the grid axis** (keeping the inclusive grid, filing a parity gap). Rejected: see
  §"Scan grid extent" for the two struck justifications and for why deferral would also entrench the
  gap, since the post-pass deletion is only safe under one of the two grids.

## Files in Scope (read + edit)

Nine primary files across three modules plus docs — more than the target three because the packet's
premise is that three separate copies must each change. **The split is enforced per step: no step
edits more than three files.**

- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: copy 1; expected change: grid bound,
  three deletions, two doc comments. Shrinks by roughly the post-pass block.
- `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` — role: the strongest
  scan-line pin; expected change: one count re-baseline, one rename (2 occurrences), two added
  assertions.
- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_edge_cases_tdd.rs` — role: the
  sub-spacing pin; expected change: one rename, one count re-baseline.
- `modules/core-modules/traditional-support/src/lib.rs` — role: copy 2; expected change: rotation
  reference, scan start, vertex test, zero-length drop, centroid fallback deleted.
- `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` — role: the eight
  pre-existing support behaviours; expected change: **fixture-only**, four density literals, zero
  assertion edits.
- `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs` — role: the five
  support invariants that do not exist today; expected change: created.
- `modules/core-modules/support-surface-ironing/src/lib.rs` — role: copy 3; expected change: scan
  start, vertex test, zero-length drop, guard alignment. No rotation added.
- `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs` — role: the
  three ironing invariants; expected change: created.
- `docs/DEVIATION_LOG.md`, `docs/adr/0009-raft-as-layer-infill-role.md`,
  `docs/07_implementation_status.md` — role: ledger; expected change: per AC-13, AC-14, AC-15 and the
  Doc Impact Statement.

## Read-Only Context

- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs`, `top_bottom_fill_tdd.rs`,
  `bridge_infill_emission_tdd.rs` — whole files — purpose: the three non-edited infill binaries Step 1
  must keep green (9 / 7 / 4 tests).
- `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` — whole file — purpose: the
  eleven ironing behaviours Step 3 must keep green **without editing the file**; every assertion in it
  is relational (non-empty, `paths.len() >= 2`, narrow-spacing-yields-more, role, z, width, flow)
  rather than an absolute count, which is why the scan-start shift should not break it. A failure is
  a real signal.
- `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` — whole file — purpose: the
  nine paint-policy tests Step 2 must keep green.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — whole file (short) — purpose: the
  prohibition. Read before any edit.
- `docs/adr/0009-raft-as-layer-infill-role.md` — whole file (short) — purpose: the exact stale strings
  to correct (`fill_expolygon_multi` 2×, `TASK-270` 2×, `docs/specs/support-modules-orca-port.md` 4×).
  Do not pin a line count.

## Out-of-Bounds Files

- `crates/slicer-core/**` — **forbidden**, read and write. This is the ADR-0026 boundary; opening it
  invites the rejected design back.
- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `crates/slicer-schema/wit/**` — this packet adds no WIT.
- `crates/slicer-sdk/src/host.rs`, `crates/slicer-wasm-host/src/host.rs` — the host-bridge surface,
  rejected as a mechanism; do not edit or browse.
- `docs/specs/deviation-remediation-206-212-plan.md` — the orchestrator owns it.
- `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` — grep or worker dispatch only in
  Steps 1-3; edited only in Step 4; never read whole.
- Unrelated crates and modules (`slicer-runtime`, `slicer-scheduler`, `slicer-gcode`, `tree-support`,
  `gyroid-infill`, `infill-linker`) — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: In `slice_region_by_vertical_lines` (`FillRectilinear.cpp`), how many intersections does a
  true-crossing vertex on the scan line contribute, and how many does a tangential-touch vertex
  contribute? Quote the generation-phase predicate and the post-sort compaction condition.; scope:
  `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp`; return: `SNIPPETS` (≤2 snippets,
  ≤25 lines each); purpose: Steps 1-3, the vertex decision.
- Question: In `fill_surface_by_lines` (`FillRectilinear.cpp`), quote the `n_vlines` formula, `x0`, the
  loop bound, and the exact condition under which the `(line_spacing + SCALED_EPSILON)/2` inset is
  applied. Also state what `make_fill_lines` — the free function `fill_surface_by_multilines`
  delegates to — computes for the bbox, `n_vlines` and `x0`, and whether `align_to_grid` runs before
  `x0` is read.; scope: same file; return:
  `SNIPPETS` (≤1 snippet, ≤20 lines); purpose: Step 1, the grid decision.
- Question: Does `fill_expolygons_generate_paths` (`SupportCommon.cpp`) set `dont_adjust` and to what
  value, and does canonical build the support filler through the same `Fill::new_from_type` as
  infill?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp`; return: `FACT`
  (≤5 lines); purpose: Step 2, justifies no solid-spacing adjustment for support.
- Question: What is the current `D-###` counter and do any of the five `D-209-*` ids already appear in
  `docs/DEVIATION_LOG.md`?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines); purpose:
  Step 4. Re-derive at the moment of writing — do not trust any id quoted in this packet as
  still-free.
- Question: What is the highest live `TASK-###` in `docs/07_implementation_status.md`, is `TASK-325`
  absent, and what is the exact row format of the last three Workstream 3 entries?; scope:
  `docs/07_implementation_status.md`; return: `FACT` (≤5 lines); purpose: Step 4.
- Question: Report `cargo xtask build-guests --check` output verbatim, truncated to the `STALE:`
  lines; scope: repo root; return: `FACT` (≤5 lines); purpose: Steps 1, 2, 3 guest-freshness gate.

## Data and Contract Notes

- IR/manifest contracts: none changed. `ExtrusionPath3D`, `ExtrusionRole`, `Point3WithWidth`,
  `InfillOutputBuilder`, `SupportOutputBuilder` are used exactly as today.
- WIT boundary: untouched. All three modules are guests; only the `ExtrusionPath3D`s they already emit
  cross the boundary.
- Determinism/scheduler constraints: each `fill_expolygon` / `scan_expolygon` must remain a pure
  function of its inputs — no interior mutability, no floating-point accumulation across scan rows
  (`scan_y` is an `i64` accumulator and stays one), no iteration over a `HashMap`. Sort x-intersections
  with a total order on `i64`. No stage, claim or dependency edge changes.

## Locked Assumptions and Invariants

- **Locked: no `slicer-core` file is created or edited.** ADR-0026. AC-2 enforces it. This is the
  single constraint that defines the packet's shape; a step that violates it has failed regardless of
  its test results.
- **Locked: three copies remain.** DEV-127 stays Open (AC-13). No artifact, deviation row, commit
  message or report may state or imply the duplication is removed.
- **Locked: the vertex contract is `true crossing → exactly 1 intersection`, `tangential touch → no
  net span`,** realised as the half-open predicate on non-scan-parallel edges plus the
  zero-length-span drop, in all three copies. Reintroducing a strictly-between test anywhere is a
  regression; AC-N3 and AC-N4 catch it.
- **Locked: the scan grid is canonical half-open** — first line at the rotated-space bbox min, bound
  `scan_y < rmax_y`, `ceil(h/s)` lines, **no** sub-spacing bail. This moves shipped infill geometry,
  by design (`D-209-HALF-OPEN-SCAN-GRID-ADOPTED`). The `line_spacing <= 0` guard and the degenerate
  bbox guard are *different* conditions and must survive (AC-N1, AC-N2).
- **Locked: `adjust_solid_spacing`'s arithmetic is byte-for-byte preserved and stays private to
  `rectilinear-infill`.** Only its attribution changes. Its divergence from
  `Fill::_adjust_solid_spacing` (`FillBase.cpp`) is **exactly three axes** — `(width - EPSILON)` as
  canonical's **numerator** in both expressions (never describe it as a divisor; that inverts it),
  truncation vs `.round()`, and `floor(distance * 1.2 + 0.5)` vs the original `distance` on the
  over-cap branch. Canonical's `number_of_intervals == 0 → return distance` guard is **not** a fourth
  axis; PnP's `if count < 1 { return distance; }` is that same guard.
- **Locked: `support-surface-ironing` gains no rotation.** AC-9.
- **Not locked:** whether `TraditionalSupport::fill_expolygon` keeps its current signature or gains a
  `refpt` helper; whether the half-open predicate is written as `scan_y >= lo && scan_y < hi` or as
  `scan_y < lo || scan_y >= hi { continue }`. Either satisfies the ACs.

## Risks and Tradeoffs

- **Shipped infill geometry moves, and it is the highest-blast-radius module.**
  `rectilinear-infill` feeds sparse, top-solid, bottom-solid, internal-solid and bridge roles. Two
  changes compound: the grid loses its top line, and sub-spacing regions gain one. Expect exactly two
  fixture re-baselines and nothing else. **A count change in any of `rectilinear_infill_tdd`,
  `top_bottom_fill_tdd` or `bridge_infill_emission_tdd` is a defect, not a re-baseline candidate** —
  those three assert relations, not absolute counts, and must pass untouched (AC-12).
- **Support and ironing output moves more.** Each gains a scan line at the bbox min and shifts every
  other line by one spacing, and support loses its centroid fallback. Both are intended and both are
  filed. Neither module has any self-captured baseline, so nothing will detect further drift —
  Steps 2 and 3 author the first geometric coverage either has ever had.
- **The strictly-between → half-open swap is a real correctness fix with a real output change.**
  Every support/ironing region whose contour has a vertex on a scan line currently loses a crossing
  and inverts the remainder of that row. Fixing it changes those rows visibly. That is the point.
- **The zero-length-span drop is easy to forget** in support and ironing, and omitting it is silent:
  the half-open test then emits a 2-point path with identical endpoints at every tangential local
  minimum. AC-N3 exists to catch exactly that.
- **Deleting the top-boundary post-pass and the sub-spacing bail in the same step as the grid change
  makes attribution hard if something breaks.** Change the bound first, run
  `rectilinear_raw_emit_tdd`, capture the counts, then delete. `target/test-output.log` is overwritten
  on every run.
- **The forbidden design will reassert itself.** Three copies making the same four changes is exactly
  the situation that motivates a shared kernel. It is forbidden. Any implementer who finds themselves
  drafting `crates/slicer-core/src/scanline_fill.rs` has re-derived the rejected proposal and must
  stop.
- **All three modules are guests.** `cargo xtask build-guests --check` after each of Steps 1-3, and a
  rebuild if `STALE:`, before attributing any failure to the reconciliation.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1, the infill reconciliation — the module file plus its two edited test files
  plus a before/after count comparison across five binaries)
- Highest-risk dispatch and required return format: the `slice_region_by_vertical_lines` vertex-event
  counts — `SNIPPETS`, ≤2 snippets, ≤25 lines each. If it returns more, redispatch asking only for the
  branch handling `p1.x() == this_x`.

## Open Questions

- `[RESOLVED 2026-08-07]` **May this packet extract a shared scan-line kernel into `slicer-core`?**
  **No.** Forbidden by `docs/adr/0026-infill-linking-algorithms-in-linker-module.md`, which records the
  2026-07-01 rejection of exactly that proposal under the name `slicer-core::infill_ops`. The user
  confirmed on 2026-08-07 that the recipe has already been tried and failed. The packet was re-scoped
  to reconcile-in-place. **Do not re-litigate.**
- `[RESOLVED 2026-08-07]` **What became of the approved ADR-0009 override?** It **lapsed unused**. It
  was granted by the user on 2026-08-07 to permit the extraction; with no extraction there is nothing
  to override, so ADR-0009 is not amended and `D-209-ADR-0009-AMENDED` is not filed. AC-15 asserts the
  ADR carries no `## Amendment` section. This is recorded so a future reader does not read the
  absence as an oversight.
- `[RESOLVED]` **Is `SupportSurfaceIroning::fill_expolygon` a third copy, and does this packet take
  it?** It is (own `collect_edges`, same strictly-between test, same `min_y + line_spacing` start; no
  rotation, no centroid fallback) and this packet **does** take it. Excluding it was the previous
  scope and it left the reconciliation half-fixed: two copies agreeing and a third still carrying the
  crossing-vertex defect.
- `[RESOLVED]` **Does deleting `traditional-support`'s centroid fallback lose behaviour?** No. Its
  purpose was to emit *something* for a region shorter than `line_spacing` along the scan axis. Under
  the canonical half-open grid, every non-degenerate region gets at least one real scan line at the
  bbox min, so the fallback is dead code. The output for such a region changes from one synthetic
  centroid segment to one real bbox-min segment; recorded in
  `D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE`.
- `[FWD]` **Is correcting ADR-0009's three rotted pointers (AC-15) in scope, given that no ADR may be
  amended?** This packet treats it as in scope because it changes **no normative content** — only a
  renamed symbol, a reused task id and a moved file path, all three of which DEV-127 lists under its
  Affected section. AC-15 asserts no `## Amendment` section appears and that the Future-Reviewer Note
  and the "Trade-offs we explicitly accept" bullet both survive verbatim. If the queue orchestrator
  reads "do not edit any ADR" strictly, drop AC-15 and Step 4's ADR edit and move the three pointer
  corrections into DEV-127's reconciliation note instead; nothing else in the packet depends on them.
- `[FWD]` `rectilinear-infill` resolves `line_width` and `infill_density` per region via
  `slicer_sdk::config_resolution::resolve_float`, while `traditional-support` uses module-global values
  and a 0–100 percent density and `support-surface-ironing` uses a module-global spacing. Converging
  the density/spacing units is out of scope here; the implementer should note in the TASK-325 row
  whether it is worth its own follow-up.
