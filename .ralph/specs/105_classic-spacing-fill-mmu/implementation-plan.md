# Implementation Plan: 105_classic-spacing-fill-mmu

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: T-P96-A0 — OrcaSlicer-source investigation one-pager

- Task IDs:
  - `T-P96-A0` — Produce `docs/specs/orca-mmu-perimeter-investigation.md`
- Objective: dispatch the OrcaSlicer SUMMARY for the MMU per-color outer-wall fragmentation; author a one-pager that cites file:line references and confirms that OrcaSlicer uses partition/both-trace (Model A) — each per-color region traces its own outer wall independently — with no shared-bisector skip mask.
- Precondition: workspace builds clean.
- Postcondition: T-P96-A0 deliverable grep passes; one-pager committed.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — read full.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read "Inherited from P96" section.
- Files allowed to edit (≤ 3):
  - `docs/specs/orca-mmu-perimeter-investigation.md` (NET-NEW — does not exist pre-packet; created by this step; all ACs/greps referencing it are sequenced after this step's exit condition)
- Files explicitly out-of-bounds for this step:
  - All source files.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp + PerimeterGenerator.cpp per-color branches for the MMU outer-wall fragmentation; return SUMMARY ≤ 200 words. No code. Specifically: does each per-color region trace its own independent outer wall (partition/both-trace, Model A), or is there a shared-bisector skip-mask mechanism (Model B)? Cite file:line references."
- Context cost: `S` (one new doc; SUMMARY-only dispatch)
- Authoritative docs: see Files allowed to read.
- OrcaSlicer refs: `MultiMaterialSegmentation.cpp`, `PerimeterGenerator.cpp` per-color branches (delegate SUMMARY).
- Verification:
  - `rg -q 'Model A\|per-color\|independent' docs/specs/orca-mmu-perimeter-investigation.md` — exit 0.
- Exit condition: one-pager exists with file:line citations confirming Model A (independent per-color tracing; no skip mask).

### Step 2: T-062b — IR enum additions

- Task IDs:
  - `T-062b` — Add `LoopType::GapFill` + `ExtrusionRole::GapFill` variants
- Objective: extend `LoopType` and `ExtrusionRole` with `GapFill` arm, mark both `#[non_exhaustive]`, bump schema from live `4.3.0` to `4.4.0`; mirror in WIT (`wall-loop-type` in `ir-types.wit`, `extrusion-role` in `types.wit`). Update every exhaustive match site in the workspace to add the new arm. NOTE: `ir_to_wit_extrusion_role` in `leaf.rs:183` is an exhaustive match — the WIT `gap-fill` arm on `extrusion-role` and the `leaf.rs` match arm MUST land in the same sub-step 2a to avoid a mid-step build break.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: `cargo xtask build-guests --check` no STALE; all exhaustive matches compile.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'LoopType|ExtrusionRole|SlicedRegion|CURRENT_SLICE_IR_SCHEMA_VERSION'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full file.
  - `crates/slicer-wasm-host/src/host.rs` — range-read by `rg -n 'SliceRegionData|sliced_region_to_data'`.
  - `crates/slicer-sdk/src/views.rs` — range-read by `rg -n 'fn bridge_areas\|fn nonplanar_surface'`.
- Files allowed to edit (≤ 3 per sub-step):
  - 2a (IR + WIT): `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit` + `crates/slicer-schema/wit/deps/types.wit` (both WIT files need edits: `wall-loop-type` in ir-types.wit, `extrusion-role` in types.wit), `crates/slicer-wasm-host/src/marshal/leaf.rs` (add `ExtrusionRole::GapFill` arm to the exhaustive `ir_to_wit_extrusion_role` match at line 183 — MUST be atomic with the WIT and IR additions).
  - 2b (downstream match arms): the LOCATIONS dispatch reports specific files; expect `modules/core-modules/part-cooling/src/lib.rs`, GCodeEmit role priority table, possibly `path-optimization-default`. Each consumer gets a 1-3 line arm addition.
- Files explicitly out-of-bounds for this step:
  - Any perimeter module `lib.rs` (Step 4+ work).
  - `slicer-core` perimeter_utils / flow modules (Step 3+ work).
  - `paint_segmentation/` (unchanged by this packet beyond Step 7).
- Expected sub-agent dispatches:
  - "Find all exhaustive `match` blocks on `LoopType` across the workspace; return LOCATIONS ≤ 20 entries."
  - "Find all exhaustive `match` blocks on `ExtrusionRole` across the workspace; return LOCATIONS ≤ 20 entries."
  - "Run `cargo build --tests --workspace`; return FACT (pass/fail) — catches WIT type identity break + missing match arms."
  - "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)."
- Context cost: `M` (three crates + downstream match arms; two LOCATIONS dispatches)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for `LoopType`, `ExtrusionRole`, `SlicedRegion`.
  - `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist".
  - `CLAUDE.md` — §"WIT/Type Changes Checklist" + §"Guest WASM Staleness".
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'LoopType::GapFill' crates/slicer-ir/src/slice_ir.rs && rg -q 'ExtrusionRole::GapFill' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `rg -q 'gap-fill' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'gap-fill' crates/slicer-schema/wit/deps/types.wit` — exit 0 (both WIT files updated).
  - `rg -q 'GapFill' crates/slicer-wasm-host/src/marshal/leaf.rs` — exit 0 (leaf.rs match arm added atomically).
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT.
  - `cargo xtask build-guests --check` — no STALE.
- Exit condition: IR additions present, workspace compiles end-to-end, no STALE guests.

### Step 3: DROPPED (Model A)

**DROPPED (Model A).** T-P96-C0 retired — Model A needs no host-side bisector mask. See rewritten ADR-0013 and D-105-BISECTOR-MASK-DROPPED. The prior draft of `compute_bisector_edge_skip_mask`, the `bisector_edge_skip_mask` field, WIT/view accessors (`bisector-edge-skip-mask`), `prepass_slice.rs` initializer, and `paint_segmentation_bisector_mask_tdd.rs` are all removed in this packet — no new code is added in this former step. This heading is retained so steps 4–8 numbering is unchanged.

### Step 4: T-050/T-051/T-052/T-053 — Spacing model + outer/inner widths

- Task IDs:
  - `T-050` — Flow math in `slicer-core::flow`
  - `T-051` — outer/inner widths replacing single line_width
  - `T-052` — `ext_perimeter_spacing2` + `perimeter_spacing` arithmetic
  - `T-053` — `precise_outer_wall` mode (gated)
- Objective: add `slicer_core::flow` module; register the four config keys; rewrite the wall-inset computation in both perimeter modules to use distinct outer/inner widths and the canonical spacing formula.
- Precondition: Step 2 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-1 verification command passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs` — range-read the `run_perimeters` body and the wall-inset loop.
  - `docs/01_system_architecture.md` — §"Crate Boundaries" full.
- Files allowed to edit (≤ 3 per sub-step):
  - 4a (helpers): `crates/slicer-core/src/flow.rs` (NEW), `crates/slicer-core/src/lib.rs` (mod declaration), `crates/slicer-core/Cargo.toml` (add `[[test]] name = "flow_tdd"` entry).
  - 4b (manifests + flow test): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `crates/slicer-core/tests/flow_tdd.rs` (NEW; spacing-formula unit test).
  - 4c (consumers + integration test): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`, `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` (NEW). ALSO register in `crates/slicer-runtime/tests/integration/main.rs`: add `mod outer_inner_width_and_spacing_tdd;` and update `docs/15_config_keys_reference.md`.
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (Step 2 closed the IR work).
  - Thin-wall / gap-fill code paths (Step 6).
  - Wall-sequence code paths (Step 5).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/Flow.cpp for `Flow::new_from_width_height` math; return SUMMARY ≤ 100 words."
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644 for ext_perimeter_spacing2 + precise_outer_wall gating; return SUMMARY ≤ 150 words."
  - "Run `cargo test -p slicer-core --test flow_tdd`; return FACT pass/fail."
  - "Run `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd`; return FACT pass/fail."
- Context cost: `M` (largest step — three sub-steps + two OrcaSlicer SUMMARYs + new tests)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-050/T-051/T-052/T-053 rows.
  - `docs/01_system_architecture.md`.
- OrcaSlicer refs: `Flow.cpp`, `PerimeterGenerator.cpp:1501-1506,1644` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-core --test flow_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1 green; spacing measured between walls matches `ext_perimeter_spacing2 = (outer + inner) / 2` and `perimeter_spacing = inner`.

### Step 5: T-054/T-054b/T-054c — wall_sequence migration + modes

- Task IDs:
  - `T-054` — Register `wall_sequence` in perimeter manifests; deregister from `path-optimization-default`
  - `T-054b` — Implement `OuterInner` + `InnerOuter` modes in `wall_sequence_reorder`
  - `T-054c` — Implement `InnerOuterInner` sandwich mode
- Objective: migrate `wall_sequence` config registration per ADR-0011; implement all three modes in `slicer_core::perimeter_utils::wall_sequence_reorder`; call from both perimeter modules; in-module wall tree built during generation and discarded after reorder.
- Precondition: Step 4 exit condition met.
- Postcondition: AC-2 verification command passes for all three modes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/perimeter_utils.rs` — full (post-P102).
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`.
  - `modules/core-modules/path-optimization-default/src/lib.rs` — range-read lines 46-51 (existing `WallSequence` enum), 143-165 (struct field + match), 276-295 (config-read parse); these call sites must migrate to `slicer_core::perimeter_utils::WallSequence`.
  - Both perimeter modules' `lib.rs` (`run_perimeters` body).
- Files allowed to edit (≤ 3 per sub-step):
  - 5a (helper): `crates/slicer-core/src/perimeter_utils.rs` (add `WallSequence` enum with all 3 variants + `wall_sequence_reorder`), `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` (NEW), `crates/slicer-core/Cargo.toml` (add `[[test]] name = "wall_sequence_reorder_tdd"` entry).
  - 5b (config migration + WallSequence migration): `modules/core-modules/path-optimization-default/path-optimization-default.toml` (deregister key), `modules/core-modules/path-optimization-default/src/lib.rs` (remove local `WallSequence` def; use `slicer_core::perimeter_utils::WallSequence`; update import and match to add `InnerOuterInner` arm), `modules/core-modules/classic-perimeters/classic-perimeters.toml` (register `wall_sequence`).
  - 5c (remaining manifests + consumers): `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (register `wall_sequence`), `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (no IR change in this step).
  - Thin-wall / gap-fill / MMU code paths.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913 for wall_sequence reorder including InnerOuterInner sandwich; return SUMMARY ≤ 200 words, no code."
  - "Run `cargo test -p slicer-core --test wall_sequence_reorder_tdd`; return FACT pass/fail per mode."
- Context cost: `M` (helper + manifests + two-module consumer)
- Authoritative docs:
  - `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — read full.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-054/b/c rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:1801-1913` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-core --test wall_sequence_reorder_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'wall_sequence' modules/core-modules/path-optimization-default/path-optimization-default.toml` — exit 1 (key deregistered).
- Exit condition: AC-2 green; `wall_sequence` registered only in perimeter manifests.

### Step 6: T-060/T-061/T-062/T-063/T-064/T-065 — Thin-walls + gap-fill emission

- Task IDs:
  - `T-060` — Register `detect_thin_wall`
  - `T-061` — Thin-wall detection cascade
  - `T-062` — ThinWall emission as WallLoop
  - `T-063` — Gap collection per-inset
  - `T-064` — Gap-fill emission as WallLoop{GapFill}
  - `T-065` — Register `gap_infill_speed` + `filter_out_gap_fill`
- Objective: register the three config keys; implement the thin-wall + gap-fill code paths in both perimeter modules using `slicer_core::medial_axis` + `offset2_ex` + `opening_ex` from P103.
- Precondition: Step 5 exit condition met.
- Postcondition: AC-3, AC-N1, AC-4, AC-N2 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs`.
  - `crates/slicer-core/src/medial_axis.rs` (from P103) — confirm signature.
- Files allowed to edit (≤ 3 per sub-step):
  - 6a (manifests): both perimeter `.toml`, `docs/15_config_keys_reference.md`.
  - 6b (thin-wall consumer): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`, `crates/slicer-runtime/tests/integration/thin_wall_emission_tdd.rs` (NEW). Register `mod thin_wall_emission_tdd;` in `crates/slicer-runtime/tests/integration/main.rs` (counts as 4th file — split to separate sub-step if it busts the ≤3 cap; see note below).
  - 6c (gap-fill consumer): same two `lib.rs` (re-edit), `crates/slicer-runtime/tests/integration/gap_fill_emission_tdd.rs` (NEW). Register `mod gap_fill_emission_tdd;` in `main.rs`.
  - NOTE: `main.rs` aggregator registration (adding two `mod` lines) can be batched into a single edit across 6b+6c since both target the same file; count as 1 file-edit within the sub-step that last touches it.
- Files explicitly out-of-bounds for this step:
  - `slicer-core` (medial_axis / offset2_ex already exist from P103; no new additions in this step).
  - MMU code paths (Step 7).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609 + 1665-1670,1930-1958 for thin-wall + gap-fill cascades; return SUMMARY ≤ 200 words."
  - "Run `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd gap_fill_emission_tdd`; return FACT pass/fail per case."
- Context cost: `M` (two-module rewrites + two new integration tests)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 6 rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:1596-1609,1665-1670,1930-1958` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-3 + AC-N1 + AC-4 + AC-N2 green.

### Step 7: T-P96-B — Remove external_contour union-trace consumption (Model A)

- Task IDs:
  - `T-P96-B` — Remove `external_contour` union-trace consumption in BOTH perimeter modules → per-color fragmentation (Model A)
- Objective: remove `external_contour` call sites from both perimeter modules. For arachne: delete the `by_object` shared-boundary branch so arachne also uses per-cell `emit_outer=true` (per-color independent outer-wall tracing). For classic: verify already correct (no union trace), confirm only. No mask consumption — Model A needs none. Single-color baseline unchanged.
- Precondition: Step 6 exit condition met.
- Postcondition: AC-6 + AC-N1/N2 verification commands pass; `external_contour()` call sites absent from both modules.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs` — range-read the per-cell trace loop.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/mmu_per_color_fragmentation_tdd.rs` (NEW). Register `mod mmu_per_color_fragmentation_tdd;` in `crates/slicer-runtime/tests/integration/main.rs` — batch this edit with one of the two `lib.rs` edits (counts as 1 of the 3 files).
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (no mask field to add; `external_contour` IR field stays until P107 T-P96-D — only consumption removed).
  - `slicer-core/paint_segmentation/` (no mask computation in this packet; see D-105-BISECTOR-MASK-DROPPED).
  - `slicer-core` flow/perimeter_utils modules (no change in this step).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration mmu_per_color_fragmentation_tdd`; return FACT pass/fail per case (4-color cube test + single-color baseline test)."
  - "Find call sites of `region.external_contour()` in the perimeter modules; return LOCATIONS ≤ 5 entries (expected zero after removal)."
- Context cost: `M` (two-module edit + new integration test with 4-color fixture)
- Authoritative docs:
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` (rewritten — confirms Model A).
- OrcaSlicer refs: cited in Step 1's one-pager.
- Verification:
  - `cargo test -p slicer-runtime --test integration mmu_per_color_fragmentation_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `! rg -q '\.external_contour\(\)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 (removal complete).
- New test assertions (`mmu_per_color_fragmentation_tdd`):
  - Per-color outer-wall extrusion-sequence count per layer equals the number of distinct colors present in that layer.
  - Each color fragment is preceded by a `T<N>` tool-change command in the emitted G-code sequence.
  - Neither module contains a call to `external_contour()` after this step.
  - Single-color baseline: extrusion count unchanged from pre-P105 baseline.
- Protected executor test reshape: `cube_4color_per_layer_per_color_fragmentation_with_tool_changes` (reshaped in P105, not deferred). G-code SHA re-baselined as `P105_CUBE_4COLOR_PARITY_SHA`.
- Exit condition: AC-6 green; no external_contour calls remaining in module code; 4-color fixture per-color fragmentation count confirmed.

### Step 8: Doc impact landing

- Task IDs:
  - Doc impact for the whole packet (covers T-062b, T-P96-A0/B, T-050..T-065, T-054*).
- Objective: land doc impact statement edits.
- Precondition: Step 7 exit condition met.
- Postcondition: all five Doc Impact Statement greps return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/02_ir_schemas.md` — range-read sections being edited.
  - `docs/01_system_architecture.md` — §"Crate Boundaries" full.
  - `docs/15_config_keys_reference.md` — range-read.
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`, `docs/01_system_architecture.md`, `docs/15_config_keys_reference.md`.
- Files explicitly out-of-bounds for this step:
  - All source files.
- Expected sub-agent dispatches:
  - "For each grep in the Doc Impact Statement, run `rg -q` on the listed path; return FACT pass/fail per grep."
- Context cost: `S` (three doc edits)
- Authoritative docs: the three edited files.
- OrcaSlicer refs: none.
- Verification:
  - All five Doc Impact Statement greps return hits.
- Exit condition: Doc Impact Statement fully landed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One new doc; SUMMARY dispatch. |
| Step 2 | M | Three crates; LOCATIONS for downstream arms; guest-WASM gate. |
| Step 3 | — | **DROPPED** — T-P96-C0 retired (Model A; no mask). |
| Step 4 | M | Three sub-steps; two OrcaSlicer SUMMARYs; two new tests. |
| Step 5 | M | Helper + manifest migration + two-module consumer; one SUMMARY. |
| Step 6 | M | Two-module rewrites + two new integration tests; one SUMMARY. |
| Step 7 | M | Two-module edit + 4-color fixture integration test (Model A fragmentation). |
| Step 8 | S | Three doc edits. |

Aggregate context cost: `M` (risk-flagged — 17 tasks; implementer should consider re-spawning a fresh agent after Step 4 if context exceeds 65%). No single step is `L`. Per-step file edit count never exceeds 3 (sub-step structure preserved throughout).

## Packet Completion Gate

- All eight steps complete (Step 3 DROPPED — no work needed); each active step's exit condition met.
- AC-1 through AC-6 + AC-N1/N2 all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-050..T-P96-B entry — via worker dispatch.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record the actual schema-bump version chosen (targeting `4.4.0` from live `4.3.0` — additive for GapFill variants only) in the closure log, along with any concurrent-bump races resolved.
- Note in the closure log that the `external_contour` IR field remains in `SlicedRegion` until P107 T-P96-D — this is by design per ADR-0013 (rewritten). Only consumption is removed in P105.
- Note in the closure log that `bisector_edge_skip_mask`, `compute_bisector_edge_skip_mask`, WIT accessor `bisector-edge-skip-mask`, view accessor, and `paint_segmentation_bisector_mask_tdd.rs` are all removed in this packet per D-105-BISECTOR-MASK-DROPPED (Model A pivot).
- Confirm the implementer's peak context usage stayed under 70%. If exceeded, log it as a packet-authoring lesson for future spec-packet-generator runs (likely indicates Step 4 needs further subdivision in similar future packets).
