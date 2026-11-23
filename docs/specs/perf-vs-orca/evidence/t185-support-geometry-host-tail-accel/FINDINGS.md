# Ticket 185 — Exact `SupportGeometry` post-planner host-tail acceleration

- **Date:** 2026-11-23 (UTC)
- **Base commit:** `5761e533` (t184 landed)
- **Active reference currency (unchanged, zero reference change):** t182's set,
  [`../t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json).
  The ticket text names t175's set; t182's is the active currency since t182
  re-recorded it, and every t185 run is checked against it.
- **Guest freshness after the candidate build:** `cargo xtask build-guests --check`
  exit `0`; `cargo xtask build-guests --accelerated --check` exit `0`.
- **Arms (`reduced/output.json` manifest):**
  - `baseline-ordinary` exe `be7fe920…`, modules tree `6823f914…` (= t184's candidate tree)
  - `baseline-accelerated` exe `6e5cf6b1…`, modules tree `718d7892…` (= t184's candidate tree)
  - `candidate-ordinary` exe `e661baa5…`, modules tree `6823f914…`
  - `candidate-accelerated` exe `a7260880…`, modules tree `718d7892…`
  - Guest module trees are identical between arms in both modes
    (`modules_identical`), so the A/B isolates the host change; no host-only
    attribution batch was needed, unlike t184.
- **Verifier:** `verify.py` → `VERIFY PASS`; `verify.py --self-test` → 15/15 mutations rejected.

## 1. Verdict

**LANDED.** The standing t137 4-cell planner gate passes 4/4 supports-on cells
on the pre-registered verdict of record (two pooled ordinary windows,
[`PREREGISTRATION.md`](PREREGISTRATION.md)). The four supports-off cells are
structurally inert and do not regress. Zero output drift and zero gate waivers.

1. **Output parity (fail-closed).** All 384 measured runs exited `0` and
   reproduced t182's active hashes byte for byte:
   - 16 ordinary and 16 accelerated output runs;
   - 160 ordinary A/B, 96 null, 48 accelerated A/B and 48 instrumented runs.

   Every run had `degraded = false`, `0` fatal or non-fatal errors and `0`
   code-1200 diagnostics. `reference.json` is unchanged.
2. **Gate, supports-on (pooled, 10 interleaved pairs/cell, 6 null pairs/cell,
   unsandboxed, `RAYON_NUM_THREADS=12`):**

   | cell | baseline / candidate median wall | wall saving | wins | CPU saving | null median | accel wall / CPU |
   |---|---|---|---|---|---|---|
   | benchy-classic-on | 2.966 / 2.726 s | **+9.28 %** | 10/10 | +2.55 % | 2.42 % | +5.92 % / +4.09 % |
   | benchy-arachne-on | 3.034 / 2.794 s | **+7.21 %** | 9/10 | +1.11 % | 1.66 % | +6.08 % / +1.63 % |
   | base-classic-on | 24.328 / 21.542 s | **+8.81 %** | 8/10 | +2.21 % | 1.44 % | +9.99 % / +2.95 % |
   | base-arachne-on | 23.747 / 22.438 s | **+8.48 %** | 9/10 | +2.48 % | 1.68 % | +10.92 % / +3.11 % |

   Every cell clears the floor by `9.02×`–`11.60×` and sits strictly above
   its pooled null median, with process CPU lower. Accelerated corroborates
   4/4 (3/3 pair wins on wall and CPU in every cell).
3. **Supports-off (inert, floor `−0.80 %`):** pooled wall `+1.01 %`
   (benchy-classic-off), `+0.57 %` (benchy-arachne-off), `+0.54 %`
   (base-classic-off), `+1.70 %` (base-arachne-off). The instrumented batch
   confirms these cells run no planner tail (`inert_cells_tail_ms` all `0`).
4. **Post-planner tail (instrumented, attribution only; median of 3 pairs).**
   The tail is the planner `SupportGeometry` stage_complete minus its last
   planner module_complete:
   - benchy-classic-on `254 → 4 ms`;
   - benchy-arachne-on `236 → 4 ms`;
   - base-classic-on `2,390 → 18 ms`;
   - base-arachne-on `2,180 → 17 ms`.

   Every candidate pair is below every baseline pair. The residual `4–18 ms`
   is not further attributed.

## 2. Per-window and strict-lens disclosure (not gated)

- **Window 2** passes all 8 cells (4 PASS + 4 INERT-PASS).
- **Window 1** passes the 4 supports-on cells. One supports-off cell,
  benchy-arachne-off, reads `−8.11 %` wall (CPU `−0.92 %`) against that
  window's `7.99 %` null median. No t185 code runs in that cell (no planner
  tail). Its window-1 null spread shows the noise: same-binary any-pair `12.83 %`.
  Window 2 reads `+0.57 %` on the same cell, and the pooled verdict passes.
- **Strict any-pair null lens:** 3/4 supports-on PASS. base-classic-on's
  `+8.81 %` is below the largest single null pair (`10.32 %`), though far
  above the null median (`1.44 %`).
- **Accelerated supports-off (3 pairs, not gated):** benchy-classic-off
  `−3.15 %`, base-classic-off `−1.29 %`, benchy-arachne-off `−0.51 %`,
  base-arachne-off `+5.80 %` wall. These cells execute no changed code.
- **Load samples** (`_meta`, instantaneous `Win32_Processor` load before/after
  each batch): `ab` read `100 %` before its first run, sampled immediately after
  the output batches. Every other sample is `0–58 %`, with no external workload
  scheduled during the campaign.

## 3. Route (ESTIMATE ONLY: t174 × t175 × t182 × t183 × t184 × t185; not a ratio of record)

`reduced/route.json` scales t184's estimate by each cell's pooled t185 wall
saving.

| cell | ordinary post-t184 → post-t185 | accelerated post-t184 → post-t185 |
|---|---|---|
| benchy-classic-on | 1.3266x → **1.2036x** | 1.2990x → 1.2221x |
| benchy-arachne-on | 1.1689x → **1.0846x** | 1.1455x → 1.0759x |
| base-classic-on | 0.9249x → **0.8434x** | 0.8537x → 0.7684x |
| base-arachne-on | 0.9106x → **0.8334x** | 0.8211x → 0.7315x |

The supports-off estimates move by their noise-level inert readings
(`route.json`). The estimated `< 1.0x` count is unchanged at 6/8 in both
modes. Only the two benchy supports-on cells remain above parity, now at
`1.20x` and `1.08x` ordinary.

## 4. Disclosures

- **`M5` (order-preserving Rayon validation) was not landed.** With `M1`–`M4`,
  t181's replay put serial validation at `3.38 ms` (benchy) and `15.03 ms`
  (base). Parallelizing it saves a few ms of a 2.7–24 s cell. That gain was not
  worth the ordering and CPU-contention risk t181 recorded for parallel
  Clipper2. Its order-preserving fold test is therefore not applicable.
- **`union_same_family_entries` / cross-family loop restructuring (`M3` tail
  items) was not landed.** The measured residual tail (`4–18 ms`) bounds what
  it could still save. `M3`'s clone removals did land (`Arc::unwrap_or_clone`,
  no `plans.clone()`, no per-entry clone, dead `identities` map removed).
- **The certificate is stricter than the ticket's sketch and t181's prototype.**
  - The ticket's sketch accepted coordinates up to `10^15` and certified when no
    edge pair crosses or touches.
  - The landed `PreparedOverlap::overlaps`
    (`crates/slicer-wasm-host/src/support_overlap.rs`) also requires every edge
    pair to be at least `BOUNDARY_MARGIN_UNITS = 4` apart (exact `i128`
    point-segment distance). It caps coordinates at `MAX_CERTIFIED_COORD = 2^30`
    and refuses rings with `< 3` points.
  - Why: Clipper2 rounds intersection points (TopX). Two near-touching but
    exactly disjoint edges can therefore yield a non-empty Clipper intersection,
    so "exactly disjoint" alone does not prove "Clipper says empty". Anything
    within the margin falls through to the unchanged Clipper2 call with
    `SUPPORT_OVERLAP_TOLERANCE = 0`, so the gate decides exactly as before.
- **Seeding is wired only at the production call site.**
  `ExactZQueryService::with_print_plane_occupancy` is constructed in
  `crates/slicer-runtime/src/prepass.rs` when `SupportAnalysisIR` and `SliceIR`
  are both present. It seeds a plane only when all of these hold:
  - the object exists in `MeshIR` with non-empty vertices;
  - its transform is plainly affine (`m3 = m7 = m11 = 0`, `m15 = 1`), so the
    world mesh is exactly what `collect_print_plane_occupancy` sliced;
  - the plane key is the `units_to_mm(mm_to_units(slice.z))` the producer used.

  Every other `(object, z)`, including independent support planes, falls through
  to the unchanged slicing path. `ExactZQueryService::new` and `query` are
  unchanged.
- **Pre-existing failures, unchanged by t185.** Three
  `support_agg_rasterizer_tdd` tests in `slicer-runtime --test integration` fail
  identically on baseline `5761e533` (stashed tree) and on the t185 tree:
  - `p241_metric_helpers_agree_on_baseline_fixture` reads
    `WallLeakageMetrics { penetration_events: 27, penetrated_area: 9432093.0 }`
    against a tracked `0`;
  - `agg_wall_leakage_measurement_beats_baseline` reads 25 events,
    `9797675.0 units^2`;
  - `agg_column_continuity_measurement_beats_baseline` reads legacy area
    `234589896907.5` against tracked `234655288153.5`.

  They are tracked-baseline drift that predates this ticket. The other targeted
  support tests pass: `support_family_routing`, `support_family_closure` and the
  remaining `support_agg_rasterizer_tdd` cases.
- **Reducer fix before the verdict.** The first `reduce_t185.py` run divided by
  the supports-off cells' zero planner tail. The fix restricts the stage
  section to supports-on cells and fails if any supports-off cell shows a
  tail. It touched no timing or gate arithmetic.

## 5. Production implementation

- **`crates/slicer-wasm-host/src/support_overlap.rs` (new).**
  `clipper_positive_overlap` is the reference decision, shared with
  `expolygon_area`. `PreparedOverlap` caches bounds and a lazily built chunked
  ring index per polygon. Its `overlaps` decides in three steps:
  1. strict bounding-box separation → disjoint;
  2. the exact `certify_disjoint` certificate:
     - every edge pair is at least the margin apart, with no crossing or touch;
     - the NonZero winding of one vertex per ring is `0` in both directions;
  3. otherwise the Clipper2 call.
- **`crates/slicer-wasm-host/src/exact_z_query.rs`.**
  - `with_print_plane_occupancy` seeds the producer's own print-plane
    cross-sections.
  - `with_occupancy` serves seeded planes without building object geometry.
  - `prefetch_cross_sections` skips seeded planes.
  - A shared `cross_section` helper keeps `query` unchanged.
- **`crates/slicer-wasm-host/src/support_aggregation.rs`.**
  - `validate_entry` reads occupancy through `with_occupancy`.
  - `validate_entry` and `entries_overlap` decide overlap through
    `PreparedOverlap`.
  - Entries and plans move instead of cloning; decline diagnostics are built
    before the move.
  - The dead `identities` map is removed.
- **`crates/slicer-runtime/src/prepass.rs`.** `Arc::unwrap_or_clone(plan)`, and
  the seeded service where support analysis and slices exist.

### Tests

- **`support_overlap` unit tests.** 17 adversarial pairs, each with its decision
  path pinned (bbox / certificate / Clipper) and its verdict equal to
  `clipper_positive_overlap`:
  - 1-unit edge and corner overlaps;
  - shared-edge and vertex touches;
  - containment both ways;
  - holes: interior vs 1-unit hole-edge crossing;
  - a reversed contour;
  - a concave bay: clear vs graze;
  - exact gaps inside (1 and 2 units) and outside (5 units) the margin;
  - the coordinate guard.

  Rings with `< 3` points are refused by construction (never certified) and
  have no dedicated case.

  A shifted-pairs sweep covers all three paths. Two mutations are each caught:
  margin `0`, and dropping the winding test.
- **`tests/contract/support_exact_z_seeding_tdd.rs`.**
  - Rotated, scaled affine object: seeded planes are bit-equal to prefetch +
    `query` over 48 planes, with `geometry_build_count` `0`.
  - A non-print plane falls through to slicing.
  - Projective, empty-mesh and missing ("ghost") objects are not seeded, and
    keep their slicing or refusal behaviour.
  - Seeded and slicing services give identical aggregation verdicts, including
    a 1-unit print-plane graze and an independent-plane collision, both
    rejected.

### Gates run

- `cargo test -p slicer-wasm-host`: all binaries green.
- `cargo test -p slicer-runtime --test integration -- support_family_routing support_family_closure support_agg_rasterizer`:
  the three pre-existing failures above, identical on baseline.
- `cargo test -p slicer-runtime --lib -- prepass`: green.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo xtask check-literals`: 0 violations.
- `cargo xtask check-test-quality --report`: 0 findings.
- Both guest freshness checks: exit `0`.
