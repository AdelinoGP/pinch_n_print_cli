#![allow(missing_docs)]

//! TDD red tests for packet `40_finalization-mutation-builder`.
//!
//! These tests are EXPECTED to fail to compile until Step 2 lands
//! (adds `ExtrusionRole::default_priority() -> u32` to slice_ir.rs).
//!
//! Acceptance criteria exercised:
//!   - AC-5: Skirt < OuterWall < InnerWall < SparseInfill < TopSolidInfill
//!     < Ironing < WipeTower, with >= 100 gap between every adjacent pair in
//!     the full 14-variant sorted priority list.

use slicer_ir::ExtrusionRole;

// ============================================================================
// Tests
// ============================================================================

#[test]
fn default_priority_orders_correctly() {
    // --- Collect priorities for all variants ---
    let all_variants: &[(ExtrusionRole, &str)] = &[
        (ExtrusionRole::Skirt, "Skirt"),
        (ExtrusionRole::OuterWall, "OuterWall"),
        (ExtrusionRole::InnerWall, "InnerWall"),
        (ExtrusionRole::ThinWall, "ThinWall"),
        (ExtrusionRole::SparseInfill, "SparseInfill"),
        (ExtrusionRole::BridgeInfill, "BridgeInfill"),
        (ExtrusionRole::BottomSolidInfill, "BottomSolidInfill"),
        (ExtrusionRole::TopSolidInfill, "TopSolidInfill"),
        (ExtrusionRole::SupportMaterial, "SupportMaterial"),
        (ExtrusionRole::SupportInterface, "SupportInterface"),
        (ExtrusionRole::Ironing, "Ironing"),
        (ExtrusionRole::WipeTower, "WipeTower"),
        (ExtrusionRole::PrimeTower, "PrimeTower"),
        (ExtrusionRole::Custom("test".to_string()), "Custom"),
    ];

    // --- Compute priorities ---
    let priorities: Vec<(u32, &str)> = all_variants
        .iter()
        .map(|(variant, name)| (variant.default_priority(), *name))
        .collect();

    // --- AC-5: Strict ordering for the required chain ---
    let p_skirt = ExtrusionRole::Skirt.default_priority();
    let p_outer = ExtrusionRole::OuterWall.default_priority();
    let p_inner = ExtrusionRole::InnerWall.default_priority();
    let p_sparse = ExtrusionRole::SparseInfill.default_priority();
    let p_top_solid = ExtrusionRole::TopSolidInfill.default_priority();
    let p_ironing = ExtrusionRole::Ironing.default_priority();
    let p_wipe = ExtrusionRole::WipeTower.default_priority();

    assert!(
        p_skirt < p_outer,
        "Expected Skirt({}) < OuterWall({})",
        p_skirt,
        p_outer
    );
    assert!(
        p_outer < p_inner,
        "Expected OuterWall({}) < InnerWall({})",
        p_outer,
        p_inner
    );
    assert!(
        p_inner < p_sparse,
        "Expected InnerWall({}) < SparseInfill({})",
        p_inner,
        p_sparse
    );
    assert!(
        p_sparse < p_top_solid,
        "Expected SparseInfill({}) < TopSolidInfill({})",
        p_sparse,
        p_top_solid
    );
    assert!(
        p_top_solid < p_ironing,
        "Expected TopSolidInfill({}) < Ironing({})",
        p_top_solid,
        p_ironing
    );
    assert!(
        p_ironing < p_wipe,
        "Expected Ironing({}) < WipeTower({})",
        p_ironing,
        p_wipe
    );

    // --- >= 100 gap invariant across ALL variants sorted by priority ---
    let mut sorted = priorities.clone();
    sorted.sort_by_key(|(p, _)| *p);

    for window in sorted.windows(2) {
        let (p_lo, name_lo) = window[0];
        let (p_hi, name_hi) = window[1];
        let gap = p_hi.saturating_sub(p_lo);
        assert!(
            gap >= 100,
            "Gap between {}({}) and {}({}) is {} — must be >= 100",
            name_lo,
            p_lo,
            name_hi,
            p_hi,
            gap
        );
    }

    // --- Determinism: calling default_priority() twice returns the same value ---
    assert_eq!(
        ExtrusionRole::Skirt.default_priority(),
        ExtrusionRole::Skirt.default_priority(),
        "default_priority() must be deterministic"
    );
    assert_eq!(
        ExtrusionRole::Custom("test".to_string()).default_priority(),
        ExtrusionRole::Custom("test".to_string()).default_priority(),
        "default_priority() must be deterministic for Custom"
    );
}
