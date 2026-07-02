//! AC4 (studio packet 60_phase4-triangle-selector-storage) — backend acceptance:
//! the backend `TriangleSelector` hex decoder must accept per-facet paint hex
//! strings byte-exactly as emitted by the studio's paint encoder.

use slicer_ir::Point3;
use slicer_model_io::loader::decode_paint_hex_strokes;

/// Studio-emitted paint hex strings for the golden fixture (leaf encodings and
/// real subdivided `paint_supports` runs). These are proven byte-exact on the
/// studio side; this test proves the backend decoder accepts them without error.
#[test]
fn paint_string_accepts_studio_output() {
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

    let studio_strings = [
        // Leaf encodings.
        "4",
        "8",
        "0C",
        "1C",
        // Real subdivided `paint_supports` runs.
        "040446400A2006040446400A6044244AA20446400AA",
        "00044434443404140943000304404304010003330034440410440534300049030049444300401033444094434433000304440944300000490330004443000450334041004903443300033",
    ];

    for hex in studio_strings {
        let result = decode_paint_hex_strokes(hex, verts, 0);
        assert!(
            result.is_ok(),
            "backend decoder rejected studio-emitted paint hex string {hex:?}: {:?}",
            result.err()
        );
    }
}
