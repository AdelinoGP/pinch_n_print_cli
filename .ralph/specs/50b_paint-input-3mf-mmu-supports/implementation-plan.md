# Implementation Plan — 50b: Paint Input 3MF MMU + Support Co-Presence Tests

## Step 1 — TDD-RED: Write 4 failing test stubs

**Task IDs:** TASK-180b  
**Objective:** Add 4 new test functions to `model_loader_tdd.rs` that compile but fail (or reveal parser defects).  

**Precondition:** All 8 packet-50 paint tests pass on current HEAD.  
`cargo test -p slicer-host --test model_loader_tdd` → all green before starting.

**Postcondition:** 4 new test functions compile; each asserts on `benchy_4color.3mf` load result.

**Files allowed to read:**
- `crates/slicer-host/tests/model_loader_tdd.rs` lines 1–50 (imports + helper)
- `crates/slicer-host/tests/model_loader_tdd.rs` — locate `load_3mf_extracts_mmu_color` and `load_3mf_extracts_support_facets` (±20 lines each) for assertion pattern

**Files allowed to edit:**
- `crates/slicer-host/tests/model_loader_tdd.rs`

**Expected sub-agent dispatches:**
- Delegate `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_extracts_mmu_color --nocapture` and return FACT: which `PaintValue::ToolIndex(n)` value the existing test asserts (to calibrate AC-2).

**Context cost:** S

**Test functions to add:**

```rust
#[test]
fn load_3mf_4color_has_mmu_and_support_layers() {
    let mesh = load_model("resources/benchy_4color.3mf").unwrap();
    let pd = mesh.paint_data.as_ref().expect("paint_data must be Some");
    assert!(pd.layers.iter().any(|l| l.semantic == PaintSemantic::Material),
        "expected Material layer");
    assert!(pd.layers.iter().any(|l| matches!(l.semantic,
        PaintSemantic::SupportEnforcer | PaintSemantic::SupportBlocker)),
        "expected support layer");
}

#[test]
fn load_3mf_4color_material_spans_four_tool_indices() {
    let mesh = load_model("resources/benchy_4color.3mf").unwrap();
    let pd = mesh.paint_data.as_ref().unwrap();
    let mat = pd.layers.iter().find(|l| l.semantic == PaintSemantic::Material)
        .expect("no Material layer");
    let indices: std::collections::HashSet<u32> = mat.facet_values.iter()
        .filter_map(|v| if let Some(PaintValue::ToolIndex(n)) = v { Some(*n) } else { None })
        .collect();
    assert!(indices.len() >= 4,
        "expected ≥4 distinct ToolIndex values, got {}: {:?}", indices.len(), indices);
}

#[test]
fn load_3mf_4color_support_enforcer_has_facets() {
    let mesh = load_model("resources/benchy_4color.3mf").unwrap();
    let pd = mesh.paint_data.as_ref().unwrap();
    let sup = pd.layers.iter()
        .find(|l| matches!(l.semantic, PaintSemantic::SupportEnforcer | PaintSemantic::SupportBlocker))
        .expect("no support layer");
    let has_any = sup.facet_values.iter()
        .any(|v| matches!(v, Some(PaintValue::Flag(true))));
    assert!(has_any, "support layer has no painted facets");
}

#[test]
fn load_3mf_4color_layer_count_at_least_two() {
    let mesh = load_model("resources/benchy_4color.3mf").unwrap();
    let pd = mesh.paint_data.as_ref().unwrap();
    assert!(pd.layers.len() >= 2,
        "expected ≥2 layers, got {}", pd.layers.len());
}
```

**Authoritative docs:** `docs/02_ir_schemas.md` (PaintSemantic variants), `crates/slicer-ir/src/slice_ir.rs:188-199` (PaintValue variants)

**Narrow verification:**
```
cargo check -p slicer-host
```
Exit condition: compiles without error. Tests may fail at this stage — that is expected.

---

## Step 2 — TDD-GREEN: Run tests; fix parser defect if needed

**Task IDs:** TASK-180b  
**Objective:** All 4 new tests pass. If they pass immediately, confirm assertions are meaningful (non-trivially true). If any fails, localize the defect to `model_loader.rs` and fix.

**Precondition:** Step 1 complete; 4 new test functions compile.  
**Postcondition:** All 4 new tests pass; assertion content is verified to be non-trivially true (layer counts, ToolIndex set size, flag presence logged via `--nocapture`).

**Files allowed to read:**
- `crates/slicer-host/src/model_loader.rs:490-600` (paint assembly loop — only if a test fails)

**Files allowed to edit:**
- `crates/slicer-host/src/model_loader.rs` — only if a test fails and the defect is localized here
- `crates/slicer-host/tests/model_loader_tdd.rs` — if an assertion needs correction based on actual ToolIndex values observed

**Expected sub-agent dispatches:**
1. Delegate `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers load_3mf_4color_material_spans_four_tool_indices load_3mf_4color_support_enforcer_has_facets load_3mf_4color_layer_count_at_least_two --nocapture` and return FACT: pass/fail + failing assertion text (≤5 lines).
2. If fail: delegate read of `model_loader.rs:490-600` for the paint assembly loop; return SNIPPETS showing where multi-channel layers are pushed.

**Context cost:** M (may require reading and editing model_loader.rs if defect found)

**Risk gate:** If `benchy_4color.3mf` causes `load_model` to return `Err` (e.g., subdivision rejection), stop here, record the blocker in `packet.spec.md`, and revert to `status: draft`. Do not implement subdivision support — that is out of scope.

**Narrow verification:**
```
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers load_3mf_4color_material_spans_four_tool_indices load_3mf_4color_support_enforcer_has_facets load_3mf_4color_layer_count_at_least_two --nocapture
```
Exit condition: all 4 PASS.

---

## Step 3 — Regression check

**Task IDs:** TASK-180b  
**Objective:** Confirm no packet-50 paint test was broken.

**Precondition:** Step 2 complete; 4 new tests pass.  
**Postcondition:** All 12 paint-related tests in `model_loader_tdd.rs` pass (8 from packet 50 + 4 new).

**Files allowed to read:** none  
**Files allowed to edit:** none

**Expected sub-agent dispatches:**
- Delegate `cargo test -p slicer-host --test model_loader_tdd` and return FACT: pass/fail + names of any FAILED tests.

**Context cost:** S

**Narrow verification:**
```
cargo test -p slicer-host --test model_loader_tdd
```
Exit condition: `test result: ok.` — zero FAILED.

---

## Step 4 — Manual GCode output

**Task IDs:** TASK-180b  
**Objective:** Run slicer CLI on `benchy_4color.3mf`, output first 100 lines of GCode into the conversation for user to inspect in their slicer.

**Precondition:** Step 3 complete; all tests pass.  
**Postcondition:** GCode artifact at `target/benchy_4color_manual_test.gcode`; first 100 lines printed in conversation.

**Files allowed to read:**
- `target/benchy_4color_manual_test.gcode` (after CLI run, first 100 lines only)

**Files allowed to edit:** none

**Expected sub-agent dispatches:**
- Delegate CLI run and return FACT: exit code + file size in bytes.
- Then: read `target/benchy_4color_manual_test.gcode` lines 1–100 and paste into conversation verbatim.

**Context cost:** S

**Run command (PowerShell):**
```powershell
cargo run --bin slicer-cli --release --slice --input resources/benchy_4color.3mf --output target/benchy_4color_manual_test.gcode
Get-Content target/benchy_4color_manual_test.gcode -TotalCount 100
```

Exit condition: CLI exits 0, output file is non-empty, first 100 lines printed in conversation.

**User action:** Copy the GCode from the conversation and load into your slicer (OrcaSlicer, PrusaSlicer, Bambu Studio, etc.) to verify MMU color changes, support structures, and any artifacts.

---

## Step 5 — Lint gate

**Task IDs:** TASK-180b  
**Objective:** No new clippy warnings in `slicer-host`.

**Precondition:** Steps 1–4 complete.  
**Postcondition:** `cargo clippy -p slicer-host -- -D warnings` exits 0.

**Files allowed to read:** none  
**Files allowed to edit:** `crates/slicer-host/tests/model_loader_tdd.rs` (lint fixes only)

**Expected sub-agent dispatches:**
- Delegate `cargo clippy -p slicer-host -- -D warnings` and return FACT: pass/fail + first warning if any.

**Context cost:** S

**Narrow verification:**
```
cargo clippy -p slicer-host -- -D warnings
```
Exit condition: exit code 0, no warnings.

---

## Packet Completion Gate

All steps complete when:
1. `cargo test -p slicer-host --test model_loader_tdd` → `test result: ok.` (12 tests)
2. `cargo clippy -p slicer-host -- -D warnings` → exit 0
3. `cargo check --workspace` → exit 0
4. GCode first 100 lines have been printed in conversation for user inspection
5. `packet.spec.md` updated to `status: implemented`

Do NOT run `cargo test --workspace` — not required at this packet boundary.
