//! Contract coverage for `module_id_matches_holder`
//! (`crates/slicer-scheduler/src/validation.rs`).
//!
//! Packet 246 (AC-3): the wave-overhang bridge-fill module must be selectable
//! as the `bridge_fill_holder` via its short name (`wave-overhangs`) as well
//! as its full ID, per the `com.core.` namespace-stripping rule documented in
//! `docs/03_wit_and_manifest.md` "Holder identifier matching".

use slicer_ir::ResolvedConfig;
use slicer_scheduler::validation::module_id_matches_holder;

#[test]
fn module_id_matches_holder_wave_overhangs() {
    // Short-name form: `bridge_fill_holder = "wave-overhangs"`.
    assert!(
        module_id_matches_holder("com.core.wave-overhangs", "wave-overhangs"),
        "short holder name must match the com.core.-namespaced module ID"
    );

    // Full-ID form must keep working.
    assert!(
        module_id_matches_holder("com.core.wave-overhangs", "com.core.wave-overhangs"),
        "full module ID must match itself"
    );

    // Negative: an unrelated holder must not select this module.
    assert!(
        !module_id_matches_holder("com.core.wave-overhangs", "rectilinear-infill"),
        "an unrelated holder name must not match wave-overhangs"
    );
    assert!(
        !module_id_matches_holder("com.core.wave-overhangs", "com.core.rectilinear-infill"),
        "an unrelated full module ID must not match wave-overhangs"
    );
}

/// AC-3, second half: with **no** `bridge_fill_holder` override, the default
/// holder of `claim:bridge-fill` is still `rectilinear-infill`. Adding the
/// wave-overhangs module must not silently re-point existing users' bridge fill.
///
/// The default lives on `ResolvedConfig::bridge_fill_holder`, declared through
/// the `declare_resolved_config!` invocation in
/// `crates/slicer-ir/src/resolved_config.rs`; this asserts against that value
/// rather than restating the literal in a second place only.
#[test]
fn default_bridge_fill_holder_is_rectilinear_infill() {
    let default_holder = ResolvedConfig::default().bridge_fill_holder;

    assert_eq!(
        default_holder, "rectilinear-infill",
        "the un-overridden bridge-fill holder must remain rectilinear-infill"
    );

    // ...and that default must resolve to the rectilinear module, not to
    // wave-overhangs.
    assert!(
        module_id_matches_holder("com.core.rectilinear-infill", &default_holder),
        "the default holder must select the rectilinear-infill module"
    );
    assert!(
        !module_id_matches_holder("com.core.wave-overhangs", &default_holder),
        "wave-overhangs must never be selected without an explicit \
         bridge_fill_holder override"
    );
}
