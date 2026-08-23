# Implementation Plan: 234-bridge-false-site-gating

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (none — this packet has no task IDs; the backlog slot is the plan's W-A row).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Port the unsupported-span test (pure function + unit tests)

- Task IDs: none (backlog slot: `docs/specs/bridge-parity-plan.md` §4 W-A)
- Objective: Add `gate_bridge_areas_by_unsupported_span(region: &mut SlicedRegion, lower_layer_slices: Option<&[ExPolygon]>)` to `crates/slicer-core/src/algos/prepass_slice.rs`. A missing lower layer clears candidates; a present lower layer subtracts its ungrown contours (an empty list subtracts nothing), with no expansion-zone growth. Write the net-new flat test file with AC-1/AC-2/AC-N1/AC-N2.
- Precondition: `cargo build -p slicer-core --features host-algos` is green; the canonical `detect_bridging_direction`/`unsupported_edges` geometry has been summarized by a delegated OrcaSlicer read.
- Postcondition: `gate_bridge_areas_by_unsupported_span` is pure and unit-tested; AC-1/AC-2/AC-N1/AC-N2 pass; `cargo xtask check-literals` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/prepass_slice.rs` - lines 197-256 (current `assemble_bridge_areas`)
  - `crates/slicer-ir/src/slice_ir.rs` - lines 599-693 (`BridgeRegion`, `SurfaceClassificationIR.prev_layer_boundaries`)
  - `docs/08_coordinate_system.md` - full (coordinate conversion)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/prepass_slice.rs`
  - `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (net-new)
  - `crates/slicer-core/Cargo.toml` (add `[[test]] name = "bridge_false_site_gating_tdd"` with `required-features = ["host-algos"]`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (wiring is Step 2)
  - `OrcaSlicerDocumented/**` (delegate only)
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` (233's module)
- Blast-radius discipline: this step adds no struct field and no schema constant, so no struct-literal blast radius. The net-new test file is a flat `tests/*.rs` that auto-registers only via the explicit `[[test]]` entry (slicer-core tests are feature-gated; without the entry the file compiles to zero tests under `-p slicer-core`).
- Expected sub-agent dispatches:
  - Question: exact `detect_bridging_direction(to_cover, anchors_area)` floating-edge computation and `unsupported_edges` `diff_pl(..., grown_lower)` geometry; scope: `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` + `BridgeDetector.cpp`; return: `SUMMARY` (≤200 words) + `LOCATIONS` (≤20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd` - FACT pass/fail
  - `cargo xtask check-literals` - exit 0
- Exit condition: AC-1, AC-2, AC-N1, AC-N2 all pass; `check-literals` green.

### Step 2: Wire the gate into `PrePass::ShellClassification` (post-slice)

- Task IDs: none (backlog slot: `docs/specs/bridge-parity-plan.md` §4 W-A)
- Objective: In `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`), use the committed `SliceIR` to build per-object layer presence and, for each region, collect the same object's polygons from `global_layer_index - 1` before calling `gate_bridge_areas_by_unsupported_span`. A missing previous layer means no lower layer and clears candidates; an existing layer, even when its polygons are empty, means a lower layer exists and subtracts ungrown contours (empty subtracts nothing). This avoids `prev_layer_boundaries`, whose omitted key for the flat ceiling's empty overhang band wrongly demoted the layer at z=28.0 (global index 139).
- Precondition: Step 1 complete; committed `SliceIR` layer presence and same-object `SlicedRegion.polygons` are available. The previous global layer index is checked per object, preserving `None` for a missing layer and `Some(&[])` for an existing empty layer.
- Postcondition: the gate runs post-slice for every region; `region.bridge_areas` is gated before `region_partition` consumes it; AC-3/AC-5 reslice commands pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines 104-213 (the `commit_shell_classification_builtin` entry point and body; file is 668 lines)
  - `crates/slicer-runtime/src/region_partition.rs` - lines 160-216 (the `bridge_areas` claim)
  - `docs/04_host_scheduler.md` - delegated SUMMARY of §"Fixed Stage Order" + §"PrePass Execution"
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (blast-radius fallout: re-scope the two `assemble_bridge_areas` non-empty assertions — **outcome (2026-08-22): no edit needed**; both call sites still pass because the gate runs post-slice and direct stamper calls are unaffected)
  - `crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (add a gating-interaction disjointness test if AC-4's existing test does not cover the gated path — **outcome (2026-08-22): no edit needed**; the existing AC-4 test covers precedence disjointness and the partition function is unchanged)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/prepass_slice.rs` (Step 1's surface; only read the gate signature)
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: this step changes the classification output consumed by `region_partition` (precedence inputs) and any golden/parity baseline asserting the current flooded behaviour. The `bridge_detector_tdd.rs` assertions at lines ~775 and ~886 are the known sites and are in this step's edit list — **outcome (2026-08-22): they pass unchanged** (the gate runs post-slice; direct stamper calls are unaffected). The file's 3 pre-existing failures at HEAD (`bridge_footprint_does_not_leak_outside_facet_z_span`, `invalid_bridge_excluded_from_slice_areas`, `supported_bridge_candidate_does_not_emit_bridge_fill`) are stash-verified as not this packet's and are out of scope. Visual-debug captures that snapshot `bridge_areas` during `PrePass::Slice` (now ungated) are enumerated by this step's dispatch but updated in Step 2b (conditional), not here — this step's three-file edit cap has no capacity for them.
- Expected sub-agent dispatches:
  - Question: how `commit_overhang_annotation_builtin` populates `prev_layer_boundaries` — which layers get an entry, and whether an empty previous-layer contour set is stored as an empty `Vec` or omitted; scope: `crates/slicer-runtime/src/`; return: `LOCATIONS` (≤20 entries)
  - Question: every visual-debug capture or golden baseline that asserts on `bridge_areas` content; scope: `crates/slicer-runtime/tests/` + `crates/slicer-runtime/src/`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` - delegate; never load (the `voids = diff(voids, *lower_layer_covered)` removal confirms the subtractive shape)
- Verification:
  - `cargo test -p slicer-runtime --test integration -- region_partition_tdd::ac2_precedence_pairwise_disjoint_under_partial_overlap --nocapture` - FACT pass/fail
  - `cargo test -p slicer-runtime --test unit -- bridge_detector_tdd --nocapture` - FACT pass/fail (the re-scoped `assemble_bridge_areas` assertions at the two call sites)
  - `cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --output target/bridge_false_site.gcode --module-dir modules/core-modules && python3 -c "…" target/bridge_false_site.gcode` - FACT `bridge_layers=N/M z=[…]`
  - `cargo xtask build-guests --check` - exit 0 (freshness gate; the `slicer-core` edit is host-only but the gate is cheap insurance)
- Exit condition: AC-3, AC-4, AC-5 pass; `build-guests --check` exit 0.

### Step 2b: Visual-debug/golden baseline fallout (conditional)

- Task IDs: none (backlog slot: `docs/specs/bridge-parity-plan.md` §4 W-A)
- Objective: Update every visual-debug capture or golden/parity baseline that asserts on `bridge_areas` content and now sees the gated (or ungated-during-`PrePass::Slice`) result. **Conditional — run only if Step 2's visual-debug/golden `LOCATIONS` dispatch returns a non-empty list.**
- Precondition: Step 2 complete; the Step 2 dispatch returned a non-empty list of visual-debug/golden sites.
- Postcondition: every enumerated visual-debug capture and golden baseline reflects the gated classification; no test asserts the old flooded behaviour.
- Files allowed to read, with ranges when over 300 lines:
  - the files named by the Step 2 `LOCATIONS` dispatch (ranged reads only)
- Files allowed to edit (at most 3):
  - the visual-debug/golden test files named by the Step 2 dispatch (up to 3; if more than 3 are affected, split this step)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/prepass_slice.rs` (Step 1's surface)
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: none beyond the enumerated files — the dispatch already enumerated them; do not let a follow-up `cargo check` discover a fourth.
- Expected sub-agent dispatches:
  - none (the enumeration was Step 2's dispatch)
- Context cost: `S` (empty if the conditional is skipped)
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test unit -- bridge_detector_tdd --nocapture` - FACT pass/fail (the only golden baseline asserting the pre-gate non-empty `bridge_areas`; re-scoped in Step 2)
  - No visual-debug capture touches `bridge_areas` at HEAD: the only visual-debug reference is the `bridge_regions` fixture in `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`, which constructs a `SurfaceClassificationIR` fixture and is unaffected by the gate. The arachne parity binaries (`crates/slicer-runtime/tests/arachne_parity.rs`, `crates/slicer-runtime/tests/arachne_parity_gaps.rs`) reference `bridge_areas` only as a self-constructed fixture, not as `assemble_bridge_areas` output, so they do not flip.
  - Discovered-fallout procedure: if Step 2's `LOCATIONS` dispatch names a file not listed above, the worker MUST first write a step-record addendum to this plan naming the discovered file(s) and the exact narrow `cargo test` command for each; that addendum becomes part of the reviewed plan before any new test command runs. No new test command runs against an unnamed file.
- Exit condition: every affected visual-debug/golden test passes; or the conditional is skipped (empty dispatch).

### Step 3: Pre-filter measurement (keep/discard) + completion notes

- Task IDs: none (backlog slot: `docs/specs/bridge-parity-plan.md` §4 W-A)
- Objective: Run AC-3/AC-5 with the `BridgeRegion.is_valid` pre-filter enabled and disabled; record whether the pre-filter changes output, and finalize the keep/discard decision in the packet's completion notes.
- Precondition: Steps 1-2 complete; AC-3/AC-5 green.
- Postcondition: the pre-filter decision is recorded with the measured delta (or "zero delta → discard").
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/prepass_slice.rs` - lines 197-256 (the `is_valid` check)
- Files allowed to edit (at most 3):
  - `docs/spec_packets/234-bridge-false-site-gating/design.md` (record the decision in §"Locked Assumptions and Invariants")
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: none (read-only measurement + a doc note).
- Expected sub-agent dispatches:
  - Question: run the two reslice commands with a temporary `is_valid` bypass and diff the `bridge_layers` counts; scope: `crates/slicer-core/src/algos/prepass_slice.rs`; return: `FACT` (delta present/absent)
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --output target/bridge_false_site.gcode --module-dir modules/core-modules && python3 -c "…" target/bridge_false_site.gcode` - FACT (baseline)
- Exit condition: the keep/discard decision and measured delta are recorded in `design.md`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | pure function + net-new test file + Cargo.toml entry |
| Step 2 | M | wiring + bridge_detector_tdd/region_partition fallout |
| Step 2b | S | visual-debug/golden fallout (conditional; skipped if empty) |
| Step 3 | S | measurement + doc note |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Reconcile reopened/superseded status transitions (none — this is a new packet; the W-A backlog slot has no `docs/07_implementation_status.md` TASK row to update).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile. `cargo test` invocations stay narrow (single crate, single test binary, optional test name), never `--all-targets`.
