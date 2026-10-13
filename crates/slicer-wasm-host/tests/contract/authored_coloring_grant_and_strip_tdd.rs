//! Contract: the authored-coloring grant is two-sided, and the marshal/commit
//! boundary is the authoritative enforcement point for authored `tool_index`.
//!
//! Packet 226. A module may author a per-path tool only when BOTH sides agree:
//! it discloses `claim:authored-coloring` in its manifest, AND at least one
//! fill-role claim it actually holds for the region is listed in the profile's
//! `fill_authored_coloring` key. Anything else — no grant, an ungranted region,
//! or an out-of-range index — is silently stripped to `None` (meaning "the host
//! resolves the region tool") by `convert_infill_output`. Stripping is never an
//! error: the call still returns `Ok`.

#![allow(missing_docs)]

use slicer_wasm_host::host::{ExtrusionPath3d, ExtrusionRole, Point3WithWidth};
use slicer_wasm_host::marshal::accumulators::InfillOutputCollected;
use slicer_wasm_host::marshal::OriginId;
use slicer_wasm_host::marshal::{
    authored_coloring_granted, convert_infill_output, AuthoredColoringContext,
};

const OBJECT_ID: &str = "cube";
const REGION_ID: u64 = 7;

fn path_with_tool(tool_index: Option<u32>) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: vec![
            // exhaustive: Point3WithWidth has no Default; fixture specifies all geometry fields
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
            // exhaustive: Point3WithWidth has no Default; fixture specifies all geometry fields
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index,
        order_lock: None,
    }
}

/// One tagged sparse path carrying `tool_index`, attributed to
/// `(OBJECT_ID, REGION_ID)`.
fn collected_with_tool(tool_index: Option<u32>) -> InfillOutputCollected {
    InfillOutputCollected {
        sparse_paths: vec![path_with_tool(tool_index)],
        sparse_path_origins: vec![Some(OriginId {
            object_id: OBJECT_ID.to_string(),
            region_id: REGION_ID,
            variant_chain: Vec::new(),
        })],
        ..Default::default()
    }
}

fn granting_ctx(tool_count: u32) -> AuthoredColoringContext {
    AuthoredColoringContext {
        tool_count,
        granted_regions: [OriginId {
            object_id: OBJECT_ID.to_string(),
            region_id: REGION_ID,
            variant_chain: Vec::new(),
        }]
        .into_iter()
        .collect(),
        ..Default::default()
    }
}

/// Read back the single committed sparse path's `tool_index`.
fn committed_tool_index(ir: &slicer_ir::InfillIR) -> Option<u32> {
    assert_eq!(ir.regions.len(), 1, "expected exactly one committed region");
    let region = &ir.regions[0];
    assert_eq!(
        region.sparse_infill.len(),
        1,
        "expected exactly one committed sparse path"
    );
    region.sparse_infill[0].tool_index
}

// ── convert_infill_output enforcement ────────────────────────────────────

#[test]
fn granted_region_in_range_keeps_authored_tool_index() {
    let ctx = granting_ctx(4);
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("granted authored tool must commit successfully");
    assert_eq!(
        committed_tool_index(&ir),
        Some(2),
        "a granted region with an in-range authored tool must KEEP tool_index = Some(2); \
         stripping it here would mean the grant is inert"
    );
}

#[test]
fn ungranted_module_authored_tool_is_stripped_to_none() {
    // `None` context = no module holds the grant on this dispatch.
    let result = convert_infill_output(&collected_with_tool(Some(2)), 0, None);
    let ir = result.expect("stripping must be silent — still Ok, never an error");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "an ungranted module's authored tool must be stripped to None \
         (host resolves the region tool)"
    );
}

#[test]
fn granted_region_out_of_range_tool_is_stripped_to_none() {
    let ctx = granting_ctx(2); // valid tools are 0 and 1
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("out-of-range strip must be silent — still Ok, never an error");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "tool_index >= tool_count must be stripped even for a granted region"
    );
}

#[test]
fn granted_but_different_region_is_stripped_to_none() {
    let ctx = AuthoredColoringContext {
        tool_count: 4,
        granted_regions: [OriginId {
            object_id: "other-object".to_string(),
            region_id: REGION_ID,
            variant_chain: Vec::new(),
        }]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let ir = convert_infill_output(&collected_with_tool(Some(2)), 0, Some(&ctx))
        .expect("strip must be silent");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "the grant is per-region: a region absent from granted_regions is denied"
    );
}

#[test]
fn absent_tool_index_is_unchanged_under_a_grant() {
    let ctx = granting_ctx(4);
    let ir = convert_infill_output(&collected_with_tool(None), 0, Some(&ctx)).expect("must commit");
    assert_eq!(
        committed_tool_index(&ir),
        None,
        "paths that author no tool are passed through untouched"
    );
}

#[test]
fn variant_grant_does_not_authorize_sibling() {
    let variant_a = OriginId {
        object_id: OBJECT_ID.to_string(),
        region_id: REGION_ID,
        variant_chain: vec![(
            "tool_index".to_string(),
            slicer_ir::PaintValue::ToolIndex(1),
        )],
    };
    let variant_b = OriginId {
        variant_chain: vec![(
            "tool_index".to_string(),
            slicer_ir::PaintValue::ToolIndex(2),
        )],
        ..variant_a.clone()
    };
    let ctx = AuthoredColoringContext {
        tool_count: 4,
        granted_regions: [variant_a.clone()].into_iter().collect(),
        ..Default::default()
    };
    let mut collected = collected_with_tool(Some(2));
    collected.sparse_path_origins = vec![Some(variant_a.clone())];
    let authorized = convert_infill_output(&collected, 0, Some(&ctx))
        .expect("exact variant grant must commit successfully");
    assert_eq!(authorized.regions[0].variant_chain, variant_a.variant_chain);
    assert_eq!(committed_tool_index(&authorized), Some(2));

    collected.sparse_path_origins = vec![Some(variant_b.clone())];
    let sibling = convert_infill_output(&collected, 0, Some(&ctx))
        .expect("sibling variant strip must be silent");
    assert_eq!(sibling.regions[0].variant_chain, variant_b.variant_chain);
    assert_eq!(
        committed_tool_index(&sibling),
        None,
        "a grant for variant A must not authorize variant B with the same object and base region"
    );
}

// ── authored_coloring_granted predicate ──────────────────────────────────

#[test]
fn variant_grants_cover_native_and_wasm_boundaries() {
    use std::sync::Arc;

    use slicer_ir::{
        ConfigView, ExPolygon, GlobalLayer, LayerStageCommit, PaintValue, Point2, Polygon,
        RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
    };
    use slicer_sdk::{
        builders::InfillOutputBuilder,
        native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry},
    };
    use slicer_wasm_host::{
        binding::LayerStageInput,
        selection::{
            build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput,
            LayerTargetSource, STAGES,
        },
        CompiledModuleLive, LayerStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
    };

    thread_local! {
        static AUTHORED_TOOL: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
        static EMITTED: std::cell::RefCell<Vec<(OriginId, u32)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn native_authored_witness(
        request: &NativeLayerRequest,
    ) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
        let tool = AUTHORED_TOOL.with(std::cell::Cell::get);
        let mut output = InfillOutputBuilder::new();
        // Deliberately author BOTH selected variants, including the denied
        // sibling. Only the production dispatch-to-commit boundary may strip.
        for region in &request.regions {
            let origin = OriginId {
                object_id: region.object_id().clone(),
                region_id: *region.region_id(),
                variant_chain: region.variant_chain().to_vec(),
            };
            output.begin_region(&origin.object_id, origin.region_id, &origin.variant_chain);
            let wire_path = path_with_tool(Some(tool));
            output
                // exhaustive: extrusion path has no Default; witness pins authored metadata.
                .push_sparse_path(slicer_ir::ExtrusionPath3D {
                    points: wire_path
                        .points
                        .iter()
                        .map(|p| slicer_ir::Point3WithWidth {
                            x: p.x,
                            y: p.y,
                            z: p.z,
                            width: p.width,
                            flow_factor: p.flow_factor,
                            ..Default::default()
                        })
                        .collect(),
                    role: slicer_ir::ExtrusionRole::SparseInfill,
                    speed_factor: 1.0,
                    tool_index: Some(tool),
                    order_lock: None,
                })
                .expect("emit even the denied sibling's authored path");
            EMITTED.with(|records| records.borrow_mut().push((origin, tool)));
        }
        // exhaustive: native response explicitly populates only infill output.
        Ok(NativeLayerResponse {
            infill: Some(output),
            perimeters: None,
            support: None,
            slice_postprocess: None,
            path_optimization: None,
            anchored_events: None,
        })
    }

    let origins = [1, 2].map(|tool| OriginId {
        object_id: OBJECT_ID.into(),
        region_id: REGION_ID,
        variant_chain: vec![("material".into(), PaintValue::ToolIndex(tool))],
    });
    // Delivered/output/grant identity is the SOURCE-space id minted by
    // `paint_variant_region_id`; `RegionMapIR` entries use the temporary
    // map-space address only.
    let source_origins: Vec<OriginId> = origins
        .iter()
        .map(|origin| OriginId {
            region_id: slicer_core::algos::paint_segmentation::paint_variant_region_id(
                origin.region_id,
                &origin.variant_chain,
            ),
            ..origin.clone()
        })
        .collect();
    let layer = GlobalLayer::default();
    // Declared read enables exact-config projection. The single-tool baseline
    // must be overridden by the three-filament per-variant printer config.
    let invocation = ConfigView::from_map(std::collections::HashMap::from([(
        "filament_density".into(),
        slicer_ir::ConfigValue::List(vec![slicer_ir::ConfigValue::Float(1.24)]),
    )]));
    let module_id = "authored-variant-witness";
    let native_module_id = module_id.to_string();
    let claims = vec!["claim:sparse-fill".into(), CLAIM.into()];
    let live_module = CompiledModuleLive::new(
        &native_module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        Arc::new(invocation.clone()),
    )
    .with_native_entry(NativeStageEntry::Layer(native_authored_witness));
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    // exhaustive: borrowed selection binding supplies all static authority.
    let binding = LayerSelectionModuleInput {
        stage_id: "Layer::Infill",
        module_id,
        claims: &claims,
        paint_semantics: None,
        invocation_config: &invocation,
    };

    // Independently permute slice order and map insertion/config-interning order.
    for slice_order in [[0, 1], [1, 0]] {
        for map_order in [[0, 1], [1, 0]] {
            let slice = SliceIR {
                regions: slice_order
                    .iter()
                    .map(|&i| SlicedRegion {
                        object_id: OBJECT_ID.into(),
                        region_id: source_origins[i].region_id,
                        variant_chain: origins[i].variant_chain.clone(),
                        polygons: vec![ExPolygon {
                            contour: Polygon {
                                points: vec![
                                    Point2::from_mm(0.0, 0.0),
                                    Point2::from_mm(10.0, 0.0),
                                    Point2::from_mm(10.0, 10.0),
                                    Point2::from_mm(0.0, 10.0),
                                ],
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            };
            let mut map = RegionMapIR::default();
            for i in map_order {
                let config = map.intern_config(ResolvedConfig {
                    sparse_fill_holder: module_id.into(),
                    // Production derive_tool_count reads this per-tool list.
                    filament_density: vec![1.24; 3],
                    fill_authored_coloring: if i == 0 {
                        vec!["claim:sparse-fill".into()]
                    } else {
                        Vec::new()
                    },
                    ..Default::default()
                });
                map.entries.insert(
                    RegionKey {
                        global_layer_index: layer.index,
                        object_id: OBJECT_ID.into(),
                        region_id: REGION_ID,
                        variant_chain: origins[i].variant_chain.clone(),
                    },
                    RegionPlan {
                        config,
                        ..Default::default()
                    },
                );
            }
            // exhaustive: public whole-print constructor receives every product channel.
            let index = build_layer_selection_index(LayerSelectionIndexInput {
                stage_roster: STAGES,
                global_layers: std::slice::from_ref(&layer),
                modules: std::slice::from_ref(&binding),
                slices: std::slice::from_ref(&slice),
                region_map: Some(&map),
                support_plan: None,
                perimeters: &[],
            })
            .expect("exact-key production selection");
            let selection = index.selection_for(layer.index, module_id).unwrap();
            assert_eq!(selection.targets.len(), 2, "both variants must be selected");
            let granted_origins: std::collections::HashSet<_> = selection
                .targets
                .iter()
                .filter(|target| target.authored_coloring)
                .filter_map(|target| match &target.source {
                    LayerTargetSource::Model { key, .. } => Some(OriginId {
                        object_id: key.object_id.clone(),
                        region_id: key.region_id,
                        variant_chain: key.variant_chain.clone(),
                    }),
                    _ => None,
                })
                .collect();
            assert_eq!(
                granted_origins,
                [source_origins[0].clone()].into_iter().collect(),
                "the production projection must grant the SOURCE-space identity of variant 1"
            );

            let run = |tool| {
                AUTHORED_TOOL.with(|value| value.set(tool));
                EMITTED.with(|records| records.borrow_mut().clear());
                // exhaustive: runner input pins every product and committed selection channel.
                let input = LayerStageInput {
                    mesh: Arc::new(slicer_ir::MeshIR::default()),
                    paint_regions: None,
                    seam_plan: None,
                    support_plan: None,
                    lightning_tree_ir: None,
                    region_map: Some(Arc::new(map.clone())),
                    slice: Some(&slice),
                    perimeter: None,
                    layer_collection: None,
                    surface_classification: None,
                    prepared_regions: None,
                    prepared_perimeter_source_regions: None,
                    infill: None,
                    selection: Some(selection),
                };
                let commit = LayerStageRunner::run_stage(
                    &dispatcher,
                    &"Layer::Infill".into(),
                    &layer,
                    &live_module,
                    input,
                )
                .expect("production native authored dispatch")
                .expect("both nonempty variants must commit");
                EMITTED.with(|records| {
                    let records = records.borrow();
                    assert_eq!(records.len(), 2, "native entry emits both variants");
                    // Raw delivery law: the native entry receives each selected painted source verbatim.
                    for origin in &source_origins {
                        assert!(
                            records.contains(&(origin.clone(), tool)),
                            "native entry must author the tool even for the denied sibling"
                        );
                    }
                });
                let LayerStageCommit::Infill(native) = commit else {
                    panic!("expected native infill commit");
                };
                native
            };
            let (native, contexts) =
                slicer_wasm_host::dispatch::with_authored_coloring_capture_for_test(|| run(1));
            assert_eq!(
                contexts.len(),
                1,
                "one production dispatch computes one context"
            );
            let ctx = &contexts[0];
            assert_eq!(
                ctx.tool_count, 3,
                "dispatch derives the per-variant tool range"
            );
            assert_eq!(
                ctx.granted_regions, granted_origins,
                "dispatch grants must retain the full selected SOURCE-space origin"
            );
            assert!(
                ctx.region_tools.is_empty(),
                "painted sources retain paint authority"
            );
            // Independent tagged wire paths consume the EXACT context observed
            // during native production dispatch, not a reconstructed grant set.
            assert_eq!(native.regions.len(), 2);
            for (i, expected) in [Some(1), None].into_iter().enumerate() {
                let region = native
                    .regions
                    .iter()
                    .find(|r| r.variant_chain == source_origins[i].variant_chain)
                    .expect("native commit preserves each variant identity");
                assert_eq!(region.object_id, OBJECT_ID);
                // Raw delivery law: commit preserves the native entry's selected painted source ID.
                assert_eq!(region.region_id, source_origins[i].region_id);
                assert_eq!(region.sparse_infill.len(), 1);
                assert_eq!(region.sparse_infill[0].points.len(), 2);
                assert_eq!(region.sparse_infill[0].tool_index, expected);

                // Independent wire fixture, not the native builder's collected
                // output. Exercise the real WASM conversion boundary directly.
                for (tool, context, expected_tool) in [
                    (Some(1), Some(ctx), expected),
                    (Some(1), None, None),
                    (Some(3), Some(ctx), None),
                ] {
                    let mut collected = collected_with_tool(tool);
                    collected.sparse_path_origins = vec![Some(source_origins[i].clone())];
                    let wasm = convert_infill_output(&collected, layer.index, context)
                        .expect("WASM boundary stripping is silent");
                    assert_eq!(
                        wasm.regions[0].variant_chain,
                        source_origins[i].variant_chain
                    );
                    assert_eq!(committed_tool_index(&wasm), expected_tool);
                }
            }
            // Pin the production-derived range to exactly three tools, not
            // merely a count large enough for the primary authored tool 1.
            for (tool, granted_tool) in [(2, Some(2)), (3, None)] {
                let native = run(tool);
                assert_eq!(native.regions.len(), 2);
                for (i, expected) in [granted_tool, None].into_iter().enumerate() {
                    let region = native
                        .regions
                        .iter()
                        .find(|r| r.variant_chain == origins[i].variant_chain)
                        .expect("native range control preserves each variant");
                    assert_eq!(region.sparse_infill.len(), 1);
                    assert_eq!(
                        region.sparse_infill[0].tool_index, expected,
                        "production dispatch must derive tool_count = 3"
                    );
                }
            }
        }
    }
}

const CLAIM: &str = "claim:authored-coloring";

#[test]
fn grant_requires_both_disclosure_and_config_listing() {
    let held = vec!["claim:fill-sparse".to_string()];
    let listed = vec!["claim:fill-sparse".to_string()];
    let disclosed = vec![CLAIM.to_string(), "claim:fill-sparse".to_string()];
    assert!(
        authored_coloring_granted(&held, &listed, &disclosed),
        "both sides agree: the grant must be issued"
    );
}

#[test]
fn disclosed_but_not_listed_in_config_is_denied() {
    let held = vec!["claim:fill-sparse".to_string()];
    // Config opts a DIFFERENT fill claim into authored coloring.
    let listed = vec!["claim:fill-top".to_string()];
    let disclosed = vec![CLAIM.to_string(), "claim:fill-sparse".to_string()];
    assert!(
        !authored_coloring_granted(&held, &listed, &disclosed),
        "disclosure alone must not grant: the held fill claim is not in fill_authored_coloring"
    );
}

#[test]
fn listed_in_config_but_not_disclosed_is_denied() {
    let held = vec!["claim:fill-sparse".to_string()];
    let listed = vec!["claim:fill-sparse".to_string()];
    // Manifest never discloses `claim:authored-coloring`.
    let disclosed = vec!["claim:fill-sparse".to_string()];
    assert!(
        !authored_coloring_granted(&held, &listed, &disclosed),
        "config opt-in alone must not grant: the module never disclosed the claim"
    );
}

#[test]
fn empty_inputs_are_denied_and_do_not_panic() {
    assert!(!authored_coloring_granted(&[], &[], &[]));
    assert!(!authored_coloring_granted(&[], &[], &[CLAIM.to_string()]));
    assert!(!authored_coloring_granted(
        &["claim:fill-sparse".to_string()],
        &[],
        &[CLAIM.to_string()]
    ));
}
