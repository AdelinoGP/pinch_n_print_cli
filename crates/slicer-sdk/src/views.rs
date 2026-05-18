//! View types for reading IR data.
//!
//! These are read-only views that the host constructs and passes to modules.
//! Per docs/03_wit_and_manifest.md (ir-types.wit), view resources cannot be
//! constructed by modules.

use std::collections::HashMap;

use slicer_ir::{
    ExPolygon, ExtrusionRole, ObjectId, PaintSemantic, PaintValue, Point3WithWidth, RegionId,
    RegionKey, SeamCandidate, SeamPosition, WallLoop,
};

/// Read-only view of a slice region.
///
/// Matches WIT `resource slice-region-view` from ir-types.wit.
/// Host constructs these; modules cannot construct them.
#[derive(Debug, Clone)]
pub struct SliceRegionView {
    object_id: ObjectId,
    region_id: RegionId,
    polygons: Vec<ExPolygon>,
    infill_areas: Vec<ExPolygon>,
    effective_layer_height: f32,
    z: f32,
    has_nonplanar: bool,
    boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    /// SurfaceClassificationIR-derived eligibility flag. Surfaces the documented
    /// `needs_support` signal from docs/02_ir_schemas.md into the support stage
    /// so generators can apply the default eligibility rules from
    /// docs/01_system_architecture.md when no support paint override applies.
    needs_support: bool,
    /// True when this region is classified as a top surface by SurfaceClassificationIR.
    /// Indicates the region needs TopSolidInfill fill rather than sparse infill.
    is_top_surface: bool,
    /// True when this region is classified as a bottom surface by SurfaceClassificationIR.
    /// Indicates the region needs BottomSolidInfill fill rather than sparse infill.
    is_bottom_surface: bool,
    /// True when this region is classified as a bridge region by SurfaceClassificationIR.
    /// Indicates the region needs BridgeInfill fill and cannot rely on support below.
    is_bridge: bool,
    /// Per-layer expanded bridge polygons (empty if not a bridge region).
    bridge_areas: Vec<ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).
    bridge_orientation_deg: f32,
    /// Claim IDs held by the module that produced this region.
    /// Modules may only emit fill paths for roles they hold; empty means
    /// the full set (rectilinear default emits all four).
    held_claims: Vec<String>,
}

impl Default for SliceRegionView {
    fn default() -> Self {
        Self {
            object_id: ObjectId::default(),
            region_id: RegionId::default(),
            polygons: Vec::new(),
            infill_areas: Vec::new(),
            effective_layer_height: 0.0,
            z: 0.0,
            has_nonplanar: false,
            boundary_paint: HashMap::new(),
            // `needs_support: true` matches the pre-TASK-200e `new()` default
            // (see docs/02_ir_schemas.md §IR 2). Test fixtures that predate
            // the SurfaceClassificationIR wiring observe the prior
            // "all candidates eligible" behavior.
            needs_support: true,
            is_top_surface: false,
            is_bottom_surface: false,
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        }
    }
}

impl SliceRegionView {
    /// Override the object ID (host-only, for testing).
    #[doc(hidden)]
    pub fn set_object_id(&mut self, object_id: impl Into<ObjectId>) {
        self.object_id = object_id.into();
    }

    /// Override the region ID (host-only, for testing).
    #[doc(hidden)]
    pub fn set_region_id(&mut self, region_id: RegionId) {
        self.region_id = region_id;
    }

    /// Override the slice polygons (host-only, for testing).
    #[doc(hidden)]
    pub fn set_polygons(&mut self, polygons: Vec<ExPolygon>) {
        self.polygons = polygons;
    }

    /// Override the infill areas (host-only, for testing).
    #[doc(hidden)]
    pub fn set_infill_areas(&mut self, infill_areas: Vec<ExPolygon>) {
        self.infill_areas = infill_areas;
    }

    /// Override the effective layer height (host-only, for testing).
    #[doc(hidden)]
    pub fn set_effective_layer_height(&mut self, effective_layer_height: f32) {
        self.effective_layer_height = effective_layer_height;
    }

    /// Override the Z height (host-only, for testing).
    #[doc(hidden)]
    pub fn set_z(&mut self, z: f32) {
        self.z = z;
    }

    /// Override the non-planar flag (host-only, for testing).
    #[doc(hidden)]
    pub fn set_has_nonplanar(&mut self, has_nonplanar: bool) {
        self.has_nonplanar = has_nonplanar;
    }

    /// Override the boundary paint map (host-only, for testing).
    #[doc(hidden)]
    pub fn set_boundary_paint(
        &mut self,
        boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    ) {
        self.boundary_paint = boundary_paint;
    }

    /// Override the `needs_support` eligibility flag (host-only, for testing).
    ///
    /// Per docs/02_ir_schemas.md §IR 2, the host populates this from
    /// `SurfaceClassificationIR.needs_support` for the region's object.
    /// Default constructors leave the flag `true` so callsites that predate the
    /// SurfaceClassificationIR wiring observe the prior "all candidates eligible"
    /// behavior.
    #[doc(hidden)]
    pub fn set_needs_support(&mut self, needs_support: bool) {
        self.needs_support = needs_support;
    }

    /// Override the top-surface classification flag (host-only, for testing).
    ///
    /// Per docs/02_ir_schemas.md §SurfaceClassificationIR, the host populates
    /// this from `ObjectSurfaceData.surface_groups` where `shell_count > 0`
    /// indicates a top surface. Default constructors leave the flag `false`.
    #[doc(hidden)]
    pub fn set_is_top_surface(&mut self, is_top_surface: bool) {
        self.is_top_surface = is_top_surface;
    }

    /// Override the bottom-surface classification flag (host-only, for testing).
    ///
    /// Per docs/02_ir_schemas.md §SurfaceClassificationIR, the host populates
    /// this from `ObjectSurfaceData.surface_groups` where the group is adjacent
    /// to the build plate. Default constructors leave the flag `false`.
    #[doc(hidden)]
    pub fn set_is_bottom_surface(&mut self, is_bottom_surface: bool) {
        self.is_bottom_surface = is_bottom_surface;
    }

    /// Override the bridge classification flag (host-only, for testing).
    ///
    /// Per docs/02_ir_schemas.md §SurfaceClassificationIR, the host populates
    /// this from `ObjectSurfaceData.bridge_regions`. Default constructors leave
    /// the flag `false`.
    #[doc(hidden)]
    pub fn set_is_bridge(&mut self, is_bridge: bool) {
        self.is_bridge = is_bridge;
    }

    /// Override the bridge areas (host-only, for testing).
    #[doc(hidden)]
    pub fn set_bridge_areas(&mut self, bridge_areas: Vec<ExPolygon>) {
        self.bridge_areas = bridge_areas;
    }

    /// Override the bridge orientation (host-only, for testing).
    #[doc(hidden)]
    pub fn set_bridge_orientation_deg(&mut self, bridge_orientation_deg: f32) {
        self.bridge_orientation_deg = bridge_orientation_deg;
    }

    /// Returns the SurfaceClassificationIR-derived support eligibility flag.
    ///
    /// Used by `Layer::Support` modules as the default-eligibility predicate when
    /// neither `SupportEnforcer` nor `SupportBlocker` paint applies; see
    /// docs/01_system_architecture.md and docs/02_ir_schemas.md.
    pub fn needs_support(&self) -> bool {
        self.needs_support
    }

    /// Returns true if this region was classified as a top surface.
    ///
    /// Used by the infill stage to determine whether to emit `TopSolidInfill`
    /// paths instead of `SparseInfill`.
    pub fn is_top_surface(&self) -> bool {
        self.is_top_surface
    }

    /// Returns true if this region was classified as a bottom surface.
    ///
    /// Used by the infill stage to determine whether to emit `BottomSolidInfill`
    /// paths instead of `SparseInfill`.
    pub fn is_bottom_surface(&self) -> bool {
        self.is_bottom_surface
    }

    /// Returns true if this region was classified as a bridge region.
    ///
    /// Used by the infill stage to determine whether to emit `BridgeInfill`
    /// paths. Bridge regions cannot rely on support below and require
    /// a different fill strategy.
    pub fn is_bridge(&self) -> bool {
        self.is_bridge
    }

    /// Returns the object ID this region belongs to.
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns the slice polygons for this region.
    pub fn polygons(&self) -> &[ExPolygon] {
        &self.polygons
    }

    /// Returns the infill areas for this region.
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Returns the effective layer height at this Z.
    pub fn effective_layer_height(&self) -> f32 {
        self.effective_layer_height
    }

    /// Returns the Z height of this region.
    pub fn z(&self) -> f32 {
        self.z
    }

    /// Returns true if this region has non-planar surfaces.
    pub fn has_nonplanar(&self) -> bool {
        self.has_nonplanar
    }

    /// Returns the boundary paint data for this region.
    ///
    /// Per-semantic, per-polygon, per-point paint values annotated by
    /// the paint-region-annotator (SlicePostProcess stage). Empty map
    /// if no paint data applies to this region.
    pub fn boundary_paint(&self) -> &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
        &self.boundary_paint
    }

    /// Returns the per-layer expanded bridge polygons.
    ///
    /// Empty if this region is not classified as a bridge region.
    pub fn bridge_areas(&self) -> &[ExPolygon] {
        &self.bridge_areas
    }

    /// Returns the best bridge direction across all valid bridge regions (degrees).
    pub fn bridge_orientation_deg(&self) -> f32 {
        self.bridge_orientation_deg
    }

    /// Override the held-claims set (host-only, for testing).
    ///
    /// Modules only emit fill paths for roles they hold. Empty means the
    /// module holds all four fill-role claims (rectilinear default).
    #[doc(hidden)]
    pub fn set_held_claims(&mut self, held_claims: Vec<String>) {
        self.held_claims = held_claims;
    }

    /// Returns the set of fill-role claim IDs held by the module that
    /// produced this region.
    ///
    /// Used by the infill stage to gate path emission: a module may only
    /// emit TopSolidInfill if it holds `claim:top-fill`, etc.
    /// Empty means the module holds all four fill-role claims (legacy
    /// fail-open default for paths that bypass the host resolver — see
    /// `should_emit` for the convention).
    pub fn held_claims(&self) -> &[String] {
        &self.held_claims
    }

    /// Returns true if this module is allowed to emit `role` for this region.
    ///
    /// Mapping:
    /// - `TopSolidInfill`     ↔ `claim:top-fill`
    /// - `BottomSolidInfill`  ↔ `claim:bottom-fill`
    /// - `BridgeInfill`       ↔ `claim:bridge-fill`
    /// - `SparseInfill`       ↔ `claim:sparse-fill`
    ///
    /// Roles outside the four fill claims (walls, support, ironing, …) are
    /// always allowed — `should_emit` returns true for them.
    ///
    /// Convention: an empty `held_claims` is treated as "holds all four"
    /// so test fixtures and code paths that bypass `dispatch_layer_call`
    /// keep the pre-packet-37 default behavior. Production dispatch
    /// populates the set authoritatively via
    /// `validation::resolve_held_claims`, after which `should_emit`
    /// reflects the configured holder per role.
    pub fn should_emit(&self, role: ExtrusionRole) -> bool {
        let claim = match role {
            ExtrusionRole::TopSolidInfill => "claim:top-fill",
            ExtrusionRole::BottomSolidInfill => "claim:bottom-fill",
            ExtrusionRole::BridgeInfill => "claim:bridge-fill",
            ExtrusionRole::SparseInfill => "claim:sparse-fill",
            _ => return true,
        };
        if self.held_claims.is_empty() {
            return true;
        }
        self.held_claims.iter().any(|c| c == claim)
    }
}

/// Read-only view of a perimeter region.
///
/// Matches WIT `resource perimeter-region-view` from ir-types.wit.
/// Host constructs these; modules cannot construct them.
#[derive(Debug, Clone, Default)]
pub struct PerimeterRegionView {
    object_id: ObjectId,
    region_id: RegionId,
    wall_loops: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
    seam_candidates: Vec<SeamCandidate>,
    /// Resolved seam position, if set by seam-placer during WallPostProcess.
    resolved_seam: Option<SeamPosition>,
}

impl PerimeterRegionView {
    /// Override the object ID (host-only, for testing).
    #[doc(hidden)]
    pub fn set_object_id(&mut self, object_id: impl Into<ObjectId>) {
        self.object_id = object_id.into();
    }

    /// Override the region ID (host-only, for testing).
    #[doc(hidden)]
    pub fn set_region_id(&mut self, region_id: RegionId) {
        self.region_id = region_id;
    }

    /// Override the wall loops (host-only, for testing).
    #[doc(hidden)]
    pub fn set_wall_loops(&mut self, wall_loops: Vec<WallLoop>) {
        self.wall_loops = wall_loops;
    }

    /// Override the infill areas (host-only, for testing).
    #[doc(hidden)]
    pub fn set_infill_areas(&mut self, infill_areas: Vec<ExPolygon>) {
        self.infill_areas = infill_areas;
    }

    /// Override the seam candidates (host-only, for testing).
    #[doc(hidden)]
    pub fn set_seam_candidates(&mut self, seam_candidates: Vec<SeamCandidate>) {
        self.seam_candidates = seam_candidates;
    }

    /// Override the resolved seam (host-only, for testing).
    #[doc(hidden)]
    pub fn set_resolved_seam(&mut self, resolved_seam: Option<SeamPosition>) {
        self.resolved_seam = resolved_seam;
    }

    /// Returns the object ID this region belongs to.
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns the wall loops for this region.
    pub fn wall_loops(&self) -> &[WallLoop] {
        &self.wall_loops
    }

    /// Returns the infill areas for this region.
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Returns the seam candidates for this region.
    pub fn seam_candidates(&self) -> &[SeamCandidate] {
        &self.seam_candidates
    }

    /// Returns the resolved seam position, if set by seam-placer.
    pub fn resolved_seam(&self) -> Option<&SeamPosition> {
        self.resolved_seam.as_ref()
    }
}

/// Read-only projection of a host-staged `LayerCollectionIR.ordered_entities`
/// entry, exposed to `Layer::PathOptimization` modules via
/// `LayerCollectionBuilder::get_ordered_entities`.
///
/// Mirrors WIT `record ordered-entity-view` from
/// `wit/deps/ir-types.wit`. Modules consume this snapshot to compute a
/// permutation proposal which they then submit through
/// `LayerCollectionBuilder::set_entity_order`.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedEntityView {
    /// Index of this entity in the host-staged ordering at snapshot time.
    pub original_index: u32,
    /// Region key (layer / object / region triple) the entity belongs to.
    pub region_key: RegionKey,
    /// Extrusion role of the entity.
    pub role: ExtrusionRole,
    /// First point of the entity's path (with width / flow factor).
    pub start_point: Point3WithWidth,
    /// Last point of the entity's path (with width / flow factor).
    pub end_point: Point3WithWidth,
    /// Total number of points on the entity's path (including endpoints).
    pub point_count: u32,
}
