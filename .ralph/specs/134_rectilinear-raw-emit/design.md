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
  geometry core replaced (~250 lines), role loop preserved.
- `modules/core-modules/rectilinear-infill/tests/` (existing file(s) + additions) — role:
  TDD; expected change: 8 new tests, stale-geometry tests rewritten.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — role: only if a dev-dep is needed
  for test fixtures; expected change: minimal or none.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` — the region-view config accessor (131) + partitioned
  polygon accessors — ranged.
- `crates/slicer-sdk/src/builders.rs` — lines 21-141 (`InfillOutputBuilder`).
- `crates/slicer-ir/src/resolved_config.rs` — lines 577-649 (infill config keys) — key names
  only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/{gyroid,lightning}-infill/**`, `modules/core-modules/infill-linker/**`
  — other packets' surfaces.
- Host crates — nothing to change; delegate any dispatch-behavior FACT.
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

## Data and Contract Notes

- IR/manifest contracts: untouched. Emission remains `push_sparse_path`/`push_solid_path`
  with `begin_region` origin discipline (packet 127).
- WIT boundary: none changed; module rebuild only.
- Determinism: scan-line order is x-ascending, intersection sort y-ascending — output order
  deterministic by construction; no hash containers.

## Locked Assumptions and Invariants

- The four-role emission structure, `solid_fill_role` mapping, `should_emit` gating, and the
  manifest are preserved verbatim (spec "stays" list).
- Raw 2-point segments only; endpoints on the un-offset wall-inset boundary; no two emitted
  segments share endpoints.
- `pattern_shift` is module-side (grilling decision; spec open question 4 RESOLVED) — the
  linker connects whatever it receives.
- Rotation rounding ≤ 50 nm is acceptable (below the 100 nm unit floor;
  `docs/08_coordinate_system.md`).

## Risks and Tradeoffs

- Existing module tests pinning the old (wrong) geometry will go red — rewriting them is in
  scope and each rewrite states WHICH bug the old expectation encoded (no silent re-pinning).
- The bridge-angle source (`bridge_orientation_deg` from mesh analysis vs config) must be
  read from the same place the current bridge emission reads — verify with a FACT dispatch,
  don't assume.
- Segment-count formula edge cases (scan line exactly on the bbox edge) — AC-1's `+1` is
  validated against the ported loop bounds, adjusted with a recorded deviation if Orca's
  bound differs.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (the scan-conversion port)
- Highest-risk dispatch: the 842-1154 SUMMARY — must be sectioned; the range is ~300 lines.

## Open Questions

- `[FWD]` `pattern_shift` config source (existing key vs derived) — resolve via the FACT
  dispatch at Step 3; if no config key exists yet, derive shift from layer index parity as
  Orca does and record the mapping.
- `[FWD]` Which existing module tests pin wrong geometry — Step 1's survey enumerates them.
