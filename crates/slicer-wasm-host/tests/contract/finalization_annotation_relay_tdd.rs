#![allow(missing_docs)]

//! Wayfinder perf-vs-orca ticket 47 — the finalization annotation relay.
//!
//! The finalization WIT surface had no annotation channel, so the macro's
//! WASM drain-back silently discarded `FinalizationOutputBuilder`'s
//! `push_annotation`/`push_fan_speed` stream: `part-cooling`'s `M106`/`M107`
//! fan commands never reached the external leg's emitted G-code, while the
//! native leg's `apply_to` merged them (t47 gate proof: integrated `M106`
//! S255 ×279 + `M107` ×2, external ×0). The relay adds
//! `finalization-output-builder.push-annotation` (package 1.0.0 → 1.1.0) and
//! rides the same drain stream the entity pushes take.

use slicer_wasm_host::host::finalization_types as fm;
use slicer_wasm_host::host::{FinalizationBuilderPush, HostExecutionContextBuilder};

fn push_and_drain(
    ctx: &mut slicer_wasm_host::host::HostExecutionContext,
    builder_rep: u32,
) -> Vec<FinalizationBuilderPush> {
    let drop_handle =
        wasmtime::component::Resource::<fm::FinalizationOutputBuilder>::new_own(builder_rep);
    <slicer_wasm_host::host::HostExecutionContext as fm::HostFinalizationOutputBuilder>::drop(
        ctx,
        drop_handle,
    )
    .expect("builder drop must not trap");
    ctx.drain_finalization_output_builder()
}

/// The relay records comment and raw annotations in emission order and
/// preserves anchors.
#[test]
fn annotation_relay_round_trips_comment_and_raw() {
    let mut ctx = HostExecutionContextBuilder::new("t47-annotation-relay", 0.0, 0.2)
        .build()
        .with_finalization_ir_writes(&["LayerCollectionIR.annotations".into()]);

    let builder_handle = ctx
        .push_finalization_output_builder()
        .expect("push_finalization_output_builder must succeed");
    let builder_rep = builder_handle.rep();
    let push_result = <slicer_wasm_host::host::HostExecutionContext
        as fm::HostFinalizationOutputBuilder>::push_annotation(
        &mut ctx,
        builder_handle,
        fm::AnnotationView {
            layer_index: 3,
            after_entity_index: 7,
            kind: fm::AnnotationKind::Raw("M106 S255".to_string()),
        },
    )
    .expect("wasmtime call must not trap");
    assert!(
        push_result.is_ok(),
        "push_annotation rejected: {push_result:?}"
    );
    assert_eq!(ctx.runtime_writes(), &["LayerCollectionIR.annotations"]);

    let pushes = push_and_drain(&mut ctx, builder_rep);
    assert_eq!(pushes.len(), 1, "exactly one relayed annotation");
    match &pushes[0] {
        FinalizationBuilderPush::Annotation(layer_index, annotation) => {
            assert_eq!(*layer_index, 3, "annotation layer index");
            assert_eq!(annotation.after_entity_index, 7, "annotation anchor");
            assert_eq!(
                annotation.kind,
                slicer_ir::LayerAnnotationKind::Raw("M106 S255".to_string()),
                "raw payload must round-trip verbatim"
            );
        }
        other => panic!("expected Annotation push, got {other:?}"),
    }
}

/// Raw WIT callers receive the same denial before any annotation or audit write.
#[test]
fn annotation_raw_wit_denial_records_neither_output_nor_write_audit() {
    for writes in [
        vec![],
        vec!["LayerCollectionIR".into()],
        vec!["LayerCollectionIR.cooling".into()],
    ] {
        let mut ctx = HostExecutionContextBuilder::new("raw-denial", 0.0, 0.2)
            .build()
            .with_finalization_ir_writes(&writes);
        let handle = ctx.push_finalization_output_builder().expect("builder");
        let rep = handle.rep();
        let error = <slicer_wasm_host::host::HostExecutionContext as fm::HostFinalizationOutputBuilder>::push_annotation(
            &mut ctx, handle, fm::AnnotationView {
                layer_index: 0, after_entity_index: 0, kind: fm::AnnotationKind::Raw("M107".into()),
            },
        ).expect("contract denial is a typed error").expect_err("exact permission required");
        assert!(error.contains("attempted write requested path LayerCollectionIR.annotations"));
        assert!(ctx.runtime_writes().is_empty());
        assert!(push_and_drain(&mut ctx, rep).is_empty());
    }
}

/// A fan-speed zero relays as the raw `M107` text the SDK builder authored
/// (the relay is content-transparent; the SDK side owns rendering).
#[test]
fn annotation_relay_carries_fan_off_as_raw_m107() {
    let mut ctx = HostExecutionContextBuilder::new("t47-fan-off", 0.0, 0.2)
        .build()
        .with_finalization_ir_writes(&["LayerCollectionIR.annotations".into()]);
    let builder_handle = ctx
        .push_finalization_output_builder()
        .expect("push builder");
    let builder_rep = builder_handle.rep();
    let result = <slicer_wasm_host::host::HostExecutionContext
        as fm::HostFinalizationOutputBuilder>::push_annotation(
        &mut ctx,
        builder_handle,
        fm::AnnotationView {
            layer_index: 0,
            after_entity_index: 0,
            kind: fm::AnnotationKind::Raw("M107".to_string()),
        },
    )
    .expect("wasmtime call must not trap");
    assert!(result.is_ok());
    let pushes = push_and_drain(&mut ctx, builder_rep);
    assert!(matches!(
        &pushes[0],
        FinalizationBuilderPush::Annotation(
            0,
            slicer_ir::LayerAnnotation {
                kind: slicer_ir::LayerAnnotationKind::Raw(text),
                ..
            }
        ) if text == "M107"
    ));
}

/// Comment annotations round-trip through the same stream, in the guest's
/// emission order relative to other annotations.
#[test]
fn annotation_relay_preserves_emission_order_across_kinds() {
    let mut ctx = HostExecutionContextBuilder::new("t47-order", 0.0, 0.2)
        .build()
        .with_finalization_ir_writes(&["LayerCollectionIR.annotations".into()]);
    // Each annotation gets its own builder resource; dropping all three moves
    // every annotation onto the context's stream in emission order.
    let mut builder_reps: Vec<u32> = Vec::new();
    for (layer, anchor, kind) in [
        (1u32, 0u32, fm::AnnotationKind::Raw("M107".to_string())),
        (
            2,
            4,
            fm::AnnotationKind::Comment("mid-layer note".to_string()),
        ),
        (2, 9, fm::AnnotationKind::Raw("M106 S255".to_string())),
    ] {
        let builder_handle = ctx
            .push_finalization_output_builder()
            .expect("push builder");
        builder_reps.push(builder_handle.rep());
        let result = <slicer_wasm_host::host::HostExecutionContext
            as fm::HostFinalizationOutputBuilder>::push_annotation(
            &mut ctx, builder_handle,
            fm::AnnotationView {
                layer_index: layer,
                after_entity_index: anchor,
                kind,
            },
        )
        .expect("wasmtime call must not trap");
        assert!(result.is_ok());
    }
    let mut pushes = Vec::new();
    for rep in builder_reps {
        pushes.extend(push_and_drain(&mut ctx, rep));
    }
    assert_eq!(pushes.len(), 3, "all three annotations relayed");
    // Order + kind are the tested contract; anchors ride inside.
    let kinds: Vec<bool> = pushes
        .iter()
        .map(|p| {
            matches!(
                p,
                FinalizationBuilderPush::Annotation(_, a)
                    if matches!(a.kind, slicer_ir::LayerAnnotationKind::Comment(_))
            )
        })
        .collect();
    assert_eq!(
        kinds,
        vec![false, true, false],
        "emission order with the comment in the middle"
    );
}
