//! Shared native/guest observation codec; deliberately contains no stage exports.
//!
//! The single UTF-8 piece is `observations`, terminated by a newline. Facts are
//! `path field=value ...` lines; strings use Rust's escaped quoted-string syntax,
//! floats use their shortest Debug representation, and polygon points are native
//! 100 nm integer units. Paint tags are flag/scalar/tool_index/custom; scalars
//! additionally include their eight-hex-digit IEEE bits (including signed zero).
//! Counts precede records, including optional-present empty collections. Returned
//! vectors retain the backing's canonical presentation order; only native maps
//! (segment annotations) and visible config keys are sorted here. Tests must
//! independently assert nonempty counts before comparing literal facts.
//!
//! Controls must be the module's declared-key-filtered *invocation* config, never
//! a substitute region config. `whole_print_fixture_mode` is `standard` (default),
//! `publish_read_errors`, `fail_after_staging`, or `attempt_denied_reads`.
//! The latter also attempts the pilot-only fixture read and a support-plan read
//! intended to be undeclared by the negative-control caller. All modes log actual
//! typed read errors and continue reading. The last mode puts a valid piece,
//! then returns ModuleError, exercising failed-transaction discard, not poisoning.
//!
//! `whole_print_batch_ops` is a semicolon-separated sequence, executed in supplied
//! order *after* the full standard reads. Empty/missing means no additional ops:
//! - `layer_count`
//! - `METHOD:OFFSET:MAX_ITEMS`, METHOD = layers | surface_objects | mesh_objects |
//!   support_analysis | support_geometry | support_plan
//! - `METHOD:LAYER_INDEX:OFFSET:MAX_ITEMS`, METHOD = regions | region_configs |
//!   selected_targets | surface_layers | seams
//! - `METHOD:OBJECT_ID:OFFSET:MAX_ITEMS`, METHOD = mesh_vertices | mesh_triangles
//! - `mesh_bounds:OBJECT_ID`
//! - `METHOD:OBJECT_ID:X:Y:Z`, METHOD = raycast | normal (Z is start_z for raycast)
//!
//! Numbers are decimal u64 (layer indices u32), query coordinates f32. Object IDs
//! in this control grammar cannot contain `:` or `;`. Bad control syntax is a
//! fixture ModuleError, not a fabricated PreparationInputError. Each operation
//! gets `batch[N] op="..."` then the same records/counts/errors as a standard read.
//!
//! Before *every* typed facade call: host::log_info("wpi read METHOD"). On error:
//! host::log_error("err METHOD DISPLAY") and that same actual-error line in the
//! piece, followed by an error-context line naming arguments. Only the explicit
//! denied-read mode attempts opaque fixture access; no host mesh-service bypass,
//! future layer-output access, or expected data.
//!
//! The current preparation facade does not expose invocation config. Callers
//! without it pass None, which is explicitly observed as unavailable; controls
//! then default to standard/no extra batches, not region-derived fake controls.

use slicer_ir::{BoundingBox3, ConfigView, ExPolygon, PaintSemantic, Point3, Point3WithWidth};
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::{log_error, log_info};
use slicer_sdk::preparation::*;

const FIXTURE_ERROR: u32 = 57440;
const ALL: u64 = u64::MAX;

/// Completion behavior, independent of the input reader's authorization policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FixtureMode {
    #[default]
    Standard,
    PublishReadErrors,
    FailAfterStaging,
    AttemptDeniedReads,
}

impl FixtureMode {
    /// Parse only the explicit invocation control; unknown values are errors.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        match config.get_string("whole_print_fixture_mode") {
            None if !config.contains_key("whole_print_fixture_mode") => Ok(Self::Standard),
            Some("standard") => Ok(Self::Standard),
            Some("publish_read_errors") => Ok(Self::PublishReadErrors),
            Some("fail_after_staging") => Ok(Self::FailAfterStaging),
            Some("attempt_denied_reads") => Ok(Self::AttemptDeniedReads),
            value => Err(fixture_error(format!(
                "invalid whole_print_fixture_mode: {value:?}"
            ))),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::PublishReadErrors => "publish_read_errors",
            Self::FailAfterStaging => "fail_after_staging",
            Self::AttemptDeniedReads => "attempt_denied_reads",
        }
    }
}

fn fixture_error(message: impl Into<String>) -> ModuleError {
    ModuleError::non_fatal(FIXTURE_ERROR, message.into())
}

#[derive(Default)]
struct Observations {
    lines: Vec<String>,
    errors: Vec<String>,
}

impl Observations {
    fn line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    fn read<T, E: std::fmt::Display>(
        &mut self,
        method: &str,
        args: &str,
        read: impl FnOnce() -> Result<T, E>,
    ) -> Option<T> {
        log_info(&format!("wpi read {method}"));
        self.line(format!("read {method} {args}"));
        match read() {
            Ok(value) => Some(value),
            Err(error) => {
                let actual = format!("err {method} {error}");
                log_error(&actual);
                self.line(actual.clone());
                self.line(format!("error_context {method} {args}"));
                self.errors.push(actual);
                None
            }
        }
    }

    fn rows<T>(&mut self, path: &str, rows: &[T], record: fn(&mut Self, &str, &T)) {
        self.line(format!("{path} count={}", rows.len()));
        for (index, row) in rows.iter().enumerate() {
            record(self, &format!("{path}[{index}]"), row);
        }
    }

    fn optional_rows<T>(
        &mut self,
        path: &str,
        rows: Option<&[T]>,
        record: fn(&mut Self, &str, &T),
    ) {
        match rows {
            None => self.line(format!("{path} absent")),
            Some(rows) => {
                self.line(format!("{path} present"));
                self.rows(path, rows, record);
            }
        }
    }

    fn polygons(&mut self, path: &str, polygons: &[ExPolygon]) {
        self.line(format!(
            "{path} count={} geometry={}",
            polygons.len(),
            polygon_text(polygons)
        ));
    }

    fn config(&mut self, path: &str, config: &ConfigView) {
        self.line(format!("{path} keys={:?}", config.keys()));
        // Dedicated lines ensure absent/undeclared is not confused with zero.
        for key in ["line_width", "private_probe"] {
            self.line(format!("{path} {key}={}", config_value(config, key)));
        }
        for key in config.keys() {
            if key != "line_width" && key != "private_probe" {
                self.line(format!("{path} {key:?}={}", config_value(config, &key)));
            }
        }
    }
}

fn config_value(config: &ConfigView, key: &str) -> String {
    config
        .get(key)
        .map_or_else(|| "absent".into(), |value| format!("{value:?}"))
}

fn paint(value: &PreparationPaintValue) -> String {
    match value {
        PreparationPaintValue::Flag(value) => format!("flag({value})"),
        PreparationPaintValue::Scalar(value) => {
            format!("scalar({value:?};bits={:08x})", value.to_bits())
        }
        PreparationPaintValue::ToolIndex(value) => format!("tool_index({value})"),
        PreparationPaintValue::Custom(value) => format!("custom({value:?})"),
    }
}

fn optional_paint(value: &Option<PreparationPaintValue>) -> String {
    value.as_ref().map_or_else(|| "none".into(), paint)
}

fn chain(chain: &PreparationVariantChain) -> String {
    format!(
        "[{}]",
        chain
            .iter()
            .map(|(semantic, value)| format!("{semantic:?}={}", paint(value)))
            .collect::<Vec<_>>()
            .join(":")
    )
}

fn key(key: &PreparationRegionKey) -> String {
    identity(
        key.global_layer_index,
        &key.object_id,
        key.region_id,
        &key.variant_chain,
    )
}

fn identity(layer: u32, object: &str, region: u64, variants: &PreparationVariantChain) -> String {
    format!("[{layer}|{object:?}|{region}|{}]", chain(variants))
}

fn point(point: &Point3) -> String {
    format!("({:?},{:?},{:?})", point.x, point.y, point.z)
}

fn bounds(bounds: &BoundingBox3) -> String {
    format!("min={} max={}", point(&bounds.min), point(&bounds.max))
}

fn wide_point(point: &Point3WithWidth) -> String {
    format!(
        "point=({:?},{:?},{:?}) width={:?} flow={:?} quartile={:?} dist_to_top={:?} overhang_distance={:?}",
        point.x, point.y, point.z, point.width, point.flow_factor,
        point.overhang_quartile, point.dist_to_top_mm, point.overhang_distance_mm
    )
}

fn polygon_text(polygons: &[ExPolygon]) -> String {
    fn ring(polygon: &slicer_ir::Polygon) -> String {
        format!(
            "[{}]",
            polygon
                .points
                .iter()
                .map(|point| format!("({},{})", point.x, point.y))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    format!(
        "[{}]",
        polygons
            .iter()
            .map(|polygon| format!(
                "{{contour={},holes=[{}]}}",
                ring(&polygon.contour),
                polygon.holes.iter().map(ring).collect::<Vec<_>>().join(",")
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn record_layer(out: &mut Observations, path: &str, layer: &PreparationLayer) {
    out.line(format!(
        "{path} index={} z={:?} nonplanar={} sync={} raft={} active_regions={}",
        layer.index,
        layer.z,
        layer.has_nonplanar,
        layer.is_sync_layer,
        layer.is_raft,
        layer.active_regions.len()
    ));
    for (index, active) in layer.active_regions.iter().enumerate() {
        let path = format!("{path}.active[{index}]");
        let shell = active.nonplanar_shell.as_ref().map_or_else(
            || "none".into(),
            |shell| format!("({},{})", shell.surface_group_id, shell.shell_index),
        );
        out.line(format!(
            "{path} object={:?} region={} height={:?} shell={shell} catchup={} catchup_z_bottom={:?} tool={}",
            active.object_id, active.region_id, active.effective_layer_height,
            active.is_catchup_layer, active.catchup_z_bottom, active.tool_index
        ));
        out.config(&format!("{path}.config"), &active.config);
    }
}

fn record_region(out: &mut Observations, path: &str, region: &PreparationRegion) {
    let phase = match region.phase {
        PreparationInputPhase::CommittedPrepass => "committed_prepass",
    };
    out.line(format!(
        "{path} key={} z={:?} nonplanar_surface={:?} height={:?} top={:?} bottom={:?} is_bridge={} orientation={:?} internal_bridge_angles={:?} phase={phase}",
        identity(region.global_layer_index, &region.object_id, region.region_id, &region.variant_chain),
        region.z, region.nonplanar_surface, region.effective_layer_height,
        region.top_shell_index, region.bottom_shell_index, region.is_bridge,
        region.bridge_orientation_deg, region.internal_bridge_angles_deg
    ));
    for (name, polygons) in [
        ("polygons", &region.polygons),
        ("infill_areas", &region.infill_areas),
        ("top_solid_fill", &region.top_solid_fill),
        ("bottom_solid_fill", &region.bottom_solid_fill),
        ("bridge_areas", &region.bridge_areas),
        ("sparse_infill_area", &region.sparse_infill_area),
        ("internal_solid_fill", &region.internal_solid_fill),
        ("internal_bridge_areas", &region.internal_bridge_areas),
        ("raft_fill", &region.raft_fill),
    ] {
        out.polygons(&format!("{path}.{name}"), polygons);
    }
    let mut annotations: Vec<_> = region.segment_annotations.iter().collect();
    annotations.sort_by(|a, b| a.0.cmp(b.0));
    out.line(format!("{path}.annotations count={}", annotations.len()));
    for (semantic, polygons) in annotations {
        let semantic = match semantic {
            PaintSemantic::Material => "material".into(),
            PaintSemantic::FuzzySkin => "fuzzy_skin".into(),
            PaintSemantic::SupportEnforcer => "support_enforcer".into(),
            PaintSemantic::SupportBlocker => "support_blocker".into(),
            PaintSemantic::Custom(name) => format!("custom({name:?})"),
        };
        out.line(format!(
            "{path}.annotations semantic={semantic} polygons={}",
            polygons.len()
        ));
        for (index, segments) in polygons.iter().enumerate() {
            out.line(format!(
                "{path}.annotations[{semantic}][{index}] count={} values=[{}]",
                segments.len(),
                segments
                    .iter()
                    .map(optional_paint)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
}

fn record_config(out: &mut Observations, path: &str, region: &PreparationRegionConfig) {
    out.line(format!("{path} key={}", key(&region.key)));
    out.config(path, &region.config);
}

fn record_surface_object(out: &mut Observations, path: &str, object: &PreparationSurfaceObject) {
    out.line(format!(
        "{path} object={:?} facets={} groups={} bridges={} overhangs={}",
        object.object_id,
        object.facet_classes.len(),
        object.surface_groups.len(),
        object.bridge_regions.len(),
        object.overhang_regions.len()
    ));
    for (index, class) in object.facet_classes.iter().enumerate() {
        let class = match class {
            PreparationFacetClass::Normal => "normal".into(),
            PreparationFacetClass::NearHorizontal { slope_angle_deg } => {
                format!("near_horizontal({slope_angle_deg:?})")
            }
            PreparationFacetClass::Overhang { angle_deg } => format!("overhang({angle_deg:?})"),
            PreparationFacetClass::Bridge => "bridge".into(),
            PreparationFacetClass::TopSurface => "top_surface".into(),
            PreparationFacetClass::BottomSurface => "bottom_surface".into(),
        };
        out.line(format!("{path}.facet[{index}] class={class}"));
    }
    for (index, group) in object.surface_groups.iter().enumerate() {
        out.line(format!(
            "{path}.group[{index}] id={} facets={:?} z_min={:?} z_max={:?} area={:?} printable={} shell_count={}",
            group.id, group.facet_indices, group.z_min, group.z_max, group.area_mm2,
            group.printable, group.shell_count
        ));
    }
    for (index, bridge) in object.bridge_regions.iter().enumerate() {
        let path = format!("{path}.bridge[{index}]");
        out.line(format!(
            "{path} id={} facets={:?} direction={:?} anchor_width={:?} length={:?} margin={:?} valid={}",
            bridge.id, bridge.facet_indices, bridge.bridge_direction_deg,
            bridge.anchor_width_mm, bridge.bridge_length_mm, bridge.expansion_margin_mm,
            bridge.is_valid
        ));
        out.polygons(&format!("{path}.footprint"), &bridge.xy_footprint);
    }
    for (index, overhang) in object.overhang_regions.iter().enumerate() {
        let path = format!("{path}.overhang[{index}]");
        out.line(format!(
            "{path} id={} facets={:?} angle={:?} needs_support={}",
            overhang.id, overhang.facet_indices, overhang.max_angle_deg, overhang.needs_support
        ));
        out.polygons(&format!("{path}.footprint"), &overhang.xy_footprint);
    }
}

fn record_surface_layer(out: &mut Observations, path: &str, layer: &PreparationSurfaceLayer) {
    out.line(format!(
        "{path} object={:?} layer={} quartiles={}",
        layer.object_id,
        layer.global_layer_index,
        layer.overhang_quartile_polygons.len()
    ));
    for (index, band) in layer.overhang_quartile_polygons.iter().enumerate() {
        let path = format!("{path}.quartile[{index}]");
        out.line(format!("{path} value={}", band.quartile));
        out.polygons(&path, &band.polygons);
    }
    out.polygons(
        &format!("{path}.prev_boundaries"),
        &layer.prev_layer_boundaries,
    );
}

fn record_mesh(out: &mut Observations, path: &str, object: &PreparationMeshObject) {
    out.line(format!(
        "{path} object={:?} vertices={} triangles={} paint_layers={}",
        object.object_id,
        object.vertex_count,
        object.triangle_count,
        object.paint_layers.len()
    ));
    out.line(format!(
        "{path}.bounds {}",
        object
            .bounds
            .as_ref()
            .map_or_else(|| "absent".into(), bounds)
    ));
    for (index, layer) in object.paint_layers.iter().enumerate() {
        let path = format!("{path}.paint[{index}]");
        out.line(format!(
            "{path} semantic={:?} facets={} values=[{}] strokes={}",
            layer.semantic,
            layer.facet_values.len(),
            layer
                .facet_values
                .iter()
                .map(optional_paint)
                .collect::<Vec<_>>()
                .join(","),
            layer.strokes.len()
        ));
        for (index, stroke) in layer.strokes.iter().enumerate() {
            out.line(format!(
                "{path}.stroke[{index}] semantic={:?} value={} vertices={} triangles_flat=[{}]",
                stroke.semantic,
                paint(&stroke.value),
                stroke.triangles.len(),
                stroke
                    .triangles
                    .iter()
                    .map(point)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
}

fn record_point(out: &mut Observations, path: &str, value: &Point3) {
    out.line(format!("{path} {}", point(value)));
}

fn record_triangle(out: &mut Observations, path: &str, triangle: &(u32, u32, u32)) {
    out.line(format!(
        "{path} ({},{},{})",
        triangle.0, triangle.1, triangle.2
    ));
}

fn record_query(out: &mut Observations, path: &str, result: &PreparationMeshQueryResult) {
    let value = match result {
        PreparationMeshQueryResult::RaycastZDown(hit) => format!("raycast_z_down={hit:?}"),
        PreparationMeshQueryResult::SurfaceNormalAt(normal) => format!(
            "surface_normal_at={}",
            normal.as_ref().map_or_else(|| "none".into(), point)
        ),
        PreparationMeshQueryResult::ObjectBounds(value) => {
            format!("object_bounds {}", bounds(value))
        }
    };
    out.line(format!("{path} {value}"));
}

fn record_seam(out: &mut Observations, path: &str, seam: &PreparationSeam) {
    out.line(format!(
        "{path} key={} candidates={}",
        key(&seam.region_key),
        seam.scored_candidates.len()
    ));
    out.line(format!(
        "{path}.chosen {} wall_index={}",
        wide_point(&seam.chosen_candidate.point),
        seam.chosen_candidate.wall_index
    ));
    for (index, candidate) in seam.scored_candidates.iter().enumerate() {
        let reason = match candidate.reason {
            PreparationSeamReason::Concave => "concave",
            PreparationSeamReason::Aligned => "aligned",
            PreparationSeamReason::UserForced => "user_forced",
            PreparationSeamReason::Sharp => "sharp",
        };
        out.line(format!(
            "{path}.candidate[{index}] {} score={:?} reason={reason}",
            wide_point(&candidate.position),
            candidate.score
        ));
    }
}

fn record_geometry_entry(
    out: &mut Observations,
    path: &str,
    entry: &PreparationSupportGeometryEntry,
) {
    out.line(format!(
        "{path} key={}",
        identity(
            entry.key.global_support_layer_index,
            &entry.key.object_id,
            entry.key.region_id,
            &entry.key.variant_chain
        )
    ));
    out.polygons(path, &entry.polygons);
}

fn record_analysis(out: &mut Observations, path: &str, batch: &PreparationSupportAnalysisBatch) {
    out.line(format!(
        "{path} candidates={} occupancy={} cantilever={} termination={} territory={} families={} settings={:?}",
        batch.candidates.len(), batch.model_occupancy.len(), batch.cantilever_surfaces.len(),
        batch.termination_surfaces.len(), batch.support_territory.len(),
        batch.family_assignments.len(), batch.shared_settings
    ));
    out.polygons(
        &format!("{path}.envelope"),
        &batch.baseline_feasible_envelope,
    );
    for (index, candidate) in batch.candidates.iter().enumerate() {
        let path = format!("{path}.candidate[{index}]");
        let source = &candidate.source;
        out.line(format!(
            "{path} id={} source={} z_units={} enforced={} blocked={}",
            candidate.id,
            identity(
                source.global_layer_index,
                &source.object_id,
                source.region_id,
                &source.variant_chain
            ),
            source.z_units,
            candidate.enforced,
            candidate.blocked
        ));
        out.polygons(&path, &candidate.geometry);
    }
    for (name, entries) in [
        ("occupancy", &batch.model_occupancy),
        ("cantilever", &batch.cantilever_surfaces),
        ("termination", &batch.termination_surfaces),
        ("territory", &batch.support_territory),
    ] {
        out.rows(&format!("{path}.{name}"), entries, record_geometry_entry);
    }
    for (index, assignment) in batch.family_assignments.iter().enumerate() {
        out.line(format!(
            "{path}.family[{index}] object={:?} region={} chain={} family={:?}",
            assignment.key.object_id,
            assignment.key.region_id,
            chain(&assignment.key.variant_chain),
            assignment.family_id
        ));
    }
}

fn record_geometry(out: &mut Observations, path: &str, batch: &PreparationSupportGeometryBatch) {
    out.line(format!(
        "{path} height={:?} top_z_distance={:?}",
        batch.support_layer_height_mm, batch.support_top_z_distance_mm
    ));
    out.rows(
        &format!("{path}.entries"),
        &batch.entries,
        record_geometry_entry,
    );
}

fn record_plan(out: &mut Observations, path: &str, batch: &PreparationSupportPlanBatch) {
    out.line(format!("{path} entries={}", batch.entries.len()));
    match &batch.raft_plan {
        None => out.line(format!("{path}.raft absent")),
        Some(raft) => out.line(format!(
            "{path}.raft layers={} density={:?} base={} interface={}",
            raft.raft_layers,
            raft.raft_first_layer_density,
            raft.base_raft_layers,
            raft.interface_raft_layers
        )),
    }
    for (index, entry) in batch.entries.iter().enumerate() {
        let path = format!("{path}.entry[{index}]");
        let decline = match entry.decline_reason {
            None => "none",
            Some(PreparationSupportPlanDeclineReason::DeclinedPolicy) => "declined_policy",
            Some(PreparationSupportPlanDeclineReason::NoRoute) => "no_route",
            Some(PreparationSupportPlanDeclineReason::Blocked) => "blocked",
            Some(PreparationSupportPlanDeclineReason::UnsupportedMode) => "unsupported_mode",
        };
        out.line(format!(
            "{path} layer={} object={:?} region={} variant_chain={} family={:?} demand={:?} body={:?} anchor_layer={} anchor_z={} roles={} capabilities={:?} provenance={:?} decline={decline}",
            entry.global_layer_index, entry.object_id, entry.region_id, chain(&entry.variant_chain), entry.family_id,
            entry.demand_ids, entry.body_ids, entry.anchor_layer_index, entry.anchor_z,
            entry.roles.len(), entry.capabilities, entry.provenance
        ));
        for (index, role) in entry.roles.iter().enumerate() {
            let path = format!("{path}.role[{index}]");
            let role_name = match role.role {
                PreparationSupportPlanRole::SupportBody => "support_body",
                PreparationSupportPlanRole::TopInterface => "top_interface",
                PreparationSupportPlanRole::BaseInterface => "base_interface",
                PreparationSupportPlanRole::BottomInterface => "bottom_interface",
                PreparationSupportPlanRole::RaftRelated => "raft_related",
            };
            out.line(format!("{path} role={role_name}"));
            out.polygons(&path, &role.regions);
        }
        match &entry.skeleton {
            None => out.line(format!("{path}.skeleton absent")),
            Some(skeleton) => out.line(format!(
                "{path}.skeleton points={} values=[{}] wall_counts={:?}",
                skeleton.points.len(),
                skeleton
                    .points
                    .iter()
                    .map(point)
                    .collect::<Vec<_>>()
                    .join(","),
                skeleton.wall_counts
            )),
        }
    }
}

fn target_source(source: &PreparationTargetSource) -> String {
    match source {
        PreparationTargetSource::Model { key: source, members } => format!(
            "model(key={},members=[{}])", key(source),
            members.iter().map(key).collect::<Vec<_>>().join(",")
        ),
        PreparationTargetSource::SupportCarrier {
            entry_index, object_id, region_id, variant_chain, family,
            demand_ids, body_ids, provenance, anchor_layer_index,
        } => format!(
            "support_carrier(entry_index={entry_index},object={object_id:?},region={region_id},chain={},family={family:?},demand={demand_ids:?},body={body_ids:?},provenance={provenance:?},anchor_layer={anchor_layer_index})",
            chain(variant_chain)
        ),
        PreparationTargetSource::Raft { global_layer_index } => {
            format!("raft(layer={global_layer_index})")
        }
        PreparationTargetSource::AnchoredStage { anchor_layer_index } => {
            format!("anchored_stage(anchor_layer={anchor_layer_index})")
        }
    }
}

fn record_target(out: &mut Observations, path: &str, target: &PreparationSelectedTarget) {
    out.line(format!(
        "{path} layer={} stage={:?} invoke={} source={} claims={:?} authored_coloring={} resolved_role_tool={:?}",
        target.global_layer_index, target.stage_id, target.invoke,
        target_source(&target.source), target.held_claims,
        target.authored_coloring, target.resolved_role_tool
    ));
    out.config(&format!("{path}.config"), &target.config);
}

/// Execute one range operation and encode every returned field (also for batches).
fn range_read(
    input: &PrintPreparationInput,
    out: &mut Observations,
    method: &str,
    layer: u32,
    object: &str,
    offset: u64,
    max_items: u64,
) -> Result<(), ModuleError> {
    let mut args = format!("offset={offset} max_items={max_items}");
    if matches!(
        method,
        "regions" | "region_configs" | "selected_targets" | "surface_layers" | "seams"
    ) {
        args = format!("layer={layer} {args}");
    }
    if matches!(method, "mesh_vertices" | "mesh_triangles") {
        args = format!("object={object:?} {args}");
    }
    macro_rules! required {
        ($call:expr, $record:ident) => {
            if let Some(rows) = out.read(method, &args, || $call) {
                out.rows(method, &rows, $record);
            }
        };
    }
    macro_rules! optional_rows {
        ($call:expr, $record:ident) => {
            if let Some(rows) = out.read(method, &args, || $call) {
                out.optional_rows(method, rows.as_deref(), $record);
            }
        };
    }
    macro_rules! optional_batch {
        ($call:expr, $record:ident) => {
            if let Some(batch) = out.read(method, &args, || $call) {
                match batch {
                    None => out.line(format!("{method} absent")),
                    Some(batch) => {
                        out.line(format!("{method} present"));
                        $record(out, method, &batch);
                    }
                }
            }
        };
    }
    match method {
        "layers" => required!(input.layers(offset, max_items), record_layer),
        "regions" => required!(input.regions(layer, offset, max_items), record_region),
        "region_configs" => required!(
            input.region_configs(layer, offset, max_items),
            record_config
        ),
        "selected_targets" => required!(
            input.selected_targets(layer, offset, max_items),
            record_target
        ),
        "surface_objects" => optional_rows!(
            input.surface_objects(offset, max_items),
            record_surface_object
        ),
        "surface_layers" => optional_rows!(
            input.surface_layers(layer, offset, max_items),
            record_surface_layer
        ),
        "seams" => optional_rows!(input.seams(layer, offset, max_items), record_seam),
        "mesh_objects" => required!(input.mesh_objects(offset, max_items), record_mesh),
        "mesh_vertices" => required!(input.mesh_vertices(object, offset, max_items), record_point),
        "mesh_triangles" => required!(
            input.mesh_triangles(object, offset, max_items),
            record_triangle
        ),
        "support_analysis" => {
            optional_batch!(input.support_analysis(offset, max_items), record_analysis)
        }
        "support_geometry" => {
            optional_batch!(input.support_geometry(offset, max_items), record_geometry)
        }
        "support_plan" => optional_batch!(input.support_plan(offset, max_items), record_plan),
        _ => {
            return Err(fixture_error(format!(
                "unknown whole_print_batch_ops method: {method}"
            )))
        }
    }
    Ok(())
}

fn query_read(input: &PrintPreparationInput, out: &mut Observations, query: PreparationMeshQuery) {
    let args = match &query {
        PreparationMeshQuery::RaycastZDown {
            object_id,
            x,
            y,
            start_z,
        } => {
            format!("raycast object={object_id:?} x={x:?} y={y:?} start_z={start_z:?}")
        }
        PreparationMeshQuery::SurfaceNormalAt { object_id, x, y, z } => {
            format!("normal object={object_id:?} x={x:?} y={y:?} z={z:?}")
        }
        PreparationMeshQuery::ObjectBounds { object_id } => format!("bounds object={object_id:?}"),
    };
    // Independent calls: an empty object's invalid bounds must not hide hits or normals.
    if let Some(results) = out.read("mesh_queries", &args, || input.mesh_queries(&[query])) {
        out.rows("mesh_queries", &results, record_query);
    }
}

fn batch_ops(
    input: &PrintPreparationInput,
    out: &mut Observations,
    spec: &str,
) -> Result<(), ModuleError> {
    for (index, operation) in spec
        .split(';')
        .filter(|op| !op.trim().is_empty())
        .enumerate()
    {
        let operation = operation.trim();
        out.line(format!("batch[{index}] op={operation:?}"));
        let fields: Vec<_> = operation.split(':').collect();
        let bad = || {
            fixture_error(format!(
                "invalid whole_print_batch_ops operation: {operation:?}"
            ))
        };
        let number = |text: &str| text.parse::<u64>().map_err(|_| bad());
        let coordinate = |text: &str| text.parse::<f32>().map_err(|_| bad());
        match fields.as_slice() {
            ["layer_count"] => {
                if let Some(count) = out.read("layer_count", "", || input.layer_count()) {
                    out.line(format!("layer_count={count}"));
                }
            }
            ["mesh_bounds", object] => query_read(
                input,
                out,
                PreparationMeshQuery::ObjectBounds {
                    object_id: (*object).into(),
                },
            ),
            [method @ ("raycast" | "normal"), object, x, y, z] => {
                let (x, y, z) = (coordinate(x)?, coordinate(y)?, coordinate(z)?);
                let object_id = (*object).into();
                let query = if *method == "raycast" {
                    PreparationMeshQuery::RaycastZDown {
                        object_id,
                        x,
                        y,
                        start_z: z,
                    }
                } else {
                    PreparationMeshQuery::SurfaceNormalAt { object_id, x, y, z }
                };
                query_read(input, out, query);
            }
            [method @ ("layers" | "surface_objects" | "mesh_objects" | "support_analysis"
            | "support_geometry" | "support_plan"), offset, max_items] => {
                range_read(
                    input,
                    out,
                    method,
                    0,
                    "",
                    number(offset)?,
                    number(max_items)?,
                )?;
            }
            [method @ ("regions" | "region_configs" | "selected_targets" | "surface_layers"
            | "seams"), layer, offset, max_items] => {
                let layer = u32::try_from(number(layer)?).map_err(|_| bad())?;
                range_read(
                    input,
                    out,
                    method,
                    layer,
                    "",
                    number(offset)?,
                    number(max_items)?,
                )?;
            }
            [method @ ("mesh_vertices" | "mesh_triangles"), object, offset, max_items] => {
                range_read(
                    input,
                    out,
                    method,
                    0,
                    object,
                    number(offset)?,
                    number(max_items)?,
                )?;
            }
            _ => return Err(bad()),
        }
    }
    Ok(())
}

/// Observe real typed results, then stage one complete module-owned text piece.
/// `invocation_config` must never be filled from a selected model's region config.
pub fn observe_all(
    input: &PrintPreparationInput,
    output: &mut PreparedPlanOutput,
    mode: FixtureMode,
    invocation_config: Option<&ConfigView>,
) -> Result<(), ModuleError> {
    let mut out = Observations::default();
    out.line(format!("mode={}", mode.name()));
    match invocation_config {
        Some(config) => out.config("invocation_config", config),
        None => out.line("invocation_config unavailable"),
    }
    if mode == FixtureMode::AttemptDeniedReads {
        // Keep the legacy fixture error's actual Display: it is not a typed
        // product read, and no expected error is substituted for its Result.
        out.read("read_fixture", "offset=0 max_bytes=1", || {
            input.read_fixture(0, 1)
        });
        out.read("support_plan", "offset=0 max_items=1", || {
            input.support_plan(0, 1)
        });
    }
    if let Some(count) = out.read("layer_count", "", || input.layer_count()) {
        out.line(format!("layer_count={count}"));
    }
    if let Some(layers) = out.read("layers", &format!("offset=0 max_items={ALL}"), || {
        input.layers(0, ALL)
    }) {
        out.rows("layers", &layers, record_layer);
        for layer in layers {
            for method in [
                "regions",
                "region_configs",
                "selected_targets",
                "surface_layers",
                "seams",
            ] {
                range_read(input, &mut out, method, layer.index, "", 0, ALL)?;
            }
        }
    }
    range_read(input, &mut out, "surface_objects", 0, "", 0, ALL)?;
    if let Some(objects) = out.read("mesh_objects", &format!("offset=0 max_items={ALL}"), || {
        input.mesh_objects(0, ALL)
    }) {
        out.rows("mesh_objects", &objects, record_mesh);
        for object in objects {
            for method in ["mesh_vertices", "mesh_triangles"] {
                range_read(input, &mut out, method, 0, &object.object_id, 0, ALL)?;
            }
            query_read(
                input,
                &mut out,
                PreparationMeshQuery::RaycastZDown {
                    object_id: object.object_id.clone(),
                    x: 0.25,
                    y: 0.25,
                    start_z: 2.0,
                },
            );
            query_read(
                input,
                &mut out,
                PreparationMeshQuery::SurfaceNormalAt {
                    object_id: object.object_id.clone(),
                    x: 0.25,
                    y: 0.25,
                    z: 1.0,
                },
            );
            query_read(
                input,
                &mut out,
                PreparationMeshQuery::ObjectBounds {
                    object_id: object.object_id,
                },
            );
        }
    }
    for method in ["support_analysis", "support_geometry", "support_plan"] {
        range_read(input, &mut out, method, 0, "", 0, ALL)?;
    }
    if let Some(config) = invocation_config {
        match config.get_string("whole_print_batch_ops") {
            Some(spec) => batch_ops(input, &mut out, spec)?,
            None if config.contains_key("whole_print_batch_ops") => {
                return Err(fixture_error("whole_print_batch_ops must be a string"));
            }
            None => {}
        }
    }
    out.line(format!("read_errors={}", out.errors.len()));
    output.put(
        "observations",
        format!("{}\n", out.lines.join("\n")).as_bytes(),
    )?;
    if mode == FixtureMode::FailAfterStaging {
        // Actual errors have already been logged before staging. The driver must
        // use those logs and owner current(), never discarded staging, as proof.
        return Err(fixture_error(format!(
            "whole-print fixture deliberate failure after staging; observed errors: {:?}",
            out.errors
        )));
    }
    Ok(())
}
