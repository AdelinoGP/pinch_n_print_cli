# Design: 170-seam-livepath-audit

## Controlling Code Paths

- Primary code path: `SeamPlacer::run_wall_postprocess` (`modules/core-modules/seam-placer/src/lib.rs:201-279`): per-region `begin_region` → optional seam-target closure (`lib.rs:242-252`; after packet 168, plus the aligned snap branch) → `set_resolved_seam` when a target exists → emission loop `lib.rs:260-275` pushing every wall via `push_reordered_wall_loop`, rotating only the target index through `rotate_wall_loop` (`lib.rs:123-181`).
- Neighboring tests/fixtures: `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` (builder idioms: `PerimeterRegionViewBuilder`, `seam_candidate`, `set_seam_candidates`, `set_resolved_seam`), `tests/seam_placer_sharp_corner_tdd.rs`, `tests/seam_placer_tdd.rs`.
- OrcaSlicer comparison: none — this is a PNP-invariant audit (wall preservation through the builder contract), not a parity port.

## Architecture Constraints

- The wall-preservation invariant is load-bearing downstream: dropping a region's walls propagates through `convert_perimeter_output` (no bucket → no `PerimeterRegion` entry) and corrupts the `(object_id, region_id)` pairing in the per-stage commit path (`layer_executor::apply`, `crates/slicer-runtime/src/layer_executor.rs:1863`, ADR-0020) for multi-region prints. The in-module comments (`lib.rs:208-218`, `lib.rs:226-238`, D-109-SEAM-FATAL-CORRECTED) cite the pre-ADR-0020 name `commit_layer_outputs`, which no longer exists as a function. Tests must assert region-level pairing, not just loop counts.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

(Coordinate-system snippet omitted: fixtures use the module's existing f32-mm point conventions; no mm/unit conversion or Orca constants are involved.)

## Code Change Surface

- Selected approach: black-box regression fixtures at the SDK dispatch level (same harness as `seam_placer_dispatch_tdd.rs`), asserting output wall sets structurally; conditional minimal fix in `run_wall_postprocess` only on falsification.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` (new): four tests named exactly as the AC commands expect — `siblings_survive_rotation`, `multi_region_wall_counts_preserved`, `aligned_snap_preserves_siblings`, `tolerance_miss_emits_all_walls_pristine`. Helper building an N-loop region from concentric square loops (distinct, easily distinguishable point sets per loop; closed-loop convention with the explicit closing repeat so `rotate_wall_loop`'s closure-aware path is exercised).
  - `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess` — edit only if a fixture fails; candidate defect surfaces are the emission loop (`lib.rs:260-275`) and the `seam_target` interplay with `push_reordered_wall_loop` ordering.
- Rejected alternatives and reasons:
  - Host-level integration fixture through `slicer-runtime` layer executor — rejected: heavier harness re-proving what the SDK-level builder already exposes; packet 120's migration deliberately moved these to module-level tests.
  - Property-based random-loop fuzz — rejected for this packet: four deterministic fixtures cover the branch matrix (rotation hit, multi-region, snap branch, tolerance miss); fuzzing adds nondeterministic CI cost without new branches.

## Files in Scope (read + edit)

- `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` - role: the audit instrument; expected change: new file, four tests + helpers.
- `modules/core-modules/seam-placer/src/lib.rs` - role: audited code; expected change: none unless falsified (then minimal emission-loop fix).
- `docs/07_implementation_status.md` - role: TASK-120c reconciliation; expected change: update the existing reopened `[~]` row at line 92, via worker dispatch.

## Read-Only Context

- `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` - purpose: builder idioms to copy.
- `crates/slicer-sdk/src/views.rs` - lines `600-660` only - purpose: `PerimeterRegionView` accessor signatures if needed.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - not consulted by this packet at all
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-wasm-host/**` - delegate symbol lookups; the audit is module-local
- `modules/core-modules/seam-planner-default/**` - packet 168's surface; do not touch

## Expected Sub-Agent Dispatches

- Question: exact `PerimeterOutputBuilder` output-inspection API for tests (how `seam_placer_dispatch_tdd.rs` reads back emitted loops/regions); scope: `crates/slicer-sdk/src/builders.rs` + that test file; return: `FACT`; purpose: Step 1 fixture authoring (only if the idiom is not evident from the test file itself).
- Question: apply the docs/07 TASK-120c disposition row (exact text supplied after the audit outcome); scope: `docs/07_implementation_status.md`; return: `FACT` (grep confirmation); purpose: Step 3.

## Data and Contract Notes

- IR/manifest contracts: none change. Output assertions go through the same builder-view API the existing dispatch tests use.
- WIT boundary: untouched.
- Determinism/scheduler constraints: fixtures pick `nearest` mode (deterministic min-by) and `aligned` (deterministic snap); avoid `random` mode in count fixtures to keep failures reproducible.
- "Point-for-point identical" comparison must include `feature_flags` and `width_profile.widths` (the parallel arrays `rotate_wall_loop` maintains, `lib.rs:127-131` debug assert) so a partial-rotation bug cannot pass on points alone.

## Locked Assumptions and Invariants

- HIGH-2 wall-preservation invariant: every region entering `run_wall_postprocess` with N wall loops exits with exactly N, in every mode and on every seam-resolution branch (hit, miss, none). This packet's tests become its permanent guard.
- Packet 168's `SeamMode::Aligned` and its snap branch exist before AC-3 is written (dependency gate).

## Risks and Tradeoffs

- Expected-green audit: all fixtures may pass immediately, making the tests look vacuous. Mitigation: each test must be demonstrated RED-capable once by temporarily inverting its assertion locally (not committed) or by construction review in the exit condition; the packet report states which outcome occurred.
- AC-3 couples this packet to 168's exact snap semantics; if 168 re-scopes the snap, AC-3's fixture (0.3 mm offset) must be re-derived from 168's landed constants before activation.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: docs/07 disposition edit — `FACT` grep confirmation.

## Open Questions

- `[FWD]` If the audit passes green with no fix, choose the TASK-120c disposition wording: close outright (invariant verified + guarded) vs re-scope to the residual nearest/rear/random tolerance-miss coordinate gap (`lib.rs:210-214`). Default: close TASK-120c and reference packet 168's D-168 deviation row for the coordinate gap; the implementer confirms with the user at the acceptance ceremony only if a residual defect was actually found.
