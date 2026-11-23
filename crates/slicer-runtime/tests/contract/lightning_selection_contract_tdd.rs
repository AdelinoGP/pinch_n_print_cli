//! Lightning preparation selection and store-ownership contract tests.
//!
//! Moved out of the e2e module by wayfinder perf-vs-orca ticket 119: these
//! claims take no slice, so they belong in contract scope. The e2e module keeps
//! the two real-print selection legs (inactive owner, Ready/no-work).

#![allow(clippy::too_many_lines)]

use slicer_ir::{ConfigView, GlobalLayer, ModuleId};
use slicer_schema::preparation::{PreparationDeclaration, PreparationTransportError};
use slicer_wasm_host::preparation::{
    freeze_native, NativePreparationArtifact, PreparationFailure, PreparedOwnerStore,
};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput,
};

/// The migrated owner under test.
const MODULE_ID: &str = "com.core.lightning-infill";
/// The projection view the owner declares.
const VIEW: &str = "planning_geometry";

/// The shared preparation predicate's target side, evaluated on the real
/// selection builder with a committed schedule and no committed slice regions.
///
/// This is `selection.invoke && !selection.targets.is_empty()` with the target
/// side empty, independent of whether a whole print can reach that shape. The
/// conjunction itself is the production gate's, not this test's: asserting
/// `invoke && !targets.is_empty()` here would only restate test-local Boolean
/// arithmetic. The e2e module exercises the gate end to end through the
/// inactive-owner and Ready/no-work print legs.
#[test]
fn selection_predicate_is_false_for_a_targetless_layer() {
    let module_id: ModuleId = MODULE_ID.into();
    let claims = vec!["claim:sparse-fill".to_string()];
    let invocation_config = ConfigView::new();
    let index = build_layer_selection_index(
        // exhaustive: the predicate fixture states all selection input authority.
        LayerSelectionIndexInput {
            stage_roster: slicer_schema::STAGES,
            global_layers: &[GlobalLayer {
                index: 0,
                ..GlobalLayer::default()
            }],
            modules: &[
                // exhaustive: the predicate fixture states every bound module field.
                LayerSelectionModuleInput {
                    stage_id: "Layer::Infill",
                    module_id: MODULE_ID,
                    claims: &claims,
                    paint_semantics: None,
                    invocation_config: &invocation_config,
                },
            ],
            slices: &[],
            region_map: None,
            support_plan: None,
            perimeters: &[],
        },
    )
    .expect("a committed schedule with no slices is a valid selection input");
    let selection = index
        .selection_for(0, &module_id)
        .expect("the module is bound to the layer");
    assert!(
        selection.invoke,
        "the layer is invoked for the bound module"
    );
    assert!(
        selection.targets.is_empty(),
        "no committed region means no target: {:?}",
        selection.targets
    );
}

/// A store refuses another print's owner as `InvalidOwner`, and a disposed
/// owner's handles are `AccessDenied`.
///
/// Driven through the already-public store and the module's real frozen pair.
/// No test-only injector is involved: the piece is staged and opened by the
/// store, then refused after disposal.
#[test]
fn foreign_and_disposed_owners_are_refused() {
    let (module_id, entry, schema) = slicer_integrated_modules::prepared_native_entries()
        .into_iter()
        .find(|(module_id, _, _)| module_id == MODULE_ID)
        .expect("the module's generated prepared pair must be exported");
    assert_eq!(module_id, MODULE_ID);
    let interface = schema
        .qualified_export
        .strip_suffix("#prepare-print")
        .expect("the schema names the capability's qualified export");
    let declaration = PreparationDeclaration::new(
        interface,
        vec!["layer_plan".into(), "slices".into(), "region_map".into()],
        vec![VIEW.into()],
    );
    let binding = freeze_native(
        MODULE_ID.into(),
        Some(declaration),
        NativePreparationArtifact::Prepared { entry, schema },
    )
    .expect("the module's real prepared pair must freeze");
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(&binding).expect("owner creation");
    let mut transaction = store.begin(&owner).expect("staging begins");
    transaction
        .put("lightning-layer-0", b"probe")
        .expect("stage one piece");
    let plan = transaction
        .finish(Ok(()), |_| Ok(()))
        .expect("the staged piece publishes");
    let piece = plan
        .open("lightning-layer-0")
        .expect("a live owner opens its piece")
        .expect("the staged piece exists");
    assert_eq!(
        piece.read(0, 5).expect("a live owner reads its piece"),
        b"probe".to_vec()
    );

    // Foreign owner: an owner created by another print's store must be refused
    // as `InvalidOwner` before any authority lookup.
    let foreign_store = PreparedOwnerStore::new();
    let foreign_owner = foreign_store
        .create_owner(&binding)
        .expect("the foreign store creates its own owner");
    assert!(
        matches!(
            store.begin(&foreign_owner),
            Err(PreparationFailure::InvalidOwner)
        ),
        "a store must refuse staging for another print's owner"
    );
    assert!(
        matches!(
            store.current(&foreign_owner),
            Err(PreparationFailure::InvalidOwner)
        ),
        "a store must refuse plan authority for another print's owner"
    );
    assert!(
        matches!(
            store.dispose_owner(&foreign_owner),
            Err(PreparationFailure::InvalidOwner)
        ),
        "a store must refuse to dispose another print's owner"
    );
    foreign_store.dispose().expect("the foreign print disposes");

    // Disposed owner: the same handle must be refused after disposal. This is a
    // disposal test, not a cross-owner test.
    store.dispose_owner(&owner).expect("an idle owner disposes");
    assert_eq!(
        owner.metadata().retained_payload_bytes(),
        0,
        "disposal releases the staged payload"
    );
    assert!(
        matches!(
            piece.read(0, 5),
            Err(PreparationTransportError::AccessDenied)
        ),
        "a disposed owner's piece handle is refused"
    );
    assert!(
        matches!(
            plan.open("lightning-layer-0"),
            Err(PreparationTransportError::AccessDenied)
        ),
        "a disposed owner's plan authority is refused"
    );
    assert!(
        matches!(store.current(&owner), Err(PreparationFailure::Disposed)),
        "the disposed owner is no longer current"
    );
    store.dispose().expect("the print authority disposes");
}
