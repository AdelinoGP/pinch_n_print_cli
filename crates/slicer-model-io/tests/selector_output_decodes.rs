//! AC5 (studio packet `61_phase4-selector-subdivision-cursors`, Step 4):
//! backend cross-decoder acceptance. Goal: "a selector painted via cursors
//! and serialized, when its `paint_*` strings are decoded by the backend,
//! decode succeeds and the backend facet states match the authored
//! states."
//!
//! The golden hex fixtures below are copied byte-exact from the studio
//! repo's `crates/pnp-domain/src/selector.rs`, test
//! `selector::tests::serialize_hex_is_backend_golden` (studio repo
//! `pinch_n_print_studio`). That test authors a representative
//! `TriangleSelector` tree — whole-facet `ENFORCER`/`BLOCKER`/extended-state
//! leaves, plus one 3-way-subdivided facet with mixed leaf states across its
//! 4 children — `serialize()`s it, and pins the exact per-facet 3MF hex the
//! studio's encoder emits (derived by running the test, never hand-derived).
//!
//! `slicer-model-io` does not depend on `pnp-domain` (crate layering), so
//! the studio output cannot be generated live here; it is embedded as
//! static string fixtures instead. This test proves the backend's ONLY
//! existing decoder (`decode_paint_hex_strokes` in `src/loader.rs`) — no
//! production code changes were made for this test — accepts these strings
//! and recovers the exact authored paint states.

use slicer_ir::Point3;
use slicer_model_io::loader::decode_paint_hex_strokes;

#[test]
fn selector_output_decodes() {
    // A small, arbitrary, non-degenerate triangle — geometry is irrelevant
    // to this test beyond being valid input for the decoder's midpoint
    // math; only the decoded *states* (and, for leaves, vertex passthrough)
    // are asserted.
    let verts = [
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ];

    // --- Whole-facet leaves ------------------------------------------
    // Studio-authored (from `serialize_hex_is_backend_golden`):
    // - triangle 1: `PaintState::ENFORCER` (raw 1) → hex "4".
    // - triangle 2: `PaintState::BLOCKER` (raw 2) → hex "8".
    // - triangle 3: `PaintState::extruder(3)` (raw 3, extended/two-nibble
    //   encoding) → hex "0C".
    let leaf_fixtures: &[(&str, u32)] = &[("4", 1), ("8", 2), ("0C", 3)];

    for &(hex, authored_state) in leaf_fixtures {
        let result = decode_paint_hex_strokes(hex, verts, 0);
        let strokes = result.unwrap_or_else(|e| {
            panic!("backend decoder rejected studio-emitted leaf hex {hex:?}: {e:?}")
        });

        assert_eq!(
            strokes.len(),
            1,
            "leaf hex {hex:?} must decode to exactly one facet, got {strokes:?}"
        );
        let (decoded_verts, decoded_state) = strokes[0];
        assert_eq!(
            decoded_state, authored_state,
            "leaf hex {hex:?}: backend-decoded state must match the studio-authored state"
        );
        assert_eq!(
            decoded_verts, verts,
            "leaf hex {hex:?}: an unsplit leaf's verts must pass through unchanged"
        );
    }

    // --- Subdivided facet ---------------------------------------------
    // Studio-authored (triangle 0 in `serialize_hex_is_backend_golden`): a
    // 3-way split whose 4 leaf children carry mixed states
    // `{ENFORCER=1, BLOCKER=2, extruder(4)=4, extruder(16)=16}`, serialized
    // to hex "481CDC3".
    let subdivided_hex = "481CDC3";
    let result = decode_paint_hex_strokes(subdivided_hex, verts, 0);
    let strokes = result.unwrap_or_else(|e| {
        panic!("backend decoder rejected studio-emitted subdivided hex {subdivided_hex:?}: {e:?}")
    });

    assert_eq!(
        strokes.len(),
        4,
        "subdivided hex {subdivided_hex:?} must decode to exactly 4 leaf sub-triangles, \
         got {strokes:?}"
    );

    let mut authored_states = vec![1u32, 2, 4, 16];
    authored_states.sort_unstable();
    let mut decoded_states: Vec<u32> = strokes.iter().map(|(_, state)| *state).collect();
    decoded_states.sort_unstable();

    assert_eq!(
        decoded_states, authored_states,
        "subdivided hex {subdivided_hex:?}: the multiset of backend-decoded leaf states must \
         match the studio-authored leaf-state multiset"
    );
}
