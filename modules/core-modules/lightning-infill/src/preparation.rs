//! Module-owned whole-print preparation for the lightning infill module.
//!
//! Preparation reads the committed typed inputs once for the whole print,
//! builds the kernel's planner inputs through `planning_input`, plans once, and
//! publishes one private piece per schedule layer (D2/D3). Decoding stays on the
//! consumer side: this module never reads its own pieces back.
//!
//! The advanced, default-`none` `prepared_plan_diagnostic` key (D6) selects an
//! optional publication mode. It is read once, from the first committed region
//! config, and applied only **after** planning: it can neither reach the kernel
//! nor change a planning scalar, and at `none` it changes no output byte.

use slicer_sdk::error::ModuleError;
use slicer_sdk::preparation::{
    LayerPreparation, PreparationInputError, PreparationRegion, PreparationRegionConfig,
    PreparedPlanOutput, PrintPreparationInput,
};

use crate::kernel::{generate_lightning_trees, KernelError, KernelPlan};
use crate::planning_input::build_planner_inputs;
use crate::private_plan::{encode_layer_piece, piece_name};
use crate::projection::publish_projection;
use crate::LightningInfill;

/// Diagnostic publication key declared in `lightning-infill.toml`.
const DIAGNOSTIC_KEY: &str = "prepared_plan_diagnostic";

/// Read a whole committed collection.
///
/// The typed facade already bounds every response itself (`batch_range` clips
/// the requested count at the committed length), so one open-ended request is
/// the whole collection rather than an unbounded host copy. The helper exists to
/// keep the input-error mapping identical at every call site.
fn read_all<T>(
    read: impl FnOnce(u64, u64) -> Result<Vec<T>, PreparationInputError>,
) -> Result<Vec<T>, ModuleError> {
    read(0, u64::MAX).map_err(input_error)
}

/// Map a failed typed read, naming the failing read in the message.
///
/// This deliberately does **not** reuse `57410`/`57411`/`57412`: those four
/// codes (`57401` included) name plan-content failures that AC-N2 requires to
/// stay four distinct failures. A refused or absent read is a granted-read
/// contract failure, and folding it into one of them would make two different
/// failures indistinguishable.
fn input_error(error: PreparationInputError) -> ModuleError {
    ModuleError::fatal(1, format!("lightning preparation input: {error}"))
}

/// Map a kernel planning failure.
fn kernel_error(error: KernelError) -> ModuleError {
    ModuleError::fatal(1, format!("lightning preparation kernel: {error}"))
}

/// Module-owned diagnostic publication mode (design D6).
///
/// The mode is a diagnostic contract, not an input: it is excluded from the
/// five-scalar agreement check by construction (it is not one of the scalars
/// `planning_input` reads) and it is applied after the plan exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreparedPlanDiagnostic {
    /// Publish the normal plan (the default).
    None,
    /// Omit the topmost layer that carries plan entries (schedule top when the
    /// plan is empty).
    MissingPiece,
    /// Publish a body the module decoder rejects with `57410`.
    MalformedPiece,
    /// Publish a well-formed header with an unsupported format version (`57411`).
    UnsupportedVersion,
    /// Publish one empty plan per layer and the empty `lightning-plan` scene.
    EmptyPlan,
    /// Publish the normal pieces but leave the declared `planning_geometry`
    /// scene unpublished, so the host's `MissingScene` rejection is reachable
    /// through the real module instead of a fixture guest. The view stays
    /// declared in the manifest; only this mode's publication is skipped.
    MissingScene,
}

impl PreparedPlanDiagnostic {
    /// Read the mode once from the first committed region config.
    ///
    /// Every committed region config carries the registry-seeded default, so an
    /// absent key or an unknown spelling is a contract defect rather than a
    /// silent fallback to `none`.
    fn read(config: Option<&PreparationRegionConfig>) -> Result<Self, ModuleError> {
        let Some(config) = config else {
            // No committed region config: no regions and no plan, so no
            // consumer can ever ask for a piece.
            return Ok(Self::None);
        };
        match config.config.require_string(DIAGNOSTIC_KEY)? {
            "none" => Ok(Self::None),
            "missing_piece" => Ok(Self::MissingPiece),
            "malformed_piece" => Ok(Self::MalformedPiece),
            "unsupported_version" => Ok(Self::UnsupportedVersion),
            "empty_plan" => Ok(Self::EmptyPlan),
            "missing_scene" => Ok(Self::MissingScene),
            other => Err(ModuleError::fatal(
                1,
                format!("unknown {DIAGNOSTIC_KEY} value: {other}"),
            )),
        }
    }

    /// The body this mode publishes for one layer.
    fn piece_body(self, plan: &KernelPlan, layer_index: i32) -> Vec<u8> {
        match self {
            Self::None | Self::MissingPiece | Self::MissingScene => {
                encode_layer_piece(plan, layer_index)
            }
            Self::MalformedPiece => malformed_piece_body(plan, layer_index),
            Self::UnsupportedVersion => unsupported_version_piece_body(layer_index),
            Self::EmptyPlan => encode_layer_piece(&KernelPlan::default(), layer_index),
        }
    }
}

/// A body the module decoder rejects with `57410`: a valid piece truncated by
/// one byte.
///
/// Using the real encoder keeps the diagnostic on the deployed format: every
/// strict prefix of a valid body is malformed, so the consumer fails at the
/// same bounds check a short read would.
fn malformed_piece_body(plan: &KernelPlan, layer_index: i32) -> Vec<u8> {
    let mut body = encode_layer_piece(plan, layer_index);
    body.pop();
    body
}

/// A well-formed header carrying format version `2`, which this decoder rejects
/// with `57411`.
///
/// The layout is D3's: `b"LTPL"`, `u16` version at bytes 4..6, `u32` entry count.
/// The entry count is zero, so the header is structurally complete; only the
/// version is unreadable.
fn unsupported_version_piece_body(layer_index: i32) -> Vec<u8> {
    let mut body = encode_layer_piece(&KernelPlan::default(), layer_index);
    body[4..6].copy_from_slice(&2_u16.to_le_bytes());
    body
}

impl LayerPreparation for LightningInfill {
    /// Plan the whole print from committed inputs and publish one piece per
    /// schedule layer.
    ///
    /// The ordinary stage is the only consumer: it asks for its own layer's
    /// piece, so a layer with no trees still needs a piece (an empty plan is a
    /// success there, not a missing piece). Layers come from the committed
    /// schedule rather than from the region population, because the consumer
    /// runs on schedule layers that may carry no regions at all.
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        let layers = read_all(|offset, max_items| input.layers(offset, max_items))?;
        if layers.is_empty() {
            // No committed schedule means no layer ever consumes a piece.
            return Ok(());
        }

        let mut regions: Vec<PreparationRegion> = Vec::new();
        let mut configs: Vec<PreparationRegionConfig> = Vec::new();
        for layer in &layers {
            regions.extend(read_all(|offset, max_items| {
                input.regions(layer.index, offset, max_items)
            })?);
            configs.extend(read_all(|offset, max_items| {
                input.region_configs(layer.index, offset, max_items)
            })?);
        }

        // Read before planning, applied only after it: the mode is not an input.
        let diagnostic = PreparedPlanDiagnostic::read(configs.first())?;

        let (slices, config) = build_planner_inputs(&regions, &configs)?;
        let plan = generate_lightning_trees(&slices, &config).map_err(kernel_error)?;

        // The topmost layer that actually carries plan entries is the one the
        // ordinary consumer is guaranteed to ask for: an entry exists only for a
        // layer with a committed region, and selection invokes the stage on every
        // such layer. When the plan is empty, the topmost scheduled layer is the
        // best available target.
        let missing_layer = plan
            .entries
            .iter()
            .map(|entry| entry.global_layer_index)
            .max()
            .map(|index| index as u32)
            .or_else(|| layers.iter().map(|layer| layer.index).max());

        for layer in &layers {
            if diagnostic == PreparedPlanDiagnostic::MissingPiece
                && Some(layer.index) == missing_layer
            {
                continue;
            }
            // The kernel stores each entry's layer as `SliceIR`'s `u32` cast to
            // `i32`, so the filter and the piece name must use that same
            // representation or a piece would search for entries that cannot
            // match its own name.
            let layer_index = layer.index as i32;
            output.put(
                &piece_name(layer_index),
                &diagnostic.piece_body(&plan, layer_index),
            )?;
        }

        // `missing_scene` keeps the pieces and deliberately publishes no scene:
        // the view stays declared, so the host's request reaches a completed
        // owner with nothing published and reports `MissingScene` — the
        // declared-but-unpublished case, not the undeclared-view case.
        if diagnostic != PreparedPlanDiagnostic::MissingScene {
            publish_projection(
                output,
                &plan,
                &regions,
                diagnostic == PreparedPlanDiagnostic::EmptyPlan,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, Mutex};

    use slicer_ir::{ConfigValue, ConfigView, ExPolygon, Point2, Polygon};
    use slicer_sdk::preparation::{
        PreparationInputError, PreparationLayer, PreparationProjectionError,
        PreparationProjectionGeometry, PreparationProjectionOutput, PreparationProjectionScene,
        PreparationProjectionWrite, PreparationRegionConfig, PreparationRegionKey,
        PrintPreparationRead,
    };

    use super::*;
    use crate::private_plan::decode_layer_piece;

    // -----------------------------------------------------------------------
    // Fixture: the kernel's 10/10/10/12 mm prism, one region per layer
    // -----------------------------------------------------------------------

    /// Square outline; the prism's fourth layer is 12 mm, so it seeds the only
    /// overhang in the fixture and therefore the only plan entries.
    fn square(side_mm: f32) -> ExPolygon {
        let max = slicer_ir::mm_to_units(side_mm);
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: max, y: 0 },
                    Point2 { x: max, y: max },
                    Point2 { x: 0, y: max },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn region(layer_index: u32, side_mm: f32) -> PreparationRegion {
        // exhaustive: `PreparationRegion` has no `Default`; every committed field
        // is spelled out so a new one is a visible compile error here rather than
        // a silently defaulted fixture input.
        PreparationRegion {
            global_layer_index: layer_index,
            z: 0.2 * (layer_index + 1) as f32,
            object_id: "cube".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
            polygons: vec![square(side_mm)],
            infill_areas: Vec::new(),
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
            internal_solid_fill: Vec::new(),
            internal_bridge_areas: Vec::new(),
            internal_bridge_angles_deg: Vec::new(),
            raft_fill: Vec::new(),
            phase: slicer_sdk::preparation::PreparationInputPhase::CommittedPrepass,
        }
    }

    fn planner_config(diagnostic: Option<&str>) -> ConfigView {
        let mut values = HashMap::new();
        values.insert("line_width".to_string(), ConfigValue::Float(0.45));
        values.insert("infill_density".to_string(), ConfigValue::Float(0.2));
        values.insert("layer_height".to_string(), ConfigValue::Float(0.2));
        values.insert("infill_angle".to_string(), ConfigValue::Float(45.0));
        if let Some(value) = diagnostic {
            values.insert(
                DIAGNOSTIC_KEY.to_string(),
                ConfigValue::String(value.to_string()),
            );
        }
        ConfigView::from_map(values)
    }

    fn region_config(layer_index: u32, diagnostic: Option<&str>) -> PreparationRegionConfig {
        PreparationRegionConfig {
            key: PreparationRegionKey {
                global_layer_index: layer_index,
                object_id: "cube".to_string(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            config: planner_config(diagnostic),
        }
    }

    struct Fixture {
        layers: Vec<PreparationLayer>,
        regions: BTreeMap<u32, Vec<PreparationRegion>>,
        configs: BTreeMap<u32, Vec<PreparationRegionConfig>>,
    }

    impl Fixture {
        /// Four schedule layers: 10, 10, 10, 12 mm outlines, so planning emits
        /// entries on layer 3 only and layers 0..=2 publish header-only pieces.
        fn prism(per_layer_diagnostics: Option<&[&str]>) -> Self {
            Self::with_sides(&[10.0, 10.0, 10.0, 12.0], per_layer_diagnostics)
        }

        /// The prism plus one 12 mm layer on top of the 12 mm layer, which is
        /// supported and therefore seeds no trees: its schedule top has no plan
        /// entries, so the `missing_piece` target is layer 3, not layer 4.
        fn tower(per_layer_diagnostics: Option<&[&str]>) -> Self {
            Self::with_sides(&[10.0, 10.0, 10.0, 12.0, 12.0], per_layer_diagnostics)
        }

        fn with_sides(sides: &[f32], per_layer_diagnostics: Option<&[&str]>) -> Self {
            if let Some(diagnostics) = per_layer_diagnostics {
                assert_eq!(
                    diagnostics.len(),
                    sides.len(),
                    "one diagnostic value per fixture layer"
                );
            }
            let mut layers = Vec::new();
            let mut regions = BTreeMap::new();
            let mut configs = BTreeMap::new();
            for (index, side) in sides.iter().enumerate() {
                let index_u32 = index as u32;
                let diagnostic = per_layer_diagnostics.map(|values| values[index]);
                layers.push(PreparationLayer {
                    index: index_u32,
                    z: 0.2 * (index_u32 + 1) as f32,
                    has_nonplanar: false,
                    is_sync_layer: false,
                    is_raft: false,
                    active_regions: Vec::new(),
                });
                regions.insert(index_u32, vec![region(index_u32, *side)]);
                configs.insert(index_u32, vec![region_config(index_u32, diagnostic)]);
            }
            Self {
                layers,
                regions,
                configs,
            }
        }
    }

    fn bounded<T: Clone>(items: &[T], offset: u64, max_items: u64) -> Vec<T> {
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(items.len());
        let take = usize::try_from(max_items)
            .unwrap_or(usize::MAX)
            .min(items.len() - start);
        items[start..start + take].to_vec()
    }

    /// Eleven typed reads preparation does not consume; any call is a fixture
    /// defect, so they fail with the named missing read rather than a default.
    macro_rules! unavailable_read {
        ($name:ident($($arg:ident: $ty:ty),*) -> $result:ty) => {
            fn $name(&self, $($arg: $ty),*) -> Result<$result, PreparationInputError> {
                $(let _ = $arg;)*
                Err(PreparationInputError::MissingRequired {
                    read: stringify!($name).into(),
                })
            }
        };
    }

    impl PrintPreparationRead for Fixture {
        fn layer_count(&self) -> Result<u64, PreparationInputError> {
            Ok(self.layers.len() as u64)
        }

        fn layers(
            &self,
            offset: u64,
            max_items: u64,
        ) -> Result<Vec<PreparationLayer>, PreparationInputError> {
            Ok(bounded(&self.layers, offset, max_items))
        }

        fn regions(
            &self,
            layer_index: u32,
            offset: u64,
            max_items: u64,
        ) -> Result<Vec<PreparationRegion>, PreparationInputError> {
            let empty = Vec::new();
            Ok(bounded(
                self.regions.get(&layer_index).unwrap_or(&empty),
                offset,
                max_items,
            ))
        }

        fn region_configs(
            &self,
            layer_index: u32,
            offset: u64,
            max_items: u64,
        ) -> Result<Vec<PreparationRegionConfig>, PreparationInputError> {
            let empty = Vec::new();
            Ok(bounded(
                self.configs.get(&layer_index).unwrap_or(&empty),
                offset,
                max_items,
            ))
        }

        unavailable_read!(surface_objects(offset: u64, max_items: u64) -> Option<Vec<slicer_sdk::preparation::PreparationSurfaceObject>>);
        unavailable_read!(surface_layers(layer_index: u32, offset: u64, max_items: u64) -> Option<Vec<slicer_sdk::preparation::PreparationSurfaceLayer>>);
        unavailable_read!(mesh_objects(offset: u64, max_items: u64) -> Vec<slicer_sdk::preparation::PreparationMeshObject>);
        unavailable_read!(mesh_vertices(object_id: &str, offset: u64, max_items: u64) -> Vec<slicer_ir::Point3>);
        unavailable_read!(mesh_triangles(object_id: &str, offset: u64, max_items: u64) -> Vec<(u32, u32, u32)>);
        unavailable_read!(mesh_queries(requests: &[slicer_sdk::preparation::PreparationMeshQuery]) -> Vec<slicer_sdk::preparation::PreparationMeshQueryResult>);
        unavailable_read!(seams(layer_index: u32, offset: u64, max_items: u64) -> Option<Vec<slicer_sdk::preparation::PreparationSeam>>);
        unavailable_read!(support_analysis(offset: u64, max_items: u64) -> Option<slicer_sdk::preparation::PreparationSupportAnalysisBatch>);
        unavailable_read!(support_geometry(offset: u64, max_items: u64) -> Option<slicer_sdk::preparation::PreparationSupportGeometryBatch>);
        unavailable_read!(support_plan(offset: u64, max_items: u64) -> Option<slicer_sdk::preparation::PreparationSupportPlanBatch>);
        unavailable_read!(selected_targets(layer_index: u32, offset: u64, max_items: u64) -> Vec<slicer_sdk::preparation::PreparationSelectedTarget>);
    }

    // -----------------------------------------------------------------------
    // Recording output + projection sink
    // -----------------------------------------------------------------------

    struct TestSink {
        requested: Vec<String>,
        scenes: Mutex<Vec<PreparationProjectionScene>>,
    }

    impl PreparationProjectionWrite for TestSink {
        fn requested_views(&self) -> Result<Vec<String>, PreparationProjectionError> {
            Ok(self.requested.clone())
        }

        fn publish(
            &self,
            scene: &PreparationProjectionScene,
        ) -> Result<(), PreparationProjectionError> {
            self.scenes
                .lock()
                .expect("scene log poisoned")
                .push(scene.clone());
            Ok(())
        }
    }

    struct Recorded {
        pieces: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
        sink: Arc<TestSink>,
        output: PreparedPlanOutput,
    }

    fn recorded_output(requested: &[&str]) -> Recorded {
        let pieces = Arc::new(Mutex::new(Vec::new()));
        let staged = Arc::clone(&pieces);
        let sink = Arc::new(TestSink {
            requested: requested.iter().map(|view| (*view).to_string()).collect(),
            scenes: Mutex::new(Vec::new()),
        });
        let projection_sink: Arc<dyn PreparationProjectionWrite + Send + Sync> =
            Arc::clone(&sink) as Arc<dyn PreparationProjectionWrite + Send + Sync>;
        let output = PreparedPlanOutput::from_callbacks(
            move |name, bytes| {
                staged
                    .lock()
                    .expect("piece log poisoned")
                    .push((name.to_string(), bytes.to_vec()));
                Ok(())
            },
            move || {
                Ok(Some(PreparationProjectionOutput::from_host_writer(
                    Arc::clone(&projection_sink),
                )))
            },
        );
        Recorded {
            pieces,
            sink,
            output,
        }
    }

    fn prepare(fixture: Fixture) -> (Arc<Mutex<Vec<(String, Vec<u8>)>>>, Arc<TestSink>) {
        let mut recorded = recorded_output(&[crate::projection::VIEW]);
        let input = PrintPreparationInput::from_host_input(Arc::new(fixture));
        LightningInfill::prepare_print(&input, &mut recorded.output)
            .expect("fixture preparation succeeds");
        // The output stays alive until the staged writes are read back, so every
        // put happened through its callbacks.
        (recorded.pieces, recorded.sink)
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    /// The `none` path is the normal plan: one piece per committed schedule
    /// layer, header-only where the prism seeds no trees, and one scene derived
    /// from the same plan.
    #[test]
    fn none_publishes_the_normal_pieces_and_the_derived_scene() {
        let (pieces, sink) = prepare(Fixture::prism(Some(&["none", "none", "none", "none"])));
        let pieces = pieces.lock().expect("piece log poisoned");
        assert_eq!(pieces.len(), 4, "one piece per schedule layer");
        let mut plan_edges: BTreeMap<i32, Vec<[Point2; 2]>> = BTreeMap::new();
        for (index, (name, body)) in pieces.iter().enumerate() {
            assert_eq!(name, &format!("lightning-layer-{index}"));
            let plan = decode_layer_piece(name, body).expect("normal pieces decode");
            if index == 3 {
                assert!(
                    !plan.entries.is_empty(),
                    "the 12 mm prism layer carries the fixture's only trees"
                );
            } else {
                assert!(
                    plan.entries.is_empty(),
                    "lower prism layers are inside the layer below, so they seed nothing"
                );
                assert_eq!(body.len(), 10, "a tree-less layer is a header-only piece");
            }
            for entry in &plan.entries {
                plan_edges
                    .entry(entry.global_layer_index)
                    .or_default()
                    .extend(entry.tree_edge_segments.iter().copied());
            }
        }

        let scenes = sink.scenes.lock().expect("scene log poisoned");
        assert_eq!(scenes.len(), 1, "one whole-print scene per preparation");
        let scene = &scenes[0];
        assert_eq!(scene.scene_id, crate::projection::SCENE_ID);
        assert_eq!(scene.view, crate::projection::VIEW);
        assert_eq!(scene.classes.len(), 4);

        // The scene's branches are exactly the plan's edges, per layer: the
        // projection derives from the plan the pieces encode, not from a second
        // planning pass.
        let mut scene_edges: BTreeMap<i32, Vec<[Point2; 2]>> = BTreeMap::new();
        let mut roots = 0usize;
        let mut groundings = 0usize;
        let mut domains = 0usize;
        let mut ids = std::collections::BTreeSet::new();
        for primitive in &scene.primitives {
            assert!(
                ids.insert(primitive.id.clone()),
                "primitive ids are unique: {}",
                primitive.id
            );
            match (&primitive.class[..], &primitive.geometry) {
                ("tree_branch", PreparationProjectionGeometry::Polyline(points)) => {
                    assert_eq!(points.len(), 2);
                    scene_edges
                        .entry(primitive.global_layer_index as i32)
                        .or_default()
                        .push([points[0], points[1]]);
                }
                ("tree_root", PreparationProjectionGeometry::Point(_)) => roots += 1,
                ("grounding", PreparationProjectionGeometry::Point(_)) => groundings += 1,
                ("planning_domain", PreparationProjectionGeometry::Polygon { .. }) => domains += 1,
                (class, geometry) => panic!("class '{class}' carries {geometry:?}"),
            }
        }
        assert_eq!(scene_edges, plan_edges, "branches are the plan's edges");
        assert!(domains > 0, "the committed planning domains are projected");
        assert!(roots > 0, "the plan's directed runs have a root side");
        assert!(
            groundings > 0,
            "each root projects its wall attachment on the planning domain"
        );
    }

    /// Capture-off: the same normal plan was not asked for the view, so no scene
    /// is derived or published and the pieces are unchanged.
    #[test]
    fn none_publishes_no_scene_when_the_view_was_not_requested() {
        let mut recorded = recorded_output(&[]);
        let input = PrintPreparationInput::from_host_input(Arc::new(Fixture::prism(Some(&[
            "none", "none", "none", "none",
        ]))));
        LightningInfill::prepare_print(&input, &mut recorded.output)
            .expect("normal preparation succeeds");
        assert!(
            recorded
                .sink
                .scenes
                .lock()
                .expect("scene log poisoned")
                .is_empty(),
            "capture-off must not publish a scene"
        );
        assert_eq!(recorded.pieces.lock().unwrap().len(), 4);
    }

    /// `missing_piece` omits the topmost layer that carries plan entries, not the
    /// topmost scheduled layer: the tower's fifth layer is supported and seeds no
    /// trees, so its consumer would never ask for a *plan* entry, while layer 3's
    /// consumer always runs.
    #[test]
    fn missing_piece_omits_the_topmost_planned_layer() {
        let (pieces, _) = prepare(Fixture::tower(Some(&[
            "missing_piece",
            "missing_piece",
            "missing_piece",
            "missing_piece",
            "missing_piece",
        ])));
        let pieces = pieces.lock().expect("piece log poisoned");
        let names: Vec<&str> = pieces.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "lightning-layer-0",
                "lightning-layer-1",
                "lightning-layer-2",
                "lightning-layer-4"
            ],
            "layer 3 carries the plan's topmost entries, so its piece is absent"
        );
        for (name, body) in pieces.iter() {
            decode_layer_piece(name, body).expect("the remaining pieces are valid");
        }
    }

    /// `malformed_piece` publishes a body the module decoder rejects with `57410`,
    /// not the unsupported-version code.
    #[test]
    fn malformed_piece_is_57410() {
        let (pieces, _) = prepare(Fixture::prism(Some(&[
            "malformed_piece",
            "malformed_piece",
            "malformed_piece",
            "malformed_piece",
        ])));
        let pieces = pieces.lock().expect("piece log poisoned");
        assert_eq!(pieces.len(), 4);
        for (name, body) in pieces.iter() {
            let error = decode_layer_piece(name, body)
                .expect_err("a malformed diagnostic body must not decode");
            assert_eq!(error.code, 57410, "{name}: {}", error.message);
            assert!(error.fatal);
            assert_eq!(
                error.message,
                format!("malformed lightning plan piece: {name}")
            );
        }
    }

    /// `unsupported_version` publishes a well-formed header carrying version 2.
    #[test]
    fn unsupported_version_is_57411() {
        let (pieces, _) = prepare(Fixture::prism(Some(&[
            "unsupported_version",
            "unsupported_version",
            "unsupported_version",
            "unsupported_version",
        ])));
        let pieces = pieces.lock().expect("piece log poisoned");
        assert_eq!(pieces.len(), 4);
        for (name, body) in pieces.iter() {
            // It really is a well-formed header: magic, version 2, zero entries.
            assert_eq!(&body[..4], b"LTPL");
            assert_eq!(u16::from_le_bytes([body[4], body[5]]), 2);
            assert_eq!(u32::from_le_bytes(body[6..10].try_into().unwrap()), 0);
            let error =
                decode_layer_piece(name, body).expect_err("an unsupported version must not decode");
            assert_eq!(error.code, 57411, "{name}: {}", error.message);
            assert!(error.fatal);
            assert_eq!(error.message, "unsupported lightning plan piece version: 2");
        }
    }

    /// `empty_plan` publishes one empty plan per layer plus the explicit empty
    /// scene: four classes, zero primitives, the declared view.
    #[test]
    fn empty_plan_publishes_empty_pieces_and_the_empty_scene() {
        let (pieces, sink) = prepare(Fixture::prism(Some(&[
            "empty_plan",
            "empty_plan",
            "empty_plan",
            "empty_plan",
        ])));
        let pieces = pieces.lock().expect("piece log poisoned");
        assert_eq!(pieces.len(), 4);
        for (name, body) in pieces.iter() {
            let plan = decode_layer_piece(name, body).expect("empty pieces decode");
            assert!(plan.entries.is_empty());
            assert_eq!(body.len(), 10);
        }
        let scenes = sink.scenes.lock().expect("scene log poisoned");
        assert_eq!(scenes.len(), 1, "exactly one empty scene per preparation");
        let scene = &scenes[0];
        assert_eq!(scene.scene_id, crate::projection::SCENE_ID);
        assert_eq!(scene.view, crate::projection::VIEW);
        assert_eq!(
            scene.classes,
            crate::projection::CLASSES
                .iter()
                .map(|class| (*class).to_string())
                .collect::<Vec<_>>()
        );
        assert!(
            scene.primitives.is_empty(),
            "the empty scene declares its classes and no primitives"
        );
    }

    /// Capture-off: an `empty_plan` run that was not asked for the view
    /// allocates and publishes no scene.
    #[test]
    fn empty_plan_publishes_nothing_when_the_view_was_not_requested() {
        let mut recorded = recorded_output(&[]);
        let input = PrintPreparationInput::from_host_input(Arc::new(Fixture::prism(Some(&[
            "empty_plan",
            "empty_plan",
            "empty_plan",
            "empty_plan",
        ]))));
        LightningInfill::prepare_print(&input, &mut recorded.output)
            .expect("empty preparation succeeds");
        assert!(
            recorded
                .sink
                .scenes
                .lock()
                .expect("scene log poisoned")
                .is_empty(),
            "capture-off must not publish a scene"
        );
        assert_eq!(recorded.pieces.lock().unwrap().len(), 4);
    }

    /// The diagnostic key is excluded from the five-scalar agreement check: a
    /// per-layer difference in it must not produce `57412`, and the first
    /// committed config's mode is the one applied.
    #[test]
    fn per_layer_diagnostic_difference_is_not_a_config_disagreement() {
        let (pieces, _) = prepare(Fixture::prism(Some(&[
            "none",
            "empty_plan",
            "missing_piece",
            "malformed_piece",
        ])));
        let pieces = pieces.lock().expect("piece log poisoned");
        assert_eq!(pieces.len(), 4, "the first config's `none` mode applies");
        let top =
            decode_layer_piece("lightning-layer-3", &pieces[3].1).expect("the normal plan is used");
        assert!(!top.entries.is_empty());
    }

    /// An unknown spelling is a contract defect, never a silent `none`.
    #[test]
    fn unknown_diagnostic_value_is_fatal() {
        let mut recorded = recorded_output(&[crate::projection::VIEW]);
        let input = PrintPreparationInput::from_host_input(Arc::new(Fixture::prism(Some(&[
            "not_a_mode",
            "not_a_mode",
            "not_a_mode",
            "not_a_mode",
        ]))));
        let error = LightningInfill::prepare_print(&input, &mut recorded.output)
            .expect_err("an unknown diagnostic value must fail preparation");
        assert!(error.fatal);
        assert!(
            error.message.contains(DIAGNOSTIC_KEY) && error.message.contains("not_a_mode"),
            "the error names the key and the value: {}",
            error.message
        );
        assert!(
            recorded.pieces.lock().unwrap().is_empty(),
            "nothing is published when the mode cannot be read"
        );
    }

    /// A committed region config that lost the declared key is a contract
    /// defect: the mode is never silently assumed.
    #[test]
    fn absent_diagnostic_key_is_fatal() {
        let mut recorded = recorded_output(&[crate::projection::VIEW]);
        let input = PrintPreparationInput::from_host_input(Arc::new(Fixture::prism(None)));
        let error = LightningInfill::prepare_print(&input, &mut recorded.output)
            .expect_err("the declared key must be present in a committed config");
        assert!(error.fatal);
        assert!(
            error.message.contains(DIAGNOSTIC_KEY),
            "the error names the missing key: {}",
            error.message
        );
    }
}
