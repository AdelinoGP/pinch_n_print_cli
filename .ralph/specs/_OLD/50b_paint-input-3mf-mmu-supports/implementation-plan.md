# Implementation Plan — 50b: Paint Input 3MF MMU + Support Co-Presence Tests & Pipeline Fix

## Step 1 — TDD-RED: Write 4 test stubs

**Task IDs:** TASK-180b
**Objective:** Add 4 new test functions to `model_loader_tdd.rs` that compile. Tests access `mesh.objects[0].paint_data` (not `mesh.paint_data`).

**Precondition:** All 8 packet-50 paint tests pass on current HEAD.
**Postcondition:** 4 new test functions compile; `cargo check -p slicer-host` succeeds.

**Files allowed to read:**
- `crates/slicer-host/tests/model_loader_tdd.rs:1-50` (imports + helper)

**Files allowed to edit:**
- `crates/slicer-host/tests/model_loader_tdd.rs`

**Context cost:** S

**Narrow verification:**
```
cargo check -p slicer-host
```
Exit condition: compiles without error.

---

## Step 2 — TDD-GREEN: Fix test compilation errors and run tests

**Task IDs:** TASK-180b
**Objective:** Fix `mesh.paint_data` → `mesh.objects[0].paint_data` and `*n` → `n` dereference errors. Run all 4 new tests.

**Precondition:** Step 1 complete; 4 test stubs compile.
**Postcondition:** All 4 new tests pass.

**Files allowed to read:**
- `crates/slicer-host/src/model_loader.rs:490-600` (only if a test fails unexpectedly)

**Files allowed to edit:**
- `crates/slicer-host/tests/model_loader_tdd.rs`

**Context cost:** S

**Risk gate:** If `benchy_4color.3mf` causes `load_model` to return `Err` (subdivision rejection), stop and record blocker.

**Narrow verification:**
```
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers load_3mf_4color_material_spans_four_tool_indices load_3mf_4color_support_enforcer_has_facets load_3mf_4color_layer_count_at_least_two --nocapture
```
Exit condition: all 4 PASS.

---

## Step 3 — Regression check

**Task IDs:** TASK-180b
**Objective:** All model_loader tests pass (8 existing + 4 new).

**Files allowed to read:** none
**Files allowed to edit:** none

**Context cost:** S

**Narrow verification:**
```
cargo test -p slicer-host --test model_loader_tdd
```
Exit condition: `test result: ok.` — zero FAILED.

---

## Step 4 — MMU pipeline fix: dominant_tool_index in layer_executor.rs

**Task IDs:** TASK-180b
**Objective:** Propagate paint-derived `WallFeatureFlags.tool_index` to entity `RegionKey.region_id` so path-optimization groups by tool and gcode_emit produces `T{n}` commands.

**Precondition:** Step 3 complete; all model_loader tests pass.
**Postcondition:** Perimeter entities carry paint-derived `region_id` when `WallFeatureFlags.tool_index` is `Some(n)`.

**Files allowed to read:**
- `crates/slicer-host/src/layer_executor.rs:589-685` — `assemble_ordered_entities`
- `crates/slicer-ir/src/slice_ir.rs:1192-1213` — `WallFeatureFlags`
- `crates/slicer-ir/src/slice_ir.rs:1000-1015` — `RegionKey`

**Files allowed to edit:**
- `crates/slicer-host/src/layer_executor.rs`

**Implementation:**
1. Add `use std::collections::HashMap;` and `use slicer_ir::WallFeatureFlags;` to imports.
2. Add `fn dominant_tool_index(flags: &[WallFeatureFlags]) -> Option<u64>` helper that counts `tool_index` occurrences and returns the most common value.
3. In `assemble_ordered_entities`, change the perimeter loop from:
   ```rust
   for region in &perim.regions {
       let key = RegionKey { global_layer_index, object_id: region.object_id.clone(), region_id: region.region_id };
       for wl in &region.walls {
           push(wl.path.clone(), role, key.clone(), &mut out);
       }
   }
   ```
   To:
   ```rust
   for region in &perim.regions {
       for wl in &region.walls {
           let paint_tool = dominant_tool_index(&wl.feature_flags);
           let entity_key = RegionKey {
               global_layer_index,
               object_id: region.object_id.clone(),
               region_id: paint_tool.unwrap_or(region.region_id),
           };
           push(wl.path.clone(), role, entity_key, &mut out);
       }
   }
   ```

**Context cost:** S

**Narrow verification:**
```
cargo check -p slicer-host
cargo test -p slicer-host --test model_loader_tdd
```
Exit condition: compiles and all model_loader tests pass.

---

## Step 5 — MMU pipeline fix: paint-segmentation guest processes paint_layers

**Task IDs:** TASK-180b
**Objective:** Fix the WASM guest to project `object.paint_layers` onto per-layer 2D polygons instead of reading only config keys.

**Precondition:** Step 4 complete.
**Postcondition:** `PaintRegionIR.per_layer` is non-empty for models with MMU paint data. `WallFeatureFlags.tool_index` is populated with `Some(n)` values.

**Files allowed to read:**
- `modules/core-modules/paint-segmentation/src/lib.rs` — reference native implementation
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — current guest
- `crates/slicer-host/src/wit_host.rs:2625-2680` — WIT view construction

**Files allowed to edit:**
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`
- `modules/core-modules/paint-segmentation/paint-segmentation.wasm` (rebuilt)

**Expected sub-agent dispatches:**
- Delegate rebuild of WASM: `cd modules/core-modules/paint-segmentation && ./build-wasm.sh`
- Delegate test: `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`

**Context cost:** M (WASM guest rebuild + debug iteration)

**Implementation requirements:**
- Iterate `objects` parameter; for each object with non-empty `paint_layers`, project 3D triangle facets onto participating layers.
- For each `(layer_index, semantic, paint_value)` tuple, push ONE region containing all 2D polygons.
- DO NOT push one region per triangle (causes millions of entries and 5+ min runtime).
- Align WIT `layer-index` type to `s32` to match host convention.

**Narrow verification:**
```
cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd
cargo build --bin slicer-host --release
```
Exit condition: roundtrip tests pass; slicer builds.

---

## Step 6 — End-to-end GCode verification

**Task IDs:** TASK-180b
**Objective:** Slice `benchy_4color.3mf` end-to-end; verify `T{n}` tool-change commands appear in GCode.

**Precondition:** Steps 4 and 5 complete; all tests pass.
**Postcondition:** GCode output contains ≥1 `T{n}` tool-change command.

**Files allowed to read:**
- `target/benchy_4color_mmu_test.gcode` (first 100 lines only)

**Files allowed to edit:** none

**Context cost:** S

**Narrow verification:**
```powershell
cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module modules/core-modules/perimeters-default/target/wasm32-unknown-unknown/release/perimeters_default.wasm --module-dir modules/core-modules --output target/benchy_4color_mmu_test.gcode
Select-String -Path target/benchy_4color_mmu_test.gcode -Pattern "^T\d" | Select-Object -First 5
```
Exit condition: at least 1 `T{n}` match found.

---

## Step 7 — Lint gate

**Task IDs:** TASK-180b
**Objective:** No new clippy warnings in `slicer-host`.

**Precondition:** Steps 1–6 complete.
**Postcondition:** `cargo clippy -p slicer-host -- -D warnings` exits 0.

**Files allowed to read:** none
**Files allowed to edit:** `crates/slicer-host/src/layer_executor.rs` (lint fixes only), `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` (lint fixes only)

**Context cost:** S

**Narrow verification:**
```
cargo clippy -p slicer-host -- -D warnings
```
Exit condition: exit code 0, no warnings.

---

## Packet Completion Gate

All steps complete when:
1. `cargo test -p slicer-host --test model_loader_tdd` → `test result: ok.` (27 tests)
2. `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd` → `test result: ok.`
3. `cargo clippy -p slicer-host -- -D warnings` → exit 0
4. `cargo check --workspace` → exit 0
5. GCode output contains ≥1 `T{n}` tool-change command
6. `packet.spec.md` updated to `status: implemented`

Do NOT run `cargo test --workspace` — not required at this packet boundary.