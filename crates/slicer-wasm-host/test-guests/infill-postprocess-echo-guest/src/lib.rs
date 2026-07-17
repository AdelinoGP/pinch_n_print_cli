//! Packet 130 (TASK: infill-postprocess-contract) echo witness for the
//! `Layer::InfillPostProcess` stage of the `world-layer` world.
//!
//! Echo semantics (AC-1): re-emits its `prior_infill` input verbatim through
//! `InfillOutputBuilder`, calling `begin_region` per prior region so the
//! host's origin-tagged drain reconstructs the SAME per-region buckets in the
//! committed replacement `InfillIR`. A per-region keyed comparison of the
//! committed IR against the pre-postprocess IR therefore proves the
//! `prior-infill` WIT parameter round-trips region identity and all three
//! bucket cardinalities.
//!
//! View witness (AC-2/3/4, gated by config int `emit_view_witness == 1`):
//! for each incoming `PerimeterRegionView`, emits solid-bucket witness paths
//! encoding the six ADR-0028 enrichment fields so contract tests can decode
//! the exact content the guest observed:
//!
//! - header path (single point, `width == HEADER_MARKER`):
//!   `x = tool_index`, `y = wall_source_region_id` (`-1.0` when `None`).
//! - one path per polygon per field (`width == FIELD_MARKER` on point 0):
//!   point 0 carries `x = field_id` (0 = sparse-infill-area, 1 =
//!   top-solid-fill, 2 = bottom-solid-fill, 3 = bridge-areas),
//!   `y = polygon index`, `flow_factor = hole count`; the remaining points
//!   are the polygon's contour vertices (scaled integer units cast to f32),
//!   followed by each hole's vertices in order.
//!
//! All witness points use `z = 0.0`; drive this guest at a layer whose Z
//! floor is `0.0` so the host's Z-envelope guard admits the paths.

use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

/// First-point width marker for the per-view header witness path.
const HEADER_MARKER: f32 = 777.0;
/// First-point width marker for per-polygon field witness paths.
const FIELD_MARKER: f32 = 888.0;

fn pt(x: f32, y: f32, width: f32, flow_factor: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.0,
        width,
        flow_factor,
        overhang_quartile: None,
    }
}

pub struct InfillPostprocessEchoModule;

#[slicer_module]
impl LayerModule for InfillPostprocessEchoModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill_postprocess(
        &self,
        _layer_index: u32,
        regions: &[PerimeterRegionView],
        prior_infill: &[slicer_ir::InfillRegion],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // ── Echo: re-emit the prior InfillIR buckets per region (AC-1) ──
        for r in prior_infill {
            output.begin_region(&r.object_id, r.region_id);
            for p in &r.sparse_infill {
                output
                    .push_sparse_path(p.clone())
                    .map_err(|e| ModuleError::fatal(1, e))?;
            }
            for p in &r.solid_infill {
                output
                    .push_solid_path(p.clone())
                    .map_err(|e| ModuleError::fatal(2, e))?;
            }
            for p in &r.ironing {
                output
                    .push_ironing_path(p.clone())
                    .map_err(|e| ModuleError::fatal(3, e))?;
            }
        }

        // ── Optional per-view field witness (AC-2/3/4) ──
        if config.get_int("emit_view_witness") == Some(1) {
            for v in regions {
                output.begin_region(v.object_id(), *v.region_id());

                let wall_source = v
                    .wall_source_region_id()
                    .map(|id| *id as f32)
                    .unwrap_or(-1.0);
                let header = ExtrusionPath3D {
                    points: vec![pt(v.tool_index() as f32, wall_source, HEADER_MARKER, 1.0)],
                    role: ExtrusionRole::TopSolidInfill,
                    speed_factor: 1.0,
                };
                output
                    .push_solid_path(header)
                    .map_err(|e| ModuleError::fatal(4, e))?;

                let fields: [&[slicer_ir::ExPolygon]; 4] = [
                    v.sparse_infill_area(),
                    v.top_solid_fill(),
                    v.bottom_solid_fill(),
                    v.bridge_areas(),
                ];
                for (field_id, polys) in fields.iter().enumerate() {
                    for (poly_idx, poly) in polys.iter().enumerate() {
                        let mut points = vec![pt(
                            field_id as f32,
                            poly_idx as f32,
                            FIELD_MARKER,
                            poly.holes.len() as f32,
                        )];
                        for p2 in &poly.contour.points {
                            points.push(pt(p2.x as f32, p2.y as f32, 0.4, 1.0));
                        }
                        for hole in &poly.holes {
                            for p2 in &hole.points {
                                points.push(pt(p2.x as f32, p2.y as f32, 0.4, 1.0));
                            }
                        }
                        output
                            .push_solid_path(ExtrusionPath3D {
                                points,
                                role: ExtrusionRole::TopSolidInfill,
                                speed_factor: 1.0,
                            })
                            .map_err(|e| ModuleError::fatal(5, e))?;
                    }
                }
            }
        }

        Ok(())
    }
}
