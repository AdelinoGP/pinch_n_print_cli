// overhang_areas_empty_until_p106_tdd.rs — AC-3-EMPTY regression test.
//
// Asserts that `SliceRegionView::overhang_areas()` returns an empty slice for
// any region constructed via the host populator or the `Default` constructor.
// The field will remain empty until packet 106 wires `OverhangRegion.xy_footprint`.

use slicer_sdk::views::SliceRegionView;

/// A default-constructed `SliceRegionView` must have empty `overhang_areas`.
#[test]
fn default_slice_region_view_has_empty_overhang_areas() {
    let region = SliceRegionView::default();
    assert!(
        region.overhang_areas().is_empty(),
        "overhang_areas() must be empty until packet 106; got {} items",
        region.overhang_areas().len()
    );
}

/// `surface_group()` on a default-constructed view must return `None`.
#[test]
fn default_slice_region_view_has_no_surface_group() {
    let region = SliceRegionView::default();
    assert!(
        region.surface_group().is_none(),
        "surface_group() must be None when no SurfaceClassificationIR is present"
    );
}
