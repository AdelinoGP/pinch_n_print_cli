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
