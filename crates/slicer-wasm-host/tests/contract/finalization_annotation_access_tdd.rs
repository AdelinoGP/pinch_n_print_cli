//! Annotation permissions at the public native and real SDK WASM dispatch seams.
#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, ConfigView, FinalizationError, LayerAnnotationKind, LayerCollectionIR, MeshIR,
};
use slicer_wasm_host::{
    CompiledModuleLive, FinalizationStageInput, FinalizationStageRunner, WasmInstancePool,
    WasmRuntimeDispatcher,
};
use std::{collections::HashMap, sync::Arc};

fn dispatch(
    native: bool,
    writes: &[String],
    target: i64,
    layers: &mut Vec<LayerCollectionIR>,
) -> Result<slicer_ir::FinalizationOutput, FinalizationError> {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(engine.clone());
    let component = if native {
        None
    } else {
        Some(Arc::new(
            engine
                .compile_component(
                    &std::fs::read(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("test-guests/sdk-finalization-guest.component.wasm"),
                    )
                    .expect("real SDK guest required; build-guests first"),
                )
                .expect("compile SDK guest"),
        ))
    };
    let id = "annotation-access-witness".to_owned();
    let mut module = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        component,
        &[],
        Arc::new(ConfigView::from_map(HashMap::from([(
            "annotation_target".into(),
            ConfigValue::Int(target),
        )]))),
    )
    .with_ir_writes(writes);
    if native {
        module = module.with_native_entry(
            sdk_finalization_guest::SdkFinalizationModule::__slicer_native_entry(),
        );
    }
    let input = FinalizationStageInput {
        mesh: Arc::new(MeshIR::default()),
        _phantom: std::marker::PhantomData,
    };
    dispatcher.run_stage(
        &"PostPass::LayerFinalization".into(),
        &module,
        input,
        layers,
    )
}

#[test]
fn annotation_requires_exact_permission_on_both_transports_even_when_sdk_swallows_denial() {
    for native in [true, false] {
        for writes in [
            vec![],
            vec!["LayerCollectionIR.cooling".into()],
            vec!["LayerCollectionIR".into()],
            vec!["Z.unrelated".into(), "A.unrelated".into()],
        ] {
            let mut layers = vec![LayerCollectionIR::default()];
            let before = layers.clone();
            let error = dispatch(native, &writes, 0, &mut layers)
                .expect_err("undeclared annotation must fail");
            assert!(matches!(error, FinalizationError::FatalModule { .. }));
            let text = error.to_string();
            assert!(text.contains("module annotation-access-witness stage PostPass::LayerFinalization: attempted write requested path LayerCollectionIR.annotations; manifest writes="), "{text}");
            if writes.len() == 2 {
                assert!(
                    text.contains("[\"A.unrelated\", \"Z.unrelated\"]"),
                    "{text}"
                );
            }
            assert_eq!(layers, before, "denial must not commit on native={native}");
        }
    }
}

#[test]
fn exact_annotation_permission_commits_fan_and_comment_on_both_transports() {
    for native in [true, false] {
        let mut layers = vec![LayerCollectionIR::default()];
        dispatch(
            native,
            &["LayerCollectionIR.annotations".into()],
            0,
            &mut layers,
        )
        .expect("authorized dispatch");
        assert_eq!(layers[0].annotations.len(), 2);
        assert_eq!(
            layers[0].annotations[0].kind,
            LayerAnnotationKind::Raw("M106 S255".into())
        );
        assert_eq!(
            layers[0].annotations[1].kind,
            LayerAnnotationKind::Comment("annotation witness".into())
        );
    }
}

#[test]
fn mixed_valid_and_invalid_annotation_targets_roll_back_both_transports() {
    for native in [true, false] {
        let mut layers = vec![LayerCollectionIR::default()];
        let before = layers.clone();
        let error = dispatch(
            native,
            &["LayerCollectionIR.annotations".into()],
            99,
            &mut layers,
        )
        .expect_err("invalid second target");
        assert!(
            error
                .to_string()
                .contains("annotation references unknown layer 99"),
            "{error}"
        );
        assert_eq!(
            layers, before,
            "valid first annotation must roll back on native={native}"
        );
    }
}

#[test]
fn malformed_finalization_merge_rolls_back_prior_entity_push_on_both_transports() {
    for native in [true, false] {
        let mut layers = vec![LayerCollectionIR::default()];
        let before = layers.clone();
        let error = dispatch(
            native,
            &["LayerCollectionIR.annotations".into()],
            -1,
            &mut layers,
        )
        .expect_err("out-of-range permutation must fail merge");
        assert!(
            error.to_string().contains("finalization merge failed"),
            "{error}"
        );
        assert_eq!(
            layers, before,
            "first entity push must not leak on native={native}"
        );
    }
}
