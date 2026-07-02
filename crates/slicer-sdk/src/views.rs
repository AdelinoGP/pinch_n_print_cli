//! View types for reading IR data.
//!
//! These are read-only views that the host constructs and passes to modules.
//! Per docs/03_wit_and_manifest.md (ir-types.wit), view resources cannot be
//! constructed by modules.

use std::collections::HashMap;

use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    ExPolygon, ExtrusionRole, ObjectId, PaintSemantic, PaintValue, Point3WithWidth, RegionId,
    RegionKey, SeamCandidate, SeamPosition, SurfaceGroup, WallLoop,
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
    segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    /// Ordered (paint-semantic-name, value) pairs identifying this region's paint
    /// variant. Carries the painted FuzzySkin signal (`("fuzzy_skin", Flag(true))`)
    /// so perimeter generators can enable per-vertex jitter without reading
    /// FuzzySkin from `segment_annotations` (D14).
    variant_chain: Vec<(String, PaintValue)>,
    /// SurfaceClassificationIR-derived eligibility flag. Surfaces the documented
    /// `needs_support` signal from docs/02_ir_schemas.md into the support stage
    /// so generators can apply the default eligibility rules from
    /// docs/01_system_architecture.md when no support paint override applies.
    needs_support: bool,
    /// Minimum depth (0 = exposed) within the top shell zone for this region.
    /// `None` outside any top shell. Populated by `PrePass::ShellClassification`.
    top_shell_index: Option<u8>,
    /// Minimum depth within the bottom shell zone. `None` outside any bottom shell.
    bottom_shell_index: Option<u8>,
    /// Polygon-precise area to solid-fill from top shell projection.
    top_solid_fill: Vec<ExPolygon>,
    /// Polygon-precise area to solid-fill from bottom shell projection.
    bottom_solid_fill: Vec<ExPolygon>,
    /// True when this region is classified as a bridge region by SurfaceClassificationIR.
    /// Indicates the region needs BridgeInfill fill and cannot rely on support below.
    is_bridge: bool,
    /// Per-layer expanded bridge polygons (empty if not a bridge region).
    bridge_areas: Vec<ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).
    bridge_orientation_deg: f32,
    /// Sparse-only infill polygon after host-side fill partition.
    /// Empty before `Layer::Perimeters` commits; populated by the host's
    /// `sync_perimeter_infill_areas_into_slice` hook so each fill claim
    /// holder emits over a disjoint canonical polygon. See
    /// `crates/slicer-runtime/src/region_partition.rs`.
    sparse_infill_area: Vec<ExPolygon>,
    /// Claim IDs held by the module that produced this region.
    /// Modules may only emit fill paths for roles they hold; empty means
    /// the module holds no fill claims for this region (suppresses all fill
    /// emission via `should_emit`).
    held_claims: Vec<String>,
    /// Clean model boundary for the painted cell group this region belongs to
    /// (AC-22b). `Some(boundary)` for painted regions; `None` for unpainted regions
    /// (the perimeter generator then traces the region's own polygon in full).
    external_contour: Option<Vec<ExPolygon>>,
    /// Overhang area polygons for this region, flattened across all severity
    /// quartiles. Populated by the host populator from
    /// `SurfaceClassificationIR.overhang_quartile_polygons` at this region's
    /// layer, pre-filtered to overlap the region (packet 107).
    overhang_areas: Vec<ExPolygon>,
    /// Quartile-banded overhang polygons for this region's layer, pre-filtered
    /// to overlap the region (packet 107). Preserves the per-quartile grouping
    /// that `overhang_areas` flattens away, for callers that need
    /// severity-aware handling (quartile 1 = least severe, 4 = most severe).
    overhang_quartile_polygons: Vec<QuartileBand>,
    /// Surface group resolved from `SurfaceClassificationIR` for this region's
    /// `nonplanar_surface` ID. `None` when no surface group applies.
    surface_group: Option<SurfaceGroup>,
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
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            // `needs_support: true` matches the pre-TASK-200e `new()` default
            // (see docs/02_ir_schemas.md §IR 2). Test fixtures that predate
            // the SurfaceClassificationIR wiring observe the prior
            // "all candidates eligible" behavior.
            needs_support: true,
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
            held_claims: Vec::new(),
            external_contour: None,
            overhang_areas: Vec::new(),
            overhang_quartile_polygons: Vec::new(),
            surface_group: None,
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
    pub fn set_segment_annotations(
        &mut self,
        segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    ) {
        self.segment_annotations = segment_annotations;
    }

    /// Override the paint variant chain (host-only, for testing).
    #[doc(hidden)]
    pub fn set_variant_chain(&mut self, variant_chain: Vec<(String, PaintValue)>) {
        self.variant_chain = variant_chain;
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

    /// Override the top shell index (host-only, for testing).
    ///
    /// `Some(0)` = exposed top surface; `Some(n)` = `n` layers below an
    /// exposed top within the shell zone; `None` = outside any top shell.
    /// Populated by `PrePass::ShellClassification`.
    #[doc(hidden)]
    pub fn set_top_shell_index(&mut self, top_shell_index: Option<u8>) {
        self.top_shell_index = top_shell_index;
    }

    /// Override the bottom shell index (host-only, for testing).
    ///
    /// `Some(0)` = exposed bottom surface; `Some(n)` = `n` layers above an
    /// exposed bottom within the shell zone; `None` = outside any bottom shell.
    #[doc(hidden)]
    pub fn set_bottom_shell_index(&mut self, bottom_shell_index: Option<u8>) {
        self.bottom_shell_index = bottom_shell_index;
    }

    /// Override the polygon-precise top solid fill area (host-only, for testing).
    #[doc(hidden)]
    pub fn set_top_solid_fill(&mut self, top_solid_fill: Vec<ExPolygon>) {
        self.top_solid_fill = top_solid_fill;
    }

    /// Override the polygon-precise bottom solid fill area (host-only, for testing).
    #[doc(hidden)]
    pub fn set_bottom_solid_fill(&mut self, bottom_solid_fill: Vec<ExPolygon>) {
        self.bottom_solid_fill = bottom_solid_fill;
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

    /// Override the sparse-only infill polygon (host-only, for testing).
    ///
    /// Production code path: host writes this from
    /// `sync_perimeter_infill_areas_into_slice` at `Layer::Perimeters` commit.
    /// Tests can populate it directly to exercise infill modules in isolation.
    #[doc(hidden)]
    pub fn set_sparse_infill_area(&mut self, sparse_infill_area: Vec<ExPolygon>) {
        self.sparse_infill_area = sparse_infill_area;
    }

    /// Override the resolved surface group (host-only, for testing).
    ///
    /// Production code path: the wasm-host marshal layer resolves
    /// `region.nonplanar_surface` against `SurfaceClassificationIR` (see
    /// `crates/slicer-wasm-host/src/marshal/in_.rs::sliced_region_to_data`).
    /// Tests that exercise non-planar-surface consumers directly (bypassing
    /// the WASM boundary) can populate it here (P108 T-074).
    #[doc(hidden)]
    pub fn set_surface_group(&mut self, surface_group: Option<SurfaceGroup>) {
        self.surface_group = surface_group;
    }

    /// Returns the SurfaceClassificationIR-derived support eligibility flag.
    ///
    /// Used by `Layer::Support` modules as the default-eligibility predicate when
    /// neither `SupportEnforcer` nor `SupportBlocker` paint applies; see
    /// docs/01_system_architecture.md and docs/02_ir_schemas.md.
    pub fn needs_support(&self) -> bool {
        self.needs_support
    }

    /// Returns the minimum top-shell depth for this region.
    ///
    /// `Some(0)` indicates an exposed top surface; `Some(n)` indicates `n`
    /// layers below an exposed top within the configured top-shell zone;
    /// `None` indicates the region is outside any top shell. Used by the
    /// infill stage to determine whether to emit `TopSolidInfill`.
    pub fn top_shell_index(&self) -> Option<u8> {
        self.top_shell_index
    }

    /// Returns the minimum bottom-shell depth for this region.
    ///
    /// `Some(0)` indicates an exposed bottom surface; `Some(n)` indicates `n`
    /// layers above an exposed bottom within the configured bottom-shell zone;
    /// `None` indicates the region is outside any bottom shell.
    pub fn bottom_shell_index(&self) -> Option<u8> {
        self.bottom_shell_index
    }

    /// Returns the polygon-precise top solid fill area for this region.
    ///
    /// Empty when `top_shell_index()` is `None`. The shape is the
    /// shrinking-shadow projection from the exposed top down through the
    /// shell zone, computed in `PrePass::ShellClassification`.
    ///
    /// **Semantic shift after `Layer::Perimeters` commit.** The host's
    /// `sync_perimeter_infill_areas_into_slice`
    /// (`crates/slicer-runtime/src/region_partition.rs`) clips this polygon
    /// to `perimeter.infill_areas` and deduplicates it by precedence
    /// `bridge > bottom > top > sparse`. So at Layer::Infill / Layer::InfillPostProcess
    /// stages this getter returns the **post-partition clipped** polygon
    /// (pairwise-disjoint with `bottom_solid_fill`, `bridge_areas`, and
    /// `sparse_infill_area`). At earlier stages (PrePass, `Layer::Slice`,
    /// `Layer::SlicePostProcess`, `Layer::Perimeters` before the commit
    /// hook) it returns the raw `PrePass::ShellClassification` projection.
    pub fn top_solid_fill(&self) -> &[ExPolygon] {
        &self.top_solid_fill
    }

    /// Returns the polygon-precise bottom solid fill area for this region.
    ///
    /// Empty when `bottom_shell_index()` is `None`.
    ///
    /// **Semantic shift after `Layer::Perimeters` commit** — see the matching
    /// note on [`Self::top_solid_fill`]. Post-partition this polygon is
    /// clipped to `perimeter.infill_areas` and deduped against
    /// `bridge_areas`.
    pub fn bottom_solid_fill(&self) -> &[ExPolygon] {
        &self.bottom_solid_fill
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
    pub fn segment_annotations(&self) -> &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
        &self.segment_annotations
    }

    /// Returns the region's paint variant chain as ordered
    /// `(paint_semantic_name, value)` pairs. Carries the painted FuzzySkin
    /// signal (`("fuzzy_skin", Flag(true))`); empty for the legacy
    /// single-variant flow.
    pub fn variant_chain(&self) -> &[(String, PaintValue)] {
        &self.variant_chain
    }

    /// Returns the per-layer expanded bridge polygons.
    ///
    /// Empty if this region is not classified as a bridge region.
    ///
    /// **Semantic shift after `Layer::Perimeters` commit** — see the note on
    /// [`Self::top_solid_fill`]. Post-partition this polygon is clipped to
    /// `perimeter.infill_areas`; it is the highest-precedence fill polygon so
    /// it is never deduped by another role.
    pub fn bridge_areas(&self) -> &[ExPolygon] {
        &self.bridge_areas
    }

    /// Returns the best bridge direction across all valid bridge regions (degrees).
    pub fn bridge_orientation_deg(&self) -> f32 {
        self.bridge_orientation_deg
    }

    /// Returns the sparse-only infill polygon for this region.
    ///
    /// After `Layer::Perimeters` commit (or `Layer::PerimetersPostProcess`
    /// commit on the three arms that newly stage a perimeter), this is the
    /// precedence-dedup remainder: `perimeter.infill_areas −
    /// union(bridge_areas, bottom_solid_fill, top_solid_fill)`. Fill modules
    /// holding `claim:sparse-fill` emit `SparseInfill` paths over this
    /// polygon and no others. Empty at all earlier stages (PrePass,
    /// `Layer::Slice`, `Layer::SlicePostProcess`) and when the entire
    /// wall-inset is covered by solid/bridge fill.
    pub fn sparse_infill_area(&self) -> &[ExPolygon] {
        &self.sparse_infill_area
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
    /// Empty means the module holds no fill claims for this region —
    /// `should_emit` returns false for all four fill roles.
    pub fn held_claims(&self) -> &[String] {
        &self.held_claims
    }

    /// Override the external contour (host-only, for testing).
    ///
    /// `Some(boundary)` for painted regions; `None` for unpainted regions.
    #[doc(hidden)]
    pub fn set_external_contour(&mut self, boundary: Option<Vec<ExPolygon>>) {
        self.external_contour = boundary;
    }

    /// Returns the clean model boundary for this region's painted cell group
    /// (AC-22b). The perimeter generator keeps an outer-wall edge only when it lies
    /// on this boundary and skips edges interior to it (paint-cell interfaces).
    /// `None` means no dedup: trace the region's own polygon in full.
    pub fn external_contour(&self) -> Option<&Vec<ExPolygon>> {
        self.external_contour.as_ref()
    }

    /// Returns the overhang area polygons for this region, flattened across
    /// all severity quartiles.
    ///
    /// Populated by the host populator from
    /// `SurfaceClassificationIR.overhang_quartile_polygons` for this region's
    /// layer, pre-filtered to overlap the region (packet 107). Empty when the
    /// PrePass has not yet run `PrePass::OverhangAnnotation`, or when this
    /// region has no overlapping overhang polygons at its layer. Safe to call
    /// at any stage; callers must handle the empty case.
    pub fn overhang_areas(&self) -> &[ExPolygon] {
        &self.overhang_areas
    }

    /// Override the overhang quartile polygons (host-only, for testing).
    #[doc(hidden)]
    pub fn set_overhang_quartile_polygons(&mut self, bands: Vec<QuartileBand>) {
        self.overhang_quartile_polygons = bands;
    }

    /// Override the flattened overhang area polygons (host-only, for testing).
    ///
    /// Production code populates `overhang_areas` from
    /// `SurfaceClassificationIR.overhang_quartile_polygons` at the WASM
    /// marshal boundary (`crates/slicer-wasm-host/src/marshal/in_.rs`). Tests
    /// that exercise overhang consumers directly (bypassing the WASM
    /// boundary) can populate it here (P108 T-077).
    #[doc(hidden)]
    pub fn set_overhang_areas(&mut self, overhang_areas: Vec<ExPolygon>) {
        self.overhang_areas = overhang_areas;
    }

    /// Returns the quartile-banded overhang polygons for this region's layer,
    /// pre-filtered to overlap the region (packet 107).
    ///
    /// Preserves the per-quartile grouping (1 = least severe, 4 = most
    /// severe) that [`Self::overhang_areas`] flattens away. Empty under the
    /// same conditions as `overhang_areas`.
    pub fn overhang_quartile_polygons(&self) -> &[QuartileBand] {
        &self.overhang_quartile_polygons
    }

    /// Returns the surface group resolved from `SurfaceClassificationIR` for
    /// this region's `nonplanar_surface` ID.
    ///
    /// `None` when this region has no associated surface group (the region is
    /// planar or the PrePass has not yet emitted surface classification data).
    pub fn surface_group(&self) -> Option<&SurfaceGroup> {
        self.surface_group.as_ref()
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
    /// Convention: an empty `held_claims` means the module holds no fill
    /// claims for this region — `should_emit` returns `false` for all four
    /// fill roles. Production dispatch populates the set authoritatively via
    /// `validation::resolve_held_claims`; test fixtures must set the correct
    /// held_claims to match the module's manifest claims.
    pub fn should_emit(&self, role: ExtrusionRole) -> bool {
        let claim = match role {
            ExtrusionRole::TopSolidInfill => "claim:top-fill",
            ExtrusionRole::BottomSolidInfill => "claim:bottom-fill",
            ExtrusionRole::BridgeInfill => "claim:bridge-fill",
            ExtrusionRole::SparseInfill => "claim:sparse-fill",
            _ => return true,
        };
        // When held_claims is empty (dispatch resolved this module holds
        // nothing for this region), suppress all fill emission. Test fixtures
        // must set the correct held_claims to match the module's manifest
        // claims — see `infill_partitioned_input_tdd.rs` for examples.
        if self.held_claims.is_empty() {
            return false;
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
    /// Resolved seam position, if set by seam-placer during PerimetersPostProcess.
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
    /// Resolved tool/extruder index (first-class selector since the
    /// region_id↔tool split; read this for the entity's tool, NOT
    /// `region_key.region_id`, which is now a pure region identity).
    pub tool_index: u32,
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
