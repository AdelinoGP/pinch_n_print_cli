//! Private shared checkers and references for the geometry property targets.
//!
//! Mounted by both `property_geometry` (registry properties) and
//! `property_geometry_controls` (deterministic controls), so a control always
//! exercises the exact checker its property uses. This module defines no
//! `proptest_` tests of its own; each target uses a subset, hence the
//! dead-code allowance.

#![allow(dead_code)]

pub mod boolean;
pub mod bounds;
pub mod clip;
pub mod controls_boolean_clip;
pub mod controls_offset_smooth;
pub mod controls_shrink;
pub mod controls_slice_query;
pub mod controls_vertex_plane_hole;
pub mod envelopes;
pub mod mesh_query;
pub mod mesh_slice;
pub mod offset;
pub mod rect;
#[cfg(feature = "property_testing_scad")]
pub mod scad_boolean;
#[cfg(feature = "property_testing_scad")]
pub mod scad_mesh;
pub mod smooth;
