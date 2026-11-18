//! Registry-backed Arachne beading property: `H-ARACHNE-BEADING`.
//!
//! The property reads its exact name and its `max_beads`, `max_width_units`
//! and `max_excess_units` bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through [`slicer_sdk::test_support::property_testing::PropertyRun`],
//! so the generative body and its independent references live in the
//! target-private `property_arachne_common` module this file mounts. Nothing
//! here hardcodes a Proptest configuration, and there is deliberately **no**
//! deterministic witness body in this file: every deterministic control and
//! concrete regression lives in `property_arachne_contracts`, which runs under
//! a plain `cargo test` with no property context.
//!
//! Running this target without the registry context must fail loudly with
//! `missing_property_context`; that is the intended plain-run result, not a
//! skipped test.
//!
//! The charter, oracle and failure destination are documented in
//! `docs/testing/properties.md#h-arachne-beading`.

#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

mod property_arachne_common;

#[test]
fn proptest_arachne_beading_relations() {
    property_arachne_common::beading::run_property();
}
