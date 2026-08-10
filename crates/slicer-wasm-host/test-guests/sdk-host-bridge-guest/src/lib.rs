use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::{self, ClipOperation, OffsetJoinType};
use slicer_sdk::prelude::*;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PrepassModule;

pub struct SdkHostBridgeGuest;

#[slicer_module]
impl PrepassModule for SdkHostBridgeGuest {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support_geometry(
        &self,
        _objects: &[MeshObjectView],
        _layer_plan: &LayerPlanView,
        _region_segmentation: &RegionSegmentationView,
        _support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let object_id = config.get_string("bridge_probe_object").unwrap_or("cube");
        let hit = host::raycast_z_down(&object_id, 5.0, 5.0, 50.0);
        let bounds =
            host::object_bounds(&object_id).map_err(|e| ModuleError::fatal(901, e.to_string()))?;
        let normal = host::surface_normal_at(&object_id, 5.0, 5.0, 10.0);

        let square = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 100_000, y: 0 },
                    Point2 {
                        x: 100_000,
                        y: 100_000,
                    },
                    Point2 { x: 0, y: 100_000 },
                ],
            },
            holes: Vec::new(),
        };
        let offset = host::offset_polygons(&[square.clone()], 1.0, OffsetJoinType::Miter, 0.0);
        let clip = host::clip_polygons(&[square.clone()], &[], ClipOperation::Union);
        let simplified = host::simplify_polygon(&square.contour, 0.0);
        let time_a = host::now_us();
        let time_b = host::now_us();
        let (min_x, max_x) = offset
            .first()
            .map(|p| {
                p.contour
                    .points
                    .iter()
                    .map(|p| p.x)
                    .fold((i64::MAX, i64::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
            })
            .unwrap_or((0, 0));
        let width = (max_x - min_x) as f32 / 10_000.0;
        let message = format!(
            "raycast={:?};bounds_z={:.3};normal_z={:?};offset_width={:.3};clip_count={};simplify_points={};now_a={};now_b={}",
            hit, bounds.max.z, normal.map(|p| p.z), width, clip.len(), simplified.points.len(), time_a, time_b
        );
        output
            .push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Info,
                code: 901,
                layer: Some(-1),
                object_id: Some(object_id.to_string()),
                message,
            })
            .map_err(|e| ModuleError::fatal(902, e))?;
        Ok(())
    }
}
