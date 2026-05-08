//! TDD anchoring tests for Packet 42 — paint-region transport widening (SDK side).
//!
//! Module layout:
//!   - `doc_grep_tests`    — file-string-grep tests; no new types needed; compile NOW.
//!   - `transport_round_trip_tests` — tests that reference `ExPolygonView`,
//!     `PaintValueInput`, and the widened `push_paint_region` signature.
//!
//! The round-trip module is gated behind `#[cfg(feature = "transport_widened")]`
//! so the grep tests compile and run even when the new types don't exist yet.
//! The compile failure on `--features transport_widened` IS the RED state.
//! The `doc_grep_tests` module runs without any feature flag.

// ── doc_grep_tests ────────────────────────────────────────────────────────────
mod doc_grep_tests {
    /// AC-3 (file-grep): `contour_points` field and parameter must be fully
    /// removed from `prepass_builders.rs` before this test goes green.
    /// RED state until Step 3 removes the field.
    #[test]
    fn contour_points_api_is_fully_removed() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir).join("src/prepass_builders.rs");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read prepass_builders.rs at {path:?}: {e}"));
        assert!(
            !src.contains("contour_points"),
            "prepass_builders.rs still contains 'contour_points'; Step 3 must remove the field \
             and the parameter"
        );
    }
}

// ── transport_round_trip_tests ─────────────────────────────────────────────────
//
// Gated behind `#[cfg(feature = "transport_widened")]` to prevent compile failure
// from blocking the doc_grep_tests above.
//
// These tests reference types that DO NOT EXIST YET:
//   - `slicer_sdk::prepass_builders::ExPolygonView`
//   - `slicer_sdk::prepass_builders::PaintValueInput`
//   - the widened `PaintSegmentationOutput::push_paint_region` signature
//
// To exercise RED state: `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd
//   --features slicer-sdk/transport_widened`
// (will fail to compile until Step 3 adds the types)

mod transport_round_trip_tests {
    use slicer_sdk::prepass_builders::{ExPolygonView, PaintSegmentationOutput, PaintValueInput};

    // ── AC-1 ─────────────────────────────────────────────────────────────────
    /// `PaintRegionEntry` must carry `polygons: Vec<ExPolygonView>` where
    /// `ExPolygonView` has `pub contour: Vec<[f64;2]>` and
    /// `pub holes: Vec<Vec<[f64;2]>>`.
    ///
    /// Compile-fail ("cannot find type `ExPolygonView`") is the RED state.
    #[test]
    fn sdk_paint_region_entry_carries_expolygon_view() {
        use slicer_sdk::prepass_builders::PaintRegionEntry;

        let poly = ExPolygonView {
            contour: vec![[0.0_f64, 0.0_f64]],
            holes: vec![],
        };
        let entry = PaintRegionEntry {
            layer_index: 0,
            semantic: "material".to_string(),
            object_id: "o1".to_string(),
            value: PaintValueInput::ToolIndex(1),
            paint_order: 0,
            polygons: vec![poly],
        };
        assert_eq!(entry.polygons[0].contour.len(), 1);
    }

    // ── AC-2 ─────────────────────────────────────────────────────────────────
    /// `PaintSegmentationOutput::push_paint_region` must accept
    /// `polygons: Vec<ExPolygonView>` (including non-empty `holes`).
    ///
    /// Compile-fail is the RED state.
    #[test]
    fn sdk_push_paint_region_preserves_holes_and_typed_value() {
        let mut builder = PaintSegmentationOutput::new();
        let outer = vec![[0.0_f64, 0.0_f64], [10.0, 0.0], [5.0, 10.0]];
        let inner = vec![[3.0_f64, 3.0_f64], [6.0, 3.0], [4.5, 6.0]];
        let poly = ExPolygonView {
            contour: outer,
            holes: vec![inner],
        };
        builder.push_paint_region(
            0,
            "material".to_string(),
            "o1".to_string(),
            0,
            PaintValueInput::ToolIndex(2),
            vec![poly],
        );
        let regions = builder.regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].polygons[0].holes.len(), 1);
        assert!(
            matches!(regions[0].value, PaintValueInput::ToolIndex(2)),
            "value must be ToolIndex(2), got {:?}",
            regions[0].value
        );
    }
}
