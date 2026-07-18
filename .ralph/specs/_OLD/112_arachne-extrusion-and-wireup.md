---
status: implemented
packet: 112_arachne-extrusion-and-wireup
task_ids:
  - T-220
  - T-221
  - T-222
  - T-223
  - T-224
  - T-225
  - T-226
  - T-227
  - T-230
  - T-231
  - T-232
  - T-233
  - T-234
---

# 112_arachne-extrusion-and-wireup

## Goal

Close M2 of the perimeter parity roadmap: port the Arachne extrusion-generation pipeline (centrality filtering T-220, per-edge bead-count assignment T-221, bead-count upward+downward propagation T-222, `generateToolpaths` T-223), add the `ExtrusionLine` + `ExtrusionJunction` IR types T-224 with serde compat (schema bump), port `stitch_extrusions` (T-225) + `simplifyToolPaths` (T-226) + `removeSmallLines` (T-227), wire the whole pipeline into `arachne-perimeters::run_perimeters` (T-230 — fills the empty P110-created skeleton with the real Voronoi/beading-based `run_perimeters`), extend the M1 parity harness with 4 Arachne fixtures (T-231: tapered-wedge, narrow-strip-with-widening, max-bead-count-cap, complex-multi-feature-polygon), walk every M2 deviation entry and close or justify (T-232), update `docs/01_system_architecture.md` Tier-2 to name the real Arachne pipeline (T-233 — no "iterative-inset" caveat exists to drop; P108 already cleaned it), and run the M2 closure-ceremony via the gated `cargo xtask test --workspace` (T-234, CLAUDE.md §"Test Discipline" workspace-test exception).

## Problem Statement

**FORWARD-DEPS RESOLVED (both P110 and P111 were `draft` sibling M2 packets at refinement time; both are `implemented` as of this packet's activation, alongside the M1 predecessors P105 + P109):**
- **P110 (`implemented`):** `SkeletalTrapezoidationGraph`, `voronoi_from_segments`, `arachne/preprocess.rs` (`preprocess_input_outline`, `preprocess_per_color_inputs`), and the `arachne-perimeters` skeleton were forward-deps at refinement time; all shipped before Steps 1-9 needed them. NOTE: the old 512-line iterative-inset fake was DELETED by P108; P110 created the fresh skeleton.
- **P111 (`implemented`):** `BeadingStrategy` trait, `BeadingStrategyFactory`, `BeadingFactoryParams`, and `crates/slicer-core/src/beading/` were forward-deps at refinement time; all shipped before Step 2 (bead-count assignment) and Step 9 (wire-up) needed them.
- **P109 (`implemented`):** The `perimeter_parity.rs` harness (`crates/slicer-runtime/tests/integration/perimeter_parity.rs`) and the `cube_4color` fixtures from P109's T-100 are PRESENT and green. Step 10 (T-231 fixtures) extends this harness.
- **P105 (`implemented`):** `LoopType::GapFill` (and `ExtrusionRole::GapFill`) ALREADY exist — P105/T-062b added them additively at schema 4.4.0, both `#[non_exhaustive]`. Gap-fill loops may be emitted directly; there is no longer a forward-dep here.

P110 will ship the foundations (Voronoi wrapper, SkeletalTrapezoidationGraph, parabolic discretization, 9-stage preprocess, per-color MMU dedup, NEW `arachne-perimeters` skeleton with empty `run_perimeters` returning `Ok(())` + `warn!`). P111 will ship the BeadingStrategy stack (trait, 5 strategies, factory, 11 config keys, D-9 strip-pass). P112 closes the loop: extrusion generation reads the SKT graph's centrality marks + per-edge bead counts + propagated transitions and emits `Vec<VariableWidthLines>`; stitch + simplify + removeSmall clean the output; `arachne-perimeters::run_perimeters` is IMPLEMENTED in the P110-created empty skeleton with the real Voronoi/beading-based path. NOTE: the old 512-line iterative-inset fake was DELETED by P108/T-090. At P112 activation the skeleton contains only the `warn!` stub — filling it is T-230's job.

T-224 adds `ExtrusionLine` + `ExtrusionJunction` IR types. These are NEW additions (additive schema change); the bump is minor-version. **Schema version computed at activation:** live `CURRENT_SLICE_IR_SCHEMA_VERSION` = `4.6.0` (`crates/slicer-ir/src/slice_ir.rs:213`; P105 already bumped to 4.4.0 for the `GapFill` variants, and later M1 work carried it to 4.6.0), so the target is `4.7.0`. At activation, the implementer MUST re-read the actual constant value and increment the minor version by 1 — do NOT assume `4.7.0` if a parallel branch bumps first. Both types use `#[serde(default)]` on any new optional fields for round-trip safety with pre-bump fixtures.

T-231 extends P109's parity harness with 4 Arachne-specific fixtures (tapered wedge tests variable widths; narrow strip with widening tests the Widening strategy; max-bead-count cap tests the Limited strategy; complex multi-feature polygon tests the whole SKT graph end-to-end). It also extends the cube_4color test from P109 to assert Arachne produces per-color fragmented walls — this is the M2 half of T-P96-E (M1 half landed in P105 via Classic; the per-color preprocessing from P110 + this packet's wire-up makes Arachne ship the same parity behavior).

T-232 (deviation walk) closes D-7 (boostvoronoi selection — via ADR-0023 in P110), D-9 (sentinel strip — via T-215b in P111), and D-15 (Arachne MMU path — via investigation in P105). **IMPORTANT:** D-7, D-9, and D-15 live in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (the roadmap), NOT in `docs/DEVIATION_LOG.md`. AC-11's closure grep MUST target the roadmap file for these three IDs. Any new deviations registered during M2 work that are added to `docs/DEVIATION_LOG.md` must use the live `D-<pkt>-<SLUG>` format observed in that file. Any new deviations registered during M2 work get closure entries or justified-residual status.

T-233 (architecture doc) updates the Tier-2 `Layer::Perimeters` box: the current text is a bare "Wall generation (Arachne variable-width or classic fixed-width)" label (line ~267) — there is no "iterative-inset" caveat left to remove (P108 already cleaned it). With real Arachne shipping, the box gains an explicit "real Arachne (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack)" description citing P112.

T-234 (closure ceremony) runs the full suite via the gated entry point `cargo xtask test --workspace` (which fires the guest-WASM freshness check before the suite — this packet rebuilds the `arachne-perimeters` guest). This is the workspace-test exception per CLAUDE.md — every prior verification in P112 was narrow (per-crate or per-test); the closure ceremony is the gate that catches cross-cutting regressions in M1 modules that M2 wire-up might have introduced.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** Every constant translated from OrcaSlicer in the centrality / bead-count / propagation / generate_toolpaths code passes through `/100` (1 unit = 100 nm vs OrcaSlicer's 1 unit = 1 nm). `preferred_bead_width_outer - 100` (slicer units) is the stitch gap (OrcaSlicer's `bead_width_x - 1nm` maps to `BeadingFactoryParams::preferred_bead_width_outer - 100` after the /100 conversion; `BeadingFactoryParams` has no `bead_width_x` field). Any constant > 1000000 in geometry code is a red flag.

<!-- snippet: wasm-staleness -->
- **Guest WASM staleness.** T-224 edits IR (adds `ExtrusionLine` + `ExtrusionJunction`), and T-230 edits `arachne-perimeters/src/lib.rs`. Both invalidate guest WASM. After every IR edit AND after Step 9 (T-230 wire-up), the implementer MUST run `cargo xtask build-guests --check`; if STALE, rebuild without `--check` BEFORE running any host/executor test. Failure to rebuild causes Arachne parity tests (AC-10) to fail with a typed-instantiation error masquerading as a wire-up bug.

- **Schema additive change.** T-224 adds new IR types but does NOT remove or rename existing ones. `#[serde(default)]` on all new optional fields keeps round-trip safety with pre-bump fixtures (AC-N2). Schema bump is minor-version — the implementer re-reads the actual `CURRENT_SLICE_IR_SCHEMA_VERSION` at activation (live value at refinement = `4.6.0`; P105/P106/P109 shipped, P105 carried it to 4.4.0 for `GapFill`) and increments minor by 1 (→ `4.7.0`). Do NOT hardcode a target if a parallel branch bumps first.
- **No mid-pipeline panics.** Every `slicer-core::arachne::*` and `skeletal_trapezoidation::*` function returns `Result<_, _>`. Internal `unwrap()` is forbidden; debug-asserts are allowed for invariant checks.
- **No floating-point HashMap keys.** Same as P111 — determinism is required. Stitch's gap-comparison uses sorted Vec, not HashMap-by-distance.

## Data & Contract Notes

- **`SkeletalTrapezoidationGraph` (post-P110, extended this packet)**: edges already carry `r_min`, `r_max`, `central`. This packet adds `bead_count: Option<u32>` (set by `assign_bead_counts`), `is_transition_middle: bool`, `is_transition_end: bool` (set by propagation).
- **`VariableWidthLines`**: type alias for `Vec<ExtrusionLine>`. Each `ExtrusionLine` carries its own `inset_idx`; the outer `Vec<VariableWidthLines>` returned by `generate_toolpaths` is sorted by `inset_idx`.
- **`ExtrusionLine` invariants**: `junctions.len() >= 2`; `is_closed == true` implies `junctions.first().p == junctions.last().p` (within ε); `is_odd` is true iff `inset_idx` is odd; `inset_idx == 0` represents the outermost wall.
- **`ExtrusionJunction.perimeter_index`**: zero-based index within the wall sequence at that vertex. Used by P112's downstream consumers (none yet — defer to follow-on if no consumer).
- **`CentralityParams`**: bundles `transition_filter_dist: f64`, `min_central_distance: f64`. Read from `BeadingFactoryParams`.
- **`BeadCountError`**: `{ CentralityNotRun, InvalidGraph(String) }`.

## Locked Assumptions and Invariants

- `SkeletalTrapezoidationGraph` from P110 is immutable through preprocessing; centrality/bead_count/propagation mutate it via `&mut` references.
- `generate_toolpaths` output's `inset_idx` is monotone: lines at lower indices are outer; the outer Vec is sorted by inset_idx ascending.
- `stitch_extrusions`'s primary preservation: any `ExtrusionLine` where `is_closed == true && inset_idx == 0` is NEVER touched. This is the invariant under AC-6.
- `remove_small_lines`'s primary preservation: same invariant under AC-8 (and AC-N3 negative test). The function MUST check `is_closed && inset_idx == 0` BEFORE the length check.
- Schema bump is minor (`#[serde(default)]` on new optional fields). Live value at refinement = `4.6.0`; P105/P106/P109 shipped (P105 carried it to 4.4.0 for `GapFill`). Implementer re-reads the actual constant at activation and increments minor by 1 (→ `4.7.0`) — the target is NOT hardcoded here. The implementer MUST run AC-N2 (legacy deserialization) before flipping status — if the test fails, the migration adapter is wrong.
- centrality, bead-count, propagation, generate_toolpaths, stitch, simplify, remove_small placed in `slicer-core` (extending P110/P111 surfaces). `ExtrusionLine`/`ExtrusionJunction` IR additions remain in `slicer-ir` (no rename). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.

## Risks and Tradeoffs

- **Real wire-up surface.** T-230 replaces the placeholder with ~50+ lines of pipeline call chain. Risk: edge cases in MMU (per-color iteration) or in degenerate inputs (empty polygons after preprocess). Mitigation: AC-9 (simple square) catches the happy path; AC-10's 4 fixtures + cube_4color cover MMU + edge cases.
- **Schema bump cross-cuts.** T-224 bumps `CURRENT_SLICE_IR_SCHEMA_VERSION`. Live value at refinement = `4.6.0` (P105/P106/P109 shipped). If any parallel branch also bumps before activation, the implementer reconciles by reading the actual constant first and incrementing minor by 1 from that value.
- **Workspace test ceremony surface.** T-234 (gated `cargo xtask test --workspace`) takes >11 minutes; the dispatch returns only the summary line + count. If a regression surfaces, the implementer runs targeted re-checks against the specific failing test — does NOT re-run the whole suite.
- **OrcaSlicer SUMMARY drift.** SUMMARYs ≤ 200 words may omit subtleties (e.g., `propagateBeadingsUpward`'s tie-break when an edge has two upstream beadings). Mitigation: the per-fixture goldens are the source of truth; if a function can't make a golden green after 2 attempts, re-dispatch a tighter SUMMARY for that specific edge case.
- **boostvoronoi version drift.** This packet doesn't change `Cargo.toml`'s `boostvoronoi` pin from P110; if a major boostvoronoi update lands during M2 work and breaks the wrapper, the implementer pins forward via a follow-on (NOT in this packet).
