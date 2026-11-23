//! Literal committed-IR seeds and the real preparation boundary, not a mock reader.
#![cfg(feature = "preparation-test-fixtures")]
#![allow(dead_code, clippy::field_reassign_with_default)]

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::{Arc, Once},
};

use slicer_ir::*;
use slicer_sdk::traits::LayerModule;
use slicer_wasm_host::preparation::{
    freeze_native, freeze_wasm, CommittedPreparationInputs, FrozenPreparationBinding,
    NativePreparationArtifact, PreparationFailure, PreparationInputData, PreparationInputError,
    PreparationOwner, PreparationSelectionContext, PreparedOwnerStore, PreparedPlanHandle,
    PreparedPlanMetadata, StagedPreparation,
};

#[derive(Debug, Clone, Copy)]
pub enum Package {
    Path,
    Perimeters,
    Infill,
    Support,
    Anchored,
}

impl Package {
    pub fn name(self) -> &'static str {
        match self {
            Self::Path => "whole-print-input-path",
            Self::Perimeters => "whole-print-input-perimeters",
            Self::Infill => "whole-print-input-infill",
            Self::Support => "whole-print-input-support",
            Self::Anchored => "whole-print-input-anchored",
        }
    }

    pub fn stage(self) -> &'static str {
        match self {
            Self::Path => "Layer::PathOptimization",
            Self::Perimeters => "Layer::Perimeters",
            Self::Infill => "Layer::Infill",
            Self::Support => "Layer::Support",
            Self::Anchored => "Layer::AnchoredEvents",
        }
    }

    pub fn manifest_path(self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-guests")
            .join(self.name())
            .join("module.toml")
    }

    pub fn manifest_text(self) -> String {
        std::fs::read_to_string(self.manifest_path())
            .expect("required whole-print fixture manifest")
    }

    pub fn manifest(self) -> toml::Value {
        toml::from_str(&self.manifest_text()).expect("valid whole-print fixture manifest")
    }

    pub fn module_id(self) -> String {
        self.manifest()["module"]["id"]
            .as_str()
            .expect("fixture module id")
            .into()
    }

    pub fn claims(self) -> Vec<String> {
        self.manifest()["claims"]["holds"]
            .as_array()
            .expect("fixture claims")
            .iter()
            .map(|claim| claim.as_str().expect("string claim").into())
            .collect()
    }

    pub fn artifact(self) -> PathBuf {
        super::preparation_transport_driver::required_artifact(&format!(
            "crates/slicer-wasm-host/test-guests/{}.component.wasm",
            self.name()
        ))
    }

    fn native_artifact(self) -> NativePreparationArtifact {
        macro_rules! prepared {
            ($module:ty) => {
                NativePreparationArtifact::Prepared {
                    entry: <$module>::__slicer_prepared_native_entry(),
                    schema: <$module>::__SLICER_PREPARATION_SCHEMA,
                }
            };
        }
        match self {
            Self::Path => prepared!(whole_print_input_path::WholePrintInputPath),
            Self::Perimeters => prepared!(whole_print_input_perimeters::WholePrintInputPerimeters),
            Self::Infill => prepared!(whole_print_input_infill::WholePrintInputInfill),
            Self::Support => prepared!(whole_print_input_support::WholePrintInputSupport),
            Self::Anchored => prepared!(whole_print_input_anchored::WholePrintInputAnchored),
        }
    }

    fn configure_native(self, config: &ConfigView) -> Result<(), PreparationFailure> {
        let result = match self {
            Self::Path => {
                whole_print_input_path::WholePrintInputPath::from_config(config).map(|_| ())
            }
            Self::Perimeters => {
                whole_print_input_perimeters::WholePrintInputPerimeters::from_config(config)
                    .map(|_| ())
            }
            Self::Infill => {
                whole_print_input_infill::WholePrintInputInfill::from_config(config).map(|_| ())
            }
            Self::Support => {
                whole_print_input_support::WholePrintInputSupport::from_config(config).map(|_| ())
            }
            Self::Anchored => {
                whole_print_input_anchored::WholePrintInputAnchored::from_config(config).map(|_| ())
            }
        };
        result.map_err(PreparationFailure::Module)
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum InsertionOrder {
    #[default]
    Canonical,
    ReversedSlices,
    ReversedMap,
    ReversedObjects,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum InvalidInputs {
    #[default]
    None,
    DuplicateScheduleIndex,
    MissingScheduleSlice,
    DuplicateFullKey,
    NonfiniteZ,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum OwnerReadScenario {
    #[default]
    Own,
    CrossOwner,
    DisposedOwner,
}

#[derive(Default, Clone)]
pub enum Product<T> {
    #[default]
    Literal,
    Absent,
    Empty,
    Value(Arc<T>),
}

impl<T: Default> Product<T> {
    fn choose(&self, literal: T) -> Option<Arc<T>> {
        match self {
            Self::Literal => Some(Arc::new(literal)),
            Self::Absent => None,
            Self::Empty => Some(Arc::new(T::default())),
            Self::Value(value) => Some(value.clone()),
        }
    }
}

#[derive(Clone)]
pub struct Scenario {
    pub module_widths: [f64; 3],
    pub invocation_config: BTreeMap<String, ConfigValue>,
    pub paint_semantics: HashSet<String>,
    pub surface: Product<SurfaceClassificationIR>,
    pub seam: Product<SeamPlanIR>,
    pub support_analysis: Product<SupportAnalysisIR>,
    pub support_geometry: Product<SupportGeometryIR>,
    pub support_plan: Product<SupportPlanIR>,
    pub order: InsertionOrder,
    /// Dedicated Custom seeds with independent (slice, map) insertion reversals.
    pub custom_cube: Option<(bool, bool)>,
    pub invalid: InvalidInputs,
    /// None uses exactly the package's on-disk preparation declaration.
    pub input_reads: Option<Vec<String>>,
    pub batch_ops: String,
    pub fixture_mode: String,
    pub counter_seed: Option<(u64, u64)>,
    pub owner_reads: OwnerReadScenario,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            module_widths: [0.41, 0.62, 0.73],
            invocation_config: BTreeMap::from([
                ("line_width".into(), ConfigValue::Float(0.8)),
                ("private_probe".into(), ConfigValue::Int(99)),
            ]),
            paint_semantics: HashSet::new(),
            surface: Product::Literal,
            seam: Product::Literal,
            support_analysis: Product::Literal,
            support_geometry: Product::Literal,
            support_plan: Product::Literal,
            order: InsertionOrder::Canonical,
            custom_cube: None,
            invalid: InvalidInputs::None,
            input_reads: None,
            batch_ops: String::new(),
            fixture_mode: "standard".into(),
            counter_seed: None,
            owner_reads: OwnerReadScenario::Own,
        }
    }
}

impl Scenario {
    /// Explicit override for absent/present-empty optional-product witnesses.
    /// The default remains the manifest's exact required/optional token set.
    pub fn with_optional_reads(mut self, package: Package) -> Self {
        let declaration =
            slicer_scheduler::preparation::parse_preparation_declaration(&package.manifest_text())
                .expect("valid fixture preparation declaration")
                .expect("fixture declaration");
        self.input_reads = Some(
            declaration
                .input_reads()
                .iter()
                .map(|read| {
                    if matches!(
                        read.as_str(),
                        "surface_classification"
                            | "seam_plan"
                            | "support_analysis"
                            | "support_geometry"
                            | "support_plan"
                    ) {
                        format!("{read}_optional")
                    } else {
                        read.clone()
                    }
                })
                .collect(),
        );
        self
    }

    /// WASM fixtures discover controls through declared layer/config reads, not
    /// through a test-only guest global. Opt in explicitly when a restricted
    /// package needs nonstandard controls; permission tests retain exact grants.
    pub fn with_control_discovery(mut self, package: Package) -> Self {
        let reads = self.input_reads.get_or_insert_with(|| {
            slicer_scheduler::preparation::parse_preparation_declaration(&package.manifest_text())
                .expect("valid fixture declaration")
                .expect("fixture preparation")
                .input_reads()
                .to_vec()
        });
        for read in ["layer_plan", "region_map"] {
            if !reads.iter().any(|existing| existing == read) {
                reads.push(read.into());
            }
        }
        self
    }

    pub fn invocation(&self, package: Package) -> Arc<ConfigView> {
        let mut fields = self.invocation_config.clone();
        fields.insert(
            "whole_print_fixture_mode".into(),
            ConfigValue::String(self.fixture_mode.clone()),
        );
        fields.insert(
            "whole_print_batch_ops".into(),
            ConfigValue::String(self.batch_ops.clone()),
        );
        let manifest = package.manifest();
        let declared = manifest["config"]["schema"]
            .as_table()
            .expect("fixture declared config");
        // A private input exists in resolved authority, but is never a declared
        // invocation key. The production projection remains the transport oracle.
        fields.retain(|key, _| declared.contains_key(key));
        Arc::new(ConfigView::from_map(fields.into_iter().collect()))
    }

    pub fn selection(&self, package: Package, claims: &[&str]) -> PreparationSelectionContext {
        let claims = match package {
            Package::Perimeters => &["outer", "inner", "support-family:traditional"][..],
            Package::Infill => &[
                "claim:top-fill",
                "claim:sparse-fill",
                "support-family:traditional",
            ][..],
            _ => claims,
        };
        PreparationSelectionContext::new(
            package.stage().into(),
            package.module_id(),
            claims.iter().map(|claim| (*claim).into()).collect(),
            self.paint_semantics.clone(),
            self.invocation(package),
        )
    }
}

pub fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

pub fn variant_key(tool: u32) -> RegionKey {
    RegionKey {
        global_layer_index: 7,
        object_id: "variant-cube".into(),
        region_id: 7,
        variant_chain: vec![("material".into(), PaintValue::ToolIndex(tool))],
    }
}

pub fn modifier_key() -> RegionKey {
    let footprint = rectangle(0.25, 0.25, 0.75, 0.75);
    RegionKey {
        region_id: slicer_ir::modifier_sub_region_id(7, "variant-cube", &[footprint]),
        ..variant_key(1)
    }
}

pub struct LiteralInputs {
    pub mesh: Arc<MeshIR>,
    pub layers: Arc<LayerPlanIR>,
    pub slices: Arc<Vec<SliceIR>>,
    pub region_map: Arc<RegionMapIR>,
    pub surface: Option<Arc<SurfaceClassificationIR>>,
    pub seam: Option<Arc<SeamPlanIR>>,
    pub support_analysis: Option<Arc<SupportAnalysisIR>>,
    pub support_geometry: Option<Arc<SupportGeometryIR>>,
    pub support_plan: Option<Arc<SupportPlanIR>>,
}

impl LiteralInputs {
    pub fn commit(&self) -> Result<Arc<CommittedPreparationInputs>, PreparationInputError> {
        CommittedPreparationInputs::new(
            self.mesh.clone(),
            self.layers.clone(),
            self.slices.clone(),
            self.region_map.clone(),
            self.surface.clone(),
            self.seam.clone(),
            self.support_analysis.clone(),
            self.support_geometry.clone(),
            self.support_plan.clone(),
        )
        .map(Arc::new)
    }

    pub fn retains_arc_identity(&self, committed: &CommittedPreparationInputs) -> bool {
        committed.controlled_retains_inputs(
            &self.mesh,
            &self.layers,
            &self.slices,
            &self.region_map,
            self.surface.as_ref(),
            self.seam.as_ref(),
            self.support_analysis.as_ref(),
            self.support_geometry.as_ref(),
            self.support_plan.as_ref(),
        )
    }
}

fn selected_region(tool: u32, dx: f32) -> SlicedRegion {
    let key = variant_key(tool);
    let mut region = SlicedRegion {
        object_id: key.object_id,
        region_id: key.region_id,
        variant_chain: key.variant_chain,
        effective_layer_height: 0.2,
        polygons: vec![rectangle(dx, 0.0, dx + 12.0, 3.0)],
        top_shell_index: Some(0),
        bottom_shell_index: Some(1),
        top_solid_fill: vec![rectangle(dx, 0.0, dx + 2.0, 2.0)],
        bottom_solid_fill: vec![rectangle(dx + 3.0, 0.0, dx + 5.0, 2.0)],
        internal_solid_fill: vec![rectangle(dx + 6.0, 0.0, dx + 8.0, 2.0)],
        bridge_areas: vec![rectangle(dx + 9.0, 0.0, dx + 11.0, 2.0)],
        internal_bridge_areas: vec![rectangle(dx + 9.0, 0.0, dx + 11.0, 2.0)],
        internal_bridge_angles_deg: vec![33.0],
        bridge_orientation_deg: 27.0,
        is_bridge: true,
        ..SlicedRegion::default()
    };
    region.polygons[0]
        .holes
        .push(rectangle(dx + 0.25, 0.25, dx + 0.75, 0.75).contour);
    region.segment_annotations.insert(
        PaintSemantic::Material,
        vec![vec![
            Some(PaintValue::ToolIndex(tool)),
            None,
            Some(PaintValue::Flag(true)),
            Some(PaintValue::Scalar(0.25)),
        ]],
    );
    region
}

fn resolved_config(
    package: Package,
    scenario: &Scenario,
    width: f64,
    selected: bool,
) -> ResolvedConfig {
    let holder = if selected {
        package.module_id()
    } else {
        "context-only-holder".into()
    };
    let mut config = ResolvedConfig {
        line_width: width,
        top_fill_holder: holder.clone(),
        bottom_fill_holder: holder.clone(),
        bridge_fill_holder: holder.clone(),
        sparse_fill_holder: holder,
        ..ResolvedConfig::default()
    };
    config
        .extensions
        .insert("private_probe".into(), ConfigValue::Int(99));
    config.extensions.insert(
        "whole_print_fixture_mode".into(),
        ConfigValue::String(scenario.fixture_mode.clone()),
    );
    config.extensions.insert(
        "whole_print_batch_ops".into(),
        ConfigValue::String(scenario.batch_ops.clone()),
    );
    config
}

pub fn literal_inputs(package: Package, scenario: &Scenario) -> LiteralInputs {
    if let Some((reverse_slices, reverse_map)) = scenario.custom_cube {
        return custom_cube_inputs(package, scenario, reverse_slices, reverse_map);
    }
    let mut first = selected_region(1, 0.0);
    let second = selected_region(2, 30.0);
    let modifier = SlicedRegion {
        object_id: "variant-cube".into(),
        region_id: modifier_key().region_id,
        variant_chain: variant_key(1).variant_chain,
        polygons: vec![rectangle(0.25, 0.25, 0.75, 0.75)],
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    };
    let context = SlicedRegion {
        object_id: "context-only".into(),
        region_id: 9,
        polygons: vec![rectangle(20.0, 0.0, 22.0, 2.0)],
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    };
    // Only the raft slice owns raft fill; model regions do not borrow it.
    let raft_region = SlicedRegion {
        object_id: "raft-context".into(),
        region_id: 0,
        raft_fill: vec![rectangle(-1.0, -1.0, 13.0, 4.0)],
        ..SlicedRegion::default()
    };
    // Explicitly empty sparse fill is a literal fixture property, not a result
    // derived from selection or from a preparation reader.
    first.sparse_infill_area.clear();
    let mut slices = vec![
        SliceIR {
            global_layer_index: 0,
            z: 0.1,
            regions: vec![raft_region],
            ..SliceIR::default()
        },
        SliceIR {
            global_layer_index: 7,
            z: 0.3,
            regions: vec![first, second, modifier, context],
            ..SliceIR::default()
        },
    ];
    let mut map = RegionMapIR::default();
    let mut configs = [
        resolved_config(package, scenario, scenario.module_widths[0], true),
        resolved_config(package, scenario, scenario.module_widths[1], true),
        resolved_config(package, scenario, scenario.module_widths[2], false),
    ];
    if matches!(package, Package::Perimeters | Package::Infill) {
        for (config, family) in configs
            .iter_mut()
            .zip(["traditional", "traditional", "tree"])
        {
            config
                .extensions
                .insert("support_family".into(), ConfigValue::String(family.into()));
        }
    }
    if matches!(package, Package::Support | Package::Anchored) {
        // The non-model fixtures retain all slice context, but its model
        // addresses must not satisfy the traditional-family module claim.
        for config in &mut configs {
            config
                .extensions
                .insert("support_family".into(), ConfigValue::String("tree".into()));
        }
    }
    if matches!(package, Package::Infill) {
        // Author distinct holder authority, not a different observed answer:
        // resolve_held_claims must reject this module for variant two's top role
        // while retaining its sparse role. Raw manifest tokens remain unchanged;
        // the resolver itself recognizes canonical claim:* fill tokens only.
        configs[1].top_fill_holder = "com.test.variant-two-top-holder".into();
    }
    // Intern before permuting insertion so config handles do not accidentally
    // become the independent-order witness under test.
    let ids: Vec<_> = configs
        .iter()
        .cloned()
        .map(|config| map.intern_config(config))
        .collect();
    let mut entries = Vec::new();
    for slice in &slices {
        for region in &slice.regions {
            let index = if region.object_id != "variant-cube" {
                2
            } else if region.variant_chain == variant_key(2).variant_chain {
                1
            } else {
                0
            };
            let key = slicer_wasm_host::selection::region_map_key_for_sliced_region(
                slice.global_layer_index,
                region,
            );
            entries.push((
                key,
                RegionPlan {
                    config: ids[index],
                    ..RegionPlan::default()
                },
            ));
        }
    }
    if matches!(scenario.order, InsertionOrder::ReversedMap) {
        entries.reverse();
    }
    map.entries.extend(entries);
    let mut layers = LayerPlanIR::default();
    layers.global_layers = vec![
        GlobalLayer {
            index: 0,
            z: 0.1,
            is_raft: true,
            ..GlobalLayer::default()
        },
        GlobalLayer {
            index: 7,
            z: 0.3,
            active_regions: vec![ActiveRegion {
                object_id: "variant-cube".into(),
                region_id: 7,
                resolved_config: configs[0].clone(),
                effective_layer_height: 0.2,
                ..ActiveRegion::default()
            }],
            ..GlobalLayer::default()
        },
    ];
    match scenario.invalid {
        InvalidInputs::None => {}
        InvalidInputs::DuplicateScheduleIndex => layers.global_layers[1].index = 0,
        InvalidInputs::MissingScheduleSlice => {
            slices.remove(1);
        }
        InvalidInputs::DuplicateFullKey => {
            let duplicate = slices[1].regions[0].clone();
            slices[1].regions.push(duplicate);
        }
        InvalidInputs::NonfiniteZ => layers.global_layers[1].z = f32::NAN,
    }
    if matches!(scenario.order, InsertionOrder::ReversedSlices) {
        slices.reverse();
        for slice in &mut slices {
            slice.regions.reverse();
        }
    }

    let triangle = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 2.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 2.0,
            z: 1.0,
        },
    ];
    let identity = Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };
    let mut object = ObjectMesh::default();
    object.id = "variant-cube".into();
    object.mesh.vertices = triangle.clone();
    object.mesh.indices = vec![0, 1, 2];
    object.transform = identity;
    let paint = object.paint_data.get_or_insert_with(Default::default);
    for (semantic, value) in [
        (PaintSemantic::Material, PaintValue::ToolIndex(1)),
        (
            PaintSemantic::Custom("fixture_flag".into()),
            PaintValue::Flag(true),
        ),
        (
            PaintSemantic::Custom("fixture_scalar".into()),
            PaintValue::Scalar(0.25),
        ),
    ] {
        paint.layers.push(PaintLayer {
            semantic: semantic.clone(),
            facet_values: vec![Some(value.clone())],
            strokes: vec![PaintStroke {
                semantic,
                value,
                triangles: vec![[triangle[0], triangle[1], triangle[2]]],
            }],
        });
    }
    let mut translated = object.clone();
    translated.id = "translated".into();
    translated.transform.matrix[12] = 10.0;
    translated.transform.matrix[13] = 20.0;
    translated.transform.matrix[14] = 3.0;
    let mut context_mesh = object.clone();
    context_mesh.id = "context-only".into();
    let mut empty = ObjectMesh::default();
    empty.id = "empty-mesh".into();
    empty.transform = identity;
    let mut mesh = MeshIR::default();
    mesh.objects = vec![object, context_mesh, translated, empty];
    if matches!(scenario.order, InsertionOrder::ReversedObjects) {
        mesh.objects.reverse();
    }

    let surface = literal_surface(scenario.order);
    let seam = literal_seam();
    let (support_analysis, support_geometry, support_plan) = literal_support(scenario.order);
    LiteralInputs {
        mesh: Arc::new(mesh),
        layers: Arc::new(layers),
        slices: Arc::new(slices),
        region_map: Arc::new(map),
        surface: scenario.surface.choose(surface),
        seam: scenario.seam.choose(seam),
        support_analysis: scenario.support_analysis.choose(support_analysis),
        support_geometry: scenario.support_geometry.choose(support_geometry),
        support_plan: scenario.support_plan.choose(support_plan),
    }
}

fn custom_cube_inputs(
    package: Package,
    scenario: &Scenario,
    reverse_slices: bool,
    reverse_map: bool,
) -> LiteralInputs {
    let chain = |route: &str| {
        vec![
            ("flag_probe".into(), PaintValue::Flag(true)),
            ("scalar_probe".into(), PaintValue::Scalar(-0.0)),
            ("material".into(), PaintValue::ToolIndex(1)),
            ("custom_route".into(), PaintValue::Custom(route.into())),
        ]
    };
    let footprint = vec![rectangle(0.25, 0.25, 0.75, 0.75)];
    let modifier_id = slicer_ir::modifier_sub_region_id(7, "custom-cube", &footprint);
    let mut regions = Vec::new();
    for (region_id, route, polygons) in [
        (7, "A/π", vec![rectangle(0.0, 0.0, 2.0, 2.0)]),
        (7, "a/π", vec![rectangle(3.0, 0.0, 5.0, 2.0)]),
        (modifier_id, "A/π", footprint),
    ] {
        let mut region = SlicedRegion {
            object_id: "custom-cube".into(),
            region_id,
            variant_chain: chain(route),
            sparse_infill_area: polygons.clone(),
            polygons,
            effective_layer_height: 0.2,
            ..SlicedRegion::default()
        };
        region.segment_annotations.insert(
            PaintSemantic::Custom("annotation_probe".into()),
            vec![vec![
                Some(PaintValue::Custom("annotation/π".into())),
                None,
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Scalar(-0.0)),
            ]],
        );
        regions.push(region);
    }

    let mut map = RegionMapIR::default();
    let configs: Vec<_> = scenario.module_widths[..2]
        .iter()
        .map(|width| {
            let mut config = resolved_config(package, scenario, *width, true);
            // The Custom witness holds sparse fill only, regardless of the
            // fixture package's additional top/support-family manifest claims.
            config.top_fill_holder = "context-only-holder".into();
            config.bottom_fill_holder = "context-only-holder".into();
            config.bridge_fill_holder = "context-only-holder".into();
            config
        })
        .collect();
    let ids: Vec<_> = configs
        .iter()
        .cloned()
        .map(|config| map.intern_config(config))
        .collect();
    let mut entries: Vec<_> = regions
        .iter()
        .zip([ids[0], ids[1], ids[0]])
        .map(|(region, config)| {
            (
                slicer_wasm_host::selection::region_map_key_for_sliced_region(7, region),
                RegionPlan {
                    config,
                    ..RegionPlan::default()
                },
            )
        })
        .collect();
    if reverse_map || matches!(scenario.order, InsertionOrder::ReversedMap) {
        entries.reverse();
    }
    map.entries.extend(entries);

    let mut seam = SeamPlanIR::default();
    let chosen = literal_seam().entries.remove(0);
    for region in &regions {
        let mut entry = chosen.clone();
        entry.region_key = RegionKey {
            global_layer_index: 7,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region.variant_chain.clone(),
        };
        seam.entries.push(entry);
    }
    if reverse_slices || matches!(scenario.order, InsertionOrder::ReversedSlices) {
        regions.reverse();
        seam.entries.reverse();
    }
    let mut layers = LayerPlanIR::default();
    layers.global_layers = vec![GlobalLayer {
        index: 7,
        z: 0.3,
        active_regions: vec![ActiveRegion {
            object_id: "custom-cube".into(),
            region_id: 7,
            resolved_config: configs[0].clone(),
            effective_layer_height: 0.2,
            ..ActiveRegion::default()
        }],
        ..GlobalLayer::default()
    }];
    let slices = vec![SliceIR {
        global_layer_index: 7,
        z: 0.3,
        regions,
        ..SliceIR::default()
    }];

    // Point3 positions are already millimeters; rectangle converts scaled XY.
    let triangle = [
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 2.0,
            y: 0.0,
            z: 1.0,
        },
        Point3 {
            x: 0.0,
            y: 2.0,
            z: 1.0,
        },
    ];
    let mut object = ObjectMesh::default();
    object.id = "custom-cube".into();
    object.mesh.vertices = triangle.to_vec();
    object.mesh.indices = vec![0, 1, 2];
    object.transform = Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };
    let semantic = PaintSemantic::Custom("mesh_probe".into());
    let value = PaintValue::Custom("mesh/π".into());
    object
        .paint_data
        .get_or_insert_with(Default::default)
        .layers
        .push(PaintLayer {
            semantic: semantic.clone(),
            facet_values: vec![Some(value.clone())],
            strokes: vec![PaintStroke {
                semantic,
                value,
                triangles: vec![triangle],
            }],
        });
    let mut mesh = MeshIR::default();
    mesh.objects = vec![object];
    LiteralInputs {
        mesh: Arc::new(mesh),
        layers: Arc::new(layers),
        slices: Arc::new(slices),
        region_map: Arc::new(map),
        surface: scenario.surface.choose(SurfaceClassificationIR::default()),
        seam: scenario.seam.choose(seam),
        support_analysis: scenario
            .support_analysis
            .choose(SupportAnalysisIR::default()),
        support_geometry: scenario
            .support_geometry
            .choose(SupportGeometryIR::default()),
        support_plan: scenario.support_plan.choose(SupportPlanIR::default()),
    }
}

pub fn literal_surface(order: InsertionOrder) -> SurfaceClassificationIR {
    let mut surface = SurfaceClassificationIR::default();
    let data = surface.per_object.entry("variant-cube".into()).or_default();
    // Six positions exercise the discriminant independently of triangle zero's
    // geometry; class projection must not collapse them to one representative.
    data.facet_classes = vec![
        FacetClass::Normal,
        FacetClass::NearHorizontal {
            slope_angle_deg: 12.0,
        },
        FacetClass::Overhang { angle_deg: 60.0 },
        FacetClass::Bridge,
        FacetClass::TopSurface,
        FacetClass::BottomSurface,
    ];
    data.surface_groups.push(Default::default());
    let group = data.surface_groups.last_mut().expect("literal group");
    group.id = 41;
    group.facet_indices = vec![0];
    group.z_min = 0.0;
    group.z_max = 1.0;
    group.area_mm2 = 2.0;
    group.printable = true;
    group.shell_count = 3;
    data.bridge_regions.push(Default::default());
    let bridge = data.bridge_regions.last_mut().expect("literal bridge");
    bridge.id = 42;
    bridge.facet_indices = vec![3];
    bridge.bridge_direction_deg = 27.0;
    bridge.anchor_width_mm = 0.5;
    bridge.bridge_length_mm = 2.0;
    bridge.expansion_margin_mm = 0.2;
    bridge.is_valid = true;
    bridge.xy_footprint = vec![rectangle(9.0, 0.0, 11.0, 2.0)];
    data.overhang_regions.push(Default::default());
    let overhang = data.overhang_regions.last_mut().expect("literal overhang");
    overhang.id = 43;
    overhang.facet_indices = vec![2];
    overhang.max_angle_deg = 60.0;
    overhang.needs_support = true;
    overhang.xy_footprint = vec![rectangle(6.0, 0.0, 8.0, 2.0)];
    surface.per_object.entry("context-only".into()).or_default();
    let mut quartiles = vec![
        ("variant-cube", 2, rectangle(0.0, 0.0, 2.0, 2.0)),
        ("context-only", 4, rectangle(20.0, 0.0, 22.0, 2.0)),
    ];
    if matches!(
        order,
        InsertionOrder::ReversedMap | InsertionOrder::ReversedObjects
    ) {
        quartiles.reverse();
    }
    for (object, quartile, polygon) in quartiles {
        surface
            .overhang_quartile_polygons
            .entry(object.into())
            .or_default()
            .entry(7)
            .or_default()
            .push(QuartileBand {
                quartile,
                polygons: vec![polygon],
            });
        surface
            .prev_layer_boundaries
            .entry(object.into())
            .or_default()
            .insert(7, vec![rectangle(-1.0, -1.0, 13.0, 4.0)]);
    }
    surface
}

pub fn literal_seam() -> SeamPlanIR {
    let mut seam = SeamPlanIR::default();
    seam.entries.push(Default::default());
    let entry = seam.entries.last_mut().expect("literal seam entry");
    entry.region_key = variant_key(1);
    // exhaustive: independent chosen-seam witness pins every transported point field.
    entry.chosen_candidate.point = Point3WithWidth {
        x: 1.0,
        y: 2.0,
        z: 0.3,
        width: 0.4,
        flow_factor: 0.9,
        overhang_quartile: Some(2),
        dist_to_top_mm: 0.7,
        overhang_distance_mm: Some(-0.2),
    };
    entry.chosen_candidate.wall_index = 1;
    entry.scored_candidates.push(ScoredSeamCandidate {
        position: Point3WithWidth {
            x: 1.5,
            y: 2.5,
            z: 0.3,
            width: 0.4,
            ..Point3WithWidth::default()
        },
        score: 0.25,
        reason: SeamReason::UserForced,
    });
    seam
}

pub fn literal_support(
    order: InsertionOrder,
) -> (SupportAnalysisIR, SupportGeometryIR, SupportPlanIR) {
    let mut analysis = SupportAnalysisIR::default();
    let key = SupportGeometryKey {
        global_support_layer_index: 7,
        object_id: "support-carrier".into(),
        region_id: 5,
        variant_chain: vec![("material".into(), PaintValue::ToolIndex(1))],
    };
    let other_geometry = SupportGeometryKey {
        global_support_layer_index: 8,
        ..key.clone()
    };
    let mut geometry_rows = vec![
        (key.clone(), vec![rectangle(1.0, 2.0, 3.0, 4.0)]),
        (other_geometry, vec![rectangle(5.0, 6.0, 7.0, 8.0)]),
    ];
    if matches!(order, InsertionOrder::ReversedMap) {
        geometry_rows.reverse();
    }
    for (key, polygons) in &geometry_rows {
        analysis
            .model_occupancy
            .insert(key.clone(), polygons.clone());
        analysis
            .cantilever_surfaces
            .insert(key.clone(), polygons.clone());
        analysis
            .termination_surfaces
            .insert(key.clone(), polygons.clone());
        analysis
            .support_territory
            .insert(key.clone(), polygons.clone());
    }
    // exhaustive: the support-candidate witness explicitly defines all five fields.
    let candidate = slicer_ir::slice_ir::SupportCandidate {
        id: 44,
        // exhaustive: candidate provenance has no Default; pin every source field.
        source: slicer_ir::slice_ir::SupportCandidateSource {
            object_id: key.object_id.clone(),
            region_id: key.region_id,
            global_layer_index: 7,
            z_units: 3000,
            variant_chain: key.variant_chain.clone(),
        },
        geometry: vec![rectangle(1.0, 2.0, 3.0, 4.0)],
        enforced: true,
        blocked: false,
    };
    analysis.candidates.push(candidate);
    analysis.family_assignments.insert(
        slicer_ir::slice_ir::SupportSourceKey {
            object_id: key.object_id.clone(),
            region_id: key.region_id,
            variant_chain: key.variant_chain.clone(),
        },
        "traditional".into(),
    );
    analysis
        .shared_settings
        .insert("fixture_setting".into(), "17".into());
    analysis.baseline_feasible_envelope = vec![rectangle(-2.0, -2.0, 14.0, 5.0)];
    let mut geometry = SupportGeometryIR::default();
    geometry.support_layer_height_mm = 0.2;
    geometry.support_top_z_distance_mm = 0.1;
    geometry.entries.extend(geometry_rows);

    let mut plan = SupportPlanIR::default();
    // exhaustive: no Default exists; this literal pins the complete support-carrier authority.
    plan.entries.push(SupportPlanEntry {
        global_layer_index: 7,
        object_id: key.object_id,
        region_id: key.region_id,
        variant_chain: key.variant_chain,
        family_id: "traditional".into(),
        demand_ids: vec!["d0".into()],
        body_ids: vec!["b0".into()],
        provenance: vec!["p0".into()],
        anchor_layer_index: 3,
        anchor_z: 3000,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![rectangle(1.0, 2.0, 3.0, 4.0)],
        }],
        skeleton: None,
        capabilities: Vec::new(),
        decline_reason: None,
    });
    let entry = plan.entries.last_mut().expect("literal support plan entry");
    let skeleton = entry.skeleton.get_or_insert_with(Default::default);
    skeleton.points = vec![Point3 {
        x: 1.0,
        y: 2.0,
        z: 0.3,
    }];
    skeleton.wall_counts = vec![2];
    let base = entry.clone();
    let mut wrong_global = base.clone();
    wrong_global.global_layer_index = 8;
    wrong_global.variant_chain = vec![("material".into(), PaintValue::ToolIndex(2))];
    wrong_global.provenance = vec!["wrong-global".into()];
    let mut wrong_family = base.clone();
    wrong_family.family_id = "tree".into();
    wrong_family.variant_chain = vec![("material".into(), PaintValue::ToolIndex(4))];
    wrong_family.provenance = vec!["wrong-family".into()];
    let mut declined = base.clone();
    declined.decline_reason = Some(SupportPlanDeclineReason::DeclinedPolicy);
    declined.variant_chain = vec![("material".into(), PaintValue::ToolIndex(3))];
    declined.provenance = vec!["declined".into()];
    let mut off_grid = base;
    off_grid.global_layer_index = -1;
    off_grid.variant_chain = vec![("material".into(), PaintValue::ToolIndex(5))];
    off_grid.provenance = vec!["off-grid".into()];
    plan.entries
        .extend([wrong_global, wrong_family, declined, off_grid]);
    let raft = plan.raft_plan.get_or_insert_with(Default::default);
    raft.raft_layers = 1;
    raft.raft_first_layer_density = 0.8;
    raft.base_raft_layers = 1;
    raft.interface_raft_layers = 0;
    (analysis, geometry, plan)
}

/// Like preparation_transport_tdd::transport_native_binding/transport_wasm_binding,
/// freeze explicit declarations, not a synthetic scheduler/runtime invocation.
/// Unlike its inline manifest, the default declaration is parsed from disk through
/// the same production parse_preparation_declaration entry point.
pub fn freeze(
    package: Package,
    scenario: &Scenario,
    wasm: bool,
) -> Result<FrozenPreparationBinding, PreparationFailure> {
    let declaration =
        slicer_scheduler::preparation::parse_preparation_declaration(&package.manifest_text())
            .map_err(PreparationFailure::Declaration)?
            .expect("fixture declares preparation");
    let declaration = if let Some(reads) = &scenario.input_reads {
        slicer_schema::preparation::PreparationDeclaration::new(
            declaration.interface(),
            reads.clone(),
            declaration.views().to_vec(),
        )
    } else {
        declaration
    };
    if wasm {
        let engine = super::preparation_transport_driver::engine();
        let component = wasmtime::component::Component::from_file(&engine, package.artifact())
            .expect("load real whole-print fixture component; no silence skip");
        freeze_wasm(
            package.module_id(),
            Some(declaration),
            engine,
            component,
            package.stage().into(),
        )
    } else {
        freeze_native(
            package.module_id(),
            Some(declaration),
            package.native_artifact(),
        )
    }
}

pub struct PreparedDriver {
    pub binding: FrozenPreparationBinding,
    pub store: PreparedOwnerStore,
    pub owner: PreparationOwner,
}

impl PreparedDriver {
    pub fn new(
        package: Package,
        scenario: &Scenario,
        wasm: bool,
    ) -> Result<Self, PreparationFailure> {
        let binding = freeze(package, scenario, wasm)?;
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding)?;
        Ok(Self {
            binding,
            store,
            owner,
        })
    }
}

/// No prepare call, no transaction and no fixture callback: negative tests own
/// this boundary and can inspect the fresh owner's real invocation metadata.
pub fn prevalidate(
    owner: &PreparationOwner,
    inputs: Arc<CommittedPreparationInputs>,
    selection: PreparationSelectionContext,
) -> Result<PreparationInputData, PreparationFailure> {
    PreparationInputData::from_committed(owner, inputs, selection)
}

pub fn set_input_counters(
    input: &PreparationInputData,
    calls: u64,
    records: u64,
) -> Result<(), PreparationInputError> {
    input.controlled_set_input_counters(calls, records)
}

pub fn input_counters(input: &PreparationInputData) -> Result<(u64, u64), PreparationInputError> {
    input.controlled_input_counters()
}

thread_local! {
    static CAPTURE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}
struct FixtureLogger;
impl log::Log for FixtureLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        CAPTURE.with(|capture| {
            if let Some(lines) = capture.borrow_mut().as_mut() {
                lines.push(record.args().to_string());
            }
        });
    }
    fn flush(&self) {}
}
static LOGGER: FixtureLogger = FixtureLogger;
static LOGGER_INIT: Once = Once::new();

struct MarkerCapture;

impl Drop for MarkerCapture {
    fn drop(&mut self) {
        // Restore the per-thread slot even on a constructor/counter error or a
        // panic. A later AC must not inherit a previous run's marker population.
        CAPTURE.with(|capture| {
            capture.borrow_mut().take();
        });
        slicer_sdk::host::test_support::take_log_messages();
        FrozenPreparationBinding::take_wasm_log_messages();
    }
}

impl MarkerCapture {
    fn finish(self) -> BTreeMap<String, u64> {
        let mut lines =
            CAPTURE.with(|capture| capture.borrow_mut().take().expect("active marker capture"));
        // Native SDK logging has its own thread-local sink and otherwise writes
        // directly to stderr; it does not dispatch through log::Log. Retain the
        // actual message (with no assumptions about a rendered [info] wrapper).
        lines.extend(
            slicer_sdk::host::test_support::take_log_messages()
                .into_iter()
                .map(|(_, message)| message),
        );
        lines.extend(
            FrozenPreparationBinding::take_wasm_log_messages()
                .into_iter()
                .map(|(_, message)| message),
        );
        let mut counts = BTreeMap::new();
        for line in lines {
            *counts.entry(line).or_default() += 1;
        }
        counts
    }
}

fn begin_capture() -> MarkerCapture {
    LOGGER_INIT.call_once(|| {
        log::set_logger(&LOGGER)
            .expect("whole-print marker capture requires the contract logger slot");
        log::set_max_level(log::LevelFilter::Info);
    });
    CAPTURE.with(|capture| {
        assert!(
            capture.borrow().is_none(),
            "whole-print driver must not nest marker captures"
        );
        *capture.borrow_mut() = Some(Vec::new());
    });
    slicer_sdk::host::test_support::install_log_capture();
    FrozenPreparationBinding::take_wasm_log_messages();
    MarkerCapture
}

/// Also usable around prevalidate alone, so rejection tests can compare actual
/// native/guest log-marker populations without starting either preparation leg.
pub fn capture_markers<R>(call: impl FnOnce() -> R) -> (R, BTreeMap<String, u64>) {
    let capture = begin_capture();
    let result = call();
    (result, capture.finish())
}

pub struct LegResult {
    pub observations: Option<String>,
    /// Exact observed log messages and multiplicities (not fabricated markers).
    pub markers: BTreeMap<String, u64>,
    pub metadata: PreparedPlanMetadata,
    pub current: Result<PreparedPlanHandle, PreparationFailure>,
    pub failure: Option<PreparationFailure>,
    /// A before-call snapshot only: prepare consumes its non-cloneable input.
    pub counters_before_prepare: Option<Result<(u64, u64), PreparationInputError>>,
    /// Actual production-reader/host-handler result after real owner disposal.
    pub post_disposal_read: Option<Result<u64, PreparationInputError>>,
    pub driver: PreparedDriver,
}

impl LegResult {
    pub fn marker_count(&self, marker: &str) -> u64 {
        self.markers
            .iter()
            .filter(|(message, _)| message.contains(marker))
            .map(|(_, count)| count)
            .sum()
    }
}

pub struct PairResult {
    pub native: LegResult,
    pub wasm: LegResult,
}

pub fn run_leg(
    package: Package,
    claims: &[&str],
    scenario: &Scenario,
    wasm: bool,
) -> Result<LegResult, PreparationFailure> {
    run_leg_with_publication(package, claims, scenario, wasm, |_| Ok(()))
}

pub fn run_leg_with_publication(
    package: Package,
    claims: &[&str],
    scenario: &Scenario,
    wasm: bool,
    publication: impl FnOnce(&StagedPreparation) -> Result<(), PreparationFailure>,
) -> Result<LegResult, PreparationFailure> {
    let driver = PreparedDriver::new(package, scenario, wasm)?;
    let inputs = literal_inputs(package, scenario)
        .commit()
        .map_err(PreparationFailure::Input)?;
    // Keep A's real authority alive while B invokes the mismatched resource.
    let foreign = if matches!(scenario.owner_reads, OwnerReadScenario::CrossOwner) {
        Some(PreparedDriver::new(package, scenario, wasm)?)
    } else {
        None
    };
    let input_owner = foreign.as_ref().map_or(&driver.owner, |a| &a.owner);
    let disposal_input = if matches!(scenario.owner_reads, OwnerReadScenario::DisposedOwner) {
        Some(prevalidate(
            &driver.owner,
            inputs.clone(),
            scenario.selection(package, claims),
        )?)
    } else {
        None
    };
    let disposal_reader = if !wasm {
        disposal_input
            .as_ref()
            .map(|input| input.controlled_native_input(&driver.owner, &driver.binding))
            .transpose()?
    } else {
        None
    };
    let capture = begin_capture();
    let input = prevalidate(input_owner, inputs, scenario.selection(package, claims));
    let (result, counters_before_prepare) = match input {
        Err(error) => (Err(error), None),
        Ok(input) => {
            if let Some((calls, records)) = scenario.counter_seed {
                set_input_counters(&input, calls, records).map_err(PreparationFailure::Input)?;
            }
            let counters = input_counters(&input);
            let result = if wasm {
                Ok(())
            } else {
                package.configure_native(&scenario.invocation(package))
            }
            .and_then(|()| {
                driver
                    .binding
                    .prepare_with_publication(&driver.owner, input, publication)
            });
            (result, Some(counters))
        }
    };
    let markers = capture.finish();
    let current = driver.store.current(&driver.owner);
    let observations = current
        .as_ref()
        .ok()
        .and_then(|plan| {
            plan.open("observations")
                .expect("open real observation piece")
        })
        .map(|piece| {
            String::from_utf8(
                piece
                    .read(0, u64::MAX)
                    .expect("read real observation piece"),
            )
            .expect("UTF-8 fixture observations")
        });
    let post_disposal_read = if let Some(input) = disposal_input {
        driver.store.dispose_owner(&driver.owner)?;
        Some(if wasm {
            slicer_wasm_host::preparation::PreparedHostState::controlled_layer_count(
                &driver.owner,
                &driver.binding,
                input,
            )
        } else {
            disposal_reader
                .expect("retained real native reader")
                .layer_count()
        })
    } else {
        None
    };
    Ok(LegResult {
        observations,
        markers,
        metadata: driver.owner.metadata(),
        current,
        failure: result.err(),
        counters_before_prepare,
        post_disposal_read,
        driver,
    })
}

pub fn run_pair(
    package: Package,
    claims: &[&str],
    scenario: &Scenario,
) -> Result<PairResult, PreparationFailure> {
    // Each leg gets fresh committed products, binding, owner store and adapter.
    Ok(PairResult {
        native: run_leg(package, claims, scenario, false)?,
        wasm: run_leg(package, claims, scenario, true)?,
    })
}
