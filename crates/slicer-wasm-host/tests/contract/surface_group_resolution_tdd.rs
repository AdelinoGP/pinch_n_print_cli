//! Contract tests for `sliced_region_to_data` surface-group resolution.
//!
//! Verifies that `SurfaceClassificationIR` threaded through to `push_slice_regions`
//! causes the WIT `surface-group` field to be populated correctly (packet 104).
//!
//! Three cases:
//!   a. Happy path: region has `nonplanar_surface` matching a group in the classification.
//!   b. Mismatched/missing entry: id present in region but absent from classification.
//!   c. No nonplanar_surface: region.nonplanar_surface is None.

use std::collections::HashMap;

use slicer_ir::{ObjectSurfaceData, SurfaceClassificationIR, SurfaceGroup};
use slicer_wasm_host::host::sliced_region_to_data;

/// Build a minimal `SlicedRegion` with the given object_id and nonplanar_surface.
fn make_region(object_id: &str, nonplanar_surface: Option<u64>) -> slicer_ir::SlicedRegion {
    slicer_ir::SlicedRegion {
        object_id: object_id.to_string(),
        region_id: 1,
        nonplanar_surface,
        ..Default::default()
    }
}

/// Build a `SurfaceClassificationIR` with one object containing one surface group.
fn make_classification(
    object_id: &str,
    group_id: u64,
    z_min: f32,
    z_max: f32,
    area_mm2: f32,
    printable: bool,
    shell_count: u32,
) -> SurfaceClassificationIR {
    let mut per_object = HashMap::new();
    per_object.insert(
        object_id.to_string(),
        ObjectSurfaceData {
            surface_groups: vec![SurfaceGroup {
                id: group_id,
                facet_indices: vec![0, 1, 2],
                z_min,
                z_max,
                area_mm2,
                printable,
                shell_count,
            }],
            ..Default::default()
        },
    );
    SurfaceClassificationIR {
        per_object,
        ..Default::default()
    }
}

// ── a. Happy path ──────────────────────────────────────────────────────────────

/// Region has `nonplanar_surface = Some(id)` and the classification contains a matching
/// group for that object+id. The resulting `SliceRegionData.surface_group` must be
/// `Some(..)` with the correct id, area_mm2, and printable flag.
#[test]
fn surface_group_resolved_when_classification_matches() {
    let region = make_region("obj-1", Some(42));
    let classification = make_classification("obj-1", 42, 0.5, 2.5, 12.34, true, 3);

    let data = sliced_region_to_data(&region, 1.0, vec![], Some(&classification), 0);

    let sg = data.surface_group.expect("surface_group should be Some");
    assert_eq!(sg.id, 42, "surface group id must match");
    assert!((sg.area_mm2 - 12.34).abs() < 1e-4, "area_mm2 must match");
    assert!(sg.printable, "printable flag must match");
    assert_eq!(sg.shell_count, 3, "shell_count must match");
}

// ── b. Mismatched / missing entry ─────────────────────────────────────────────

/// Region references `nonplanar_surface = Some(99)` but the classification only has
/// group id 42 for that object. No panic; result is `None`.
#[test]
fn surface_group_none_when_id_not_found() {
    let region = make_region("obj-1", Some(99));
    let classification = make_classification("obj-1", 42, 0.0, 1.0, 1.0, false, 1);

    let data = sliced_region_to_data(&region, 1.0, vec![], Some(&classification), 0);

    assert!(
        data.surface_group.is_none(),
        "surface_group should be None when id is not in classification"
    );
}

/// Region references an object_id that has no entry in the classification at all.
/// No panic; result is `None`.
#[test]
fn surface_group_none_when_object_not_in_classification() {
    let region = make_region("obj-unknown", Some(42));
    let classification = make_classification("obj-1", 42, 0.0, 1.0, 1.0, false, 1);

    let data = sliced_region_to_data(&region, 1.0, vec![], Some(&classification), 0);

    assert!(
        data.surface_group.is_none(),
        "surface_group should be None when object_id is absent from classification"
    );
}

// ── c. No nonplanar_surface ────────────────────────────────────────────────────

/// Region has `nonplanar_surface = None`. Even if a valid classification is provided,
/// `surface_group` must be `None`.
#[test]
fn surface_group_none_when_region_has_no_nonplanar_surface() {
    let region = make_region("obj-1", None);
    let classification = make_classification("obj-1", 42, 0.0, 1.0, 1.0, true, 2);

    let data = sliced_region_to_data(&region, 1.0, vec![], Some(&classification), 0);

    assert!(
        data.surface_group.is_none(),
        "surface_group should be None when region.nonplanar_surface is None"
    );
}
