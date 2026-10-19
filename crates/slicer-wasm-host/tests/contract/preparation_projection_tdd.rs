//! Gate A: independent literal scene oracles over the real production adapters.

use super::preparation_projection_driver::{self as driver, Leg, Print, VIEW};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_schema::preparation::PreparationTransportError as Transport;
use slicer_sdk::{
    error::ModuleError,
    preparation::{
        PreparationProjectionError as Error, PreparationProjectionGeometry as Geometry,
        PreparationProjectionOutput, PreparationProjectionPrimitive as Primitive,
        PreparationProjectionScene as Scene, PreparedPlanOutput, PrintPreparationInput,
    },
};
use slicer_wasm_host::preparation::{
    FrozenPreparationBinding, PreparationFailure, PreparationOwner, PreparationProjectionRequest,
    PreparedOwnerStore,
};
use std::{cell::RefCell, sync::Arc};

fn points(values: &[(i64, i64)]) -> Vec<Point2> {
    values.iter().map(|&(x, y)| Point2 { x, y }).collect()
}

fn primitive(id: &str, layer: u32, class: &str, label: &str, geometry: Geometry) -> Primitive {
    // exhaustive: gate A pins all public primitive identity and geometry fields.
    Primitive {
        id: id.into(),
        global_layer_index: layer,
        class: class.into(),
        label: Some(label.into()),
        geometry,
    }
}

/// Authored from AC-1, never obtained from fixture scene generation or host validation.
fn expected_scene(two_layers: bool) -> Scene {
    let mut scene = Scene {
        scene_id: "final".into(),
        view: "probe_xy".into(),
        classes: vec!["anchors".into(), "domains".into(), "paths".into()],
        primitives: vec![
            primitive(
                "anchor_a",
                0,
                "anchors",
                "anchor",
                Geometry::Point(Point2 { x: 10000, y: 20000 }),
            ),
            primitive(
                "path_a",
                0,
                "paths",
                "route",
                Geometry::Polyline(points(&[(0, 0), (20000, 0)])),
            ),
            primitive(
                "domain_a",
                0,
                "domains",
                "area",
                Geometry::Polygon {
                    polygon: ExPolygon {
                        contour: Polygon {
                            points: points(&[(0, 0), (40000, 0), (40000, 40000), (0, 40000)]),
                        },
                        holes: vec![Polygon {
                            points: points(&[
                                (10000, 10000),
                                (10000, 30000),
                                (30000, 30000),
                                (30000, 10000),
                            ]),
                        }],
                    },
                    filled: true,
                },
            ),
            primitive(
                "outline_a",
                0,
                "domains",
                "outline",
                Geometry::Polygon {
                    polygon: ExPolygon {
                        contour: Polygon {
                            points: points(&[
                                (60000, 0),
                                (80000, 0),
                                (80000, 20000),
                                (60000, 20000),
                            ]),
                        },
                        holes: vec![],
                    },
                    filled: false,
                },
            ),
        ],
    };
    if two_layers {
        scene.classes.push("remote".into());
        scene.primitives.push(primitive(
            "remote_a",
            1,
            "remote",
            "remote",
            Geometry::Point(Point2 {
                x: 2000000,
                y: 1500000,
            }),
        ));
    }
    scene
}

fn expected_bytes(two_layers: bool) -> u64 {
    // Independent UTF-8 inventory, including each stored class occurrence.
    let strings = [
        "final",
        "probe_xy",
        "anchors",
        "domains",
        "paths",
        "anchor_a",
        "anchors",
        "anchor",
        "path_a",
        "paths",
        "route",
        "domain_a",
        "domains",
        "area",
        "outline_a",
        "domains",
        "outline",
    ];
    let base = strings.iter().map(|s| s.len() as u64).sum::<u64>()
        + (1 + 2 + 4 + 4 + 4) * 16 // point, open line, contour, hole, outline
        + 4 * 4 + 4 + 2; // layer indices, geometry tags, polygon filled flags
    base + if two_layers {
        6 + 8 + 6 + 6 + 16 + 4 + 1
    } else {
        0
    }
}

fn assert_metadata_redacted(owner: &PreparationOwner) {
    let metadata = owner.metadata();
    assert_eq!(metadata.pieces(), &[("probe".to_owned(), 4)]);
    // Complete Debug shape excludes arbitrary additional payload/store fields,
    // not merely a handpicked blacklist of leak spellings.
    assert_eq!(format!("{metadata:?}"),
        "PreparedPlanMetadata { module_id: \"test.preparation-visual\", phase: \"ready\", ready: true, pieces: [(\"probe\", 4)], retained_payload_bytes: 4, retained_name_bytes: 5, staged_payload_bytes: 0, staged_name_bytes: 0, put_calls: 1, read_calls: 0, requested_read_bytes: 0, returned_read_bytes: 0, prepare_invocations: 1 }");
}

fn assert_unpublished(store: &PreparedOwnerStore, owner: &PreparationOwner) {
    assert!(matches!(
        store.current(owner),
        Err(PreparationFailure::Transport(Transport::NotReady))
    ));
    let metadata = owner.metadata();
    assert!(!metadata.ready());
    assert!(metadata.pieces().is_empty());
    assert_eq!(metadata.retained_payload_bytes(), 0);
    assert_eq!(metadata.retained_name_bytes(), 0);
    assert_eq!(metadata.staged_payload_bytes(), 0);
    assert_eq!(metadata.staged_name_bytes(), 0);
    assert_eq!(owner.diagnostic_accounting().staged_bytes, 0);
    assert_eq!(owner.diagnostic_accounting().retained_bytes, 0);
}

fn assert_rejected(error: PreparationFailure, expected: Error) {
    match error {
        PreparationFailure::PublicationRejected { detail } => {
            assert!(
                detail.starts_with(&format!(
                    "preparation projection: {expected:?}: owner=test.preparation-visual view="
                )),
                "{detail}"
            );
        }
        other => panic!("wrong projection failure, expected {expected:?}: {other:?}"),
    }
}

#[test]
fn typed_projection_native_and_wasm() {
    for leg in [Leg::Native, Leg::Component] {
        for two_layers in [false, true] {
            let print = Print::new(leg);
            let mut validated = false;
            print
                .prepare_validated("valid", "PRB1", two_layers, |staged| {
                    validated = true;
                    assert_eq!(staged.pieces(), &[("probe".to_owned(), 4)]);
                    assert_eq!(staged.owner().metadata().staged_payload_bytes(), 4);
                    assert_eq!(
                        staged.owner().diagnostic_accounting().staged_bytes,
                        expected_bytes(two_layers)
                    );
                    assert_eq!(staged.owner().diagnostic_accounting().retained_bytes, 0);
                    assert!(matches!(
                        print.store.current(&print.owner),
                        Err(PreparationFailure::Transport(Transport::NotReady))
                    ));
                    Ok(())
                })
                .unwrap_or_else(|e| panic!("{leg:?}: {e:?}"));
            assert!(validated, "publication validation must run before Ready");
            let plan = print.store.current(&print.owner).expect("atomic Ready");
            let scene = plan
                .projection(VIEW)
                .expect("authorized projection")
                .expect("requested scene");
            assert_eq!(scene.scene(), &expected_scene(two_layers), "{leg:?}");
            assert_eq!(scene.owner_module_id(), "test.preparation-visual");
            assert_eq!(scene.view(), "probe_xy");
            assert_eq!(scene.layer_z(0), Some(0.2));
            assert_eq!(scene.layer_z(1), if two_layers { Some(0.4) } else { None });
            assert_eq!(scene.diagnostic_bytes(), expected_bytes(two_layers));
            assert_eq!(
                plan.open("probe")
                    .expect("private piece lookup")
                    .expect("probe")
                    .len(),
                4
            );
            assert_metadata_redacted(&print.owner);
            let accounting = print.owner.diagnostic_accounting();
            assert_eq!(accounting.staged_bytes, 0);
            assert_eq!(accounting.retained_bytes, expected_bytes(two_layers));
            assert_eq!(accounting.published_scenes, 1);
            assert_eq!(plan.projection("not_requested_xy"), Ok(None));
        }
    }
}

#[test]
fn projection_owner_and_lifetime() {
    // Same frozen module/view/scene identities across two separate print authorities.
    let a = Print::new(Leg::Native);
    let b = Print::new(Leg::Component);
    a.prepare("valid", "PRB1", false).expect("first owner");
    b.prepare("valid", "QXY2", true).expect("second owner");
    let pa = a.store.current(&a.owner).expect("A Ready");
    let pb = b.store.current(&b.owner).expect("B Ready");
    let sa = pa.projection(VIEW).unwrap().unwrap();
    let sb = pb.projection(VIEW).unwrap().unwrap();
    assert_eq!(sa.scene(), &expected_scene(false));
    assert_eq!(sb.scene(), &expected_scene(true));
    assert!(!Arc::ptr_eq(&sa, &sb), "no cross-print snapshot cache");
    assert!(Arc::ptr_eq(&sa, &pa.projection(VIEW).unwrap().unwrap()));
    assert!(Arc::ptr_eq(&sb, &pb.projection(VIEW).unwrap().unwrap()));
    assert_metadata_redacted(&a.owner);
    assert_metadata_redacted(&b.owner);
    // Store lookup has its established producer InvalidOwner boundary; projection
    // capability AccessDenied is checked below with a retained real native sink.
    assert!(matches!(
        a.store.current(&b.owner),
        Err(PreparationFailure::InvalidOwner)
    ));
    assert!(matches!(
        b.store.current(&a.owner),
        Err(PreparationFailure::InvalidOwner)
    ));
    for print in [&a, &b] {
        print.with_active_call(|| {
            assert!(matches!(
                print.store.dispose_owner(&print.owner),
                Err(PreparationFailure::Busy)
            ));
            assert!(matches!(
                print.store.dispose(),
                Err(PreparationFailure::Busy)
            ));
            assert_eq!(print.owner.metadata().retained_payload_bytes(), 4);
            assert!(print.owner.diagnostic_accounting().retained_bytes > 0);
        });
        print
            .store
            .dispose_owner(&print.owner)
            .expect("release after active call drains");
        assert_eq!(print.owner.metadata().retained_payload_bytes(), 0);
        assert_eq!(print.owner.metadata().retained_name_bytes(), 0);
        assert_eq!(print.owner.diagnostic_accounting().retained_bytes, 0);
        assert_eq!(print.owner.diagnostic_accounting().staged_bytes, 0);
    }
    assert_eq!(pa.projection(VIEW), Err(Error::AccessDenied));
    assert_eq!(pb.projection(VIEW), Err(Error::AccessDenied));
    assert_eq!(
        sa.scene(),
        &expected_scene(false),
        "immutable public scene survives private disposal"
    );
    assert_eq!(sb.scene(), &expected_scene(true));

    // Native-adapter level: a sink retained from an ended invocation is not
    // usable by a foreign owner, and cannot mutate its otherwise valid scene.
    let binding = driver::native_case(retain_sink, vec![VIEW.into()]);
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(&binding).unwrap();
    binding
        .prepare_with_diagnostics(
            &owner,
            driver::input(&owner, "valid", "PRB1", false),
            driver::request(),
            |_| Ok(()),
        )
        .unwrap();
    let foreign = Print::new(Leg::Native);
    foreign
        .prepare_validated("valid", "PRB1", false, |_| {
            RETAINED_SINK.with(|slot| {
                assert_eq!(
                    slot.borrow()
                        .as_ref()
                        .unwrap()
                        .publish(&expected_scene(false)),
                    Err(Error::AccessDenied)
                )
            });
            Ok(())
        })
        .unwrap();
    assert_eq!(
        foreign.store.projection_failure(&foreign.owner).unwrap(),
        None
    );
    assert_eq!(
        foreign
            .store
            .current(&foreign.owner)
            .unwrap()
            .projection(VIEW)
            .unwrap()
            .unwrap()
            .scene(),
        &expected_scene(false)
    );
    RETAINED_SINK.with(|slot| slot.borrow_mut().take());
    store.dispose().unwrap();

    assert_live_foreign_sink_denied();
}

thread_local! {
    static RETAINED_SINK: RefCell<Option<PreparationProjectionOutput>> = const { RefCell::new(None) };
    static FOREIGN_WRITE_RESULT: RefCell<Option<Result<(), Error>>> = const { RefCell::new(None) };
}

/// Native production-adapter level: both owners are in active preparation,
/// in different prints, with identical module/view/scene identities. Unlike the
/// stale-capability control above, owner A's sink must remain fully usable by A.
fn assert_live_foreign_sink_denied() {
    let binding = driver::native_case(retain_sink, vec![VIEW.into()]);
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(&binding).unwrap();
    let foreign_binding = driver::native_case(attempt_live_foreign_sink, vec![VIEW.into()]);
    let foreign_store = PreparedOwnerStore::new();
    let foreign_owner = foreign_store.create_owner(&foreign_binding).unwrap();
    FOREIGN_WRITE_RESULT.with(|slot| *slot.borrow_mut() = None);
    binding
        .prepare_with_diagnostics(
            &owner,
            driver::input(&owner, "valid", "PRB1", false),
            driver::request(),
            |_| {
                // A has valid typed geometry plus four private bytes, but its
                // call identity is still live until this validator completes.
                assert_eq!(owner.metadata().phase(), "staging");
                assert_eq!(owner.metadata().staged_payload_bytes(), 4);
                RETAINED_SINK.with(|slot| {
                    assert_eq!(slot.borrow().as_ref().unwrap().requested_views(), Ok(vec![VIEW.into()]));
                });
                assert_eq!(
                    owner.diagnostic_accounting().staged_bytes,
                    expected_bytes(false)
                );
                let error = foreign_binding
                    .prepare_with_diagnostics(
                        &foreign_owner,
                        driver::input(&foreign_owner, "valid", "QXY2", false),
                        driver::request(),
                        |_| panic!("foreign caller's original error must prevent publication"),
                    )
                    .expect_err("foreign caller deliberately aborts after the access attempt");
                match error {
                    PreparationFailure::Module(error) => {
                        assert_eq!(error.code, 57591);
                        assert_eq!(error.message, "foreign sink caller aborted");
                        assert!(!error.fatal);
                    }
                    other => panic!("lost foreign caller failure: {other:?}"),
                }
                assert_unpublished(&foreign_store, &foreign_owner);
                let actual = FOREIGN_WRITE_RESULT.with(|slot| {
                    slot.borrow_mut()
                        .take()
                        .expect("real foreign sink publication executed")
                });
                assert_eq!(
                    actual,
                    Err(Error::AccessDenied),
                    "live owner B must not publish through owner A's still-active sink; A rejection sidecar: {:?}",
                    store.projection_failure(&owner).unwrap()
                );
                assert_eq!(store.projection_failure(&owner).unwrap(), None);
                assert_eq!(owner.metadata().staged_payload_bytes(), 4);
                assert_eq!(
                    owner.diagnostic_accounting().staged_bytes,
                    expected_bytes(false)
                );
                assert_eq!(owner.diagnostic_accounting().published_scenes, 1);
                Ok(())
            },
        )
        .expect("foreign access cannot poison A or prevent its valid publication");
    let scene = store
        .current(&owner)
        .unwrap()
        .projection(VIEW)
        .unwrap()
        .unwrap();
    assert_eq!(scene.scene(), &expected_scene(false));
    assert_metadata_redacted(&owner);
    RETAINED_SINK.with(|slot| slot.borrow_mut().take());
    store.dispose().unwrap();
    foreign_store.dispose().unwrap();
}

fn attempt_live_foreign_sink(
    _: &PrintPreparationInput,
    output: &mut PreparedPlanOutput,
) -> Result<(), ModuleError> {
    output.put("probe", b"QXY2")?;
    let own_sink = output.projection().unwrap().unwrap();
    assert_eq!(own_sink.publish(&expected_scene(false)), Ok(()));
    let actual = RETAINED_SINK.with(|slot| {
        slot.borrow()
            .as_ref()
            .expect("A's live sink")
            .publish(&expected_scene(false))
    });
    FOREIGN_WRITE_RESULT.with(|slot| *slot.borrow_mut() = Some(actual));
    Err(ModuleError::non_fatal(57591, "foreign sink caller aborted"))
}

fn retain_sink(
    _: &PrintPreparationInput,
    output: &mut PreparedPlanOutput,
) -> Result<(), ModuleError> {
    output.put("probe", b"PRB1")?;
    let sink = output.projection().unwrap().unwrap();
    assert_eq!(sink.requested_views(), Ok(vec!["probe_xy".into()]));
    assert_eq!(sink.publish(&expected_scene(false)), Ok(()));
    RETAINED_SINK.with(|slot| *slot.borrow_mut() = Some(sink));
    Ok(())
}

/// Supplemental native-adapter validator controls absent from the fixed guest modes.
fn invalid_identity(
    input: &PrintPreparationInput,
    output: &mut PreparedPlanOutput,
) -> Result<(), ModuleError> {
    let targets = input.selected_targets(0, 0, 1).unwrap();
    assert_eq!(targets.len(), 1);
    let mode = targets[0]
        .config
        .get_string("preparation_visual_mode")
        .unwrap();
    output.put("probe", b"PRB1")?;
    let sink = output.projection().unwrap().unwrap();
    let mut scene = expected_scene(false);
    let expected = match mode {
        "empty_scene_id" => {
            scene.scene_id.clear();
            Error::InvalidReference
        }
        "empty_view" => {
            scene.view.clear();
            Error::InvalidReference
        }
        "empty_class_id" => {
            scene.classes.push(String::new());
            Error::InvalidReference
        }
        "duplicate_class_id" => {
            scene.classes.push("anchors".into());
            Error::InvalidReference
        }
        "empty_primitive_id" => {
            scene.primitives[0].id.clear();
            Error::InvalidReference
        }
        "duplicate_primitive_id" => {
            scene.primitives[1].id = "anchor_a".into();
            Error::InvalidReference
        }
        "duplicate_scene_id" => {
            assert_eq!(sink.publish(&scene), Ok(()));
            scene.view = "second_xy".into();
            Error::InvalidReference
        }
        "unrequested_view" => {
            scene.view = "second_xy".into();
            Error::NotRequested
        }
        _ => panic!("unknown supplemental native case {mode}"),
    };
    assert_eq!(sink.publish(&scene), Err(expected), "{mode}");
    assert_eq!(
        sink.publish(&expected_scene(false)),
        Err(Error::PoisonedOutput),
        "{mode}"
    );
    Ok(()) // Deliberately caught errors still forbid publication.
}

#[test]
fn projection_rejections_are_atomic() {
    for leg in [Leg::Native, Leg::Component] {
        let foreign = Print::new(leg);
        foreign
            .prepare("valid", "QXY2", true)
            .expect("foreign negative-control owner");
        let foreign_plan = foreign.store.current(&foreign.owner).unwrap();
        let foreign_scene = foreign_plan.projection(VIEW).unwrap().unwrap();
        for (mode, expected) in [
            ("duplicate", Error::DuplicateScene),
            ("missing", Error::MissingScene),
            ("undeclared_view", Error::UndeclaredView),
            ("undeclared_class", Error::InvalidReference),
            ("unknown_layer", Error::InvalidReference),
            ("invalid_line", Error::InvalidShape),
            ("invalid_ring", Error::InvalidShape),
            ("invalid_hole", Error::InvalidShape),
            ("invalid_coordinate", Error::InvalidCoordinate),
            ("caught_error", Error::InvalidShape),
        ] {
            let print = Print::new(leg);
            let error = print
                .prepare_validated(mode, "PRB1", false, |_| {
                    panic!("{mode}: invalid output must precede publication hook")
                })
                .expect_err("caught invalid projection cannot publish");
            assert_rejected(error, expected);
            assert_eq!(
                print.store.projection_failure(&print.owner).unwrap(),
                Some((
                    if mode == "undeclared_view" {
                        "undeclared_xy"
                    } else {
                        VIEW
                    }
                    .into(),
                    expected
                )),
                "{leg:?} {mode}"
            );
            assert_unpublished(&print.store, &print.owner);
            assert_eq!(print.owner.metadata().prepare_invocations(), 1);
            assert_eq!(
                print.owner.metadata().put_calls(),
                1,
                "valid private bytes were actually staged"
            );
            if matches!(leg, Leg::Component) {
                let messages = FrozenPreparationBinding::take_wasm_log_messages();
                assert!(messages
                    .iter()
                    .any(|(_, text)| text == "PREPARATION-VISUAL-PREPARE"));
                if mode != "missing" {
                    assert!(
                        messages.iter().any(|(_, text)| text
                            == &format!("PREPARATION-VISUAL-PUBLISH-ERROR:{expected:?}")),
                        "actual resource-call rejection missing: {messages:?}"
                    );
                }
                if mode == "caught_error" {
                    assert!(
                        messages
                            .iter()
                            .any(|(_, text)| text
                                == "PREPARATION-VISUAL-PUBLISH-ERROR:PoisonedOutput")
                    );
                }
            }
            assert!(Arc::ptr_eq(
                &foreign_scene,
                &foreign_plan.projection(VIEW).unwrap().unwrap()
            ));
            assert_eq!(foreign_scene.scene(), &expected_scene(true));
            assert_metadata_redacted(&foreign.owner);
        }
        for mode in ["module_error", "trap"] {
            let print = Print::new(leg);
            let error = print
                .prepare_validated(mode, "PRB1", false, |_| {
                    panic!("original failure must precede validation")
                })
                .expect_err("producer failure must win over caught diagnostic poison");
            match (mode, error) {
                ("module_error", PreparationFailure::Module(e)) => {
                    assert_eq!(e.code, 57551);
                    assert_eq!(e.message, "visual fixture module error");
                    assert!(!e.fatal);
                }
                ("trap", PreparationFailure::Trap { detail }) => {
                    assert!(!detail.is_empty());
                    if matches!(leg, Leg::Native) {
                        assert_eq!(detail, "visual fixture preparation trap");
                    } else {
                        assert!(
                            detail.contains("error while executing at wasm backtrace:"),
                            "{detail}"
                        );
                    }
                }
                (_, other) => panic!("lost original {leg:?} {mode}: {other:?}"),
            }
            assert_eq!(
                print.store.projection_failure(&print.owner).unwrap(),
                Some((VIEW.into(), Error::InvalidShape))
            );
            assert_unpublished(&print.store, &print.owner);
        }
        let print = Print::new(leg);
        let mut cancelled = false;
        let error = print
            .prepare_validated("valid", "PRB1", false, |staged| {
                cancelled = true;
                assert_eq!(staged.pieces(), &[("probe".into(), 4)]);
                assert_eq!(
                    staged.owner().diagnostic_accounting().staged_bytes,
                    expected_bytes(false)
                );
                Err(PreparationFailure::PublicationRejected {
                    detail: "cancelled before Ready".into(),
                })
            })
            .expect_err("cancellation rolls back both attachments");
        assert!(cancelled);
        assert!(
            matches!(error, PreparationFailure::PublicationRejected { detail } if detail == "cancelled before Ready")
        );
        assert_unpublished(&print.store, &print.owner);
    }

    // Native-adapter level (not component coverage): exact identity and declared-
    // but-unrequested controls absent from preparation-visual-guest's fixed modes.
    for mode in [
        "empty_scene_id",
        "empty_view",
        "empty_class_id",
        "duplicate_class_id",
        "empty_primitive_id",
        "duplicate_primitive_id",
        "duplicate_scene_id",
        "unrequested_view",
    ] {
        let binding = driver::native_case(invalid_identity, vec![VIEW.into(), "second_xy".into()]);
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).unwrap();
        let views = if mode == "duplicate_scene_id" {
            vec![VIEW.into(), "second_xy".into()]
        } else {
            vec![VIEW.into()]
        };
        let error = binding
            .prepare_with_diagnostics(
                &owner,
                driver::input(&owner, mode, "PRB1", false),
                PreparationProjectionRequest::new(views).unwrap(),
                |_| panic!("caught identity failure cannot publish"),
            )
            .expect_err("identity validation is atomic");
        let expected = if mode == "unrequested_view" {
            Error::NotRequested
        } else {
            Error::InvalidReference
        };
        assert_rejected(error, expected);
        assert_eq!(
            store.projection_failure(&owner).unwrap().unwrap().1,
            expected
        );
        assert_unpublished(&store, &owner);
    }

    assert_live_foreign_sink_denied();

    // ArithmeticOverflow remains blocked at this integration-test boundary:
    // PreparationOwner::diagnostic_accounting returns a Copy, not a mutable
    // counter authority. The actual counter test is
    // preparation_projection_tests::diagnostic_counter_overflow_is_typed_atomic_and_does_not_wrap
    // in crates/slicer-wasm-host/src/preparation_store.rs (library-unit level).
    // It must run separately; it is not claimed as real-driver overflow coverage.
}
