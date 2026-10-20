// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintObject.cpp
// (`PrintObject::bridge_over_infill`, `PrintObject::prepare_lightning_infill_data`),
// src/libslic3r/Fill/Fill.cpp (`LayerRegion::make_fill`,
// `calculate_infill_rotation_angle`), src/libslic3r/Flow.cpp
// (`Flow::new_from_config_width`, `Flow::rounded_rectangle_extrusion_spacing`),
// src/libslic3r/PrintRegion.cpp (`PrintRegion::flow`)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
// `KernelError` is defined by the read-only packet model and carries an
// `EvidenceLocation` plus named fields; it is deliberately not boxed here
// because `model.rs` is out of this step's write scope.
#![allow(clippy::result_large_err)]
//! Gate A source-only planning-domain reconstruction.
//!
//! [`build_planning_domain`] derives the planning domains, boundary provenance,
//! geometry operations, Lightning parameters, scalar-source region, and per
//! region/layer fill policies from a [`GateASourceInput`] alone. It is pure: no
//! filesystem, environment, configuration lookup, reference/B handle, or host
//! algorithm is reachable from here.
//!
//! # Canonical semantics ported here
//!
//! * The planning `internal` / `void` / `temporary_anchor` split comes from the
//!   Lightning-specific section of `PrintObject::bridge_over_infill` (Phase 1
//!   gate, Phase 2 surface expansion, restoration seam). Phase 2 is:
//!   collect `stInternal` (`sparse`) and `stInternalSolid` (`solid`) surfaces of
//!   the layer, intersect `sparse` with `expand(candidate, 3 mm)` for every
//!   bridge candidate of the same region, union-safety-offset `solid`, then
//!   `sparse = sparse - solid`. The promoted pieces are the temporary anchors;
//!   `stInternalVoid` is never touched.
//! * Bridge candidates come from the gather/filter section of the same function:
//!   the lower layer's fill polygons are the unsupported area, the lower layer's
//!   non-`stInternal` surfaces are the supported area, both are opened by one
//!   solid-infill spacing, and each upper-layer `stInternalSolid` surface is
//!   intersected with what is left.
//! * Scalar inputs mirror the `FillLightning::Generator` constructor: the first
//!   shared region supplies the scalar-source config, the largest nozzle
//!   diameter is used, and `supporting_radius`, `wall_supporting_radius`,
//!   `prune_length`, and `straightening_max_distance` are derived from those raw
//!   values by the canonical constructor formulas (see [`resolve_parameters`]).
//! * Resolved fill angles mirror `LayerRegion::make_fill`: sparse infill uses
//!   `infill_direction` plus `sparse_infill_rotate_template`, solid infill uses
//!   `solid_infill_direction` plus `solid_infill_rotate_template`, top/bottom
//!   surfaces use their own `*_layer_direction` override when it is set, and the
//!   model-alignment offset is added when `align_infill_direction_to_model` is
//!   on. Angles are stored in the canonical 32-bit float field, so the value is
//!   `(angle as f32 + align as f32) as f64`.
//!
//! # Unit discipline
//!
//! Every length entering this module is an IR unit (`1 unit = 100 nm`, 10 000
//! units per millimetre). Canonical millimetre constants are therefore
//! multiplied by [`crate::polygon::UNITS_PER_MM`], never by the canonical
//! 1 nm-per-unit factor.

use std::collections::BTreeMap;

use slicer_ir::ExPolygon;

use crate::model::{
    BoundaryAddress, BoundaryContribution, BoundaryProvenance, ContributionRelation, DerivedRegionPolicy,
    DomainKind, EarlierLayerInput, EarlierOriginRecord, EarlierRegionInput, EarlierSurface,
    EarlierSurfaceKind, EvidenceLocation, FillPolicyInput, GateAConstruction, GateASourceInput,
    GeometryOpKind, GeometryOperand, GeometryOperation, InternalBridgeFilterPolicy, KernelError,
    LightningParameters, OperandRole, PlanningLayer, RawObjectConfig, RawRegionConfig, RawWidthSetting,
    RegionIdentity, SourceCheckpoint, SourceSegmentRef, SourceSurface, SourceSurfaceId,
};
use crate::polygon;

/// Canonical `expand(surface.new_polys, scaled<float>(3.0))` of
/// `PrintObject::bridge_over_infill` Phase 2, in IR units (3 mm).
pub const PHASE2_ANCHOR_EXPANSION_UNITS: f64 = 3.0 * polygon::UNITS_PER_MM;

/// Canonical `scale_(12.0)` from the small-leftover merge-back rule of
/// `PrintObject::bridge_over_infill`, in IR units (12 mm).
const LEFTOVER_MERGE_LENGTH_UNITS: f64 = 12.0 * polygon::UNITS_PER_MM;

/// Canonical `EPSILON` (`1e-4`) used by the candidate area comparison.
const AREA_EPSILON: f64 = 1e-4;

/// Canonical `auto_extrusion_width(frSolidInfill | frInfill, nozzle)`.
const AUTO_WIDTH_NOZZLE_FACTOR: f64 = 1.125;

// -----------------------------------------------------------------------------
// Planning-domain entry points
// -----------------------------------------------------------------------------

/// Derive the Gate A construction from source-only evidence.
///
/// The candidate-gather branch comes from
/// [`GateASourceInput::object_config`]'s `dont_filter_internal_bridges`, exactly
/// as canonical `PrintObject::bridge_over_infill` reads
/// `po->config().dont_filter_internal_bridges`. This entry point never chooses a
/// branch itself: when the source envelope does not record the key it returns a
/// named [`KernelError::RequiredSourceInputUnavailable`] rather than silently
/// running the OrcaSlicer default, which would report a config gap as a
/// scientific mismatch.
pub fn build_planning_domain(source: &GateASourceInput) -> Result<GateAConstruction, KernelError> {
    let policy = source
        .object_config
        .dont_filter_internal_bridges
        .ok_or_else(|| {
            missing(
                source,
                0,
                None,
                "object_config.dont_filter_internal_bridges",
                "the source envelope records no internal-bridge anchor filter policy, so the \
                 candidate-gather branch cannot be chosen; canonical reads it from the object config",
            )
        })?;
    build_planning_domain_with_policy(source, policy)
}

/// Derive the Gate A construction under an explicit candidate-filter branch.
///
/// This is the documented over-ride: it exists so a caller already holding the
/// recorded branch (or a unit test exercising one branch in isolation) can run
/// the construction without first building a [`GateASourceInput`] that carries
/// it. The gate itself must call [`build_planning_domain`].
pub fn build_planning_domain_with_policy(
    source: &GateASourceInput,
    policy: InternalBridgeFilterPolicy,
) -> Result<GateAConstruction, KernelError> {
    let provenance = index_provenance(source)?;
    let parameters = resolve_parameters(source)?;
    let scalar_source_region = source
        .canonical_region_order
        .first()
        .cloned()
        .ok_or_else(|| missing(source, 0, None, "canonical_region_order", "the canonical region order is empty"))?;
    let scalar_config = scalar_region_config(source, &scalar_source_region)?;

    if parameters.layer_height_mm <= 0.0 {
        return Err(invalid(
            source,
            0,
            None,
            "object_config.layer_height_mm must be positive",
        ));
    }

    let layers: Vec<LayerCtx<'_>> = source
        .layers
        .iter()
        .map(|layer| LayerCtx::build(source, layer, &provenance))
        .collect::<Result<_, _>>()?;

    let mut planning_layers: Vec<PlanningLayer> = Vec::with_capacity(layers.len());
    let mut policies: Vec<DerivedRegionPolicy> = Vec::new();
    let mut next_operation_id: u32 = 0;

    for (index, layer) in layers.iter().enumerate() {
        let lower = index
            .checked_sub(1)
            .and_then(|previous| layers.get(previous));
        // Phase 1 gate: the *lower* layer must have at least one sparse
        // (`stInternal`) fill surface in a Lightning region. `RawRegionConfig`
        // now carries `sparse_infill_pattern`, but the pattern test stays
        // implicit: this kernel is only ever invoked for Lightning objects, and
        // evaluating the string here would be a reconstruction-behaviour change
        // outside this step's scope.
        let expand = lower.map(|lower| lower.has_internal_surface()).unwrap_or(false);

        let spacing_units = solid_spacing_units(
            solid_width_mm(&source.object_config, parameters.nozzle_diameter_mm),
            layer.layer.effective_layer_height_mm,
        );

        let outcome = if expand {
            plan_layer_with_anchors(layer, lower, spacing_units, policy, &mut next_operation_id)
        } else {
            plan_layer_without_anchors(layer)
        };

        policies.extend(derive_policies(source, layer)?);

        let boundary_provenance = build_boundary_provenance(layer, &outcome);
        planning_layers.push(PlanningLayer {
            object_id: source.object_id.clone(),
            local_layer_index: layer.layer.local_layer_index,
            global_layer_index: layer.layer.global_layer_index,
            z_mm: layer.layer.z_mm,
            internal_domains: outcome.internal,
            void_domains: outcome.void,
            temporary_anchors: outcome.anchors,
            source_regions: layer.regions.iter().map(|r| r.identity.clone()).collect(),
            source_catalog: layer.surfaces.iter().map(|s| s.to_source_surface()).collect(),
            boundary_provenance,
            operations: outcome.operations,
        });
    }

    // The scalar-source region config must agree across layers: one shared
    // region has exactly one config, so a difference is measured evidence, not
    // a missing value.
    for layer in &layers {
        if let Some(region) = layer
            .regions
            .iter()
            .find(|r| r.identity == scalar_source_region)
        {
            if &region.input.config != scalar_config {
                return Err(KernelError::RequiredSourceInputMismatch {
                    context: location(source, layer.layer.global_layer_index, Some(&scalar_source_region)),
                    field: "region_config".to_string(),
                    expected: format!("{scalar_config:?}"),
                    actual: format!("{:?}", region.input.config),
                });
            }
        }
    }

    Ok(GateAConstruction {
        object_id: source.object_id.clone(),
        planning_layers,
        canonical_region_order: source.canonical_region_order.clone(),
        scalar_source_region,
        parameters,
        policies,
    })
}

// -----------------------------------------------------------------------------
// Per-layer source context
// -----------------------------------------------------------------------------

struct RegionCtx<'a> {
    identity: RegionIdentity,
    input: &'a EarlierRegionInput,
}

struct SurfaceRecord {
    /// Index into `LayerCtx::regions` (canonical region order).
    region_index: usize,
    kind: EarlierSurfaceKind,
    geometry: ExPolygon,
    id: SourceSurfaceId,
    detected_bridge_angle_rad: Option<f64>,
}

impl SurfaceRecord {
    fn to_source_surface(&self) -> SourceSurface {
        SourceSurface {
            id: self.id.clone(),
            geometry: self.geometry.clone(),
            surface_kind: surface_kind_name(self.kind).to_string(),
            // The earlier classification does not record a solid angle or a
            // proven-not-applicable bridge angle; absence is `None`.
            solid_angle_rad: None,
            bridge_angle_rad: None,
        }
    }

    fn segment_ref(&self, ring_id: Option<u32>, segment_id: Option<u32>) -> SourceSegmentRef {
        SourceSegmentRef {
            surface: self.id.clone(),
            ring_id,
            segment_id,
        }
    }

    /// Surface-level reference: the recorded occurrence names the surface only,
    /// so both leaf ids are absent. `0/0` would claim ring 0 segment 0, which the
    /// capture never means.
    fn surface_ref(&self) -> SourceSegmentRef {
        self.segment_ref(None, None)
    }
}

struct LayerCtx<'a> {
    layer: &'a EarlierLayerInput,
    regions: Vec<RegionCtx<'a>>,
    surfaces: Vec<SurfaceRecord>,
    /// Surface indices per canonical region, in input order.
    region_surfaces: Vec<Vec<usize>>,
}

impl<'a> LayerCtx<'a> {
    fn build(
        source: &GateASourceInput,
        layer: &'a EarlierLayerInput,
        provenance: &BTreeMap<u32, &'a EarlierOriginRecord>,
    ) -> Result<Self, KernelError> {
        if layer.regions.is_empty() {
            return Err(missing(
                source,
                layer.global_layer_index,
                None,
                "layers[].regions",
                "the layer carries no regions",
            ));
        }

        let mut regions: Vec<RegionCtx<'a>> = Vec::with_capacity(source.canonical_region_order.len());
        for identity in &source.canonical_region_order {
            let input = layer
                .regions
                .iter()
                .find(|region| &region.identity == identity)
                .ok_or_else(|| {
                    missing(
                        source,
                        layer.global_layer_index,
                        Some(identity),
                        "layers[].regions[].identity",
                        "the canonical region is absent from this layer",
                    )
                })?;
            regions.push(RegionCtx {
                identity: identity.clone(),
                input,
            });
        }
        for region in &layer.regions {
            if !source
                .canonical_region_order
                .iter()
                .any(|identity| identity == &region.identity)
            {
                return Err(KernelError::RequiredSourceInputMismatch {
                    context: location(source, layer.global_layer_index, Some(&region.identity)),
                    field: "canonical_region_order".to_string(),
                    expected: "every layer region appears in canonical_region_order".to_string(),
                    actual: format!("unlisted region {} on layer {}", region.identity.region_id, layer.global_layer_index),
                });
            }
        }

        let mut surfaces: Vec<SurfaceRecord> = Vec::new();
        let mut region_surfaces: Vec<Vec<usize>> = vec![Vec::new(); regions.len()];
        for (region_index, region) in regions.iter().enumerate() {
            for surface in &region.input.surfaces {
                validate_surface(source, layer, &region.identity, surface, provenance)?;
                region_surfaces[region_index].push(surfaces.len());
                surfaces.push(SurfaceRecord {
                    region_index,
                    kind: surface.kind,
                    geometry: polygon::normalize_expolygon(&surface.geometry),
                    id: SourceSurfaceId {
                        identity: region.identity.clone(),
                        local_layer_index: layer.local_layer_index,
                        global_layer_index: layer.global_layer_index,
                        checkpoint: SourceCheckpoint::BeforeAnchorEdit,
                        surface_token: surface.surface_token.clone(),
                    },
                    detected_bridge_angle_rad: surface.detected_bridge_angle_rad,
                });
            }
        }

        Ok(Self {
            layer,
            regions,
            surfaces,
            region_surfaces,
        })
    }

    fn indices_of_kind(&self, region_index: usize, kind: EarlierSurfaceKind) -> Vec<usize> {
        self.region_surfaces[region_index]
            .iter()
            .copied()
            .filter(|index| self.surfaces[*index].kind == kind)
            .collect()
    }

    fn geometries(&self, indices: &[usize]) -> Vec<ExPolygon> {
        indices
            .iter()
            .map(|index| self.surfaces[*index].geometry.clone())
            .collect()
    }

    /// Geometry of the region's fill surfaces only (canonical `fill_surfaces`
    /// never holds outlines or shells).
    fn fill_geometries(&self, region_index: usize) -> Vec<ExPolygon> {
        self.region_surfaces[region_index]
            .iter()
            .filter(|index| is_fill_surface(self.surfaces[**index].kind))
            .map(|index| self.surfaces[*index].geometry.clone())
            .collect()
    }

    fn has_internal_surface(&self) -> bool {
        self.surfaces
            .iter()
            .any(|surface| surface.kind == EarlierSurfaceKind::Internal)
    }

    fn config_of(&self, region_index: usize) -> &RawRegionConfig {
        &self.regions[region_index].input.config
    }
}

fn validate_surface(
    source: &GateASourceInput,
    layer: &EarlierLayerInput,
    identity: &RegionIdentity,
    surface: &EarlierSurface,
    provenance: &BTreeMap<u32, &EarlierOriginRecord>,
) -> Result<(), KernelError> {
    let context = location(source, layer.global_layer_index, Some(identity));
    let record = provenance.get(&surface.provenance_id).ok_or_else(|| {
        KernelError::SourceProvenanceUnavailable {
            context,
            reason: format!(
                "surface `{}` references provenance_id {} which has no origin record",
                surface.surface_token, surface.provenance_id
            ),
        }
    })?;
    if record.source_geometry_sha256.is_empty() || record.raw_config_sha256.is_empty() {
        return Err(KernelError::SourceProvenanceUnavailable {
            context: location(source, layer.global_layer_index, Some(identity)),
            reason: format!(
                "origin record {} for surface `{}` has an empty evidence hash",
                record.provenance_id, surface.surface_token
            ),
        });
    }
    Ok(())
}

fn index_provenance(
    source: &GateASourceInput,
) -> Result<BTreeMap<u32, &EarlierOriginRecord>, KernelError> {
    let mut index: BTreeMap<u32, &EarlierOriginRecord> = BTreeMap::new();
    for record in &source.provenance {
        if index.insert(record.provenance_id, record).is_some() {
            return Err(KernelError::SourceProvenanceMismatch {
                context: location(source, 0, None),
                expected: format!("one origin record per provenance_id (duplicate {})", record.provenance_id),
                actual: format!("{} records", source.provenance.len()),
            });
        }
    }
    Ok(index)
}

// -----------------------------------------------------------------------------
// Planning layer construction
// -----------------------------------------------------------------------------

struct LayerOutcome {
    internal: Vec<ExPolygon>,
    void: Vec<ExPolygon>,
    anchors: Vec<ExPolygon>,
    operations: Vec<GeometryOperation>,
    /// `(operation_id, input surface indices)` parallel to `anchors`, for
    /// generated-boundary attribution.
    anchor_provenance: Vec<(u32, Vec<usize>)>,
}

impl LayerOutcome {
    fn plain(layer: &LayerCtx<'_>) -> Self {
        let mut internal = Vec::new();
        let mut void = Vec::new();
        for region_index in 0..layer.regions.len() {
            internal.extend(layer.geometries(&layer.indices_of_kind(region_index, EarlierSurfaceKind::Internal)));
            void.extend(layer.geometries(&layer.indices_of_kind(region_index, EarlierSurfaceKind::Void)));
        }
        Self {
            internal,
            void,
            anchors: Vec::new(),
            operations: Vec::new(),
            anchor_provenance: Vec::new(),
        }
    }
}

fn plan_layer_without_anchors(layer: &LayerCtx<'_>) -> LayerOutcome {
    LayerOutcome::plain(layer)
}

fn plan_layer_with_anchors(
    layer: &LayerCtx<'_>,
    lower: Option<&LayerCtx<'_>>,
    spacing_units: i64,
    policy: InternalBridgeFilterPolicy,
    next_operation_id: &mut u32,
) -> LayerOutcome {
    let candidates = match lower {
        Some(lower) => gather_candidates(layer, lower, spacing_units, policy),
        None => Vec::new(),
    };
    // Phase 1 gate: `needs_lightning_expansion[lidx]` is only set for layers that
    // have at least one candidate surface *and* whose lower layer carries sparse
    // Lightning infill. Without a candidate the layer's surfaces are untouched.
    if candidates.is_empty() {
        return LayerOutcome::plain(layer);
    }

    let mut internal: Vec<ExPolygon> = Vec::new();
    let mut anchors: Vec<ExPolygon> = Vec::new();
    let mut operation_bodies: Vec<OperationBody> = Vec::new();
    let mut anchor_provenance: Vec<(u32, Vec<usize>)> = Vec::new();
    let mut stopped = false;

    for region_index in 0..layer.regions.len() {
        if stopped {
            // `bridge_over_infill` breaks out of the region loop once a region
            // has no sparse infill left to expand.
            internal.extend(layer.geometries(&layer.indices_of_kind(region_index, EarlierSurfaceKind::Internal)));
            continue;
        }
        let sparse_indices = layer.indices_of_kind(region_index, EarlierSurfaceKind::Internal);
        let solid_indices = layer.indices_of_kind(region_index, EarlierSurfaceKind::InternalSolid);
        if sparse_indices.is_empty() {
            stopped = true;
            continue;
        }
        let sparse = layer.geometries(&sparse_indices);
        let solid = layer.geometries(&solid_indices);

        let mut region_anchors: Vec<ExPolygon> = Vec::new();
        for candidate in candidates
            .iter()
            .filter(|candidate| candidate.region_index == region_index)
        {
            let halo = polygon::expand(&candidate.new_polys, PHASE2_ANCHOR_EXPANSION_UNITS);
            region_anchors.extend(polygon::intersection(&sparse, &halo));
        }

        let mut promoted = solid.clone();
        promoted.extend(region_anchors.iter().cloned());
        let solid_union = polygon::union_safety_offset(&promoted);
        // Derived geometry has no source order to preserve, so these helpers
        // return a deterministic total order (see `polygon::normalize_expolygons`).
        let sparse_after = polygon::difference(&sparse, &solid_union);

        // AnchorExpansion -> Union -> Difference, the Phase 2 causal chain.
        let expansion_id = *next_operation_id;
        let union_id = expansion_id + 1;
        let difference_id = expansion_id + 2;
        *next_operation_id += 3;
        operation_bodies.push(OperationBody {
            expansion_id,
            union_id,
            difference_id,
            solid_indices: solid_indices.clone(),
            sparse_indices: sparse_indices.clone(),
            anchors: region_anchors.clone(),
            solid_union,
            sparse_after: sparse_after.clone(),
        });
        let inputs: Vec<usize> = solid_indices
            .iter()
            .chain(sparse_indices.iter())
            .copied()
            .collect();
        for _ in 0..region_anchors.len() {
            anchor_provenance.push((expansion_id, inputs.clone()));
        }

        internal.extend(sparse_after);
        anchors.extend(region_anchors);
    }

    let mut void: Vec<ExPolygon> = Vec::new();
    for region_index in 0..layer.regions.len() {
        void.extend(layer.geometries(&layer.indices_of_kind(region_index, EarlierSurfaceKind::Void)));
    }

    let mut outcome = LayerOutcome {
        internal,
        void,
        anchors,
        operations: Vec::new(),
        anchor_provenance,
    };

    if !operation_bodies.is_empty() {
        outcome.operations = build_operations(layer, &operation_bodies, &outcome.anchors);
    }
    outcome
}

struct OperationBody {
    expansion_id: u32,
    union_id: u32,
    difference_id: u32,
    solid_indices: Vec<usize>,
    sparse_indices: Vec<usize>,
    anchors: Vec<ExPolygon>,
    solid_union: Vec<ExPolygon>,
    sparse_after: Vec<ExPolygon>,
}

fn build_operations(
    layer: &LayerCtx<'_>,
    bodies: &[OperationBody],
    anchors: &[ExPolygon],
) -> Vec<GeometryOperation> {
    let mut operations = Vec::new();
    let mut anchor_cursor = 0usize;
    for body in bodies {
        let subject_refs: Vec<SourceSegmentRef> = body
            .solid_indices
            .iter()
            .map(|index| layer.surfaces[*index].surface_ref())
            .collect();
        let clip_refs: Vec<SourceSegmentRef> = body
            .sparse_indices
            .iter()
            .map(|index| layer.surfaces[*index].surface_ref())
            .collect();
        let mut inputs = subject_refs.clone();
        inputs.extend(clip_refs.iter().cloned());

        let body_start = anchor_cursor;
        anchor_cursor += body.anchors.len();
        let anchor_addresses: Vec<BoundaryAddress> = (0..anchors.len())
            .filter(|index| *index >= body_start && *index < body_start + body.anchors.len())
            .flat_map(|index| domain_boundary_addresses(DomainKind::TemporaryAnchor, index, &anchors[index]))
            .collect();

        operations.push(GeometryOperation {
            operation_id: body.expansion_id,
            kind: GeometryOpKind::AnchorExpansion,
            // The Phase-2 anchor chain runs immediately after the anchor edit and
            // is recorded at that checkpoint (`post_anchor`), not `during_planning`.
            checkpoint: SourceCheckpoint::PostAnchor,
            inputs,
            parent_operations: Vec::new(),
            operands: vec![
                GeometryOperand {
                    role: OperandRole::Subject,
                    sources: subject_refs.clone(),
                    operation_id: None,
                },
                GeometryOperand {
                    role: OperandRole::Clip,
                    sources: clip_refs.clone(),
                    operation_id: None,
                },
            ],
            // `expand(new_polys, scaled<float>(3.0))`.
            offset_units: Some(PHASE2_ANCHOR_EXPANSION_UNITS as i64),
            join: Some("miter".to_string()),
            result_geometry: body.anchors.clone(),
            outputs: anchor_addresses,
        });

        operations.push(GeometryOperation {
            operation_id: body.union_id,
            kind: GeometryOpKind::Union,
            checkpoint: SourceCheckpoint::PostAnchor,
            inputs: subject_refs.clone(),
            parent_operations: vec![body.expansion_id],
            operands: vec![
                GeometryOperand {
                    role: OperandRole::Subject,
                    sources: subject_refs,
                    operation_id: None,
                },
                GeometryOperand {
                    role: OperandRole::Clip,
                    sources: Vec::new(),
                    operation_id: Some(body.expansion_id),
                },
            ],
            offset_units: None,
            join: None,
            result_geometry: body.solid_union.clone(),
            outputs: Vec::new(),
        });

        operations.push(GeometryOperation {
            operation_id: body.difference_id,
            kind: GeometryOpKind::Difference,
            checkpoint: SourceCheckpoint::PostAnchor,
            inputs: clip_refs.clone(),
            parent_operations: vec![body.union_id],
            operands: vec![
                GeometryOperand {
                    role: OperandRole::Subject,
                    sources: clip_refs,
                    operation_id: None,
                },
                GeometryOperand {
                    role: OperandRole::Clip,
                    sources: Vec::new(),
                    operation_id: Some(body.union_id),
                },
            ],
            offset_units: None,
            join: None,
            result_geometry: body.sparse_after.clone(),
            outputs: Vec::new(),
        });
    }
    operations
}

fn domain_boundary_addresses(kind: DomainKind, surface_id: usize, poly: &ExPolygon) -> Vec<BoundaryAddress> {
    let mut addresses = Vec::new();
    for (ring_id, ring) in std::iter::once(&poly.contour).chain(poly.holes.iter()).enumerate() {
        for segment_id in 0..ring.points.len() {
            addresses.push(BoundaryAddress {
                domain: kind,
                surface_id: surface_id as u32,
                ring_id: ring_id as u32,
                segment_id: segment_id as u32,
            });
        }
    }
    addresses
}

// -----------------------------------------------------------------------------
// Bridge candidate gather (`PrintObject::bridge_over_infill`)
// -----------------------------------------------------------------------------

struct AnchorCandidate {
    region_index: usize,
    new_polys: Vec<ExPolygon>,
}

fn gather_candidates(
    layer: &LayerCtx<'_>,
    lower: &LayerCtx<'_>,
    spacing_units: i64,
    policy: InternalBridgeFilterPolicy,
) -> Vec<AnchorCandidate> {
    let spacing = spacing_units as f64;
    if spacing <= 0.0 {
        return Vec::new();
    }
    // `dont_filter_internal_bridges != ibfDisabled` lowers the opening multiplier
    // from 3 to 1.
    let expansion_multiplier = if policy == InternalBridgeFilterPolicy::Disabled {
        3.0
    } else {
        1.0
    };

    // The lower layer's whole fill area starts out considered unsupported, and
    // its non-sparse surfaces are the parts that already support the layer above.
    let mut unsupported_area: Vec<ExPolygon> = Vec::new();
    let mut lower_layer_solids: Vec<ExPolygon> = Vec::new();
    for (region_index, _) in lower.regions.iter().enumerate() {
        unsupported_area.extend(lower.fill_geometries(region_index));
        let sparse_is_dense = lower.config_of(region_index).density_percent >= 100.0;
        for surface in &lower.surfaces {
            if surface.region_index != region_index {
                continue;
            }
            if is_fill_surface(surface.kind)
                && (surface.kind != EarlierSurfaceKind::Internal || sparse_is_dense)
            {
                lower_layer_solids.push(surface.geometry.clone());
            }
        }
    }
    unsupported_area = polygon::closing(&unsupported_area, polygon::SCALED_EPSILON_UNITS);

    lower_layer_solids = polygon::shrink(&lower_layer_solids, spacing);
    lower_layer_solids = polygon::expand(&lower_layer_solids, (1.0 + expansion_multiplier) * spacing);
    unsupported_area = polygon::shrink(&unsupported_area, expansion_multiplier * spacing);
    unsupported_area = polygon::difference(&unsupported_area, &lower_layer_solids);

    let mut candidates = Vec::new();
    for region_index in 0..layer.regions.len() {
        for index in layer.indices_of_kind(region_index, EarlierSurfaceKind::InternalSolid) {
            let surface = &layer.surfaces[index];
            let unsupported = polygon::intersection(
                std::slice::from_ref(&surface.geometry),
                &unsupported_area,
            );
            if unsupported.is_empty() {
                continue;
            }
            let new_polys = match policy {
                InternalBridgeFilterPolicy::NoFilter => {
                    // "expand the unsupported area by 4x spacing to trigger
                    // internal bridging", no area filter.
                    polygon::expand(&unsupported, 4.0 * spacing)
                }
                InternalBridgeFilterPolicy::Disabled | InternalBridgeFilterPolicy::Limited => {
                    let surface_area = polygon::expolygon_area(&surface.geometry);
                    let unsupported_area_values = polygon::area(&unsupported);
                    let partially_supported = unsupported_area_values < surface_area - AREA_EPSILON;
                    if partially_supported && unsupported_area_values <= 9.0 * spacing * spacing {
                        // Too small to be worth bridging; skipped exactly like the
                        // canonical area filter.
                        continue;
                    }
                    let mut worth_bridging = polygon::intersection(
                        std::slice::from_ref(&surface.geometry),
                        &polygon::expand(&unsupported, 4.0 * spacing),
                    );
                    // Merge tiny leftovers back so the surface is not broken up.
                    let leftovers = polygon::difference(
                        std::slice::from_ref(&surface.geometry),
                        &polygon::expand(&worth_bridging, spacing),
                    );
                    for leftover in leftovers {
                        let leftover_area = polygon::expolygon_area(&leftover);
                        if leftover_area < spacing * LEFTOVER_MERGE_LENGTH_UNITS
                            && leftover_area > spacing * spacing
                        {
                            worth_bridging.push(leftover);
                        }
                    }
                    polygon::intersection(
                        &polygon::closing(&worth_bridging, polygon::SCALED_EPSILON_UNITS),
                        std::slice::from_ref(&surface.geometry),
                    )
                }
            };
            if new_polys.is_empty() {
                continue;
            }
            candidates.push(AnchorCandidate {
                region_index,
                new_polys,
            });
        }
    }
    candidates
}

// -----------------------------------------------------------------------------
// Boundary provenance
// -----------------------------------------------------------------------------

fn build_boundary_provenance(layer: &LayerCtx<'_>, outcome: &LayerOutcome) -> Vec<BoundaryProvenance> {
    let mut provenance = Vec::new();
    // Inherited domains: every boundary edge is attributed to each source segment
    // it coincides with, preserving multiplicity (a seam edge belongs to both
    // neighbouring surfaces).
    for (kind, domains) in [
        (DomainKind::Internal, &outcome.internal),
        (DomainKind::Void, &outcome.void),
    ] {
        for (surface_id, poly) in domains.iter().enumerate() {
            push_inherited_provenance(&mut provenance, layer, kind, surface_id, poly);
        }
    }
    // Generated domains: the operation lineage, one contributor per operation
    // input surface.
    for (surface_id, poly) in outcome.anchors.iter().enumerate() {
        let Some((operation_id, inputs)) = outcome.anchor_provenance.get(surface_id) else {
            continue;
        };
        for (ring_id, ring) in std::iter::once(&poly.contour).chain(poly.holes.iter()).enumerate() {
            for segment_id in 0..ring.points.len() {
                let start = ring.points[segment_id];
                let end = ring.points[(segment_id + 1) % ring.points.len()];
                provenance.push(BoundaryProvenance {
                    address: BoundaryAddress {
                        domain: DomainKind::TemporaryAnchor,
                        surface_id: surface_id as u32,
                        ring_id: ring_id as u32,
                        segment_id: segment_id as u32,
                    },
                    endpoints: [start, end],
                    contributors: inputs
                        .iter()
                        .map(|index| BoundaryContribution {
                            // A generated edge has no source ring/segment; the
                            // recorder writes `null` for both, so the identity is
                            // surface-level. `0/0` would be a different claim.
                            source: layer.surfaces[*index].surface_ref(),
                            relation: ContributionRelation::Generated {
                                operation_id: *operation_id,
                            },
                        })
                        .collect(),
                });
            }
        }
    }
    provenance
}

fn push_inherited_provenance(
    out: &mut Vec<BoundaryProvenance>,
    layer: &LayerCtx<'_>,
    kind: DomainKind,
    surface_id: usize,
    poly: &ExPolygon,
) {
    for (ring_id, ring) in std::iter::once(&poly.contour).chain(poly.holes.iter()).enumerate() {
        for segment_id in 0..ring.points.len() {
            let start = ring.points[segment_id];
            let end = ring.points[(segment_id + 1) % ring.points.len()];
            let edge = [start, end];
            let mut contributors = Vec::new();
            for surface in &layer.surfaces {
                for (source_ring_id, source_ring) in std::iter::once(&surface.geometry.contour)
                    .chain(surface.geometry.holes.iter())
                    .enumerate()
                {
                    let points = &source_ring.points;
                    if points.len() < 2 {
                        continue;
                    }
                    for source_segment_id in 0..points.len() {
                        let segment = [
                            points[source_segment_id],
                            points[(source_segment_id + 1) % points.len()],
                        ];
                        if let Some((start_t, end_t)) = polygon::segment_overlap(edge, segment) {
                            contributors.push(BoundaryContribution {
                                source: surface.segment_ref(
                                    Some(source_ring_id as u32),
                                    Some(source_segment_id as u32),
                                ),
                                relation: ContributionRelation::Overlap {
                                    start: start_t,
                                    end: end_t,
                                },
                            });
                        }
                    }
                }
            }
            out.push(BoundaryProvenance {
                address: BoundaryAddress {
                    domain: kind,
                    surface_id: surface_id as u32,
                    ring_id: ring_id as u32,
                    segment_id: segment_id as u32,
                },
                endpoints: edge,
                contributors,
            });
        }
    }
}

// -----------------------------------------------------------------------------
// Parameters, scalar source, and policies
// -----------------------------------------------------------------------------

/// The `FillLightning::Generator` constructor scalars, derived from the raw
/// config that `a_source` carries.
///
/// Canonical `FillLightning::Generator::Generator` (`Generator.cpp`) derives
/// `supporting_radius` from the infill extrusion width, `fill_multiline` and
/// `sparse_infill_density`, and the three angle-scaled lengths from the object
/// layer height and the three Lightning angles:
///
/// ```text
/// supporting_radius          = width * multiline / density
/// wall_supporting_radius     = layer_height * tan(overhang_angle)
/// prune_length               = layer_height * tan(prune_angle)
/// straightening_max_distance = layer_height * tan(straightening_angle)
/// ```
///
/// Canonical evaluates these in integer `coord_t` units before unscaling them
/// for the record; this kernel derives the exact real-valued millimetre scalar
/// from the same recorded inputs, so neither side invents precision. Every input
/// is recorded in `a_source`. Nothing is defaulted: a zero density, where the
/// canonical `supporting_radius` expression divides by zero, yields an absent
/// `supporting_radius` rather than an invented one.
fn resolve_parameters(source: &GateASourceInput) -> Result<LightningParameters, KernelError> {
    let nozzle_diameter_mm = source
        .object_config
        .nozzle_diameters_mm
        .iter()
        .copied()
        .fold(None::<f64>, |acc, value| match acc {
            Some(current) if current >= value => Some(current),
            _ => Some(value),
        })
        .ok_or_else(|| missing(source, 0, None, "object_config.nozzle_diameters_mm", "no nozzle diameter was recorded"))?;
    if nozzle_diameter_mm <= 0.0 {
        return Err(invalid(
            source,
            0,
            None,
            "object_config.nozzle_diameters_mm contains no positive diameter",
        ));
    }

    let region = scalar_region_config(source, source.canonical_region_order.first().ok_or_else(|| {
        missing(source, 0, None, "canonical_region_order", "the canonical region order is empty")
    })?)?;
    if !(0.0..=100.0).contains(&region.density_percent) {
        return Err(invalid(
            source,
            0,
            source.canonical_region_order.first(),
            "region_config.density_percent must be within 0..=100",
        ));
    }

    let extrusion_width_mm = sparse_width_mm(region, &source.object_config, nozzle_diameter_mm);
    let density_fraction = region.density_percent / 100.0;
    let layer_height_mm = source.object_config.layer_height_mm;
    // `coord_t(infill_extrusion_width) * 100 * n_multiline / sparse_infill_density`;
    // `* 100 / percent` is `/ fraction`.
    let supporting_radius_mm = if density_fraction > 0.0 {
        Some(extrusion_width_mm * region.fill_multiline as f64 / density_fraction)
    } else {
        None
    };

    Ok(LightningParameters {
        nozzle_diameter_mm,
        extrusion_width_mm,
        density_fraction,
        multiline: region.fill_multiline,
        layer_height_mm,
        overhang_angle_deg: region.overhang_angle_deg,
        prune_angle_deg: region.prune_angle_deg,
        straightening_angle_deg: region.straightening_angle_deg,
        supporting_radius_mm,
        wall_supporting_radius_mm: angle_distance_mm(layer_height_mm, region.overhang_angle_deg),
        prune_length_mm: angle_distance_mm(layer_height_mm, region.prune_angle_deg),
        straightening_max_distance_mm: angle_distance_mm(
            layer_height_mm,
            region.straightening_angle_deg,
        ),
    })
}

/// Canonical `angle_distance`: `layer_thickness * tan(angle)`, in millimetres.
///
/// Canonical `Generator::Generator` stores `coord_t(layer_thickness * std::tan(angle))`;
/// the angle is converted from the recorded degrees exactly once.
fn angle_distance_mm(layer_height_mm: f64, angle_deg: f64) -> f64 {
    layer_height_mm * deg_to_rad(angle_deg).tan()
}

fn scalar_region_config<'a>(
    source: &'a GateASourceInput,
    identity: &RegionIdentity,
) -> Result<&'a RawRegionConfig, KernelError> {
    // `FillLightning::Generator` reads `shared_regions()->all_regions.front()`,
    // i.e. one shared region config for the whole object.
    for layer in &source.layers {
        if let Some(region) = layer.regions.iter().find(|region| &region.identity == identity) {
            return Ok(&region.config);
        }
    }
    Err(missing(
        source,
        0,
        Some(identity),
        "canonical_region_order[0]",
        "the scalar-source region has no recorded config on any layer",
    ))
}

/// `get_abs_value` of a raw width option: `None` means "auto" (zero).
fn width_abs_mm(setting: &RawWidthSetting, nozzle_mm: f64) -> Option<f64> {
    match setting {
        RawWidthSetting::Automatic => None,
        RawWidthSetting::AbsoluteMm(value) if *value > 0.0 => Some(*value),
        RawWidthSetting::AbsoluteMm(_) => None,
        RawWidthSetting::NozzlePercent(percent) if *percent > 0.0 => {
            // Canonical `ConfigOptionFloatOrPercent::get_abs_value`:
            // `ratio_over * value / 100`.
            Some(nozzle_mm * percent / 100.0)
        }
        RawWidthSetting::NozzlePercent(_) => None,
    }
}

/// `sparse_infill_line_width`, falling back to the object `line_width`, then to
/// `Flow::auto_extrusion_width(frInfill, nozzle)`.
fn sparse_width_mm(region: &RawRegionConfig, object: &RawObjectConfig, nozzle_mm: f64) -> f64 {
    width_abs_mm(&region.sparse_width, nozzle_mm)
        .or_else(|| width_abs_mm(&object.line_width, nozzle_mm))
        .unwrap_or(AUTO_WIDTH_NOZZLE_FACTOR * nozzle_mm)
}

/// Solid-infill extrusion width.
///
/// **Schema gap.** Canonical `PrintRegion::flow(frSolidInfill, ...)` reads
/// `internal_solid_infill_line_width`, which `RawRegionConfig` does not carry.
/// The reconstruction can therefore only follow the canonical zero-value
/// fallback to the object `line_width` (and then to
/// `Flow::auto_extrusion_width(frSolidInfill, nozzle)`).
fn solid_width_mm(object: &RawObjectConfig, nozzle_mm: f64) -> f64 {
    width_abs_mm(&object.line_width, nozzle_mm).unwrap_or(AUTO_WIDTH_NOZZLE_FACTOR * nozzle_mm)
}

/// `Flow::scaled_spacing()` for a solid-infill flow, in IR units.
///
/// `spacing = width - height * (1 - PI/4)` in 32-bit floats (canonical
/// `Flow::rounded_rectangle_extrusion_spacing`), then `coord_t(scale_(spacing))`
/// truncates to IR units.
fn solid_spacing_units(width_mm: f64, layer_height_mm: f64) -> i64 {
    let width = width_mm as f32;
    let height = layer_height_mm as f32;
    let spacing = width - height * (1.0 - 0.25 * std::f32::consts::PI);
    if spacing <= 0.0 {
        return 0;
    }
    (spacing as f64 * polygon::UNITS_PER_MM) as i64
}

fn derive_policies(
    source: &GateASourceInput,
    layer: &LayerCtx<'_>,
) -> Result<Vec<DerivedRegionPolicy>, KernelError> {
    let mut policies = Vec::with_capacity(layer.regions.len());
    for (region_index, region) in layer.regions.iter().enumerate() {
        let config = &region.input.config;
        let align_offset = if config.align_to_model {
            source.object_config.model_alignment_offset_rad
        } else {
            0.0
        };
        let context = location(source, layer.layer.global_layer_index, Some(&region.identity));

        let resolved_sparse_angle_rad = rotation_angle(
            config.infill_direction_deg,
            &config.sparse_rotate_template,
            align_offset,
            &context,
            "sparse_infill_rotate_template",
        )?;
        let resolved_solid_angle_rad = rotation_angle(
            config.solid_infill_direction_deg,
            &config.solid_rotate_template,
            align_offset,
            &context,
            "solid_infill_rotate_template",
        )?;
        let resolved_top_angle_rad = if config.top_layer_direction_deg >= 0.0 {
            Some(apply_alignment(
                deg_to_rad(config.top_layer_direction_deg),
                align_offset,
            ))
        } else {
            None
        };
        let resolved_bottom_angle_rad = if config.bottom_layer_direction_deg >= 0.0 {
            Some(apply_alignment(
                deg_to_rad(config.bottom_layer_direction_deg),
                align_offset,
            ))
        } else {
            None
        };

        let bridge_angles: Vec<Option<f64>> = layer.region_surfaces[region_index]
            .iter()
            .filter(|index| layer.surfaces[**index].kind == EarlierSurfaceKind::InternalBridge)
            .map(|index| layer.surfaces[*index].detected_bridge_angle_rad)
            .collect();
        let is_bridge = !bridge_angles.is_empty();
        let bridge_angle_rad = if is_bridge
            && bridge_angles.iter().all(|angle| angle.is_some())
            && bridge_angles
                .iter()
                .all(|angle| angle == &bridge_angles[0])
        {
            bridge_angles[0]
        } else {
            None
        };

        policies.push(DerivedRegionPolicy {
            identity: region.identity.clone(),
            global_layer_index: layer.layer.global_layer_index,
            policy: FillPolicyInput {
                infill_direction_deg: config.infill_direction_deg,
                sparse_rotate_template: config.sparse_rotate_template.clone(),
                solid_infill_direction_deg: config.solid_infill_direction_deg,
                solid_rotate_template: config.solid_rotate_template.clone(),
                top_layer_direction_deg: config.top_layer_direction_deg,
                bottom_layer_direction_deg: config.bottom_layer_direction_deg,
                align_to_model: config.align_to_model,
                model_alignment_offset_rad: source.object_config.model_alignment_offset_rad,
                resolved_sparse_angle_rad,
                resolved_solid_angle_rad,
                resolved_top_angle_rad,
                resolved_bottom_angle_rad,
                // Canonical `FillParams::fixed_angle` for sparse infill: a
                // non-empty rotate template fixes the direction. Top/bottom
                // fixedness is carried losslessly by the `Option` above, which is
                // `Some` exactly when the canonical direction override is set.
                fixed_angle: !config.sparse_rotate_template.is_empty(),
                is_bridge,
                bridge_angle_rad,
            },
        });
    }
    Ok(policies)
}

fn rotation_angle(
    direction_deg: f64,
    template: &str,
    align_offset: f64,
    context: &EvidenceLocation,
    field: &str,
) -> Result<f64, KernelError> {
    if !template.is_empty() {
        // `calculate_infill_rotation_angle` tokenizes a rotate-template
        // metalanguage (ranges, absolute/relative accumulation, Z ranges). That
        // evaluator is not ported here, and the resolved angle cannot be
        // reconstructed from a source record that carries the template only.
        return Err(KernelError::RequiredSourceInputUnavailable {
            context: context.clone(),
            field: field.to_string(),
            reason: format!(
                "rotate-template metalanguage `{template}` cannot be evaluated, so the resolved angle is unavailable"
            ),
        });
    }
    Ok(apply_alignment(deg_to_rad(direction_deg), align_offset))
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// Canonical `params.angle` is a 32-bit float field, and the alignment offset is
/// added to it in that width.
fn apply_alignment(angle: f64, align_offset: f64) -> f64 {
    ((angle as f32) + (align_offset as f32)) as f64
}

// -----------------------------------------------------------------------------
// Names, errors, and locations
// -----------------------------------------------------------------------------

/// Canonical `Surface` kinds that live in a layer region's `fill_surfaces`.
fn is_fill_surface(kind: EarlierSurfaceKind) -> bool {
    !matches!(
        kind,
        EarlierSurfaceKind::Outline | EarlierSurfaceKind::Shell
    )
}

fn surface_kind_name(kind: EarlierSurfaceKind) -> &'static str {
    match kind {
        EarlierSurfaceKind::Outline => "outline",
        EarlierSurfaceKind::Internal => "internal",
        EarlierSurfaceKind::Void => "void",
        EarlierSurfaceKind::TopSolid => "top",
        EarlierSurfaceKind::BottomSolid => "bottom",
        EarlierSurfaceKind::BottomBridge => "bottom_bridge",
        EarlierSurfaceKind::InternalSolid => "internal_solid",
        EarlierSurfaceKind::InternalBridge => "internal_bridge",
        EarlierSurfaceKind::Shell => "shell",
    }
}

fn location(
    source: &GateASourceInput,
    global_layer_index: u32,
    region: Option<&RegionIdentity>,
) -> EvidenceLocation {
    EvidenceLocation {
        object_id: source.object_id.clone(),
        global_layer_index,
        region: region.cloned(),
        source: None,
        checkpoint: None,
    }
}

fn missing(
    source: &GateASourceInput,
    global_layer_index: u32,
    region: Option<&RegionIdentity>,
    field: &str,
    reason: &str,
) -> KernelError {
    KernelError::RequiredSourceInputUnavailable {
        context: location(source, global_layer_index, region),
        field: field.to_string(),
        reason: reason.to_string(),
    }
}

fn invalid(
    source: &GateASourceInput,
    global_layer_index: u32,
    region: Option<&RegionIdentity>,
    reason: &str,
) -> KernelError {
    KernelError::InvalidInput {
        context: location(source, global_layer_index, region),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        EarlierOriginRecord, EarlierProducer, EarlierSourcePhase, RawObjectConfig, RawRegionConfig,
        RawWidthSetting, RegionIdentity,
    };
    use slicer_ir::{PaintValue, Point2, Polygon};

    fn identity(region_id: u64) -> RegionIdentity {
        RegionIdentity {
            object_id: "object-a".to_string(),
            region_id,
            variant_chain: Vec::< (String, PaintValue) >::new(),
        }
    }

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn region_config() -> RawRegionConfig {
        RawRegionConfig {
            fill_multiline: 1,
            sparse_width: RawWidthSetting::AbsoluteMm(0.45),
            density_percent: 15.0,
            overhang_angle_deg: 45.0,
            prune_angle_deg: 5.0,
            straightening_angle_deg: 5.0,
            infill_direction_deg: 45.0,
            sparse_rotate_template: String::new(),
            solid_infill_direction_deg: 45.0,
            solid_rotate_template: String::new(),
            top_layer_direction_deg: 45.0,
            bottom_layer_direction_deg: 45.0,
            align_to_model: false,
            sparse_infill_pattern: "lightning".to_string(),
        }
    }

    struct SurfaceSpec {
        kind: EarlierSurfaceKind,
        geometry: ExPolygon,
    }

    fn surface(spec: &SurfaceSpec, token: &str, provenance_id: u32) -> EarlierSurface {
        EarlierSurface {
            surface_token: token.to_string(),
            kind: spec.kind,
            geometry: spec.geometry.clone(),
            detected_bridge_angle_rad: None,
            provenance_id,
        }
    }

    /// Two layers: the lower one has a sparse strip, the upper one has a sparse
    /// strip that overhangs an internal solid. Built to exercise the Phase 2
    /// expansion without any reference data.
    fn source_with_layers(layers: Vec<Vec<Vec<SurfaceSpec>>>) -> GateASourceInput {
        let mut provenance = Vec::new();
        let mut next_id = 0u32;
        let mut earlier_layers = Vec::new();
        for (layer_index, regions) in layers.into_iter().enumerate() {
            let mut earlier_regions = Vec::new();
            for (region_index, surfaces) in regions.into_iter().enumerate() {
                let mut earlier_surfaces = Vec::new();
                for (surface_index, spec) in surfaces.iter().enumerate() {
                    provenance.push(EarlierOriginRecord {
                        provenance_id: next_id,
                        phase: EarlierSourcePhase::EarlierClassification,
                        producer: EarlierProducer::SliceGeometry,
                        source_geometry_sha256: format!("sha-{next_id}"),
                        raw_config_sha256: "sha-config".to_string(),
                    });
                    let token = format!("o0_l{layer_index}_r{region_index}_s{surface_index}");
                    earlier_surfaces.push(surface(spec, &token, next_id));
                    next_id += 1;
                }
                earlier_regions.push(EarlierRegionInput {
                    identity: identity(region_index as u64),
                    surfaces: earlier_surfaces,
                    top_shell_index: Some(0),
                    bottom_shell_index: Some(0),
                    config: region_config(),
                });
            }
            earlier_layers.push(EarlierLayerInput {
                local_layer_index: layer_index as u32,
                global_layer_index: layer_index as u32,
                z_mm: 0.2 * (layer_index as f64 + 1.0),
                effective_layer_height_mm: 0.2,
                regions: earlier_regions,
            });
        }
        GateASourceInput {
            object_id: "object-a".to_string(),
            layers: earlier_layers,
            canonical_region_order: vec![identity(0)],
            object_config: RawObjectConfig {
                layer_height_mm: 0.2,
                line_width: RawWidthSetting::AbsoluteMm(0.45),
                nozzle_diameters_mm: vec![0.4],
                model_alignment_offset_rad: 0.0,
                // The synthetic fixture records the canonical object-level
                // branch explicitly; `build_planning_domain` never assumes it.
                dont_filter_internal_bridges: Some(InternalBridgeFilterPolicy::Disabled),
            },
            provenance,
        }
    }

    #[test]
    fn solid_spacing_matches_the_rounded_rectangle_flow_model() {
        // 0.45 - 0.2 * (1 - PI/4) = 0.4070796... mm -> 4070 IR units.
        assert_eq!(solid_spacing_units(0.45, 0.2), 4070);
        // A wider line and a taller layer both change the spacing.
        assert!(solid_spacing_units(0.6, 0.2) > 4070);
        assert!(solid_spacing_units(0.45, 0.3) < 4070);
    }

    #[test]
    fn parameters_come_from_raw_config_not_defaults() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let parameters = resolve_parameters(&source).unwrap();
        assert_eq!(parameters.nozzle_diameter_mm, 0.4);
        assert_eq!(parameters.extrusion_width_mm, 0.45);
        assert_eq!(parameters.density_fraction, 0.15);
        assert_eq!(parameters.multiline, 1);
        assert_eq!(parameters.overhang_angle_deg, 45.0);
        assert_eq!(parameters.prune_angle_deg, 5.0);
    }

    #[test]
    fn auto_width_falls_back_through_line_width_then_nozzle() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let mut region = region_config();
        region.sparse_width = RawWidthSetting::Automatic;
        assert_eq!(sparse_width_mm(&region, &source.object_config, 0.4), 0.45);
        let mut object = source.object_config.clone();
        object.line_width = RawWidthSetting::Automatic;
        assert!((sparse_width_mm(&region, &object, 0.4) - 0.45).abs() < 1e-12);
        region.sparse_width = RawWidthSetting::NozzlePercent(150.0);
        assert!((sparse_width_mm(&region, &source.object_config, 0.4) - 0.6).abs() < 1e-12);
    }

    #[test]
    fn a_temporary_anchor_is_promoted_out_of_the_sparse_domain() {
        // Lower layer: sparse covering everything (so the gate opens and the
        // unsupported region is the upper solid's overhang).
        // Upper layer: a solid block flanked by sparse strips.
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::InternalSolid,
                geometry: rect(-1000, -20_000, 1000, 20_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(1000, -20_000, 19_000, 20_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction = build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let planning = &construction.planning_layers[1];
        assert!(
            !planning.temporary_anchors.is_empty(),
            "the overhanging sparse must be promoted to a temporary anchor"
        );
        let anchor_area = polygon::area(&planning.temporary_anchors);
        let sparse_area_before = 18_000.0 * 40_000.0;
        assert!(
            anchor_area > 0.0 && anchor_area <= sparse_area_before,
            "anchor area {anchor_area} must be a piece of the sparse area"
        );
        assert!(
            polygon::area(&planning.internal_domains) < sparse_area_before,
            "the promoted piece must leave the internal domain"
        );
        // The union of internal + anchor is the sparse area (nothing lost).
        let mut combined = planning.internal_domains.clone();
        combined.extend(planning.temporary_anchors.iter().cloned());
        let combined_area = polygon::area(&polygon::union(&combined));
        assert!((combined_area - sparse_area_before).abs() < 1.0);
    }

    #[test]
    fn anchor_boundaries_carry_generated_operation_lineage() {
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::InternalSolid,
                geometry: rect(-1000, -20_000, 1000, 20_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(1000, -20_000, 19_000, 20_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction = build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let planning = &construction.planning_layers[1];
        let anchor_edges: Vec<&BoundaryProvenance> = planning
            .boundary_provenance
            .iter()
            .filter(|edge| edge.address.domain == DomainKind::TemporaryAnchor)
            .collect();
        assert!(!anchor_edges.is_empty());
        for edge in anchor_edges {
            assert_eq!(edge.contributors.len(), 2, "one contributor per operation input");
            for contribution in &edge.contributors {
                match &contribution.relation {
                    ContributionRelation::Generated { operation_id } => {
                        assert_eq!(*operation_id, planning.operations[0].operation_id)
                    }
                    other => panic!("anchor edge must be generated, got {other:?}"),
                }
            }
        }
        assert_eq!(planning.operations.len(), 3, "AnchorExpansion -> Union -> Difference");
        assert_eq!(
            planning.operations[0].offset_units,
            Some(PHASE2_ANCHOR_EXPANSION_UNITS as i64)
        );
    }

    #[test]
    fn inherited_boundaries_record_overlap_intervals_and_seams() {
        // Two touching sparse surfaces: the shared seam edge must carry two
        // contributors, and a full coincidence is the [0/1, 1/1] interval.
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::InternalSolid,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(-10_000, -10_000, 0, 10_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(0, -10_000, 10_000, 10_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction = build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let planning = &construction.planning_layers[1];
        let seam = planning
            .boundary_provenance
            .iter()
            .find(|edge| {
                edge.address.domain == DomainKind::Internal
                    && edge.endpoints[0] == (Point2 { x: 0, y: -10_000 })
                    && edge.endpoints[1] == (Point2 { x: 0, y: 10_000 })
            })
            .expect("the seam edge between the two sparse surfaces must exist");
        assert_eq!(
            seam.contributors.len(),
            2,
            "both surfaces own the seam: {:?}",
            seam.contributors
        );
        for contribution in &seam.contributors {
            match &contribution.relation {
                ContributionRelation::Overlap { start, end } => {
                    assert_eq!((start.numerator, start.denominator), (0, 1));
                    assert_eq!((end.numerator, end.denominator), (1, 1));
                }
                other => panic!("seam must be an overlap relation, got {other:?}"),
            }
        }
    }

    #[test]
    fn policies_resolve_angles_and_carry_override_presence() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let construction = build_planning_domain(&source).unwrap();
        assert_eq!(construction.policies.len(), 1);
        let policy = &construction.policies[0].policy;
        let expected = ((std::f64::consts::FRAC_PI_4 as f32) + 0.0f32) as f64;
        assert_eq!(policy.resolved_sparse_angle_rad, expected);
        assert_eq!(policy.resolved_solid_angle_rad, expected);
        assert_eq!(policy.resolved_top_angle_rad, Some(expected));
        assert_eq!(policy.resolved_bottom_angle_rad, Some(expected));
        assert!(!policy.fixed_angle, "an empty rotate template is not fixed");
        assert!(!policy.is_bridge);
        assert_eq!(policy.bridge_angle_rad, None);
        assert_eq!(policy.model_alignment_offset_rad, 0.0);
    }

    #[test]
    fn an_unset_direction_override_yields_no_resolved_top_angle() {
        let mut source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        source.layers[0].regions[0].config.top_layer_direction_deg = -1.0;
        source.layers[0].regions[0].config.bottom_layer_direction_deg = -1.0;
        source.layers[0].regions[0].config.align_to_model = true;
        source.object_config.model_alignment_offset_rad = 0.1;
        let construction = build_planning_domain(&source).unwrap();
        let policy = &construction.policies[0].policy;
        assert_eq!(policy.resolved_top_angle_rad, None);
        assert_eq!(policy.resolved_bottom_angle_rad, None);
        // The alignment offset still reaches the sparse angle.
        let expected = ((std::f64::consts::FRAC_PI_4 as f32) + (0.1f32)) as f64;
        assert_eq!(policy.resolved_sparse_angle_rad, expected);
    }

    #[test]
    fn a_rotate_template_is_reported_as_unavailable_not_defaulted() {
        let mut source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        source.layers[0].regions[0].config.sparse_rotate_template = "10,20".to_string();
        let error = build_planning_domain(&source).unwrap_err();
        match error {
            KernelError::RequiredSourceInputUnavailable { field, .. } => {
                assert_eq!(field, "sparse_infill_rotate_template")
            }
            other => panic!("expected a named unavailable field, got {other:?}"),
        }
        // A non-empty template also fixes the direction when it can be resolved.
        source.layers[0].regions[0].config.sparse_rotate_template = String::new();
        source.layers[0].regions[0].config.solid_rotate_template = "10,20".to_string();
        let error = build_planning_domain(&source).unwrap_err();
        match error {
            KernelError::RequiredSourceInputUnavailable { field, .. } => {
                assert_eq!(field, "solid_infill_rotate_template")
            }
            other => panic!("expected a named unavailable field, got {other:?}"),
        }
    }

    #[test]
    fn missing_provenance_and_missing_regions_are_named_errors() {
        let mut source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        source.provenance.clear();
        match build_planning_domain(&source).unwrap_err() {
            KernelError::SourceProvenanceUnavailable { reason, .. } => {
                assert!(reason.contains("provenance_id"), "unexpected reason: {reason}")
            }
            other => panic!("expected SourceProvenanceUnavailable, got {other:?}"),
        }

        let mut source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        source.canonical_region_order.push(identity(7));
        match build_planning_domain(&source).unwrap_err() {
            KernelError::RequiredSourceInputUnavailable { field, .. } => {
                assert_eq!(field, "layers[].regions[].identity")
            }
            other => panic!("expected RequiredSourceInputUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn layers_without_sparse_infill_produce_no_domains_and_no_operations() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::InternalSolid,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let construction = build_planning_domain(&source).unwrap();
        let planning = &construction.planning_layers[0];
        assert!(planning.internal_domains.is_empty());
        assert!(planning.void_domains.is_empty());
        assert!(planning.temporary_anchors.is_empty());
        assert!(planning.operations.is_empty());
        assert_eq!(planning.source_catalog.len(), 1);
        assert_eq!(planning.source_catalog[0].surface_kind, "internal_solid");
    }

    #[test]
    fn void_domains_are_separate_and_never_promoted() {
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::Void,
                geometry: rect(-15_000, -15_000, 15_000, 15_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(16_000, -19_000, 19_000, 19_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction = build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let planning = &construction.planning_layers[1];
        assert_eq!(planning.void_domains.len(), 1);
        assert_eq!(polygon::area(&planning.void_domains), 30_000.0 * 30_000.0);
        assert!(
            planning
                .temporary_anchors
                .iter()
                .all(|anchor| polygon::area(std::slice::from_ref(anchor)) < 30_000.0 * 30_000.0),
            "a void domain must never appear as a promoted anchor"
        );
    }

    #[test]
    fn the_anchor_filter_branch_comes_from_the_source_config() {
        assert_eq!(PHASE2_ANCHOR_EXPANSION_UNITS, 30_000.0);

        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::InternalSolid,
                geometry: rect(-1000, -20_000, 1000, 20_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(1000, -20_000, 19_000, 20_000),
            },
        ]];
        let mut source = source_with_layers(vec![lower, upper]);

        // Config-driven: the object config selects the NoFilter branch, and the
        // result is identical to passing that branch explicitly.
        source.object_config.dont_filter_internal_bridges =
            Some(InternalBridgeFilterPolicy::NoFilter);
        let from_source = build_planning_domain(&source).unwrap();
        let explicit =
            build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        assert_eq!(
            from_source.planning_layers[1].temporary_anchors,
            explicit.planning_layers[1].temporary_anchors
        );
        assert!(
            !from_source.planning_layers[1].temporary_anchors.is_empty(),
            "the recorded nofilter branch must promote the overhang"
        );

        // The documented OrcaSlicer default is a different branch, not a
        // fallback: this fixture diverges under it.
        source.object_config.dont_filter_internal_bridges =
            Some(InternalBridgeFilterPolicy::Disabled);
        let disabled = build_planning_domain(&source).unwrap();
        assert_ne!(
            disabled.planning_layers[1].temporary_anchors,
            from_source.planning_layers[1].temporary_anchors,
            "the fixture must discriminate the two branches"
        );

        // An unrecorded branch is a named unavailability, never a silent default.
        source.object_config.dont_filter_internal_bridges = None;
        match build_planning_domain(&source).unwrap_err() {
            KernelError::RequiredSourceInputUnavailable { field, .. } => assert_eq!(
                field,
                "object_config.dont_filter_internal_bridges"
            ),
            other => panic!("expected a named unavailable field, got {other:?}"),
        }
    }

    #[test]
    fn the_constructor_scalars_are_derived_from_the_raw_config() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let parameters = resolve_parameters(&source).unwrap();
        // 0.45 mm * 1 / 0.15, 0.2 mm * tan(45 deg), 0.2 mm * tan(5 deg).
        assert!((parameters.supporting_radius_mm.unwrap() - 3.0).abs() < 1e-12);
        assert!((parameters.wall_supporting_radius_mm - 0.2).abs() < 1e-12);
        assert!((parameters.prune_length_mm - 0.2 * (5.0f64.to_radians()).tan()).abs() < 1e-15);
        assert!(
            (parameters.straightening_max_distance_mm - parameters.prune_length_mm).abs() < 1e-15
        );
        // Each scalar moves with its own input, so none is a constant.
        let mut denser = source.clone();
        denser.layers[0].regions[0].config.density_percent = 30.0;
        assert!(
            resolve_parameters(&denser).unwrap().supporting_radius_mm.unwrap() < parameters.supporting_radius_mm.unwrap()
        );
        let mut steeper = source.clone();
        steeper.layers[0].regions[0].config.prune_angle_deg = 10.0;
        assert!(
            resolve_parameters(&steeper).unwrap().prune_length_mm > parameters.prune_length_mm
        );
    }

    #[test]
    fn a_zero_density_has_no_supporting_radius_instead_of_a_default() {
        let mut source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        source.layers[0].regions[0].config.density_percent = 0.0;
        let parameters = resolve_parameters(&source).unwrap();
        assert_eq!(parameters.supporting_radius_mm, None);
        // The angle-scaled lengths do not depend on density and stay present.
        assert!(parameters.prune_length_mm > 0.0);
    }

    #[test]
    fn generated_and_surface_level_identities_are_absent_not_zero() {
        // Promotion layout: the temporary-anchor boundaries are generated.
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::InternalSolid,
                geometry: rect(-1000, -20_000, 1000, 20_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(1000, -20_000, 19_000, 20_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction =
            build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let planning = &construction.planning_layers[1];

        let generated: Vec<&SourceSegmentRef> = planning
            .boundary_provenance
            .iter()
            .filter(|edge| edge.address.domain == DomainKind::TemporaryAnchor)
            .flat_map(|edge| edge.contributors.iter())
            .map(|contribution| &contribution.source)
            .collect();
        assert!(!generated.is_empty());
        for source_ref in generated {
            assert_eq!(source_ref.ring_id, None);
            assert_eq!(source_ref.segment_id, None);
        }

        // Operation inputs are surface-level in the record: absent, not 0/0.
        for operation in &planning.operations {
            for input in &operation.inputs {
                assert_eq!((input.ring_id, input.segment_id), (None, None));
            }
        }

        // An inherited edge keeps a real ring/segment pair, including ring 0 and
        // segment 0, which is exactly what the absent shape must not collide with.
        let untouched = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let untouched = build_planning_domain(&untouched).unwrap();
        let inherited = untouched.planning_layers[0]
            .boundary_provenance
            .iter()
            .find(|edge| edge.address.domain == DomainKind::Internal)
            .expect("the inherited internal domain has boundaries");
        assert!(!inherited.contributors.is_empty());
        assert!(inherited
            .contributors
            .iter()
            .any(|contribution| contribution.source.ring_id == Some(0)
                && contribution.source.segment_id == Some(0)));
    }

    #[test]
    fn the_phase_two_chain_is_recorded_at_the_post_anchor_checkpoint() {
        let lower = vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::Internal,
            geometry: rect(-20_000, -20_000, 20_000, 20_000),
        }]];
        let upper = vec![vec![
            SurfaceSpec {
                kind: EarlierSurfaceKind::InternalSolid,
                geometry: rect(-1000, -20_000, 1000, 20_000),
            },
            SurfaceSpec {
                kind: EarlierSurfaceKind::Internal,
                geometry: rect(1000, -20_000, 19_000, 20_000),
            },
        ]];
        let source = source_with_layers(vec![lower, upper]);
        let construction =
            build_planning_domain_with_policy(&source, InternalBridgeFilterPolicy::NoFilter).unwrap();
        let operations = &construction.planning_layers[1].operations;
        assert_eq!(operations.len(), 3);
        assert!(operations
            .iter()
            .all(|operation| operation.checkpoint == SourceCheckpoint::PostAnchor));
    }

    #[test]
    fn a_bottom_bridge_surface_keeps_its_recorded_kind() {
        let source = source_with_layers(vec![vec![vec![SurfaceSpec {
            kind: EarlierSurfaceKind::BottomBridge,
            geometry: rect(0, 0, 10_000, 10_000),
        }]]]);
        let construction = build_planning_domain(&source).unwrap();
        let catalog = &construction.planning_layers[0].source_catalog;
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].surface_kind, "bottom_bridge");
    }

    #[test]
    fn the_new_evidence_vocabulary_has_its_recorded_serde_spellings() {
        // Step 5 deserializes strictly, so the wire spellings are part of the
        // contract this test pins: `post_anchor`, `bottom_bridge`, `nofilter`.
        assert_eq!(
            serde_json::to_string(&SourceCheckpoint::PostAnchor).unwrap(),
            "\"post_anchor\""
        );
        assert_eq!(
            serde_json::to_string(&EarlierSurfaceKind::BottomBridge).unwrap(),
            "\"bottom_bridge\""
        );
        assert_eq!(
            serde_json::to_string(&InternalBridgeFilterPolicy::NoFilter).unwrap(),
            "\"nofilter\""
        );
        assert_eq!(
            serde_json::to_string(&InternalBridgeFilterPolicy::Disabled).unwrap(),
            "\"disabled\""
        );
        assert_eq!(
            serde_json::to_string(&InternalBridgeFilterPolicy::Limited).unwrap(),
            "\"limited\""
        );
        // An absent segment identity serializes as an explicit `null` pair, and
        // ring 0 serializes as `0`.
        let identity = identity(0);
        let surface = SourceSurfaceId {
            identity: identity.clone(),
            local_layer_index: 0,
            global_layer_index: 0,
            checkpoint: SourceCheckpoint::BeforeAnchorEdit,
            surface_token: "o0_l0_r0_s0".to_string(),
        };
        let absent = serde_json::to_value(SourceSegmentRef {
            surface: surface.clone(),
            ring_id: None,
            segment_id: None,
        })
        .unwrap();
        assert_eq!(absent["ring_id"], serde_json::Value::Null);
        assert_eq!(absent["segment_id"], serde_json::Value::Null);
        let ring_zero = serde_json::to_value(SourceSegmentRef {
            surface,
            ring_id: Some(0),
            segment_id: Some(0),
        })
        .unwrap();
        assert_eq!(ring_zero["ring_id"], serde_json::json!(0));
        assert_eq!(ring_zero["segment_id"], serde_json::json!(0));
        // The two shapes must not round-trip into each other.
        assert_ne!(absent, ring_zero);
    }
}
