# Implementation Plan: 168-seam-aligned-modes

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: WIT `layer-plan` parameter on `run-seam-planning`

- Task IDs: `TASK-274`
- Objective: Add `layer-plan: layer-plan-view` to `export run-seam-planning` in world-prepass, plumb it through the SDK trait, macro guest shim, and host dispatch arm, with a major world-version bump.
- Precondition: clean tree; `cargo xtask build-guests --check` clean.
- Postcondition: `run_seam_planning(&self, objects, layer_plan, output, config)` compiles workspace-wide; both prepass guest call sites (seam-planner-default plus any test guests exporting the world) updated; guests rebuilt; WIT-drift contract suite green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
  - `crates/slicer-sdk/src/traits.rs` - lines `560-640`
  - `crates/slicer-wasm-host/src/dispatch.rs` - lines `742-860` (mirror the run-support-geometry layer-plan handling)
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/`, `target/`, perimeter modules
- Expected sub-agent dispatches:
  - Question: which macro arm in `crates/slicer-macros` marshals `run-seam-planning`, and which test guests export world-prepass; scope: `crates/slicer-macros/**`, `crates/slicer-wasm-host/test-guests/**`; return: `LOCATIONS`
  - Question: `cargo build --tests` result after edits; scope: workspace; return: `FACT` (name + ≤20 lines per failure)
- Context cost: `M`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - WIT version policy range only
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of world-prepass section
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests && cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail
- Exit condition: AC-3 grep returns PASS and the contract suite is green; note the macro shim edit (a 4th file) is permitted here as it is mechanical — if it exceeds ~30 lines, stop and split.

### Step 2: Mode surface — enums, parsing, manifests

- Task IDs: `TASK-274`
- Objective: Add `SeamMode::{Aligned, AlignedBack}` to seam-placer (parse `"aligned"`/`"aligned_back"`, extend `seam_mode()`), replace seam-planner-default's `mode: String` with a matching enum, and extend both manifests' `[config.schema.seam_mode].values`.
- Precondition: Step 1 complete (compiling signature).
- Postcondition: AC-1, AC-2, AC-N1 pass; aligned modes parse but planner/placer aligned behavior may still be stubbed (falls through to existing behavior) — stubs must be marked `todo-by-step-6/7` comments, not silent.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/seam-planner-default/src/lib.rs`
- Files allowed to edit (at most 3 + manifests):
  - `modules/core-modules/seam-placer/src/lib.rs` and `modules/core-modules/seam-placer/seam-placer.toml`
  - `modules/core-modules/seam-planner-default/src/lib.rs` and `modules/core-modules/seam-planner-default/seam-planner-default.toml`
  - `modules/core-modules/seam-placer/tests/seam_aligned_mode_tdd.rs` (new; write parse/rejection tests RED first)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host`, `crates/slicer-runtime`
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - locate the `seam_mode` entry only (edit deferred to Step 8)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p seam-placer --test seam_aligned_mode_tdd 2>&1 | grep '^test result'` - FACT pass/fail
  - AC-2 grep chain from `packet.spec.md` - FACT PASS
- Exit condition: AC-1/AC-2/AC-N1 commands PASS; `cargo xtask build-guests --check` reports the two edited guests rebuilt or rebuild performed.

### Step 3: Port comparator and point-picking (comparator.rs)

- Task IDs: `TASK-274`
- Objective: Create `modules/core-modules/seam-planner-default/src/comparator.rs` with the attribution header: internal `SeamCandidate` (visibility, overhang, embedded_distance, local_ccw_angle, central_enforcer), `compute_angle_penalty`, `SeamComparator` (spAligned/spAlignedBack/spRear branch behavior), `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point` — cited by file + function name only.
- Precondition: Step 2 complete.
- Postcondition: unit tests in the same file cover: angle-penalty monotonicity (concave beats convex), comparator rear-branch prefers max-Y, aligned_back visibility bias, deterministic random pick.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/lib.rs`
  - `docs/08_coordinate_system.md`
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/comparator.rs` (new)
  - `modules/core-modules/seam-planner-default/src/lib.rs` (mod declaration only)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/` (delegate), host crates
- Expected sub-agent dispatches:
  - Question: `SeamComparator` predicate bodies + `compute_angle_penalty` formula with all constants and their units; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SNIPPETS` (≤3 × ≤30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - header text
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` - delegate; never load
- Verification:
  - `cargo test -p seam-planner-default comparator 2>&1 | grep '^test result'` - FACT pass/fail
- Exit condition: comparator unit tests green; every ported constant has a unit comment; attribution header present (AC-7 grep passes for this file).

### Step 4: Per-layer contours, visibility, and overhang penalties (contours.rs + visibility.rs)

- Task IDs: `TASK-274`
- Objective: Extract per-layer closed contours from `MeshObjectView` triangles at each layer-plan z (`contours.rs`), then port visibility scoring (`raycast_visibility` / `calculate_candidates_visibility`) and overhang/layer-embedding penalties (`calculate_overhangs_and_layer_embedding`) into `visibility.rs`, populating Step 3's internal `SeamCandidate` per contour vertex.
- Precondition: Step 3 complete.
- Postcondition: for the 20-layer prism fixture, each layer yields one 4-corner contour; corner candidates carry finite visibility and `local_ccw_angle` with concave/convex sign matching canonical convention.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/comparator.rs`
  - `modules/core-modules/seam-planner-default/src/lib.rs`
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/contours.rs` (new)
  - `modules/core-modules/seam-planner-default/src/visibility.rs` (new)
  - `modules/core-modules/seam-planner-default/src/lib.rs` (mod declarations)
- Files explicitly out of bounds:
  - `crates/slicer-core` slicing internals (do not duplicate host slicing; the guest sectioning is deliberately independent)
- Expected sub-agent dispatches:
  - Question: `raycast_visibility` sampling scheme, ray directions, and how visibility folds into the final score; plus `calculate_overhangs_and_layer_embedding` distance conventions; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (≤200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - mm/unit hazards for contour math
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate; never load
- Verification:
  - `cargo test -p seam-planner-default contours 2>&1 | grep '^test result'` and `cargo test -p seam-planner-default visibility 2>&1 | grep '^test result'` - FACT pass/fail
- Exit condition: prism contour/visibility unit tests green; deterministic across two consecutive runs (assert equal output in a test, not by eye); attribution header present on `visibility.rs` (ported); `contours.rs` is PNP-original and carries no porting header.

### Step 5: Port chaining and smoothing (align.rs)

- Task IDs: `TASK-274`
- Objective: Port `find_next_seam_in_layer`, `find_seam_string`, and the `align_seam_points` driver including the least-squares cubic B-spline smoothing (`fit_cubic_bspline` from `Curves.hpp`), operating on Step 3/4 candidates, with `spAlignedBack` rear-biased seeding and the `spRear` branch behavior.
- Precondition: Step 4 complete.
- Postcondition: given per-layer candidate sets for the prism, `align_seam_points` returns per-layer final positions whose XY spread is <= 0.5 mm around one corner (aligned) or a rear corner (aligned_back).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/comparator.rs`
  - `modules/core-modules/seam-planner-default/src/visibility.rs`
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/align.rs` (new)
  - `modules/core-modules/seam-planner-default/src/lib.rs` (mod declaration)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/` (delegate)
- Expected sub-agent dispatches:
  - Question: `align_seam_points` seeding order, seam-string acceptance tolerances, smoothing weights (`angle_weight`), and the final `t*current + (1-t)*fitted` blend; plus `find_seam_string` bidirectional walk rules; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` + ≤3 `SNIPPETS` ≤30 lines
  - Question: `fit_cubic_bspline` signature/algorithm outline; scope: `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - header text
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` - delegate; never load
- Verification:
  - `cargo test -p seam-planner-default align 2>&1 | grep '^test result'` - FACT pass/fail
- Exit condition: align unit tests green including an alignment-spread assertion and an aligned_back rear assertion at the function level (pre-wire-up); attribution header present.

### Step 6: Wire aligned/aligned_back into `run_seam_planning`

- Task IDs: `TASK-274`
- Objective: For `Aligned`/`AlignedBack`, drive Steps 3-5 over real layer z's from the new `layer_plan` parameter and emit one `SeamPlanEntry` per `(global_layer_index, object_id, region_id)` with the chained+smoothed `chosen_position` and full `scored_candidates`; nearest/rear/random keep the existing MVP path byte-for-byte.
- Precondition: Steps 1-5 complete.
- Postcondition: AC-4 and AC-5 pass via `tests/seam_aligned_planning_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/{comparator,visibility,align,contours}.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_planner_tdd.rs` (fixture idioms)
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/lib.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_aligned_planning_tdd.rs` (new; RED first)
- Files explicitly out of bounds:
  - host crates; seam-placer
- Expected sub-agent dispatches: none
- Context cost: `M`
- Authoritative docs:
  - none beyond prior steps
- OrcaSlicer refs:
  - none (behavior already extracted)
- Verification:
  - `cargo test -p seam-planner-default 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail (new suite + existing `seam_planner_tdd.rs` regression)
- Exit condition: AC-4/AC-5 commands PASS; existing planner suite green; `cargo xtask build-guests --check` clean after rebuild.

### Step 7: seam-placer aligned consumption (snap-to-candidate)

- Task IDs: `TASK-274`
- Objective: In `run_wall_postprocess`, when mode is `Aligned`/`AlignedBack`, compute the seam target from the host-injected `region.resolved_seam()` snapped to the nearest `seam_candidates()` position by 2D XY distance (fallback: nearest wall-loop vertex when candidates are empty; emit pristine when neither exists), then reuse `find_seam_location` + `rotate_wall_loop`. Nearest/rear/random keep the existing candidate-preference path unchanged.
- Precondition: Step 6 complete (real injected coordinates known for fixtures).
- Postcondition: AC-6 passes; wall-preservation invariant untouched (all loops emitted for every region and branch).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` (builder idioms)
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/seam-placer/tests/seam_aligned_mode_tdd.rs`
- Files explicitly out of bounds:
  - seam-planner-default; host crates
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none (snap mirrors canonical `place_seam`'s nearest-perimeter-point behavior, already extracted)
- Verification:
  - `cargo test -p seam-placer 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail (AC-6 + AC-N2 whole-module)
- Exit condition: AC-6 and AC-N2 commands PASS; guest freshness check clean after rebuild.

### Step 8: Docs, ADR, deviation row, crosswalk

- Task IDs: `TASK-274`
- Objective: Land the four Doc Impact edits (docs/03 signature + version, docs/15 `seam_mode` values, `docs/DEVIATION_LOG.md` row `D-168-SEAM-PREPASS-SOURCE`, new `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md`), and mint the `TASK-274` row in `docs/07_implementation_status.md` per `task-map.md`.
- Precondition: Steps 1-7 complete.
- Postcondition: all Doc Impact greps PASS; docs/07 row exists.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail rows only for format
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` - format reference
- Files allowed to edit (at most 3 + crosswalk dispatch):
  - `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` (new)
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - direct full reads of `docs/03_wit_and_manifest.md` and `docs/07_implementation_status.md` — perform both edits via a worker dispatch with exact anchor + replacement text
- Expected sub-agent dispatches:
  - Question: apply the docs/03 world-prepass signature edit and the docs/07 TASK-274 row (exact text supplied); scope: those two files; return: `FACT` (grep confirmations)
- Context cost: `S`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - version policy citation for the ADR
- OrcaSlicer refs:
  - none
- Verification:
  - the four Doc Impact greps from `packet.spec.md` - FACT PASS each
- Exit condition: all greps PASS; ADR cites canonical functions by name only (no line numbers).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | WIT + 3-crate plumbing + guest rebuild |
| Step 2 | S | enums + manifests + parse tests |
| Step 3 | M | comparator port (delegated snippets) |
| Step 4 | M | contours + visibility port |
| Step 5 | M | chaining + B-spline port |
| Step 6 | M | wire-up + planner fixture suite |
| Step 7 | S | snap consumption + placer tests |
| Step 8 | S | docs/ADR/deviation/crosswalk |

Split before activation if aggregate cost exceeds M or any step is L. (Aggregate here is M overall because steps are independent context windows; no step is L.)

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (visibility-sampling deviation, snap-radius constant).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
