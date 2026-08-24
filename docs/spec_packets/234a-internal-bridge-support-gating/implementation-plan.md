# Implementation Plan: 234a-internal-bridge-support-gating

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (none — backlog slot is ISSUE-82's internal-bridge filtering half).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Guest freshness: after any slicer-core edit, `cargo xtask build-guests --check` exit code arbitrates (0 fresh / 1 stale → rebuild / 3 wasm-tools infra) before attributing guest-touching failures elsewhere.

## Steps

### Step 1: Pure support-math port + Q1/Q2 discovery

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Add `unsupported_span_areas(lower_fills, lower_solids, spacing_mm, expansion_multiplier)` and `qualify_internal_bridge_surface(surface, unsupported, spacing_mm, nofilter)` to `crates/slicer-core/src/algos/bridge_over_infill.rs` implementing the canonical arithmetic in design.md (closing of lower fills; shrink `mult*spacing`; minus lower solids shrunk 1*spacing then expanded `(1+mult)*spacing`; per-surface intersection; empty + `9*spacing^2` partial gates; `expand(4*spacing)` clip; leftover remerge `spacing^2 < area < 12*spacing^2`) — plus net-new `crates/slicer-core/tests/bridge_support_gating_tdd.rs` with AC-1..AC-3 AND AC-N1 (`fully_supported_surface_qualifies_nothing`) polygon-primitive fixtures — PLUS resolve Q1/Q2 via the design.md dispatches and record answers in design.md Open Questions.
- Precondition: prerequisite symbols verified present (`determine_bridging_angle`, `construct_anchored_polygon`, `InfillRegion.internal_bridge_infill`, `build_region_timelines`, `gate_bridge_areas_by_unsupported_span` call site); canonical gather-lambda SNIPPETS dispatch returned.
- Postcondition: all `bridge_support_gating_tdd` tests pass under `-p slicer-core --features host-algos` (AC-1..AC-3 and AC-N1); Q1 names the candidate field with writer evidence; Q2 names the mechanism (area-subtract vs path-replace) with consumer evidence; both recorded in design.md.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - full (existing fns are the neighbours)
  - `crates/slicer-ir/src/slice_ir.rs` - lines ~1480-1540 only
  - delegated reads only for everything else per design.md dispatches
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/bridge_over_infill.rs`
  - `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (net-new)
  - `crates/slicer-core/Cargo.toml` ([[test]] entry with `required-features = ["host-algos"]`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (Step 2), `modules/core-modules/**`, `crates/slicer-schema/wit/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: new pure functions add no fields/constants — no struct-literal blast radius. Test literals constructing watched types use `..` rest or waiver.
- Expected sub-agent dispatches: the two design.md LOCATIONS dispatches (Q1 writers; Q2 consumers) + the PrintObject.cpp SNIPPETS dispatch if not already satisfied at authoring.
- Context cost: `M`
- Authoritative docs: `docs/specs/bridge-parity-plan.md` §3/F3 only (direct); `docs/04_host_scheduler.md` ShellClassification section (delegated SUMMARY).
- OrcaSlicer refs: `PrintObject.cpp` (delegate; never load)
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask build-guests --check` - FACT exit code (expected 0)
- Exit condition: AC-1..AC-3 and AC-N1 green; Q1/Q2 answers written into design.md Open Questions verbatim; no runtime files touched.

### Step 2: Relocate construction into the sequential prepass (Option-A revision)

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Add net-new host-only field `pub internal_bridge_lines: Vec<Vec<Point2>>` to `SlicedRegion` in `crates/slicer-ir/src/slice_ir.rs` (derives Default; un-mirrored — do NOT touch `SliceRegionView::from_ir`); update the ONE exhaustive production literal (`crates/slicer-core/src/algos/prepass_slice.rs` ~1085, `execute_prepass_slice_single_layer_impl`) explicitly, and convert broken test literals to FRU. Add the relocated pass to `commit_shell_classification_builtin`'s stage in `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, ordered AFTER the shell-classification passes (verify `top_solid_fill` is populated for every layer by that point — if not, STOP and report) and strictly after 234's `gate_bridge_areas_by_unsupported_span` invocation: iterate region timelines, qualify each region's upper-layer `top_solid_fill` surfaces against committed L-1 (`lower_fills` = lower-layer `region.polygons`; `lower_solids` = lower-layer top+bottom solid fills) using Step 1's `unsupported_span_areas` + `qualify_internal_bridge_surface`; resolve flow/spacing/angle/nofilter via `region_map.config_for(&key)` keys `bridge_line_width`, `internal_bridge_flow`, `nozzle_diameter`, `internal_bridge_angle`, `dont_filter_internal_bridges` (false = full gates, true = bypass area/partial gate); construct anchored lines via `construct_anchored_polygon` (anchors from current-region `polygons`/`infill_areas` contours); write centerlines into `region.internal_bridge_lines` and extend `region.bridge_areas` with the qualified polygons. Reduce the InfillPostProcess arm in `crates/slicer-runtime/src/layer_executor.rs` to a pure emitter: map `slice_region.internal_bridge_lines` → `ExtrusionRole::InternalBridgeInfill` `ExtrusionPath3D`s (z = `slice.z`, width = `flow.thread_diameter_mm` via `ctx.config_view`) into `region.internal_bridge_infill`; DELETE anchor-gathering, candidate_voids, `construct_anchored_polygon`/`determine_bridging_angle` calls, the sliver/dont-filter gate logic, and the post-hoc `sparse_infill_area` subtraction.
- Precondition: Step 1 exit met (landed); Q1/Q2 answers recorded in design.md (done); verified scheduler facts in design.md Architecture Constraints.
- Postcondition: old arm constructs nothing (AC-4 rg half); prepass pass present after the gate (AC-4 rg half); wedge e2e green UNCHANGED (AC-6); any wedge assertion drift is a STOP-and-report blocker per design.md Risks.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - rg-locate InfillPostProcess arm, read ±120 lines
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - rg-locate `commit_shell_classification_builtin`, the shell passes, and the gate call, read ±120 lines
  - `crates/slicer-ir/src/slice_ir.rs` - lines ~1450-1560 only
  - `crates/slicer-core/src/algos/prepass_slice.rs` - rg-locate the SlicedRegion literal (~1085), read ±60 lines
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` - rg-locate exhaustive literals (:746/:856/:1088), ±20 lines each
- Files allowed to edit (at most 6):
  - `crates/slicer-ir/src/slice_ir.rs` (the one field only)
  - `crates/slicer-core/src/algos/prepass_slice.rs` (the one literal only)
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (FRU fixes only)
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - any additional file ONLY where `cargo check --workspace --all-targets` proves another exhaustive-literal break — fix minimally with FRU or explicit field, note it in the return
- Files explicitly out of bounds:
  - `modules/core-modules/**`, `crates/slicer-schema/**`, `OrcaSlicerDocumented/**`, `crates/slicer-sdk/**`
- Blast-radius discipline: production literals stay EXHAUSTIVE (extend `prepass_slice.rs:1085` explicitly per docs/21); test literals use FRU. Compile-clean with `cargo check --workspace --all-targets` before running suites. After slicer-ir/slicer-core edits run `cargo xtask build-guests --check`; if stale (exit 1) rebuild guests before attributing failures elsewhere.
- Expected sub-agent dispatches: none required; if the wedge e2e shifts, one FACT dispatch identifying the exact shifted assertion BEFORE anything else — then STOP and report per the postcondition.
- Context cost: `M`
- Authoritative docs: `docs/04_host_scheduler.md` stage-ordering facts already captured in design.md Architecture Constraints.
- OrcaSlicer refs: none new.
- Verification:
  - AC-4's two rg halves + `cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd && cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd` - FACT
  - `cargo test -p slicer-runtime --test e2e wedge_linked_infill_report_tdd` - FACT (AC-6)
  - `cargo check --workspace --all-targets` - FACT
  - `cargo xtask build-guests --check` - FACT exit code
- Exit condition: all listed verifications pass with NO wedge assertions changed; `internal_bridge_lines` populated by prepass and consumed by arm (proven by AC-5 in Step 3); any wedge drift is a STOP-and-report blocker, not a local re-pin.

### Step 3: Calicat regression surface + blast radius + gates

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Import the measured model as `resources/calicat.stl` (from this packet's authoring-session artifact; binary import, no regeneration); write `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (registered through the e2e aggregator) slicing twice, asserting byte-identity, Internal-Bridge layer count ≤6, total bridge-labelled extrusion ≤5000 mm, and Z≈3.2 external angle ∈ [85°,95°] using M83-relative-E/Z-keyed parsing mirrored from the existing e2e parsing helpers; run the full verification ladder including clippy/check/check-literals/build-guests; recalibrate `no_linker_module_degraded_raw_output_tdd` ONLY if legitimately shifted (both sides re-measured, numbers in-comment).
- Precondition: Step 2 exits green.
- Postcondition: AC-5 green on the committed model; all gates clean; closure suite run once at ceremony.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/` existing parsing helpers (rg-locate, ±60 lines each)
- Files allowed to edit (at most 3):
  - `resources/calicat.stl` (imported binary)
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` (net-new)
  - `crates/slicer-runtime/tests/e2e/main.rs` (aggregator registration for the net-new test)
- Files explicitly out of bounds:
  - `tmp/**` (session-local evidence stays uncommitted), `modules/core-modules/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: the aggregator registration is MANDATORY — an unregistered file compiles to zero tests and looks green (known trap). Watched-type literals need FRU/waiver.
- Conditional follow-up OUTSIDE this packet's edit contract: if `no_linker_module_degraded_raw_output_tdd` legitimately shifts, recalibrate in a SEPARATE commit with both sides freshly measured and documented in-comment — never inside this packet's commits.
- Expected sub-agent dispatches: none new.
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none new.
- Verification:
  - AC-5 command exactly as written in packet.spec.md - FACT + printed counts
  - `cargo clippy --workspace --all-targets -- -D warnings` / `cargo check --workspace --all-targets` / `cargo xtask check-literals` / `cargo xtask build-guests --check` - FACT each
- Exit condition: AC-5 and AC-6 green; gates clean.

### Step 4: Doc Impact edits

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Apply the two Doc Impact edits from packet.spec.md — update the `dont_filter_internal_bridges` semantics row in `docs/15_config_keys_reference.md` (false = canonical full filtering of internal bridges by lower-layer support, true = bypass) and append the dated F3 addendum row to `docs/specs/bridge-parity-plan.md` pointing at this packet with the authoring-session measurements.
- Precondition: Step 3 exits green.
- Postcondition: both Doc Impact verification greps return hits.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - rg-locate the key row, read ±30 lines
  - `docs/specs/bridge-parity-plan.md` - §3/F3 only
- Files allowed to edit (at most 2):
  - `docs/15_config_keys_reference.md`
  - `docs/specs/bridge-parity-plan.md`
- Files explicitly out of bounds: everything else.
- Blast-radius discipline: doc-only edits; no code targets recompile.
- Expected sub-agent dispatches: one FACT re-running both Doc Impact greps after the edits.
- Context cost: `S`
- Authoritative docs: the two files being edited.
- OrcaSlicer refs: none new.
- Verification:
  - `rg -n "dont_filter_internal_bridges" docs/15_config_keys_reference.md` shows updated semantics - FACT
  - `rg -n "234a" docs/specs/bridge-parity-plan.md` returns a hit - FACT
- Exit condition: both greps hit; no other doc content changed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Math fidelity + discovery dispatches + AC-N1 fixture (LANDED) |
| Step 2 | M | Option-A field + prepass pass + arm reduction |
| Step 3 | M | Model import, e2e, gates |
| Step 4 | S | Doc Impact edits |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete; every pipe-suffixed AC command returned PASS.
- Q1/Q2 answers present in design.md Open Questions.
- Doc Impact greps verified by dispatched FACT.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace` ONCE (closure requirement), dispatched, FACT return.
- Record remaining packet-local risk.

All `cargo check`/`clippy` gate invocations must use `--all-targets`; whole-suite runs only via `cargo xtask test --summary` at ceremony.
