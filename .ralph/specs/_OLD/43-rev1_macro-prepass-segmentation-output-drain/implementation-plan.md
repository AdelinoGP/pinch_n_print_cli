# Implementation Plan: 43-rev1_macro-prepass-segmentation-output-drain

## Execution Rules

- One atomic step at a time. Each step has its own precondition + postcondition + falsifying check.
- Each step honors the context-discipline preamble. Files-allowed-to-read, files-allowed-to-edit, expected dispatches, and context cost are budget contracts, not metadata.
- Stop reading at 60% context. Hand off at 85%.
- Read budget excludes `crates/slicer-macros/src/lib.rs`. Delegate any verification of macro behavior. The bounded one-line edit at line 1317 (Step 2.5) is the only direct write to that file in this packet.
- Do not run `cargo test --workspace` at any step. Use targeted `cargo test -p <crate> --test <file>` only.

## Steps

### Step 1: Activation gate + assumption verification

- Task IDs: `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: Confirm the locked assumptions in `design.md` against current master, and capture the loader/registry shape in `macro_all_worlds_roundtrip_tdd.rs` so Step 8 has a concrete edit target.
- Precondition: master is clean; commit `0c4e8b2` is HEAD; commit `46aed61` is in master.
- Postcondition: A 5-line FACT block recording: (a) pre-deviation `sdk-prepass-guest/src/lib.rs` line count and first-line, (b) `macro_all_worlds_roundtrip_tdd.rs` registry shape (hardcoded array vs constant vs enum), (c) `Cargo.toml` workspace membership of the existing `sdk-*-guest` crates (empty `[workspace]` confirms standalone).
- Files allowed to read (read-only — discovery only):
  - none directly. Use dispatch.
- Files allowed to edit: none.
- Files explicitly out-of-bounds:
  - `crates/slicer-macros/src/lib.rs` — already verified via grounding dispatch; no further reads.
- Expected sub-agent dispatches:
  - `Question: report the exact pre-0c4e8b2 contents and line count of test-guests/sdk-prepass-guest/src/lib.rs; Scope: git show 0c4e8b2^:test-guests/sdk-prepass-guest/src/lib.rs; Return: SNIPPET (≤ 110 lines verbatim)`
  - `Question: in crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs, what is the exact loader pattern for sdk-*-guests (registry constant vs hardcoded calls vs builder); how do new guests get registered; show the relevant 30-line region; Scope: that file only; Return: SNIPPET (≤ 30 lines, with file:line)`
  - `Question: confirm test-guests/sdk-finalization-guest/Cargo.toml and test-guests/sdk-layer-pathopt-guest/Cargo.toml both declare empty [workspace]; Return: FACT yes/no with line refs`
- Context cost: S
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - All three dispatches return well-formed answers; record into Step 2/8 preconditions.
- Exit condition: 5-line FACT block captured + `macro_all_worlds_roundtrip_tdd.rs` registry shape known.

### Step 2: Revert sdk-prepass-guest + rebuild + regression-defense pass

- Task IDs: `TASK-130`
- Objective: Restore `test-guests/sdk-prepass-guest/src/lib.rs` to its pre-`0c4e8b2` single-stage `#[slicer_module]` MeshAnalysis form, rebuild its `.component.wasm`, and confirm the previously-demoted tests still pass (now against macro-emitted bytes).
- Precondition: Step 1 captured the pre-deviation source.
- Postcondition: AC-1 green; `dispatch_tdd.rs` macro-path tests still pass; `macro_all_worlds_roundtrip_tdd.rs` prepass tests still pass.
- Files allowed to read:
  - none beyond Step 1's captured snippet.
- Files allowed to edit (≤ 3):
  - `test-guests/sdk-prepass-guest/src/lib.rs`
- Files explicitly out-of-bounds:
  - any other `test-guests/` crate.
- Expected sub-agent dispatches:
  - `Question: did the rebuild emit test-guests/sdk-prepass-guest.component.wasm and is its byte length plausible (> 30 KB and < 100 KB)?; Scope: bash test-guests/build-test-guests.sh && stat test-guests/sdk-prepass-guest.component.wasm; Return: FACT pass/fail`
  - `Question: do dispatch_tdd macro-path tests still PASS against the reverted guest?; Scope: cargo test -p slicer-host --test dispatch_tdd macro_path -- --nocapture; Return: FACT pass/fail with the test result line`
  - `Question: do macro_all_worlds_roundtrip_tdd prepass tests still PASS?; Scope: cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass -- --nocapture; Return: FACT pass/fail`
- Context cost: S
- Authoritative docs: none for this step.
- OrcaSlicer refs: none.
- Verification:
  - `git diff --quiet 0c4e8b2^ -- test-guests/sdk-prepass-guest/src/lib.rs` exits 0.
  - All three dispatches return PASS.
- Exit condition: AC-1 green; pre-existing macro-path tests stay GREEN against the reverted guest.

### Step 2.5: Macro paint_seg_arm scope fix (bounded two-hunk edit)

- Task IDs: `TASK-130a` (closes the latent bug that blocked the original packet 43 path).
- Objective: Apply a bounded two-hunk edit in `build_prepass_world_glue` at `crates/slicer-macros/src/lib.rs` so the existing paint_seg_arm quote-block (lines 1814-1829, untouched) resolves bare `Polygon` and `Point2`. Hunk 1: line 1317 inline-WIT extended from `use geometry.{ex-polygon};` to `use geometry.{ex-polygon, polygon, point2};`. Hunk 2: explicit `use self::slicer::world_prepass::geometry::{Polygon, Point2};` (with brief comment) added to the `segmentation_helpers` quote block, mirroring the finalization-world pattern at lib.rs:998. Discovered during the original Step 3 attempt; the line-1317 fix alone is necessary but not sufficient because wit-bindgen 0.24 skips flat re-exports for world-level `use` items whose TypeInfo modes_of() returns empty.
- Precondition: Step 2 complete; sdk-prepass-guest reverted and rebuilds clean against the unfixed macro (the revert path uses MeshAnalysis only and is unaffected by the prepass paint-seg bug).
- Postcondition: AC for the macro fix green. `git diff --numstat crates/slicer-macros/src/lib.rs` shows total churn < 20 lines.
- Files allowed to read:
  - `crates/slicer-macros/src/lib.rs` lines 1310-1340 (line 1317 edit) AND the `segmentation_helpers` region inside `build_prepass_world_glue` (Read with offset/limit). Confirm line 1317 reads `            use geometry.{ex-polygon};`.
- Files allowed to edit (≤ 1):
  - `crates/slicer-macros/src/lib.rs` (the two hunks above only — paint_seg_arm at 1814-1829 stays byte-identical).
- Files explicitly out-of-bounds:
  - the paint_seg_arm quote-block (1814-1829) and every other macro arm; any other macro behavior change.
- Expected sub-agent dispatches:
  - `Question: Apply hunk 1 (line 1317 inline-WIT) and hunk 2 (use Polygon/Point2 in segmentation_helpers, mirroring finalization-world pattern at lib.rs:998), then run cargo build --workspace; Return: FACT pass/fail with numstat`
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `rg -q '^\s*use geometry\.\{ex-polygon, polygon, point2\};\s*$' crates/slicer-macros/src/lib.rs` succeeds.
  - `rg -q '^\s*use self::slicer::world_prepass::geometry::Polygon;\s*$' crates/slicer-macros/src/lib.rs` succeeds.
  - `rg -q '^\s*use self::slicer::world_prepass::geometry::Point2;\s*$' crates/slicer-macros/src/lib.rs` succeeds.
  - `git diff --numstat crates/slicer-macros/src/lib.rs` reports total churn < 20 lines.
- Exit condition: macro builds; the two hunks are present; paint_seg_arm at 1814-1829 unchanged.

### Step 2.6: Host layer-idx alignment with canonical wit/

- Task IDs: `TASK-130b` (closes a contract drift that blocks any test guest from invoking `push-paint-region` end-to-end).
- Objective: Align `crates/slicer-host/src/wit_host.rs:543` `type layer-idx = u32;` with the canonical `wit/deps/ir-types.wit:8` `s32`. The four non-paint view records keep explicit `u32` (they don't use the `layer-idx` alias in the macros crate WIT). Add negative-rejection in the host push_paint_region validator. Cast `entry.layer_index as u32` at the IR boundary in `dispatch.rs:harvest_paint_segmentation_ir`. PaintRegionIR contract is preserved (HashMap<u32, _>). Discovered during Step 6 — wasmtime 43 component linker rejects the s32/u32 mismatch when the new paintseg guest invokes push-paint-region.
- Precondition: Step 2.5 complete; macro paint_seg_arm compiles end-to-end.
- Postcondition: AC for the host fix green; AC-5/6/7 (paint round-trip ACs blocked by the linker mismatch) unblock; regression checks (`dispatch_tdd macro_path`, `macro_all_worlds_roundtrip prepass`) stay green.
- Files allowed to read:
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `wit/deps/ir-types.wit` (≤ 60 lines — canonical reference)
  - `crates/slicer-ir/src/slice_ir.rs` only the PaintRegionIR / LayerPaintMap / SemanticRegion region
- Files allowed to edit (≤ 2):
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
- Files explicitly out-of-bounds:
  - `crates/slicer-macros/src/lib.rs` (no further macro changes after Step 2.5)
  - `crates/slicer-ir/...` (IR contract stays — PaintRegionIR.per_layer keys are u32)
  - any test files
- Expected sub-agent dispatches:
  - `Question: Apply the layer-idx alignment in wit_host.rs (alias to s32; explicit u32 retention for the four non-paint records; negative-rejection in push_paint_region validator) and dispatch.rs (cast i32→u32 at IR boundary), then run cargo build --workspace, bash test-guests/build-test-guests.sh, and the regression test commands. Return pass/fail counts.`
- Context cost: M (cascade compile errors may surface for any record that loses the alias — the worker must navigate the cascade carefully and keep the four non-paint records on explicit u32 to match the macros crate WIT).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `bash test-guests/build-test-guests.sh` PASS.
  - `cargo test -p slicer-host --test dispatch_tdd macro_path` PASS (regression).
  - `cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass` PASS (regression).
- Exit condition: paint-segmentation push-paint-region accepts the new paintseg guest in the linker; the four non-paint records and PaintRegionIR shape unchanged.

### Step 3: Author sdk-prepass-paintseg-guest

- Task IDs: `TASK-130a`, `TASK-130b`
- Objective: Create `test-guests/sdk-prepass-paintseg-guest/` (Cargo.toml + src/lib.rs) with one `#[slicer_module] impl PrepassModule` overriding `on_print_start` + `run_paint_segmentation` and a three-branch `fixture_case` switch plus default no-op (hole_bearing, custom_payload, force_push_failure). The original four-branch design's `empty_polygons` fixture was retired in the 2026-05-08 packet revision because the host validator rejects empty `polygons` lists; the no-fixture default branch covers AC-7 (silent path) and `force_push_failure` covers the host-validator rejection path.
- Precondition: Step 2 complete; `sdk-finalization-guest`/`sdk-layer-pathopt-guest` template inspected.
- Postcondition: Crate compiles standalone for `wasm32-unknown-unknown`; AC-2 + AC-15-negative-grep green.
- Files allowed to read (templates):
  - `test-guests/sdk-finalization-guest/src/lib.rs` (≤ 60 lines; already inspected)
  - `test-guests/sdk-layer-pathopt-guest/Cargo.toml` (≤ 15 lines)
- Files allowed to edit (≤ 3):
  - `test-guests/sdk-prepass-paintseg-guest/Cargo.toml` (new)
  - `test-guests/sdk-prepass-paintseg-guest/src/lib.rs` (new)
- Files explicitly out-of-bounds:
  - `crates/slicer-macros/`, `crates/slicer-sdk/`, any host code.
- Expected sub-agent dispatches:
  - `Question: in crates/slicer-sdk/src/prepass_builders.rs, return the exact public signature of PaintSegmentationOutput::push_paint_region (or whatever it is called) and the PaintRegion / ExPolygon / PaintValue field shapes; Return: SNIPPET ≤ 25 lines`
  - `Question: build sdk-prepass-paintseg-guest in isolation (cargo build -p sdk-prepass-paintseg-guest --target wasm32-unknown-unknown); Return: FACT pass/fail with first error line if fail`
- Context cost: M (one new file with four fixture branches; expect a small dispatch loop while iterating on builder API).
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for PaintRegionIR / PaintValue.
  - `docs/08_coordinate_system.md` — delegate FACT for the × 10_000 conversion.
- OrcaSlicer refs: none.
- Verification:
  - AC-2 verification command (cargo build + grep counts).
  - AC-15-negative grep (no `wit_bindgen::generate!`).
- Exit condition: AC-2 green and the cargo build succeeds.

### Step 4: Author sdk-prepass-meshseg-guest

- Task IDs: `TASK-130a`, `TASK-130b`
- Objective: Create `test-guests/sdk-prepass-meshseg-guest/` (Cargo.toml + src/lib.rs) with one `#[slicer_module] impl PrepassModule` overriding `on_print_start` + `run_mesh_segmentation` and a single-branch `fixture_case == "marks_basic"` switch that marks triangle 12 on `obj-a`.
- Precondition: Step 3 complete (template + builder API understood).
- Postcondition: Crate compiles for `wasm32-unknown-unknown`; AC-3 + AC-15-negative-grep green.
- Files allowed to read:
  - `test-guests/sdk-prepass-paintseg-guest/src/lib.rs` (just authored — useful as immediate template)
- Files allowed to edit (≤ 3):
  - `test-guests/sdk-prepass-meshseg-guest/Cargo.toml` (new)
  - `test-guests/sdk-prepass-meshseg-guest/src/lib.rs` (new)
- Files explicitly out-of-bounds:
  - any other crate.
- Expected sub-agent dispatches:
  - `Question: in crates/slicer-sdk/src/prepass_builders.rs (or wherever MeshSegmentationOutput is defined), return the exact public signature for marking a triangle (push_mark / mark_triangle / etc.); Return: SNIPPET ≤ 15 lines`
  - `Question: build sdk-prepass-meshseg-guest in isolation; Return: FACT pass/fail`
- Context cost: S
- Authoritative docs: `docs/02_ir_schemas.md` — delegate SUMMARY for `MeshSegmentationIR`.
- OrcaSlicer refs: none.
- Verification:
  - AC-3 verification command.
  - AC-15-negative grep.
- Exit condition: AC-3 green and the cargo build succeeds.

### Step 5: Wire siblings into build script + workspace, full guest rebuild

- Task IDs: `TASK-130a`, `TASK-130b`
- Objective: Add `sdk-prepass-paintseg-guest:sdk_prepass_paintseg_guest` and `sdk-prepass-meshseg-guest:sdk_prepass_meshseg_guest` to the GUESTS array in `test-guests/build-test-guests.sh`. Run the full guest build script. Confirm both new `.component.wasm` files exist.
- Precondition: Steps 3 and 4 complete; both crates build standalone.
- Postcondition: AC-4 green; both `.component.wasm` artifacts present.
- Files allowed to read:
  - `test-guests/build-test-guests.sh` (≤ 30 lines).
- Files allowed to edit (≤ 3):
  - `test-guests/build-test-guests.sh`
- Files explicitly out-of-bounds:
  - guest crate sources (already complete).
- Expected sub-agent dispatches:
  - `Question: run bash test-guests/build-test-guests.sh and report exit code + last 10 lines of stdout; Return: FACT pass/fail`
  - `Question: stat the two new .component.wasm files; Return: FACT yes/no for each`
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 verification command.
- Exit condition: AC-4 green.

### Step 6: Retarget paint-segmentation round-trip TDD

- Task IDs: `TASK-130b`
- Objective: Edit `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` so its 10 tests load `sdk-prepass-paintseg-guest.component.wasm` instead of `sdk-prepass-guest.component.wasm`. Run the file. All 10 tests must pass.
- Precondition: Step 5 complete; `sdk-prepass-paintseg-guest.component.wasm` exists in `test-guests/`.
- Postcondition: AC-5 + AC-6 + AC-7 + AC-14 (negative push_failure) green.
- Files allowed to read:
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (≤ 600 lines — locate the load-path constant via grep; do not read the full file).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs`
- Files explicitly out-of-bounds:
  - other test files.
- Expected sub-agent dispatches:
  - `Question: in macro_paint_segmentation_output_roundtrip_tdd.rs, what is the exact constant or function defining the .component.wasm load path? Return file:line and the ≤ 5-line definition; Return: SNIPPET`
  - `Question: run cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd; Return: FACT for total/passed/failed counts and the names of any failures`
- Context cost: M (test file has 10 cases; understanding what each asserts may require a SNIPPET dispatch per failed test).
- Authoritative docs: none beyond `docs/02_ir_schemas.md` (already gathered).
- OrcaSlicer refs: none.
- Verification:
  - AC-5, AC-6, AC-7, AC-14 verification commands.
- Exit condition: All 10 tests in `macro_paint_segmentation_output_roundtrip_tdd.rs` PASS.

### Step 7: Retarget mesh-segmentation round-trip TDD

- Task IDs: `TASK-130b`
- Objective: Edit `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` so its 1 test loads `sdk-prepass-meshseg-guest.component.wasm`. Run the file. The 1 test must pass.
- Precondition: Step 5 complete; sibling .wasm exists.
- Postcondition: AC-8 green.
- Files allowed to read:
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (≤ 200 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs`
- Files explicitly out-of-bounds:
  - other test files.
- Expected sub-agent dispatches:
  - `Question: run cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd; Return: FACT pass/fail`
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - AC-8 verification command.
- Exit condition: The 1 test PASSes.

### Step 8: Extend freshness + macro-all-worlds registries

- Task IDs: `TASK-130a`, `TASK-130b`
- Objective: Update `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` GUESTS table (lines 11-31) and `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` registry to include both new siblings. Run both targeted tests.
- Precondition: Steps 5/6/7 complete; siblings registered in build script and round-trips green.
- Postcondition: AC-9 + AC-10 + AC-11 green; macro-arm proof loop now extends to the two new siblings automatically.
- Files allowed to read:
  - `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` (lines 11-50 only)
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (registry section only — Step 1 captured the line range)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs`
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs`
- Files explicitly out-of-bounds:
  - everything else.
- Expected sub-agent dispatches:
  - `Question: run cargo test -p slicer-host --test guest_fixture_freshness_tdd; Return: FACT pass/fail`
  - `Question: run cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd; Return: FACT pass/fail with breakdown of prepass cases`
- Context cost: M (loader-shape work in `macro_all_worlds_roundtrip_tdd.rs` may be one-line or refactor — bounded by Step 1 dispatch finding).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - AC-9, AC-10, AC-11 verification commands.
- Exit condition: Both targeted tests green; both new sibling .component.wasm names appear in both registries.

### Step 9: docs/05 Single-Stage-Per-Impl section

- Task IDs: `TASK-130`
- Objective: Add a section to `docs/05_module_sdk.md` that records the macro single-stage-per-impl constraint, cites `crates/slicer-macros/src/lib.rs:43-52` and `:2024`, and explains the sibling-crate workaround using `sdk-prepass-paintseg-guest` / `sdk-prepass-meshseg-guest` as exemplars.
- Precondition: Steps 6/7/8 green (so the exemplars are real).
- Postcondition: AC-12 green.
- Files allowed to read:
  - `docs/05_module_sdk.md` (read directly; expected ≤ 300 lines).
- Files allowed to edit (≤ 3):
  - `docs/05_module_sdk.md`
- Files explicitly out-of-bounds:
  - `crates/slicer-macros/src/lib.rs` — cite line numbers from the macro inspection in Step 0 grounding; do not re-read the macro source.
- Expected sub-agent dispatches:
  - none (docs edit only).
- Context cost: S
- Authoritative docs: `docs/05_module_sdk.md` itself.
- OrcaSlicer refs: none.
- Verification:
  - AC-12 verification command (rg for "Single-Stage-Per-Impl" + line refs + `__slicer_prepass_world_export`).
- Exit condition: AC-12 green.

### Step 10: Close TASK-130 cluster + DEV-025 mismatch 3

- Task IDs: `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: Update `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, and `docs/14_deviation_audit_history.md` so TASK-130/130a/130b are closed and DEV-025 mismatch 3 is closed.
- Precondition: Step 9 complete (so the doc trail is consistent).
- Postcondition: AC-13 green.
- Files allowed to read:
  - none directly. All three docs are large; delegate.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds:
  - everything else.
- Expected sub-agent dispatches:
  - `Question: in docs/07_implementation_status.md, locate the TASK-130, TASK-130a, TASK-130b rows; return file:line for each with the current status marker; Return: LOCATIONS (≤ 3 entries)`
  - `Question: in docs/DEVIATION_LOG.md, locate the DEV-025 row and report its current "open mismatch" wording; Return: SNIPPET ≤ 20 lines`
  - `Question: in docs/14_deviation_audit_history.md, where (file:line) does DEV-025 currently appear and what is the existing TASK-128/130 cross-reference structure; Return: SNIPPET ≤ 25 lines`
  - Apply edits via Edit tool with the exact line/anchor returned. After each edit, dispatch a one-line FACT verification.
- Context cost: M (three docs; delegated reads only).
- Authoritative docs: the three docs being edited.
- OrcaSlicer refs: none.
- Verification:
  - AC-13 verification command (rg for `[x] TASK-130*` and absence of `DEV-025.*open`).
- Exit condition: AC-13 green.

### Step 11: Mark Packet 43 superseded

- Task IDs: `TASK-130` (housekeeping).
- Objective: Edit `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` frontmatter to add `status: superseded` and `superseded_by: 43-rev1_macro-prepass-segmentation-output-drain`.
- Precondition: Step 10 complete; packet substantively done.
- Postcondition: AC-14 green.
- Files allowed to read:
  - `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` (frontmatter only — first 15 lines).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md`
- Files explicitly out-of-bounds:
  - all other files in `.ralph/specs/43_macro-prepass-segmentation-output-drain/` (do not edit `design.md`, `requirements.md`, etc.; they remain as historical record).
- Expected sub-agent dispatches: none.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - AC-14 verification command (rg for `status: superseded` + `superseded_by:`).
- Exit condition: AC-14 green.

### Step 12: Acceptance ceremony

- Task IDs: all packet task ids.
- Objective: Re-dispatch every pipe-suffixed AC verification command and confirm green. Run `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` for the final sweep.
- Precondition: Steps 1-11 complete.
- Postcondition: Packet ready to flip to `status: implemented`.
- Files allowed to read:
  - none. Pure dispatch.
- Files allowed to edit:
  - `.ralph/specs/43-rev1_macro-prepass-segmentation-output-drain/packet.spec.md` (status flip from `active` to `implemented` after all ACs green).
- Files explicitly out-of-bounds:
  - everything else.
- Expected sub-agent dispatches:
  - One dispatch per AC (re-run each pipe-suffixed command). Each returns FACT pass/fail.
  - Final dispatch: `cargo clippy --workspace -- -D warnings`. Return FACT pass/fail.
- Context cost: M (15 dispatches but each is FACT-shaped — minimal context cost per).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands return PASS.
  - Clippy GREEN.
- Exit condition: All 15 ACs green; clippy green; packet ready for implementer to set `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | Pure dispatch — three FACT/SNIPPET queries. |
| 2 | S | One file revert + three regression dispatches. |
| 2.5 | S | One-line inline-WIT edit at lib.rs:1317; cargo build --workspace check. |
| 2.6 | M | Host layer-idx alignment cascade (wit_host.rs alias change + four record retentions + dispatch.rs cast). Regression checks. |
| 3 | M | Builder-API discovery loop; three-branch fixture body + default no-op. |
| 4 | S | Reuses Step 3's builder discovery; one-branch body. |
| 5 | S | Two GUESTS entries + one rebuild. |
| 6 | M | 10 tests; failure inspection may need SNIPPET dispatches. |
| 7 | S | One test. |
| 8 | M | Registry shape from Step 1; two file edits + targeted tests. |
| 9 | S | One docs section addition. |
| 10 | M | Three docs; all delegated reads. |
| 11 | S | One frontmatter edit. |
| 12 | M | 15 FACT dispatches. |

Aggregate: M. No step is L. If any step measures L during execution, split before proceeding.

## Packet Completion Gate

- All 14 steps complete (including Steps 2.5 and 2.6 from the 2026-05-08 revision).
- Every step's exit condition met.
- All 18 ACs (16 positive + 2 negative) green.
- `cargo clippy --workspace -- -D warnings` green.
- `docs/07_implementation_status.md` shows TASK-130/130a/130b closed (verified via dispatch — never edit by loading the full backlog into the implementer's context).
- `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` shows `status: superseded` + `superseded_by:` field.
- `packet.spec.md` for this packet ready to move from `status: active` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`. Each returns FACT pass/fail.
- Confirm packet-level verification commands are green (cargo build, cargo clippy, build-test-guests.sh).
- Record the implementer's peak context usage. If it exceeded 70%, log it as a packet-authoring lesson — this packet was estimated M and should not have approached the budget.
