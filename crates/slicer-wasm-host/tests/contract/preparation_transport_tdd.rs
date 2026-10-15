//! Contract tests for preparation transport.
#![cfg(feature = "preparation-test-fixtures")]
//! The shared driver lives in `preparation_transport_driver` and is registered by Step 9.

use super::preparation_transport_driver as driver;
use slicer_scheduler::preparation::parse_preparation_declaration;
use slicer_schema::preparation::PreparationDeclarationError as D;
use slicer_wasm_host::preparation::{freeze_wasm, PreparationFailure, PreparedHostState};
use std::sync::atomic::{AtomicUsize, Ordering};

// Only the rejection test uses this pointer, so concurrent positive tests cannot
// perturb its independently pinned zero-invocation witness.
static REJECTED_PREPARATION_CALLS: AtomicUsize = AtomicUsize::new(0);

fn rejected_prepare(
    _: &slicer_sdk::preparation::PrintPreparationInput,
    _: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    REJECTED_PREPARATION_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn paired_prepare(
    _: &slicer_sdk::preparation::PrintPreparationInput,
    _: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    Ok(())
}

const MANIFEST: &str = "[preparation]\ninterface = 'slicer:layer-preparation/prepare@1.0.0'\ninput_reads = ['fixture', 'exact.input']\nviews = ['輪郭-plan', 'Case Sensitive']\n";

fn transport_native_binding() -> slicer_wasm_host::preparation::FrozenPreparationBinding {
    use preparation_transport_path::TransportPathModule;
    use slicer_wasm_host::preparation::NativePreparationArtifact;
    driver::native_binding(
        "fresh-native",
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
        NativePreparationArtifact::Prepared {
            entry: TransportPathModule::__slicer_prepared_native_entry(),
            schema: TransportPathModule::__SLICER_PREPARATION_SCHEMA,
        },
    )
    .expect("freeze native fixture")
}

fn transport_wasm_binding() -> slicer_wasm_host::preparation::FrozenPreparationBinding {
    driver::wasm_binding(
        "fresh-wasm",
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
    )
    .expect("freeze WASM fixture")
}

fn native_ordinary_output(
    binding: &slicer_wasm_host::preparation::FrozenPreparationBinding,
    owner: &slicer_wasm_host::preparation::PreparationOwner,
) -> Result<Vec<String>, PreparationFailure> {
    use slicer_sdk::native::{NativeLayerRequest, NativeStageEntry};
    use slicer_sdk::postpass_types::{GcodeCommand, GcodeOutputCommand};
    binding.with_native_plan(owner, |stage| {
        let NativeStageEntry::Layer(entry) = stage else {
            panic!("expected frozen native layer entry: {stage:?}")
        };
        // exhaustive: pin the ordinary path-stage request, including unused inputs.
        let request = NativeLayerRequest {
            layer_index: 0,
            regions: Vec::new(),
            perimeter_regions: Some(Vec::new()),
            paint: None,
            prior_infill: None,
            config: slicer_ir::ConfigView::from_map(Default::default()),
            stage_export: driver::path_stage_id(),
            ordered_entities: Vec::new(),
        };
        let response = entry(&request)?;
        let path = response
            .path_optimization
            .expect("native path-stage output");
        Ok(path
            .output
            .commands()
            .iter()
            .map(|command| match command {
                GcodeOutputCommand::Command(GcodeCommand::Comment { text }) => text.clone(),
                other => panic!("unexpected native ordinary command: {other:?}"),
            })
            .collect())
    })
}

type WasmOrdinaryOutput = (
    Result<
        (),
        slicer_wasm_host::host::layer_perimeters::slicer::common::module_errors::ModuleError,
    >,
    Vec<String>,
);

fn wasm_ordinary_output(
    binding: &slicer_wasm_host::preparation::FrozenPreparationBinding,
    owner: &slicer_wasm_host::preparation::PreparationOwner,
) -> Result<WasmOrdinaryOutput, PreparationFailure> {
    use slicer_wasm_host::host::{self, GcodeCommandCollected};
    // Resources belong to the ordinary context's shared table, not a test table.
    let mut context = driver::ordinary_context("fresh-wasm");
    let config = context
        .push_config_view(host::ConfigViewData {
            fields: Default::default(),
        })
        .expect("config");
    let output = context.push_gcode_output_builder().expect("output");
    let collection = context
        .push_layer_collection_builder(Vec::new())
        .expect("collection");
    binding.with_wasm_plan(owner, context, |component, store| {
        let linker = PreparedHostState::linker(store.engine(), driver::path_stage_id())?;
        let module = host::layer_path_optimization::PathOptimizationModule::instantiate(
            &mut *store,
            component,
            &linker,
        )?;
        let result = module
            .slicer_layer_path_optimization_path_optimization()
            .call_run(&mut *store, 0, &[], output, collection, config)?;
        let comments = store
            .data()
            .gcode_output()
            .commands
            .iter()
            .map(|command| match command {
                GcodeCommandCollected::Comment(text) => text.clone(),
                other => panic!("unexpected ordinary command: {other:?}"),
            })
            .collect();
        Ok((result, comments))
    })
}

#[test]
fn fresh_native_wasm_literals() {
    for binding in [transport_native_binding(), transport_wasm_binding()] {
        let (store, owner) = driver::owner_store(&binding).expect("owner");
        // prepare consumes its input and drops its call-local values/store before returning.
        binding
            .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
            .expect("prepare");
        {
            let plan = store.current(&owner).expect("published");
            let empty = driver::piece(&plan, "π");
            assert_eq!(empty.len(), 0);
        } // No preparation-time plan or piece handle reaches the ordinary call.
        if binding.artifact_kind() == "native" {
            assert_eq!(
                native_ordinary_output(&binding, &owner).expect("native ordinary"),
                ["3,5", "11,17"]
            );
        } else {
            let (result, comments) = wasm_ordinary_output(&binding, &owner).expect("WASM ordinary");
            result.expect("guest ordinary success");
            assert_eq!(comments, ["3,5", "11,17"]);
        }
        assert_eq!(owner.metadata().read_calls(), 2);
        assert_eq!(owner.metadata().requested_read_bytes(), 4);
        assert_eq!(owner.metadata().returned_read_bytes(), 4);
    }
}

#[test]
fn owner_print_concurrency_disposal() {
    use slicer_schema::preparation::PreparationTransportError as T;
    use slicer_sdk::preparation::{PreparationError, PreparedPlanView};
    use slicer_wasm_host::preparation::{
        PreparedOwnerStore, WitPlanAccess, WitPreparedPiece, WitPreparedPlanView,
    };
    use std::sync::Barrier;
    use wasmtime::component::Resource;

    // Same module id, but independent frozen artifacts and print authorities.
    let native = driver::native_binding(
        "shared-module",
        Some(driver::declaration(vec![], vec!["numbers".into()])),
        driver::native_artifact(paired_prepare, driver::native_schema()),
    )
    .expect("native binding");
    let wasm = driver::wasm_binding(
        "shared-module",
        Some(driver::declaration(vec![], vec!["numbers".into()])),
    )
    .expect("WASM binding");
    let first = PreparedOwnerStore::new();
    let second = PreparedOwnerStore::new();
    let a = first.create_owner(&native).expect("first native owner");
    let b = first.create_owner(&wasm).expect("first WASM owner");
    let c = second.create_owner(&native).expect("second native owner");
    let d = second.create_owner(&wasm).expect("second WASM owner");
    // Production transactions, not a replacement transport; the guest codec
    // accepts these independently pinned pairs without changing the fixture.
    for (store, owner, bytes) in [
        (&first, &a, [3, 5, 11, 17]),
        (&first, &b, [19, 23, 29, 31]),
        (&second, &c, [37, 41, 43, 47]),
        (&second, &d, [53, 59, 61, 67]),
    ] {
        let mut transaction = store.begin(owner).expect("begin real staging");
        transaction.put("numbers", &bytes).expect("put literal");
        transaction.finish(Ok(()), |_| Ok(())).expect("publish");
    }
    assert!(matches!(
        first.current(&c),
        Err(PreparationFailure::InvalidOwner)
    ));
    assert!(matches!(
        second.current(&a),
        Err(PreparationFailure::InvalidOwner)
    ));

    let barrier = Barrier::new(4);
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for (owner, expected) in [(&a, [5, 11]), (&c, [41, 43])] {
            let native = &native;
            let barrier = &barrier;
            threads.push(scope.spawn(move || {
                native
                    .with_native_plan(owner, |_| {
                        let piece = PreparedPlanView::current()?.require("numbers")?;
                        barrier.wait(); // All four fresh production scopes are active.
                        assert_eq!(piece.read(1, 2)?, expected);
                        assert_eq!(piece.read(1, 0)?, Vec::<u8>::new());
                        assert_eq!(piece.read(1, 2)?, expected, "no native cursor");
                        barrier.wait();
                        Ok(())
                    })
                    .expect("concurrent native scope");
            }));
        }
        for (owner, expected, comments_expected) in [
            (&b, [23, 29], ["19,23", "29,31"]),
            (&d, [59, 61], ["53,59", "61,67"]),
        ] {
            let wasm = &wasm;
            let barrier = &barrier;
            threads.push(scope.spawn(move || {
                wasm.with_wasm_plan(
                    owner,
                    driver::ordinary_context("shared-module"),
                    |_, store| {
                        let state = store.data_mut();
                        let plan = WitPlanAccess::current(state)?.expect("published WIT plan");
                        let piece = WitPreparedPlanView::open(state, plan, "numbers".into())?
                            .expect("authorized")
                            .expect("numbers");
                        barrier.wait();
                        for maximum in [2, 0, 2] {
                            assert_eq!(
                                WitPreparedPiece::read(
                                    state,
                                    Resource::new_borrow(piece.rep()),
                                    1,
                                    maximum
                                )?,
                                Ok(if maximum == 0 {
                                    vec![]
                                } else {
                                    expected.to_vec()
                                }),
                                "no WIT resource cursor"
                            );
                        }
                        // Execute the actual guest export before releasing
                        // the other concurrent native/WASM read scopes.
                        let (result, comments) =
                            wasm_ordinary_output(wasm, owner).expect("concurrent fresh guest call");
                        result.expect("concurrent guest reads");
                        assert_eq!(comments, comments_expected);
                        barrier.wait();
                        WitPreparedPiece::drop(state, piece)?;
                        Ok(())
                    },
                )
                .expect("concurrent fresh WASM store");
            }));
        }
        for thread in threads {
            thread.join().expect("concurrent reader");
        }
    });
    // Also execute the actual component's ordinary exported call with fresh stores.
    for (owner, expected) in [(&b, ["19,23", "29,31"]), (&d, ["53,59", "61,67"])] {
        for _ in 0..2 {
            let (result, comments) = wasm_ordinary_output(&wasm, owner).expect("fresh guest call");
            result.expect("guest reads owner-local literal");
            assert_eq!(comments, expected);
        }
    }

    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
    native
        .with_native_plan(&a, |_| {
            let outer = PreparedPlanView::current()?.require("numbers")?;
            assert_eq!(outer.read(1, 2)?, [5, 11]);
            let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = native.with_native_plan(
                    &c,
                    |_| -> Result<(), slicer_sdk::error::ModuleError> {
                        assert_eq!(
                            PreparedPlanView::current()?
                                .require("numbers")?
                                .read(1, 2)?,
                            [41, 43]
                        );
                        panic!("AC-6 intentional native scope unwind");
                    },
                );
            }));
            assert!(unwind.is_err());
            assert_eq!(
                PreparedPlanView::current()?
                    .require("numbers")?
                    .read(1, 2)?,
                [5, 11],
                "outer TLS restored"
            );
            assert_eq!(outer.read(1, 2)?, [5, 11]);
            assert!(matches!(
                first.dispose_owner(&a),
                Err(PreparationFailure::Busy)
            ));
            Ok(())
        })
        .expect("outer native scope");
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );

    let old_a = driver::piece(&first.current(&a).expect("A ready"), "numbers");
    let old_b = driver::piece(&first.current(&b).expect("B ready"), "numbers");
    let old_c = driver::piece(&second.current(&c).expect("C ready"), "numbers");
    let old_d = driver::piece(&second.current(&d).expect("D ready"), "numbers");
    first
        .dispose_owner(&a)
        .expect("idle A disposal after unwind");
    assert_eq!(old_a.read(1, 2), Err(T::AccessDenied));
    assert_eq!(old_b.read(1, 2), Ok(vec![23, 29]));
    let (result, comments) = wasm_ordinary_output(&wasm, &b).expect("B remains live");
    result.expect("B guest still reads");
    assert_eq!(comments, ["19,23", "29,31"]);
    first.dispose().expect("dispose first print");
    assert_eq!(old_b.read(1, 2), Err(T::AccessDenied));
    assert_eq!(old_c.read(1, 2), Ok(vec![41, 43]));
    assert_eq!(old_d.read(1, 2), Ok(vec![59, 61]));
    second.dispose().expect("dispose second print");
    assert_eq!(old_c.read(1, 2), Err(T::AccessDenied));
    assert_eq!(old_d.read(1, 2), Err(T::AccessDenied));
    for owner in [&a, &b, &c, &d] {
        let metadata = owner.metadata();
        assert_eq!(metadata.phase(), "disposed");
        assert!(!metadata.ready());
        assert!(metadata.pieces().is_empty());
        assert_eq!(metadata.retained_payload_bytes(), 0);
        assert_eq!(metadata.retained_name_bytes(), 0);
        assert_eq!(metadata.staged_payload_bytes(), 0);
        assert_eq!(metadata.staged_name_bytes(), 0);
    }
}

#[test]
fn metadata_excludes_payload() {
    for binding in [transport_native_binding(), transport_wasm_binding()] {
        let (store, owner) = driver::owner_store(&binding).expect("owner");
        binding
            .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
            .expect("real preparation commits numbers and empty π");
        let plan = store.current(&owner).expect("published plan");
        let before = plan.metadata();
        assert_eq!(before.read_calls(), 0);
        assert_eq!(before.requested_read_bytes(), 0);
        assert_eq!(before.returned_read_bytes(), 0);
        assert_eq!(driver::piece(&plan, "numbers").read(1, 2), Ok(vec![5, 11]));
        let metadata = plan.metadata();
        assert_eq!(
            metadata.module_id(),
            if binding.artifact_kind() == "native" {
                "fresh-native"
            } else {
                "fresh-wasm"
            }
        );
        assert_eq!(metadata.phase(), "ready");
        assert!(metadata.ready());
        assert_eq!(
            metadata.pieces(),
            &[("numbers".to_owned(), 4), ("π".to_owned(), 0)]
        );
        assert_eq!(metadata.retained_payload_bytes(), 4);
        assert_eq!(metadata.retained_name_bytes(), 9);
        assert_eq!(metadata.staged_payload_bytes(), 0);
        assert_eq!(metadata.staged_name_bytes(), 0);
        assert_eq!(metadata.put_calls(), 2);
        assert_eq!(metadata.prepare_invocations(), 1);
        assert_eq!(metadata.read_calls(), 1);
        assert_eq!(metadata.requested_read_bytes(), 2);
        assert_eq!(metadata.returned_read_bytes(), 2);
        // Exact complete production Debug shape rejects extra raw-payload,
        // allocator or peak-memory fields, not merely one chosen leak spelling.
        let expected = format!(
            "PreparedPlanMetadata {{ module_id: {:?}, phase: \"ready\", ready: true, pieces: [(\"numbers\", 4), (\"π\", 0)], retained_payload_bytes: 4, retained_name_bytes: 9, staged_payload_bytes: 0, staged_name_bytes: 0, put_calls: 2, read_calls: 1, requested_read_bytes: 2, returned_read_bytes: 2, prepare_invocations: 1 }}",
            metadata.module_id()
        );
        let debug = format!("{metadata:?}");
        assert_eq!(debug, expected);
        assert_eq!(format!("{owner:?}"), expected);
        assert!(!debug.contains("[3, 5, 11, 17]"));
        assert!(!debug.contains("[5, 11]"));
    }
}

#[test]
fn names_are_exact() {
    use slicer_sdk::preparation::{PreparedPlanOutput, PreparedPlanView, PrintPreparationInput};

    // Independent literal payloads make every normalization/aliasing pair observable.
    const PIECES: [(&str, &[u8]); 9] = [
        ("A", &[1, 31]),
        ("a", &[2, 37]),
        ("./a", &[3, 41]),
        ("a/../b", &[4, 43]),
        ("b", &[5, 47]),
        ("π", &[6, 53]),
        (" ", &[7, 59]),
        ("é", &[8, 61]),
        ("e\u{301}", &[9, 67]),
    ];
    fn prepare_names(
        _: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        for (name, bytes) in PIECES {
            output.put(name, bytes)?;
        }
        output.put("empty", &[])?;
        Ok(())
    }

    let binding = driver::native_binding(
        "exact-names",
        Some(driver::declaration(
            vec![],
            PIECES
                .iter()
                .map(|(name, _)| (*name).into())
                .chain(std::iter::once("empty".into()))
                .collect(),
        )),
        driver::native_artifact(prepare_names, driver::native_schema()),
    )
    .expect("freeze real native output facade");
    let (store, owner) = driver::owner_store(&binding).expect("owner");
    binding
        .prepare(
            &owner,
            slicer_wasm_host::preparation::PreparationInputData::for_owner(&owner),
        )
        .expect("complete puts publish after call-local output is dropped");

    // A new read scope obtains fresh production plan/piece resources after publication.
    binding
        .with_native_plan(&owner, |_| {
            let plan = PreparedPlanView::current()?;
            for (name, bytes) in PIECES {
                let piece = plan.open(name)?.expect("each exact key exists");
                assert_eq!(piece.len(), Ok(2), "{name:?}");
                assert_eq!(piece.read(0, 2)?, bytes, "{name:?}");
            }
            let empty = plan.open("empty")?.expect("zero-length piece exists");
            assert_eq!(empty.len(), Ok(0));
            assert_eq!(empty.read(0, 1)?, Vec::<u8>::new());
            Ok(())
        })
        .expect("fresh native read facade");
    // Fresh storage handles independently reopen the same published exact names.
    let plan = store.current(&owner).expect("published collection");
    for (name, bytes) in PIECES {
        assert_eq!(
            driver::piece(&plan, name).read(0, 2),
            Ok(bytes.to_vec()),
            "{name:?}"
        );
    }
    let empty = driver::piece(&plan, "empty");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.read(0, 1), Ok(vec![]));
}

#[test]
fn ranges_are_checked() {
    use slicer_sdk::preparation::{PreparationError, PreparedPlanView};
    use slicer_wasm_host::preparation::{
        WitPlanAccess, WitPreparationError, WitPreparedPiece, WitPreparedPlanView,
    };
    use wasmtime::component::Resource;

    // Literal oracles, independent of the production clipping calculation.
    const CASES: [(u64, u64, Option<&[u8]>); 11] = [
        (1, 2, Some(&[5, 11])),
        (0, u64::MAX, Some(&[3, 5, 11, 17])),
        (1, u64::MAX, Some(&[5, 11, 17])),
        (4, 0, Some(&[])),
        (4, 1, Some(&[])),
        (4, u64::MAX, Some(&[])),
        (5, 0, None),
        (u64::MAX, 0, None),
        (u64::MAX, 1, None),
        (0, 0, Some(&[])),
        (1, 0, Some(&[])),
    ];
    let binding = transport_native_binding();
    for wit in [false, true] {
        for (offset, maximum, bytes) in CASES {
            // A new owner per boundary/case starts all read counters at zero.
            // In particular each maximum request can be accounted without overflow.
            let (store, owner) = driver::owner_store(&binding).expect("fresh owner");
            binding
                .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
                .expect("publish literal four-byte piece");
            assert_eq!(owner.metadata().read_calls(), 0);
            assert_eq!(owner.metadata().requested_read_bytes(), 0);
            assert_eq!(owner.metadata().returned_read_bytes(), 0);
            if wit {
                let mut state =
                    PreparedHostState::new(driver::ordinary_context("fresh-native"), owner.clone());
                let plan = WitPlanAccess::current(&mut state)
                    .expect("real WIT current handler")
                    .expect("published plan");
                let piece = WitPreparedPlanView::open(&mut state, plan, "numbers".into())
                    .expect("real WIT open handler")
                    .expect("authorized")
                    .expect("literal piece");
                assert_eq!(
                    WitPreparedPiece::read(
                        &mut state,
                        Resource::new_borrow(piece.rep()),
                        offset,
                        maximum,
                    )
                    .expect("real WIT read handler must not trap"),
                    bytes.map(Vec::from).ok_or(WitPreparationError::RangeError),
                    "WIT ({offset}, {maximum})"
                );
                if (offset, maximum) == (1, 2) {
                    for (offset, maximum, expected) in
                        [(1, 0, vec![]), (0, 1, vec![3]), (1, 2, vec![5, 11])]
                    {
                        assert_eq!(
                            WitPreparedPiece::read(
                                &mut state,
                                Resource::new_borrow(piece.rep()),
                                offset,
                                maximum,
                            )
                            .expect("repeat real WIT handler"),
                            Ok(expected),
                            "same WIT resource has no shared cursor"
                        );
                    }
                }
                WitPreparedPiece::drop(&mut state, piece).expect("drop WIT piece");
            } else {
                binding
                    .with_native_plan(&owner, |_| {
                        let plan = PreparedPlanView::current()?;
                        let piece = plan.open("numbers")?.expect("literal piece");
                        assert_eq!(
                            piece.read(offset, maximum),
                            bytes.map(Vec::from).ok_or(PreparationError::RangeError),
                            "native ({offset}, {maximum})"
                        );
                        if (offset, maximum) == (1, 2) {
                            assert_eq!(piece.read(1, 0), Ok(vec![]));
                            assert_eq!(piece.read(0, 1), Ok(vec![3]));
                            assert_eq!(piece.read(1, 2), Ok(vec![5, 11]));
                        }
                        Ok(())
                    })
                    .expect("real native prepared read facade");
            }
            // Keep the owner-private authority alive through both read boundaries.
            assert!(store.current(&owner).is_ok());
        }
    }
}

#[test]
fn empty_optional_required() {
    use slicer_schema::preparation::PreparationTransportError;
    use slicer_sdk::preparation::{PreparationError, PreparedPlanView};
    // Pin missing required pieces through both production transport facades;
    // the SDK test remains an additional independent witness.
    for binding in [transport_native_binding(), transport_wasm_binding()] {
        let (store, owner) = driver::owner_store(&binding).expect("owner");
        assert!(matches!(
            store.current(&owner),
            Err(PreparationFailure::Transport(
                PreparationTransportError::NotReady
            ))
        ));
        if binding.artifact_kind() == "native" {
            binding
                .with_native_plan(&owner, |_| {
                    assert_eq!(
                        PreparedPlanView::current().err(),
                        Some(PreparationError::NotReady)
                    );
                    Ok(())
                })
                .expect("uncalled native scope");
        } else {
            let (result, comments) =
                wasm_ordinary_output(&binding, &owner).expect("uncalled WASM scope");
            let error = result.expect_err("guest current must be NotReady");
            assert_eq!(error.message, "layer preparation: not-ready");
            assert!(error.fatal);
            assert!(comments.is_empty());
        }
        binding
            .prepare(&owner, driver::fixture_input(&owner, Vec::<u8>::new()))
            .expect("empty preparation");
        let plan = store.current(&owner).expect("ready-empty current");
        assert!(plan.open("optional").expect("optional open").is_none());
        assert!(owner.metadata().pieces().is_empty());
        if binding.artifact_kind() == "native" {
            binding
                .with_native_plan(&owner, |_| {
                    let plan = PreparedPlanView::current()?;
                    assert!(plan.open("optional")?.is_none());
                    let error = plan
                        .require("numbers")
                        .err()
                        .expect("required piece missing");
                    assert_eq!(error.code, 57401);
                    assert_eq!(error.message, "required preparation piece missing: numbers");
                    assert!(error.fatal);
                    Ok(())
                })
                .expect("ready-empty native facade");
        } else {
            let (result, comments) =
                wasm_ordinary_output(&binding, &owner).expect("ready-empty WASM facade");
            let error = result.expect_err("required piece missing");
            assert_eq!(error.code, 57401);
            assert_eq!(error.message, "required preparation piece missing: numbers");
            assert!(error.fatal);
            assert!(comments.is_empty());
        }
        // A distinct owner is needed: preparation is once per owner.
        let malformed = store.create_owner(&binding).expect("malformed owner");
        binding
            .prepare(&malformed, driver::fixture_input(&malformed, vec![3]))
            .expect("publish opaque malformed payload");
        if binding.artifact_kind() == "native" {
            let error = native_ordinary_output(&binding, &malformed)
                .expect_err("module decoder rejects malformed bytes");
            let PreparationFailure::Module(error) = error else {
                panic!("wrong boundary: {error:?}")
            };
            assert_eq!(error.code, 57403);
            assert_eq!(error.message, "transport fixture malformed payload");
            assert!(error.fatal);
        } else {
            let (result, comments) =
                wasm_ordinary_output(&binding, &malformed).expect("WASM decoder call");
            let error = result.expect_err("guest decoder rejects malformed bytes");
            assert_eq!(error.code, 57403);
            assert_eq!(error.message, "transport fixture malformed payload");
            assert!(error.fatal);
            assert!(comments.is_empty());
        }
    }
}

#[test]
fn failures_never_publish() {
    use slicer_schema::preparation::PreparationTransportError as T;

    fn assert_unpublished(
        store: &slicer_wasm_host::preparation::PreparedOwnerStore,
        owner: &slicer_wasm_host::preparation::PreparationOwner,
    ) {
        assert!(matches!(
            store.current(owner),
            Err(PreparationFailure::Transport(T::NotReady))
        ));
        let metadata = owner.metadata();
        assert!(!metadata.ready());
        assert!(metadata.pieces().is_empty());
        assert_eq!(metadata.retained_payload_bytes(), 0);
        assert_eq!(metadata.retained_name_bytes(), 0);
        assert_eq!(metadata.staged_payload_bytes(), 0);
        assert_eq!(metadata.staged_name_bytes(), 0);
        assert_eq!(metadata.prepare_invocations(), 1);
    }

    for binding in [transport_native_binding(), transport_wasm_binding()] {
        // Independently abandon successful staging without finish, using only
        // the production store API (not a guest-driven early-drop mode).
        let (store, owner) = driver::owner_store(&binding).expect("unfinished-drop owner");
        let mut transaction = store.begin(&owner).expect("begin unfinished output");
        transaction
            .put("numbers", &[3, 5, 11, 17])
            .expect("stage before drop");
        assert_eq!(owner.metadata().staged_payload_bytes(), 4);
        assert_eq!(owner.metadata().staged_name_bytes(), 7);
        drop(transaction);
        assert!(matches!(
            store.current(&owner),
            Err(PreparationFailure::Transport(T::NotReady))
        ));
        let metadata = owner.metadata();
        assert!(!metadata.ready());
        assert!(metadata.pieces().is_empty());
        assert_eq!(metadata.retained_payload_bytes(), 0);
        assert_eq!(metadata.retained_name_bytes(), 0);
        assert_eq!(metadata.staged_payload_bytes(), 0);
        assert_eq!(metadata.staged_name_bytes(), 0);
        assert_eq!(metadata.prepare_invocations(), 0);

        // Fixture modes: 255 propagates duplicate; 252 catches duplicate; 251
        // catches empty name; 254 traps; 253 returns the original module error.
        // These adapter-failure legs supplement the independent drop above.
        for mode in [255, 252, 251, 254, 253] {
            let (store, owner) = driver::owner_store(&binding).expect("fresh failure owner");
            let mut publication_called = false;
            let error = binding
                .prepare_with_publication(&owner, driver::fixture_input(&owner, vec![mode]), |_| {
                    publication_called = true;
                    Ok(())
                })
                .expect_err("failure must not publish");
            match (mode, error) {
                (255, PreparationFailure::Module(error)) => {
                    assert_eq!(error.code, 1);
                    assert_eq!(error.message, "layer preparation: duplicate-name");
                    assert!(error.fatal);
                }
                (252 | 251, PreparationFailure::Transport(T::PoisonedOutput)) => {
                    assert_eq!(
                        owner.writer_fault(),
                        Some(if binding.artifact_kind() == "wasm" {
                            // The generated guest completion marker crosses
                            // WIT as poison, not as the SDK-local first error.
                            T::PoisonedOutput
                        } else if mode == 252 {
                            T::DuplicateName
                        } else {
                            T::InvalidName
                        })
                    );
                }
                (254, PreparationFailure::Trap { detail }) => {
                    if binding.artifact_kind() == "wasm" {
                        // AC-N3/design preserve guest trap detail, not the native panic payload.
                        assert!(!detail.is_empty(), "lost WASM trap detail");
                        assert!(
                            detail.contains("error while executing at wasm backtrace:"),
                            "lost original WASM trap detail: {detail}"
                        );
                    } else {
                        assert!(
                            detail.contains("transport fixture trap"),
                            "lost original trap detail: {detail}"
                        );
                    }
                }
                (253, PreparationFailure::Module(error)) => {
                    assert_eq!(error.code, 57402);
                    assert_eq!(error.message, "transport fixture failure");
                    assert!(
                        !error.fatal,
                        "failed required dependency must preserve nonfatal field"
                    );
                }
                (_, error) => panic!(
                    "{} mode {mode}: wrong failure {error:?}",
                    binding.artifact_kind()
                ),
            }
            assert!(!publication_called);
            assert_unpublished(&store, &owner);
            println!(
                "{} mode={mode}: original failure preserved; current=NotReady retained=0 staged=0",
                binding.artifact_kind()
            );
        }
    }

    // The shared WASM fixture has no uncaught-empty-name mode. Drive this
    // additional native module through the same production SDK writer adapter.
    fn uncaught_empty_name(
        _: &slicer_sdk::preparation::PrintPreparationInput,
        output: &mut slicer_sdk::preparation::PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        output.put("numbers", &[3, 5, 11, 17])?;
        output.put("", &[])?;
        Ok(())
    }
    let binding = driver::native_binding(
        "uncaught-empty-name",
        Some(driver::declaration(vec![], vec!["numbers".into()])),
        driver::native_artifact(uncaught_empty_name, driver::native_schema()),
    )
    .expect("freeze native uncaught empty-name module");
    let (store, owner) = driver::owner_store(&binding).expect("empty-name owner");
    let error = binding
        .prepare(
            &owner,
            slicer_wasm_host::preparation::PreparationInputData::for_owner(&owner),
        )
        .expect_err("uncaught empty name fails");
    let PreparationFailure::Module(error) = error else {
        panic!("wrong empty-name failure: {error:?}")
    };
    assert_eq!(error.code, 1);
    assert_eq!(error.message, "layer preparation: invalid-name");
    assert!(error.fatal);
    assert_eq!(owner.writer_fault(), Some(T::InvalidName));
    assert_unpublished(&store, &owner);
}

fn assert_completion_provenance(binding: slicer_wasm_host::preparation::FrozenPreparationBinding) {
    use slicer_schema::preparation::PreparationTransportError as T;

    for mode in [251, 252, 253, 250] {
        let (store, owner) = driver::owner_store(&binding).expect("fresh provenance owner");
        let mut publication_called = false;
        let error = binding
            .prepare_with_publication(&owner, driver::fixture_input(&owner, vec![mode]), |_| {
                publication_called = true;
                Ok(())
            })
            .expect_err("failed preparation cannot publish");
        match (mode, error) {
            (251 | 252, PreparationFailure::Transport(T::PoisonedOutput)) => {
                assert_eq!(
                    owner.writer_fault(),
                    Some(if binding.artifact_kind() == "wasm" {
                        T::PoisonedOutput
                    } else if mode == 251 {
                        T::InvalidName
                    } else {
                        T::DuplicateName
                    })
                );
            }
            (253 | 250, PreparationFailure::Module(error)) => {
                let (code, message, fatal) = if mode == 253 {
                    (57402, "transport fixture failure", false)
                } else {
                    (
                        slicer_sdk::preparation::POISONED_PREPARATION_CODE,
                        "transport fixture reserved code",
                        true,
                    )
                };
                assert_eq!(error.code, code);
                assert_eq!(error.message, message);
                assert_eq!(error.fatal, fatal);
                assert_eq!(owner.writer_fault(), None);
            }
            (_, error) => panic!("{} mode {mode}: {error:?}", binding.artifact_kind()),
        }
        assert!(!publication_called);
        assert!(matches!(
            store.current(&owner),
            Err(PreparationFailure::Transport(T::NotReady))
        ));
        let metadata = owner.metadata();
        assert!(!metadata.ready());
        assert!(metadata.pieces().is_empty());
        assert_eq!(metadata.retained_payload_bytes(), 0);
        assert_eq!(metadata.retained_name_bytes(), 0);
        assert_eq!(metadata.staged_payload_bytes(), 0);
        assert_eq!(metadata.staged_name_bytes(), 0);
        println!(
            "{} mode={mode}: provenance preserved, unpublished",
            binding.artifact_kind()
        );
    }
}

#[test]
fn native_completion_cannot_mask_later_module_error() {
    use slicer_schema::preparation::PreparationTransportError as T;

    fn complete_then_fail(
        _: &slicer_sdk::preparation::PrintPreparationInput,
        output: &mut slicer_sdk::preparation::PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        output.put("numbers", &[3, 5, 11, 17])?;
        let _ = output.put("", &[]);
        let _completion = output.complete(Ok(()));
        Err(slicer_sdk::error::ModuleError::non_fatal(
            57402,
            "transport fixture failure",
        ))
    }

    let binding = driver::native_binding(
        "complete-then-fail",
        Some(driver::declaration(vec![], vec!["numbers".into()])),
        driver::native_artifact(complete_then_fail, driver::native_schema()),
    )
    .expect("freeze native completion-then-failure module");
    let (store, owner) = driver::owner_store(&binding).expect("completion provenance owner");
    let mut publication_called = false;
    let error = binding
        .prepare_with_publication(
            &owner,
            slicer_wasm_host::preparation::PreparationInputData::for_owner(&owner),
            |_| {
                publication_called = true;
                Ok(())
            },
        )
        .expect_err("later module error must prevent publication");
    let PreparationFailure::Module(error) = error else {
        panic!("later module error was masked: {error:?}")
    };
    assert_eq!(error.code, 57402);
    assert_eq!(error.message, "transport fixture failure");
    assert!(!error.fatal);
    assert!(!publication_called);
    assert_eq!(owner.writer_fault(), Some(T::InvalidName));
    assert!(matches!(
        store.current(&owner),
        Err(PreparationFailure::Transport(T::NotReady))
    ));
    let metadata = owner.metadata();
    assert!(!metadata.ready());
    assert!(metadata.pieces().is_empty());
    assert_eq!(metadata.retained_payload_bytes(), 0);
    assert_eq!(metadata.retained_name_bytes(), 0);
    assert_eq!(metadata.staged_payload_bytes(), 0);
    assert_eq!(metadata.staged_name_bytes(), 0);
}

#[test]
fn generated_native_completion_provenance() {
    use preparation_transport_path::TransportPathModule;
    use slicer_wasm_host::preparation::{freeze_native, NativePreparationArtifact};

    let binding = freeze_native(
        "generated-provenance".into(),
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
        NativePreparationArtifact::Prepared {
            entry: TransportPathModule::__slicer_prepared_native_entry(),
            schema: TransportPathModule::__SLICER_PREPARATION_SCHEMA,
        },
    )
    .expect("freeze generated pair");
    assert_completion_provenance(binding);
}

#[test]
fn native_sdk_oversized_reads_account_original_maximum() {
    use preparation_transport_path::TransportPathModule;
    use slicer_sdk::preparation::PreparedPlanView;
    use slicer_wasm_host::preparation::{freeze_native, NativePreparationArtifact};

    let binding = freeze_native(
        "original-read-maximum".into(),
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
        NativePreparationArtifact::Prepared {
            entry: TransportPathModule::__slicer_prepared_native_entry(),
            schema: TransportPathModule::__SLICER_PREPARATION_SCHEMA,
        },
    )
    .expect("freeze generated pair");
    for (offset, expected) in [(0, vec![3, 5, 11, 17]), (1, vec![5, 11, 17])] {
        // Each maximum request needs fresh accounting: a second would overflow.
        let (_store, owner) = driver::owner_store(&binding).expect("fresh read owner");
        binding
            .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
            .expect("prepare generated native pair");
        let bytes = binding
            .with_native_plan(&owner, |_entry| {
                let plan = PreparedPlanView::current()?;
                let piece = plan.require("numbers")?;
                Ok(piece.read(offset, u64::MAX)?)
            })
            .expect("oversized module-side read through SDK callbacks");
        assert_eq!(bytes, expected);
        let metadata = owner.metadata();
        assert_eq!(metadata.read_calls(), 1);
        assert_eq!(metadata.requested_read_bytes(), u64::MAX);
        assert_eq!(
            metadata.returned_read_bytes(),
            if offset == 0 { 4 } else { 3 }
        );
    }
}

#[test]
fn wasm_completion_provenance() {
    assert_completion_provenance(transport_wasm_binding());
}

#[test]
fn publication_hook_is_atomic() {
    use slicer_schema::preparation::PreparationTransportError as T;

    for binding in [transport_native_binding(), transport_wasm_binding()] {
        for (reject, bytes) in [
            (true, vec![3, 5, 11, 17]),
            (false, vec![3, 5, 11, 17]),
            (false, vec![]),
        ] {
            let empty = bytes.is_empty();
            let expected = if empty {
                vec![]
            } else {
                vec![("numbers".to_owned(), 4), ("π".to_owned(), 0)]
            };
            let (store, owner) = driver::owner_store(&binding).expect("fresh publication owner");
            let mut hook_called = false;
            let result = binding.prepare_with_publication(
                &owner,
                driver::fixture_input(&owner, bytes),
                |staged| {
                    hook_called = true;
                    // Real production hook sees names/lengths but cannot obtain a
                    // ready plan or a partial collection while validation is active.
                    assert_eq!(staged.piece_count(), expected.len());
                    assert_eq!(staged.pieces(), expected);
                    assert_eq!(staged.owner().metadata().phase(), "staging");
                    let metadata = owner.metadata();
                    assert!(!metadata.ready());
                    assert!(metadata.pieces().is_empty());
                    assert_eq!(metadata.retained_payload_bytes(), 0);
                    assert_eq!(metadata.staged_payload_bytes(), if empty { 0 } else { 4 });
                    assert!(matches!(
                        store.current(&owner),
                        Err(PreparationFailure::Transport(T::NotReady))
                    ));
                    if reject {
                        Err(PreparationFailure::PublicationRejected {
                            detail: "projection rejected".into(),
                        })
                    } else {
                        Ok(())
                    }
                },
            );
            assert!(
                hook_called,
                "validator must execute even for the empty collection"
            );
            if reject {
                let error = result.expect_err("validator rejection aborts publication");
                let PreparationFailure::PublicationRejected { detail } = error else {
                    panic!("wrong rejection: {error:?}")
                };
                assert_eq!(detail, "projection rejected");
                assert!(matches!(
                    store.current(&owner),
                    Err(PreparationFailure::Transport(T::NotReady))
                ));
                let metadata = owner.metadata();
                assert!(!metadata.ready());
                assert!(metadata.pieces().is_empty());
                assert_eq!(metadata.retained_payload_bytes(), 0);
                assert_eq!(metadata.retained_name_bytes(), 0);
                assert_eq!(metadata.staged_payload_bytes(), 0);
                assert_eq!(metadata.staged_name_bytes(), 0);
            } else {
                result.expect("accepting hook publishes atomically");
                let plan = store
                    .current(&owner)
                    .expect("entire collection ready after hook");
                let metadata = owner.metadata();
                assert!(metadata.ready());
                assert_eq!(metadata.pieces(), expected);
                assert_eq!(metadata.staged_payload_bytes(), 0);
                assert_eq!(metadata.staged_name_bytes(), 0);
                assert_eq!(metadata.retained_payload_bytes(), if empty { 0 } else { 4 });
                if empty {
                    assert!(plan.open("numbers").expect("empty open").is_none());
                    assert!(plan.open("π").expect("empty open").is_none());
                } else {
                    assert_eq!(
                        driver::piece(&plan, "numbers").read(0, 4),
                        Ok(vec![3, 5, 11, 17])
                    );
                    assert_eq!(driver::piece(&plan, "π").read(0, 0), Ok(vec![]));
                }
            }
            println!(
                "{} reject={reject} empty={empty}: hook observed staging before ready",
                binding.artifact_kind()
            );
        }
    }
}

#[test]
fn wasm_owned_output_publishes() {
    let binding = driver::wasm_binding(
        "owned-output-publication",
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
    )
    .expect("freeze real transport fixture");
    let (store, owner) = driver::owner_store(&binding).expect("owner");
    binding
        .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
        .expect("owned output drop must preserve publication");
    assert_eq!(owner.metadata().prepare_invocations(), 1);
    println!("WASM completed prepare_invocations=1");
    let plan = store.current(&owner).expect("published plan is ready");
    let numbers = driver::piece(&plan, "numbers");
    assert_eq!(numbers.len(), 4);
    assert_eq!(numbers.read(0, u64::MAX), Ok(vec![3, 5, 11, 17]));
    assert_eq!(driver::piece(&plan, "π").len(), 0);
}

#[test]
fn wasm_owned_output_preserves_module_error() {
    use slicer_schema::preparation::PreparationTransportError;

    let binding = driver::wasm_binding(
        "owned-output-error",
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
    )
    .expect("freeze real transport fixture");
    let (store, owner) = driver::owner_store(&binding).expect("owner");
    let error = binding
        .prepare(&owner, driver::fixture_input(&owner, vec![253]))
        .expect_err("fixture mode 253 fails");
    let PreparationFailure::Module(error) = error else {
        panic!("original module error replaced: {error:?}");
    };
    assert_eq!(error.code, 57402);
    assert_eq!(error.message, "transport fixture failure");
    assert!(!error.fatal);
    assert!(matches!(
        store.current(&owner),
        Err(PreparationFailure::Transport(
            PreparationTransportError::NotReady
        ))
    ));
    assert!(!owner.metadata().ready());
    assert_eq!(owner.metadata().prepare_invocations(), 1);
    assert_eq!(owner.metadata().retained_payload_bytes(), 0);
    println!(
        "preserved code=57402 message=\"transport fixture failure\" fatal=false; current=NotReady"
    );
}

#[test]
fn wasm_caught_writer_faults_are_transport() {
    use slicer_schema::preparation::PreparationTransportError;
    for mode in [251, 252] {
        let binding = driver::wasm_binding(
            "caught-writer-fault",
            Some(driver::declaration(
                vec!["fixture".into()],
                vec!["numbers".into(), "π".into()],
            )),
        )
        .expect("freeze fixture");
        let (store, owner) = driver::owner_store(&binding).expect("owner");
        assert!(matches!(
            binding.prepare(&owner, driver::fixture_input(&owner, vec![mode])),
            Err(PreparationFailure::Transport(
                PreparationTransportError::PoisonedOutput
            ))
        ));
        assert!(owner.writer_fault().is_some());
        assert!(!owner.metadata().ready());
        assert_eq!(owner.metadata().retained_payload_bytes(), 0);
        assert_eq!(owner.metadata().staged_payload_bytes(), 0);
        assert_eq!(owner.metadata().prepare_invocations(), 1);
        assert!(matches!(
            store.current(&owner),
            Err(PreparationFailure::Transport(
                PreparationTransportError::NotReady
            ))
        ));
        println!("WASM mode={mode} Transport(PoisonedOutput); current=NotReady retained=0");
    }
}

#[test]
fn fixture_ordinary_reads_two_bounded_pairs() {
    use slicer_sdk::preparation::LayerPreparation;
    use slicer_sdk::traits::LayerModule;
    let binding = driver::native_binding(
        "bounded-pairs",
        Some(driver::declaration(
            vec!["fixture".into()],
            vec!["numbers".into(), "π".into()],
        )),
        driver::native_artifact(
            preparation_transport_path::TransportPathModule::prepare_print,
            driver::native_schema(),
        ),
    )
    .expect("pair");
    // Keep the print authority alive: owners intentionally hold only weak storage.
    let (store, owner) = driver::owner_store(&binding).expect("print owner");
    binding
        .prepare(&owner, driver::fixture_input(&owner, vec![3, 5, 11, 17]))
        .expect("prepare");
    binding
        .with_native_plan(&owner, |_| {
            let config = slicer_ir::ConfigView::from_map(Default::default());
            let module = preparation_transport_path::TransportPathModule::from_config(&config)?;
            module.run_path_optimization(
                0,
                &[],
                &mut slicer_sdk::postpass_builders::GcodeOutputBuilder::new(),
                &mut slicer_sdk::layer_collection_builder::LayerCollectionBuilder::new(),
                &config,
            )
        })
        .expect("ordinary fixture call");
    assert!(store.current(&owner).is_ok());
    let metadata = owner.metadata();
    assert_eq!(metadata.read_calls(), 2);
    assert_eq!(metadata.requested_read_bytes(), 4);
    assert_eq!(metadata.returned_read_bytes(), 4);
}

#[test]
fn preparation_permissions_are_separate() {
    use slicer_schema::preparation::PreparationTransportError;
    use slicer_wasm_host::preparation::PreparationInputData;

    fn copy_fixture(
        input: &slicer_sdk::preparation::PrintPreparationInput,
        output: &mut slicer_sdk::preparation::PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        output.put("staged", &[9])?;
        output.put("view", &input.read_fixture(0, u64::MAX)?)?;
        Ok(())
    }

    let binding = |reads| {
        driver::native_binding(
            "permission-boundary",
            Some(driver::declaration(
                reads,
                vec!["staged".into(), "view".into()],
            )),
            driver::native_artifact(copy_fixture, driver::native_schema()),
        )
        .expect("freeze actual native pair")
    };
    let assert_denied = |binding: &slicer_wasm_host::preparation::FrozenPreparationBinding,
                         store: &slicer_wasm_host::preparation::PreparedOwnerStore,
                         owner: &slicer_wasm_host::preparation::PreparationOwner,
                         input,
                         reason| {
        let mut publication_called = false;
        let result = binding.prepare_with_publication(owner, input, |_| {
            publication_called = true;
            Ok(())
        });
        assert!(
            matches!(&result, Err(PreparationFailure::Module(error))
                if error.message == "layer preparation: access-denied" && error.fatal),
            "{reason}: expected AccessDenied, got {result:?}"
        );
        assert!(
            !publication_called,
            "denied read must not reach publication"
        );
        assert!(
            !owner.metadata().ready(),
            "denied read must not publish ready"
        );
        assert_eq!(owner.metadata().retained_payload_bytes(), 0);
        assert!(matches!(
            store.current(owner),
            Err(PreparationFailure::Transport(
                PreparationTransportError::NotReady
            ))
        ));
    };

    // Real ordinary read metadata remains separate from preparation metadata.
    // freeze_native and prepare_with_publication accept no LoadedModule or
    // ordinary execution context: this proves declaration separation, not
    // installation of runtime ordinary-read authority at this boundary.
    let manifest = slicer_scheduler::preparation::PreparationManifest::new(
        slicer_scheduler::manifest::LoadedModuleBuilder::new(
            "permission-boundary",
            Default::default(),
            "Layer::Perimeters",
            "perimeters-module",
            "unused.wasm",
        )
        .ir_reads(vec!["SliceIR.regions.polygons".into()])
        .build(),
        Some(driver::declaration(
            vec![],
            vec!["staged".into(), "view".into()],
        )),
    );
    assert_eq!(manifest.module().ir_reads(), &["SliceIR.regions.polygons"]);
    let ungranted = driver::native_binding(
        manifest.module().id(),
        manifest.preparation().cloned(),
        driver::native_artifact(copy_fixture, driver::native_schema()),
    )
    .expect("freeze companion declaration with ordinary IR reads");
    assert!(ungranted
        .declaration()
        .expect("preparation declaration")
        .input_reads()
        .is_empty());
    let (store, owner) = driver::owner_store(&ungranted).expect("ungranted owner");
    assert_denied(
        &ungranted,
        &store,
        &owner,
        driver::fixture_input(&owner, vec![3, 5]),
        "native declaration-permission bypass",
    );

    // The real guest reads the controlled input through the owned imported
    // resource, not the native SDK callback facade.
    let wasm_ungranted =
        driver::wasm_binding("wasm-permission-boundary", manifest.preparation().cloned())
            .expect("freeze ungranted real component");
    let (store, owner) = driver::owner_store(&wasm_ungranted).expect("WASM ungranted owner");
    assert_denied(
        &wasm_ungranted,
        &store,
        &owner,
        driver::fixture_input(&owner, vec![3, 5, 11, 17]),
        "WASM direct imported-resource declaration-permission bypass",
    );

    let granted = binding(vec!["fixture".into()]);
    let (store, owner) = driver::owner_store(&granted).expect("granted owner");
    assert_denied(
        &granted,
        &store,
        &owner,
        PreparationInputData::for_owner(&owner),
        "production input denies even an exact fixture grant",
    );
    let foreign = store.create_owner(&granted).expect("different owner");
    assert_denied(
        &granted,
        &store,
        &owner,
        driver::fixture_input(&foreign, vec![3, 5]),
        "foreign-owner fixture cannot grant access",
    );

    granted
        .prepare(&owner, driver::fixture_input(&owner, vec![3, 5]))
        .expect("exact declaration grant and matching fixture owner remain valid");
    let plan = store.current(&owner).expect("legitimate grant publishes");
    assert_eq!(
        driver::piece(&plan, "view").read(0, u64::MAX),
        Ok(vec![3, 5])
    );
}

#[test]
fn declarations_agree() {
    assert_eq!(
        parse_preparation_declaration("[module]\nid = 'plain'"),
        Ok(None)
    );
    let declared = parse_preparation_declaration(MANIFEST)
        .expect("valid declaration")
        .expect("table present");
    assert_eq!(
        declared.interface(),
        "slicer:layer-preparation/prepare@1.0.0"
    );
    assert_eq!(declared.input_reads(), &["fixture", "exact.input"]);
    assert_eq!(declared.views(), &["輪郭-plan", "Case Sensitive"]);
    let prepared =
        driver::wasm_binding("prepared", Some(declared)).expect("typed composed capability agrees");
    let frozen = prepared.declaration().expect("frozen declaration");
    assert_eq!(frozen.interface(), "slicer:layer-preparation/prepare@1.0.0");
    assert_eq!(frozen.input_reads(), &["fixture", "exact.input"]);
    assert_eq!(frozen.views(), &["輪郭-plan", "Case Sensitive"]);
    assert_eq!(prepared.artifact_kind(), "wasm");
    let engine = driver::engine();
    let plain = freeze_wasm(
        "plain".into(),
        None,
        engine.clone(),
        driver::old_component(&engine),
        "Layer::Perimeters".into(),
    )
    .expect("plain frozen without preparation");
    assert_eq!(plain.declaration(), None);
    assert_eq!(plain.stage_id(), "Layer::Perimeters");
    let native = driver::native_binding(
        "native-prepared",
        parse_preparation_declaration(MANIFEST).expect("native declaration"),
        driver::native_artifact(paired_prepare, driver::native_schema()),
    )
    .expect("matching native compiled pair freezes");
    assert_eq!(native.artifact_kind(), "native");
    assert_eq!(native.stage_id(), "Layer::Perimeters");
    assert_eq!(native.declaration(), prepared.declaration());
    let native_plain = driver::native_binding(
        "native-plain",
        None,
        slicer_wasm_host::preparation::NativePreparationArtifact::Plain(
            slicer_sdk::native::NativeStageEntry::Layer(driver::native_ordinary),
        ),
    )
    .expect("native plain freezes without preparation");
    assert_eq!(native_plain.declaration(), None);
    assert_eq!(native_plain.artifact_kind(), "native");
}

#[test]
fn declaration_export_rejections() {
    assert_eq!(REJECTED_PREPARATION_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(
        parse_preparation_declaration("preparation = 'x'"),
        Err(D::InvalidTable("preparation must be a table".into()))
    );
    assert_eq!(
        parse_preparation_declaration("[preparation]"),
        Err(D::MissingInterface("preparation.interface".into()))
    );
    assert_eq!(
        parse_preparation_declaration(
            "[preparation]\ninterface = 'slicer:layer-preparation/prepare@1.0.0'"
        ),
        Err(D::MissingInputReads("preparation.input_reads".into()))
    );
    assert_eq!(
        parse_preparation_declaration(
            &MANIFEST.replace("['fixture', 'exact.input']", "['fixture', 'fixture']")
        ),
        Err(D::DuplicateInputRead("fixture".into()))
    );
    assert_eq!(
        parse_preparation_declaration(
            &MANIFEST.replace("['輪郭-plan', 'Case Sensitive']", "['view', 'view']")
        ),
        Err(D::DuplicateViewName("view".into()))
    );
    assert_eq!(
        parse_preparation_declaration(&MANIFEST.replace("['fixture', 'exact.input']", "42")),
        Err(D::InvalidInputRead(
            "preparation.input_reads must be an array of strings".into()
        ))
    );
    assert_eq!(
        parse_preparation_declaration(&MANIFEST.replace("['輪郭-plan', 'Case Sensitive']", "['']")),
        Err(D::InvalidViewName(
            "preparation.views entries must be nonempty strings".into()
        ))
    );
    assert_eq!(
        parse_preparation_declaration(&format!("{MANIFEST}input-reads = []\n")),
        Err(D::UnknownKey("input-reads".into()))
    );
    let wrong = parse_preparation_declaration(&MANIFEST.replace(
        "slicer:layer-preparation/prepare@1.0.0",
        "prepared-perimeters-module",
    ));
    match wrong {
        Err(D::UnsupportedInterface(detail)) => assert_eq!(detail, "preparation.interface must equal slicer:layer-preparation/prepare@1.0.0; got String(\"prepared-perimeters-module\")"),
        other => panic!("expected UnsupportedInterface, got {other:?}"),
    }
    let reject = |result: Result<_, PreparationFailure>, expected| match result {
        Err(PreparationFailure::Declaration(actual)) => assert_eq!(actual, expected),
        Err(other) => panic!("wrong failure: {other:?}"),
        Ok(_) => panic!("unexpected successful freeze"),
    };
    reject(
        driver::wasm_binding("undeclared", None),
        D::UndeclaredExport("undeclared".into()),
    );
    let engine = driver::engine();
    reject(
        freeze_wasm(
            "missing".into(),
            Some(driver::declaration(vec![], vec![])),
            engine.clone(),
            driver::old_component(&engine),
            "Layer::Perimeters".into(),
        ),
        D::MissingExport("missing".into()),
    );
    reject(
        freeze_wasm(
            "non-layer".into(),
            Some(driver::declaration(vec![], vec![])),
            engine.clone(),
            driver::fixture_component(&engine),
            "PrePass::MeshRepair".into(),
        ),
        D::UnsupportedStage("non-layer".into()),
    );
    reject(
        freeze_wasm(
            "wrong-stage".into(),
            Some(driver::declaration(vec![], vec![])),
            engine.clone(),
            driver::fixture_component(&engine),
            "Layer::Perimeters".into(),
        ),
        D::IncompatibleExport("wrong-stage".into()),
    );
    let fixture_text = driver::fixture_component_text();
    freeze_wasm(
        "identity-control".into(),
        Some(driver::declaration(vec![], vec![])),
        engine.clone(),
        driver::component_from_text(&engine, &fixture_text),
        driver::path_stage_id().into(),
    )
    .expect("unchanged component text must pass the same production typed freeze");
    for (defect, component) in [
        (
            "wasm-wrong-resource",
            driver::wrong_resource_component(&engine, &fixture_text),
        ),
        (
            "wasm-wrong-signature",
            driver::wrong_signature_component(&engine, &fixture_text),
        ),
    ] {
        // If freeze ever accepts this component, take the actual owner/prepare
        // path, rather than treating a helper's schema comparison as rejection.
        let store = slicer_wasm_host::preparation::PreparedOwnerStore::new();
        let mut rejected_owner = None;
        let outcome = freeze_wasm(
            defect.into(),
            Some(driver::declaration(vec![], vec![])),
            engine.clone(),
            component,
            driver::path_stage_id().into(),
        )
        .and_then(|binding| {
            let owner = store.create_owner(&binding)?;
            let input = driver::fixture_input(&owner, Vec::<u8>::new());
            rejected_owner = Some(owner.clone());
            binding.prepare(&owner, input)
        });
        match outcome {
            Err(PreparationFailure::Declaration(actual)) => {
                assert_eq!(actual, D::IncompatibleExport(defect.into()), "{defect}");
            }
            other => panic!("{defect}: expected typed-freeze IncompatibleExport, got {other:?}"),
        }
        assert!(
            rejected_owner.is_none(),
            "{defect}: rejected freeze must not reach production owner creation or prepare"
        );
        println!("{defect}: IncompatibleExport; owner/prepare continuation not reached");
    }
    reject(
        driver::native_binding(
            "native-undeclared",
            None,
            driver::native_artifact(rejected_prepare, driver::native_schema()),
        ),
        D::UndeclaredExport("native-undeclared".into()),
    );
    reject(
        driver::native_binding(
            "native-missing",
            Some(driver::declaration(vec![], vec![])),
            slicer_wasm_host::preparation::NativePreparationArtifact::Plain(
                slicer_sdk::native::NativeStageEntry::Layer(driver::native_ordinary),
            ),
        ),
        D::MissingExport("native-missing".into()),
    );
    let mut wrong_schema = driver::native_schema();
    wrong_schema.qualified_export = "slicer:layer-preparation/prepare@2.0.0#prepare-print";
    reject(
        driver::native_binding(
            "native-incompatible",
            Some(driver::declaration(vec![], vec![])),
            driver::native_artifact(rejected_prepare, wrong_schema),
        ),
        D::IncompatibleExport("native-incompatible".into()),
    );
    let mut wrong_schema = driver::native_schema();
    wrong_schema.ordinary_world = "not-a-layer-world";
    reject(
        driver::native_binding(
            "native-non-layer",
            Some(driver::declaration(vec![], vec![])),
            driver::native_artifact(rejected_prepare, wrong_schema),
        ),
        D::UnsupportedStage("native-non-layer".into()),
    );
    assert_eq!(
        REJECTED_PREPARATION_CALLS.load(Ordering::SeqCst),
        0,
        "rejected native preparation pointers must never be invoked"
    );
}

#[test]
fn plain_artifact_and_stage_bindings() {
    let path = driver::old_artifact_path();
    assert!(path.ends_with("target/preparation-pilot/plain-before.component.wasm"));
    // Recorded-hash comparison, not a re-hash: the host's directly reachable
    // dependencies expose no hashing crate. This pins the ADR-0066 provenance
    // sidecar and measured archive length, but cannot detect same-length tampering.
    assert_eq!(
        std::fs::read_to_string(driver::required_artifact(
            "target/preparation-pilot/plain-before.sha256",
        ))
        .expect("archive provenance sidecar")
        .trim(),
        "6499a7d809a1812bc581adab2bc394ca660f209b0f38b9ce459e9c47c43e5c5c"
    );
    assert_eq!(
        std::fs::metadata(&path).expect("archive metadata").len(),
        480830
    );
    let engine = driver::engine();
    let component = driver::old_component(&engine);
    assert_eq!(
        component.get_export_index(None, "slicer:layer-preparation/prepare@1.0.0"),
        None
    );
    let mut linker = wasmtime::component::Linker::new(&engine);
    slicer_wasm_host::host::LayerModule::add_to_linker::<
        _,
        wasmtime::component::HasSelf<slicer_wasm_host::host::HostExecutionContext>,
    >(&mut linker, |ctx| ctx)
    .expect("ordinary-only linker");
    let mut store = wasmtime::Store::new(&engine, driver::ordinary_context("archived"));
    slicer_wasm_host::host::LayerModule::instantiate(&mut store, &component, &linker)
        .expect("archived ordinary signatures instantiate without preparation imports");
    let stages: Vec<_> = slicer_schema::STAGES
        .iter()
        .filter(|stage| stage.tier_id == slicer_schema::TIER_LAYER && !stage.wit_package.is_empty())
        .collect();
    assert!(!stages.is_empty(), "no schema-derived Layer worlds");
    let mut resolve = wit_parser::Resolve::default();
    resolve
        .push_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../slicer-schema/wit"))
        .expect("resolve canonical worlds");
    for stage in stages {
        PreparedHostState::linker(&engine, stage.stage_id)
            .expect("every schema-backed Layer stage has production composed linker");
        let plain = resolve
            .worlds
            .iter()
            .find(|(_, world)| world.name == stage.wit_world)
            .map(|(_, world)| world)
            .expect("schema ordinary world exists");
        let prepared_name = format!("prepared-{}", stage.wit_world);
        let prepared = resolve
            .worlds
            .iter()
            .find(|(_, world)| world.name == prepared_name)
            .map(|(_, world)| world)
            .expect("schema composed world exists");
        for (ordinary, composed) in [
            (&plain.imports, &prepared.imports),
            (&plain.exports, &prepared.exports),
        ] {
            assert!(
                !ordinary.is_empty(),
                "ordinary stage surface must not be vacuous"
            );
            for (key, item) in ordinary {
                let counterpart = composed
                    .get(key)
                    .expect("composed world preserves ordinary item");
                // Shared interface/type identity preserves every function's
                // parameter/result resource types, not just the export name.
                assert_eq!(
                    format!("{item:?}"),
                    format!("{counterpart:?}"),
                    "ordinary import/export/parameter identity changed for {}",
                    stage.stage_id
                );
                if let wit_parser::WorldItem::Interface { id, .. } = item {
                    let package = resolve.interfaces[*id].package.expect("packaged interface");
                    assert_ne!(
                        resolve.packages[package].name.name, "layer-preparation",
                        "plain world acquired a mandatory preparation interface"
                    );
                }
            }
        }
    }
}
