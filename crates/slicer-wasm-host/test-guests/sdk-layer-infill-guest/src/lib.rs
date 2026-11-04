//! TASK-109 round-trip witness for the `world-layer` world
//! (Layer::Infill stage). Authored purely via `#[slicer_module]`.
//!
//! Blocker-#2 content witness: this guest proves the macro-emitted
//! layer-world glue performs real resource-level deep copy. It reads
//! the per-region fields of the incoming `SliceRegionView` values
//! (`polygons()`, `infill_areas()`, `z()`, `effective_layer_height()`)
//! and writes SDK `InfillOutputBuilder::push_sparse_path` so the host
//! can observe whether real content reached the trait body *and*
//! whether the drain-back path applied the SDK builder's output.

use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;
use witness::{SdkInfillWitness, SliceRegionFieldsWitness};

pub struct SdkLayerInfillModule;

#[slicer_module]
impl LayerModule for SdkLayerInfillModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &slicer_sdk::PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Preserved: the typed `ConfigView` intentional-error channel
        // (witnesses typed-config round-trip). The error message now
        // also carries the observed region count so a failing test
        // can distinguish "trait body ran but saw zero regions" from
        // "trait body did not run".
        if let Some(code) = config.get_int("intentional_error_code") {
            let region_count = regions.len();
            let total_polys: usize = regions.iter().map(|r| r.polygons().len()).sum();
            let msg = format!(
                "sdk-layer-infill-guest: typed error from config at layer {layer_index} \
                 (regions={region_count}, total_polygons={total_polys})"
            );
            return Err(ModuleError::non_fatal(code as u32, msg));
        }

        // Deep-copy input witness: encode real per-region content via
        // `witness::SdkInfillWitness` so the field meanings are defined
        // once in the witness crate (see witness/src/lib.rs).
        let (first_z, first_lh, first_infill_n) = regions
            .first()
            .map(|r| {
                (
                    r.z(),
                    r.effective_layer_height(),
                    r.infill_areas().len() as f32,
                )
            })
            .unwrap_or((0.0, 0.0, 0.0));

        let w = SdkInfillWitness {
            region_count: regions.len() as f32,
            total_polys: regions.iter().map(|r| r.polygons().len() as f32).sum(),
            first_region_z: first_z,
            first_region_layer_height: first_lh,
            first_region_infill_areas_len: first_infill_n,
        };

        let path = ExtrusionPath3D {
            points: w.encode(layer_index),
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        };
        output
            .push_sparse_path(path)
            .map_err(|e| ModuleError::fatal(1, e))?;

        // Optional field-witness emission for the
        // `adapt_slice_regions_completeness_tdd` regression test. Gated by an
        // explicit int config key so all other tests that drive this guest
        // see the original single-path output unchanged.
        if config.get_int("emit_field_witness") == Some(1) {
            if let Some(r) = regions.first() {
                let object_id_bytes = r.object_id().as_bytes();
                let object_id_byte_sum: u32 = object_id_bytes.iter().map(|b| *b as u32).sum();
                let first_held_claim_byte_sum: u32 = r
                    .held_claims()
                    .first()
                    .map(|s| s.as_bytes().iter().map(|b| *b as u32).sum())
                    .unwrap_or(0);
                let region_id_value: u64 = *r.region_id();
                let fields = SliceRegionFieldsWitness {
                    object_id_len: object_id_bytes.len() as f32,
                    region_id: region_id_value as f32,
                    polygons_len: r.polygons().len() as f32,
                    infill_areas_len: r.infill_areas().len() as f32,
                    effective_layer_height: r.effective_layer_height(),
                    z: r.z(),
                    has_nonplanar: if r.has_nonplanar() { 1.0 } else { 0.0 },
                    segment_annotations_len: r.segment_annotations().len() as f32,
                    top_shell_index: r.top_shell_index().map(|n| n as f32).unwrap_or(-1.0),
                    bottom_shell_index: r.bottom_shell_index().map(|n| n as f32).unwrap_or(-1.0),
                    top_solid_fill_len: r.top_solid_fill().len() as f32,
                    bottom_solid_fill_len: r.bottom_solid_fill().len() as f32,
                    is_bridge: if r.is_bridge() { 1.0 } else { 0.0 },
                    bridge_areas_len: r.bridge_areas().len() as f32,
                    bridge_orientation_deg: r.bridge_orientation_deg(),
                    sparse_infill_area_len: r.sparse_infill_area().len() as f32,
                    held_claims_len: r.held_claims().len() as f32,
                    first_held_claim_byte_sum: first_held_claim_byte_sum as f32,
                    object_id_byte_sum: object_id_byte_sum as f32,
                    marker: SliceRegionFieldsWitness::MARKER,
                };
                // Header point keeps the path's first-point z equal to the
                // SliceRegionView's z so the host's Z envelope guard admits
                // the path through `push_sparse_path`. Without this the
                // drain-back silently drops the path (it `let _ = ...`s
                // builder errors).
                let fpath = ExtrusionPath3D {
                    points: fields.encode(r.z()),
                    role: ExtrusionRole::SparseInfill,
                    speed_factor: 1.0,
                    tool_index: None,
                    order_lock: None,
                };
                output
                    .push_sparse_path(fpath)
                    .map_err(|e| ModuleError::fatal(2, e))?;
            }
        }

        // Optional per-region/invocation config witness for
        // `adapt_slice_regions_completeness_tdd::per_region_config_and_invocation_config_are_observed`.
        // The producer layout is specified here and decoded independently in
        // the test file against literal expectations. Gated by its own int
        // config key so every other test driving this guest sees unchanged
        // output.
        //
        // One path per incoming region:
        //   point[0] header   : x=0, y=0, z=region z (Z-envelope safe),
        //                       width=0, flow_factor=CONFIG_WITNESS_MARKER
        //   point[1] payload  : x=region config `line_width` (-1 when absent),
        //                       y=invocation config `line_width` (-1 when absent),
        //                       z=region config `private_probe` present (0/1),
        //                       width=invocation config `private_probe` present (0/1),
        //                       flow_factor=region config present (0/1)
        //   point[2] identity/authority:
        //                       x=region_id as f32,
        //                       y=region config `extruder` present (0/1),
        //                       z=region `extruder` int (-1 when unavailable),
        //                       width=region `private_probe` int (-1 when unavailable),
        //                       flow_factor=held top-fill (1) + sparse-fill (2)
        // Config accessors cannot distinguish undeclared from absent: the host
        // fixture must independently supply both keys before filtering.
        if config.get_int("emit_config_witness") == Some(1) {
            const CONFIG_WITNESS_MARKER: f32 = 9.0;
            let invocation_width = config.get_float("line_width").unwrap_or(-1.0) as f32;
            let invocation_probe = if config.get("private_probe").is_some() {
                1.0
            } else {
                0.0
            };
            for r in regions.iter() {
                let region_cfg = r.config();
                let region_width = region_cfg
                    .and_then(|c| c.get_float("line_width"))
                    .unwrap_or(-1.0) as f32;
                let region_probe = if region_cfg
                    .map(|c| c.get("private_probe").is_some())
                    .unwrap_or(false)
                {
                    1.0
                } else {
                    0.0
                };
                let has_region_config = if region_cfg.is_some() { 1.0 } else { 0.0 };
                let region_extruder = if region_cfg.and_then(|c| c.get("extruder")).is_some() {
                    1.0
                } else {
                    0.0
                };
                let held_roles = f32::from(r.held_claims().iter().any(|c| c == "claim:top-fill"))
                    + 2.0 * f32::from(r.held_claims().iter().any(|c| c == "claim:sparse-fill"));
                let points = vec![
                    slicer_ir::Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: r.z(),
                        width: 0.0,
                        flow_factor: CONFIG_WITNESS_MARKER,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    },
                    slicer_ir::Point3WithWidth {
                        x: region_width,
                        y: invocation_width,
                        z: region_probe,
                        width: invocation_probe,
                        flow_factor: has_region_config,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    },
                    slicer_ir::Point3WithWidth {
                        x: *r.region_id() as f32,
                        y: region_extruder,
                        z: region_cfg.and_then(|c| c.get_int("extruder")).unwrap_or(-1) as f32,
                        width: region_cfg
                            .and_then(|c| c.get_int("private_probe"))
                            .unwrap_or(-1) as f32,
                        flow_factor: held_roles,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: None,
                    },
                ];
                let path = ExtrusionPath3D {
                    points,
                    role: ExtrusionRole::SparseInfill,
                    speed_factor: 1.0,
                    tool_index: None,
                    order_lock: None,
                };
                output
                    .push_sparse_path(path)
                    .map_err(|e| ModuleError::fatal(3, e))?;
            }
        }

        Ok(())
    }
}
