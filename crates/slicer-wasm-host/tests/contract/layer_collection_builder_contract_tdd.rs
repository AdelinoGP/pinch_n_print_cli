//! Direct host-trait contract coverage for `layer-collection-builder`.

#![allow(missing_docs)]

use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostLayerCollectionBuilder;
use slicer_wasm_host::host::HostExecutionContextBuilder;
use wasmtime::component::Resource;

fn own<T>(rep: u32) -> Resource<T> {
    Resource::new_own(rep)
}

#[test]
fn layer_collection_builder_contract() {
    let mut ctx = HostExecutionContextBuilder::new("layer-collection-contract", 0.2, 0.2).build();
    let builder = ctx.push_layer_collection_builder(Vec::new()).unwrap();
    let rep = builder.rep();
    let order = vec![(1, true), (0, false)];

    assert_eq!(
        HostLayerCollectionBuilder::set_entity_order(&mut ctx, own(rep), order.clone()).unwrap(),
        Ok(())
    );
    assert_eq!(ctx.layer_collection_proposal(), Some(&order));
    assert!(
        HostLayerCollectionBuilder::get_ordered_entities(&mut ctx, own(rep))
            .unwrap()
            .is_empty()
    );

    HostLayerCollectionBuilder::drop(&mut ctx, own(rep)).unwrap();
    assert!(HostLayerCollectionBuilder::get_ordered_entities(&mut ctx, own(rep)).is_err());
}

#[test]
fn layer_collection_builder_rejects_stale_set_entity_order_handle() {
    let mut ctx = HostExecutionContextBuilder::new("layer-collection-contract", 0.2, 0.2).build();
    let builder = ctx.push_layer_collection_builder(Vec::new()).unwrap();
    let rep = builder.rep();

    HostLayerCollectionBuilder::drop(&mut ctx, own(rep)).unwrap();

    assert!(
        HostLayerCollectionBuilder::set_entity_order(&mut ctx, own(rep), vec![(1, true)],).is_err()
    );
    assert_eq!(ctx.layer_collection_proposal(), None);
}
