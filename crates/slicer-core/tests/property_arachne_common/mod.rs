//! Private shared checker and controls for the Arachne beading property target.
//!
//! Mounted by both `property_arachne` (registry property) and
//! `property_arachne_contracts` (deterministic controls), so a control always
//! exercises the exact comparator its property uses. This module defines no
//! `proptest_` tests of its own; each target uses a subset, hence the dead-code
//! allowance.

#![allow(dead_code)]

pub mod beading;
pub mod controls_beading;
