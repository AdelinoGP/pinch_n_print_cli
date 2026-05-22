# Implementation Plan: 58_extruder-per-modifier-gcode

## Execution Rules

- One atomic step at a time.
- TASK-208 maps to config stamping + tool fallback production changes.
- TASK-209 maps to integration tests and doc registration.
- Discovery step (Step 1) confirms assumptions before any edits — if assumptions are wrong, adjust design before coding.
- Aggregate context cost is **M**. All steps are S or M.
- This packet depends on Packets 57, 56c, 64, and 51 being `status: implemented`. Step 0 verifies.

## Steps

### Step 0: Precondition gate

- Task IDs: TASK-208, TASK-209 (precursor)
- Objective: Verify Packet 57 is `status: implemented`, fixtures exist, and the two RED tests from Packet 57 exist and fail as expected.
- Precondition: Packet activated.
- Postcondition: All preconditions confirmed OR halt.
- Files allowed to read: none directly. Pure dispatch step.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: everything.
- Expected sub-agent dispatches:
  - "What is the `status:` value in the frontmatter of `.ralph/specs/57_3mf-fixture-e2e-hardening/packet.spec.md`? Return FACT one-line." → FACT. Expected: `implemented`.
  - "Does `resources/bridge_support_enforcers.3mf` exist? Return FACT yes/no." → FACT. Expected: yes.
  - "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd -- --list` and return FACT: do test names `extruder_metadata_reaches_tool_index` and `extruder_per_object_vs_support_extruder` appear in the list?" → FACT. Expected: both present.
  - "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd extruder_metadata_reaches_tool_index -- --exact --nocapture` and return FACT: does it fail? Return assertion message." → FACT. Expected: fails.
  - "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd extruder_per_object_vs_support_extruder -- --exact --nocapture` and return FACT: does it fail? Return assertion message." → FACT. Expected: fails.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All FACTs return expected values.
- Exit condition: Step 1 may begin.

### Step 1: Discovery — region_mapping internals and MeshIR availability

- Task IDs: TASK-208
- Objective: Confirm (a) `overlay_resolved()` handles `extensions` merging, (b) the per-region loop shape in `execute_region_mapping_with_cap()`, (c) whether `mesh_ir: &MeshIR` is directly available or must be threaded through.
- Precondition: Step 0 clean.
- Postcondition: Exact insertion site and call signature for `stamp_modifier_config_deltas()` confirmed. Design adjusted if needed.
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` — narrow reads at `overlay_resolved` (lines 100-193, specifically lines 190-195 for extensions handling) and `execute_region_mapping_with_cap` (lines 279-455 for loop structure and parameter availability).
- Files allowed to edit (≤ 3): none (discovery only).
- Files explicitly out-of-bounds: `model_loader.rs`, `paint_segmentation.rs`, `gcode_emit.rs`, WIT, SDK, macros.
- Expected sub-agent dispatches:
  - "In `crates/slicer-host/src/region_mapping.rs::overlay_resolved` at line ~193, does the function insert entries from `overlay.extensions` into `base.extensions`? Return SNIPPETS (lines 188-198)." → SNIPPETS. Expected: `base.extensions.insert(...)` or similar merge logic.
  - "In `crates/slicer-host/src/region_mapping.rs::execute_region_mapping_with_cap` (line 279+), what is the per-region loop structure? Where does `base_config` get resolved relative to `paint_overrides`? Is `mesh_ir` available as a parameter or from the blackboard? Return SNIPPETS of the function signature and the region-loop body (≤ 40 lines)." → SNIPPETS.
  - "What type does `ModifierVolume.applies_to` have? Is it `ModifierScope`? Return SNIPPETS of the `ModifierScope` enum definition from `crates/slicer-ir/src/slice_ir.rs`." → SNIPPETS.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: SNIPPETS confirm `overlay_resolved` handles extensions and reveal the exact insertion site. If `mesh_ir` is unavailable, the design is amended to add a `mesh_ir` parameter.
- Exit condition: Insertion site understood. Ready to code.

### Step 2: Implement stamp_modifier_config_deltas() in region_mapping.rs

- Task IDs: TASK-208
- Objective: Add `stamp_modifier_config_deltas(mesh_ir: &MeshIR, region_extent: &BoundingBox2, base_config: &mut ResolvedConfig)` function and call it inside `execute_region_mapping_with_cap()` after per-object config resolution, before paint_overrides.
- Precondition: Step 1 clean (insertion site and parameter availability confirmed).
- Postcondition: `config_delta_extruder_stamped_into_extensions` test GREEN. All synthetic 56c tests stay GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` — lines 100-193 (`overlay_resolved`), lines 279-455 (`execute_region_mapping_with_cap`).
  - `crates/slicer-ir/src/slice_ir.rs` — lines 365-398 (`ConfigDelta`, `ModifierVolume` structs).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/region_mapping.rs` — add `stamp_modifier_config_deltas()` function (~30 lines) + call site (~2 lines).
- Files explicitly out-of-bounds: `model_loader.rs`, `paint_segmentation.rs`, `gcode_emit.rs`, `layer_executor.rs` (edited in Step 3), WIT, SDK, macros.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --tests`. FACT pass/fail." → FACT. Expected: PASS (or fix compilation).
  - "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd`. FACT pass/fail." → FACT. Expected: PASS (no regression — synthetic fixtures have empty config_delta).
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --tests` clean.
  - `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` stays GREEN.
- Exit condition: Compiles clean; 56c regression green.

### Step 3: Implement config-extensions tool fallback in layer_executor.rs

- Task IDs: TASK-208
- Objective: In `layer_executor.rs`, add fallback: when `dominant_tool_index()` returns `None`, check `region_plan.config.extensions["extruder"]` for a `ConfigValue::Int(n)` and use `n as u32` for `region_id`-as-tool assignment.
- Precondition: Step 2 clean (config_delta values reach `RegionPlan.config.extensions`).
- Postcondition: `extruder_metadata_reaches_tool_index` (was RED from Packet 57) turned GREEN. The RED test originally asserted `PaintValue::ToolIndex` on `SemanticRegion` — this step instead asserts `ConfigValue::Int(0)` in `config.extensions` + full pipeline produces `T0`/`T1` in GCode.
- Files allowed to read:
  - `crates/slicer-host/src/layer_executor.rs` — lines 756-766 (`dominant_tool_index` → `region_id` assignment).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_executor.rs` — add 8-line fallback after `dominant_tool_index()` call.
- Files explicitly out-of-bounds: `region_mapping.rs` (edited in Step 2, re-read for context only), `gcode_emit.rs`.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --tests`. FACT pass/fail." → FACT. Expected: PASS.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --tests` clean.
- Exit condition: Compiles clean. Tool fallback is ready for test verification in Step 4.

### Step 4: Author integration tests

- Task IDs: TASK-209
- Objective: Extend `threemf_fixture_e2e_tdd.rs` with 5 new tests + update 2 RED tests from Packet 57. Turn the 2 RED tests GREEN (update their assertions to match the config-stamping path). Add new tests covering: config_delta extruder in extensions, non-extruder key survival, negative_part benign, subtype-only no-op, and conflicting modifier priority.
- Precondition: Step 3 clean (config stamping + tool fallback implemented).
- Postcondition: All 14+ fixture E2E tests GREEN (7-9 from Packet 57 + 2 RED turned GREEN + 5 new).
- Files allowed to read:
  - `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — full (existing RED test bodies for reference).
  - `crates/slicer-host/src/gcode_emit.rs` — narrow read at line 1374 for `T{n}` format confirmation.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — add/update 7 test functions (~250 lines).
- Files explicitly out-of-bounds: all production source except `gcode_emit.rs` (read-only format confirmation).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd -- --nocapture`. Return FACT pass/fail per test." → FACT.
  - "Run `cargo test -p slicer-host --test gcode_emit_tdd`. FACT pass/fail." → FACT. Expected: PASS (tool-change regression).
- Context cost: M
- Authoritative docs: `packet.spec.md` Acceptance Criteria (this packet).
- OrcaSlicer refs: none.
- Verification:
  - All fixture E2E tests GREEN.
  - `gcode_emit_tdd` stays GREEN.
- Exit condition: All tests GREEN; GCode regression clean.

### Step 5: Regression sweep + clippy

- Task IDs: TASK-208, TASK-209
- Objective: Re-run Packet 56/56b/56c/57 + paint pipeline regression suites. Assert clippy clean.
- Precondition: Step 4 GREEN.
- Postcondition: All regression suites GREEN; clippy clean.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): any source file clippy demands (sticking to files-in-scope: `region_mapping.rs`, `layer_executor.rs`).
- Files explicitly out-of-bounds: macros, WIT, SDK, IR, files owned by Packets 56/56b/56c.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test threemf_fixture_e2e_tdd`. Return FACT pass/fail per test file." → FACT.
  - "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail." → FACT.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All FACTs GREEN.
- Exit condition: Clean workspace; no regressions.

### Step 6: Doc registration

- Task IDs: TASK-208, TASK-209
- Objective: Append TASK-208 and TASK-209 rows to `docs/07_implementation_status.md` after TASK-207.
- Precondition: Step 5 clean.
- Postcondition: `docs/07` reflects packet outcome.
- Files allowed to read:
  - `docs/07_implementation_status.md` — narrow read around TASK-207 row to confirm insertion point (line ~154).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - "Append `[x] TASK-208` and `[x] TASK-209` rows after TASK-207 in `docs/07_implementation_status.md`, each naming packet `58_extruder-per-modifier-gcode`. TASK-208 summary: 'Stamp config_delta.fields from overlapping modifier volumes into RegionPlan.config.extensions; add config-extensions-driven required_tool fallback in layer_executor.rs.' TASK-209 summary: 'GCode and config integration tests: 7 tests proving extruder tool-change, non-extruder key survival, backward compatibility, and modifier priority.' Return the resulting two lines verbatim. SNIPPETS, ≤ 5 lines." → SNIPPETS.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `rg -c 'TASK-208.*58_extruder-per-modifier-gcode' docs/07_implementation_status.md` → 1.
  - `rg -c 'TASK-209.*58_extruder-per-modifier-gcode' docs/07_implementation_status.md` → 1.
- Exit condition: Both `rg` checks pass.

### Step 7: Pre-ceremony verification

- Task IDs: TASK-208, TASK-209
- Objective: Re-run every pipe-suffixed AC command from `packet.spec.md` to confirm all GREEN.
- Precondition: Step 6 complete.
- Postcondition: All AC commands return PASS.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command (AC-1 through AC-7, AC-N1, AC-N2), each returning FACT pass/fail.
- Context cost: S
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification: All AC commands PASS.
- Exit condition: All FACTs GREEN.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 0 | S | Precondition + RED test confirmation dispatches. |
| Step 1 | S | Three narrow SNIPPETS dispatches. |
| Step 2 | S | ~30-line function + call site; verify compiles + no regression. |
| Step 3 | S | ~8-line fallback in layer_executor.rs. |
| Step 4 | M | ~250-line test authoring — most context-intensive step. |
| Step 5 | S | Regression sweep dispatches + clippy. |
| Step 6 | S | Doc registration. |
| Step 7 | S | Pre-ceremony AC verification dispatches. |

Aggregate: **M** (1 M + 7 S).

## Packet Completion Gate

- All 8 steps complete.
- Every step exit condition met.
- All fixture E2E tests GREEN (7-9 original from Packet 57 + 2 RED → GREEN + 5 new).
- All regression suites GREEN (56/56b/56c/57); clippy clean.
- `docs/07_implementation_status.md` updated with TASK-208, TASK-209 rows.
- `packet.spec.md` ready to move to `status: implemented`.
- No `cargo test --workspace` — targeted regression commands sufficient for ~50-line production change.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (Step 7).
- Confirm all 9 ACs GREEN (7 positive + 2 negative).
- The two RED tests from Packet 57 are now GREEN (updated assertions matching config-stamping path).
- No workspace-level gate required.
