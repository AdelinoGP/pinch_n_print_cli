//! Deterministic controls for `H-REGIONMAP`.
//!
//! Each function is a hand-built witness that (a) the real system under test
//! passes the independent checker on a named fixture and (b) a named mutation of
//! that observation makes the same checker fail. The mutations stand in for the
//! wrong implementations the property must catch, so a passing property is
//! never mistaken for a property that cannot fail.

use std::collections::BTreeMap;

use super::model::{
    expected_chains, lexical_overlay_witness, material_fuzzy_case, observe, run_with_cap,
    validate_bounds, verify, AxisDecl, BelowCap, Case, ObjectAxis, ObjectCase, ValueKind,
    AXIS_POOL,
};
use slicer_core::algos::region_mapping::{
    execute_region_mapping_with_cap, RegionMappingError, RegionMappingPlanProjection,
};
use slicer_ir::{
    ActiveRegion, GlobalLayer, LayerPlanIR, ModuleInvocation, PaintValue, RegionKey,
    RegionSplitValueType, ResolvedConfig, SemVer, StageId,
};

/// The observed value sequence of the chain with `len` pairs, if present.
fn chain_of_len(
    case: &Case,
    observed: &super::model::Observed,
    len: usize,
) -> Vec<(String, PaintValue)> {
    observed
        .entries
        .iter()
        .map(|(key, _, _)| key.variant_chain.clone())
        .find(|chain| chain.len() == len)
        .unwrap_or_else(|| panic!("no chain of length {len} in {:?}", case.axes))
}

fn assert_chain_absent(
    case: &Case,
    observed: &super::model::Observed,
    chain: &[(String, PaintValue)],
) {
    assert!(
        !observed
            .entries
            .iter()
            .any(|(key, _, _)| key.variant_chain == chain),
        "chain {chain:?} must be absent, axes {:?}",
        case.axes
    );
}

/// `prepass_model_controls` — the checker rejects every structural mutation.
pub fn model_controls() {
    let case = material_fuzzy_case();
    let observed = observe(&case).expect("the witness fixture must run");
    verify(&case, &observed).expect("the live witness must satisfy the checker");

    // 1. A dropped empty chain (the BASE parent entry).
    let mut dropped_empty = observed.clone();
    dropped_empty
        .entries
        .retain(|(key, _, _)| !key.variant_chain.is_empty());
    assert!(
        verify(&case, &dropped_empty).is_err(),
        "a dropped empty chain must fail the checker"
    );

    // 2. A dropped combined chain.
    let combined = chain_of_len(&case, &observed, 2);
    let mut dropped_combined = observed.clone();
    dropped_combined
        .entries
        .retain(|(key, _, _)| key.variant_chain != combined);
    assert!(
        verify(&case, &dropped_combined).is_err(),
        "a dropped combined chain must fail the checker"
    );

    // 3. A name-order substitution: the combined chain reversed.
    let mut name_ordered = observed.clone();
    for (key, _, _) in &mut name_ordered.entries {
        if key.variant_chain == combined {
            key.variant_chain.reverse();
        }
    }
    assert!(
        verify(&case, &name_ordered).is_err(),
        "a name-ordered combined chain must fail the checker"
    );

    // 4. A wrong value on a singleton chain.
    let mut wrong_value = observed.clone();
    for (key, _, _) in &mut wrong_value.entries {
        if key.variant_chain.len() == 1 && key.variant_chain[0].0 == "material" {
            key.variant_chain[0].1 = PaintValue::ToolIndex(1);
        }
    }
    assert!(
        verify(&case, &wrong_value).is_err(),
        "a substituted value must fail the checker"
    );

    // 5. A duplicate key.
    let mut duplicated = observed.clone();
    duplicated.entries.push(duplicated.entries[0].clone());
    assert!(
        verify(&case, &duplicated).is_err(),
        "a duplicated RegionKey must fail the checker"
    );

    // 6. A wrong per-key config.
    let mut wrong_config = observed.clone();
    wrong_config.entries[0].1.wall_loops += 1;
    assert!(
        verify(&case, &wrong_config).is_err(),
        "a wrong config_for result must fail the checker"
    );

    // 7. A non-idempotent interner (duplicate pool slot).
    let mut duplicated_pool = observed.clone();
    let duplicate = duplicated_pool.interned[0].clone();
    duplicated_pool.interned.push(duplicate);
    assert!(
        verify(&case, &duplicated_pool).is_err(),
        "a duplicated interned config must fail the checker"
    );

    // 8. A missing interned config.
    let mut missing_pool = observed.clone();
    missing_pool.interned.pop();
    assert!(
        verify(&case, &missing_pool).is_err(),
        "a truncated config pool must fail the checker"
    );

    // 9. A cap boundary reported as success.
    let mut wrong_cap = observed.clone();
    wrong_cap.below_cap = BelowCap::Succeeded(wrong_cap.exact_cap_len);
    assert!(
        verify(&case, &wrong_cap).is_err(),
        "a one-below-cap success must fail the checker"
    );
}

/// `prepass_regionmap_order_and_cap_controls` — the named order and width
/// witnesses, plus the cap boundaries.
pub fn order_and_cap_controls() {
    // --- The four explicit material/fuzzy chains -------------------------
    let case = material_fuzzy_case();
    let observed = observe(&case).expect("the material/fuzzy witness must run");
    let mut chains: Vec<Vec<(String, PaintValue)>> = observed
        .entries
        .iter()
        .map(|(key, _, _)| key.variant_chain.clone())
        .collect();
    chains.sort();
    let mut expected = vec![
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(0))],
        vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
        vec![
            ("material".to_string(), PaintValue::ToolIndex(0)),
            ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ],
    ];
    expected.sort();
    assert_eq!(
        chains, expected,
        "the complete chain set must be the four named vectors"
    );
    assert_chain_absent(
        &case,
        &observed,
        &[
            ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
            ("material".to_string(), PaintValue::ToolIndex(0)),
        ],
    );

    // --- Community priority beats name; equal priorities break lexically ---
    let community = Case {
        layers: 1,
        regions_per_object: 1,
        axes: vec![
            AxisDecl {
                name: "com.example.zeta",
                priority: 1000,
                kind: ValueKind::ToolIndex,
            },
            AxisDecl {
                name: "com.example.alpha",
                priority: 1100,
                kind: ValueKind::Custom,
            },
            AxisDecl {
                name: "com.example.beta",
                priority: 1100,
                kind: ValueKind::Flag,
            },
        ],
        objects: vec![ObjectCase {
            id: "obj_0".to_string(),
            axes: vec![
                ObjectAxis {
                    axis: 0,
                    values: vec![PaintValue::ToolIndex(0)],
                },
                ObjectAxis {
                    axis: 1,
                    values: vec![PaintValue::Custom("alpha".to_string())],
                },
                ObjectAxis {
                    axis: 2,
                    values: vec![PaintValue::Flag(true)],
                },
            ],
        }],
    };
    let community_observed = observe(&community).expect("the community witness must run");
    verify(&community, &community_observed).expect("the community witness must be modelled");
    let full = chain_of_len(&community, &community_observed, 3);
    assert_eq!(
        full,
        vec![
            ("com.example.zeta".to_string(), PaintValue::ToolIndex(0)),
            (
                "com.example.alpha".to_string(),
                PaintValue::Custom("alpha".to_string())
            ),
            ("com.example.beta".to_string(), PaintValue::Flag(true)),
        ],
        "priority 1000 precedes the tied 1100 pair, which breaks lexically"
    );

    // --- Lexical paint-overlay precedence and tool-last -------------------
    let without_tool = lexical_overlay_witness(None).expect("overlay witness must run");
    assert_eq!(
        without_tool, 0.75,
        "the combined key must take material's 0.75 mm, because lexical overlay \
         order is fuzzy_skin then material even though the chain is material-then-fuzzy"
    );
    let with_tool = lexical_overlay_witness(Some(1.0)).expect("tool overlay witness must run");
    assert_eq!(
        with_tool, 1.0,
        "the tool overlay is applied last and must win over both paint overlays"
    );

    // --- Cap boundaries ---------------------------------------------------
    let total = observed.exact_cap_len;
    let exact = run_with_cap(&case, total).expect("the exact-count cap must succeed");
    assert_eq!(exact.entries.len(), total);

    match run_with_cap(&case, total - 1) {
        Err(RegionMappingError::CapExceeded {
            entry_count, cap, ..
        }) => {
            assert_eq!(cap, total - 1);
            assert_eq!(entry_count, total);
        }
        other => panic!("one-below cap must fail with CapExceeded, got {other:?}"),
    }

    // Zero-count / zero-cap mode: no parents at all is not a cap failure.
    let empty = Case {
        layers: 0,
        regions_per_object: 1,
        axes: vec![],
        objects: vec![ObjectCase {
            id: "obj_0".to_string(),
            axes: vec![],
        }],
    };
    let empty_map = run_with_cap(&empty, 0).expect("an empty plan with cap 0 must succeed");
    assert!(empty_map.entries.is_empty());

    // Unique top contributor: obj_0's two regions are fully inserted, obj_1's
    // first insert trips the cap, so obj_0 is the only contributor.
    let two_objects = Case {
        layers: 1,
        regions_per_object: 2,
        axes: vec![AXIS_POOL[0], AXIS_POOL[1]],
        objects: vec![
            ObjectCase {
                id: "obj_0".to_string(),
                axes: vec![
                    ObjectAxis {
                        axis: 0,
                        values: vec![PaintValue::ToolIndex(0)],
                    },
                    ObjectAxis {
                        axis: 1,
                        values: vec![PaintValue::Flag(true)],
                    },
                ],
            },
            ObjectCase {
                id: "obj_1".to_string(),
                axes: vec![ObjectAxis {
                    axis: 1,
                    values: vec![PaintValue::Flag(true)],
                }],
            },
        ],
    };
    let per_parent = {
        let canonical = two_objects.canonical_axes();
        expected_chains(&canonical, &two_objects.objects[0], &two_objects)
            .expect("model chains must build")
            .len()
    };
    let cap = per_parent * 2;
    match run_with_cap(&two_objects, cap) {
        Err(RegionMappingError::CapExceeded {
            top_contributors,
            entry_count,
            cap: reported,
            ..
        }) => {
            assert_eq!(reported, cap);
            assert_eq!(entry_count, cap + 1);
            assert_eq!(
                top_contributors.len(),
                1,
                "obj_1 contributed no committed entry, so the top contributor is unique"
            );
            assert_eq!(top_contributors[0].object_id, "obj_0");
            assert_eq!(top_contributors[0].region_count, cap);
        }
        other => panic!("the two-object cap must fail with CapExceeded, got {other:?}"),
    }

    // The checker must reject a cap outcome that claims success.
    let mut mutated = observed.clone();
    mutated.below_cap = BelowCap::Succeeded(mutated.exact_cap_len);
    assert!(verify(&case, &mutated).is_err());

    // And it must reject an observation that omits the combined chain.
    let mut omitted = observed.clone();
    omitted
        .entries
        .retain(|(key, _, _)| key.variant_chain.len() != 2);
    assert!(verify(&case, &omitted).is_err());
}

/// `prepass_regionmap_rejection_controls` — caller-level error fields.
///
/// Covers the declared-scalar error, a duplicated full active identity, and the
/// modifier/painted-id obligations delegated to `controls_modifier`.
pub fn rejection_controls() {
    // --- Declared scalar value for an opted-in semantic --------------------
    let scalar_case = Case {
        layers: 1,
        regions_per_object: 1,
        axes: vec![AXIS_POOL[0]],
        objects: vec![ObjectCase {
            id: "obj_0".to_string(),
            axes: vec![ObjectAxis {
                axis: 0,
                values: vec![PaintValue::Scalar(0.5)],
            }],
        }],
    };
    match run_with_cap(&scalar_case, 16) {
        Err(RegionMappingError::ScalarInRegionSplitFacetValue {
            object_id,
            semantic,
            scalar_bits,
        }) => {
            assert_eq!(object_id, "obj_0");
            assert_eq!(semantic, "material");
            assert_eq!(scalar_bits, 0.5_f32.to_bits());
        }
        other => panic!("a declared scalar must be rejected, got {other:?}"),
    }

    // --- A duplicated full active identity ---------------------------------
    let layer_plan = LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![
                ActiveRegion {
                    object_id: "obj_0".to_string(),
                    region_id: 0,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    ..ActiveRegion::default()
                },
                ActiveRegion {
                    object_id: "obj_0".to_string(),
                    region_id: 0,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    ..ActiveRegion::default()
                },
            ],
            ..GlobalLayer::default()
        }],
        object_participation: std::collections::HashMap::new(),
    };
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let aggregated: BTreeMap<String, slicer_ir::slice_ir::AggregatedRegionSplitEntry> =
        BTreeMap::from([(
            "material".to_string(),
            slicer_ir::slice_ir::AggregatedRegionSplitEntry {
                priority: 100,
                value_type: RegionSplitValueType::ToolIndex,
                declaring_modules: Vec::new(),
            },
        )]);
    let error = execute_region_mapping_with_cap(
        &layer_plan,
        &projection,
        &BTreeMap::new(),
        &aggregated,
        &[],
        16,
    )
    .expect_err("a duplicated active identity must be rejected");
    match error {
        RegionMappingError::DuplicateRegionKey { key } => {
            assert_eq!(
                key,
                RegionKey {
                    global_layer_index: 0,
                    object_id: "obj_0".to_string(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                }
            );
        }
        other => panic!("expected DuplicateRegionKey, got {other:?}"),
    }

    // --- Modifier identity and the painted-id contract --------------------
    super::controls_modifier::modifier_identity_controls();
    super::controls_modifier::painted_id_contract_controls();
}

/// `prepass_owned_shrink_and_bound_controls` — a narrowing bound is rejected
/// before sampling.
pub fn bound_controls() {
    // Widest product with 3 axes and 3 values each is (1 + 3)^3 = 64.
    validate_bounds(3, 3, 64).expect("64 chains is exactly the bound");
    assert!(
        validate_bounds(3, 3, 63).is_err(),
        "a bound below the widest reachable product must be rejected"
    );
    // (1 + 4)^5 = 3125.
    validate_bounds(5, 4, 3125).expect("3125 chains is exactly the bound");
    assert!(
        validate_bounds(5, 4, 3124).is_err(),
        "3124 chains is one below the widest reachable product"
    );
    // No axes means exactly one chain (the empty chain).
    validate_bounds(0, 4, 1).expect("an axis-free domain has one chain");
    assert!(
        validate_bounds(0, 4, 0).is_err(),
        "zero chains can never bound the empty chain"
    );
}
