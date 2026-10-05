//! Packet `config-scope-resolution_09_layer-range-scope` — AC-2, AC-N1, AC-N2.
//!
//! A minimal hand-authored registry carries the ordinary keys the assertions
//! type (`layer_height`, `first_layer_height`, `infill_density`), the selector
//! `wall_generator`, and the machine key `bed_shape`. Every denial comes from
//! the registry's own `denied_scopes` read through `admission_set`; there is no
//! key roster in this file.

use std::collections::{BTreeMap, HashMap};

use slicer_config::resolution::{layer_top_zs, query_layer_height_profile, HeightProfileSegment};
use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigIngestor, ConfigSchemaRegistry, ConfigScope,
    ExpansionContext, HostChannels, IngestionWarning, LayerConfigRange, LayerRangeInput,
    LayerRangeLoadError, ModuleDeclaration, ResolutionError, ResolutionTarget, ScopeDelta,
    ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::WHOLE_PRINT_ONLY_SCOPES;
use slicer_ir::ConfigValue;

fn float_field() -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "float".to_owned(),
        ..ConfigFieldEntry::default()
    }
}

fn denied_field(field_type: &str, denied: &[&str]) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: field_type.to_owned(),
        denied_scopes: denied.iter().map(|scope| (*scope).to_owned()).collect(),
        ..ConfigFieldEntry::default()
    }
}

fn selector_field(field_type: &str, denied: &[&str]) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: field_type.to_owned(),
        selector: true,
        denied_scopes: denied.iter().map(|scope| (*scope).to_owned()).collect(),
        ..ConfigFieldEntry::default()
    }
}

fn range_registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.layer-range-scope".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([
                    ("layer_height".to_owned(), float_field()),
                    ("first_layer_height".to_owned(), float_field()),
                    ("infill_density".to_owned(), float_field()),
                    (
                        "wall_generator".to_owned(),
                        selector_field("string", WHOLE_PRINT_ONLY_SCOPES),
                    ),
                    (
                        "bed_shape".to_owned(),
                        denied_field("float-list", WHOLE_PRINT_ONLY_SCOPES),
                    ),
                ]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
    )
    .expect("layer-range fixture registry must assemble")
    .registry
}

/// Build one raw-string transport value from the model-IO parser.
fn range_input(
    object_id: &str,
    source_index: u32,
    min_z: f64,
    max_z: f64,
    values: &[(&str, &str)],
) -> LayerRangeInput {
    // exhaustive: this transport fixture pins every LayerRangeInput field explicitly
    LayerRangeInput {
        object_id: object_id.to_owned(),
        source_index,
        min_z,
        max_z,
        values: values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

/// AC-2: a parsed world-Z interval `[0.4, 0.8)` types both admitted raw
/// strings through the registry, keeps millimetre bounds, and is indexed per
/// object in `(min_z, max_z, source_index)` order.
#[test]
fn world_z_half_open_range_is_typed_and_indexed_per_object() {
    let registry = range_registry();
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_layer_ranges(&[
            range_input(
                "obj-a",
                4,
                0.4,
                0.8,
                &[("layer_height", "0.1"), ("infill_density", "0.35")],
            ),
            range_input("obj-b", 2, 0.9, 1.1, &[("infill_density", "0.45")]),
            range_input("obj-b", 0, 0.2, 0.6, &[("infill_density", "0.25")]),
        ])
        .expect("admitted typed layer ranges should load");
    let outcome = ingestor.finish();

    let obj_a = "obj-a".to_owned();
    let ranges = outcome.scoped.ranges_for(&obj_a);
    assert_eq!(ranges.len(), 1, "obj-a states exactly one range");
    let range = &ranges[0];
    assert_eq!(
        range.scope,
        ConfigScope::LayerRange {
            object_id: obj_a.clone(),
            range_index: 0,
        }
    );
    assert_eq!(range.min_z, 0.4);
    assert_eq!(range.max_z, 0.8);
    assert_eq!(
        range.delta.values.get("layer_height"),
        Some(&ConfigValue::Float(0.1))
    );
    assert_eq!(
        range.delta.values.get("infill_density"),
        Some(&ConfigValue::Float(0.35))
    );

    // World-Z membership is half-open `[min_z, max_z)` on the production
    // authority (`LayerConfigRange::covers`), not a test-local predicate. The
    // transported layer top is an `f32`, so membership is asserted against the
    // exact values the pipeline compares: `f64::from(0.4_f32)` is included and
    // `f64::from(0.8_f32)` excluded, and `max_z` itself would still be excluded
    // even if the top arrived exactly on it.
    assert!(
        range.covers(f64::from(0.4_f32)),
        "the transported min_z bound is inside the half-open interval"
    );
    assert!(range.covers(0.79));
    assert!(
        !range.covers(f64::from(0.8_f32)),
        "the transported max_z bound is outside the half-open interval"
    );
    assert!(
        !range.covers(0.8),
        "max_z itself is outside the half-open interval"
    );

    // Indices restart per object and follow `(min_z, max_z, source_index)`.
    let obj_b = "obj-b".to_owned();
    let ranges = outcome.scoped.ranges_for(&obj_b);
    assert_eq!(ranges.len(), 2);
    assert_eq!(
        ranges[0].scope,
        ConfigScope::LayerRange {
            object_id: obj_b.clone(),
            range_index: 0,
        }
    );
    assert_eq!(ranges[0].min_z, 0.2);
    assert_eq!(
        ranges[1].scope,
        ConfigScope::LayerRange {
            object_id: obj_b.clone(),
            range_index: 1,
        }
    );
    assert_eq!(ranges[1].min_z, 0.9);
    assert!(outcome.scoped.has_layer_ranges());
}

/// AC-N1: unequal typed values over a non-empty overlap on a non-`layer_height`
/// key are a load error, equal values are accepted, `layer_height` is exempt,
/// and a failed call commits nothing.
#[test]
fn conflicting_non_layer_height_overlap_is_atomic_load_error() {
    let registry = range_registry();
    let object_id = "obj-a".to_owned();

    let mut rejected = ConfigIngestor::new(&registry);
    let error = rejected
        .ingest_layer_ranges(&[
            range_input("obj-a", 0, 0.2, 0.6, &[("infill_density", "0.30")]),
            range_input("obj-a", 3, 0.4, 0.8, &[("infill_density", "0.35")]),
        ])
        .expect_err("unequal values over a non-empty overlap must be a load error");
    assert_eq!(
        error,
        LayerRangeLoadError::ConflictingOverlap {
            object_id: object_id.clone(),
            key: "infill_density".to_owned(),
            first_range: 0,
            second_range: 1,
        }
    );
    let outcome = rejected.finish();
    assert!(
        !outcome.scoped.has_layer_ranges(),
        "a rejected load must leave no committed layer ranges"
    );
    assert!(outcome.scoped.ranges_for(&object_id).is_empty());
    assert!(outcome.scoped.deltas.is_empty());

    // Equal typed values over a non-empty overlap are accepted.
    let mut equal = ConfigIngestor::new(&registry);
    equal
        .ingest_layer_ranges(&[
            range_input("obj-a", 0, 0.2, 0.6, &[("infill_density", "0.35")]),
            range_input("obj-a", 3, 0.4, 0.8, &[("infill_density", "0.35")]),
        ])
        .expect("equal values over an overlap are accepted");
    assert_eq!(equal.finish().scoped.ranges_for(&object_id).len(), 2);

    // `layer_height` participates in no conflict check.
    let mut heights = ConfigIngestor::new(&registry);
    heights
        .ingest_layer_ranges(&[
            range_input("obj-a", 0, 0.2, 0.6, &[("layer_height", "0.1")]),
            range_input("obj-a", 3, 0.4, 0.8, &[("layer_height", "0.25")]),
        ])
        .expect("layer_height is exempt from overlap conflict checks");
    let outcome = heights.finish();
    let ranges = outcome.scoped.ranges_for(&object_id);
    assert_eq!(ranges.len(), 2);
    assert_eq!(
        ranges[0].delta.values.get("layer_height"),
        Some(&ConfigValue::Float(0.1))
    );
    assert_eq!(
        ranges[1].delta.values.get("layer_height"),
        Some(&ConfigValue::Float(0.25))
    );

    // A failing call leaves already committed state untouched.
    let mut committed = ConfigIngestor::new(&registry);
    committed
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.2,
            0.6,
            &[("infill_density", "0.30")],
        )])
        .expect("baseline range commits");
    let failed = committed.ingest_layer_ranges(&[
        range_input("obj-a", 3, 0.4, 0.8, &[("infill_density", "0.35")]),
        range_input("obj-a", 0, 0.2, 0.6, &[("infill_density", "0.30")]),
    ]);
    assert!(
        matches!(failed, Err(LayerRangeLoadError::ConflictingOverlap { .. })),
        "the second call must fail on the overlapping unequal values"
    );
    let outcome = committed.finish();
    let ranges = outcome.scoped.ranges_for(&object_id);
    assert_eq!(
        ranges.len(),
        1,
        "the failing call must not append or alter committed ranges"
    );
    assert_eq!(
        ranges[0].scope,
        ConfigScope::LayerRange {
            object_id,
            range_index: 0,
        }
    );
}

/// AC-N2: a selector key and a denied machine key are rejected at layer-range
/// scope by `admission_set`, atomically; a kebab-case spelling is never
/// silently accepted.
#[test]
fn selector_or_denied_key_in_layer_range_is_load_error() {
    let registry = range_registry();
    let object_id = "obj-a".to_owned();

    // Selector key: the registry's own `denied_scopes` deny layer_range, so
    // `admission_set` rejects it — no local roster.
    let mut selector = ConfigIngestor::new(&registry);
    let error = selector
        .ingest_layer_ranges(&[
            range_input("obj-a", 0, 0.4, 0.8, &[("wall_generator", "arachne")]),
            range_input("obj-a", 0, 0.4, 0.8, &[("infill_density", "0.35")]),
        ])
        .expect_err("a selector statement at layer-range scope must be denied");
    assert_eq!(
        error,
        LayerRangeLoadError::Denied(ResolutionError::ScopeDenied {
            key: "wall_generator".to_owned(),
            scope: ConfigScope::LayerRange {
                object_id: object_id.clone(),
                range_index: 0,
            },
        })
    );
    let outcome = selector.finish();
    assert!(
        !outcome.scoped.has_layer_ranges(),
        "denial retains no partial range"
    );
    assert!(
        outcome.scoped.deltas.is_empty(),
        "denial retains no partial range delta either"
    );

    // Machine key: same admission path, different registry declaration.
    let mut machine = ConfigIngestor::new(&registry);
    let error = machine
        .ingest_layer_ranges(&[range_input("obj-a", 0, 0.4, 0.8, &[("bed_shape", "0x0")])])
        .expect_err("a machine key statement at layer-range scope must be denied");
    assert_eq!(
        error,
        LayerRangeLoadError::Denied(ResolutionError::ScopeDenied {
            key: "bed_shape".to_owned(),
            scope: ConfigScope::LayerRange {
                object_id: object_id.clone(),
                range_index: 0,
            },
        })
    );
    assert!(!machine.finish().scoped.has_layer_ranges());

    // Snake_case only: a kebab-case spelling is never silently accepted.
    let mut kebab = ConfigIngestor::new(&registry);
    kebab
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            0.8,
            &[("infill-density", "0.35")],
        )])
        .expect("an undeclared key warns and drops; it is never an error");
    let outcome = kebab.finish();
    let ranges = outcome.scoped.ranges_for(&object_id);
    assert_eq!(ranges.len(), 1);
    assert!(
        !ranges[0].delta.values.contains_key("infill-density"),
        "a kebab-case key must never be retained in a typed delta"
    );
    assert!(
        outcome.warnings.iter().any(|warning| matches!(
            warning,
            IngestionWarning::UnrecognizedKey { wire_key, .. } if wire_key == "infill-density"
        )),
        "the dropped kebab-case key must be reported, got {:?}",
        outcome.warnings
    );
}

/// `LayerConfigRange::new` rejects every invalid interval class before any
/// ingestion state exists.
#[test]
fn invalid_interval_constructor_rejects_non_ascending_or_negative_bounds() {
    let delta = ScopeDelta {
        values: BTreeMap::new(),
    };
    for (min_z, max_z) in [(-0.1, 0.5), (0.5, 0.5), (0.6, 0.5), (0.0, f64::INFINITY)] {
        let error = LayerConfigRange::new("obj-a".to_owned(), 0, min_z, max_z, delta.clone())
            .expect_err("invalid intervals must be rejected");
        assert_eq!(
            error,
            LayerRangeLoadError::InvalidInterval {
                object_id: "obj-a".to_owned(),
                range_index: 0,
                min_z,
                max_z,
            }
        );
    }
    assert!(
        matches!(
            LayerConfigRange::new("obj-a".to_owned(), 0, f64::NAN, 0.5, delta),
            Err(LayerRangeLoadError::InvalidInterval { min_z, .. }) if min_z.is_nan()
        ),
        "a non-finite lower bound must be rejected"
    );
}

fn expansion() -> ExpansionContext {
    ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    }
}

/// Ingest range inputs and return the typed range world, or panic with context.
fn scoped_with_ranges(registry: &ConfigSchemaRegistry, inputs: &[LayerRangeInput]) -> ScopedConfig {
    let mut ingestor = ConfigIngestor::new(registry);
    ingestor
        .ingest_layer_ranges(inputs)
        .expect("admitted typed layer ranges should load");
    ingestor.finish().scoped
}

/// Build one typed scope delta from canonical keys and values.
fn delta(entries: impl IntoIterator<Item = (&'static str, ConfigValue)>) -> ScopeDelta {
    ScopeDelta {
        values: entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

/// Compare a top-Z schedule with literal expectations at the packet's
/// documented `1e-9` tolerance: lengths must match exactly and every element
/// must agree within the tolerance. `0.2 + 0.1` is `0.30000000000000004` in
/// `f64`, so a direct `assert_eq!` on the literal vector is a float-identity
/// assertion, not a schedule assertion.
fn assert_top_zs(actual: &[f64], expected: &[f64]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "schedule length must match exactly; actual {actual:?}, expected {expected:?}"
    );
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 1.0e-9,
            "top Z {index} must be {expected} within 1e-9, got {actual}"
        );
    }
}

fn scoped_height_range(registry: &ConfigSchemaRegistry, min_z: f64, max_z: f64) -> ScopedConfig {
    let mut ingestor = ConfigIngestor::new(registry);
    ingestor
        .ingest_flat(&HashMap::from([
            ("layer_height".to_owned(), ConfigValue::Float(0.2)),
            ("first_layer_height".to_owned(), ConfigValue::Float(0.2)),
        ]))
        .expect("base heights are admitted");
    ingestor
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            min_z,
            max_z,
            &[("layer_height", "0.1")],
        )])
        .expect("authored world-Z range is admitted");
    ingestor.finish().scoped
}

/// World [0.4, 0.8) shifts to local [0, 0.4), but the object's fixed
/// first-layer interval retains [0, 0.2). These are authored-literal oracles.
#[test]
fn raft_offset_shifts_world_range_and_retains_first_layer() {
    let registry = range_registry();
    let scoped = scoped_height_range(&registry, 0.4, 0.8);
    let profile = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.4, &expansion())
        .expect("raft-shifted profile must compose");
    assert_eq!(
        profile,
        vec![
            HeightProfileSegment {
                z_start: 0.0,
                z_end: 0.2,
                height: 0.2
            },
            HeightProfileSegment {
                z_start: 0.2,
                z_end: 0.4,
                height: 0.1
            },
            HeightProfileSegment {
                z_start: 0.4,
                z_end: 1.0,
                height: 0.2
            },
        ]
    );
    assert_top_zs(
        &layer_top_zs(&profile, 1.0),
        &[0.2, 0.3, 0.4, 0.6, 0.8, 1.0],
    );
}

/// An authored range entirely below the raft cannot alter object layers.
#[test]
fn raft_offset_skips_world_range_below_object() {
    let registry = range_registry();
    let scoped = scoped_height_range(&registry, 0.1, 0.3);
    let profile = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.4, &expansion())
        .expect("range below the object must be skipped");
    assert_eq!(
        profile,
        vec![HeightProfileSegment {
            z_start: 0.0,
            z_end: 1.0,
            height: 0.2
        },]
    );
    assert_top_zs(&layer_top_zs(&profile, 1.0), &[0.2, 0.4, 0.6, 0.8, 1.0]);
}

/// A zero offset preserves the fixture's original no-raft profile and tops.
#[test]
fn zero_raft_offset_preserves_no_raft_fixture() {
    let registry = range_registry();
    let scoped = scoped_height_range(&registry, 0.4, 0.8);
    let profile = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.0, &expansion())
        .expect("zero-offset profile must compose");
    assert_eq!(
        profile,
        vec![
            HeightProfileSegment {
                z_start: 0.0,
                z_end: 0.4,
                height: 0.2
            },
            HeightProfileSegment {
                z_start: 0.4,
                z_end: 0.8,
                height: 0.1
            },
            HeightProfileSegment {
                z_start: 0.8,
                z_end: 1.0,
                height: 0.2
            },
        ]
    );
    assert_top_zs(
        &layer_top_zs(&profile, 1.0),
        &[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0],
    );
}

/// AC-3: an earlier-starting `layer_height` range keeps an overlap and trims a
/// later range's low edge, uncovered intervals use the base height, and the
/// derived top-Z schedule steps through the trimmed profile. The negative
/// control overlays a literal later-wins profile and proves the two schedules
/// disagree, so a later-wins implementation cannot pass this test.
#[test]
fn earlier_starting_layer_height_range_retains_overlap_and_gap_fills() {
    let registry = range_registry();
    let mut ingestor = ConfigIngestor::new(&registry);
    let mut globals = HashMap::new();
    globals.insert("layer_height".to_owned(), ConfigValue::Float(0.20));
    globals.insert("first_layer_height".to_owned(), ConfigValue::Float(0.20));
    ingestor
        .ingest_flat(&globals)
        .expect("global base heights are admitted");
    ingestor
        .ingest_layer_ranges(&[
            range_input("obj-a", 0, 0.20, 0.70, &[("layer_height", "0.10")]),
            range_input("obj-a", 1, 0.50, 0.90, &[("layer_height", "0.25")]),
        ])
        .expect("overlapping layer_height ranges are exempt from conflict checks");
    let scoped = ingestor.finish().scoped;

    let profile = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.0, &expansion())
        .expect("a valid object profile must compose");
    assert_eq!(
        profile,
        vec![
            HeightProfileSegment {
                z_start: 0.00,
                z_end: 0.20,
                height: 0.20,
            },
            HeightProfileSegment {
                z_start: 0.20,
                z_end: 0.70,
                height: 0.10,
            },
            HeightProfileSegment {
                z_start: 0.70,
                z_end: 0.90,
                height: 0.25,
            },
            HeightProfileSegment {
                z_start: 0.90,
                z_end: 1.00,
                height: 0.20,
            },
        ],
        "earlier-starting range retains [0.20, 0.70); the later range's low is trimmed to 0.70"
    );

    let tops = layer_top_zs(&profile, 1.0);
    assert_top_zs(&tops, &[0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.95]);
    assert_eq!(
        tops.len(),
        7,
        "the final 0.25 step from 0.95 would exceed the 1.0 object height"
    );

    // Negative control: a hand-built later-wins overlay would produce a
    // different schedule at the overlap. It is compared segment-by-segment with
    // literal arithmetic, never by calling the production composer.
    let later_wins = vec![
        HeightProfileSegment {
            z_start: 0.00,
            z_end: 0.20,
            height: 0.20,
        },
        HeightProfileSegment {
            z_start: 0.20,
            z_end: 0.50,
            height: 0.10,
        },
        HeightProfileSegment {
            z_start: 0.50,
            z_end: 0.90,
            height: 0.25,
        },
        HeightProfileSegment {
            z_start: 0.90,
            z_end: 1.00,
            height: 0.20,
        },
    ];
    // Evaluate the control with the same stepping rule spelled inline, so the
    // control never borrows even the production evaluator.
    let mut later_wins_tops = Vec::new();
    let mut z = 0.0;
    while let Some(segment) = later_wins
        .iter()
        .find(|segment| segment.z_start <= z && z < segment.z_end)
    {
        z += segment.height;
        if z <= 1.0 + 1.0e-9 {
            later_wins_tops.push(z);
        } else {
            break;
        }
    }
    assert_top_zs(&later_wins_tops, &[0.20, 0.30, 0.40, 0.50, 0.75, 1.00]);
    assert_eq!(
        later_wins_tops.len(),
        6,
        "the later-wins control steps through its own literal segments"
    );
    assert_ne!(
        tops.len(),
        later_wins_tops.len(),
        "a later-wins implementation must not match the canonical earlier-starting schedule"
    );
    assert!(
        (tops[4] - 0.60).abs() <= 1.0e-9 && (later_wins_tops[4] - 0.75).abs() <= 1.0e-9,
        "the disagreement must be visible at the first top Z that starts inside the trimmed segment; \
         canonical {tops:?}, later-wins {later_wins_tops:?}"
    );
}

/// Regression for the range-boundary truncation root cause: a `layer_height`
/// range that ends mid-object must not swallow the object's final layers.
///
/// `0.7 + 0.1` accumulates to `0.7999999999999999`, one ULP below the
/// `[0.4, 0.8)` range end. Testing the accumulated top with a half-open
/// `z_start <= z < z_end` predicate therefore re-selected the exhausted
/// 0.1 mm segment and the walk never crossed into the 0.2 mm segment above the
/// range: a 1.0 mm object was planned only to Z=0.9 and its top layer was
/// silently dropped. The walk must select a layer's height from a probe above
/// the top, so each boundary is crossed exactly once; canonical
/// `generate_object_layers` (`Slicing.cpp`) probes inside the layer for the
/// same reason.
///
/// The expectations are the fixture's literal schedule: this is the regression
/// input that previously failed at exactly the last step.
#[test]
fn range_boundary_does_not_truncate_the_object_top_layer() {
    let registry = range_registry();
    let mut ingestor = ConfigIngestor::new(&registry);
    let mut globals = HashMap::new();
    globals.insert("layer_height".to_owned(), ConfigValue::Float(0.20));
    globals.insert("first_layer_height".to_owned(), ConfigValue::Float(0.20));
    ingestor
        .ingest_flat(&globals)
        .expect("global base heights are admitted");
    ingestor
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            0.8,
            &[("layer_height", "0.1")],
        )])
        .expect("the single range is admitted");
    let scoped = ingestor.finish().scoped;

    let profile = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.0, &expansion())
        .expect("a valid object profile must compose");
    assert_eq!(
        profile,
        vec![
            HeightProfileSegment {
                z_start: 0.0,
                z_end: 0.4,
                height: 0.2,
            },
            HeightProfileSegment {
                z_start: 0.4,
                z_end: 0.8,
                height: 0.1,
            },
            HeightProfileSegment {
                z_start: 0.8,
                z_end: 1.0,
                height: 0.2,
            },
        ],
        "the profile is the fixture's three literal segments"
    );

    // The pre-fix output ended at `0.8999999999999999` (f32 0.9): the top
    // 0.2 mm layer of the 1.0 mm object was never planned.
    let tops = layer_top_zs(&profile, 1.0);
    assert_top_zs(&tops, &[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]);
    assert_eq!(
        tops.len(),
        7,
        "a 1.0 mm object with a 0.1 mm range over [0.4, 0.8) must be sliced to \
         its top: the pre-fix walk stopped one layer short at {tops:?}"
    );
    assert!(
        (tops[6] - 1.0).abs() <= 1.0e-9,
        "the final top must reach the object height, got {tops:?}"
    );

    // A range ending exactly on the object top has no layer above it: the walk
    // must still stop, never spin or emit a top beyond the object height.
    let flush = vec![HeightProfileSegment {
        z_start: 0.0,
        z_end: 1.0,
        height: 0.2,
    }];
    assert_top_zs(&layer_top_zs(&flush, 1.0), &[0.2, 0.4, 0.6, 0.8, 1.0]);
}

/// Regression: the transported layer top is an `f32`, so a top the author
/// placed exactly on a range bound arrives slightly off it.
///
/// `f32(0.7)` widens back to `0.69999998807…`, one rounding step *below* its
/// own authored `0.7`. A plain `min_z <= z && z < max_z` test therefore
/// included the layer at top `0.7` in a `[0.4, 0.7)` range — the exclusive
/// bound was effectively inclusive. `LayerConfigRange::covers` absorbs that
/// rounding, so the boundary stays excluded and `min_z` stays included.
#[test]
fn transported_top_at_a_range_bound_keeps_the_half_open_rule() {
    let registry = range_registry();
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            0.7,
            &[("infill_density", "0.35")],
        )])
        .expect("the layer range is admitted");
    let scoped = ingestor.finish().scoped;
    let range = &scoped.ranges_for(&"obj-a".to_owned())[0];

    // The transported values, at the precision the pipeline actually compares.
    let transported_max = f64::from(0.7_f32);
    assert!(
        transported_max < 0.7,
        "the transported f32 bound must be the value that exposed the defect; got {transported_max}"
    );
    assert!(
        !range.covers(transported_max),
        "the layer top transported for the authored exclusive bound 0.7 must be \
         excluded, even though it widens to {transported_max}"
    );
    assert!(
        !range.covers(0.7),
        "the authored max_z itself is also excluded"
    );
    assert!(
        range.covers(f64::from(0.4_f32)),
        "the transported min_z bound must be included"
    );
    assert!(range.covers(0.69), "a top strictly inside is included");

    // Within the half-ULP transport band the authored distinction is gone:
    // any authored position that narrows to the same `f32` as the bound IS the
    // bound (that is what `transported_max` above demonstrates), so it stays
    // excluded rather than being guessed interior. A top a full `f32` ULP
    // below the bound is still genuinely interior and must be included.
    let one_ulp_below = f64::from(f32::from_bits(0.7_f32.to_bits() - 1));
    assert!(
        range.covers(one_ulp_below),
        "a top a full f32 ULP below max_z is genuinely interior and must be included; got {one_ulp_below}"
    );

    // A power-of-two bound is the case that defeated a fixed half-ULP shave:
    // at 1.0 the forward and backward f32 gaps differ by a factor of two
    // (2^-23 vs 2^-24), so half the forward gap lands exactly on the
    // representable interior top below 1.0. It must stay included.
    let mut power_of_two = ConfigIngestor::new(&registry);
    power_of_two
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            1.0,
            &[("infill_density", "0.5")],
        )])
        .expect("the power-of-two range is admitted");
    let power_of_two = power_of_two.finish().scoped;
    let range = &power_of_two.ranges_for(&"obj-a".to_owned())[0];
    let interior_below_one = f64::from(f32::from_bits(1.0_f32.to_bits() - 1));
    assert!(
        range.covers(interior_below_one),
        "the representable interior top {interior_below_one} below the exclusive \
         bound 1.0 must be included"
    );
    assert!(!range.covers(1.0), "the exclusive bound 1.0 is excluded");
    assert!(
        !range.covers(f64::from(f32::from_bits(1.0_f32.to_bits() + 1))),
        "a top above 1.0 is excluded"
    );

    // The same rule through the real resolver: the boundary layer keeps the
    // object value while the layer below it takes the range value.
    let mut world = ScopedConfig {
        layer_ranges: scoped.layer_ranges.clone(),
        ..ScopedConfig::default()
    };
    world.deltas.insert(
        ConfigScope::Object("obj-a".to_owned()),
        delta([("infill_density", ConfigValue::Float(0.20))]),
    );
    let density_at = |layer_top_z: f64| {
        resolve_scope_stack(
            &registry,
            &world,
            &ResolutionTarget {
                object_id: "obj-a".to_owned(),
                layer_top_z: Some(layer_top_z),
                ..ResolutionTarget::default()
            },
            &expansion(),
        )
        .expect("the transported-bound target must resolve")
        .infill_density
    };
    assert_eq!(
        density_at(transported_max),
        0.20,
        "the layer at the authored exclusive bound 0.7 must keep the object value"
    );
    assert_eq!(
        density_at(f64::from(0.65_f32)),
        0.35,
        "a layer strictly inside [0.4, 0.7) must take the range value"
    );
}

/// A range stating an unusable `layer_height` is a load error, never a range
/// the profile silently skips.
///
/// `layer_height` is the one key the profile composes directly; dropping a
/// non-positive value while still applying the range's other values would
/// silently ignore part of an authored range.
#[test]
fn non_positive_layer_height_in_a_range_is_a_load_error() {
    let registry = range_registry();

    for unusable in [0.0_f64, -0.1] {
        let mut ingestor = ConfigIngestor::new(&registry);
        let error = ingestor
            .ingest_layer_ranges(&[range_input(
                "obj-a",
                0,
                0.4,
                0.8,
                &[
                    ("layer_height", &format!("{unusable}")),
                    ("infill_density", "0.35"),
                ],
            )])
            .expect_err("an unusable layer_height must reject the whole range");
        assert_eq!(
            error,
            LayerRangeLoadError::InvalidLayerHeight {
                object_id: "obj-a".to_owned(),
                range_index: 0,
                layer_height: unusable,
            },
            "layer_height = {unusable} must be a named load error"
        );

        // Atomic: the rejected load leaves no range and no resolved value.
        let outcome = ingestor.finish();
        assert!(
            !outcome.scoped.has_layer_ranges(),
            "a rejected load must commit no ranges"
        );
        assert!(
            outcome.scoped.deltas.is_empty(),
            "a rejected load must commit no deltas"
        );
    }

    // The boundary is exclusive: a positive height is still accepted, and a
    // NaN-authored value is a typing failure rather than a silent skip.
    let mut accepted = ConfigIngestor::new(&registry);
    accepted
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            0.8,
            &[("layer_height", "0.0001")],
        )])
        .expect("a tiny positive height is usable");
    assert!(accepted.finish().scoped.has_layer_ranges());
}

/// AC-4: a catch-up layer whose top Z falls inside a layer range inherits the
/// range's value at its canonical precedence slot (object < range < modifier),
/// and the range is half-open so its `max_z` is excluded.
#[test]
fn catch_up_layer_inherits_range_covering_its_top_z_at_reserved_precedence() {
    let registry = range_registry();
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_layer_ranges(&[range_input(
            "obj-a",
            0,
            0.4,
            0.8,
            &[("infill_density", "0.35")],
        )])
        .expect("the layer range is admitted");
    let ranges = ingestor.finish().scoped;

    // Object scope stated on its own, so each later case can add exactly one
    // more applicable scope to the same typed range world and expose the
    // precedence chain by removal. `layer_ranges` must travel with the typed
    // range world; a fresh `ScopedConfig` would silently drop it.
    let mut object_only = ScopedConfig {
        layer_ranges: ranges.layer_ranges.clone(),
        ..ScopedConfig::default()
    };
    object_only.deltas.insert(
        ConfigScope::Object("obj-a".to_owned()),
        delta([("infill_density", ConfigValue::Float(0.20))]),
    );

    let mut with_modifier = object_only.clone();
    with_modifier.deltas.insert(
        ConfigScope::Modifier {
            object_id: "obj-a".to_owned(),
            modifier_id: "priority-10".to_owned(),
        },
        delta([("infill_density", ConfigValue::Float(0.45))]),
    );

    let resolve = |scoped: &ScopedConfig, target: ResolutionTarget| {
        resolve_scope_stack(&registry, scoped, &target, &expansion())
            .expect("literal precedence case must resolve")
            .infill_density
    };
    let covering = |layer_top_z: Option<f64>, modifier_ids: Vec<String>| ResolutionTarget {
        object_id: "obj-a".to_owned(),
        modifier_ids,
        layer_top_z,
        ..ResolutionTarget::default()
    };

    // Precedence chain, three scopes removed one at a time: modifier (0.45)
    // over object+range, then range (0.35) over the object alone, then the
    // object value (0.20) once the range no longer applies.
    assert_eq!(
        resolve(
            &with_modifier,
            covering(Some(0.65), vec!["priority-10".to_owned()])
        ),
        0.45,
        "a modifier outranks a covering layer range"
    );
    // Range outranks the object value; the modifier id is absent so the
    // modifier delta is not part of this target's applicable scope set.
    assert_eq!(
        resolve(&with_modifier, covering(Some(0.65), Vec::new())),
        0.35,
        "a covering layer range outranks the object value"
    );
    // Half-open: `max_z` and everything above it are outside the interval;
    // the object-only world isolates the object value from the modifier.
    assert_eq!(
        resolve(&object_only, covering(Some(0.85), Vec::new())),
        0.20,
        "0.85 is outside [0.4, 0.8) so the object value survives"
    );
    assert_eq!(
        resolve(&object_only, covering(Some(0.80), Vec::new())),
        0.20,
        "max_z itself is excluded by the half-open interval"
    );
    // No layer context: no range applies.
    assert_eq!(
        resolve(&object_only, covering(None, Vec::new())),
        0.20,
        "layer_top_z: None means no range applies, so only the object value remains"
    );
    // Range removed entirely: the authored object value is exposed. This world
    // keeps the object delta but drops the typed ranges. The object states a
    // value that is deliberately NOT the typed default, so "the range stopped
    // applying" cannot be satisfied by the default leaking through.
    let mut object_without_ranges = ScopedConfig {
        deltas: object_only.deltas.clone(),
        ..ScopedConfig::default()
    };
    object_without_ranges.deltas.insert(
        ConfigScope::Object("obj-a".to_owned()),
        delta([("infill_density", ConfigValue::Float(0.15))]),
    );
    assert_ne!(
        0.15_f32,
        <slicer_ir::ResolvedConfig as Default>::default().infill_density,
        "the control value must differ from the typed default, or this case could \
         pass with the range simply never applying"
    );
    assert_eq!(
        resolve(&object_without_ranges, covering(Some(0.65), Vec::new())),
        0.15,
        "without the range, the authored non-default object value is exposed"
    );
    // Object removed too: the typed default is exposed, never a range value.
    assert_eq!(
        resolve(&ScopedConfig::default(), covering(Some(0.65), Vec::new())),
        0.2,
        "with no scopes at all the typed default is exposed"
    );
}

/// The profile rejects an invalid object height before touching the range
/// world, rejects non-positive resolved base heights too, and the top-Z
/// evaluator is pure: it never consults the registry.
#[test]
fn profile_rejects_invalid_heights_and_top_zs_are_pure() {
    let registry = range_registry();
    let scoped = scoped_with_ranges(&registry, &[]);

    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let error =
            query_layer_height_profile(&registry, &scoped, "obj-a", invalid, 0.0, &expansion())
                .expect_err("an invalid object height must be rejected");
        assert_eq!(
            error,
            ResolutionError::InvalidObjectHeight {
                object_id: "obj-a".to_owned(),
                value: invalid.is_finite().then_some(invalid),
            }
        );
    }

    // A non-positive resolved base height is rejected through the same atomic
    // error before any segment is composed. Non-finite floats cannot reach
    // this path: ingestion rejects them as a typed mismatch.
    for (key, invalid) in [
        ("layer_height", 0.0),
        ("layer_height", -0.5),
        ("first_layer_height", 0.0),
        ("first_layer_height", -1.0),
    ] {
        let mut ingestor = ConfigIngestor::new(&registry);
        let mut globals = HashMap::new();
        globals.insert(key.to_owned(), ConfigValue::Float(invalid));
        ingestor
            .ingest_flat(&globals)
            .expect("non-positive floats are still typable");
        let scoped = ingestor.finish().scoped;

        let error = query_layer_height_profile(&registry, &scoped, "obj-a", 1.0, 0.0, &expansion())
            .expect_err("a non-positive resolved base height must be rejected");
        assert_eq!(
            error,
            ResolutionError::InvalidObjectHeight {
                object_id: "obj-a".to_owned(),
                value: invalid.is_finite().then_some(invalid),
            },
            "{key} = {invalid} must be rejected"
        );
    }

    // A zero-step profile cannot loop, and a z outside all coverage stops.
    assert_eq!(
        layer_top_zs(
            &[HeightProfileSegment {
                z_start: 0.0,
                z_end: 1.0,
                height: 0.0,
            }],
            1.0
        ),
        Vec::<f64>::new()
    );
    assert_eq!(
        layer_top_zs(
            &[HeightProfileSegment {
                z_start: 0.0,
                z_end: 0.5,
                height: 0.25,
            }],
            1.0
        ),
        vec![0.25, 0.5]
    );
}
