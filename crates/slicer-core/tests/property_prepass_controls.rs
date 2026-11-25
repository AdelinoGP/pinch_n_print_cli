//! Deterministic controls for the pre-pass charters (`H-REGIONMAP`).
//!
//! Every control here runs under a plain
//! `cargo test -p slicer-core --features property_testing,host-algos --test property_prepass_controls`.
//! They are hand-built witnesses that prove the independent reference used by
//! the registry property rejects the faults that property must catch, that the
//! real system under test passes them on named fixtures, and that invalid
//! profile bounds fail before sampling. No control in this binary substitutes
//! for a registry execution.

#![allow(missing_docs)]

mod property_prepass_common;

#[test]
fn prepass_model_controls() {
    property_prepass_common::controls::model_controls();
}

#[test]
fn prepass_regionmap_order_and_cap_controls() {
    property_prepass_common::controls::order_and_cap_controls();
}

#[test]
fn prepass_regionmap_rejection_controls() {
    property_prepass_common::controls::rejection_controls();
}

#[test]
fn prepass_owned_shrink_and_bound_controls() {
    // AC-N2: the real value trees survive complication and simplification, plus
    // the narrowing-bound rejections.
    property_prepass_common::controls_shrink::owned_shrink_controls();
    property_prepass_common::controls_shrink::bound_rejections();
    property_prepass_common::controls::bound_controls();
}

#[test]
fn prepass_paint_partition_and_driver_controls() {
    property_prepass_common::controls_paint::paint_partition_controls();
}

#[test]
fn prepass_overhang_area_controls() {
    property_prepass_common::controls_paint::overhang_area_controls();
}

#[test]
fn prepass_support_schedule_controls() {
    property_prepass_common::controls_paint::support_schedule_controls();
}

#[test]
fn prepass_support_contact_role_controls() {
    property_prepass_common::controls_paint::contact_role_controls();
}
