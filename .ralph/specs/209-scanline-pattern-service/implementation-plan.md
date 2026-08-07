# Implementation Plan: 209-scanline-pattern-service

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation. Each of Steps 1-3 adjusts or
  authors its pinning tests **first** (red), then changes the module (green).
- Every field below is a context-budget contract and must be filled independently; never write
  "see Step 1".
- **Standing prohibition, every step:** no file may be created or edited under `crates/slicer-core/`.
  See `design.md` §ADR conformance check. A step that wants a shared helper has re-derived the design
  ADR-0026 rejected; stop and re-read the ADR.

## Steps

### Step 1: Reconcile `rectilinear-infill` onto the canonical half-open grid

- Task IDs: `TASK-325`
- Objective: In `modules/core-modules/rectilinear-infill/src/lib.rs`, change `scan_expolygon`'s loop
  bound from `while scan_y <= rmax_y` to `while scan_y < rmax_y` (canonical `ceil(h/s)`); delete the
  `rmax_y - rmin_y < effective_spacing` sub-spacing bail (canonical `ceil(w/s) >= 1` never bails);
  delete the top-boundary post-pass over `rotated_contour` and the `contour_edges` / `rotated_contour`
  vectors that feed it, including the `contour_edges.push(...)` inside the contour collection loop;
  update `scan_expolygon`'s doc comment to describe the canonical half-open grid; and replace
  `adjust_solid_spacing`'s false `/// Ported from OrcaSlicer FillBase.cpp::adjust_solid_spacing` doc
  comment with text naming `D-209-ADJUST-SOLID-SPACING-DIVERGENCE` and the three divergences from
  `Fill::_adjust_solid_spacing` (`FillBase.cpp`). **Its body stays byte-for-byte identical and it stays
  a private fn in this module** — ADR-0026's 2026-08-05 amendment places it in the rectilinear emitter,
  and promoting it is precisely what ADR-0026 forbids. The three axes: canonical uses
  `(width - EPSILON)` as the **numerator** of both divisions (`number_of_intervals = (width - EPSILON) / distance`,
  then `distance_new = (width - EPSILON) / number_of_intervals`) where PnP uses bare `width`; canonical
  **truncates** where PnP `.round()`s; and on the over-cap branch canonical returns
  `floor(distance * 1.2 + 0.5)` where PnP returns the unmodified `distance`. Do **not** write that
  canonical *divides by* `(width - EPSILON)` — that inverts it. Do **not** add canonical's
  `number_of_intervals == 0 -> return distance` guard as a fourth axis: PnP's opening
  `let count = width / distance; if count < 1 { return distance; }` is that same guard, both sides
  return `distance`, and claiming otherwise writes a fabricated canonical claim into a permanent
  artifact. In the two test files, re-baseline the two self-captured fixtures and rename/strengthen the
  vertex test per AC-1, AC-3 and AC-4.
- Precondition: Working tree clean of packet-209 changes.
  `rg -c 'while scan_y <= rmax_y' modules/core-modules/rectilinear-infill/src/lib.rs` returns 1, and
  `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd` is green on the pre-change tree
  with its per-test path counts captured to `target/test-output.log` **and read out** before this step
  overwrites the log.
- Postcondition: AC-1, AC-2, AC-3, AC-4 and AC-5 hold; all five `rectilinear-infill` test binaries are
  green (7 / 2 / 9 / 7 / 4 passed).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - `adjust_solid_spacing` through
    `rotate_point` inclusive — that is `adjust_solid_spacing`, all of `scan_expolygon` (its inlined
    `edges` / `contour_edges` construction, the `refpt` computation, the rotation, the
    `while scan_y <= rmax_y` grid, the half-open edge test, the pairing loop and the top-boundary
    post-pass) and `rotate_point`. Locate by symbol name; the file is 565 lines at time of writing and
    shrinks in this step, so treat any line number as a navigation hint only.
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs`,
    `top_bottom_fill_tdd.rs`, `bridge_infill_emission_tdd.rs` - whole files - purpose: confirm none of
    them asserts an absolute scan-line count (they assert relations), so a failure there is a defect
    rather than a re-baseline candidate.
- Files allowed to edit (at most 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_edge_cases_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/**` (forbidden — ADR-0026)
  - `modules/core-modules/traditional-support/**` (Step 2)
  - `modules/core-modules/support-surface-ironing/**` (Step 3)
  - `docs/**` (Step 4)
- Expected sub-agent dispatches:
  - Question: In `fill_surface_by_lines` (`FillRectilinear.cpp`), quote the `n_vlines` formula, `x0`,
    the loop bound over vlines, and the exact condition gating the
    `(line_spacing + SCALED_EPSILON)/2` inset; also state what the free function `make_fill_lines` —
    which `fill_surface_by_multilines` delegates to — computes for the bbox, `n_vlines` and `x0`, and
    whether `align_to_grid` is merged into that bbox before `x0` is read.; scope: `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp`; return: `SNIPPETS`
    (≤1 snippet, ≤20 lines)
  - Question: Report `cargo xtask build-guests --check` output, truncated to any `STALE:` lines;
    scope: repo root; return: `FACT` (≤5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - direct read, whole (short); the
    prohibition, and the 2026-08-05 amendment that keeps `adjust_solid_spacing` here
  - `docs/08_coordinate_system.md` - delegated SUMMARY; the scan loop is integer-unit throughout
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` - delegate; never load
- Verification:
  - `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 7 passed'` - FACT pass/fail (AC-1, AC-3 count pin; no test is added or removed by the rename)
  - `cd F:/slicerProject/pinch_n_print_cli && F=modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs; rg -q 'ceil\(\) as usize' $F && ! rg -q 'floor\(\) as usize \+ 1' $F && rg -q 'no endpoints shared' $F && rg -q 'fn vertex_event_test_no_double_count' $F && ! rg -q 'half_open_vertex_test_no_double_count' $F && rg -q 'AC-3 crossing vertex:' $F && rg -q 'AC-3 tangential touch:' $F && rg -q 'AC-N1: expected 9 segments for triangle with apex on scan line, got \{\}' $F && echo PASS` - FACT pass/fail (AC-1 + AC-3 static half; the old test name occurs **twice** today — the numbered doc-comment index at the top of the file and the `fn` — and both must go)
  - `cargo test -p rectilinear-infill --test rectilinear_infill_edge_cases_tdd 2>&1 | rg -q '^test result: ok\. 2 passed'` - FACT pass/fail (AC-4, AC-N2)
  - `cd F:/slicerProject/pinch_n_print_cli && F=modules/core-modules/rectilinear-infill/tests/rectilinear_infill_edge_cases_tdd.rs; rg -q 'fn very_small_polygon_emits_one_scan_row_without_panic' $F && ! rg -q 'fn very_small_polygon_emits_no_paths_without_panic' $F && rg -q 'must not panic on a sub-spacing polygon' $F && echo PASS` - FACT pass/fail (AC-4 static half)
  - `cd F:/slicerProject/pinch_n_print_cli && L=modules/core-modules/rectilinear-infill/src/lib.rs; rg -q 'while scan_y < rmax_y' $L && ! rg -q 'rotated_contour|contour_edges' $L && ! rg -q 'rmax_y - rmin_y < effective_spacing' $L && [ ! -f crates/slicer-core/src/scanline_fill.rs ] && ! rg -q '^pub mod (scanline_fill|infill_ops|patterns);' crates/slicer-core/src/lib.rs && echo PASS` - FACT pass/fail (AC-2, including the ADR-0026 no-extraction guard)
  - `cd F:/slicerProject/pinch_n_print_cli && L=modules/core-modules/rectilinear-infill/src/lib.rs; ! rg -q 'Ported from OrcaSlicer FillBase.cpp::adjust_solid_spacing' $L && rg -q 'D-209-ADJUST-SOLID-SPACING-DIVERGENCE' $L && rg -q 'fn adjust_solid_spacing' $L && rg -q 'let new_distance = \(\(width as f64\) / \(count as f64\)\)\.round\(\) as i64;' $L && echo PASS` - FACT pass/fail (AC-5; the last clause proves the arithmetic survived the doc-comment edit untouched)
  - `cargo test -p rectilinear-infill --test rectilinear_infill_tdd 2>&1 | rg -q '^test result: ok\. 9 passed'` - FACT pass/fail
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 | rg -q '^test result: ok\. 7 passed'` - FACT pass/fail
  - `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd 2>&1 | rg -q '^test result: ok\. 4 passed'` - FACT pass/fail
  - `cargo clippy -p rectilinear-infill --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean or `STALE:` list; rebuild without `--check` if stale
- Exit condition: All five infill binaries green at their pinned counts, both static greps pass, and
  the grid change accounts for **exactly** the two re-baselined counts and nothing else.
  `square_10mm_density_20_emits_n_raw_segments` must see **5** — the five main-loop rows at
  `rmin_y + k*2mm` for `k in 0..5`, with no row at `rmax_y` and no post-pass segment.
  `very_small_polygon_emits_one_scan_row_without_panic` must see **1**. If any of
  `rectilinear_infill_tdd`, `top_bottom_fill_tdd` or `bridge_infill_emission_tdd` changes count, that
  is a **defect**: those three assert relations, not absolute counts. Stop and re-derive rather than
  editing an assertion. Sequence the work bound-first: change `<=` to `<`, run
  `rectilinear_raw_emit_tdd`, read the counts out of `target/test-output.log`, **then** delete the
  post-pass and the bail — deleting all three at once makes a wrong count impossible to attribute.

### Step 2: Reconcile `traditional-support` and give it its first geometric coverage

- Task IDs: `TASK-325`
- Objective: In `modules/core-modules/traditional-support/src/lib.rs`, change
  `TraditionalSupport::fill_expolygon` to (a) compute an unrotated-space bbox and a per-ExPolygon
  `refpt` bbox centre, guard `min_x >= max_x || min_y >= max_y`, rotate about `refpt` and add `refpt`
  back after the inverse rotation; (b) start the scan at the rotated-space bbox min instead of
  `min_y + line_spacing`, keeping the exclusive `while scan_y < rmax_y` bound; (c) replace the
  strictly-between test `scan_y > edge_min_y && scan_y < edge_max_y` with a scan-parallel skip
  (`ry1 == ry2`) plus the half-open test (`scan_y >= lo && scan_y < hi`); (d) drop zero-length spans
  (`x_start == x_end`); (e) delete the whole centroid fallback block, which carries the same
  strictly-between **shape** under a different identifier
  (`centroid_y > edge_min_y && centroid_y < edge_max_y`) — it is *not* a second occurrence of the
  literal `scan_y > edge_min_y`. Author
  `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs` with the five tests
  AC-7 names, and correct the density argument `0.2` -> `20.0` in **all four**
  `make_config(true, 0.2, ...)` calls in
  `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` — re-derive with
  `rg -n 'make_config\(true, 0\.2,'`; they are `extrusion_role_is_support_material`,
  `speed_factor_from_config`, `alternating_angle` and `empty_regions_no_output`. **No assertion in that
  file may change.** `collect_edges` and `rotate_point` stay private to this module; ADR-0026 requires
  it.
- Precondition: Step 1 complete and green.
  `rg -c 'scan_y > edge_min_y' modules/core-modules/traditional-support/src/lib.rs` returns **1** (the
  scan loop only — measured; the centroid fallback's predicate is spelled `centroid_y > edge_min_y`
  and does not match this string) and must return 0 afterwards.
  `rg -c 'centroid_y' modules/core-modules/traditional-support/src/lib.rs` is non-zero today and must
  return 0 afterwards.
  `cargo test -p traditional-support --test traditional_support_tdd` is green on the pre-change tree.
- Postcondition: AC-6, AC-7, AC-8, AC-N1, AC-N2, AC-N3 and AC-N4 (support half) hold.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` - `TraditionalSupport::fill_expolygon` in
    full (its `min_y + line_spacing` start, its strictly-between test, its centroid fallback) plus the
    free functions `collect_edges` and `rotate_point`, and `run_support`'s
    `density_ratio = self.density / 100.0` line (the percent semantics behind AC-8). Locate by symbol
    name; the file is 375 lines at time of writing.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - `scan_expolygon`'s `refpt` computation,
    rotation and inverse-rotation blocks only - purpose: the bbox-centre idiom to mirror. **Read only;
    this file is frozen after Step 1.**
  - `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` - whole file - purpose:
    confirm none of its nine paint-policy tests depends on scan-line counts.
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs`
  - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` (density literal only:
    four `0.2` -> `20.0`, 8 changed diff lines; zero assertion edits)
- Files explicitly out of bounds:
  - `crates/slicer-core/**` (forbidden — ADR-0026)
  - `modules/core-modules/rectilinear-infill/**` (frozen after Step 1)
  - `modules/core-modules/support-surface-ironing/**` (Step 3)
  - `docs/**` (Step 4)
- Expected sub-agent dispatches:
  - Question: Does `fill_expolygons_generate_paths` (`SupportCommon.cpp`) set `dont_adjust`, to what
    value, and does canonical build the support filler through the same `Fill::new_from_type` as
    infill?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp`; return: `FACT`
    (≤5 lines)
  - Question: Report `cargo xtask build-guests --check` output, truncated to any `STALE:` lines;
    scope: repo root; return: `FACT` (≤5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY; `refpt` arithmetic is integer-unit
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` - delegate; never load
- Verification:
  - `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 8 passed'` - FACT pass/fail (AC-8)
  - `cargo test -p traditional-support --test support_fill_geometry_tdd 2>&1 | rg -q '^test result: ok\. 5 passed'` - FACT pass/fail (AC-7; the count is pinned because an empty newly-authored test file prints `ok. 0 passed` and exits 0)
  - `cd F:/slicerProject/pinch_n_print_cli && F=modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs; for n in scan_starts_at_rotated_bbox_min crossing_vertex_contributes_one_intersection fill_phase_is_translation_invariant zero_length_span_is_dropped non_positive_spacing_yields_no_paths; do rg -q "fn $n" $F || { echo "MISSING $n"; exit 1; }; done && rg -q 'run_support' $F && echo PASS` - FACT pass/fail (AC-7 static half; `fill_expolygon` is private, so the tests must drive `run_support`)
  - `cd F:/slicerProject/pinch_n_print_cli && L=modules/core-modules/traditional-support/src/lib.rs; ! rg -q 'scan_y > edge_min_y' $L && ! rg -q 'centroid_y' $L && ! rg -q 'scan_y = min_y \+ line_spacing' $L && rg -q 'refpt' $L && rg -q 'min_x >= max_x' $L && echo PASS` - FACT pass/fail (AC-6, AC-N2 support half)
  - `cd F:/slicerProject/pinch_n_print_cli && ! rg -q 'make_config\(true, 0\.2,' modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` - FACT pass/fail (AC-8 density-fixture half)
  - `cd F:/slicerProject/pinch_n_print_cli && F=modules/core-modules/traditional-support/tests/traditional_support_tdd.rs; B="$(git merge-base HEAD master)"; D="$(git diff --unified=0 "$B" -- $F | rg '^[+-][^+-]')"; [ "$(printf '%s\n' "$D" | wc -l)" -eq 8 ] && [ "$(printf '%s\n' "$D" | rg -v 'make_config\(true, (0\.2|20\.0),' | wc -l)" -eq 0 ] && echo PASS` - FACT pass/fail: exactly 8 changed lines (four removed, four added) **and every one of them a `make_config` density literal**. Note `rg '^[+-][^+-]'`, not `rg '^[+-]'`: the latter also matches git's `--- a/` and `+++ b/` headers, so it can never equal the changed-line count. Measured on this tree — compliant four-literal change: `rg -c '^[+-]'` = 10, `rg -c '^[+-][^+-]'` = 8, non-`make_config` lines = 0; the same change plus one relaxed assertion: 12 / 10 / **2**, so the second predicate fails, which is the point. **The base ref is `git merge-base HEAD master`, not `HEAD` and not the index.** A bare `git diff -- $F` is working-tree-vs-index and goes empty once the change is staged; `git diff HEAD -- $F` goes empty once it is committed, which the acceptance-ceremony re-dispatch makes likely. The merge-base form passes unstaged, staged **and** committed, and still trips the content predicate on an assertion relaxation. Verified against this tree: `git merge-base HEAD master` resolves and the file is unmodified on this branch, so the expected count of 8 is meaningful.
  - `cargo test -p traditional-support --test enforcer_blocker_tdd 2>&1 | rg -q '^test result: ok\. 9 passed'` - FACT pass/fail
  - `cargo clippy -p traditional-support --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean or `STALE:` list
- Exit condition: Both support binaries green at their pinned counts (8 and 5), the three static greps
  pass, and the diff-content guard passes. `alternating_angle` is the test most likely to fail: at the
  old 0.2 % density it exercised exactly one degenerate boundary scan line and could not distinguish
  the 0°/90° alternation at all, so its previous green tells you nothing. If it fails at 20.0 %, that
  is a **real signal about the rotation or scan-start change** — diagnose it, never widen the fixture
  (arithmetically impossible at 200 mm spacing), never restore the centroid fallback, never relax an
  assertion.

### Step 3: Reconcile `support-surface-ironing`, the third copy

- Task IDs: `TASK-325`
- Objective: In `modules/core-modules/support-surface-ironing/src/lib.rs`, change
  `SupportSurfaceIroning::fill_expolygon` to start the scan at the bbox min instead of
  `min_y + line_spacing` (keeping the exclusive `while scan_y < max_y` bound), replace the
  strictly-between test with a scan-parallel skip plus the half-open test, drop zero-length spans, and
  add the `min_x >= max_x` half of the degenerate guard so all three copies guard alike. **Add no
  rotation** — ironing scans axis-aligned by design, and giving it a `rotate_point` would move its
  geometry on an axis this packet did not decide for it (AC-9). Author
  `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs` with the three
  tests AC-10 names. `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` is **read-only
  in this step** and must remain byte-identical to the merge-base.
- Precondition: Step 2 complete and green.
  `rg -c 'scan_y > edge_min_y' modules/core-modules/support-surface-ironing/src/lib.rs` returns 1 and
  must return 0 afterwards. `rg -c 'min_x >= max_x'` on the same file returns **0** today (measured —
  the guard is `min_y >= max_y || line_spacing <= 0` and no x-bounds are computed at all) and must be
  non-zero afterwards. `cargo test -p support-surface-ironing --test ironing_tdd` is green on the
  pre-change tree.
- Postcondition: AC-9, AC-10, AC-11, AC-N2 (ironing half), AC-N3 and AC-N4 (ironing halves) hold.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-surface-ironing/src/lib.rs` - whole file (256 lines at time of
    writing): `fill_expolygon`, the free `collect_edges`, and `run_support_postprocess` (the public
    entry point the new tests must drive).
  - `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` - whole file - purpose: confirm
    every one of the eleven tests asserts a **relation** (non-empty, `paths.len() >= 2`,
    narrow-spacing-yields-more-paths, role, z, width, flow) rather than an absolute line count, and to
    borrow its `config_with` / `region_with_square_at_z` fixture idioms. **Read only.**
  - `modules/core-modules/traditional-support/src/lib.rs` - the reconciled `fill_expolygon` scan loop
    only - purpose: the half-open + zero-length-drop idiom to mirror. **Read only; frozen after
    Step 2.**
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-surface-ironing/src/lib.rs`
  - `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/**` (forbidden — ADR-0026)
  - `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` (must stay byte-identical;
    AC-11 checks it against the merge-base)
  - `modules/core-modules/rectilinear-infill/**`, `modules/core-modules/traditional-support/**`
    (frozen)
  - `docs/**` (Step 4)
- Expected sub-agent dispatches:
  - Question: Report `cargo xtask build-guests --check` output, truncated to any `STALE:` lines;
    scope: repo root; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY; `ironing_spacing` crosses mm→units at
    `slicer_ir::mm_to_units(self.ironing_spacing)` and the scan loop stays integer
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` - delegate; never load
- Verification:
  - `cargo test -p support-surface-ironing --test ironing_scanline_parity_tdd 2>&1 | tee target/test-output.log | rg -q '^test result: ok\. 3 passed'` - FACT pass/fail (AC-10)
  - `cd F:/slicerProject/pinch_n_print_cli && F=modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs; for n in scan_starts_at_bbox_min crossing_vertex_contributes_one_intersection zero_length_span_is_dropped; do rg -q "fn $n" $F || { echo "MISSING $n"; exit 1; }; done && rg -q 'run_support_postprocess' $F && echo PASS` - FACT pass/fail (AC-10 static half)
  - `cargo test -p support-surface-ironing --test ironing_tdd 2>&1 | rg -q '^test result: ok\. 11 passed'` - FACT pass/fail (AC-11)
  - `cd F:/slicerProject/pinch_n_print_cli && B="$(git merge-base HEAD master)"; [ -z "$(git diff --unified=0 "$B" -- modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs)" ] && echo PASS` - FACT pass/fail (AC-11 byte-unchanged half; merge-base base ref for the same reason as Step 2's guard)
  - `cd F:/slicerProject/pinch_n_print_cli && L=modules/core-modules/support-surface-ironing/src/lib.rs; ! rg -q 'scan_y > edge_min_y' $L && ! rg -q 'scan_y = min_y \+ line_spacing' $L && ! rg -q 'fn rotate_point' $L && rg -q 'x_start == x_end' $L && rg -q 'min_x >= max_x' $L && echo PASS` - FACT pass/fail (AC-9, AC-N2 ironing half)
  - `cargo clippy -p support-surface-ironing --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT: clean or `STALE:` list
- Exit condition: Both ironing binaries green at their pinned counts (3 and 11), `ironing_tdd.rs`
  byte-unchanged, and the static grep passes (including its `min_x >= max_x` clause, which is the
  only AC coverage of ironing's new x-axis degenerate guard alongside AC-N2's two-file loop). Ironing
  gains one scan line per region and shifts every
  other line by one spacing; every existing assertion is relational, so none should break. **A failure
  in `ironing_tdd` is a real signal about the reconciliation, never a licence to edit that file** —
  if one fails, diagnose the scan-start or vertex change and fix the module.

### Step 4: Amend DEV-127 (it stays Open), file the five deviations, correct ADR-0009's pointers, add the TASK-325 row

- Task IDs: `TASK-325`
- Objective: In `docs/DEVIATION_LOG.md`, leave DEV-127's Status beginning with `Open` — the
  duplication is **not** removed and its target close remains the WIT pattern-services packet — and
  amend its body with a dated reconciliation note stating that packet 209 converged the copies'
  scan-line semantics (vertex test, rotation reference, scan start, scan grid) without de-duplicating
  them; add the **third** copy (`modules/core-modules/support-surface-ironing/src/lib.rs`) to the row
  and its Affected section; and correct the stale type name `SupportFiller` -> `TraditionalSupport` at
  **both** occurrences (row body and Affected section) — **there is no `SupportFiller` type in this
  tree**. File the five new rows with the contents AC-14 specifies:
  `D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE`, `D-209-IRONING-FILL-BEHAVIOUR-CHANGE`,
  `D-209-HALF-OPEN-SCAN-GRID-ADOPTED`, `D-209-ADJUST-SOLID-SPACING-DIVERGENCE`,
  `D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`. For `D-209-HALF-OPEN-SCAN-GRID-ADOPTED`, carry the **corrected**
  separability wording from `design.md` §"Scan grid extent": `make_fill_lines` refutes inseparability
  from the `full_infill()` inset **only**. It does **not** refute inseparability from `align_to_grid`
  — that function merges `align_to_grid` into the bbox before reading `min.x()`, so `align_to_grid`
  sets the grid origin. The row must not claim otherwise.
  In `docs/adr/0009-raft-as-layer-infill-role.md`, correct the
  three rotted pointers **and nothing else**: `fill_expolygon_multi` -> `scan_expolygon`
  (2 occurrences, the alternatives list and the trade-off bullet), `TASK-270` -> `DEV-127`
  (2 occurrences), and the dead path `docs/specs/support-modules-orca-port.md` (**4 occurrences** — the
  Status line, the `raft-default` module description, the trade-off bullet and the References list;
  each must be repointed or removed individually, and the file must end with zero). Note the trade-off
  text lives under `**Trade-offs we explicitly accept**:` — a **bolded list label, not a markdown
  heading**, so it cannot be found with a `^#` grep; its claim that the duplication is NOT addressed
  **remains true and must stay**, as must the `slicer_core::patterns` Future-Reviewer Note. **Add no
  `## Amendment` section and change no normative content**: with no extraction there is nothing to
  amend, and the user-approved 2026-08-07 ADR-0009 override lapses unused. Add the TASK-325
  Workstream 3 row via dispatch.
- Precondition: Steps 1, 2 and 3 all green. Nothing may be recorded against unproven code.
- Postcondition: AC-13, AC-14 and AC-15 hold, and `rg -q 'TASK-325' docs/07_implementation_status.md`
  succeeds.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0009-raft-as-layer-infill-role.md` - whole file (short; re-derive its length rather than
    trusting any figure written here)
  - `docs/DEVIATION_LOG.md` - **grep-only**: for `DEV-127`, for the table header row, and for a recent
    `D-###` row to copy the column format from. Never read whole.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/adr/0009-raft-as-layer-infill-role.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` §"Open Deviation Map" — generated by
    `cargo xtask check-deviations`; edit only the Workstream 3 TASK row
  - `docs/specs/deviation-remediation-206-212-plan.md` — the orchestrator owns it
  - `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` and every other ADR — read-only
  - `crates/**`, `modules/**` (code is frozen after Step 3)
  - `docs/specs/_OLD/**` (historical; do not resurrect)
- Blast-radius discipline: not applicable — this packet adds no struct field and bumps no schema or
  version constant.
- Expected sub-agent dispatches:
  - Question: What is the current `D-###` counter in `docs/DEVIATION_LOG.md` and do any of the five
    `D-209-*` ids already appear there?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤5 lines).
    This is a ledger fact — re-derive now; do not reuse any id quoted earlier in this packet as
    still-free.
  - Question: What is the highest live `TASK-###` in `docs/07_implementation_status.md`, is `TASK-325`
    absent, and what is the exact row format of the last three Workstream 3 entries?; scope:
    `docs/07_implementation_status.md`; return: `FACT` (≤5 lines)
  - Question: Insert the TASK-325 row into Workstream 3 matching the surrounding format; scope:
    `docs/07_implementation_status.md`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read, whole file (short; its content changes
    in this very step, so re-derive rather than quoting a line count)
  - `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - direct read, whole (short);
    confirm nothing written in this step contradicts it
- OrcaSlicer refs:
  - None for this step.
- Verification:
  - `cd F:/slicerProject/pinch_n_print_cli && rg -q '^\| DEV-127 \|.*\| Open' docs/DEVIATION_LOG.md && ! rg -q 'SupportFiller' docs/DEVIATION_LOG.md && rg -q '^\| DEV-127 \|.*support-surface-ironing' docs/DEVIATION_LOG.md && rg -q '^\| DEV-127 \|.*TraditionalSupport::fill_expolygon' docs/DEVIATION_LOG.md && echo PASS` - FACT pass/fail (AC-13; DEV-127 must NOT close — three copies remain)
  - `cd F:/slicerProject/pinch_n_print_cli && for id in D-209-SUPPORT-FILL-BEHAVIOUR-CHANGE D-209-IRONING-FILL-BEHAVIOUR-CHANGE D-209-HALF-OPEN-SCAN-GRID-ADOPTED D-209-ADJUST-SOLID-SPACING-DIVERGENCE D-209-TANGENTIAL-TOUCH-SPAN-SPLIT; do rg -q "$id" docs/DEVIATION_LOG.md || { echo "MISSING $id"; exit 1; }; done && ! rg -q 'D-209-ADR-0009-AMENDED|D-209-INCLUSIVE-SCAN-GRID|D-209-IRONING-SCANLINE-COPY' docs/DEVIATION_LOG.md && echo PASS` - FACT pass/fail (AC-14 rows half; the trailing negative grep proves the three retired ids from the superseded scope were not filed)
  - `cd F:/slicerProject/pinch_n_print_cli && A=docs/adr/0009-raft-as-layer-infill-role.md; ! rg -q '## Amendment' $A && ! rg -q 'fill_expolygon_multi' $A && ! rg -q 'TASK-270' $A && ! rg -q 'docs/specs/support-modules-orca-port\.md' $A && rg -q 'DEV-127' $A && rg -q 'scan_expolygon' $A && rg -q '\*\*Trade-offs we explicitly accept\*\*:' $A && rg -q 'Do not re-suggest extracting patterns to `slicer_core::patterns`' $A && echo PASS` - FACT pass/fail (AC-15; the two trailing positive greps prove the normative content survived, and the leading `! rg -q '## Amendment'` proves the lapsed override was not discharged anyway)
  - `cd F:/slicerProject/pinch_n_print_cli && out="$(cargo xtask check-deviations --check 2>&1)"; rc=$?; if [ "$rc" -eq 0 ] && ! printf '%s\n' "$out" | rg -q 'error|mismatch'; then echo PASS; else printf '%s\n' "$out"; echo FAIL; exit 1; fi` - FACT pass/fail. **`--check` is load-bearing, and so is the `out=$(...); rc=$?` shape: the guard requires exit 0 AND no `error`/`mismatch` in the output, and exits non-zero on failure.** Two earlier forms were vacuous, for different reasons. (1) Without `--check`, `cargo xtask check-deviations` *regenerates* the doc blocks and returns 0 unconditionally — it only fails on an unparseable `docs/DEVIATION_LOG.md`, missing `BEGIN/END GENERATED` markers, or a write error — so it can never detect the drift the guard exists to detect. `xtask/src/main.rs`'s usage text states this outright: `check-deviations --check  Exit 1 if doc 07 or doc 15 generated sections are stale.` (2) The pipe form — `cargo xtask check-deviations 2>&1 | rg -q 'error|mismatch' && echo FAIL || echo PASS` — is vacuous under any exit-code-based dispatch: it exits 0 whether it prints PASS or FAIL (the `||` branch swallows the failure) and ignores a non-zero exit from `xtask` itself. This step adds five deviation rows and amends a sixth, so the gate must **confirm** the generated views are in sync afterwards, never silently rewrite them.
  - `cd F:/slicerProject/pinch_n_print_cli && rg -q 'TASK-325' docs/07_implementation_status.md` - FACT pass/fail
- Exit condition: All five doc greps pass and `cargo xtask check-deviations --check` reports no
  mismatch. DEV-127 is still **Open**. ADR-0009 contains no `## Amendment` section, zero occurrences of
  `fill_expolygon_multi`, `TASK-270` and `docs/specs/support-modules-orca-port.md`, and still contains
  its `slicer_core::patterns` Future-Reviewer Note and its "Trade-offs we explicitly accept" bullet.
  No artifact written in this step states or implies that the scan-line duplication is removed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Highest-blast-radius copy; one module file plus two edited test files; five test binaries and a before/after count comparison |
| Step 2 | M | Second copy, a net-new test file for a module with no geometric coverage, and a **four-literal** density-fixture correction in `traditional_support_tdd.rs` |
| Step 3 | S | Third copy; a strict subset of Step 2's change minus the rotation, plus a small net-new test file; `ironing_tdd.rs` stays byte-unchanged |
| Step 4 | S | Docs and ledger only; DEV-127 amended in place, five deviation rows, three pointer corrections in ADR-0009; all ledger facts re-derived by dispatch |

Aggregate: `M`. No step is L. Split before activation if Step 1's before/after comparison turns out to
need the whole `rectilinear-infill` test corpus rather than the five named binaries.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`
  are clean.
- `cargo xtask build-guests --check` is clean, or a rebuild was performed and all six module test
  binaries re-run afterwards. All three edited modules are guests.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: **DEV-127 stays Open.** The duplication is not
  removed; three copies remain, now agreeing on behaviour. Five `D-209-*` rows are added; the three
  ids from the superseded scope (`D-209-ADR-0009-AMENDED`, `D-209-INCLUSIVE-SCAN-GRID`,
  `D-209-IRONING-SCANLINE-COPY`) are **not** filed.
- **No `crates/slicer-core/` file was created or edited.** This is the ADR-0026 gate and it is checked
  by AC-2's guard clause; a packet that passes every test but violates it has failed.
- No ADR was amended. ADR-0009 carries no `## Amendment` section; the user-approved 2026-08-07 override
  lapsed unused.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command. Note that Step 2's and Step 3's
  diff guards use `git merge-base HEAD master` as the base ref precisely so they still pass after the
  work is committed.
- Record remaining packet-local risk: `rectilinear-infill` output has moved (one fewer scan row per
  region, one more on sub-spacing regions); support and ironing output has moved (one extra row per
  region, every row shifted by one spacing, support's centroid fallback gone); no self-captured
  support or ironing baseline exists to detect further drift.
- Record the residual parity gaps left Open: `D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`,
  `D-209-ADJUST-SOLID-SPACING-DIVERGENCE`, and — inside
  `D-209-HALF-OPEN-SCAN-GRID-ADOPTED` — canonical's `align_to_grid` anchor and the `full_infill()`
  half-spacing inset.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm
  ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must
use `--all-targets` so the test, bench, and example targets compile.
