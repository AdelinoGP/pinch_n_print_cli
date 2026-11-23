//! Independent literal witnesses at the real native/component preparation seam.
//!
//! AC-1 partitions the entire standard observation piece into two literal
//! witnesses. The complement is exhaustive: no read family, prefix, error or
//! summary line can silently escape comparison. AC-2 pins selected model roles.

use super::whole_print_input_driver::{
    capture_markers, literal_inputs, literal_support, prevalidate, run_pair, InsertionOrder,
    InvalidInputs, LegResult, OwnerReadScenario, Package, PreparedDriver, Product, Scenario,
};
use slicer_schema::preparation::PreparationTransportError;
use slicer_wasm_host::preparation::{PreparationFailure, PreparationInputError};
use std::sync::Arc;

/// Select complete read sections, never individual successful record fields.
fn read_sections(observations: &str, headers: &[&str]) -> String {
    partition_sections(observations, headers).0
}

/// Both halves together account for every emitted line, including the prefix
/// and final error count. An additional read family fails the complement pin.
fn partition_sections(observations: &str, headers: &[&str]) -> (String, String) {
    let mut selected = false;
    let mut lines = Vec::new();
    let mut complement = Vec::new();
    for line in observations.lines() {
        if line.starts_with("read ") {
            selected = headers.iter().any(|header| line.starts_with(header));
        }
        if line.starts_with("read_errors=") {
            selected = false;
        }
        if selected {
            lines.push(line);
        } else {
            complement.push(line);
        }
    }
    (
        format!("{}\n", lines.join("\n")),
        format!("{}\n", complement.join("\n")),
    )
}

fn assert_literal_lines(actual: &str, expected: &str, name: &str) {
    let actual: Vec<_> = actual.lines().collect();
    let expected: Vec<_> = expected.lines().collect();
    assert!(!actual.is_empty(), "{name}: missing observed population");
    assert!(!expected.is_empty(), "{name}: missing literal population");
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "{name}: observation line {index}");
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "{name}: exact line population"
    );
}

fn observations<'a>(leg: &'a LegResult, name: &str) -> &'a str {
    assert!(leg.failure.is_none(), "{name}: {:?}", leg.failure);
    leg.observations
        .as_deref()
        .unwrap_or_else(|| panic!("{name}: preparation must publish the observations piece"))
}

fn assert_unpublished(leg: &LegResult, name: &str) {
    assert!(
        matches!(
            &leg.current,
            Err(PreparationFailure::Transport(
                PreparationTransportError::NotReady
            ))
        ),
        "{name}: current must remain NotReady"
    );
    assert_eq!(leg.metadata.phase(), "unready", "{name}");
    assert!(!leg.metadata.ready(), "{name}");
    assert!(leg.metadata.pieces().is_empty(), "{name}");
    assert_eq!(leg.metadata.retained_payload_bytes(), 0, "{name}");
    assert_eq!(leg.metadata.retained_name_bytes(), 0, "{name}");
    assert_eq!(leg.metadata.staged_payload_bytes(), 0, "{name}");
    assert_eq!(leg.metadata.staged_name_bytes(), 0, "{name}");
    assert!(
        leg.observations.is_none(),
        "{name}: no discarded piece is readable"
    );
    assert!(
        !matches!(
            &leg.failure,
            Some(PreparationFailure::Transport(
                PreparationTransportError::PoisonedOutput
            ))
        ),
        "{name}: input read errors must not poison output"
    );
}

fn assert_before_call(leg: &LegResult, expected: PreparationInputError, name: &str) {
    let Some(PreparationFailure::Input(actual)) = &leg.failure else {
        panic!(
            "{name}: expected exact input rejection, got {:?}",
            leg.failure
        );
    };
    assert_eq!(actual, &expected, "{name}");
    assert_eq!(leg.metadata.prepare_invocations(), 0, "{name}");
    assert!(
        leg.markers.is_empty(),
        "{name}: no native/guest log markers"
    );
    assert!(leg.counters_before_prepare.is_none(), "{name}");
    assert_eq!(leg.metadata.put_calls(), 0, "{name}: no transaction writes");
    assert_unpublished(leg, name);
}

#[test]
fn undeclared_owner_and_future_reads_denied() {
    // Ordinary IR permission is not preparation permission (docs/03). In
    // particular, absence without an exact optional grant must not return None.
    let scenario = Scenario {
        input_reads: Some(
            [
                "layer_plan",
                "slices",
                "region_map",
                "mesh",
                "selected_targets",
                "surface_classification",
                "seam_plan",
                "support_analysis",
                "support_geometry",
                "fixture",
            ]
            .map(str::to_owned)
            .to_vec(),
        ),
        support_plan: Product::Absent,
        fixture_mode: "attempt_denied_reads".into(),
        ..Scenario::default()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
        assert!(leg.marker_count("wpi read read_fixture") > 0, "{name}");
        assert!(leg.marker_count("wpi read support_plan") > 0, "{name}");
        assert_eq!(
            leg.marker_count("err read_fixture access-denied"),
            1,
            "{name}"
        );
        assert_eq!(
            leg.marker_count("err support_plan access-denied"),
            2,
            "{name}"
        );
        assert!(
            observed.contains(
                "read read_fixture offset=0 max_bytes=1\nerr read_fixture access-denied\n"
            ),
            "{name}"
        );
        assert!(
            observed.contains(
                "read support_plan offset=0 max_items=1\nerr support_plan access-denied\n"
            ),
            "{name}"
        );
        assert!(
            observed.contains("layer_count=2\n"),
            "{name}: successful reads also publish"
        );
        assert_eq!(observed.lines().last(), Some("read_errors=4"), "{name}");
        assert!(
            leg.metadata.ready(),
            "{name}: caught errors permit publication"
        );
        assert_eq!(leg.metadata.put_calls(), 1, "{name}");
    }

    // No preparation tokens at all: these fixture artifacts still have their
    // ordinary world, but every entered typed resource read must be denied.
    let ordinary_only = Scenario {
        input_reads: Some(Vec::new()),
        support_plan: Product::Absent,
        fixture_mode: "publish_read_errors".into(),
        ..Scenario::default()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &ordinary_only).unwrap();
    let errors = [
        "err layer_count access-denied",
        "err layers access-denied",
        "err surface_objects access-denied",
        "err mesh_objects access-denied",
        "err support_analysis access-denied",
        "err support_geometry access-denied",
        "err support_plan access-denied",
    ];
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
        assert!(leg.marker_count("wpi read layer_count") > 0, "{name}");
        assert_eq!(
            observed
                .lines()
                .filter(|line| line.starts_with("err "))
                .collect::<Vec<_>>(),
            errors,
            "{name}"
        );
        assert!(leg.metadata.ready(), "{name}");
    }

    // A was bound through from_committed, B invokes: neither adapter may turn
    // this into a before-call denial or disclose A's committed products.
    let cross_owner = Scenario {
        owner_reads: OwnerReadScenario::CrossOwner,
        ..Scenario::default()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &cross_owner).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        assert_eq!(
            leg.metadata.prepare_invocations(),
            1,
            "{name}: B actually entered"
        );
        assert!(leg.marker_count("wpi read layer_count") > 0, "{name}");
        assert_eq!(
            observed
                .lines()
                .filter(|line| line.starts_with("err "))
                .collect::<Vec<_>>(),
            errors,
            "{name}: foreign input resource"
        );
        assert!(
            !observed.contains("layer_count=2"),
            "{name}: no foreign data"
        );
    }

    let disposed = Scenario {
        owner_reads: OwnerReadScenario::DisposedOwner,
        ..Scenario::default()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &disposed).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        assert!(
            observations(leg, name).contains("layer_count=2\n"),
            "{name}: live positive control"
        );
        assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
        assert!(leg.marker_count("wpi read layer_count") > 0, "{name}");
        assert_eq!(
            leg.post_disposal_read,
            Some(Err(PreparationInputError::AccessDenied)),
            "{name}: real retained handle after disposal"
        );
        assert_eq!(leg.metadata.phase(), "disposed", "{name}");
        assert_eq!(leg.metadata.retained_payload_bytes(), 0, "{name}");
    }

    let failed = Scenario {
        fixture_mode: "fail_after_staging".into(),
        ..scenario.clone()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &failed).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        assert!(
            matches!(
                &leg.failure,
                Some(PreparationFailure::Input(
                    PreparationInputError::AccessDenied
                ))
            ),
            "{name}: {:?}",
            leg.failure
        );
        assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
        assert_eq!(
            leg.marker_count("err support_plan access-denied"),
            1,
            "{name}"
        );
        assert_eq!(
            leg.metadata.put_calls(),
            1,
            "{name}: valid piece staged before returned failure"
        );
        assert_unpublished(leg, name);
    }

    let unsupported = Scenario {
        input_reads: Some(vec!["perimeter_ir".into()]),
        ..Scenario::default()
    };
    let pair = run_pair(Package::Path, &["path-optimizer"], &unsupported).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        assert_before_call(
            leg,
            PreparationInputError::UnsupportedRead {
                read: "perimeter_ir".into(),
            },
            name,
        );
    }
}

#[test]
fn required_inputs_fail_before_call() {
    for (scenario, read) in [
        (
            Scenario {
                surface: Product::Absent,
                ..Scenario::default()
            },
            "surface_classification",
        ),
        (
            Scenario {
                support_plan: Product::Absent,
                ..Scenario::default()
            },
            "support_plan",
        ),
    ] {
        let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            assert_before_call(
                leg,
                PreparationInputError::MissingRequired { read: read.into() },
                name,
            );
        }
    }

    for wasm in [false, true] {
        for missing_config in [true, false] {
            let scenario = Scenario::default();
            let driver = PreparedDriver::new(Package::Path, &scenario, wasm).unwrap();
            let mut seed = literal_inputs(Package::Path, &scenario);
            let (stage, expected_detail) = if missing_config {
                // Remove precisely tool 2's map-space key, keeping tool 1 and
                // its modifier config. A near-match is never a valid fallback.
                let missing = slicer_ir::RegionKey {
                    global_layer_index: 7,
                    object_id: "variant-cube".into(),
                    region_id: 0,
                    variant_chain: vec![("material".into(), slicer_ir::PaintValue::ToolIndex(2))],
                };
                assert!(Arc::make_mut(&mut seed.region_map)
                    .entries
                    .remove(&missing)
                    .is_some());
                (
                    "Layer::PathOptimization",
                    r#"Layer::PathOptimization / com.test.whole-print-input-path: MissingModelConfig { key: RegionKey { global_layer_index: 7, object_id: "variant-cube", region_id: 0, variant_chain: [("material", ToolIndex(2))] } }"#,
                )
            } else {
                (
                    "PrePass::NotALayer",
                    r#"PrePass::NotALayer / com.test.whole-print-input-path: UnknownLayerStage { stage_id: "PrePass::NotALayer" }"#,
                )
            };
            let inputs = seed.commit().unwrap();
            let selection = slicer_wasm_host::preparation::PreparationSelectionContext::new(
                stage.into(),
                Package::Path.module_id(),
                vec!["path-optimizer".into()],
                Default::default(),
                scenario.invocation(Package::Path),
            );
            let (result, markers) =
                capture_markers(|| prevalidate(&driver.owner, inputs, selection));
            let Err(PreparationFailure::Input(PreparationInputError::SelectionRejected { detail })) =
                result
            else {
                panic!(
                    "wasm={wasm}/stage={stage}: expected SelectionRejected, got {:?}",
                    result.err()
                );
            };
            assert_eq!(
                detail, expected_detail,
                "wasm={wasm}/stage={stage}: full diagnostic"
            );
            assert!(markers.is_empty(), "no native/guest invocation markers");
            let metadata = driver.owner.metadata();
            assert_eq!(metadata.prepare_invocations(), 0);
            assert_eq!(metadata.phase(), "unready");
            assert_eq!(metadata.put_calls(), 0);
            assert_eq!(metadata.staged_payload_bytes(), 0);
            assert!(matches!(
                driver.store.current(&driver.owner),
                Err(PreparationFailure::Transport(
                    PreparationTransportError::NotReady
                ))
            ));
        }
    }

    let optional = Scenario {
        support_plan: Product::Absent,
        ..Scenario::default()
    }
    .with_optional_reads(Package::Path);
    let pair = run_pair(Package::Path, &["path-optimizer"], &optional).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
        assert_eq!(leg.marker_count("wpi read support_plan"), 1, "{name}");
        assert_eq!(
            read_sections(observed, &["read support_plan "]),
            "read support_plan offset=0 max_items=18446744073709551615\nsupport_plan absent\n",
            "{name}"
        );
        assert!(
            leg.metadata.ready(),
            "{name}: granted optional absence is legal"
        );
    }
}

#[test]
fn invalid_ranges_and_snapshots() {
    for (invalid, detail) in [
        (InvalidInputs::DuplicateScheduleIndex, "layers.global_layers.index and z must be unique and increasing"),
        (InvalidInputs::MissingScheduleSlice, "slices missing schedule global_layer_index 7"),
        (InvalidInputs::DuplicateFullKey, "slices.regions duplicate full region key at layer 7: \"variant-cube\"/7/[(\"material\", ToolIndex(1))]"),
        (InvalidInputs::NonfiniteZ, "layers.global_layers.z must be finite"),
    ] {
        let scenario = Scenario { invalid, ..Scenario::default() };
        for wasm in [false, true] {
            let driver = PreparedDriver::new(Package::Path, &scenario, wasm).unwrap();
            let seed = literal_inputs(Package::Path, &scenario);
            let (result, markers) = capture_markers(|| seed.commit());
            assert_eq!(result.err(), Some(PreparationInputError::InvalidInput { detail: detail.into() }), "{invalid:?}/wasm={wasm}");
            assert!(markers.is_empty(), "malformed constructor never invokes a fixture");
            let metadata = driver.owner.metadata();
            assert_eq!(metadata.prepare_invocations(), 0);
            assert_eq!(metadata.phase(), "unready");
            assert_eq!(metadata.put_calls(), 0);
            assert_eq!(metadata.staged_payload_bytes(), 0);
            assert!(matches!(driver.store.current(&driver.owner), Err(PreparationFailure::Transport(PreparationTransportError::NotReady))));
        }
    }

    // These are independently pinned seed cardinalities, not lengths learned
    // from a returned full batch. Both invalid offsets must reject even count 0.
    let families = [
        ("layers", "", 2),
        ("regions", "7:", 4),
        ("region_configs", "7:", 4),
        ("selected_targets", "7:", 4),
        ("surface_objects", "", 2),
        ("surface_layers", "7:", 2),
        ("seams", "7:", 1),
        ("mesh_objects", "", 4),
        ("mesh_vertices", "variant-cube:", 3),
        ("mesh_triangles", "variant-cube:", 1),
        ("support_analysis", "", 2),
        ("support_geometry", "", 2),
        ("support_plan", "", 5),
    ];
    let mut ops = Vec::new();
    let mut expected = Vec::new();
    for (method, argument, len) in families {
        ops.push(format!("{method}:{argument}{}:0", len + 1));
        ops.push(format!("{method}:{argument}18446744073709551615:1"));
        expected.extend([
            format!("err {method} range-error"),
            format!("err {method} range-error"),
        ]);
    }
    ops.extend(
        [
            "mesh_vertices:missing:0:1",
            "mesh_triangles:missing:0:1",
            "mesh_bounds:missing",
            "raycast:missing:0:0:1",
            "normal:missing:0:0:1",
        ]
        .map(str::to_owned),
    );
    expected.extend(
        [
            "err mesh_vertices unknown-object: missing",
            "err mesh_triangles unknown-object: missing",
            "err mesh_queries unknown-object: missing",
            "err mesh_queries unknown-object: missing",
            "err mesh_queries unknown-object: missing",
        ]
        .map(str::to_owned),
    );
    assert_eq!(ops.len(), 31);
    for mode in ["publish_read_errors", "fail_after_staging"] {
        let scenario = Scenario {
            batch_ops: ops.join(";"),
            fixture_mode: mode.into(),
            ..Scenario::default()
        };
        let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            assert_eq!(leg.metadata.prepare_invocations(), 1, "{mode}/{name}");
            assert_eq!(
                leg.metadata.put_calls(),
                1,
                "{mode}/{name}: valid staged piece"
            );
            for (method, _, _) in families {
                assert!(
                    leg.marker_count(&format!("wpi read {method}")) > 0,
                    "{mode}/{name}/{method}"
                );
                assert_eq!(
                    leg.marker_count(&format!("err {method} range-error")),
                    2,
                    "{mode}/{name}/{method}"
                );
            }
            assert_eq!(
                leg.marker_count("err mesh_queries unknown-object: missing"),
                3,
                "{mode}/{name}"
            );
            if mode == "publish_read_errors" {
                let observed = observations(leg, name);
                let (_, batches) = observed
                    .split_once("batch[0] ")
                    .expect("actual invalid batch population");
                assert_eq!(
                    batches
                        .lines()
                        .filter(|line| line.starts_with("err "))
                        .collect::<Vec<_>>(),
                    expected,
                    "{name}: exact actual resource errors"
                );
                assert_eq!(observed.lines().last(), Some("read_errors=32"), "{name}");
                assert!(
                    leg.metadata.ready(),
                    "{name}: caught range/query errors do not poison output"
                );
            } else {
                assert!(
                    matches!(&leg.failure, Some(PreparationFailure::Input(PreparationInputError::UnknownObject { object_id })) if object_id == "missing"),
                    "{name}: {:?}",
                    leg.failure
                );
                assert_unpublished(leg, name);
            }
        }
    }

    // Counters are the real shared backing's checked accounting. Overflow must
    // reject before projection (the empty-bounds query otherwise reports
    // InvalidInput), without partially incrementing either counter.
    let scenario = Scenario::default();
    let driver = PreparedDriver::new(Package::Path, &scenario, false).unwrap();
    let input = prevalidate(
        &driver.owner,
        literal_inputs(Package::Path, &scenario).commit().unwrap(),
        scenario.selection(Package::Path, &["path-optimizer"]),
    )
    .unwrap();
    let reader = input
        .controlled_native_input(&driver.owner, &driver.binding)
        .unwrap();
    for (calls, records) in [(u64::MAX, 0), (5, u64::MAX)] {
        input.controlled_set_input_counters(calls, records).unwrap();
        assert!(matches!(
            reader.layers(0, 1),
            Err(PreparationInputError::ArithmeticOverflow)
        ));
        assert!(
            matches!(
                reader.mesh_queries(&[
                    slicer_sdk::preparation::PreparationMeshQuery::ObjectBounds {
                        object_id: "empty-mesh".into()
                    }
                ]),
                Err(PreparationInputError::ArithmeticOverflow)
            ),
            "counter rejection must precede projection"
        );
        assert_eq!(input.controlled_input_counters().unwrap(), (calls, records));
    }
    input.controlled_set_input_counters(0, 0).unwrap();
    let layers = reader.layers(0, 1).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].index, 0);
    assert_eq!(input.controlled_input_counters().unwrap(), (1, 1));

    for counter_seed in [(u64::MAX, 0), (0, u64::MAX)] {
        let scenario = Scenario {
            counter_seed: Some(counter_seed),
            fixture_mode: "fail_after_staging".into(),
            ..Scenario::default()
        };
        let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            assert_eq!(
                leg.counters_before_prepare,
                Some(Ok(counter_seed)),
                "{name}: real counter seed"
            );
            assert_eq!(leg.metadata.prepare_invocations(), 1, "{name}");
            assert!(leg.marker_count("wpi read layers") > 0, "{name}");
            assert_eq!(
                leg.marker_count("err layers arithmetic-overflow"),
                1,
                "{name}"
            );
            assert!(
                matches!(
                    &leg.failure,
                    Some(PreparationFailure::Input(
                        PreparationInputError::ArithmeticOverflow
                    ))
                ),
                "{name}: {:?}",
                leg.failure
            );
            assert_eq!(leg.metadata.put_calls(), 1, "{name}");
            assert_unpublished(leg, name);
        }
    }
}

#[test]
fn typed_slice_surface_literals() {
    let scenario = Scenario::default();
    let seed = literal_inputs(Package::Path, &scenario);
    // Guard the population before comparing either transport leg. These are
    // preconditions, not the oracle: all transported values below are literals.
    assert_eq!(seed.layers.global_layers.len(), 2);
    assert_eq!(seed.slices.len(), 2);
    assert_eq!(seed.slices[1].regions.len(), 4);
    let enriched = &seed.slices[1].regions[0];
    assert_eq!(enriched.polygons.len(), 1);
    assert_eq!(enriched.polygons[0].holes.len(), 1);
    assert_eq!(enriched.top_solid_fill.len(), 1);
    assert_eq!(enriched.bottom_solid_fill.len(), 1);
    assert_eq!(enriched.internal_solid_fill.len(), 1);
    assert_eq!(enriched.bridge_areas.len(), 1);
    assert_eq!(enriched.internal_bridge_areas.len(), 1);
    assert_eq!(enriched.internal_bridge_angles_deg.len(), 1);
    let surface = seed.surface.as_ref().expect("nonempty surface seed");
    assert_eq!(surface.per_object.len(), 2);
    let classified = &surface.per_object["variant-cube"];
    assert_eq!(classified.facet_classes.len(), 6);
    assert_eq!(classified.surface_groups.len(), 1);
    assert_eq!(classified.bridge_regions.len(), 1);
    assert_eq!(classified.overhang_regions.len(), 1);
    assert_eq!(surface.overhang_quartile_polygons.len(), 2);
    assert_eq!(surface.prev_layer_boundaries.len(), 2);
    assert_eq!(seed.mesh.objects.len(), 4);
    let seam = seed.seam.as_ref().expect("nonempty seam seed");
    assert_eq!(seam.entries.len(), 1);
    assert_eq!(seam.entries[0].scored_candidates.len(), 1);
    let analysis = seed
        .support_analysis
        .as_ref()
        .expect("nonempty support analysis seed");
    assert_eq!(analysis.candidates.len(), 1);
    assert_eq!(analysis.model_occupancy.len(), 2);
    assert_eq!(analysis.cantilever_surfaces.len(), 2);
    assert_eq!(analysis.termination_surfaces.len(), 2);
    assert_eq!(analysis.support_territory.len(), 2);
    assert_eq!(analysis.family_assignments.len(), 1);
    assert_eq!(
        seed.support_geometry
            .as_ref()
            .expect("nonempty support geometry seed")
            .entries
            .len(),
        2
    );
    assert_eq!(
        seed.support_plan
            .as_ref()
            .expect("nonempty support plan seed")
            .entries
            .len(),
        5
    );

    let (expected, expected_complement) = standard_literals();
    let pair = run_pair(Package::Path, &["path-optimizer"], &scenario)
        .expect("freeze and invoke both real preparation legs");
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        assert_eq!(leg.marker_count("wpi read regions"), 2, "{name}");
        assert_eq!(leg.marker_count("wpi read surface_layers"), 2, "{name}");
        assert_eq!(leg.marker_count("wpi read surface_objects"), 1, "{name}");
        let (owned, complement) = partition_sections(
            observed,
            &[
                "read regions layer=7 ",
                "read surface_layers layer=7 ",
                "read surface_objects ",
            ],
        );
        assert_literal_lines(
            &owned,
            expected,
            &format!("{name}: enriched committed slices and object-keyed surfaces"),
        );
        assert_literal_lines(
            &complement,
            expected_complement,
            &format!("{name}: exhaustive standard-observation complement"),
        );
    }
}

/// Shared independently authored literals, never captured output. AC-4/5 use
/// the same documented seed authority without copying a production projection.
fn standard_literals() -> (&'static str, &'static str) {
    // Hand-authored from the mm seed rectangles and the observation line
    // contract: 1 mm = 10_000 integer units. No projector builds this oracle.
    // The modifier literal is the FNV identity of the 0.25–0.75 mm footprint
    // under parent 7/object variant-cube, not a preparation converter result.
    let expected = r#"read regions layer=7 offset=0 max_items=18446744073709551615
regions count=4
regions[0] key=[7|"context-only"|9|[]] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[0].polygons count=1 geometry=[{contour=[(200000,0),(220000,0),(220000,20000),(200000,20000)],holes=[]}]
regions[0].infill_areas count=0 geometry=[]
regions[0].top_solid_fill count=0 geometry=[]
regions[0].bottom_solid_fill count=0 geometry=[]
regions[0].bridge_areas count=0 geometry=[]
regions[0].sparse_infill_area count=0 geometry=[]
regions[0].internal_solid_fill count=0 geometry=[]
regions[0].internal_bridge_areas count=0 geometry=[]
regions[0].raft_fill count=0 geometry=[]
regions[0].annotations count=0
regions[1] key=[7|"variant-cube"|7|["material"=tool_index(1)]] z=0.3 nonplanar_surface=None height=0.2 top=Some(0) bottom=Some(1) is_bridge=true orientation=27.0 internal_bridge_angles=[33.0] phase=committed_prepass
regions[1].polygons count=1 geometry=[{contour=[(0,0),(120000,0),(120000,30000),(0,30000)],holes=[[(2500,2500),(7500,2500),(7500,7500),(2500,7500)]]}]
regions[1].infill_areas count=0 geometry=[]
regions[1].top_solid_fill count=1 geometry=[{contour=[(0,0),(20000,0),(20000,20000),(0,20000)],holes=[]}]
regions[1].bottom_solid_fill count=1 geometry=[{contour=[(30000,0),(50000,0),(50000,20000),(30000,20000)],holes=[]}]
regions[1].bridge_areas count=1 geometry=[{contour=[(90000,0),(110000,0),(110000,20000),(90000,20000)],holes=[]}]
regions[1].sparse_infill_area count=0 geometry=[]
regions[1].internal_solid_fill count=1 geometry=[{contour=[(60000,0),(80000,0),(80000,20000),(60000,20000)],holes=[]}]
regions[1].internal_bridge_areas count=1 geometry=[{contour=[(90000,0),(110000,0),(110000,20000),(90000,20000)],holes=[]}]
regions[1].raft_fill count=0 geometry=[]
regions[1].annotations count=1
regions[1].annotations semantic=material polygons=1
regions[1].annotations[material][0] count=4 values=[tool_index(1),none,flag(true),scalar(0.25;bits=3e800000)]
regions[2] key=[7|"variant-cube"|7|["material"=tool_index(2)]] z=0.3 nonplanar_surface=None height=0.2 top=Some(0) bottom=Some(1) is_bridge=true orientation=27.0 internal_bridge_angles=[33.0] phase=committed_prepass
regions[2].polygons count=1 geometry=[{contour=[(300000,0),(420000,0),(420000,30000),(300000,30000)],holes=[[(302500,2500),(307500,2500),(307500,7500),(302500,7500)]]}]
regions[2].infill_areas count=0 geometry=[]
regions[2].top_solid_fill count=1 geometry=[{contour=[(300000,0),(320000,0),(320000,20000),(300000,20000)],holes=[]}]
regions[2].bottom_solid_fill count=1 geometry=[{contour=[(330000,0),(350000,0),(350000,20000),(330000,20000)],holes=[]}]
regions[2].bridge_areas count=1 geometry=[{contour=[(390000,0),(410000,0),(410000,20000),(390000,20000)],holes=[]}]
regions[2].sparse_infill_area count=0 geometry=[]
regions[2].internal_solid_fill count=1 geometry=[{contour=[(360000,0),(380000,0),(380000,20000),(360000,20000)],holes=[]}]
regions[2].internal_bridge_areas count=1 geometry=[{contour=[(390000,0),(410000,0),(410000,20000),(390000,20000)],holes=[]}]
regions[2].raft_fill count=0 geometry=[]
regions[2].annotations count=1
regions[2].annotations semantic=material polygons=1
regions[2].annotations[material][0] count=4 values=[tool_index(2),none,flag(true),scalar(0.25;bits=3e800000)]
regions[3] key=[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[3].polygons count=1 geometry=[{contour=[(2500,2500),(7500,2500),(7500,7500),(2500,7500)],holes=[]}]
regions[3].infill_areas count=0 geometry=[]
regions[3].top_solid_fill count=0 geometry=[]
regions[3].bottom_solid_fill count=0 geometry=[]
regions[3].bridge_areas count=0 geometry=[]
regions[3].sparse_infill_area count=0 geometry=[]
regions[3].internal_solid_fill count=0 geometry=[]
regions[3].internal_bridge_areas count=0 geometry=[]
regions[3].raft_fill count=0 geometry=[]
regions[3].annotations count=0
read surface_layers layer=7 offset=0 max_items=18446744073709551615
surface_layers present
surface_layers count=2
surface_layers[0] object="context-only" layer=7 quartiles=1
surface_layers[0].quartile[0] value=4
surface_layers[0].quartile[0] count=1 geometry=[{contour=[(200000,0),(220000,0),(220000,20000),(200000,20000)],holes=[]}]
surface_layers[0].prev_boundaries count=1 geometry=[{contour=[(-10000,-10000),(130000,-10000),(130000,40000),(-10000,40000)],holes=[]}]
surface_layers[1] object="variant-cube" layer=7 quartiles=1
surface_layers[1].quartile[0] value=2
surface_layers[1].quartile[0] count=1 geometry=[{contour=[(0,0),(20000,0),(20000,20000),(0,20000)],holes=[]}]
surface_layers[1].prev_boundaries count=1 geometry=[{contour=[(-10000,-10000),(130000,-10000),(130000,40000),(-10000,40000)],holes=[]}]
read surface_objects offset=0 max_items=18446744073709551615
surface_objects present
surface_objects count=2
surface_objects[0] object="context-only" facets=0 groups=0 bridges=0 overhangs=0
surface_objects[1] object="variant-cube" facets=6 groups=1 bridges=1 overhangs=1
surface_objects[1].facet[0] class=normal
surface_objects[1].facet[1] class=near_horizontal(12.0)
surface_objects[1].facet[2] class=overhang(60.0)
surface_objects[1].facet[3] class=bridge
surface_objects[1].facet[4] class=top_surface
surface_objects[1].facet[5] class=bottom_surface
surface_objects[1].group[0] id=41 facets=[0] z_min=0.0 z_max=1.0 area=2.0 printable=true shell_count=3
surface_objects[1].bridge[0] id=42 facets=[3] direction=27.0 anchor_width=0.5 length=2.0 margin=0.2 valid=true
surface_objects[1].bridge[0].footprint count=1 geometry=[{contour=[(90000,0),(110000,0),(110000,20000),(90000,20000)],holes=[]}]
surface_objects[1].overhang[0] id=43 facets=[2] angle=60.0 needs_support=true
surface_objects[1].overhang[0].footprint count=1 geometry=[{contour=[(60000,0),(80000,0),(80000,20000),(60000,20000)],holes=[]}]
"#;
    // Independent literals for every other standard-scenario family. Together
    // with `expected`, this covers the entire piece, not just successful reads.
    // Geometry, including paint-stroke vertices, uses world coordinates: the
    // translated triangle adds (10,20,3) once. Empty bounds yield a typed error.
    let expected_complement = r#"mode=standard
invocation_config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
invocation_config line_width=Float(0.8)
invocation_config private_probe=absent
invocation_config "whole_print_batch_ops"=String("")
invocation_config "whole_print_fixture_mode"=String("standard")
read layer_count 
layer_count=2
read layers offset=0 max_items=18446744073709551615
layers count=2
layers[0] index=0 z=0.1 nonplanar=false sync=false raft=true active_regions=0
layers[1] index=7 z=0.3 nonplanar=false sync=false raft=false active_regions=1
layers[1].active[0] object="variant-cube" region=7 height=0.2 shell=none catchup=false catchup_z_bottom=0.0 tool=0
layers[1].active[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
layers[1].active[0].config line_width=Float(0.41)
layers[1].active[0].config private_probe=absent
layers[1].active[0].config "whole_print_batch_ops"=String("")
layers[1].active[0].config "whole_print_fixture_mode"=String("standard")
read regions layer=0 offset=0 max_items=18446744073709551615
regions count=1
regions[0] key=[0|"raft-context"|0|[]] z=0.1 nonplanar_surface=None height=0.0 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[0].polygons count=0 geometry=[]
regions[0].infill_areas count=0 geometry=[]
regions[0].top_solid_fill count=0 geometry=[]
regions[0].bottom_solid_fill count=0 geometry=[]
regions[0].bridge_areas count=0 geometry=[]
regions[0].sparse_infill_area count=0 geometry=[]
regions[0].internal_solid_fill count=0 geometry=[]
regions[0].internal_bridge_areas count=0 geometry=[]
regions[0].raft_fill count=1 geometry=[{contour=[(-10000,-10000),(130000,-10000),(130000,40000),(-10000,40000)],holes=[]}]
regions[0].annotations count=0
read region_configs layer=0 offset=0 max_items=18446744073709551615
region_configs count=1
region_configs[0] key=[0|"raft-context"|0|[]]
region_configs[0] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[0] line_width=Float(0.73)
region_configs[0] private_probe=absent
region_configs[0] "whole_print_batch_ops"=String("")
region_configs[0] "whole_print_fixture_mode"=String("standard")
read selected_targets layer=0 offset=0 max_items=18446744073709551615
selected_targets count=1
selected_targets[0] layer=0 stage="Layer::PathOptimization" invoke=true source=model(key=[0|"raft-context"|0|[]],members=[[0|"raft-context"|0|[]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.73)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
read surface_layers layer=0 offset=0 max_items=18446744073709551615
surface_layers present
surface_layers count=0
read seams layer=0 offset=0 max_items=18446744073709551615
seams present
seams count=0
read region_configs layer=7 offset=0 max_items=18446744073709551615
region_configs count=4
region_configs[0] key=[7|"context-only"|9|[]]
region_configs[0] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[0] line_width=Float(0.73)
region_configs[0] private_probe=absent
region_configs[0] "whole_print_batch_ops"=String("")
region_configs[0] "whole_print_fixture_mode"=String("standard")
region_configs[1] key=[7|"variant-cube"|0|["material"=tool_index(1)]]
region_configs[1] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[1] line_width=Float(0.41)
region_configs[1] private_probe=absent
region_configs[1] "whole_print_batch_ops"=String("")
region_configs[1] "whole_print_fixture_mode"=String("standard")
region_configs[2] key=[7|"variant-cube"|0|["material"=tool_index(2)]]
region_configs[2] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[2] line_width=Float(0.62)
region_configs[2] private_probe=absent
region_configs[2] "whole_print_batch_ops"=String("")
region_configs[2] "whole_print_fixture_mode"=String("standard")
region_configs[3] key=[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]]
region_configs[3] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[3] line_width=Float(0.41)
region_configs[3] private_probe=absent
region_configs[3] "whole_print_batch_ops"=String("")
region_configs[3] "whole_print_fixture_mode"=String("standard")
read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=4
selected_targets[0] layer=7 stage="Layer::PathOptimization" invoke=true source=model(key=[7|"context-only"|9|[]],members=[[7|"context-only"|9|[]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.73)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
selected_targets[1] layer=7 stage="Layer::PathOptimization" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(1)]],members=[[7|"variant-cube"|7|["material"=tool_index(1)]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[1].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[1].config line_width=Float(0.41)
selected_targets[1].config private_probe=absent
selected_targets[1].config "whole_print_batch_ops"=String("")
selected_targets[1].config "whole_print_fixture_mode"=String("standard")
selected_targets[2] layer=7 stage="Layer::PathOptimization" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(2)]],members=[[7|"variant-cube"|7|["material"=tool_index(2)]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[2].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[2].config line_width=Float(0.62)
selected_targets[2].config private_probe=absent
selected_targets[2].config "whole_print_batch_ops"=String("")
selected_targets[2].config "whole_print_fixture_mode"=String("standard")
selected_targets[3] layer=7 stage="Layer::PathOptimization" invoke=true source=model(key=[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]],members=[[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[3].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[3].config line_width=Float(0.41)
selected_targets[3].config private_probe=absent
selected_targets[3].config "whole_print_batch_ops"=String("")
selected_targets[3].config "whole_print_fixture_mode"=String("standard")
read seams layer=7 offset=0 max_items=18446744073709551615
seams present
seams count=1
seams[0] key=[7|"variant-cube"|7|["material"=tool_index(1)]] candidates=1
seams[0].chosen point=(1.0,2.0,0.3) width=0.4 flow=0.9 quartile=Some(2) dist_to_top=0.7 overhang_distance=Some(-0.2) wall_index=1
seams[0].candidate[0] point=(1.5,2.5,0.3) width=0.4 flow=0.0 quartile=None dist_to_top=0.0 overhang_distance=None score=0.25 reason=user_forced
read mesh_objects offset=0 max_items=18446744073709551615
mesh_objects count=4
mesh_objects[0] object="context-only" vertices=3 triangles=1 paint_layers=3
mesh_objects[0].bounds min=(0.0,0.0,1.0) max=(2.0,2.0,1.0)
mesh_objects[0].paint[0] semantic="material" facets=1 values=[tool_index(1)] strokes=1
mesh_objects[0].paint[0].stroke[0] semantic="material" value=tool_index(1) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
mesh_objects[0].paint[1] semantic="fixture_flag" facets=1 values=[flag(true)] strokes=1
mesh_objects[0].paint[1].stroke[0] semantic="fixture_flag" value=flag(true) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
mesh_objects[0].paint[2] semantic="fixture_scalar" facets=1 values=[scalar(0.25;bits=3e800000)] strokes=1
mesh_objects[0].paint[2].stroke[0] semantic="fixture_scalar" value=scalar(0.25;bits=3e800000) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
mesh_objects[1] object="empty-mesh" vertices=0 triangles=0 paint_layers=0
mesh_objects[1].bounds absent
mesh_objects[2] object="translated" vertices=3 triangles=1 paint_layers=3
mesh_objects[2].bounds min=(10.0,20.0,4.0) max=(12.0,22.0,4.0)
mesh_objects[2].paint[0] semantic="material" facets=1 values=[tool_index(1)] strokes=1
mesh_objects[2].paint[0].stroke[0] semantic="material" value=tool_index(1) vertices=3 triangles_flat=[(10.0,20.0,4.0),(12.0,20.0,4.0),(10.0,22.0,4.0)]
mesh_objects[2].paint[1] semantic="fixture_flag" facets=1 values=[flag(true)] strokes=1
mesh_objects[2].paint[1].stroke[0] semantic="fixture_flag" value=flag(true) vertices=3 triangles_flat=[(10.0,20.0,4.0),(12.0,20.0,4.0),(10.0,22.0,4.0)]
mesh_objects[2].paint[2] semantic="fixture_scalar" facets=1 values=[scalar(0.25;bits=3e800000)] strokes=1
mesh_objects[2].paint[2].stroke[0] semantic="fixture_scalar" value=scalar(0.25;bits=3e800000) vertices=3 triangles_flat=[(10.0,20.0,4.0),(12.0,20.0,4.0),(10.0,22.0,4.0)]
mesh_objects[3] object="variant-cube" vertices=3 triangles=1 paint_layers=3
mesh_objects[3].bounds min=(0.0,0.0,1.0) max=(2.0,2.0,1.0)
mesh_objects[3].paint[0] semantic="material" facets=1 values=[tool_index(1)] strokes=1
mesh_objects[3].paint[0].stroke[0] semantic="material" value=tool_index(1) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
mesh_objects[3].paint[1] semantic="fixture_flag" facets=1 values=[flag(true)] strokes=1
mesh_objects[3].paint[1].stroke[0] semantic="fixture_flag" value=flag(true) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
mesh_objects[3].paint[2] semantic="fixture_scalar" facets=1 values=[scalar(0.25;bits=3e800000)] strokes=1
mesh_objects[3].paint[2].stroke[0] semantic="fixture_scalar" value=scalar(0.25;bits=3e800000) vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
read mesh_vertices object="context-only" offset=0 max_items=18446744073709551615
mesh_vertices count=3
mesh_vertices[0] (0.0,0.0,1.0)
mesh_vertices[1] (2.0,0.0,1.0)
mesh_vertices[2] (0.0,2.0,1.0)
read mesh_triangles object="context-only" offset=0 max_items=18446744073709551615
mesh_triangles count=1
mesh_triangles[0] (0,1,2)
read mesh_queries raycast object="context-only" x=0.25 y=0.25 start_z=2.0
mesh_queries count=1
mesh_queries[0] raycast_z_down=Some(1.0)
read mesh_queries normal object="context-only" x=0.25 y=0.25 z=1.0
mesh_queries count=1
mesh_queries[0] surface_normal_at=(0.0,0.0,1.0)
read mesh_queries bounds object="context-only"
mesh_queries count=1
mesh_queries[0] object_bounds min=(0.0,0.0,1.0) max=(2.0,2.0,1.0)
read mesh_vertices object="empty-mesh" offset=0 max_items=18446744073709551615
mesh_vertices count=0
read mesh_triangles object="empty-mesh" offset=0 max_items=18446744073709551615
mesh_triangles count=0
read mesh_queries raycast object="empty-mesh" x=0.25 y=0.25 start_z=2.0
mesh_queries count=1
mesh_queries[0] raycast_z_down=None
read mesh_queries normal object="empty-mesh" x=0.25 y=0.25 z=1.0
mesh_queries count=1
mesh_queries[0] surface_normal_at=none
read mesh_queries bounds object="empty-mesh"
err mesh_queries invalid-input: mesh_queries.object_bounds empty mesh "empty-mesh"
error_context mesh_queries bounds object="empty-mesh"
read mesh_vertices object="translated" offset=0 max_items=18446744073709551615
mesh_vertices count=3
mesh_vertices[0] (10.0,20.0,4.0)
mesh_vertices[1] (12.0,20.0,4.0)
mesh_vertices[2] (10.0,22.0,4.0)
read mesh_triangles object="translated" offset=0 max_items=18446744073709551615
mesh_triangles count=1
mesh_triangles[0] (0,1,2)
read mesh_queries raycast object="translated" x=0.25 y=0.25 start_z=2.0
mesh_queries count=1
mesh_queries[0] raycast_z_down=None
read mesh_queries normal object="translated" x=0.25 y=0.25 z=1.0
mesh_queries count=1
mesh_queries[0] surface_normal_at=none
read mesh_queries bounds object="translated"
mesh_queries count=1
mesh_queries[0] object_bounds min=(10.0,20.0,4.0) max=(12.0,22.0,4.0)
read mesh_vertices object="variant-cube" offset=0 max_items=18446744073709551615
mesh_vertices count=3
mesh_vertices[0] (0.0,0.0,1.0)
mesh_vertices[1] (2.0,0.0,1.0)
mesh_vertices[2] (0.0,2.0,1.0)
read mesh_triangles object="variant-cube" offset=0 max_items=18446744073709551615
mesh_triangles count=1
mesh_triangles[0] (0,1,2)
read mesh_queries raycast object="variant-cube" x=0.25 y=0.25 start_z=2.0
mesh_queries count=1
mesh_queries[0] raycast_z_down=Some(1.0)
read mesh_queries normal object="variant-cube" x=0.25 y=0.25 z=1.0
mesh_queries count=1
mesh_queries[0] surface_normal_at=(0.0,0.0,1.0)
read mesh_queries bounds object="variant-cube"
mesh_queries count=1
mesh_queries[0] object_bounds min=(0.0,0.0,1.0) max=(2.0,2.0,1.0)
read support_analysis offset=0 max_items=18446744073709551615
support_analysis present
support_analysis candidates=1 occupancy=2 cantilever=2 termination=2 territory=2 families=1 settings=[("fixture_setting", "17")]
support_analysis.envelope count=1 geometry=[{contour=[(-20000,-20000),(140000,-20000),(140000,50000),(-20000,50000)],holes=[]}]
support_analysis.candidate[0] id=44 source=[7|"support-carrier"|5|["material"=tool_index(1)]] z_units=3000 enforced=true blocked=false
support_analysis.candidate[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_analysis.occupancy count=2
support_analysis.occupancy[0] key=[7|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.occupancy[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_analysis.occupancy[1] key=[8|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.occupancy[1] count=1 geometry=[{contour=[(50000,60000),(70000,60000),(70000,80000),(50000,80000)],holes=[]}]
support_analysis.cantilever count=2
support_analysis.cantilever[0] key=[7|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.cantilever[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_analysis.cantilever[1] key=[8|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.cantilever[1] count=1 geometry=[{contour=[(50000,60000),(70000,60000),(70000,80000),(50000,80000)],holes=[]}]
support_analysis.termination count=2
support_analysis.termination[0] key=[7|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.termination[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_analysis.termination[1] key=[8|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.termination[1] count=1 geometry=[{contour=[(50000,60000),(70000,60000),(70000,80000),(50000,80000)],holes=[]}]
support_analysis.territory count=2
support_analysis.territory[0] key=[7|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.territory[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_analysis.territory[1] key=[8|"support-carrier"|5|["material"=tool_index(1)]]
support_analysis.territory[1] count=1 geometry=[{contour=[(50000,60000),(70000,60000),(70000,80000),(50000,80000)],holes=[]}]
support_analysis.family[0] object="support-carrier" region=5 chain=["material"=tool_index(1)] family="traditional"
read support_geometry offset=0 max_items=18446744073709551615
support_geometry present
support_geometry height=0.2 top_z_distance=0.1
support_geometry.entries count=2
support_geometry.entries[0] key=[7|"support-carrier"|5|["material"=tool_index(1)]]
support_geometry.entries[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_geometry.entries[1] key=[8|"support-carrier"|5|["material"=tool_index(1)]]
support_geometry.entries[1] count=1 geometry=[{contour=[(50000,60000),(70000,60000),(70000,80000),(50000,80000)],holes=[]}]
read support_plan offset=0 max_items=18446744073709551615
support_plan present
support_plan entries=5
support_plan.raft layers=1 density=0.8 base=1 interface=0
support_plan.entry[0] layer=-1 object="support-carrier" region=5 variant_chain=["material"=tool_index(5)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["off-grid"] decline=none
support_plan.entry[0].role[0] role=support_body
support_plan.entry[0].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[0].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[1] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(1)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["p0"] decline=none
support_plan.entry[1].role[0] role=support_body
support_plan.entry[1].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[1].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[2] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(3)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["declined"] decline=declined_policy
support_plan.entry[2].role[0] role=support_body
support_plan.entry[2].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[2].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[3] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(4)] family="tree" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["wrong-family"] decline=none
support_plan.entry[3].role[0] role=support_body
support_plan.entry[3].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[3].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[4] layer=8 object="support-carrier" region=5 variant_chain=["material"=tool_index(2)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["wrong-global"] decline=none
support_plan.entry[4].role[0] role=support_body
support_plan.entry[4].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[4].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
read_errors=1
"#;
    (expected, expected_complement)
}

#[test]
fn selected_model_literals() {
    // Widths and invocation config deliberately differ: selected model targets
    // carry source configs, not the invocation's 0.8 mm line width.
    let scenario = Scenario {
        module_widths: [0.4, 0.6, 0.73],
        ..Scenario::default()
    };
    // Config records preserve RegionMap keys, not slice source keys. The shared
    // selection address rule decodes painted source region 7 to map base 0;
    // selected source identities below must nevertheless retain region 7.
    let expected_configs = r#"read region_configs layer=7 offset=0 max_items=18446744073709551615
region_configs count=4
region_configs[0] key=[7|"context-only"|9|[]]
region_configs[0] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[0] line_width=Float(0.73)
region_configs[0] private_probe=absent
region_configs[0] "whole_print_batch_ops"=String("")
region_configs[0] "whole_print_fixture_mode"=String("standard")
region_configs[1] key=[7|"variant-cube"|0|["material"=tool_index(1)]]
region_configs[1] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[1] line_width=Float(0.4)
region_configs[1] private_probe=absent
region_configs[1] "whole_print_batch_ops"=String("")
region_configs[1] "whole_print_fixture_mode"=String("standard")
region_configs[2] key=[7|"variant-cube"|0|["material"=tool_index(2)]]
region_configs[2] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[2] line_width=Float(0.6)
region_configs[2] private_probe=absent
region_configs[2] "whole_print_batch_ops"=String("")
region_configs[2] "whole_print_fixture_mode"=String("standard")
region_configs[3] key=[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]]
region_configs[3] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[3] line_width=Float(0.4)
region_configs[3] private_probe=absent
region_configs[3] "whole_print_batch_ops"=String("")
region_configs[3] "whole_print_fixture_mode"=String("standard")
"#;
    let expected_perimeters = r#"read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=2
selected_targets[0] layer=7 stage="Layer::Perimeters" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(1)]],members=[[7|"variant-cube"|7|["material"=tool_index(1)]],[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.4)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
selected_targets[1] layer=7 stage="Layer::Perimeters" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(2)]],members=[[7|"variant-cube"|7|["material"=tool_index(2)]]]) claims=[] authored_coloring=false resolved_role_tool=None
selected_targets[1].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[1].config line_width=Float(0.6)
selected_targets[1].config private_probe=absent
selected_targets[1].config "whole_print_batch_ops"=String("")
selected_targets[1].config "whole_print_fixture_mode"=String("standard")
"#;
    let expected_infill = r#"read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=3
selected_targets[0] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(1)]],members=[[7|"variant-cube"|7|["material"=tool_index(1)]]]) claims=["claim:top-fill", "claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.4)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
selected_targets[1] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"variant-cube"|7|["material"=tool_index(2)]],members=[[7|"variant-cube"|7|["material"=tool_index(2)]]]) claims=["claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[1].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[1].config line_width=Float(0.6)
selected_targets[1].config private_probe=absent
selected_targets[1].config "whole_print_batch_ops"=String("")
selected_targets[1].config "whole_print_fixture_mode"=String("standard")
selected_targets[2] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]],members=[[7|"variant-cube"|9223372036862760330|["material"=tool_index(1)]]]) claims=["claim:top-fill", "claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[2].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[2].config line_width=Float(0.4)
selected_targets[2].config private_probe=absent
selected_targets[2].config "whole_print_batch_ops"=String("")
selected_targets[2].config "whole_print_fixture_mode"=String("standard")
"#;
    let expected_invocation = r#"invocation_config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
invocation_config line_width=Float(0.8)
invocation_config private_probe=absent
invocation_config "whole_print_batch_ops"=String("")
invocation_config "whole_print_fixture_mode"=String("standard")
"#;
    // The tree-family context is excluded by the real selection authority,
    // not removed from committed reads. Pin its complete region record too.
    let expected_context = r#"read regions layer=7 offset=0 max_items=18446744073709551615
regions count=4
regions[0] key=[7|"context-only"|9|[]] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[0].polygons count=1 geometry=[{contour=[(200000,0),(220000,0),(220000,20000),(200000,20000)],holes=[]}]
regions[0].infill_areas count=0 geometry=[]
regions[0].top_solid_fill count=0 geometry=[]
regions[0].bottom_solid_fill count=0 geometry=[]
regions[0].bridge_areas count=0 geometry=[]
regions[0].sparse_infill_area count=0 geometry=[]
regions[0].internal_solid_fill count=0 geometry=[]
regions[0].internal_bridge_areas count=0 geometry=[]
regions[0].raft_fill count=0 geometry=[]
regions[0].annotations count=0
"#;

    // Controls explicitly grant schedule discovery to these restricted packages.
    // No perimeter product is supplied, read, or predicted by either scenario.
    // resolve_held_claims preserves declared order while retaining fill tokens
    // only: top then sparse for variant 1, sparse alone for variant 2. Perimeter
    // outer/inner roles and support-family eligibility are not held fill roles.
    for (package, claims, expected_targets) in [
        (
            Package::Perimeters,
            &["outer", "inner", "support-family:traditional"][..],
            expected_perimeters,
        ),
        (
            Package::Infill,
            &[
                "claim:top-fill",
                "claim:sparse-fill",
                "support-family:traditional",
            ][..],
            expected_infill,
        ),
    ] {
        let scenario = scenario.clone().with_control_discovery(package);
        let seed = literal_inputs(package, &scenario);
        assert_eq!(seed.slices.len(), 2);
        assert_eq!(seed.slices[1].regions.len(), 4);
        assert_eq!(seed.region_map.entries.len(), 5);
        let pair = run_pair(package, claims, &scenario)
            .expect("freeze and invoke both real model-selection preparation legs");
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            let observed = observations(leg, name);
            let invocation = observed
                .lines()
                .filter(|line| line.starts_with("invocation_config "))
                .collect::<Vec<_>>();
            assert_eq!(invocation.len(), 5, "{package:?}/{name}: invocation config");
            assert_eq!(
                format!("{}\n", invocation.join("\n")),
                expected_invocation,
                "{package:?}/{name}: declared invocation keys, not source config"
            );
            assert_eq!(
                read_sections(observed, &["read region_configs layer=7 "]),
                expected_configs,
                "{package:?}/{name}: whole-print config context includes unselected geometry"
            );
            assert!(
                read_sections(observed, &["read regions layer=7 "]).starts_with(expected_context),
                "{package:?}/{name}: unselected context region remains fully readable"
            );
            assert_eq!(
                read_sections(observed, &["read selected_targets layer=7 "]),
                expected_targets,
                "{package:?}/{name}: selected source identities, members and complete claims"
            );
        }
    }
}

#[test]
fn non_region_source_literals() {
    // Input order is deliberately different from presentation order. Entry 1's
    // anchor is on the requested layer, but its GLOBAL layer is not. This catches
    // anchor-based selection and presentation-renumbered carrier references.
    let (_, _, mut plan) = literal_support(InsertionOrder::Canonical);
    assert_eq!(plan.entries.len(), 5);
    plan.entries[1].global_layer_index = 3;
    plan.entries[1].anchor_layer_index = 7;
    let scenario = Scenario {
        support_plan: Product::Value(Arc::new(plan)),
        ..Scenario::default()
    };
    let expected_support = r#"read selected_targets layer=0 offset=0 max_items=18446744073709551615
selected_targets count=1
selected_targets[0] layer=0 stage="Layer::Support" invoke=true source=raft(layer=0) claims=["support", "support-family:traditional"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.8)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=1
selected_targets[0] layer=7 stage="Layer::Support" invoke=true source=support_carrier(entry_index=0,object="support-carrier",region=5,chain=["material"=tool_index(1)],family="traditional",demand=["d0"],body=["b0"],provenance=["p0"],anchor_layer=3) claims=["support", "support-family:traditional"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.8)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
"#;
    let expected_anchored = r#"read selected_targets layer=0 offset=0 max_items=18446744073709551615
selected_targets count=1
selected_targets[0] layer=0 stage="Layer::AnchoredEvents" invoke=true source=anchored_stage(anchor_layer=0) claims=["anchored", "support-family:traditional"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.8)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=1
selected_targets[0] layer=7 stage="Layer::AnchoredEvents" invoke=true source=anchored_stage(anchor_layer=7) claims=["anchored", "support-family:traditional"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.8)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
"#;
    let expected_plan = r#"read support_plan offset=0 max_items=18446744073709551615
support_plan present
support_plan entries=5
support_plan.raft layers=1 density=0.8 base=1 interface=0
support_plan.entry[0] layer=-1 object="support-carrier" region=5 variant_chain=["material"=tool_index(5)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["off-grid"] decline=none
support_plan.entry[0].role[0] role=support_body
support_plan.entry[0].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[0].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[1] layer=3 object="support-carrier" region=5 variant_chain=["material"=tool_index(2)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=7 anchor_z=3000 roles=1 capabilities=[] provenance=["wrong-global"] decline=none
support_plan.entry[1].role[0] role=support_body
support_plan.entry[1].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[1].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[2] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(1)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["p0"] decline=none
support_plan.entry[2].role[0] role=support_body
support_plan.entry[2].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[2].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[3] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(3)] family="traditional" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["declined"] decline=declined_policy
support_plan.entry[3].role[0] role=support_body
support_plan.entry[3].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[3].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
support_plan.entry[4] layer=7 object="support-carrier" region=5 variant_chain=["material"=tool_index(4)] family="tree" demand=["d0"] body=["b0"] anchor_layer=3 anchor_z=3000 roles=1 capabilities=[] provenance=["wrong-family"] decline=none
support_plan.entry[4].role[0] role=support_body
support_plan.entry[4].role[0] count=1 geometry=[{contour=[(10000,20000),(30000,20000),(30000,40000),(10000,40000)],holes=[]}]
support_plan.entry[4].skeleton points=1 values=[(1.0,2.0,0.3)] wall_counts=[2]
"#;
    for (package, claims, expected) in [
        (
            Package::Support,
            &["support", "support-family:traditional"][..],
            expected_support,
        ),
        (
            Package::Anchored,
            &["anchored", "support-family:traditional"][..],
            expected_anchored,
        ),
    ] {
        let scenario = scenario
            .clone()
            .with_optional_reads(package)
            .with_control_discovery(package);
        let seed = literal_inputs(package, &scenario);
        let plan = seed.support_plan.as_ref().expect("carrier context exists");
        assert_eq!(plan.entries.len(), 5);
        assert_eq!(plan.entries[0].global_layer_index, 7);
        assert_eq!(plan.entries[0].anchor_layer_index, 3);
        assert_eq!(plan.entries[1].global_layer_index, 3);
        assert_eq!(plan.entries[1].anchor_layer_index, 7);
        assert_eq!(plan.entries[0].roles[0].regions.len(), 1);
        assert_eq!(plan.entries[0].skeleton.as_ref().unwrap().points.len(), 1);
        let pair =
            run_pair(package, claims, &scenario).expect("actual support/anchored frozen adapters");
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            let observed = observations(leg, name);
            assert_eq!(leg.marker_count("wpi read selected_targets"), 2, "{name}");
            assert_literal_lines(
                &read_sections(observed, &["read selected_targets "]),
                expected,
                &format!("{package:?}/{name}: non-model sources and invocation config"),
            );
            assert_literal_lines(
                &read_sections(observed, &["read support_plan "]),
                expected_plan,
                &format!("{package:?}/{name}: unselected carrier context"),
            );
        }
    }
}

#[test]
fn mesh_seam_support_literals() {
    let scenario = Scenario {
        batch_ops: "raycast:translated:10.25:20.25:5;normal:translated:10.25:20.25:4".into(),
        ..Scenario::default()
    };
    let seed = literal_inputs(Package::Path, &scenario);
    assert_eq!(seed.mesh.objects.len(), 4);
    assert_eq!(seed.mesh.objects[0].mesh.vertices.len(), 3);
    assert_eq!(seed.mesh.objects[0].mesh.indices.len(), 3);
    assert_eq!(
        seed.mesh.objects[0]
            .paint_data
            .as_ref()
            .unwrap()
            .layers
            .len(),
        3
    );
    assert_eq!(seed.seam.as_ref().unwrap().entries.len(), 1);
    assert_eq!(seed.support_analysis.as_ref().unwrap().candidates.len(), 1);
    assert_eq!(seed.support_geometry.as_ref().unwrap().entries.len(), 2);
    assert_eq!(seed.support_plan.as_ref().unwrap().entries.len(), 5);
    // docs/08's column-major translation adds (10,20,3) once. The ordinary
    // triangle, every paint vertex, and the typed queries share world space.
    let headers = [
        "read seams ",
        "read mesh_objects ",
        "read mesh_vertices ",
        "read mesh_triangles ",
        "read mesh_queries ",
        "read support_analysis ",
        "read support_geometry ",
        "read support_plan ",
    ];
    let expected = read_sections(standard_literals().1, &headers);
    let expected_queries = r#"batch[0] op="raycast:translated:10.25:20.25:5"
read mesh_queries raycast object="translated" x=10.25 y=20.25 start_z=5.0
mesh_queries count=1
mesh_queries[0] raycast_z_down=Some(4.0)
batch[1] op="normal:translated:10.25:20.25:4"
read mesh_queries normal object="translated" x=10.25 y=20.25 z=4.0
mesh_queries count=1
mesh_queries[0] surface_normal_at=(0.0,0.0,1.0)
read_errors=1
"#;
    let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
    for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
        let observed = observations(leg, name);
        let (standard, queries) = observed
            .split_once("batch[0] ")
            .expect("world-space query batches");
        assert_literal_lines(&read_sections(standard, &headers), &expected, name);
        assert_literal_lines(&format!("batch[0] {queries}"), expected_queries, name);
        assert_eq!(leg.marker_count("wpi read mesh_queries"), 14, "{name}");
    }

    let optional_headers = [
        "read surface_layers ",
        "read seams ",
        "read surface_objects ",
        "read support_analysis ",
        "read support_geometry ",
        "read support_plan ",
    ];
    let absent = r#"read surface_layers layer=0 offset=0 max_items=18446744073709551615
surface_layers absent
read seams layer=0 offset=0 max_items=18446744073709551615
seams absent
read surface_layers layer=7 offset=0 max_items=18446744073709551615
surface_layers absent
read seams layer=7 offset=0 max_items=18446744073709551615
seams absent
read surface_objects offset=0 max_items=18446744073709551615
surface_objects absent
read support_analysis offset=0 max_items=18446744073709551615
support_analysis absent
read support_geometry offset=0 max_items=18446744073709551615
support_geometry absent
read support_plan offset=0 max_items=18446744073709551615
support_plan absent
"#;
    let empty = r#"read surface_layers layer=0 offset=0 max_items=18446744073709551615
surface_layers present
surface_layers count=0
read seams layer=0 offset=0 max_items=18446744073709551615
seams present
seams count=0
read surface_layers layer=7 offset=0 max_items=18446744073709551615
surface_layers present
surface_layers count=0
read seams layer=7 offset=0 max_items=18446744073709551615
seams present
seams count=0
read surface_objects offset=0 max_items=18446744073709551615
surface_objects present
surface_objects count=0
read support_analysis offset=0 max_items=18446744073709551615
support_analysis present
support_analysis candidates=0 occupancy=0 cantilever=0 termination=0 territory=0 families=0 settings=[]
support_analysis.envelope count=0 geometry=[]
support_analysis.occupancy count=0
support_analysis.cantilever count=0
support_analysis.termination count=0
support_analysis.territory count=0
read support_geometry offset=0 max_items=18446744073709551615
support_geometry present
support_geometry height=0.0 top_z_distance=0.0
support_geometry.entries count=0
read support_plan offset=0 max_items=18446744073709551615
support_plan present
support_plan entries=0
support_plan.raft absent
"#;
    for (present, expected) in [(false, absent), (true, empty)] {
        let scenario = if present {
            Scenario {
                surface: Product::Empty,
                seam: Product::Empty,
                support_analysis: Product::Empty,
                support_geometry: Product::Empty,
                support_plan: Product::Empty,
                ..Scenario::default()
            }
        } else {
            Scenario {
                surface: Product::Absent,
                seam: Product::Absent,
                support_analysis: Product::Absent,
                support_geometry: Product::Absent,
                support_plan: Product::Absent,
                ..Scenario::default()
            }
        }
        .with_optional_reads(Package::Path);
        let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            let observed = observations(leg, name);
            assert_literal_lines(
                &read_sections(observed, &optional_headers),
                expected,
                &format!("{name}: present={present}; optional availability is not permission"),
            );
            assert_eq!(observed.lines().last(), Some("read_errors=1"), "{name}");
        }
    }
}

/// Format explicitly chosen independent literal records. No offset/count logic
/// lives here: the cases below spell out which records each request must return.
fn chosen_literal_rows(literals: &str, path: &str, chosen: &[usize]) -> String {
    let mut result = String::new();
    for (returned, original) in chosen.iter().enumerate() {
        let prefix = format!("{path}[{original}]");
        let rows: Vec<_> = literals
            .lines()
            .filter(|line| line.starts_with(&prefix))
            .collect();
        assert!(
            !rows.is_empty(),
            "missing independently authored literal {prefix}"
        );
        for row in rows {
            result.push_str(&row.replacen(&prefix, &format!("{path}[{returned}]"), 1));
            result.push('\n');
        }
    }
    result
}

fn bounded_literal_body(literals: &str, method: &str, chosen: &[usize]) -> String {
    match method {
        "support_analysis" => {
            // Candidates/families have one row; each keyed geometry map has two.
            // Thus request (1,1) must NOT zip them all down to an empty batch.
            let short: &[usize] = if chosen.contains(&0) { &[0] } else { &[] };
            let mut body = format!(
                "support_analysis present\nsupport_analysis candidates={} occupancy={} cantilever={} termination={} territory={} families={} settings=[(\"fixture_setting\", \"17\")]\nsupport_analysis.envelope count=1 geometry=[{{contour=[(-20000,-20000),(140000,-20000),(140000,50000),(-20000,50000)],holes=[]}}]\n",
                short.len(), chosen.len(), chosen.len(), chosen.len(), chosen.len(), short.len(),
            );
            body.push_str(&chosen_literal_rows(literals, "support_analysis.candidate", short));
            for path in ["occupancy", "cantilever", "termination", "territory"] {
                let path = format!("support_analysis.{path}");
                body.push_str(&format!("{path} count={}\n", chosen.len()));
                body.push_str(&chosen_literal_rows(literals, &path, chosen));
            }
            body.push_str(&chosen_literal_rows(literals, "support_analysis.family", short));
            body
        }
        "support_geometry" => format!(
            "support_geometry present\nsupport_geometry height=0.2 top_z_distance=0.1\nsupport_geometry.entries count={}\n{}",
            chosen.len(), chosen_literal_rows(literals, "support_geometry.entries", chosen),
        ),
        "support_plan" => format!(
            "support_plan present\nsupport_plan entries={}\nsupport_plan.raft layers=1 density=0.8 base=1 interface=0\n{}",
            chosen.len(), chosen_literal_rows(literals, "support_plan.entry", chosen),
        ),
        _ => {
            let present = if matches!(method, "surface_objects" | "surface_layers" | "seams") {
                format!("{method} present\n")
            } else { String::new() };
            format!("{present}{method} count={}\n{}", chosen.len(), chosen_literal_rows(literals, method, chosen))
        }
    }
}

#[test]
fn canonical_bounded_batches() {
    // docs/01 full RegionKey order, plus design's object/signed-carrier ordering.
    // Every index set is authored independently, not calculated from a returned
    // full batch (which would let both full and bounded projections be wrong).
    let families: [(&str, &str, &str, u64, &[usize], &[usize]); 13] = [
        ("layers", "", "", 2, &[0, 1], &[1]),
        ("regions", "7:", "layer=7 ", 4, &[0, 1, 2, 3], &[1]),
        ("region_configs", "7:", "layer=7 ", 4, &[0, 1, 2, 3], &[1]),
        ("selected_targets", "7:", "layer=7 ", 4, &[0, 1, 2, 3], &[1]),
        ("surface_objects", "", "", 2, &[0, 1], &[1]),
        ("surface_layers", "7:", "layer=7 ", 2, &[0, 1], &[1]),
        ("seams", "7:", "layer=7 ", 1, &[0], &[]),
        ("mesh_objects", "", "", 4, &[0, 1, 2, 3], &[1]),
        (
            "mesh_vertices",
            "variant-cube:",
            "object=\"variant-cube\" ",
            3,
            &[0, 1, 2],
            &[1],
        ),
        (
            "mesh_triangles",
            "variant-cube:",
            "object=\"variant-cube\" ",
            1,
            &[0],
            &[],
        ),
        ("support_analysis", "", "", 2, &[0, 1], &[1]),
        ("support_geometry", "", "", 2, &[0, 1], &[1]),
        ("support_plan", "", "", 5, &[0, 1, 2, 3, 4], &[1]),
    ];
    let (typed, context) = standard_literals();
    let context_reads = &context[context.find("read layer_count ").unwrap()..];
    let independent = format!("{typed}{context_reads}");
    let mut requests = Vec::new();
    let mut expected = String::new();
    for (method, argument, scope, len, all, second) in &families {
        let section = read_sections(&independent, &[&format!("read {method} {scope}offset=")]);
        for (offset, count, chosen) in [
            (0, 1, &[0][..]),
            (1, 1, *second),
            (*len, 0, &[][..]),
            (*len, u64::MAX, &[][..]),
            (0, u64::MAX, *all),
        ] {
            let operation = format!("{method}:{argument}{offset}:{count}");
            expected.push_str(&format!(
                "batch[{}] op={operation:?}\nread {method} {scope}offset={offset} max_items={count}\n{}",
                requests.len(), bounded_literal_body(&section, method, chosen),
            ));
            requests.push(operation);
        }
    }
    assert_eq!(requests.len(), 65);
    expected.push_str("read_errors=1\n");
    let batch_ops = requests.join(";");
    // Config controls are inputs too; use their supplied literal value, not any
    // converted config. The geometry/identity oracle above remains unchanged.
    let control = format!("\"whole_print_batch_ops\"=String({batch_ops:?})");
    let expected = expected.replace("\"whole_print_batch_ops\"=String(\"\")", &control);
    for order in [
        InsertionOrder::ReversedSlices,
        InsertionOrder::ReversedMap,
        InsertionOrder::ReversedObjects,
    ] {
        let scenario = Scenario {
            order,
            batch_ops: batch_ops.clone(),
            ..Scenario::default()
        };
        let seed = literal_inputs(Package::Path, &scenario);
        assert_eq!(seed.slices.len(), 2);
        assert_eq!(seed.mesh.objects.len(), 4);
        let pair = run_pair(Package::Path, &["path-optimizer"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            let observed = observations(leg, name);
            let (standard, batches) = observed
                .split_once("batch[0] ")
                .expect("bounded operation population");
            for (method, _, scope, _, _, _) in &families {
                let header = format!("read {method} {scope}offset=");
                let literal = read_sections(&independent, &[&header])
                    .replace("\"whole_print_batch_ops\"=String(\"\")", &control);
                assert_literal_lines(&read_sections(standard, &[&header]), &literal, name);
            }
            assert_literal_lines(
                &format!("batch[0] {batches}"),
                &expected,
                &format!("{order:?}/{name}: exact bounded records"),
            );
        }

        // Real owner backing, not a fixture-local store. Each actual adapter
        // consumes the SAME committed Arc checked here, then releases it after
        // preparation input/resource disposal. No per-Layer snapshot copy lives
        // in the owner or published observations plan.
        for wasm in [false, true] {
            let driver = PreparedDriver::new(Package::Path, &scenario, wasm).unwrap();
            let original_mesh_refs = Arc::strong_count(&seed.mesh);
            let committed = seed.commit().unwrap();
            assert!(seed.retains_arc_identity(&committed));
            let input = prevalidate(
                &driver.owner,
                committed.clone(),
                scenario.selection(Package::Path, &["path-optimizer"]),
            )
            .unwrap();
            assert!(input.controlled_retains_committed(&committed));
            let weak = Arc::downgrade(&committed);
            drop(committed);
            assert_eq!(weak.strong_count(), 1);
            if !wasm {
                use slicer_sdk::traits::LayerModule;
                whole_print_input_path::WholePrintInputPath::from_config(
                    &scenario.invocation(Package::Path),
                )
                .unwrap();
            }
            driver.binding.prepare(&driver.owner, input).unwrap();
            assert!(
                weak.upgrade().is_none(),
                "{order:?}/wasm={wasm}: resource retained whole-print backing"
            );
            assert_eq!(Arc::strong_count(&seed.mesh), original_mesh_refs);
            let piece = driver
                .store
                .current(&driver.owner)
                .unwrap()
                .open("observations")
                .unwrap()
                .unwrap();
            assert_eq!(piece.read(0, 5).unwrap(), b"mode=");
        }
    }
}

fn custom_native_key(region_id: u64, route: &str) -> slicer_ir::RegionKey {
    use slicer_ir::{PaintValue, RegionKey};
    // Literal native oracle: never construct this by reversing a DTO or by
    // reading the driver's input. Semantic order is part of the identity.
    RegionKey {
        global_layer_index: 7,
        object_id: "custom-cube".into(),
        region_id,
        variant_chain: vec![
            ("flag_probe".into(), PaintValue::Flag(true)),
            ("scalar_probe".into(), PaintValue::Scalar(-0.0)),
            ("material".into(), PaintValue::ToolIndex(1)),
            ("custom_route".into(), PaintValue::Custom(route.into())),
        ],
    }
}

#[test]
fn custom_paint_identity_literals() {
    use slicer_ir::{PaintSemantic, PaintValue, RegionKey};
    use slicer_sdk::preparation::{PreparationRegionKey, PreparationTargetSource};
    // Independently worked FNV footprint identity for parent 7/custom-cube and
    // the 0.25–0.75 mm square; not returned by selection or a converter.
    const MODIFIER: u64 = 9_223_372_036_862_319_096;
    let chain_a = r#"["flag_probe"=flag(true):"scalar_probe"=scalar(-0.0;bits=80000000):"material"=tool_index(1):"custom_route"=custom("A/π")]"#;
    let chain_lower = r#"["flag_probe"=flag(true):"scalar_probe"=scalar(-0.0;bits=80000000):"material"=tool_index(1):"custom_route"=custom("a/π")]"#;
    let expected = r#"read regions layer=7 offset=0 max_items=18446744073709551615
regions count=3
regions[0] key=[7|"custom-cube"|7|$A] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[0].polygons count=1 geometry=[{contour=[(0,0),(20000,0),(20000,20000),(0,20000)],holes=[]}]
regions[0].infill_areas count=0 geometry=[]
regions[0].top_solid_fill count=0 geometry=[]
regions[0].bottom_solid_fill count=0 geometry=[]
regions[0].bridge_areas count=0 geometry=[]
regions[0].sparse_infill_area count=1 geometry=[{contour=[(0,0),(20000,0),(20000,20000),(0,20000)],holes=[]}]
regions[0].internal_solid_fill count=0 geometry=[]
regions[0].internal_bridge_areas count=0 geometry=[]
regions[0].raft_fill count=0 geometry=[]
regions[0].annotations count=1
regions[0].annotations semantic=custom("annotation_probe") polygons=1
regions[0].annotations[custom("annotation_probe")][0] count=4 values=[custom("annotation/π"),none,flag(true),scalar(-0.0;bits=80000000)]
regions[1] key=[7|"custom-cube"|7|$a] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[1].polygons count=1 geometry=[{contour=[(30000,0),(50000,0),(50000,20000),(30000,20000)],holes=[]}]
regions[1].infill_areas count=0 geometry=[]
regions[1].top_solid_fill count=0 geometry=[]
regions[1].bottom_solid_fill count=0 geometry=[]
regions[1].bridge_areas count=0 geometry=[]
regions[1].sparse_infill_area count=1 geometry=[{contour=[(30000,0),(50000,0),(50000,20000),(30000,20000)],holes=[]}]
regions[1].internal_solid_fill count=0 geometry=[]
regions[1].internal_bridge_areas count=0 geometry=[]
regions[1].raft_fill count=0 geometry=[]
regions[1].annotations count=1
regions[1].annotations semantic=custom("annotation_probe") polygons=1
regions[1].annotations[custom("annotation_probe")][0] count=4 values=[custom("annotation/π"),none,flag(true),scalar(-0.0;bits=80000000)]
regions[2] key=[7|"custom-cube"|9223372036862319096|$A] z=0.3 nonplanar_surface=None height=0.2 top=None bottom=None is_bridge=false orientation=0.0 internal_bridge_angles=[] phase=committed_prepass
regions[2].polygons count=1 geometry=[{contour=[(2500,2500),(7500,2500),(7500,7500),(2500,7500)],holes=[]}]
regions[2].infill_areas count=0 geometry=[]
regions[2].top_solid_fill count=0 geometry=[]
regions[2].bottom_solid_fill count=0 geometry=[]
regions[2].bridge_areas count=0 geometry=[]
regions[2].sparse_infill_area count=1 geometry=[{contour=[(2500,2500),(7500,2500),(7500,7500),(2500,7500)],holes=[]}]
regions[2].internal_solid_fill count=0 geometry=[]
regions[2].internal_bridge_areas count=0 geometry=[]
regions[2].raft_fill count=0 geometry=[]
regions[2].annotations count=1
regions[2].annotations semantic=custom("annotation_probe") polygons=1
regions[2].annotations[custom("annotation_probe")][0] count=4 values=[custom("annotation/π"),none,flag(true),scalar(-0.0;bits=80000000)]
read region_configs layer=7 offset=0 max_items=18446744073709551615
region_configs count=3
region_configs[0] key=[7|"custom-cube"|0|$A]
region_configs[0] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[0] line_width=Float(0.4)
region_configs[0] private_probe=absent
region_configs[0] "whole_print_batch_ops"=String("")
region_configs[0] "whole_print_fixture_mode"=String("standard")
region_configs[1] key=[7|"custom-cube"|0|$a]
region_configs[1] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[1] line_width=Float(0.6)
region_configs[1] private_probe=absent
region_configs[1] "whole_print_batch_ops"=String("")
region_configs[1] "whole_print_fixture_mode"=String("standard")
region_configs[2] key=[7|"custom-cube"|9223372036862319096|$A]
region_configs[2] keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
region_configs[2] line_width=Float(0.4)
region_configs[2] private_probe=absent
region_configs[2] "whole_print_batch_ops"=String("")
region_configs[2] "whole_print_fixture_mode"=String("standard")
read selected_targets layer=7 offset=0 max_items=18446744073709551615
selected_targets count=3
selected_targets[0] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"custom-cube"|7|$A],members=[[7|"custom-cube"|7|$A]]) claims=["claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[0].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[0].config line_width=Float(0.4)
selected_targets[0].config private_probe=absent
selected_targets[0].config "whole_print_batch_ops"=String("")
selected_targets[0].config "whole_print_fixture_mode"=String("standard")
selected_targets[1] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"custom-cube"|7|$a],members=[[7|"custom-cube"|7|$a]]) claims=["claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[1].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[1].config line_width=Float(0.6)
selected_targets[1].config private_probe=absent
selected_targets[1].config "whole_print_batch_ops"=String("")
selected_targets[1].config "whole_print_fixture_mode"=String("standard")
selected_targets[2] layer=7 stage="Layer::Infill" invoke=true source=model(key=[7|"custom-cube"|9223372036862319096|$A],members=[[7|"custom-cube"|9223372036862319096|$A]]) claims=["claim:sparse-fill"] authored_coloring=false resolved_role_tool=None
selected_targets[2].config keys=["line_width", "whole_print_batch_ops", "whole_print_fixture_mode"]
selected_targets[2].config line_width=Float(0.4)
selected_targets[2].config private_probe=absent
selected_targets[2].config "whole_print_batch_ops"=String("")
selected_targets[2].config "whole_print_fixture_mode"=String("standard")
read seams layer=7 offset=0 max_items=18446744073709551615
seams present
seams count=3
seams[0] key=[7|"custom-cube"|7|$A] candidates=1
seams[0].chosen point=(1.0,2.0,0.3) width=0.4 flow=0.9 quartile=Some(2) dist_to_top=0.7 overhang_distance=Some(-0.2) wall_index=1
seams[0].candidate[0] point=(1.5,2.5,0.3) width=0.4 flow=0.0 quartile=None dist_to_top=0.0 overhang_distance=None score=0.25 reason=user_forced
seams[1] key=[7|"custom-cube"|7|$a] candidates=1
seams[1].chosen point=(1.0,2.0,0.3) width=0.4 flow=0.9 quartile=Some(2) dist_to_top=0.7 overhang_distance=Some(-0.2) wall_index=1
seams[1].candidate[0] point=(1.5,2.5,0.3) width=0.4 flow=0.0 quartile=None dist_to_top=0.0 overhang_distance=None score=0.25 reason=user_forced
seams[2] key=[7|"custom-cube"|9223372036862319096|$A] candidates=1
seams[2].chosen point=(1.0,2.0,0.3) width=0.4 flow=0.9 quartile=Some(2) dist_to_top=0.7 overhang_distance=Some(-0.2) wall_index=1
seams[2].candidate[0] point=(1.5,2.5,0.3) width=0.4 flow=0.0 quartile=None dist_to_top=0.0 overhang_distance=None score=0.25 reason=user_forced
read mesh_objects offset=0 max_items=18446744073709551615
mesh_objects count=1
mesh_objects[0] object="custom-cube" vertices=3 triangles=1 paint_layers=1
mesh_objects[0].bounds min=(0.0,0.0,1.0) max=(2.0,2.0,1.0)
mesh_objects[0].paint[0] semantic="mesh_probe" facets=1 values=[custom("mesh/π")] strokes=1
mesh_objects[0].paint[0].stroke[0] semantic="mesh_probe" value=custom("mesh/π") vertices=3 triangles_flat=[(0.0,0.0,1.0),(2.0,0.0,1.0),(0.0,2.0,1.0)]
"#.replace("$A", chain_a).replace("$a", chain_lower);
    let source_keys = [
        custom_native_key(7, "A/π"),
        custom_native_key(7, "a/π"),
        custom_native_key(MODIFIER, "A/π"),
    ];
    let map_keys = [
        custom_native_key(0, "A/π"),
        custom_native_key(0, "a/π"),
        custom_native_key(MODIFIER, "A/π"),
    ];
    for permutation in [(false, false), (true, false), (false, true), (true, true)] {
        let scenario = Scenario {
            custom_cube: Some(permutation),
            module_widths: [0.4, 0.6, 0.73],
            input_reads: Some(
                [
                    "layer_plan",
                    "slices",
                    "region_map",
                    "selected_targets",
                    "seam_plan",
                    "mesh",
                ]
                .map(str::to_owned)
                .to_vec(),
            ),
            ..Scenario::default()
        };
        let seed = literal_inputs(Package::Infill, &scenario);
        assert_eq!(seed.slices.len(), 1);
        assert_eq!(seed.slices[0].regions.len(), 3);
        assert_eq!(seed.region_map.entries.len(), 3);
        assert_eq!(seed.seam.as_ref().unwrap().entries.len(), 3);
        assert_eq!(seed.mesh.objects.len(), 1);
        for region in &seed.slices[0].regions {
            assert_eq!(region.polygons.len(), 1);
            assert_eq!(region.sparse_infill_area.len(), 1);
        }
        let pair = run_pair(Package::Infill, &["claim:sparse-fill"], &scenario).unwrap();
        for (name, leg) in [("native", &pair.native), ("wasm", &pair.wasm)] {
            assert_literal_lines(
                &read_sections(
                    observations(leg, name),
                    &[
                        "read regions ",
                        "read region_configs ",
                        "read selected_targets ",
                        "read seams ",
                        "read mesh_objects ",
                    ],
                ),
                &expected,
                &format!("{permutation:?}/{name}: all lossless paint-bearing fields"),
            );
        }

        // Read actual returned DTOs through the SAME production native reader,
        // then exercise the SDK reverse impls against native literals. No
        // expectation is constructed by either From implementation under test.
        let driver = PreparedDriver::new(Package::Infill, &scenario, false).unwrap();
        let committed = seed.commit().unwrap();
        let input = prevalidate(
            &driver.owner,
            committed.clone(),
            scenario.selection(Package::Infill, &["claim:sparse-fill"]),
        )
        .unwrap();
        assert!(input.controlled_retains_committed(&committed));
        let reader = input
            .controlled_native_input(&driver.owner, &driver.binding)
            .unwrap();
        let regions = reader.regions(7, 0, u64::MAX).unwrap();
        assert_eq!(regions.len(), 3);
        for (region, expected) in regions.iter().zip(&source_keys) {
            let returned = PreparationRegionKey {
                global_layer_index: region.global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
                variant_chain: region.variant_chain.clone(),
            };
            let native = RegionKey::from(&returned);
            assert_eq!(&native, expected);
            let PaintValue::Scalar(scalar) = native.variant_chain[1].1 else {
                panic!("lost scalar variant")
            };
            assert_eq!(scalar.to_bits(), 0x8000_0000);
            let annotation =
                &region.segment_annotations[&PaintSemantic::Custom("annotation_probe".into())];
            assert_eq!(annotation.len(), 1);
            assert_eq!(annotation[0].len(), 4);
            let native: Vec<_> = annotation[0]
                .iter()
                .map(|value| value.as_ref().map(PaintValue::from))
                .collect();
            assert_eq!(
                native,
                [
                    Some(PaintValue::Custom("annotation/π".into())),
                    None,
                    Some(PaintValue::Flag(true)),
                    Some(PaintValue::Scalar(-0.0))
                ]
            );
            let Some(PaintValue::Scalar(scalar)) = native[3] else {
                panic!("lost annotation scalar")
            };
            assert_eq!(scalar.to_bits(), 0x8000_0000);
        }
        let configs = reader.region_configs(7, 0, u64::MAX).unwrap();
        assert_eq!(configs.len(), 3);
        assert_eq!(
            configs
                .iter()
                .map(|row| RegionKey::from(&row.key))
                .collect::<Vec<_>>(),
            map_keys
        );
        let seams = reader.seams(7, 0, u64::MAX).unwrap().unwrap();
        assert_eq!(seams.len(), 3);
        assert_eq!(
            seams
                .iter()
                .map(|row| RegionKey::from(&row.region_key))
                .collect::<Vec<_>>(),
            source_keys
        );
        let targets = reader.selected_targets(7, 0, u64::MAX).unwrap();
        assert_eq!(targets.len(), 3);
        for (target, expected) in targets.iter().zip(&source_keys) {
            let PreparationTargetSource::Model { key, members } = &target.source else {
                panic!("expected custom model source")
            };
            assert_eq!(RegionKey::from(key), *expected);
            assert_eq!(members.len(), 1);
            assert_eq!(RegionKey::from(&members[0]), *expected);
        }
        let objects = reader.mesh_objects(0, u64::MAX).unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].paint_layers.len(), 1);
        let paint = &objects[0].paint_layers[0];
        assert_eq!(paint.facet_values.len(), 1);
        assert_eq!(
            PaintValue::from(paint.facet_values[0].as_ref().unwrap()),
            PaintValue::Custom("mesh/π".into())
        );
        assert_eq!(paint.strokes.len(), 1);
        assert_eq!(
            PaintValue::from(&paint.strokes[0].value),
            PaintValue::Custom("mesh/π".into())
        );
    }

    // Inspect the REAL compiled component's ordinary type, not WIT source text.
    // Capability-owned Custom must not silently widen the ordinary paint value.
    let wit = std::process::Command::new("wasm-tools")
        .args(["component", "wit"])
        .arg(Package::Infill.artifact())
        .output()
        .expect("required wasm-tools for component contract");
    assert!(
        wit.status.success(),
        "{}",
        String::from_utf8_lossy(&wit.stderr)
    );
    let wit = String::from_utf8(wit.stdout).unwrap();
    let ordinary = wit
        .split_once("variant paint-value {")
        .expect("ordinary paint type in compiled component")
        .1
        .split_once('}')
        .unwrap()
        .0;
    assert_eq!(
        ordinary.split_whitespace().collect::<String>(),
        "flag(bool),scalar(f32),tool-index(u32),"
    );
}
