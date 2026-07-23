# Design: 134_rectilinear-raw-emit

## Controlling Code Paths

- Primary code path: `modules/core-modules/rectilinear-infill/src/lib.rs` — the per-region,
  per-role emission loop stays; the geometry pipeline inside it is replaced:
  `infill_direction` → rotate(−angle) → per-ExPolygon vertical scan (half-open test) →
  pair (enter, exit) → rotate(+angle) → emit `ExtrusionPath3D { points: [start, end] }`.
- Neighboring tests or fixtures: `modules/core-modules/rectilinear-infill/tests/` (existing
  suite — survey which tests pin the OLD wrong geometry and rewrite them alongside; the
  four-role structure tests stay).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Raw-emit boundary (ADR-0025): the module emits geometry only — the spec's "NOT added" list
  is a hard fence; reviewers reject any linking/overlap/filter code here.
- `infill_direction` stays module-local (ADR-0026 trade-off note: it is infill-specific; do
  not promote to slicer-core).
- WIT contract is already in place (TASK-255 closed 2026-07-17): the four partition fields
  (`sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) live on
  `PerimeterRegionView` at `crates/slicer-sdk/src/views.rs:103-108`. The module reads them
  through the existing accessor pattern (`lib.rs:108-109, 120, 139, 158, 178-179`). Do NOT
  re-add WIT/scheduler work — the host partition contract is realized.
- `clip_polylines` is available in `slicer-core::polygon_ops` (TASK-254 closed 2026-07-16).
  The module does not call it (raw emit, linker's job), but the linker (packet 133,
  currently OPEN) will.

## Code Change Surface

- Selected approach: straight port of the single-level scan-line discipline. Integer
  y-intersections per edge with the half-open test (include at `min_y`, exclude at `max_y`);
  per-ExPolygon processing (contour + holes of ONE expolygon form one even-odd universe);
  sort intersections per scan line, pair as (enter, exit). Solid roles pass spacing through
  `adjust_solid_spacing` first. `pattern_shift` offsets the scan-line origin x per layer.
  Per-region config via the 131 region accessor.
- Exact changes: replace `fill_expolygon_multi` + `collect_edges` with
  `scan_convert_expolygon(expoly, spacing, shift) -> Vec<[Point2; 2]>` + `infill_direction` +
  `adjust_solid_spacing` helpers; keep the role loop, `solid_fill_role`, `should_emit`,
  `begin_region` calls; rewrite affected tests; attribution header.
- Rejected alternatives: (a) porting `ExPolygonWithOffset`'s two-level scan — rejected:
  overlap is the linker's (single-level suffices for raw emit); (b) rational-arithmetic
  vertex handling — rejected: the half-open integer test is Orca's own discipline and
  sufficient; (c) keeping the global edge merge with per-polygon tagging — rejected: strictly
  more complex than per-ExPolygon scans and still fragile for nested polygons.

## Files in Scope (read + edit)

- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: the rewrite; expected change:
  geometry core replaced (~250 lines), role loop preserved, `lib.rs:231-237` global-edge-merge
  removed.
- `modules/core-modules/rectilinear-infill/tests/` — `rectilinear_infill_tdd.rs`,
  `rectilinear_infill_edge_cases_tdd.rs`, `top_bottom_fill_tdd.rs`,
  `bridge_infill_emission_tdd.rs` — role: TDD; expected change: 7 new tests
  (AC-1, AC-2, AC-3, AC-4, AC-5, AC-7, AC-N1) in a new `rectilinear_raw_emit_tdd.rs` (or
  appended to the existing edge-cases file); stale-geometry tests in
  `rectilinear_infill_tdd.rs` rewritten with header comments naming each encoded bug. The
  four-role and bridge-angle tests stay green.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — role: only if a dev-dep is needed
  for test fixtures; expected change: minimal or none.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` — the four `perimeter-region-view` partition fields
  (`views.rs:103-108`); the SDK region-view config accessor (131, TASK-256 closed). Ranged
  reads only.
- `crates/slicer-sdk/src/builders.rs` — `InfillOutputBuilder` (`push_sparse_path` /
  `push_solid_path`).
- `crates/slicer-ir/src/resolved_config.rs` — the infill config keys
  (`infill_density`, `infill_angle`, `infill_speed`, `line_width`).
- `.ralph/specs/131_per-region-config-delivery/carve-list.md` — the carve worklist this
  packet appends to at Step 4 (already lists 5 cube_4color_* files as `carved: infill-parity
  D6`).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/{gyroid,lightning}-infill/**`, `modules/core-modules/infill-linker/**`
  — other packets' surfaces.
- Host crates (`slicer-wasm-host`, `slicer-runtime`, `slicer-schema/wit/`) — nothing to
  change; the WIT contract is already realized. Delegate any dispatch-behavior FACT.
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "SUMMARY of FillRectilinear.cpp:842-1154 edge-intersection discipline (how vertices and
  horizontal edges are classified); then SNIPPETS ≤30 lines for the intersection loop" —
  Step 2 driver.
- "SNIPPETS of FillBase.cpp:352-391 (`infill_direction`) and 326-340
  (`adjust_solid_spacing`); ≤30 lines each" — Step 1/3.
- "FACT: what value/config drives `pattern_shift` in FillRectilinear.cpp:3023-3024 (≤5
  lines)" — Step 3.
- "Run `cargo test -p rectilinear-infill 2>&1 | tee target/test-output.log | grep '^test
  result'`; FACT + counts; SNIPPETS ≤20 on failure" — every gate.
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- "FACT: from `crates/slicer-sdk/src/views.rs`, the exact accessor method names for the
  four partition fields and the per-region config accessor (≤ 8 lines)" — Step 1 driver.

## Data and Contract Notes

- IR/manifest contracts: untouched. Emission remains `push_sparse_path`/`push_solid_path`
  with `begin_region` origin discipline (packet 127).
- WIT boundary: none changed; module rebuild only.
- Determinism: scan-line order is x-ascending, intersection sort y-ascending — output order
  deterministic by construction; no hash containers.

## Locked Assumptions and Invariants

- The four-role emission structure, `solid_fill_role` mapping, `should_emit` gating, and the
  manifest are preserved verbatim (spec "stays" list). The
  `top_bottom_fill_tdd.rs` and `bridge_infill_emission_tdd.rs` test suites are the
  pre-commit canary.
- Raw 2-point segments only; endpoints on the un-offset wall-inset boundary; no two emitted
  segments share endpoints.
- `pattern_shift` is module-side (grilling decision; spec open question 4 RESOLVED) — the
  linker connects whatever it receives.
- Rotation rounding ≤ 50 nm is acceptable (below the 100 nm unit floor;
  `docs/08_coordinate_system.md`).
- The linker (packet 133, currently OPEN) is NOT a code-blocker. Output degrades to raw
  segments until 133 lands; the roadmap's degraded-not-failed trade-off (ADR-0025) is
  documented in the implementation plan and exercised by packet 136's AC-N1.

## Risks and Tradeoffs

- Existing module tests pinning the old (wrong) geometry will go red — rewriting them is in
  scope and each rewrite states WHICH bug the old expectation encoded (no silent re-pinning).
  The Step-1 survey must enumerate them by name in a header comment.
- The bridge-angle source (`bridge_orientation_deg` from mesh analysis vs config) must be
  read from the same place the current bridge emission reads — verify with a FACT dispatch,
  don't assume.
- Segment-count formula edge cases (scan line exactly on the bbox edge) — AC-1's `+1` is
  validated against the ported loop bounds, adjusted with a recorded deviation if Orca's
  bound differs.
- The packet is shippable with TASK-258 open, but the user-visible print is degraded until
  133 lands. Document this in the implementation-plan pre-condition and in any closure
  log; the integration packet (136) AC-N1 pins the trade-off at the e2e level.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (the scan-conversion port)
- Highest-risk dispatch: the 842-1154 SUMMARY — must be sectioned; the range is ~300 lines.

## Open Questions

- `[FWD]` `pattern_shift` config source (existing key vs derived) — resolve via the FACT
  dispatch at Step 3; if no config key exists yet, derive shift from layer index parity as
  Orca does and record the mapping.
- `[FWD]` Which existing module tests pin wrong geometry — Step 1's survey enumerates them.
