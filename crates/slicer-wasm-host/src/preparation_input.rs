//! Indexed committed products and bounded projections, independent of adapter policy.

use super::{PreparationFailure, PreparationInputData, PreparationInputError, PreparationOwner};
use crate::selection::{
    build_layer_selection_index, LayerSelectionIndex, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, SelectedLayerTarget,
};
use slicer_ir::slice_ir::SupportSourceKey;
use slicer_ir::{
    BoundingBox3, ConfigView, ExPolygon, LayerPlanIR, MeshIR, ModuleId, ObjectMesh, PaintValue,
    Point3, Point3WithWidth, RegionKey, RegionMapIR, SeamPlanIR, SliceIR, StageId,
    SupportAnalysisIR, SupportGeometryIR, SupportGeometryKey, SupportPlanIR,
    SurfaceClassificationIR, Transform3d,
};
use slicer_sdk::preparation::*;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ops::Range,
    sync::{Arc, Mutex},
};

type InputResult<T> = Result<T, PreparationInputError>;
type GeometryMap = HashMap<SupportGeometryKey, Vec<ExPolygon>>;

struct LayerRows {
    slice: usize,
    regions: Vec<usize>,
    configs: Vec<RegionKey>,
    surface_objects: Vec<String>,
    seams: Vec<usize>,
}

#[derive(Default)]
struct AnalysisRows {
    candidates: Vec<usize>,
    model_occupancy: Vec<SupportGeometryKey>,
    cantilever_surfaces: Vec<SupportGeometryKey>,
    termination_surfaces: Vec<SupportGeometryKey>,
    support_territory: Vec<SupportGeometryKey>,
    family_assignments: Vec<SupportSourceKey>,
}

/// Immutable PrePass products with canonical, geometry-free presentation indices.
///
/// All supplied product Arcs are retained unchanged. Neither future Layer output
/// nor a live Layer arena can be installed in this backing.
pub struct CommittedPreparationInputs {
    mesh: Arc<MeshIR>,
    layers: Arc<LayerPlanIR>,
    slices: Arc<Vec<SliceIR>>,
    region_map: Arc<RegionMapIR>,
    surface: Option<Arc<SurfaceClassificationIR>>,
    seam: Option<Arc<SeamPlanIR>>,
    support_analysis: Option<Arc<SupportAnalysisIR>>,
    support_geometry: Option<Arc<SupportGeometryIR>>,
    support_plan: Option<Arc<SupportPlanIR>>,
    layer_rows: BTreeMap<u32, LayerRows>,
    objects: Vec<usize>,
    object_lookup: HashMap<String, usize>,
    surface_objects: Vec<String>,
    analysis_rows: AnalysisRows,
    geometry_rows: Vec<SupportGeometryKey>,
    plan_rows: Vec<usize>,
}

/// Frozen invocation authority supplied by the selected module's caller.
///
/// This context never resolves holders independently of the shared selection
/// index. Its stage and module must match the owner that binds it.
pub struct PreparationSelectionContext {
    stage_id: StageId,
    module_id: ModuleId,
    claims: Vec<String>,
    paint_semantics: HashSet<String>,
    invocation_config: Arc<ConfigView>,
}

impl PreparationSelectionContext {
    /// Capture already-resolved, declared-key-filtered invocation authority.
    pub fn new(
        stage_id: StageId,
        module_id: ModuleId,
        claims: Vec<String>,
        paint_semantics: HashSet<String>,
        invocation_config: Arc<ConfigView>,
    ) -> Self {
        Self {
            stage_id,
            module_id,
            claims,
            paint_semantics,
            invocation_config,
        }
    }
}

fn invalid(detail: impl Into<String>) -> PreparationInputError {
    PreparationInputError::InvalidInput {
        detail: detail.into(),
    }
}

fn finite(value: f32, field: &str) -> InputResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(format!("{field} must be finite")))
    }
}

fn point_finite(point: &Point3, field: &str) -> InputResult<()> {
    finite(point.x, &format!("{field}.x"))?;
    finite(point.y, &format!("{field}.y"))?;
    finite(point.z, &format!("{field}.z"))
}

fn width_point_finite(point: &Point3WithWidth, field: &str) -> InputResult<()> {
    for (name, value) in [
        ("x", point.x),
        ("y", point.y),
        ("z", point.z),
        ("width", point.width),
        ("flow_factor", point.flow_factor),
        ("dist_to_top_mm", point.dist_to_top_mm),
    ] {
        finite(value, &format!("{field}.{name}"))?;
    }
    if let Some(value) = point.overhang_distance_mm {
        finite(value, &format!("{field}.overhang_distance_mm"))?;
    }
    Ok(())
}

fn paint_finite(value: &PaintValue, field: &str) -> InputResult<()> {
    if let PaintValue::Scalar(value) = value {
        finite(*value, field)?;
    }
    Ok(())
}

fn chain_finite(chain: &[(String, PaintValue)], field: &str) -> InputResult<()> {
    for (_, value) in chain {
        paint_finite(value, field)?;
    }
    Ok(())
}

fn count(value: usize) -> InputResult<u64> {
    u64::try_from(value).map_err(|_| PreparationInputError::ArithmeticOverflow)
}

/// Clip before end arithmetic so a huge requested count is valid at EOF.
fn batch_range(len: usize, offset: u64, max_items: u64) -> InputResult<Range<usize>> {
    let len64 = count(len)?;
    if offset > len64 {
        return Err(PreparationInputError::RangeError);
    }
    let start = usize::try_from(offset).map_err(|_| PreparationInputError::ArithmeticOverflow)?;
    let take = usize::try_from(max_items.min(len64 - offset))
        .map_err(|_| PreparationInputError::ArithmeticOverflow)?;
    let end = start
        .checked_add(take)
        .ok_or(PreparationInputError::ArithmeticOverflow)?;
    Ok(start..end)
}

fn key_cmp(a: &RegionKey, b: &RegionKey) -> Ordering {
    (
        a.global_layer_index,
        &a.object_id,
        a.region_id,
        &a.variant_chain,
    )
        .cmp(&(
            b.global_layer_index,
            &b.object_id,
            b.region_id,
            &b.variant_chain,
        ))
}

fn geometry_key_cmp(a: &SupportGeometryKey, b: &SupportGeometryKey) -> Ordering {
    (
        a.global_support_layer_index,
        &a.object_id,
        a.region_id,
        &a.variant_chain,
    )
        .cmp(&(
            b.global_support_layer_index,
            &b.object_id,
            b.region_id,
            &b.variant_chain,
        ))
}

fn geometry_rows(map: &GeometryMap, field: &str) -> InputResult<Vec<SupportGeometryKey>> {
    count(map.len())?;
    let mut keys = Vec::with_capacity(map.len());
    for key in map.keys() {
        chain_finite(&key.variant_chain, &format!("{field}.variant_chain"))?;
        keys.push(key.clone());
    }
    keys.sort_by(geometry_key_cmp);
    Ok(keys)
}

impl CommittedPreparationInputs {
    /// Validate and retain committed products, without cloning their payload vectors.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mesh: Arc<MeshIR>,
        layers: Arc<LayerPlanIR>,
        slices: Arc<Vec<SliceIR>>,
        region_map: Arc<RegionMapIR>,
        surface: Option<Arc<SurfaceClassificationIR>>,
        seam: Option<Arc<SeamPlanIR>>,
        support_analysis: Option<Arc<SupportAnalysisIR>>,
        support_geometry: Option<Arc<SupportGeometryIR>>,
        support_plan: Option<Arc<SupportPlanIR>>,
    ) -> InputResult<Self> {
        count(layers.global_layers.len())?;
        count(slices.len())?;
        count(mesh.objects.len())?;
        point_finite(&mesh.build_volume.min, "mesh.build_volume.min")?;
        point_finite(&mesh.build_volume.max, "mesh.build_volume.max")?;
        let mut object_lookup = HashMap::new();
        for (index, object) in mesh.objects.iter().enumerate() {
            if object_lookup.insert(object.id.clone(), index).is_some() {
                return Err(invalid(format!(
                    "mesh.objects duplicate object_id {:?}",
                    object.id
                )));
            }
            count(object.mesh.vertices.len())?;
            count(object.mesh.indices.len() / 3)?;
            if object.mesh.indices.len() % 3 != 0 {
                return Err(invalid(
                    "mesh.objects.mesh.indices must contain complete triangles",
                ));
            }
            for index in &object.mesh.indices {
                let index = usize::try_from(*index)
                    .map_err(|_| PreparationInputError::ArithmeticOverflow)?;
                if index >= object.mesh.vertices.len() {
                    return Err(invalid(
                        "mesh.objects.mesh.indices vertex index out of range",
                    ));
                }
            }
            for point in &object.mesh.vertices {
                point_finite(point, "mesh.objects.mesh.vertices")?;
            }
            if object.transform.matrix.iter().any(|v| !v.is_finite()) {
                return Err(invalid("mesh.objects.transform.matrix must be finite"));
            }
            if let Some(paint) = &object.paint_data {
                for layer in &paint.layers {
                    for value in layer.facet_values.iter().flatten() {
                        paint_finite(value, "mesh.objects.paint_data.facet_values")?;
                    }
                    for stroke in &layer.strokes {
                        count(stroke.triangles.len())?
                            .checked_mul(3)
                            .ok_or(PreparationInputError::ArithmeticOverflow)?;
                        paint_finite(&stroke.value, "mesh.objects.paint_data.strokes.value")?;
                        for point in stroke.triangles.iter().flatten() {
                            point_finite(point, "mesh.objects.paint_data.strokes.triangles")?;
                        }
                    }
                }
            }
        }
        let mut objects: Vec<_> = (0..mesh.objects.len()).collect();
        objects.sort_by(|a, b| mesh.objects[*a].id.cmp(&mesh.objects[*b].id));

        let mut slice_lookup = HashMap::new();
        for (index, slice) in slices.iter().enumerate() {
            if slice_lookup
                .insert(slice.global_layer_index, index)
                .is_some()
            {
                return Err(invalid(format!(
                    "slices duplicate global_layer_index {}",
                    slice.global_layer_index
                )));
            }
            finite(slice.z, "slices.z")?;
            let mut keys = HashSet::new();
            for region in &slice.regions {
                if !keys.insert((&region.object_id, region.region_id, &region.variant_chain)) {
                    return Err(invalid(format!(
                        "slices.regions duplicate full region key at layer {}: {:?}/{}/{:?}",
                        slice.global_layer_index,
                        region.object_id,
                        region.region_id,
                        region.variant_chain
                    )));
                }
                finite(
                    region.effective_layer_height,
                    "slices.regions.effective_layer_height",
                )?;
                finite(
                    region.bridge_orientation_deg,
                    "slices.regions.bridge_orientation_deg",
                )?;
                if !region.internal_bridge_angles_deg.is_empty()
                    && region.internal_bridge_angles_deg.len() != region.internal_bridge_areas.len()
                {
                    return Err(invalid("slices.regions.internal_bridge_angles_deg must align with internal_bridge_areas"));
                }
                for angle in &region.internal_bridge_angles_deg {
                    finite(*angle, "slices.regions.internal_bridge_angles_deg")?;
                }
                chain_finite(&region.variant_chain, "slices.regions.variant_chain")?;
                for value in region
                    .segment_annotations
                    .values()
                    .flatten()
                    .flatten()
                    .flatten()
                {
                    paint_finite(value, "slices.regions.segment_annotations")?;
                }
            }
        }
        let mut layer_rows = BTreeMap::new();
        let mut previous = None;
        for layer in &layers.global_layers {
            finite(layer.z, "layers.global_layers.z")?;
            if previous.is_some_and(|(index, z)| layer.index <= index || layer.z <= z) {
                return Err(invalid(
                    "layers.global_layers.index and z must be unique and increasing",
                ));
            }
            previous = Some((layer.index, layer.z));
            for active in &layer.active_regions {
                finite(
                    active.effective_layer_height,
                    "layers.active_regions.effective_layer_height",
                )?;
                finite(
                    active.catchup_z_bottom,
                    "layers.active_regions.catchup_z_bottom",
                )?;
            }
            let slice = *slice_lookup.get(&layer.index).ok_or_else(|| {
                invalid(format!(
                    "slices missing schedule global_layer_index {}",
                    layer.index
                ))
            })?;
            if slices[slice].z.to_bits() != layer.z.to_bits() {
                return Err(invalid(format!(
                    "slices.z does not match layers.global_layers.z at index {}",
                    layer.index
                )));
            }
            let mut regions: Vec<_> = (0..slices[slice].regions.len()).collect();
            regions.sort_by(|a, b| {
                let a = &slices[slice].regions[*a];
                let b = &slices[slice].regions[*b];
                (&a.object_id, a.region_id, &a.variant_chain).cmp(&(
                    &b.object_id,
                    b.region_id,
                    &b.variant_chain,
                ))
            });
            layer_rows.insert(
                layer.index,
                LayerRows {
                    slice,
                    regions,
                    configs: Vec::new(),
                    surface_objects: Vec::new(),
                    seams: Vec::new(),
                },
            );
        }
        count(region_map.entries.len())?;
        for (key, plan) in &region_map.entries {
            chain_finite(&key.variant_chain, "region_map.entries.key.variant_chain")?;
            let index = usize::try_from(plan.config.0)
                .map_err(|_| PreparationInputError::ArithmeticOverflow)?;
            if index >= region_map.configs.len() {
                return Err(invalid(format!(
                    "region_map.entries.config out of range for key {key:?}"
                )));
            }
            if let Some(rows) = layer_rows.get_mut(&key.global_layer_index) {
                rows.configs.push(key.clone());
            }
        }
        for rows in layer_rows.values_mut() {
            rows.configs.sort_by(key_cmp);
        }

        let mut surface_objects = Vec::new();
        if let Some(surface) = &surface {
            let referenced: BTreeSet<_> = surface
                .per_object
                .keys()
                .chain(surface.overhang_quartile_polygons.keys())
                .chain(surface.prev_layer_boundaries.keys())
                .collect();
            for object_id in referenced {
                if !object_lookup.contains_key(object_id) {
                    return Err(PreparationInputError::UnknownObject {
                        object_id: object_id.clone(),
                    });
                }
            }
            surface_objects.extend(surface.per_object.keys().cloned());
            surface_objects.sort();
            for data in surface.per_object.values() {
                for class in &data.facet_classes {
                    match class {
                        slicer_ir::FacetClass::NearHorizontal { slope_angle_deg } => {
                            finite(*slope_angle_deg, "surface.facet_classes.slope_angle_deg")?
                        }
                        slicer_ir::FacetClass::Overhang { angle_deg } => {
                            finite(*angle_deg, "surface.facet_classes.angle_deg")?
                        }
                        _ => {}
                    }
                }
                for group in &data.surface_groups {
                    finite(group.z_min, "surface.surface_groups.z_min")?;
                    finite(group.z_max, "surface.surface_groups.z_max")?;
                    finite(group.area_mm2, "surface.surface_groups.area_mm2")?;
                }
                for bridge in &data.bridge_regions {
                    finite(
                        bridge.bridge_direction_deg,
                        "surface.bridge_regions.bridge_direction_deg",
                    )?;
                    finite(
                        bridge.anchor_width_mm,
                        "surface.bridge_regions.anchor_width_mm",
                    )?;
                    finite(
                        bridge.bridge_length_mm,
                        "surface.bridge_regions.bridge_length_mm",
                    )?;
                    finite(
                        bridge.expansion_margin_mm,
                        "surface.bridge_regions.expansion_margin_mm",
                    )?;
                }
                for overhang in &data.overhang_regions {
                    finite(
                        overhang.max_angle_deg,
                        "surface.overhang_regions.max_angle_deg",
                    )?;
                }
            }
            let mut per_layer: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
            for (object, layers) in &surface.overhang_quartile_polygons {
                for layer in layers.keys() {
                    per_layer.entry(*layer).or_default().insert(object.clone());
                }
            }
            for (object, layers) in &surface.prev_layer_boundaries {
                for layer in layers.keys() {
                    per_layer.entry(*layer).or_default().insert(object.clone());
                }
            }
            for (layer, objects) in per_layer {
                if let Some(rows) = layer_rows.get_mut(&layer) {
                    rows.surface_objects = objects.into_iter().collect();
                }
            }
        }
        if let Some(seam) = &seam {
            count(seam.entries.len())?;
            if let Some(key) = seam.duplicate_region_key() {
                return Err(invalid(format!(
                    "seam.entries duplicate region_key {key:?}"
                )));
            }
            for (index, entry) in seam.entries.iter().enumerate() {
                chain_finite(
                    &entry.region_key.variant_chain,
                    "seam.entries.region_key.variant_chain",
                )?;
                width_point_finite(
                    &entry.chosen_candidate.point,
                    "seam.entries.chosen_candidate.point",
                )?;
                for candidate in &entry.scored_candidates {
                    width_point_finite(
                        &candidate.position,
                        "seam.entries.scored_candidates.position",
                    )?;
                    finite(candidate.score, "seam.entries.scored_candidates.score")?;
                }
                if let Some(rows) = layer_rows.get_mut(&entry.region_key.global_layer_index) {
                    rows.seams.push(index);
                }
            }
            for rows in layer_rows.values_mut() {
                rows.seams.sort_by(|a, b| {
                    key_cmp(&seam.entries[*a].region_key, &seam.entries[*b].region_key)
                });
            }
        }
        let mut analysis_rows = AnalysisRows::default();
        if let Some(analysis) = &support_analysis {
            count(analysis.candidates.len())?;
            for candidate in &analysis.candidates {
                chain_finite(
                    &candidate.source.variant_chain,
                    "support_analysis.candidates.source.variant_chain",
                )?;
            }
            analysis_rows.candidates = (0..analysis.candidates.len()).collect();
            analysis_rows.candidates.sort_by(|a, b| {
                let left = &analysis.candidates[*a];
                let right = &analysis.candidates[*b];
                (
                    left.source.global_layer_index,
                    &left.source.object_id,
                    left.source.region_id,
                    &left.source.variant_chain,
                    left.source.z_units,
                    left.id,
                    a,
                )
                    .cmp(&(
                        right.source.global_layer_index,
                        &right.source.object_id,
                        right.source.region_id,
                        &right.source.variant_chain,
                        right.source.z_units,
                        right.id,
                        b,
                    ))
            });
            analysis_rows.model_occupancy = geometry_rows(
                &analysis.model_occupancy,
                "support_analysis.model_occupancy",
            )?;
            analysis_rows.cantilever_surfaces = geometry_rows(
                &analysis.cantilever_surfaces,
                "support_analysis.cantilever_surfaces",
            )?;
            analysis_rows.termination_surfaces = geometry_rows(
                &analysis.termination_surfaces,
                "support_analysis.termination_surfaces",
            )?;
            analysis_rows.support_territory = geometry_rows(
                &analysis.support_territory,
                "support_analysis.support_territory",
            )?;
            for key in analysis.family_assignments.keys() {
                chain_finite(
                    &key.variant_chain,
                    "support_analysis.family_assignments.variant_chain",
                )?;
                analysis_rows.family_assignments.push(key.clone());
            }
            count(analysis_rows.family_assignments.len())?;
        }
        let geometry_rows = match &support_geometry {
            Some(geometry) => {
                finite(
                    geometry.support_layer_height_mm,
                    "support_geometry.support_layer_height_mm",
                )?;
                finite(
                    geometry.support_top_z_distance_mm,
                    "support_geometry.support_top_z_distance_mm",
                )?;
                geometry_rows(&geometry.entries, "support_geometry.entries")?
            }
            None => Vec::new(),
        };
        let mut plan_rows = Vec::new();
        if let Some(plan) = &support_plan {
            count(plan.entries.len())?;
            for entry in &plan.entries {
                chain_finite(&entry.variant_chain, "support_plan.entries.variant_chain")?;
                if let Some(skeleton) = &entry.skeleton {
                    if skeleton.points.len() != skeleton.wall_counts.len() {
                        return Err(invalid(
                            "support_plan.entries.skeleton.wall_counts must align with points",
                        ));
                    }
                    for point in &skeleton.points {
                        point_finite(point, "support_plan.entries.skeleton.points")?;
                    }
                }
            }
            if let Some(raft) = &plan.raft_plan {
                finite(
                    raft.raft_first_layer_density,
                    "support_plan.raft_plan.raft_first_layer_density",
                )?;
            }
            plan_rows = (0..plan.entries.len()).collect();
            plan_rows.sort_by(|a, b| {
                let left = &plan.entries[*a];
                let right = &plan.entries[*b];
                (
                    left.global_layer_index,
                    &left.object_id,
                    left.region_id,
                    &left.family_id,
                    a,
                )
                    .cmp(&(
                        right.global_layer_index,
                        &right.object_id,
                        right.region_id,
                        &right.family_id,
                        b,
                    ))
            });
        }
        Ok(Self {
            mesh,
            layers,
            slices,
            region_map,
            surface,
            seam,
            support_analysis,
            support_geometry,
            support_plan,
            layer_rows,
            objects,
            object_lookup,
            surface_objects,
            analysis_rows,
            geometry_rows,
            plan_rows,
        })
    }

    fn rows(&self, layer: u32) -> InputResult<&LayerRows> {
        self.layer_rows
            .get(&layer)
            .ok_or(PreparationInputError::RangeError)
    }

    fn object(&self, object_id: &str) -> InputResult<&ObjectMesh> {
        self.object_lookup
            .get(object_id)
            .map(|index| &self.mesh.objects[*index])
            .ok_or_else(|| PreparationInputError::UnknownObject {
                object_id: object_id.into(),
            })
    }
}

#[derive(Default)]
struct InputCounters {
    read_calls: u64,
    returned_records: u64,
}

/// Owner-local authority; adapters perform grant and live-owner checks before projection.
pub(crate) struct OwnerPreparationInput {
    pub(crate) owner: PreparationOwner,
    inputs: Arc<CommittedPreparationInputs>,
    grants: HashSet<String>,
    context: PreparationSelectionContext,
    selections: LayerSelectionIndex,
    target_rows: BTreeMap<u32, Vec<usize>>,
    counters: Mutex<InputCounters>,
}

impl PreparationInputData {
    /// Bind committed products only after owner, declaration and selection validation.
    /// No module invocation or output transaction is started by this constructor.
    pub fn from_committed(
        owner: &PreparationOwner,
        inputs: Arc<CommittedPreparationInputs>,
        selection: PreparationSelectionContext,
    ) -> Result<Self, PreparationFailure> {
        // active_guard checks the retained artifact identity against the originating
        // store's owner entry. Stage/module checks then bind the caller's context
        // to that same unforgeable frozen owner, without another binding lookup.
        let _guard = owner
            .active_guard()
            .map_err(|_| PreparationFailure::Input(PreparationInputError::AccessDenied))?;
        if owner.metadata().module_id() != selection.module_id {
            return Err(PreparationFailure::InvalidOwner);
        }
        let grants =
            validate_reads(owner.input_reads(), &inputs).map_err(PreparationFailure::Input)?;
        let modules = [LayerSelectionModuleInput {
            stage_id: &selection.stage_id,
            module_id: &selection.module_id,
            claims: &selection.claims,
            paint_semantics: Some(&selection.paint_semantics),
            invocation_config: &selection.invocation_config,
        }];
        let selections = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: slicer_schema::STAGES,
            global_layers: &inputs.layers.global_layers,
            modules: &modules,
            slices: &inputs.slices,
            region_map: Some(&inputs.region_map),
            support_plan: inputs.support_plan.as_deref(),
            perimeters: &[], // Preparation has no committed Layer perimeter product.
        })
        .map_err(|error| {
            PreparationFailure::Input(PreparationInputError::SelectionRejected {
                detail: error.to_string(),
            })
        })?;
        // Let the shared authority preserve an unknown/non-Layer stage in its
        // SelectionRejected diagnostic. A valid but different Layer stage still
        // cannot bind another frozen owner's identity.
        if owner.stage_id() != selection.stage_id {
            return Err(PreparationFailure::InvalidOwner);
        }
        let mut target_rows = BTreeMap::new();
        for layer in &inputs.layers.global_layers {
            let selected = selections
                .selection_for(layer.index, &selection.module_id)
                .ok_or_else(|| {
                    PreparationFailure::Input(PreparationInputError::SelectionRejected {
                        detail: format!(
                            "missing selection for stage {:?}, module {:?}, global_layer_index {}",
                            selection.stage_id, selection.module_id, layer.index
                        ),
                    })
                })?;
            // Keep the index, including invoke, slice identity handles and exact
            // perimeter_configs. None of these identity handles is final geometry.
            let mut rows: Vec<_> = (0..selected.targets.len()).collect();
            rows.sort_by(|a, b| {
                target_cmp(&selected.targets[*a], &selected.targets[*b]).then_with(|| a.cmp(b))
            });
            for target in &selected.targets {
                if let LayerTargetSource::SupportCarrier { entry_index, .. } = &target.source {
                    count(*entry_index).map_err(PreparationFailure::Input)?;
                }
            }
            target_rows.insert(layer.index, rows);
        }
        let mut data = Self::for_owner(owner);
        data.committed = Some(Arc::new(OwnerPreparationInput {
            owner: owner.clone(),
            inputs,
            grants,
            context: selection,
            selections,
            target_rows,
            counters: Mutex::new(InputCounters::default()),
        }));
        Ok(data)
    }
}

fn validate_reads(
    reads: &[String],
    inputs: &CommittedPreparationInputs,
) -> InputResult<HashSet<String>> {
    const BASE: &[&str] = &[
        "layer_plan",
        "slices",
        "region_map",
        "mesh",
        "selected_targets",
        "fixture",
    ];
    const OPTIONAL: &[&str] = &[
        "surface_classification",
        "seam_plan",
        "support_analysis",
        "support_geometry",
        "support_plan",
    ];
    let mut grants = HashSet::new();
    for read in reads {
        if !BASE.contains(&read.as_str())
            && !OPTIONAL
                .iter()
                .any(|base| read == base || read == &format!("{base}_optional"))
        {
            return Err(PreparationInputError::UnsupportedRead { read: read.clone() });
        }
        if read != "fixture" {
            grants.insert(read.clone());
        }
    }
    for base in OPTIONAL {
        let optional = format!("{base}_optional");
        if grants.contains(*base) && grants.contains(&optional) {
            return Err(invalid(format!(
                "conflicting input_reads tokens {base} and {optional}"
            )));
        }
    }
    for (read, present) in [
        ("surface_classification", inputs.surface.is_some()),
        ("seam_plan", inputs.seam.is_some()),
        ("support_analysis", inputs.support_analysis.is_some()),
        ("support_geometry", inputs.support_geometry.is_some()),
        ("support_plan", inputs.support_plan.is_some()),
    ] {
        if grants.contains(read) && !present {
            return Err(PreparationInputError::MissingRequired { read: read.into() });
        }
    }
    Ok(grants)
}

fn target_cmp(a: &SelectedLayerTarget, b: &SelectedLayerTarget) -> Ordering {
    fn tag(source: &LayerTargetSource) -> u8 {
        match source {
            LayerTargetSource::Model { .. } => 0,
            LayerTargetSource::SupportCarrier { .. } => 1,
            LayerTargetSource::Raft { .. } => 2,
            LayerTargetSource::AnchoredStage { .. } => 3,
        }
    }
    tag(&a.source)
        .cmp(&tag(&b.source))
        .then_with(|| match (&a.source, &b.source) {
            (LayerTargetSource::Model { key: a, .. }, LayerTargetSource::Model { key: b, .. }) => {
                key_cmp(a, b)
            }
            (
                LayerTargetSource::SupportCarrier {
                    entry_index: ai,
                    object_id: ao,
                    region_id: ar,
                    variant_chain: ac,
                    family: af,
                    demand_ids: ad,
                    body_ids: ab,
                    provenance: ap,
                    anchor_layer_index: aa,
                },
                LayerTargetSource::SupportCarrier {
                    entry_index: bi,
                    object_id: bo,
                    region_id: br,
                    variant_chain: bc,
                    family: bf,
                    demand_ids: bd,
                    body_ids: bb,
                    provenance: bp,
                    anchor_layer_index: ba,
                },
            ) => (ao, ar, ac, af, ad, ab, ap, aa, ai).cmp(&(bo, br, bc, bf, bd, bb, bp, ba, bi)),
            (
                LayerTargetSource::Raft {
                    global_layer_index: a,
                },
                LayerTargetSource::Raft {
                    global_layer_index: b,
                },
            ) => a.cmp(b),
            (
                LayerTargetSource::AnchoredStage {
                    anchor_layer_index: a,
                },
                LayerTargetSource::AnchoredStage {
                    anchor_layer_index: b,
                },
            ) => a.cmp(b),
            _ => Ordering::Equal,
        })
}

/// Pure mesh helpers supplied by the adapter after its access check. Step 5/6
/// supplies the existing host implementations; no second geometry algorithm lives here.
pub(crate) struct PreparationMeshHelpers {
    pub(crate) transform: fn(&Transform3d, &Point3) -> Point3,
    pub(crate) bounds: fn(&ObjectMesh) -> Option<BoundingBox3>,
    pub(crate) raycast: fn(&ObjectMesh, f32, f32, f32) -> Option<f32>,
    pub(crate) normal: fn(&ObjectMesh, f32, f32, f32) -> Option<Point3>,
}

impl OwnerPreparationInput {
    /// Ingestion-path accessor for the already declared-key-filtered invocation view.
    pub(crate) fn invocation_config(&self) -> &ConfigView {
        &self.context.invocation_config
    }

    pub(crate) fn permits(&self, read: &str) -> bool {
        read != "fixture"
            && (self.grants.contains(read) || self.grants.contains(&format!("{read}_optional")))
    }

    /// Check both counters before building a returned payload. Hold the guard
    /// across projection so concurrent reads cannot invalidate the preflight;
    /// only a completely successful projection commits either counter.
    fn transfer<T>(
        &self,
        returned: usize,
        project: impl FnOnce() -> InputResult<T>,
    ) -> InputResult<T> {
        let returned = count(returned)?;
        let mut counters = self
            .counters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let read_calls = counters
            .read_calls
            .checked_add(1)
            .ok_or(PreparationInputError::ArithmeticOverflow)?;
        let returned_records = counters
            .returned_records
            .checked_add(returned)
            .ok_or(PreparationInputError::ArithmeticOverflow)?;
        let value = project()?;
        counters.read_calls = read_calls;
        counters.returned_records = returned_records;
        Ok(value)
    }

    pub(crate) fn layer_count(&self) -> InputResult<u64> {
        self.transfer(0, || count(self.inputs.layers.global_layers.len()))
    }

    pub(crate) fn layers(&self, offset: u64, max_items: u64) -> InputResult<Vec<PreparationLayer>> {
        let range = batch_range(self.inputs.layers.global_layers.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = self.inputs.layers.global_layers[range]
                .iter()
                .map(|layer| PreparationLayer {
                    index: layer.index,
                    z: layer.z,
                    has_nonplanar: layer.has_nonplanar,
                    is_sync_layer: layer.is_sync_layer,
                    is_raft: layer.is_raft,
                    active_regions: layer
                        .active_regions
                        .iter()
                        .map(|active| PreparationActiveRegion {
                            object_id: active.object_id.clone(),
                            region_id: active.region_id,
                            config: self.config(&active.resolved_config),
                            effective_layer_height: active.effective_layer_height,
                            nonplanar_shell: active.nonplanar_shell.map(|shell| {
                                PreparationNonplanarShell {
                                    surface_group_id: shell.surface_group_id,
                                    shell_index: shell.shell_index,
                                }
                            }),
                            is_catchup_layer: active.is_catchup_layer,
                            catchup_z_bottom: active.catchup_z_bottom,
                            tool_index: active.tool_index,
                        })
                        .collect(),
                })
                .collect::<Vec<_>>();
            Ok(result)
        })
    }

    fn config(&self, config: &slicer_ir::ResolvedConfig) -> ConfigView {
        slicer_scheduler::execution_plan::project_declared_config_view(
            config,
            self.context
                .invocation_config
                .keys()
                .iter()
                .map(String::as_str),
        )
    }

    pub(crate) fn regions(
        &self,
        layer: u32,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Vec<PreparationRegion>> {
        let rows = self.inputs.rows(layer)?;
        let range = batch_range(rows.regions.len(), offset, max_items)?;
        let slice = &self.inputs.slices[rows.slice];
        self.transfer(range.len(), || {
            let result = rows.regions[range]
                .iter()
                .map(|index| project_region(slice, &slice.regions[*index]))
                .collect::<Vec<_>>();
            Ok(result)
        })
    }

    pub(crate) fn region_configs(
        &self,
        layer: u32,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Vec<PreparationRegionConfig>> {
        let rows = self.inputs.rows(layer)?;
        let range = batch_range(rows.configs.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = rows.configs[range]
                .iter()
                .map(|key| PreparationRegionConfig {
                    key: key.into(),
                    config: self.config(self.inputs.region_map.config_for(key)),
                })
                .collect::<Vec<_>>();
            Ok(result)
        })
    }

    pub(crate) fn surface_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<Vec<PreparationSurfaceObject>>> {
        let Some(surface) = &self.inputs.surface else {
            return self.transfer(0, || Ok(None));
        };
        let range = batch_range(self.inputs.surface_objects.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = self.inputs.surface_objects[range]
                .iter()
                .map(|object_id| {
                    let data = &surface.per_object[object_id];
                    PreparationSurfaceObject {
                        object_id: object_id.clone(),
                        facet_classes: data
                            .facet_classes
                            .iter()
                            .map(|class| match class {
                                slicer_ir::FacetClass::Normal => PreparationFacetClass::Normal,
                                slicer_ir::FacetClass::NearHorizontal { slope_angle_deg } => {
                                    PreparationFacetClass::NearHorizontal {
                                        slope_angle_deg: *slope_angle_deg,
                                    }
                                }
                                slicer_ir::FacetClass::Overhang { angle_deg } => {
                                    PreparationFacetClass::Overhang {
                                        angle_deg: *angle_deg,
                                    }
                                }
                                slicer_ir::FacetClass::Bridge => PreparationFacetClass::Bridge,
                                slicer_ir::FacetClass::TopSurface => {
                                    PreparationFacetClass::TopSurface
                                }
                                slicer_ir::FacetClass::BottomSurface => {
                                    PreparationFacetClass::BottomSurface
                                }
                            })
                            .collect(),
                        surface_groups: data.surface_groups.clone(),
                        bridge_regions: data
                            .bridge_regions
                            .iter()
                            .map(|bridge| PreparationBridgeRegion {
                                id: bridge.id,
                                facet_indices: bridge.facet_indices.clone(),
                                bridge_direction_deg: bridge.bridge_direction_deg,
                                anchor_width_mm: bridge.anchor_width_mm,
                                bridge_length_mm: bridge.bridge_length_mm,
                                expansion_margin_mm: bridge.expansion_margin_mm,
                                is_valid: bridge.is_valid,
                                xy_footprint: bridge.xy_footprint.clone(),
                            })
                            .collect(),
                        overhang_regions: data
                            .overhang_regions
                            .iter()
                            .map(|overhang| PreparationOverhangRegion {
                                id: overhang.id,
                                facet_indices: overhang.facet_indices.clone(),
                                max_angle_deg: overhang.max_angle_deg,
                                needs_support: overhang.needs_support,
                                xy_footprint: overhang.xy_footprint.clone(),
                            })
                            .collect(),
                    }
                })
                .collect::<Vec<_>>();
            Ok(Some(result))
        })
    }

    pub(crate) fn surface_layers(
        &self,
        layer: u32,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<Vec<PreparationSurfaceLayer>>> {
        let rows = self.inputs.rows(layer)?;
        let Some(surface) = &self.inputs.surface else {
            return self.transfer(0, || Ok(None));
        };
        let range = batch_range(rows.surface_objects.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = rows.surface_objects[range]
                .iter()
                .map(|object_id| PreparationSurfaceLayer {
                    object_id: object_id.clone(),
                    global_layer_index: layer,
                    overhang_quartile_polygons: surface
                        .overhang_quartile_polygons
                        .get(object_id)
                        .and_then(|layers| layers.get(&layer))
                        .cloned()
                        .unwrap_or_default(),
                    prev_layer_boundaries: surface
                        .prev_layer_boundaries
                        .get(object_id)
                        .and_then(|layers| layers.get(&layer))
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            Ok(Some(result))
        })
    }

    pub(crate) fn mesh_objects(
        &self,
        offset: u64,
        max_items: u64,
        helpers: &PreparationMeshHelpers,
    ) -> InputResult<Vec<PreparationMeshObject>> {
        let range = batch_range(self.inputs.objects.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = self.inputs.objects[range]
                .iter()
                .map(|index| {
                    let object = &self.inputs.mesh.objects[*index];
                    let bounds = (helpers.bounds)(object);
                    if let Some(bounds) = &bounds {
                        point_finite(&bounds.min, "mesh_objects.bounds.min")?;
                        point_finite(&bounds.max, "mesh_objects.bounds.max")?;
                    }
                    let paint_layers = object
                        .paint_data
                        .iter()
                        .flat_map(|paint| &paint.layers)
                        .map(|layer| {
                            Ok(PreparationPaintLayer {
                                semantic: crate::marshal::paint_semantic_to_string(&layer.semantic),
                                facet_values: layer
                                    .facet_values
                                    .iter()
                                    .map(|value| value.as_ref().map(Into::into))
                                    .collect(),
                                strokes: layer
                                    .strokes
                                    .iter()
                                    .map(|stroke| {
                                        let triangles = stroke
                                            .triangles
                                            .iter()
                                            .flatten()
                                            .map(|point| {
                                                let point =
                                                    (helpers.transform)(&object.transform, point);
                                                point_finite(
                                                    &point,
                                                    "mesh_objects.paint_layers.strokes.triangles",
                                                )?;
                                                Ok(point)
                                            })
                                            .collect::<InputResult<Vec<_>>>()?;
                                        Ok(PreparationPaintStroke {
                                            triangles,
                                            semantic: crate::marshal::paint_semantic_to_string(
                                                &stroke.semantic,
                                            ),
                                            value: (&stroke.value).into(),
                                        })
                                    })
                                    .collect::<InputResult<Vec<_>>>()?,
                            })
                        })
                        .collect::<InputResult<Vec<_>>>()?;
                    Ok(PreparationMeshObject {
                        object_id: object.id.clone(),
                        bounds,
                        vertex_count: count(object.mesh.vertices.len())?,
                        triangle_count: count(object.mesh.indices.len() / 3)?,
                        paint_layers,
                    })
                })
                .collect::<InputResult<Vec<_>>>()?;
            Ok(result)
        })
    }

    pub(crate) fn mesh_vertices(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
        helpers: &PreparationMeshHelpers,
    ) -> InputResult<Vec<Point3>> {
        let object = self.inputs.object(object_id)?;
        let range = batch_range(object.mesh.vertices.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = object.mesh.vertices[range]
                .iter()
                .map(|point| {
                    let point = (helpers.transform)(&object.transform, point);
                    point_finite(&point, "mesh_vertices.world_position")?;
                    Ok(point)
                })
                .collect::<InputResult<Vec<_>>>()?;
            Ok(result)
        })
    }

    pub(crate) fn mesh_triangles(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Vec<(u32, u32, u32)>> {
        let object = self.inputs.object(object_id)?;
        let range = batch_range(object.mesh.indices.len() / 3, offset, max_items)?;
        self.transfer(range.len(), || {
            let result = range
                .map(|index| {
                    let start = index
                        .checked_mul(3)
                        .ok_or(PreparationInputError::ArithmeticOverflow)?;
                    let end = start
                        .checked_add(3)
                        .ok_or(PreparationInputError::ArithmeticOverflow)?;
                    let triangle = object
                        .mesh
                        .indices
                        .get(start..end)
                        .ok_or(PreparationInputError::RangeError)?;
                    Ok((triangle[0], triangle[1], triangle[2]))
                })
                .collect::<InputResult<Vec<_>>>()?;
            Ok(result)
        })
    }

    pub(crate) fn mesh_queries(
        &self,
        requests: &[PreparationMeshQuery],
        helpers: &PreparationMeshHelpers,
    ) -> InputResult<Vec<PreparationMeshQueryResult>> {
        self.transfer(requests.len(), || {
            let result = requests
                .iter()
                .map(|query| match query {
                    PreparationMeshQuery::RaycastZDown {
                        object_id,
                        x,
                        y,
                        start_z,
                    } => {
                        finite(*x, "mesh_queries.raycast_z_down.x")?;
                        finite(*y, "mesh_queries.raycast_z_down.y")?;
                        finite(*start_z, "mesh_queries.raycast_z_down.start_z")?;
                        let hit =
                            (helpers.raycast)(self.inputs.object(object_id)?, *x, *y, *start_z);
                        if let Some(z) = hit {
                            finite(z, "mesh_queries.raycast_z_down.result")?;
                        }
                        Ok(PreparationMeshQueryResult::RaycastZDown(hit))
                    }
                    PreparationMeshQuery::SurfaceNormalAt { object_id, x, y, z } => {
                        finite(*x, "mesh_queries.surface_normal_at.x")?;
                        finite(*y, "mesh_queries.surface_normal_at.y")?;
                        finite(*z, "mesh_queries.surface_normal_at.z")?;
                        let normal = (helpers.normal)(self.inputs.object(object_id)?, *x, *y, *z);
                        if let Some(point) = &normal {
                            point_finite(point, "mesh_queries.surface_normal_at.result")?;
                        }
                        Ok(PreparationMeshQueryResult::SurfaceNormalAt(normal))
                    }
                    PreparationMeshQuery::ObjectBounds { object_id } => {
                        let bounds =
                            (helpers.bounds)(self.inputs.object(object_id)?).ok_or_else(|| {
                                invalid(format!(
                                    "mesh_queries.object_bounds empty mesh {object_id:?}"
                                ))
                            })?;
                        point_finite(&bounds.min, "mesh_queries.object_bounds.min")?;
                        point_finite(&bounds.max, "mesh_queries.object_bounds.max")?;
                        Ok(PreparationMeshQueryResult::ObjectBounds(bounds))
                    }
                })
                .collect::<InputResult<Vec<_>>>()?;
            Ok(result)
        })
    }

    pub(crate) fn seams(
        &self,
        layer: u32,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<Vec<PreparationSeam>>> {
        let rows = self.inputs.rows(layer)?;
        let Some(seam) = &self.inputs.seam else {
            return self.transfer(0, || Ok(None));
        };
        let range = batch_range(rows.seams.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = rows.seams[range]
                .iter()
                .map(|index| {
                    let entry = &seam.entries[*index];
                    PreparationSeam {
                        region_key: (&entry.region_key).into(),
                        chosen_candidate: entry.chosen_candidate.clone(),
                        scored_candidates: entry
                            .scored_candidates
                            .iter()
                            .map(|candidate| PreparationScoredSeamCandidate {
                                position: candidate.position,
                                score: candidate.score,
                                reason: match candidate.reason {
                                    slicer_ir::SeamReason::Concave => {
                                        PreparationSeamReason::Concave
                                    }
                                    slicer_ir::SeamReason::Aligned => {
                                        PreparationSeamReason::Aligned
                                    }
                                    slicer_ir::SeamReason::UserForced => {
                                        PreparationSeamReason::UserForced
                                    }
                                    slicer_ir::SeamReason::Sharp => PreparationSeamReason::Sharp,
                                },
                            })
                            .collect(),
                    }
                })
                .collect::<Vec<_>>();
            Ok(Some(result))
        })
    }

    pub(crate) fn support_analysis(
        &self,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<PreparationSupportAnalysisBatch>> {
        let Some(analysis) = &self.inputs.support_analysis else {
            return self.transfer(0, || Ok(None));
        };
        let rows = &self.inputs.analysis_rows;
        // Independent collection ranges: no zip, no shortest-collection truncation.
        // A shared offset may be beyond a shorter collection while other collections
        // still have rows. Only an offset beyond the longest collection is invalid.
        let longest = [
            rows.candidates.len(),
            rows.model_occupancy.len(),
            rows.cantilever_surfaces.len(),
            rows.termination_surfaces.len(),
            rows.support_territory.len(),
            rows.family_assignments.len(),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        batch_range(longest, offset, max_items)?;
        let returned = [
            rows.candidates.len(),
            rows.model_occupancy.len(),
            rows.cantilever_surfaces.len(),
            rows.termination_surfaces.len(),
            rows.support_territory.len(),
            rows.family_assignments.len(),
        ]
        .into_iter()
        .try_fold(0usize, |sum, len| {
            let take = batch_range(len, offset.min(count(len)?), max_items)?.len();
            sum.checked_add(take)
                .ok_or(PreparationInputError::ArithmeticOverflow)
        })?;
        self.transfer(returned, || {
            let range = batch_range(
                rows.candidates.len(),
                offset.min(count(rows.candidates.len())?),
                max_items,
            )?;
            let candidates = rows.candidates[range]
                .iter()
                .map(|index| {
                    let candidate = &analysis.candidates[*index];
                    PreparationSupportCandidate {
                        id: candidate.id,
                        geometry: candidate.geometry.clone(),
                        source: PreparationSupportCandidateSource {
                            object_id: candidate.source.object_id.clone(),
                            region_id: candidate.source.region_id,
                            global_layer_index: candidate.source.global_layer_index,
                            z_units: candidate.source.z_units,
                            variant_chain: project_chain(&candidate.source.variant_chain),
                        },
                        enforced: candidate.enforced,
                        blocked: candidate.blocked,
                    }
                })
                .collect::<Vec<_>>();
            let independent = |map: &GeometryMap, keys: &[SupportGeometryKey]| {
                project_geometry(map, keys, offset.min(count(keys.len())?), max_items)
            };
            let range = batch_range(
                rows.family_assignments.len(),
                offset.min(count(rows.family_assignments.len())?),
                max_items,
            )?;
            let family_assignments = rows.family_assignments[range]
                .iter()
                .map(|key| PreparationSupportFamilyAssignment {
                    key: PreparationSupportSourceKey {
                        object_id: key.object_id.clone(),
                        region_id: key.region_id,
                        variant_chain: project_chain(&key.variant_chain),
                    },
                    family_id: analysis.family_assignments[key].clone(),
                })
                .collect();
            let batch = PreparationSupportAnalysisBatch {
                candidates,
                model_occupancy: independent(&analysis.model_occupancy, &rows.model_occupancy)?,
                cantilever_surfaces: independent(
                    &analysis.cantilever_surfaces,
                    &rows.cantilever_surfaces,
                )?,
                termination_surfaces: independent(
                    &analysis.termination_surfaces,
                    &rows.termination_surfaces,
                )?,
                support_territory: independent(
                    &analysis.support_territory,
                    &rows.support_territory,
                )?,
                shared_settings: analysis
                    .shared_settings
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                baseline_feasible_envelope: analysis.baseline_feasible_envelope.clone(),
                family_assignments,
            };
            Ok(Some(batch))
        })
    }

    pub(crate) fn support_geometry(
        &self,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<PreparationSupportGeometryBatch>> {
        let Some(geometry) = &self.inputs.support_geometry else {
            return self.transfer(0, || Ok(None));
        };
        let range = batch_range(self.inputs.geometry_rows.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let entries = project_geometry(
                &geometry.entries,
                &self.inputs.geometry_rows,
                offset,
                max_items,
            )?;
            Ok(Some(PreparationSupportGeometryBatch {
                support_layer_height_mm: geometry.support_layer_height_mm,
                support_top_z_distance_mm: geometry.support_top_z_distance_mm,
                entries,
            }))
        })
    }

    pub(crate) fn support_plan(
        &self,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Option<PreparationSupportPlanBatch>> {
        let Some(plan) = &self.inputs.support_plan else {
            return self.transfer(0, || Ok(None));
        };
        let range = batch_range(self.inputs.plan_rows.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let entries = self.inputs.plan_rows[range]
                .iter()
                .map(|index| {
                    let entry = &plan.entries[*index];
                    PreparationSupportPlanEntry {
                        global_layer_index: entry.global_layer_index,
                        object_id: entry.object_id.clone(),
                        region_id: entry.region_id,
                        family_id: entry.family_id.clone(),
                        demand_ids: entry.demand_ids.clone(),
                        body_ids: entry.body_ids.clone(),
                        anchor_layer_index: entry.anchor_layer_index,
                        anchor_z: entry.anchor_z,
                        roles: entry
                            .roles
                            .iter()
                            .map(|role| PreparationSupportPlanRoleRegion {
                                role: match role.role {
                                    slicer_ir::SupportPlanRole::SupportBody => {
                                        PreparationSupportPlanRole::SupportBody
                                    }
                                    slicer_ir::SupportPlanRole::TopInterface => {
                                        PreparationSupportPlanRole::TopInterface
                                    }
                                    slicer_ir::SupportPlanRole::BaseInterface => {
                                        PreparationSupportPlanRole::BaseInterface
                                    }
                                    slicer_ir::SupportPlanRole::BottomInterface => {
                                        PreparationSupportPlanRole::BottomInterface
                                    }
                                    slicer_ir::SupportPlanRole::RaftRelated => {
                                        PreparationSupportPlanRole::RaftRelated
                                    }
                                },
                                regions: role.regions.clone(),
                            })
                            .collect(),
                        skeleton: entry.skeleton.as_ref().map(|skeleton| {
                            PreparationSupportPlanSkeleton {
                                points: skeleton.points.clone(),
                                wall_counts: skeleton.wall_counts.clone(),
                            }
                        }),
                        capabilities: entry.capabilities.clone(),
                        provenance: entry.provenance.clone(),
                        decline_reason: entry.decline_reason.map(|reason| match reason {
                            slicer_ir::SupportPlanDeclineReason::DeclinedPolicy => {
                                PreparationSupportPlanDeclineReason::DeclinedPolicy
                            }
                            slicer_ir::SupportPlanDeclineReason::NoRoute => {
                                PreparationSupportPlanDeclineReason::NoRoute
                            }
                            slicer_ir::SupportPlanDeclineReason::Blocked => {
                                PreparationSupportPlanDeclineReason::Blocked
                            }
                            slicer_ir::SupportPlanDeclineReason::UnsupportedMode => {
                                PreparationSupportPlanDeclineReason::UnsupportedMode
                            }
                        }),
                        variant_chain: project_chain(&entry.variant_chain),
                    }
                })
                .collect::<Vec<_>>();
            Ok(Some(PreparationSupportPlanBatch {
                entries,
                raft_plan: plan.raft_plan.as_ref().map(|raft| PreparationRaftPlan {
                    raft_layers: raft.raft_layers,
                    raft_first_layer_density: raft.raft_first_layer_density,
                    base_raft_layers: raft.base_raft_layers,
                    interface_raft_layers: raft.interface_raft_layers,
                }),
            }))
        })
    }

    pub(crate) fn selected_targets(
        &self,
        layer: u32,
        offset: u64,
        max_items: u64,
    ) -> InputResult<Vec<PreparationSelectedTarget>> {
        self.inputs.rows(layer)?;
        let selected = self
            .selections
            .selection_for(layer, &self.context.module_id)
            .ok_or_else(|| PreparationInputError::SelectionRejected {
                detail: format!(
                    "missing selection for stage {:?}, module {:?}, global_layer_index {layer}",
                    self.context.stage_id, self.context.module_id
                ),
            })?;
        let rows = &self.target_rows[&layer];
        let range = batch_range(rows.len(), offset, max_items)?;
        self.transfer(range.len(), || {
            let result = rows[range]
                .iter()
                .map(|index| {
                    let target = &selected.targets[*index];
                    Ok(PreparationSelectedTarget {
                        global_layer_index: layer,
                        stage_id: selected.stage_id.clone(),
                        invoke: selected.invoke,
                        source: project_target_source(&target.source)?,
                        config: (*target.config).clone(),
                        held_claims: target.held_claims.clone(),
                        authored_coloring: target.authored_coloring,
                        resolved_role_tool: target.resolved_role_tool,
                    })
                })
                .collect::<InputResult<Vec<_>>>()?;
            Ok(result)
        })
    }
}

fn project_chain(chain: &[(String, PaintValue)]) -> PreparationVariantChain {
    chain
        .iter()
        .map(|(semantic, value)| (semantic.clone(), value.into()))
        .collect()
}

fn project_region(slice: &SliceIR, region: &slicer_ir::SlicedRegion) -> PreparationRegion {
    PreparationRegion {
        global_layer_index: slice.global_layer_index,
        z: slice.z,
        object_id: region.object_id.clone(),
        region_id: region.region_id,
        variant_chain: project_chain(&region.variant_chain),
        polygons: region.polygons.clone(),
        infill_areas: region.infill_areas.clone(),
        nonplanar_surface: region.nonplanar_surface,
        effective_layer_height: region.effective_layer_height,
        segment_annotations: region
            .segment_annotations
            .iter()
            .map(|(semantic, polygons)| {
                (
                    semantic.clone(),
                    polygons
                        .iter()
                        .map(|segments| {
                            segments
                                .iter()
                                .map(|value| value.as_ref().map(Into::into))
                                .collect()
                        })
                        .collect(),
                )
            })
            .collect(),
        top_shell_index: region.top_shell_index,
        bottom_shell_index: region.bottom_shell_index,
        top_solid_fill: region.top_solid_fill.clone(),
        bottom_solid_fill: region.bottom_solid_fill.clone(),
        is_bridge: region.is_bridge,
        bridge_areas: region.bridge_areas.clone(),
        bridge_orientation_deg: region.bridge_orientation_deg,
        sparse_infill_area: region.sparse_infill_area.clone(),
        internal_solid_fill: region.internal_solid_fill.clone(),
        internal_bridge_areas: region.internal_bridge_areas.clone(),
        internal_bridge_angles_deg: region.internal_bridge_angles_deg.clone(),
        raft_fill: region.raft_fill.clone(),
        phase: PreparationInputPhase::CommittedPrepass,
    }
}

fn project_geometry(
    map: &GeometryMap,
    keys: &[SupportGeometryKey],
    offset: u64,
    max_items: u64,
) -> InputResult<Vec<PreparationSupportGeometryEntry>> {
    let range = batch_range(keys.len(), offset, max_items)?;
    Ok(keys[range]
        .iter()
        .map(|key| PreparationSupportGeometryEntry {
            key: PreparationSupportGeometryKey {
                global_support_layer_index: key.global_support_layer_index,
                object_id: key.object_id.clone(),
                region_id: key.region_id,
                variant_chain: project_chain(&key.variant_chain),
            },
            polygons: map[key].clone(),
        })
        .collect())
}

fn project_target_source(source: &LayerTargetSource) -> InputResult<PreparationTargetSource> {
    Ok(match source {
        LayerTargetSource::Model { key, members } => {
            let mut members: Vec<_> = members.iter().collect();
            members.sort_by(|a, b| key_cmp(a, b));
            PreparationTargetSource::Model {
                key: key.into(),
                members: members.into_iter().map(Into::into).collect(),
            }
        }
        LayerTargetSource::SupportCarrier {
            entry_index,
            object_id,
            region_id,
            variant_chain,
            family,
            demand_ids,
            body_ids,
            provenance,
            anchor_layer_index,
        } => PreparationTargetSource::SupportCarrier {
            entry_index: count(*entry_index)?,
            object_id: object_id.clone(),
            region_id: *region_id,
            variant_chain: project_chain(variant_chain),
            family: family.clone(),
            demand_ids: demand_ids.clone(),
            body_ids: body_ids.clone(),
            provenance: provenance.clone(),
            anchor_layer_index: *anchor_layer_index,
        },
        LayerTargetSource::Raft { global_layer_index } => PreparationTargetSource::Raft {
            global_layer_index: *global_layer_index,
        },
        LayerTargetSource::AnchoredStage { anchor_layer_index } => {
            PreparationTargetSource::AnchoredStage {
                anchor_layer_index: *anchor_layer_index,
            }
        }
    })
}

#[cfg(feature = "preparation-test-fixtures")]
impl CommittedPreparationInputs {
    /// Inspect original Arc identity in controlled transport fixtures only.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn controlled_retains_inputs(
        &self,
        mesh: &Arc<MeshIR>,
        layers: &Arc<LayerPlanIR>,
        slices: &Arc<Vec<SliceIR>>,
        region_map: &Arc<RegionMapIR>,
        surface: Option<&Arc<SurfaceClassificationIR>>,
        seam: Option<&Arc<SeamPlanIR>>,
        support_analysis: Option<&Arc<SupportAnalysisIR>>,
        support_geometry: Option<&Arc<SupportGeometryIR>>,
        support_plan: Option<&Arc<SupportPlanIR>>,
    ) -> bool {
        fn same<T>(a: Option<&Arc<T>>, b: Option<&Arc<T>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        Arc::ptr_eq(&self.mesh, mesh)
            && Arc::ptr_eq(&self.layers, layers)
            && Arc::ptr_eq(&self.slices, slices)
            && Arc::ptr_eq(&self.region_map, region_map)
            && same(self.surface.as_ref(), surface)
            && same(self.seam.as_ref(), seam)
            && same(self.support_analysis.as_ref(), support_analysis)
            && same(self.support_geometry.as_ref(), support_geometry)
            && same(self.support_plan.as_ref(), support_plan)
    }
}

#[cfg(feature = "preparation-test-fixtures")]
impl PreparationInputData {
    /// Observe the actual owner backing's committed Arc, never a copied snapshot.
    #[doc(hidden)]
    pub fn controlled_retains_committed(&self, inputs: &Arc<CommittedPreparationInputs>) -> bool {
        self.committed
            .as_ref()
            .is_some_and(|backing| Arc::ptr_eq(&backing.inputs, inputs))
    }

    /// Preset checked input counters for controlled handler-overflow witnesses.
    #[doc(hidden)]
    pub fn controlled_set_input_counters(
        &self,
        read_calls: u64,
        returned_records: u64,
    ) -> InputResult<()> {
        let input = self
            .committed
            .as_ref()
            .ok_or(PreparationInputError::AccessDenied)?;
        let mut counters = input
            .counters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        counters.read_calls = read_calls;
        counters.returned_records = returned_records;
        Ok(())
    }

    /// Inspect checked read-call/returned-record accounting without payloads.
    #[doc(hidden)]
    pub fn controlled_input_counters(&self) -> InputResult<(u64, u64)> {
        let input = self
            .committed
            .as_ref()
            .ok_or(PreparationInputError::AccessDenied)?;
        let counters = input
            .counters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok((counters.read_calls, counters.returned_records))
    }
}
