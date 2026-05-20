//! TDD anchoring tests for Packet 42 — paint-region transport widening (host side).
//!
//! Module layout:
//!   - `doc_grep_tests` — file-string-grep tests; no new WIT/IR types needed;
//!     compile and run RIGHT NOW.
//!   - `transport_round_trip_tests` — end-to-end tests that reference
//!     `pm::PaintValueInput`, `PaintValue::Custom`, and the typed
//!     `paint-value-input` WIT variant on `pm::PaintRegionEntry`.
//!
//! The round-trip module is gated behind `#[cfg(feature = "transport_widened")]`
//! so the grep tests compile and run even when the new types don't exist yet.
//! The compile failure on `--features slicer-host/transport_widened` IS the RED
//! state for the unimplemented parts.
//!
//! `doc_grep_tests` must be GREEN after Step 1 docs are committed.

// ── doc_grep_tests ────────────────────────────────────────────────────────────
mod doc_grep_tests {
    use std::path::Path;

    // ── AC-host-1 ─────────────────────────────────────────────────────────────
    /// The WIT `paint-region-entry` record must have `value: paint-value-input`
    /// (typed variant), NOT `value: string`.
    ///
    /// Also checks that the variant `paint-value-input` exposes the four
    /// expected arms: `flag(bool)`, `scalar(f32)`, `tool-index(u32)`,
    /// `custom(string)`.
    ///
    /// RED until Step 4 lands the WIT widening.
    #[test]
    fn wit_paint_region_entry_value_is_typed_variant() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wit");
        let world_prepass =
            std::fs::read_to_string(base.join("world-prepass.wit")).unwrap_or_default();
        let ir_types = std::fs::read_to_string(base.join("deps/ir-types.wit")).unwrap_or_default();
        let combined = format!("{world_prepass}\n{ir_types}");

        // (a) paint-region-entry record must declare `value: paint-value-input`
        assert!(
            combined.contains("value: paint-value-input"),
            "paint-region-entry should have 'value: paint-value-input'; found `value: string` \
             or no typed value field at all"
        );

        // (b) the OLD `value: string` shape must be gone from world-prepass.wit
        assert!(
            !world_prepass.contains("value: string"),
            "world-prepass.wit still contains 'value: string'; Step 4 must retype to \
             paint-value-input"
        );

        // (c) the variant paint-value-input must declare all four arms
        for arm in &[
            "flag(bool)",
            "scalar(f32)",
            "tool-index(u32)",
            "custom(string)",
        ] {
            assert!(
                combined.contains(arm),
                "paint-value-input variant missing arm '{arm}' in wit/ files"
            );
        }
    }

    // ── AC-host-2 ─────────────────────────────────────────────────────────────
    /// The `harvest_paint_segmentation_ir` function body must contain zero
    /// occurrences of `parse_value`, `parse::<u32>()`, `parse::<f32>()` after
    /// Step 5 removes the string-coercion parser.
    ///
    /// RED until Step 5 lands the host harvest re-write.
    #[test]
    fn host_harvest_drops_string_parsing() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dispatch.rs");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read dispatch.rs: {e}"));

        // Locate the harvest function body
        let fn_start = src
            .find("fn harvest_paint_segmentation_ir")
            .expect("harvest_paint_segmentation_ir not found in dispatch.rs");
        // Heuristic end: find the next top-level `\nfn ` after the start
        let search_window = &src[fn_start..];
        let fn_end = search_window[20..] // skip past `fn` keyword
            .find("\nfn ")
            .map(|off| fn_start + 20 + off)
            .unwrap_or(src.len());
        let body = &src[fn_start..fn_end];

        for banned in &["parse_value", "parse::<u32>()", "parse::<f32>()"] {
            assert!(
                !body.contains(banned),
                "harvest_paint_segmentation_ir still contains '{banned}'; Step 5 must \
                 replace string-coercion with typed variant mapping"
            );
        }
    }

    // ── AC-host-6 ─────────────────────────────────────────────────────────────
    /// The inline WIT `paint-region-entry` record in `slicer-macros/src/lib.rs`
    /// must match the canonical `wit/world-prepass.wit` definition (whitespace
    /// stripped).
    ///
    /// RED until Steps 4 + 5 keep both in sync.
    #[test]
    fn inline_and_canonical_wit_match() {
        let canonical_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wit/world-prepass.wit");
        let macros_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/slicer-macros/src/lib.rs");

        let canonical_src = std::fs::read_to_string(&canonical_path)
            .unwrap_or_else(|e| panic!("cannot read world-prepass.wit: {e}"));
        let macros_src = std::fs::read_to_string(&macros_path)
            .unwrap_or_else(|e| panic!("cannot read slicer-macros/src/lib.rs: {e}"));

        // Extract the paint-region-entry record block from each source.
        fn extract_paint_region_entry_block(src: &str) -> String {
            let start_token = "record paint-region-entry {";
            let start = src
                .find(start_token)
                .expect("'record paint-region-entry {' not found");
            let after = &src[start..];
            let end = after
                .find('}')
                .expect("no closing '}' for paint-region-entry")
                + 1;
            after[..end].to_string()
        }

        let canonical_block = extract_paint_region_entry_block(&canonical_src);
        let inline_block = extract_paint_region_entry_block(&macros_src);

        // Strip all whitespace before comparing
        let canonical_stripped: String = canonical_block
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let inline_stripped: String = inline_block
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        assert_eq!(
            canonical_stripped, inline_stripped,
            "inline WIT paint-region-entry in slicer-macros does not match \
             canonical wit/world-prepass.wit;\n  canonical: {canonical_block}\n  \
             inline: {inline_block}"
        );
    }

    // ── AC-host-7 ─────────────────────────────────────────────────────────────
    /// `docs/07_implementation_status.md` must contain a row for TASK-130c
    /// titled "Widen paint-region transport" and must list TASK-130c in the
    /// blocker list.
    ///
    /// GREEN after Step 1.
    #[test]
    fn docs_07_registers_task_130c() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/07_implementation_status.md");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read 07_implementation_status.md: {e}"));

        // (a) A line must contain both TASK-130c and "Widen paint-region transport"
        let has_task_row = src
            .lines()
            .any(|l| l.contains("TASK-130c") && l.contains("Widen paint-region transport"));
        assert!(
            has_task_row,
            "07_implementation_status.md must have a line with TASK-130c and \
             'Widen paint-region transport'"
        );

        // (b) A line must reference TASK-130c's relationship to a blocker.
        // The task can be registered as either an open blocker
        // (`Blocking`/`blocker`) OR as the closure of a blocker
        // (`Closed`/`closed`/`Covers DEV-025`). After packet 42 the task
        // was closed; either form still satisfies the registration contract.
        let has_blocker_or_closure = src.lines().any(|l| {
            l.contains("TASK-130c")
                && (l.contains("Blocking")
                    || l.contains("blocker")
                    || l.contains("Closed")
                    || l.contains("closed")
                    || l.contains("DEV-025"))
        });
        assert!(
            has_blocker_or_closure,
            "07_implementation_status.md must reference TASK-130c as a blocker or its closure"
        );
    }

    // ── AC-host-8 ─────────────────────────────────────────────────────────────
    /// `docs/DEVIATION_LOG.md` DEV-025 entry must reference mismatches 4 and 5,
    /// plus contain the phrases "paint value" and "hole-blind".
    ///
    /// GREEN after Step 1.
    #[test]
    fn dev_log_extends_dev025_with_4_and_5() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/DEVIATION_LOG.md");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read DEVIATION_LOG.md: {e}"));

        // Locate the DEV-025 entry row — search for the row-start delimiter
        // `| DEV-025 |` to avoid false hits in rows that reference DEV-025.
        let row_marker = "| DEV-025 |";
        let dev025_start = src
            .find(row_marker)
            .expect("'| DEV-025 |' row not found in DEVIATION_LOG.md");
        let after_start = &src[dev025_start..];
        let dev025_end = after_start[row_marker.len()..] // skip past the row marker
            .find("\n| DEV-") // find the next row boundary (newline + pipe)
            .map(|off| dev025_start + row_marker.len() + off)
            .unwrap_or(src.len());
        let dev025_block = &src[dev025_start..dev025_end];

        assert!(
            dev025_block.contains("Mismatch 4"),
            "DEV-025 must reference 'Mismatch 4'"
        );
        assert!(
            dev025_block.contains("Mismatch 5"),
            "DEV-025 must reference 'Mismatch 5'"
        );
        // Case-insensitive — "Paint value" and "paint value" are both valid.
        let dev025_lower = dev025_block.to_lowercase();
        assert!(
            dev025_lower.contains("paint value"),
            "DEV-025 must contain 'paint value' (mismatch 4 description)"
        );
        assert!(
            dev025_block.contains("hole-blind"),
            "DEV-025 must contain 'hole-blind' (mismatch 5 description)"
        );
    }
}

// ── transport_round_trip_tests ─────────────────────────────────────────────────
//
// These tests reference WIT-bindgen types and IR types in their widened form:
//   - `pm::PaintRegionEntry.value` as `pm::PaintValueInput` (variant, not String)
//   - `PaintValue::Custom(String)` IR variant
//   - `dispatch_helpers::harvest_paint_segmentation_ir_from_ctx` (new helper)

mod transport_round_trip_tests {
    use slicer_host::wit_host::prepass::slicer::world_prepass::geometry as geo;
    use slicer_host::wit_host::prepass::{self as pm, HostPaintSegmentationOutput};
    use slicer_host::wit_host::{HostExecutionContext, HostExecutionContextBuilder};
    use slicer_ir::{PaintSemantic, PaintValue};
    use wasmtime::component::Resource;

    fn make_ctx() -> HostExecutionContext {
        HostExecutionContextBuilder::new("com.test.paint-transport-wide", 0.0, 0.0).build()
    }

    fn square_geo_polygon(x: i64, y: i64, side: i64) -> geo::Polygon {
        geo::Polygon {
            points: vec![
                geo::Point2 { x, y },
                geo::Point2 { x: x + side, y },
                geo::Point2 {
                    x: x + side,
                    y: y + side,
                },
                geo::Point2 { x, y: y + side },
            ],
        }
    }

    fn square_expolygon(x: i64, y: i64, side: i64) -> geo::ExPolygon {
        geo::ExPolygon {
            contour: square_geo_polygon(x, y, side),
            holes: vec![],
        }
    }

    fn square_expolygon_with_hole(
        ox: i64,
        oy: i64,
        outer: i64,
        ix: i64,
        iy: i64,
        inner: i64,
    ) -> geo::ExPolygon {
        geo::ExPolygon {
            contour: square_geo_polygon(ox, oy, outer),
            holes: vec![square_geo_polygon(ix, iy, inner)],
        }
    }

    // ── AC-host-3 ─────────────────────────────────────────────────────────────
    /// End-to-end: hole-bearing polygon + typed tool-index value round-trips
    /// through `push_paint_region` → `harvest_paint_segmentation_ir`.
    ///
    /// Compile-fail on `pm::PaintValueInput::ToolIndex(7)` is the RED state.
    #[test]
    fn hole_bearing_region_round_trips_through_typed_value() {
        use slicer_host::dispatch_helpers::harvest_paint_segmentation_ir_from_ctx;

        let mut ctx = make_ctx();
        let handle = ctx.push_paint_segmentation_output().expect("push resource");

        let entry = pm::PaintRegionEntry {
            object_id: "obj-a".into(),
            layer_index: 3,
            semantic: "material".into(),
            polygons: vec![square_expolygon_with_hole(0, 0, 100, 20, 20, 60)],
            value: pm::PaintValueInput::ToolIndex(7),
        };
        HostPaintSegmentationOutput::push_paint_region(
            &mut ctx,
            Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
            entry,
        )
        .expect("wasmtime call")
        .expect("push must succeed");

        let (ir, _rtree) = harvest_paint_segmentation_ir_from_ctx(ctx);

        let layer = ir.per_layer.get(&3).expect("layer 3 must exist");
        let regions = layer
            .semantic_regions
            .get(&PaintSemantic::Material)
            .expect("Material semantic must exist");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].polygons.len(), 1);
        assert_eq!(
            regions[0].polygons[0].holes.len(),
            1,
            "hole must be preserved through harvest"
        );
        assert_eq!(
            regions[0].value,
            PaintValue::ToolIndex(7),
            "typed tool-index must be preserved"
        );
        assert_eq!(regions[0].object_id, "obj-a");
    }

    // ── AC-host-4 ─────────────────────────────────────────────────────────────
    /// FuzzySkin ring with a hole preserves hole; parallel fixture with no hole
    /// proves hole fidelity is real (not vestigial).
    ///
    /// Compile-fail on `pm::PaintValueInput::Flag(true)` is the RED state.
    #[test]
    fn fuzzy_skin_ring_with_hole_preserves_hole() {
        use slicer_host::dispatch_helpers::harvest_paint_segmentation_ir_from_ctx;

        // ── fixture A: with hole ──────────────────────────────────────────────
        let mut ctx_a = make_ctx();
        let handle_a = ctx_a
            .push_paint_segmentation_output()
            .expect("push resource");
        HostPaintSegmentationOutput::push_paint_region(
            &mut ctx_a,
            Resource::<pm::PaintSegmentationOutput>::new_own(handle_a.rep()),
            pm::PaintRegionEntry {
                object_id: "obj-b".into(),
                layer_index: 1,
                semantic: "fuzzy_skin".into(),
                polygons: vec![square_expolygon_with_hole(0, 0, 200, 50, 50, 100)],
                value: pm::PaintValueInput::Flag(true),
            },
        )
        .expect("wasmtime call")
        .expect("push A must succeed");
        let (ir_a, _rtree) = harvest_paint_segmentation_ir_from_ctx(ctx_a);
        let regions_a = ir_a
            .per_layer
            .get(&1)
            .expect("layer 1")
            .semantic_regions
            .get(&PaintSemantic::FuzzySkin)
            .expect("FuzzySkin");
        assert_eq!(
            regions_a[0].polygons[0].holes.len(),
            1,
            "hole must be preserved"
        );

        // ── fixture B: no hole ────────────────────────────────────────────────
        let mut ctx_b = make_ctx();
        let handle_b = ctx_b
            .push_paint_segmentation_output()
            .expect("push resource");
        HostPaintSegmentationOutput::push_paint_region(
            &mut ctx_b,
            Resource::<pm::PaintSegmentationOutput>::new_own(handle_b.rep()),
            pm::PaintRegionEntry {
                object_id: "obj-c".into(),
                layer_index: 1,
                semantic: "fuzzy_skin".into(),
                polygons: vec![square_expolygon(0, 0, 200)],
                value: pm::PaintValueInput::Flag(true),
            },
        )
        .expect("wasmtime call")
        .expect("push B must succeed");
        let (ir_b, _rtree) = harvest_paint_segmentation_ir_from_ctx(ctx_b);
        let regions_b = ir_b
            .per_layer
            .get(&1)
            .expect("layer 1")
            .semantic_regions
            .get(&PaintSemantic::FuzzySkin)
            .expect("FuzzySkin");
        assert_eq!(regions_b[0].polygons[0].holes.len(), 0, "no holes expected");
    }

    // ── AC-host-5 ─────────────────────────────────────────────────────────────
    /// Custom value must NOT coerce to ToolIndex(0); the payload must survive.
    ///
    /// Compile-fail on `pm::PaintValueInput::Custom(...)` is the RED state.
    /// After Step 5 lands `PaintValue::Custom(String)`, the assertion is
    /// `value == PaintValue::Custom("profile_high".to_string())`.
    #[test]
    fn custom_value_does_not_coerce_to_tool_index_zero() {
        use slicer_host::dispatch_helpers::harvest_paint_segmentation_ir_from_ctx;

        let mut ctx = make_ctx();
        let handle = ctx.push_paint_segmentation_output().expect("push resource");
        HostPaintSegmentationOutput::push_paint_region(
            &mut ctx,
            Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
            pm::PaintRegionEntry {
                object_id: "obj-d".into(),
                layer_index: 0,
                semantic: "material".into(),
                polygons: vec![square_expolygon(0, 0, 50)],
                value: pm::PaintValueInput::Custom("profile_high".to_string()),
            },
        )
        .expect("wasmtime call")
        .expect("push must succeed");

        let (ir, _rtree) = harvest_paint_segmentation_ir_from_ctx(ctx);
        let regions = ir
            .per_layer
            .get(&0)
            .expect("layer 0")
            .semantic_regions
            .get(&PaintSemantic::Material)
            .expect("Material");

        // Must NOT coerce to ToolIndex(0)
        assert_ne!(
            regions[0].value,
            PaintValue::ToolIndex(0),
            "Custom value 'profile_high' must not coerce to ToolIndex(0)"
        );
        // After Step 5 adds PaintValue::Custom(String):
        assert_eq!(
            regions[0].value,
            PaintValue::Custom("profile_high".to_string()),
            "Custom payload must be preserved verbatim"
        );
    }
}
