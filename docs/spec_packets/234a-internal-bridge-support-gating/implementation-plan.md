# Implementation Plan: 234a-internal-bridge-support-gating

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (none — backlog slot is ISSUE-82's internal-bridge filtering half).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Guest freshness: after any slicer-core edit, `cargo xtask build-guests --check` exit code arbitrates (0 fresh / 1 stale → rebuild / 3 wasm-tools infra) before attributing guest-touching failures elsewhere.

## Steps

### Step 1: Pure support-math port + Q1/Q2 discovery

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Add `unsupported_span_areas(lower_fills, lower_solids, spacing_mm, expansion_multiplier)` and `qualify_internal_bridge_surface(surface, unsupported, spacing_mm, nofilter)` to `crates/slicer-core/src/algos/bridge_over_infill.rs` implementing the canonical arithmetic in design.md (closing of lower fills; shrink `mult*spacing`; minus lower solids shrunk 1*spacing then expanded `(1+mult)*spacing`; per-surface intersection; empty + `9*spacing^2` partial gates; `expand(4*spacing)` clip; leftover remerge `spacing^2 < area < 12*spacing^2`) — plus net-new `crates/slicer-core/tests/bridge_support_gating_tdd.rs` with AC-1..AC-3 polygon-primitive fixtures — PLUS resolve Q1/Q2 via the design.md dispatches and record answers in design.md Open Questions.
- Precondition: prerequisite symbols verified present (`determine_bridging_angle`, `construct_anchored_polygon`, `InfillRegion.internal_bridge_infill`, `build_region_timelines`, `gate_bridge_areas_by_unsupported_span` call site); canonical gather-lambda SNIPPETS dispatch returned.
- Postcondition: three gating tests pass under `-p slicer-core --features host-algos`; Q1 names the candidate field with writer evidence; Q2 names the mechanism (area-subtract vs path-replace) with consumer evidence; both recorded in design.md.
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
- Exit condition: AC-1..AC-3 green; Q1/Q2 answers written into design.md Open Questions verbatim; no runtime files touched.

### Step 2: Relocate construction into the sequential prepass

- Task IDs: none (backlog slot ISSUE-82)
- Objective: Delete the InfillPostProcess construction block from `crates/slicer-runtime/src/layer_executor.rs`; add the relocated pass to `commit_shell_classification_builtin`'s stage in `crates/slicer-runtime/src/slice_postprocess_prepass.rs` strictly after 234's `gate_bridge_areas_by_unsupported_span` invocation — iterate region timelines, qualify layer-L surfaces against committed L-1 using Step-1 functions, construct anchored lines via `construct_anchored_polygon` with flow values from the same ConfigView keys (`bridge_line_width`, `internal_bridge_flow`, `nozzle_diameter`, `internal_bridge_angle`), populate `InfillRegion.internal_bridge_infill` (confirming the committed infill artifact is reachable at ShellClassification and mirroring the existing commit-key pattern; if it is NOT reachable, STOP and report — that is a stage-boundary blocker, not a local choice), apply the Q2-decided sparse-material mechanism, honour the `dont_filter_internal_bridges` mapping (false = full gates, true = bypass area/partial gate). Add the runtime unit fixture for AC-N1.
- Precondition: Step 1 exit met; Q1/Q2 answers recorded in design.md.
- Postcondition: old arm constructs nothing (AC-4 rg half); prepass pass present after the gate (AC-4 rg half); AC-N1 green; wedge e2e still green (AC-6) or re-pins justified per design.md Risks.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - rg-locate InfillPostProcess arm, read ±120 lines
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - rg-locate `commit_shell_classification_builtin` and the gate call, read ±120 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - `crates/slicer-runtime/tests/unit/internal_bridge_support_gating_tdd.rs` (net-new; registered via the unit aggregator main.rs)
- Files explicitly out of bounds:
  - `modules/core-modules/**`, `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: removing the block may orphan imports/helpers in layer_executor.rs — compile-clean with `cargo check --workspace --all-targets` before proceeding. Any watched-type literal edits need FRU/waiver.
- Expected sub-agent dispatches: none required beyond Step 1's; if wedge assertions break, one FACT dispatch identifying the exact shifted assertion before any re-pin.
- Context cost: `S`
- Authoritative docs: `docs/04_host_scheduler.md` stage-ordering SUMMARY (if not already captured in Step 1).
- OrcaSlicer refs: none new.
- Verification:
  - AC-4's two rg halves + `cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd && cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd` - FACT
  - `cargo test -p slicer-runtime --test e2e wedge_linked_infill_report_tdd` - FACT (AC-6)
  - `cargo test -p slicer-runtime --test unit -- internal_bridge_support_gating_tdd::fully_supported_pair_emits_no_internal_bridges` - FACT (AC-N1)
  - `cargo check --workspace --all-targets` - FACT
- Exit condition: all listed verifications pass; no module files touched.

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
  - `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs` (only under the recalibration condition above)
- Files explicitly out of bounds:
  - `tmp/**` (session-local evidence stays uncommitted), `modules/core-modules/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: new [[test]]-free e2e registration goes through the existing aggregator main.rs mod list — confirm registration so the binary-count cannot silently drop. Watched-type literals need FRU/waiver.
- Expected sub-agent dispatches: one FACT for doc-impact greps (`dont_filter_internal_bridges` row updated in docs/15; `234a` hit in bridge-parity-plan.md).
- Context cost: `M`
- Authoritative docs: `docs/15_config_keys_reference.md` entry edit is Doc Impact, executed here.
- OrcaSlicer refs: none new.
- Verification:
  - AC-5 command exactly as written in packet.spec.md - FACT + printed counts
  - `cargo clippy --workspace --all-targets -- -D warnings` / `cargo check --workspace --all-targets` / `cargo xtask check-literals` / `cargo xtask build-guests --check` - FACT each
  - Doc-impact grep FACTs listed above
- Exit condition: AC-5/AC-6/AC-N1 green; gates clean; doc impact greps return hits.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Math fidelity + discovery dispatches |
| Step 2 | S | Deletion + prepass pass + unit fixture |
| Step 3 | M | Model import, e2e, gates |

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
