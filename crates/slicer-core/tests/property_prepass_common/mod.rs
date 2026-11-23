//! Private shared checker and independent references for the pre-pass property
//! targets (`property_prepass`, `property_prepass_controls`).
//!
//! Mounted by both targets so every deterministic control exercises the exact
//! checker its registry property uses. This module defines no `proptest_` test
//! of its own; each target uses a subset, hence the dead-code allowance.

#![allow(dead_code)]

pub mod contact;
pub mod controls;
pub mod controls_modifier;
pub mod controls_paint;
pub mod controls_shrink;
pub mod driver;
pub mod model;
pub mod overhang;
pub mod paint;
pub mod roles;
pub mod support;
