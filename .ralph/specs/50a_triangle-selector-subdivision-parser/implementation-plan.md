# Implementation Plan — Packet 50a

Ordered steps. Complete each step's postcondition before moving to the next.
No step has cost L. Phase 1 (Steps 1–3) unblocks packet 50b. Phase 2 (Steps 4–6) adds stroke geometry.

---

## Step 1 — Implement `parse_nibbles` + `walk_triangle_selector_tree` + `dominant_paint_state`

**Task IDs**: TASK-180b-prereq (Phase 1)  
**Objective**: Add the three private helpers that form the core tree decoder. No callers yet.  
**Precondition**: All 8 packet-50 paint tests pass (`cargo test -p slicer-host --test model_loader_tdd`).  
**Context cost**: S

**Files allowed to read**:
- `crates/slicer-host/src/model_loader.rs` lines 640–730 (existing `hex_nibble` + `decode_paint_hex_state`)

**Files allowed to edit**:
- `crates/slicer-host/src/model_loader.rs`

**Expected sub-agent dispatches**: none

**Implementation**:

Add after the existing `hex_nibble` function:

```rust
fn parse_nibbles(hex: &str, byte_offset: usize) -> Result<Vec<u8>, ModelLoadError> {
    hex.bytes()
        .map(|b| hex_nibble(b).map_err(|e| ModelLoadError::PaintMetadata {
            reason: format!("invalid hex digit in paint state: {e}"),
            byte_offset,
        }))
        .collect()
}

fn walk_triangle_selector_tree(
    nibbles: &[u8],
    pos: &mut usize,
    states: &mut Vec<u32>,
    byte_offset: usize,
    depth: u32,
) -> Result<(), ModelLoadError> {
    if depth > 64 {
        return Err(ModelLoadError::PaintMetadata {
            reason: "TriangleSelector tree exceeds maximum depth".into(),
            byte_offset,
        });
    }
    if *pos >= nibbles.len() {
        return Err(ModelLoadError::PaintMetadata {
            reason: "unexpected end of TriangleSelector tree data".into(),
            byte_offset,
        });
    }
    let nibble = nibbles[*pos];
    *pos += 1;
    let split_type = nibble & 0x3;
    let state_bits = nibble >> 2;

    if split_type == 0 {
        // Leaf node
        let state = if state_bits == 3 {
            // Extended state: next nibble holds (state - 3)
            if *pos >= nibbles.len() {
                return Err(ModelLoadError::PaintMetadata {
                    reason: "unexpected end of TriangleSelector tree: missing extended state nibble".into(),
                    byte_offset,
                });
            }
            let ext = nibbles[*pos] as u32;
            *pos += 1;
            ext + 3
        } else {
            state_bits as u32
        };
        states.push(state);
    } else {
        // Non-leaf: recurse into split_type + 1 children
        let num_children = (split_type + 1) as usize;
        for _ in 0..num_children {
            walk_triangle_selector_tree(nibbles, pos, states, byte_offset, depth + 1)?;
        }
    }
    Ok(())
}

fn dominant_paint_state(states: &[u32]) -> u32 {
    let mut counts = std::collections::HashMap::new();
    for &s in states {
        if s != 0 {
            *counts.entry(s).or_insert(0u32) += 1;
        }
    }
    counts.into_iter().max_by_key(|&(_, c)| c).map(|(s, _)| s).unwrap_or(0)
}
```

**Authoritative docs**: none needed for this step  
**OrcaSlicer refs**: none (encoding rules extracted from prior delegation; see requirements.md)

**Narrow verification command**:
```bash
cargo check -p slicer-host
```

**Postcondition**: `cargo check` passes with no new errors. Functions are defined but uncalled.  
**Falsifying check**: `cargo check` fails → review nibble arithmetic and borrow checker errors.

---

## Step 2 — Wire Tree Walker into `decode_paint_hex_state`

**Objective**: Replace the long-string rejection with a call to `walk_triangle_selector_tree` + `dominant_paint_state`.  
**Precondition**: Step 1 complete; `cargo check` passes.  
**Context cost**: S

**Files allowed to read**:
- `crates/slicer-host/src/model_loader.rs` lines 640–730 (`decode_paint_hex_state`)

**Files allowed to edit**:
- `crates/slicer-host/src/model_loader.rs`

**Expected sub-agent dispatches**: none

**Implementation**:

In `decode_paint_hex_state`, replace the current `else` branch (for `bytes.len() > 2`):

```rust
} else {
    // TriangleSelector subdivision tree: walk DFS, return dominant state
    let nibbles = parse_nibbles(hex_str, byte_offset)?;
    let mut pos = 0;
    let mut states = Vec::new();
    walk_triangle_selector_tree(&nibbles, &mut pos, &mut states, byte_offset, 0)?;
    Ok(dominant_paint_state(&states))
}
```

**Narrow verification command**:
```bash
cargo check -p slicer-host
```

**Postcondition**: `cargo check` passes. `decode_paint_hex_state` no longer errors on strings >2 chars.  
**Falsifying check**: `cargo check` fails → type mismatch in return; check `Ok(u32)` conversion.

---

## Step 3 — Update Tests and Add Phase 1 Tests

**Objective**: Update the now-incorrect `load_3mf_subdivision_paint_rejects` test; add AC-4, AC-5, AC-6 tests; add AC-1/AC-2/AC-3 integration test loading `benchy_4color.3mf`.  
**Precondition**: Step 2 complete; `cargo check` passes.  
**Context cost**: M

**Files allowed to read**:
- `crates/slicer-host/tests/model_loader_tdd.rs` lines 1–50 (imports, helpers), 555–600 (subdivision test)
- `crates/slicer-host/tests/model_loader_tdd.rs` lines 193–350 (builder helpers `threemf_custom_paint_file`, `threemf_paint_file`)

**Files allowed to edit**:
- `crates/slicer-host/tests/model_loader_tdd.rs`

**Expected sub-agent dispatches**:
- Delegate `cargo test -p slicer-host --test model_loader_tdd` after all edits; return `FACT: pass/fail + failing test name and assertion if fail`.

**Changes**:

1. **Rename `load_3mf_subdivision_paint_rejects`** → `load_3mf_truncated_paint_tree_rejects`
   - Change fixture: use `paint_fuzzy_skin="5"` (nibble `0101`: split_type=1, declares 2 children, string ends → truncated tree error)
   - Change expected error: assert `err.to_string().contains("unexpected end")`

2. **Add `load_3mf_invalid_paint_hex_rejects`**:
   - Fixture: `threemf_paint_file(r#"<triangle v1="0" v2="1" v3="2" paint_fuzzy_skin="GG" />"#)`
   - Assert `Err(ModelLoadError::PaintMetadata { .. })` with message containing `"invalid hex digit"`

3. **Add `load_3mf_subdivision_dominant_state`**:
   - Need a synthetic subdivided tree for paint_color. Construct a 2-child tree (split_type=1):
     - Nibble 1: `0x1` (split_type=1, 2 children) — hex char `"1"`
     - Child 0 (leaf, state=0, no paint): nibble `0x0` — hex char `"0"`
     - Child 1 (leaf, state=1, T0): nibble `0x4` — hex char `"4"`
     - Full hex: `"104"` (3 chars → triggers tree walker)
   - Fixture: `threemf_paint_file(r#"<triangle v1="0" v2="1" v3="2" paint_color="104" />"#)`
   - Assert: Material layer present, `matches!(facet_values[0], Some(PaintValue::ToolIndex(_)))` — any non-zero ToolIndex; exact index depends on model_loader's state→ToolIndex mapping (confirm at implementation time)

4. **Add `load_3mf_benchy_4color_loads`** (AC-1 + AC-2 + AC-3):
   ```rust
   #[test]
   fn load_3mf_benchy_4color_loads() {
       let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/benchy_4color.3mf");
       let result = load_model(path);
       assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
       let objects = result.unwrap();
       // AC-2: Material layer present with at least one ToolIndex
       let has_material = objects.iter().any(|obj| {
           obj.mesh.paint_data.as_ref().map_or(false, |pd| {
               pd.layers.iter().any(|l| {
                   matches!(l.semantic, PaintSemantic::Material)
                   && l.facet_values.iter().any(|v| matches!(v, Some(PaintValue::ToolIndex(_))))
               })
           })
       });
       assert!(has_material, "expected Material layer with ToolIndex entries");
       // AC-3: SupportEnforcer layer present
       let has_support = objects.iter().any(|obj| {
           obj.mesh.paint_data.as_ref().map_or(false, |pd| {
               pd.layers.iter().any(|l| {
                   matches!(l.semantic, PaintSemantic::SupportEnforcer)
                   && l.facet_values.iter().any(|v| matches!(v, Some(PaintValue::Flag(true))))
               })
           })
       });
       assert!(has_support, "expected SupportEnforcer layer with Flag(true) entries");
   }
   ```

**Narrow verification command**:
```bash
cargo test -p slicer-host --test model_loader_tdd
```

**Postcondition**: All tests in `model_loader_tdd.rs` pass, including the 8 original packet-50 tests.  
**Falsifying check**: Any test failure → check tree encoding of the `"104"` synthetic fixture against the tree-walker algorithm.

---

## Step 4 — Research Split Geometry Formulas via Sub-Agent

**Objective**: Obtain the exact vertex-index formulas for OrcaSlicer split types 1, 2, 3.  
**Precondition**: Step 3 complete; all tests pass.  
**Context cost**: S (dispatch only; no file reads by implementer)

**Files allowed to read**: none directly  
**Files allowed to edit**: none

**Expected sub-agent dispatch** (REQUIRED before Step 5):
```
Question: In TriangleSelector.cpp, what are the exact child-triangle vertex formulas for
split_sides=1, split_sides=2, and split_sides=3? Given parent triangle vertices v[0], v[1], v[2]:
- Which edges are split (by midpoint) for each split_sides value?
- In what order are children emitted during DFS serialization?
Scope: OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp
Return: FACT ≤ 5 lines — one formula per split_sides value
```

**Do not read TriangleSelector.cpp directly. Sub-agent only.**

Record the returned FACT in this step's notes before proceeding to Step 5.

**Postcondition**: FACT received with 3 formulas (one per split type). No ambiguity about child order.  
**Falsifying check**: Sub-agent returns SUMMARY or SNIPPETS → reject; re-dispatch requesting FACT ≤ 5 lines.

---

## Step 5 — Implement `decode_paint_hex_strokes`

**Objective**: Add a geometry-threaded tree walker that returns sub-triangle vertex triples + states.  
**Precondition**: Step 4 complete; split geometry formulas known.  
**Context cost**: M

**Files allowed to read**:
- `crates/slicer-host/src/model_loader.rs` lines 640–760 (existing helpers + decode_paint_hex_state)
- `docs/08_coordinate_system.md` lines 1–80 (`mm_to_units`, unit convention)

**Files allowed to edit**:
- `crates/slicer-host/src/model_loader.rs`

**Expected sub-agent dispatches**: none (geometry formulas known from Step 4)

**Implementation**:

Add after `dominant_paint_state`:

```rust
// Sub-triangle geometry walker. Returns (child_triangle_verts, state) for every leaf.
// `verts` is the current triangle's three Point3 corners.
fn walk_triangle_selector_strokes(
    nibbles: &[u8],
    pos: &mut usize,
    verts: [Point3; 3],
    out: &mut Vec<([Point3; 3], u32)>,
    byte_offset: usize,
    depth: u32,
) -> Result<(), ModelLoadError> {
    if depth > 64 {
        return Err(ModelLoadError::PaintMetadata {
            reason: "TriangleSelector stroke tree exceeds maximum depth".into(),
            byte_offset,
        });
    }
    if *pos >= nibbles.len() {
        return Err(ModelLoadError::PaintMetadata {
            reason: "unexpected end of TriangleSelector stroke tree data".into(),
            byte_offset,
        });
    }
    let nibble = nibbles[*pos];
    *pos += 1;
    let split_type = nibble & 0x3;
    let state_bits = nibble >> 2;

    if split_type == 0 {
        // Leaf: extended state handling identical to tree walker
        let state = if state_bits == 3 {
            if *pos >= nibbles.len() {
                return Err(ModelLoadError::PaintMetadata {
                    reason: "unexpected end: missing extended state nibble in stroke tree".into(),
                    byte_offset,
                });
            }
            let ext = nibbles[*pos] as u32;
            *pos += 1;
            ext + 3
        } else {
            state_bits as u32
        };
        if state != 0 {
            out.push((verts, state));
        }
    } else {
        // Compute child sub-triangles using formulas from Step 4 FACT.
        // Formulas to be filled in by implementer from the Step 4 sub-agent FACT.
        // Placeholder structure — replace midpoint computations with exact formulas:
        let children = split_triangle(verts, split_type); // see note below
        for child_verts in children {
            walk_triangle_selector_strokes(nibbles, pos, child_verts, out, byte_offset, depth + 1)?;
        }
    }
    Ok(())
}

// Compute child triangles for a given split_type (1=2ch, 2=3ch, 3=4ch).
// Implementer fills exact midpoint formulas from Step 4 FACT.
fn split_triangle(verts: [Point3; 3], split_type: u8) -> Vec<[Point3; 3]> {
    let mid = |a: Point3, b: Point3| -> Point3 {
        Point3::new((a.x + b.x) / 2, (a.y + b.y) / 2, (a.z + b.z) / 2)
    };
    let [v0, v1, v2] = verts;
    match split_type {
        1 => {
            // split_sides=1 formula: replace with FACT from Step 4
            let m = mid(v0, v1);
            vec![[v0, m, v2], [m, v1, v2]]
        }
        2 => {
            // split_sides=2 formula: replace with FACT from Step 4
            let m01 = mid(v0, v1);
            let m12 = mid(v1, v2);
            vec![[v0, m01, v2], [m01, v1, m12], [m01, m12, v2]]
        }
        3 => {
            // split_sides=3 (4-way split): midpoints of all 3 edges
            let m01 = mid(v0, v1);
            let m12 = mid(v1, v2);
            let m20 = mid(v2, v0);
            vec![[v0, m01, m20], [m01, v1, m12], [m01, m12, m20], [m20, m12, v2]]
        }
        _ => vec![verts], // unreachable; split_type is 2-bit
    }
}

pub fn decode_paint_hex_strokes(
    hex: &str,
    verts: [Point3; 3],
    byte_offset: usize,
) -> Result<Vec<([Point3; 3], u32)>, ModelLoadError> {
    if hex.trim().is_empty() {
        return Ok(vec![]);
    }
    let nibbles = parse_nibbles(hex, byte_offset)?;
    let mut pos = 0;
    let mut out = Vec::new();
    walk_triangle_selector_strokes(&nibbles, &mut pos, verts, &mut out, byte_offset, 0)?;
    Ok(out)
}
```

**Authoritative docs**: `docs/08_coordinate_system.md` (unit conversion rules)  
**OrcaSlicer refs**: Step 4 FACT (applied in `split_triangle`)

**Narrow verification command**:
```bash
cargo check -p slicer-host
```

**Postcondition**: `cargo check` passes; `decode_paint_hex_strokes` is defined and correct per FACT.  
**Falsifying check**: `cargo check` fails → check `Point3` arithmetic and return types.

---

## Step 6 — Wire Stroke Decoder into Model-Loader Loop + Add Phase 2 Tests

**Objective**: In the triangle-processing loop, call `decode_paint_hex_strokes` for subdivided triangles and populate `PaintLayer.strokes`. Add AC-7, AC-8, AC-9 tests.  
**Precondition**: Step 5 complete; `cargo check` passes.  
**Context cost**: M

**Files allowed to read**:
- `crates/slicer-host/src/model_loader.rs` lines 340–530 (triangle loop, PaintLayer construction)
- `crates/slicer-ir/src/slice_ir.rs` lines covering `PaintStroke`, `PaintLayer` (read only if needed; delegate if > 40 lines of context)

**Files allowed to edit**:
- `crates/slicer-host/src/model_loader.rs`
- `crates/slicer-host/tests/model_loader_tdd.rs`

**Expected sub-agent dispatches**:
- After all edits: delegate `cargo test -p slicer-host --test model_loader_tdd` and return `FACT: pass/fail + failing test name and assertion if fail`.

**Model-loader wiring**:

In the triangle loop, after collecting `color_state`, `support_state`, etc. per triangle, also collect strokes:

```rust
// Collect strokes for subdivided paint channels (hex len > 2 only)
if color_hex.len() > 2 && dominant_color_state != 0 {
    let triangle_verts = [
        mesh_vertices[v1_idx],
        mesh_vertices[v2_idx],
        mesh_vertices[v3_idx],
    ];
    // Apply mm_to_units() per docs/08_coordinate_system.md
    let scaled_verts = triangle_verts.map(|v| Point3::new(
        mm_to_units(v.x),
        mm_to_units(v.y),
        mm_to_units(v.z),
    ));
    if let Ok(pairs) = decode_paint_hex_strokes(color_hex, scaled_verts, byte_offset) {
        for (sub_verts, state) in pairs {
            color_strokes.push(PaintStroke {
                triangles: vec![sub_verts],
                semantic: PaintSemantic::Material,
                value: PaintValue::ToolIndex(state.saturating_sub(1)),
            });
        }
    }
}
// (Repeat pattern for paint_supports if needed)
```

After the triangle loop, when constructing `PaintLayer` for Material, set
`.strokes = color_strokes` (replacing the empty `Vec::new()`).

**Phase 2 tests** (add to `model_loader_tdd.rs`):

```rust
#[test]
fn load_3mf_benchy_4color_strokes_populated() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/benchy_4color.3mf");
    let objects = load_model(path).expect("should load without error");
    let has_strokes = objects.iter().any(|obj| {
        obj.mesh.paint_data.as_ref().map_or(false, |pd| {
            pd.layers.iter().any(|l| {
                matches!(l.semantic, PaintSemantic::Material) && !l.strokes.is_empty()
            })
        })
    });
    assert!(has_strokes, "expected non-empty strokes in Material layer");
    // AC-8: all stroke triangles are non-degenerate
    for obj in &objects {
        if let Some(pd) = &obj.mesh.paint_data {
            for layer in &pd.layers {
                for stroke in &layer.strokes {
                    for tri in &stroke.triangles {
                        let [a, b, c] = tri;
                        assert!(a != b || b != c, "degenerate stroke triangle found");
                    }
                }
            }
        }
    }
}

#[test]
fn load_3mf_wholefacet_has_no_strokes() {
    let triangle_xml = r#"<triangle v1="0" v2="1" v3="2" paint_color="4" />"#;
    let file = threemf_paint_file(triangle_xml);
    let objects = load_model(file.path().to_str().unwrap()).expect("should load");
    for obj in &objects {
        if let Some(pd) = &obj.mesh.paint_data {
            for layer in &pd.layers {
                assert!(layer.strokes.is_empty(),
                    "whole-facet paint should produce no strokes, semantic={:?}", layer.semantic);
            }
        }
    }
}
```

**Narrow verification command**:
```bash
cargo test -p slicer-host --test model_loader_tdd
```

**Postcondition**: All tests pass; `load_3mf_benchy_4color_strokes_populated` passes (has_strokes = true).  
**Falsifying check**: `has_strokes` is false → check whether color_hex.len() > 2 condition is triggered; add a debug print to verify subdivided triangles are being processed.

---

## Step 7 — Packet Completion Gate

**Objective**: Confirm all ACs pass, no regressions, code meets quality standards.  
**Precondition**: Steps 1–6 complete; all targeted tests pass.  
**Context cost**: S (delegate all commands)

**Sub-agent dispatches**:

1. `FACT: cargo clippy --workspace -- -D warnings` → pass/fail + first error if fail
2. `FACT: cargo check --workspace` → pass/fail
3. `FACT: cargo test -p slicer-host --test model_loader_tdd` → pass/fail + list of failing tests if any

If all three pass:

4. `FACT: cargo test --workspace` → pass/fail + summary line (test counts)

**Postcondition**: All pass. AC-1 through AC-9 verifiable by targeted commands.  
**Falsifying check**: Any fail → do NOT mark packet closed; fix the issue and re-run Step 7 dispatches.

**Packet close signal**: All 4 dispatches return `FACT: pass`. Update `packet.spec.md` to `status: implemented`.
