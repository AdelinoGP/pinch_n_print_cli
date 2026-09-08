# Implementation Plan: 235-external-bridge-orientation

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (none — this packet has no task IDs; the backlog slot is ISSUE-84's `bridge_angle` half).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- FIRST implementation session pops `stash@{0}` (D10) before Step 1; the stash's floating-edge heuristic is discarded per design.md rejected alternatives. After the pop run `cargo xtask build-guests --check` and arbitrate by exit code (0 fresh / 1 stale / 3 missing wasm-tools) before attributing any guest failure.

## Steps

### Step 1: Port the active inline orientation semantics as pure functions + unit tests

- Task IDs: none (backlog slot: ISSUE-84 `docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md`, `bridge_angle` half)
- Objective: Add `detect_bridging_direction_deg(to_cover: &[ExPolygon], anchors: &[ExPolygon]) -> f32` plus private `floating_edges_of_gated_area` to `crates/slicer-core/src/algos/prepass_slice.rs` — floating-edge candidate normals (`(dy, −dx)` normalized), quantization keys `ceil(atan2·1000)`, cost Σ|edge·normal| over all floating edges over the gated-area boundary minus `expand(raw anchors, 1 unit = SCALED_EPSILON)`, minimal-cost perpendicular returned as degrees mod 180 with the ADR-0061 smallest-quantized-angle tie-break, principal-component minor-axis fallback for empty edge sets, `{1,0}` → 0.0° fully-degenerate fallback — and write the net-new test file with AC-1..AC-5, AC-N1, AC-N2 fixtures.
- Precondition: stash popped (see Execution Rules); guest freshness arbitrated; `cargo build -p slicer-core --features host-algos` green on the post-pop tree; canonical overload bodies summarized by delegated read.
- Postcondition: both functions are pure and deterministic; AC-1..AC-5, AC-N1, AC-N2 pass under `-p slicer-core --features host-algos`; `cargo xtask check-literals` exit 0.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/prepass_slice.rs` - lines 197-256 (current `assemble_bridge_areas`)
  - `crates/slicer-ir/src/slice_ir.rs` - lines 599-693 (`BridgeRegion`, region fields)
  - `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - full (short)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/prepass_slice.rs`
  - `crates/slicer-core/tests/bridge_orientation_tdd.rs` (net-new)
  - `crates/slicer-core/Cargo.toml` (add `[[test]] name = "bridge_orientation_tdd"` with `required-features = ["host-algos"]`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (seam wiring is Step 2)
  - `crates/slicer-core/src/algos/mesh_analysis.rs` (retirement is Step 3)
  - `OrcaSlicerDocumented/**` (delegate only)
  - `crates/slicer-schema/wit/**`
- Blast-radius discipline: this step adds no struct field and no schema constant, so no struct-literal blast radius. The net-new flat test file auto-registers ONLY via the explicit `[[test]]` entry — without it the file compiles to zero tests under a bare narrow run (feature-gate blindness rule); every AC invocation therefore passes `--features host-algos`. Test literals constructing watched types (`BridgeRegion`) must use a `..` rest or an `// exhaustive:` waiver per `docs/21_data_defaults_and_fixtures.md`.
- Expected sub-agent dispatches:
  - Question: exact bodies of the inline `detect_bridging_direction(Lines, Polygons)` / `(Polygons, Polygons)` overloads (`BridgeDetector.hpp`) — candidate normals, quantization keys, cost accumulation order, perpendicular flip, PC minor-axis and `{1,0}` fallbacks — plus `compute_principal_components` zero-area/sort contracts (`PrincipalComponents2D.cpp`); scope: those two files; return: `SUMMARY` (≤200 words) + `LOCATIONS` (≤20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - direct read
  - `docs/specs/bridge-parity-plan.md` - §3/F2 bullets only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/PrincipalComponents2D.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask check-literals` - FACT exit code
- Exit condition: all eight port-semantics tests in the new binary pass; the function compiles standalone with no caller yet (wiring deliberately deferred).

### Step 2: Wire orientation into the post-gate seam

- Task IDs: none (backlog slot: ISSUE-84 `bridge_angle` half)
- Objective: Add `update_external_bridge_orientation(region: &mut SlicedRegion, lower_layer_slices: &[ExPolygon])` (writes `region.bridge_orientation_deg = detect_bridging_direction_deg(gated bridge_areas, raw lower contours)`; no-op on empty gated areas) and invoke it from `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) immediately after 234's `gate_bridge_areas_by_unsupported_span` call, using the same `prev_layer_boundaries.get(&global_layer_index)` lookup (map keyed by CURRENT global layer index holding previous-layer contours). Add AC-6's two tests to `bridge_orientation_tdd.rs`.
- Precondition: Step 1 exit condition met; 234's gate symbol exists in the tree (prerequisite packet implemented or co-landed in the same branch); the gate call site in `commit_shell_classification_builtin` is locatable by rg.
- Postcondition: `region.bridge_orientation_deg` derives from GATED geometry at runtime; AC-6 passes including its rg half; `cargo check --workspace --all-targets` stays green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - rg-locate `commit_shell_classification_builtin`, then read ±120 lines around it (file exceeds 300 lines)
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - rg-locate `prev_layer_boundaries` consumer only (keying confirmation)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/prepass_slice.rs` (add the region updater next to the pure function)
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (seam call)
  - `crates/slicer-core/tests/bridge_orientation_tdd.rs` (AC-6 tests)
- Files explicitly out of bounds:
  - `modules/core-modules/**` (module reads `bridge_orientation_deg()` unchanged)
  - `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`
  - `crates/slicer-core/src/algos/mesh_analysis.rs` (Step 3)
- Blast-radius discipline: adds one function parameter surface but NO struct field and NO schema constant — no struct-literal blast radius. The seam changes WHEN orientation is computed (post-slice vs during mesh analysis); any consumer reading `region.bridge_orientation_deg` between `PrePass::Slice` and `PrePass::ShellClassification` would observe a stale value — the Step 3 LOCATIONS dispatch enumerates them; if any such mid-window reader exists, record it here as a finding rather than silently reordering prepass.
- Expected sub-agent dispatches:
  - Question: does anything read `region.bridge_orientation_deg` (or the SDK view setter/getter) between `PrePass::Slice` commit and `commit_shell_classification_builtin`? scope: `crates/slicer-runtime/src/` + `crates/slicer-sdk/src/`; return: `LOCATIONS` (≤20 entries); purpose: ordering-safety proof for the seam
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY of the `ShellClassification` stage section only (file >300 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` - delegate; never load (call-site storage convention already captured Step 1)
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- orientation_written_from_gated_geometry empty_bridge_areas_leave_orientation_untouched` - FACT pass/fail
  - `rg -q 'update_external_bridge_orientation' crates/slicer-runtime/src/slice_postprocess_prepass.rs` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: AC-6 passes end-to-end (test + rg halves); workspace check clean.

### Step 3: Retire the heuristic, close blast radius, verify end-to-end guards

- Task IDs: none (backlog slot: ISSUE-84 `bridge_angle` half)
- Objective: Delete `compute_bridge_direction_deg` from `crates/slicer-core/src/algos/mesh_analysis.rs` (adjusting `compute_bridge_metrics`/`assemble_bridge_areas` write-through so `bridge_direction_deg` keeps feeding only surfaces still consumed), re-pin the heuristic-angle assertions in `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`, add AC-8's retirement checks, and run the end-to-end determinism/I2/I7 guard (AC-7) plus the guest-freshness gate.
- Precondition: Steps 1-2 exits met; LOCATIONS dispatch (below) has enumerated every literal-angle assertion and consumer before deletion.
- Postcondition: `rg 'fn compute_bridge_direction_deg' crates/slicer-core/src/algos/mesh_analysis.rs` finds nothing; `algo_mesh_analysis_tdd` green; AC-7 passes byte-identical reslices with uniform bridge feedrate; clippy/check gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mesh_analysis.rs` - lines 486-510 and 640-690 only (function + sole caller context)
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` - rg-locate the assertion sites from the LOCATIONS result, read ±60 lines each (file >300 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`
  - `crates/slicer-core/tests/bridge_orientation_tdd.rs` (AC-8 structural companion test, if expressed as a test)
- Files explicitly out of bounds:
  - `modules/core-modules/**`, `crates/slicer-sdk/**` (view plumbing unchanged)
  - `target/test-output.log` re-invocations (read the log instead)
- Blast-radius discipline (mandatory): deleting a function consumed by watched-type construction sites shifts literal values, not shapes — but the step MUST first dispatch:
  - Question: every caller of `compute_bridge_direction_deg` and every test/golden/visual-debug site asserting literal `bridge_direction_deg`/`bridge_orientation_deg` values or snapshotting direction-bearing captures; scope: `crates/*/src` + `crates/*/tests` + `modules/*/src`; return: `LOCATIONS` (≤20 entries); cite the result inline here before editing
  Watched-type literals touched while re-pinning need `..` rest or `// exhaustive:` waiver; `cargo xtask check-literals` runs after edits.
- Expected sub-agent dispatches: the LOCATIONS sweep above; plus
  - Question: confirm `detect_angle`'s distinguishing markers (angle step constant, coverage cost, spacing tie-break) in `BridgeDetector.cpp` so AC-N2's rg patterns name real sweep constructs, and confirm the ACTIVE call path (`LayerRegion::process_external_surfaces`) selects the inline `detect_bridging_direction` overload rather than `detect_angle`; scope: `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` + `LayerRegion.cpp`; return: `SNIPPETS` (≤30 lines); purpose: AC-N2 grounding
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/bridge-parity-plan.md` - §6 invariant list only (I3/I7 wording)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` - delegate; never load
- Verification:
  - `bash -c 'rg -q "fn compute_bridge_direction_deg" crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 || exit 0'` - FACT exit code (AC-8 structural half)
  - `bash -c 'rg -q "detect_angle|angle_step|for angle in" crates/slicer-core/src/algos/prepass_slice.rs crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 || exit 0'` - FACT exit code (AC-N2 structural half)
  - `bash -c 'rg -q "fn compute_bridge_direction_deg" crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 || exit 0' && cargo test -p slicer-core --features host-algos --test algo_mesh_analysis_tdd` - FACT pass/fail (AC-8 structural + test halves, single command)
  - `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- rotated_cross_rejects_legacy_five_degree_snap equal_cost_tie_resolves_smallest_quantized_angle` - FACT pass/fail
  - `cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_a.gcode --module-dir modules/core-modules && cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_b.gcode --module-dir modules/core-modules && cmp target/orient_a.gcode target/orient_b.gcode` then the AC-7 python parser - FACT pass/fail + printed counts
  - `cargo xtask build-guests --check` - FACT exit code (0 fresh expected after rebuild; 3 = wasm-tools infra error, not staleness)
- Exit condition: all listed verifications pass; `target/test-output.log` shows no heuristic-dependent failures.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Port fidelity + numeric fixtures; largest risk is misreading the perpendicular flip |
| Step 2 | S | One function + one call site + two tests |
| Step 3 | M | Retirement fallout + end-to-end guards |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- No `docs/07_implementation_status.md` update is required — ISSUE-84's `bridge_angle` half has no docs/07 TASK row (the backlog slot is the issue file itself). If implementation chooses to note progress in `docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md`, that status edit belongs to implementation, not to this gate (`counterbore_hole_bridging` remains open there regardless).
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` gate invocations must use `--all-targets` so the test, bench, and example targets compile; `cargo test` runs stay narrow (single `-p <crate> --test <binary>`), with whole-suite runs only via `cargo xtask test --summary` when a gate requires them.
