# Design: 170-seam-livepath-audit

## Controlling Code Paths

- Primary code path: `SeamPlacer::run_wall_postprocess` in `modules/core-modules/seam-placer/src/lib.rs`: per-region `begin_region` → optional seam-target computation (in the per-mode dispatch, with the `aligned` arm now routing through `aligned_seam_target` + `project_onto_wall_segment` for empty `seam_candidates` and an off-vertex `resolved_seam`, per packet 180) → `set_resolved_seam` when a target exists → emission loop pushing every wall via `push_reordered_wall_loop`, rotating only the target index through `rotate_wall_loop`. The wall-preservation invariant (every region entering with N wall loops exits with N) is the audit's load-bearing claim.
- Neighboring tests/fixtures: `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` (builder idioms: `PerimeterRegionViewBuilder`, `seam_candidate`, `set_seam_candidates`, `set_resolved_seam`), `tests/seam_placer_sharp_corner_tdd.rs`, `tests/seam_placer_tdd.rs`, and the new packet-180 fixtures `tests/seam_continuous_projection_tdd.rs` and `tests/seam_degraded_fallback_tdd.rs` (whose `ir_point` / `ir_wall` / `aligned_region` / `config_with_mode` helpers are the right idiom to mirror).
- OrcaSlicer comparison: none — this is a PNP-invariant audit (wall preservation through the builder contract), not a parity port.

## Architecture Constraints

- The wall-preservation invariant is load-bearing downstream: dropping a region's walls propagates through `convert_perimeter_output` (no bucket → no `PerimeterRegion` entry) and corrupts the `(object_id, region_id)` pairing in the per-stage commit path (`layer_executor::apply` in `crates/slicer-runtime/src/layer_executor.rs`, ADR-0020) for multi-region prints. Tests must assert region-level pairing, not just loop counts. The in-module comments document this invariant; the historical "commit_layer_outputs" name is a pre-ADR-0020 legacy reference in the comments and is not a function in the current source — cite `layer_executor::apply` instead.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

(Coordinate-system snippet omitted: fixtures use the module's existing f32-mm point conventions; no mm/unit conversion or Orca constants are involved.)

## Code Change Surface

- Selected approach: black-box regression fixtures at the SDK dispatch level (same harness as `seam_placer_dispatch_tdd.rs` and the packet-180 TDDs), asserting output wall sets structurally; conditional minimal fix in `run_wall_postprocess` only on falsification.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` (new): four tests named exactly as the AC commands expect — `siblings_survive_rotation`, `multi_region_wall_counts_preserved`, `aligned_snap_preserves_siblings`, `tolerance_miss_emits_all_walls_pristine`. Helper building an N-loop region from concentric square loops (distinct, easily distinguishable point sets per loop; closed-loop convention with the explicit closing repeat so `rotate_wall_loop`'s closure-aware path is exercised). Mirror the helper shape used in `seam_continuous_projection_tdd.rs` / `seam_degraded_fallback_tdd.rs`.
  - `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess` — edit only if a fixture fails; candidate defect surfaces are the emission loop and the `seam_target` interplay with `push_reordered_wall_loop` ordering.
- Rejected alternatives and reasons:
  - Host-level integration fixture through `slicer-runtime` layer executor — rejected: heavier harness re-proving what the SDK-level builder already exposes; packet 120's migration deliberately moved these to module-level tests.
  - Property-based random-loop fuzz — rejected for this packet: four deterministic fixtures cover the branch matrix (rotation hit, multi-region, aligned branch, tolerance miss); fuzzing adds nondeterministic CI cost without new branches.

## Files in Scope (read + edit)

- `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` - role: the audit instrument; expected change: new file, four tests + helpers.
- `modules/core-modules/seam-placer/src/lib.rs` - role: audited code; expected change: none unless falsified (then minimal emission-loop fix).
- `docs/07_implementation_status.md` - role: TASK-120c reconciliation; expected change: update the existing reopened `[~]` row, via worker dispatch.

## Read-Only Context

- `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` - purpose: builder idioms to copy.
- `modules/core-modules/seam-placer/tests/seam_continuous_projection_tdd.rs` and `tests/seam_degraded_fallback_tdd.rs` - purpose: packet-180 test style to mirror (same `ir_point` / `ir_wall` / `aligned_region` helper shape, same `expect_err` / `assert_eq!` / `assert!` idioms).
- `crates/slicer-sdk/src/views.rs` - purpose: `PerimeterRegionView` accessor signatures if needed (no line-number pin — the file has shifted under 178/179/180).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - not consulted by this packet at all
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-wasm-host/**` - delegate symbol lookups; the audit is module-local
- `modules/core-modules/seam-planner-default/**` - packet 178's surface; do not touch
- Packet 180's continuous projection, degraded fallback, and default-mode change - already implemented; this audit does not modify any of those paths

## Expected Sub-Agent Dispatches

- Question: exact `PerimeterOutputBuilder` output-inspection API for tests (how `seam_placer_dispatch_tdd.rs` reads back emitted loops/regions); scope: `crates/slicer-sdk/src/builders.rs` + that test file; return: `FACT`; purpose: Step 1 fixture authoring (only if the idiom is not evident from the test file itself).
- Question: apply the docs/07 TASK-120c disposition row (exact text supplied after the audit outcome); scope: `docs/07_implementation_status.md`; return: `FACT` (grep confirmation); purpose: Step 3.

## Data and Contract Notes

- IR/manifest contracts: none change. Output assertions go through the same builder-view API the existing dispatch tests use.
- WIT boundary: untouched.
- Determinism/scheduler constraints: fixtures pick `nearest` mode (deterministic min-by) and `aligned` (deterministic snap via `select_seam_candidate`; deterministic continuous projection via `project_onto_wall_segment`); avoid `random` mode in count fixtures to keep failures reproducible.
- "Point-for-point identical" comparison must include `feature_flags` and `width_profile.widths` (the parallel arrays `rotate_wall_loop` maintains) so a partial-rotation bug cannot pass on points alone. The `rotate_wall_loop` debug-assert that parallelism holds during rotation is the in-module safety net this audit pins externally.

## Locked Assumptions and Invariants

- Wall-preservation invariant: every region entering `run_wall_postprocess` with N wall loops exits with exactly N, in every mode (`nearest`, `rear`, `random`, `aligned`, `aligned_back`) and on every seam-resolution branch (hit, miss, none — including the post-180 aligned continuous-projection path). This packet's tests become its permanent guard.
- Packet 180's `aligned` mode continuous projection and the host-injection of `resolved_seam` exist before AC-3 is written; the aligned branch is the post-180 form, not the pre-180 vertex-snap form.
- The historical `D-109B-SEAM-FATAL-CORRECTED` / `D-108-SEAM-CONSUMED` / `D-98-SEAM-NO-CONSUMER` triad (registered retroactively in `docs/DEVIATION_LOG.md` on 2026-07-23) records the P108→P109 seam-placer correctness arc. The historical claim is that P109 corrected P108's "fatal on empty seam-candidates" carve-out (T-082) to graceful wall preservation; the audit's wall-preservation invariant is the codified form of that correction. `D-109-SEAM-FATAL-CORRECTED` (the pre-rename ID, before the slot was recognised as already taken by `D-109-SELF-CAPTURED-FIXTURES`) is the citation carried by `docs/05_module_sdk.md` and the in-module comment; the canonical log row is `D-109B-SEAM-FATAL-CORRECTED` to match the `D-105B/C/D/E` sub-row convention.

## Risks and Tradeoffs

- Expected-green audit: all fixtures may pass immediately, making the tests look vacuous. Mitigation: each test must be demonstrated RED-capable once by temporarily inverting its assertion locally (not committed) or by construction review in the exit condition; the packet report states which outcome occurred.
- AC-3 couples this packet to the post-180 aligned semantics; if a future packet changes the continuous-projection behavior, AC-3's fixture (0.3 mm offset with a non-empty `seam_candidates` list, exercising `select_seam_candidate`) is the one to re-derive. The fixture was chosen so the `select_seam_candidate` path (rather than the `project_onto_wall_segment` path) is exercised, decoupling the audit from any future projection-behavior change.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: docs/07 disposition edit — `FACT` grep confirmation.

## Open Questions

- `[FWD]` If the audit passes green with no fix, choose the TASK-120c disposition wording: close outright (invariant verified + guarded) vs re-scope to the residual nearest/rear/random tolerance-miss coordinate gap. Default: close TASK-120c and reference the `D-168-SEAM-PREPASS-SOURCE` "Closed in full" entry in `docs/DEVIATION_LOG.md` for the source-geometry half; the planner mesh-corner vs inset-boundary coordinate gap in `nearest` / `rear` / `random` modes is a separate residual that the implementer confirms with the user at the acceptance ceremony only if a residual defect was actually found.
