// -----------------------------------------------------------------------------
// Original Rust: this file is not a translation of OrcaSlicer source.
//
// It defines the typed input, output, and evidence model for the isolated
// Lightning feasibility kernel. The original packet's detailed design is in Git
// history; its design digest records the Gate A goal and constraints at
// `docs/spec_packets/_OLD/layer-module-preparation_07_lightning-geometry-portability-gate.md`.
//
// Serde policy: every struct denies unknown fields, every enum is closed, and
// no `Option` field uses `#[serde(default)]`, so serialization always emits the
// key (null when absent). Executable required-presence assertions are Step 9.
// Coordinate semantics: all `Point2` values are IR integer units (1 unit = 100 nm).
// -----------------------------------------------------------------------------

use std::fmt;

use serde::{Deserialize, Serialize};
use slicer_ir::{ExPolygon, PaintValue, Point2};

/// Canonical scene identity for a region: object plus numeric region id plus the
/// ordered paint/variant chain that selected it. Vector position is never identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionIdentity {
    pub object_id: String,
    pub region_id: u64,
    pub variant_chain: Vec<(String, PaintValue)>,
}

// -----------------------------------------------------------------------------
// Restricted Gate A source (earlier-layer capture) and construction result
// -----------------------------------------------------------------------------

/// Source-only Gate A input graph. Contains earlier geometry, raw configuration,
/// and independently recorded source-origin metadata only; it can never carry
/// B/reference domains, resolved policy/scalars, expected graphs, or final fills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateASourceInput {
    pub object_id: String,
    pub layers: Vec<EarlierLayerInput>,
    pub canonical_region_order: Vec<RegionIdentity>,
    pub object_config: RawObjectConfig,
    pub provenance: Vec<EarlierOriginRecord>,
}

/// One earlier layer as captured before planning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierLayerInput {
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub effective_layer_height_mm: f64,
    pub regions: Vec<EarlierRegionInput>,
}

/// One earlier region: raw surfaces, shell schedule indices, and raw config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierRegionInput {
    pub identity: RegionIdentity,
    pub surfaces: Vec<EarlierSurface>,
    pub top_shell_index: Option<u32>,
    pub bottom_shell_index: Option<u32>,
    pub config: RawRegionConfig,
}

/// A pre-Layer geometry occurrence. Identity is `surface_token` plus the full
/// region/layer identity; the token is never parsed as serialized data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierSurface {
    pub surface_token: String,
    pub kind: EarlierSurfaceKind,
    pub geometry: ExPolygon,
    pub detected_bridge_angle_rad: Option<f64>,
    pub provenance_id: u32,
}

/// Closed classification of an earlier surface occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarlierSurfaceKind {
    Outline,
    Internal,
    Void,
    TopSolid,
    BottomSolid,
    /// A bottom bridge (`stBottomBridge`), recorded as `bottom_bridge`. The
    /// capture records it as its own kind, never as `BottomSolid`: for example
    /// `island_split_merge_classic` carries surface `o0_l5_r0_s2` as
    /// `bottom_bridge` on global layer 5.
    BottomBridge,
    InternalSolid,
    InternalBridge,
    Shell,
}

/// Closed pre-Layer phase. Earlier classification is the only admissible phase;
/// planning-time and final-fill phases cannot fit this schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarlierSourcePhase {
    EarlierClassification,
}

/// Closed producer that recorded the earlier source occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarlierProducer {
    SliceGeometry,
    PrePassClassification,
}

/// Independently recorded origin/audit record for an earlier source occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierOriginRecord {
    pub provenance_id: u32,
    pub phase: EarlierSourcePhase,
    pub producer: EarlierProducer,
    pub source_geometry_sha256: String,
    pub raw_config_sha256: String,
}

/// Raw object-level configuration; an exhaustive whitelist, not a map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawObjectConfig {
    pub layer_height_mm: f64,
    pub line_width: RawWidthSetting,
    pub nozzle_diameters_mm: Vec<f64>,
    pub model_alignment_offset_rad: f64,
    /// `PrintObjectConfig.dont_filter_internal_bridges`, which
    /// `PrintObject::bridge_over_infill` reads from `po->config()` to choose the
    /// candidate-gather branch. Recorded per case in the capture's
    /// `settings.json` (`nofilter` for every current case); the object-level
    /// `source.json` snapshot does not carry it. `None` means the source
    /// envelope did not record the key, and the branch is then undecidable.
    pub dont_filter_internal_bridges: Option<InternalBridgeFilterPolicy>,
}

/// Closed `dont_filter_internal_bridges` (`InternalBridgeFilter`) branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalBridgeFilterPolicy {
    /// `ibfDisabled` — "Filter", the OrcaSlicer default: candidates are area
    /// filtered and the opening multiplier is 3.
    Disabled,
    /// `ibfLimited` — "Limited filtering": same area filtering, multiplier 1.
    Limited,
    /// `ibfNofilter` — "No filtering", recorded as `nofilter`. Every potentially
    /// overhanging internal solid surface becomes a candidate, expanded by
    /// `4 * spacing`; the recorded `anchor_bridge_classic` case uses it and the
    /// reconstruction only reproduces that case on this branch.
    #[serde(rename = "nofilter")]
    NoFilter,
}

/// Raw region-level configuration; an exhaustive whitelist, not a map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRegionConfig {
    pub fill_multiline: u32,
    pub sparse_width: RawWidthSetting,
    pub density_percent: f64,
    pub overhang_angle_deg: f64,
    pub prune_angle_deg: f64,
    pub straightening_angle_deg: f64,
    pub infill_direction_deg: f64,
    pub sparse_rotate_template: String,
    pub solid_infill_direction_deg: f64,
    pub solid_rotate_template: String,
    pub top_layer_direction_deg: f64,
    pub bottom_layer_direction_deg: f64,
    pub align_to_model: bool,
    /// `sparse_infill_pattern`, recorded verbatim in `source.json`
    /// (`lightning` for every current case). Transported as recorded evidence;
    /// this kernel is only ever invoked for Lightning objects, so the Phase-1
    /// pattern gate stays implicit rather than re-testing the string.
    pub sparse_infill_pattern: String,
}

/// The actual raw width selection, before any resolution/defaulting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawWidthSetting {
    Automatic,
    AbsoluteMm(f64),
    NozzlePercent(f64),
}

/// Complete Gate A reconstruction result. Policies and parameters here are
/// derived outputs, never copied reference values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateAConstruction {
    pub object_id: String,
    pub planning_layers: Vec<PlanningLayer>,
    pub canonical_region_order: Vec<RegionIdentity>,
    pub scalar_source_region: RegionIdentity,
    pub parameters: LightningParameters,
    pub policies: Vec<DerivedRegionPolicy>,
}

/// Derived fill policy for one region occurrence on one global layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedRegionPolicy {
    pub identity: RegionIdentity,
    pub global_layer_index: u32,
    pub policy: FillPolicyInput,
}

// -----------------------------------------------------------------------------
// Planning layer and lossless source/boundary/operation provenance
// -----------------------------------------------------------------------------

/// One computed (A) or independently recorded (B) planning layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningLayer {
    pub object_id: String,
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub internal_domains: Vec<ExPolygon>,
    pub void_domains: Vec<ExPolygon>,
    pub temporary_anchors: Vec<ExPolygon>,
    pub source_regions: Vec<RegionIdentity>,
    pub source_catalog: Vec<SourceSurface>,
    pub boundary_provenance: Vec<BoundaryProvenance>,
    pub operations: Vec<GeometryOperation>,
}

/// Full identity of a transported source surface occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSurfaceId {
    pub identity: RegionIdentity,
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub checkpoint: SourceCheckpoint,
    pub surface_token: String,
}

/// Closed lifecycle checkpoint for a source occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCheckpoint {
    EarlierClassification,
    BeforeAnchorEdit,
    /// The anchor-expansion/union/difference events recorded immediately after
    /// the anchor edit, spelled `post_anchor`. `operations[].checkpoint` in
    /// `anchor_bridge_classic` is `post_anchor` for all three recorded
    /// operations; they run before planning resumes.
    PostAnchor,
    DuringPlanning,
    AfterRestoration,
    FinalFill,
}

/// A normalized source surface with its optional proven-not-applicable angles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSurface {
    pub id: SourceSurfaceId,
    pub geometry: ExPolygon,
    pub surface_kind: String,
    pub solid_angle_rad: Option<f64>,
    pub bridge_angle_rad: Option<f64>,
}

/// Addresses a contour/hole edge of a normalized source surface ring.
///
/// `ring_id`/`segment_id` are `None` when the recorded occurrence is
/// surface-level rather than edge-level, which the capture writes as JSON
/// `null`: the 36 generated contributors in `anchor_bridge_classic`'s
/// `provenance.json` (and every surface-level `operations[].inputs` /
/// `operands[].sources` entry) have no ring or segment. Absence is never
/// ring/segment `0`; both ids are present together or absent together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSegmentRef {
    pub surface: SourceSurfaceId,
    pub ring_id: Option<u32>,
    pub segment_id: Option<u32>,
}

/// Addresses a normalized output planning boundary edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryAddress {
    pub domain: DomainKind,
    pub surface_id: u32,
    pub ring_id: u32,
    pub segment_id: u32,
}

/// Closed output planning-domain kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainKind {
    Internal,
    Void,
    TemporaryAnchor,
}

/// Boundary edge plus every causal contributor, preserving multiplicity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryProvenance {
    pub address: BoundaryAddress,
    pub endpoints: [Point2; 2],
    pub contributors: Vec<BoundaryContribution>,
}

/// One source segment contributing to an output boundary edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryContribution {
    pub source: SourceSegmentRef,
    pub relation: ContributionRelation,
}

/// How a source segment relates to an output boundary edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionRelation {
    Overlap { start: Rational, end: Rational },
    Generated { operation_id: u32 },
}

/// Exact directed rational interval bound, reduced with a positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rational {
    pub numerator: i64,
    pub denominator: u64,
}

/// One geometry operation in the causal DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryOperation {
    pub operation_id: u32,
    pub kind: GeometryOpKind,
    pub checkpoint: SourceCheckpoint,
    pub inputs: Vec<SourceSegmentRef>,
    pub parent_operations: Vec<u32>,
    pub operands: Vec<GeometryOperand>,
    pub offset_units: Option<i64>,
    pub join: Option<String>,
    pub result_geometry: Vec<ExPolygon>,
    pub outputs: Vec<BoundaryAddress>,
}

/// Closed geometry operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryOpKind {
    Union,
    Difference,
    Intersection,
    Offset,
    AnchorExpansion,
    Reclassification,
}

/// One ordered operand of a geometry operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryOperand {
    pub role: OperandRole,
    pub sources: Vec<SourceSegmentRef>,
    pub operation_id: Option<u32>,
}

/// Closed operand role; distinguishes subtraction subject from cutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperandRole {
    Subject,
    Clip,
}

/// Complete provenance table attached to each computed graph layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphProvenance {
    pub source_catalog: Vec<SourceSurface>,
    pub boundary_provenance: Vec<BoundaryProvenance>,
    pub operations: Vec<GeometryOperation>,
}

// -----------------------------------------------------------------------------
// Final fills (B-only clipping transport)
// -----------------------------------------------------------------------------

/// One grouped final expolygon with its whole original contributor multiset and
/// the actual caller's effective policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalFillInput {
    pub dispatch_token: String,
    pub caller_identity: RegionIdentity,
    pub geometry: ExPolygon,
    pub contributors: Vec<SourceSurfaceId>,
    pub policy: FillPolicyInput,
}

/// Clipped fill segments for one layer, keyed by full region/dispatch identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClippedLayer {
    pub object_id: String,
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub fills: Vec<ClippedFill>,
}

/// One dispatch's final clipped segments with its actual caller origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClippedFill {
    pub dispatch_token: String,
    pub caller_identity: RegionIdentity,
    pub contributors: Vec<SourceSurfaceId>,
    pub segments: Vec<[Point2; 2]>,
}

// -----------------------------------------------------------------------------
// Parameters and fill policy
// -----------------------------------------------------------------------------

/// Explicit constructor/unit-conversion inputs; never borrowed from print-wide
/// defaults. Angle fields are degrees, lengths millimeters, density a fraction.
///
/// The four radius/length scalars are the `FillLightning::Generator` constructor
/// outputs recorded in `planning.json` `scalars`; they are derived from this same
/// raw config, never copied from the recorded value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightningParameters {
    pub nozzle_diameter_mm: f64,
    pub extrusion_width_mm: f64,
    pub density_fraction: f64,
    pub multiline: u32,
    pub layer_height_mm: f64,
    pub overhang_angle_deg: f64,
    pub prune_angle_deg: f64,
    pub straightening_angle_deg: f64,
    /// `scalars.supporting_radius` (recorded `3.0` for the Classic captures): the
    /// radius that resolves unsupported cells into branches. `None` only when the
    /// density is zero, where the canonical expression divides by zero and no
    /// value exists.
    pub supporting_radius_mm: Option<f64>,
    /// `scalars.wall_supporting_radius` (recorded `0.2`): the radius that
    /// classifies an outline as supported by the layer below.
    pub wall_supporting_radius_mm: f64,
    /// `scalars.prune_length` (recorded `0.017497`): the maximum branch length
    /// removed while propagating a tree to the next layer.
    pub prune_length_mm: f64,
    /// `scalars.straightening_max_distance` (recorded `0.017497`): the maximum
    /// node movement used while straightening a propagated tree.
    pub straightening_max_distance_mm: f64,
}

/// Raw fill-direction/template settings plus the recorded resolved angles and
/// fixed-direction/bridge policies that protect source classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillPolicyInput {
    pub infill_direction_deg: f64,
    pub sparse_rotate_template: String,
    pub solid_infill_direction_deg: f64,
    pub solid_rotate_template: String,
    pub top_layer_direction_deg: f64,
    pub bottom_layer_direction_deg: f64,
    pub align_to_model: bool,
    pub model_alignment_offset_rad: f64,
    pub resolved_sparse_angle_rad: f64,
    pub resolved_solid_angle_rad: f64,
    pub resolved_top_angle_rad: Option<f64>,
    pub resolved_bottom_angle_rad: Option<f64>,
    pub fixed_angle: bool,
    pub is_bridge: bool,
    pub bridge_angle_rad: Option<f64>,
}

// -----------------------------------------------------------------------------
// B-only planner/clipping input
// -----------------------------------------------------------------------------

/// B-only canonical-domain planner/clipping input. Never a Gate A construction
/// input and never a carrier of expected graph/output data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelInput {
    pub object_id: String,
    pub layers: Vec<LayerInput>,
    pub planning_layers: Vec<PlanningLayer>,
    pub canonical_region_order: Vec<RegionIdentity>,
    pub scalar_source_region: RegionIdentity,
    pub parameters: LightningParameters,
}

/// One layer of B-only clipping transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerInput {
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub effective_layer_height_mm: f64,
    pub regions: Vec<RegionInput>,
}

/// Complete B-only region record. Not reachable from `GateASourceInput`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionInput {
    pub identity: RegionIdentity,
    pub outlines: Vec<ExPolygon>,
    pub top_solid_fill: Vec<ExPolygon>,
    pub bottom_solid_fill: Vec<ExPolygon>,
    pub internal_solid_fill: Vec<ExPolygon>,
    pub internal_bridge_areas: Vec<ExPolygon>,
    pub internal_bridge_angles_deg: Vec<f32>,
    pub top_shell_index: Option<u32>,
    pub bottom_shell_index: Option<u32>,
    pub effective_parameters: LightningParameters,
    pub shell_surfaces: Vec<SourceSurface>,
    pub classified_surfaces: Vec<SourceSurface>,
    pub fill_policy: FillPolicyInput,
    pub final_fill_expolygons: Vec<ExPolygon>,
    pub final_source_catalog: Vec<SourceSurface>,
    pub final_surface_sources: Vec<Vec<SourceSurfaceId>>,
    pub final_fills: Vec<FinalFillInput>,
}

// -----------------------------------------------------------------------------
// Producer model
// -----------------------------------------------------------------------------

/// Complete computed plan for one object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelPlan {
    pub object_id: String,
    pub layers: Vec<GraphLayer>,
}

/// One computed graph layer with its provenance table and sampled fill segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphLayer {
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub provenance: GraphProvenance,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub sampled_segments: Vec<[Point2; 2]>,
}

/// One directed graph node; ungrounded nodes keep their actual root fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNode {
    pub node_id: u32,
    pub point: Point2,
    pub parent_id: Option<u32>,
    pub root_id: u32,
    pub origin_region_index: u32,
    pub grounding: Option<GroundingWitness>,
}

/// One directed graph edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphEdge {
    pub edge_id: u32,
    pub parent_id: u32,
    pub child_id: u32,
    pub root_id: u32,
    pub origin_region_index: u32,
}

/// Grounding witness resolving into the layer's provenance table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundingWitness {
    pub kind: GroundKind,
    pub boundary: BoundaryAddress,
}

/// Closed grounding kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundKind {
    Boundary,
    TemporaryAnchor,
}

// -----------------------------------------------------------------------------
// Errors and evidence localization
// -----------------------------------------------------------------------------

/// Structured kernel error. Unavailable/unrepresentable evidence is
/// inconclusive; measured mismatch/malformed supplied data is fail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KernelError {
    SourceProvenanceUnavailable {
        context: EvidenceLocation,
        reason: String,
    },
    SourceProvenanceMismatch {
        context: EvidenceLocation,
        expected: String,
        actual: String,
    },
    RequiredSourceInputUnavailable {
        context: EvidenceLocation,
        field: String,
        reason: String,
    },
    RequiredSourceInputMismatch {
        context: EvidenceLocation,
        field: String,
        expected: String,
        actual: String,
    },
    InvalidInput {
        context: EvidenceLocation,
        reason: String,
    },
    UnrepresentableTransport {
        context: EvidenceLocation,
        reason: String,
    },
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProvenanceUnavailable { context, reason } => {
                write!(f, "source provenance unavailable at {context}: {reason}")
            }
            Self::SourceProvenanceMismatch {
                context,
                expected,
                actual,
            } => write!(
                f,
                "source provenance mismatch at {context}: expected {expected}, actual {actual}"
            ),
            Self::RequiredSourceInputUnavailable {
                context,
                field,
                reason,
            } => write!(
                f,
                "required source input `{field}` unavailable at {context}: {reason}"
            ),
            Self::RequiredSourceInputMismatch {
                context,
                field,
                expected,
                actual,
            } => write!(
                f,
                "required source input `{field}` mismatch at {context}: expected {expected}, actual {actual}"
            ),
            Self::InvalidInput { context, reason } => {
                write!(f, "invalid input at {context}: {reason}")
            }
            Self::UnrepresentableTransport { context, reason } => {
                write!(f, "unrepresentable transport at {context}: {reason}")
            }
        }
    }
}

impl std::error::Error for KernelError {}

/// Localizes an error/evidence record without inventing absent identities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLocation {
    pub object_id: String,
    pub global_layer_index: u32,
    pub region: Option<RegionIdentity>,
    pub source: Option<SourceSurfaceId>,
    pub checkpoint: Option<SourceCheckpoint>,
}

impl fmt::Display for EvidenceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "object={} layer={}",
            self.object_id, self.global_layer_index
        )?;
        if let Some(region) = &self.region {
            write!(f, " region={}", region.region_id)?;
        }
        if let Some(source) = &self.source {
            write!(f, " source={}", source.surface_token)?;
        }
        if let Some(checkpoint) = &self.checkpoint {
            write!(f, " checkpoint={checkpoint:?}")?;
        }
        Ok(())
    }
}
