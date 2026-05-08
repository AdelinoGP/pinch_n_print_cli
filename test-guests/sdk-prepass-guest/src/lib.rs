//! SDK prepass guest — multi-stage test fixture (Packet-43, Step 3).
//!
//! Implements all three prepass stages (MeshAnalysis, PaintSegmentation,
//! MeshSegmentation) in one WASM component so both the existing mesh-analysis
//! round-trip tests and the new paint/mesh-segmentation round-trip tests can
//! load the same `sdk-prepass-guest.component.wasm`.
//!
//! The `#[slicer_module]` macro cannot be used here because it enforces
//! exactly one stage per impl block and emits a fixed module name
//! (`__slicer_prepass_world_export`) that would collide if applied more
//! than once in the same crate. We therefore use raw `wit_bindgen::generate!`
//! inline (the same pattern as `test-guests/prepass-guest`) with the updated
//! WIT that includes `paint-value-input` and `layer-idx = s32`.
//!
//! # Stage dispatch
//!
//! | Stage            | Config key        | Config value       | Behaviour |
//! |------------------|-------------------|--------------------|-----------|
//! | run-mesh-analysis | `emit_mesh_analysis` | `N: i64` | emit N facet annotations + 1 surface-group per object |
//! | run-mesh-analysis | `intentional_error_code` | `code: i64` | return non-fatal error |
//! | run-paint-segmentation | `fixture_case` | `"hole_bearing"` | push one region with hole polygon |
//! | run-paint-segmentation | `fixture_case` | `"custom_payload"` | push one region with Custom paint value |
//! | run-paint-segmentation | `fixture_case` | `"empty_polygons"` | push one region with empty polygon list |
//! | run-mesh-segmentation | `fixture_case` | `"marks_basic"` | mark triangle 12 on obj-a |

wit_bindgen::generate!({
    inline: r#"
        package slicer:world-prepass@1.0.0;

        interface geometry {
            record point3 { x: f32, y: f32, z: f32 }
            record bounding-box3 { min: point3, max: point3 }
            record point2 { x: s64, y: s64 }
            record polygon { points: list<point2> }
            record ex-polygon { contour: polygon, holes: list<polygon> }
            record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
        }

        interface config-types {
            variant config-value {
                bool-val(bool), int-val(s64), float-val(f64),
                string-val(string), float-list(list<f64>), string-list(list<string>),
            }
            resource config-view {
                get:        func(key: string) -> option<config-value>;
                get-bool:   func(key: string) -> option<bool>;
                get-float:  func(key: string) -> option<f64>;
                get-int:    func(key: string) -> option<s64>;
                get-string: func(key: string) -> option<string>;
                keys:       func() -> list<string>;
            }
        }

        interface host-services {
            use geometry.{point3, bounding-box3, ex-polygon, polygon};
            type object-id = string;
            enum log-level { trace, debug, info, warn, error }
            log: func(level: log-level, message: string);
            raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
            surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
            object-bounds:      func(object-id: object-id) -> bounding-box3;
            enum clip-operation   { union, intersection, difference, xor }
            enum offset-join-type { miter, round, square }
            clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
            offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
            simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
            now-us: func() -> u64;
        }

        world prepass-module {
            import host-services;
            import config-types;
            type object-id = string;
            type region-id = string;
            record module-error { code: u32, message: string, fatal: bool }

            enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }
            record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }
            record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }

            use config-types.{config-view};
            use geometry.{point3, ex-polygon, point3-with-width};

            resource mesh-analysis-output {
                push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
                push-surface-group:    func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
            }

            export run-mesh-analysis: func(
                objects: list<object-id>,
                output: mesh-analysis-output,
                config: config-view,
            ) -> result<_, module-error>;

            resource mesh-segmentation-output {
                mark-triangle-paint: func(obj: object-id, facet-index: u32, semantic: string, value: string) -> result<_, string>;
            }

            variant paint-value-view {
                flag(bool),
                scalar(f32),
                tool-index(u32),
            }

            record paint-stroke-view {
                triangles: list<point3>,
                semantic: string,
                value: paint-value-view,
            }

            record paint-layer-view {
                semantic: string,
                facet-values: list<option<paint-value-view>>,
                strokes: list<paint-stroke-view>,
            }

            record mesh-object-view {
                object-id: object-id,
                vertices: list<point3>,
                triangles: list<tuple<u32, u32, u32>>,
                paint-layers: list<paint-layer-view>,
            }

            export run-mesh-segmentation: func(
                objects: list<mesh-object-view>,
                output: mesh-segmentation-output,
                config: config-view,
            ) -> result<_, module-error>;

            type layer-idx = s32;

            variant paint-value-input {
                flag(bool),
                scalar(f32),
                tool-index(u32),
                custom(string),
            }

            record paint-region-entry {
                object-id: object-id,
                layer-index: layer-idx,
                semantic: string,
                polygons: list<ex-polygon>,
                value: paint-value-input,
            }
            resource paint-segmentation-output {
                push-paint-region: func(entry: paint-region-entry) -> result<_, string>;
            }

            record paint-segmentation-object-view {
                object-id: object-id,
                vertices: list<point3>,
                triangles: list<tuple<u32, u32, u32>>,
                paint-layers: list<paint-layer-view>,
                transform-matrix: list<f64>,
                participating-layer-indices: list<u32>,
            }

            export run-paint-segmentation: func(
                objects: list<paint-segmentation-object-view>,
                output: paint-segmentation-output,
                config: config-view,
            ) -> result<_, module-error>;

            record region-layer-proposal {
                object-id: object-id, region-id: region-id,
                effective-layer-height: f32,
                is-catchup: bool, catchup-z-bottom: f32,
            }
            record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }

            resource layer-plan-output {
                push-layer: func(proposal: layer-proposal) -> result<_, string>;
            }

            export run-layer-planning: func(
                objects: list<object-id>,
                output: layer-plan-output,
                config: config-view,
            ) -> result<_, module-error>;

            record seam-reason { tag: string }
            record scored-seam-candidate {
                position: point3-with-width,
                score: f32,
                reason: seam-reason,
            }
            record seam-plan-entry {
                global-layer-index: u32,
                object-id: object-id,
                region-id: region-id,
                chosen-position: point3-with-width,
                chosen-wall-index: u32,
                scored-candidates: list<scored-seam-candidate>,
            }
            resource seam-planning-output {
                push-seam-plan: func(entry: seam-plan-entry) -> result<_, string>;
            }
            export run-seam-planning: func(
                objects: list<mesh-object-view>,
                output: seam-planning-output,
                config: config-view,
            ) -> result<_, module-error>;

            record support-plan-entry {
                global-layer-index: s32,
                object-id: object-id,
                region-id: region-id,
                branch-segments: list<list<point3-with-width>>,
            }
            record layer-plan-view-entry { global-layer-index: u32, z: f32, effective-layer-height: f32 }
            record layer-plan-view { layers: list<layer-plan-view-entry> }
            record region-segmentation-view-entry { object-id: object-id, layer-index: u32, region-ids: list<region-id> }
            record region-segmentation-view { entries: list<region-segmentation-view-entry> }
            record support-geometry-view-entry { global-support-layer-index: u32, object-id: object-id, region-id: region-id, outlines: list<ex-polygon> }
            record support-geometry-view { entries: list<support-geometry-view-entry> }
            record support-geometry-output { support-plan-entries: list<support-plan-entry> }

            export run-support-geometry: func(
                objects: list<mesh-object-view>,
                layer-plan: layer-plan-view,
                region-segmentation: region-segmentation-view,
                support-geometry: support-geometry-view,
            ) -> support-geometry-output;
        }
    "#,
    world: "prepass-module",
});

use crate::slicer::world_prepass::geometry::{Point2, Polygon};

struct Component;

impl Guest for Component {
    // ── run-mesh-analysis ─────────────────────────────────────────────────

    fn run_mesh_analysis(
        objects: Vec<ObjectId>,
        output: MeshAnalysisOutput,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        // Intentional error path (used by existing error-propagation tests).
        if let Some(code) = config.get_int("intentional_error_code") {
            return Err(ModuleError {
                code: code as u32,
                message: "sdk-prepass-guest: intentional typed error from config".to_string(),
                fatal: false,
            });
        }

        // Emit N facet annotations + 1 surface-group per object when
        // `emit_mesh_analysis` is set to a positive integer N.
        if let Some(n) = config.get_int("emit_mesh_analysis") {
            let n = n.max(0) as u32;
            for obj in &objects {
                for i in 0..n {
                    let class = match i % 6 {
                        0 => FacetClass::Normal,
                        1 => FacetClass::NearHorizontal,
                        2 => FacetClass::Overhang,
                        3 => FacetClass::Bridge,
                        4 => FacetClass::TopSurface,
                        _ => FacetClass::BottomSurface,
                    };
                    let ann = FacetAnnotation {
                        facet_index: i,
                        slope_angle_deg: (i as f32) * 10.0,
                        classification: class,
                    };
                    if let Err(e) = output.push_facet_annotation(obj, ann) {
                        return Err(ModuleError {
                            code: 8,
                            message: e,
                            fatal: true,
                        });
                    }
                }
                let grp = SurfaceGroupProposal {
                    facet_indices: (0..n).collect(),
                    z_min: 0.0,
                    z_max: (n as f32) * 0.2,
                    shell_count: 2,
                };
                if let Err(e) = output.push_surface_group(obj, &grp) {
                    return Err(ModuleError {
                        code: 9,
                        message: e,
                        fatal: true,
                    });
                }
            }
        }

        Ok(())
    }

    // ── run-paint-segmentation ────────────────────────────────────────────

    fn run_paint_segmentation(
        _objects: Vec<PaintSegmentationObjectView>,
        output: PaintSegmentationOutput,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        // Unit square contour (1 mm × 1 mm) encoded in 100 nm units.
        // 1 mm = 10_000 units (1 unit = 100 nm = 10⁻⁴ mm).
        let unit_square = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 10_000, y: 0 },
            Point2 { x: 10_000, y: 10_000 },
            Point2 { x: 0, y: 10_000 },
        ];
        // Inner hole (0.25 mm inset = 2500 units on each side).
        let inner_square = vec![
            Point2 { x: 2_500, y: 2_500 },
            Point2 { x: 7_500, y: 2_500 },
            Point2 { x: 7_500, y: 7_500 },
            Point2 { x: 2_500, y: 7_500 },
        ];

        let case = config.get_string("fixture_case");

        match case.as_deref() {
            Some("hole_bearing") => {
                let entry = PaintRegionEntry {
                    object_id: "obj-a".to_string(),
                    layer_index: 3,
                    semantic: "material".to_string(),
                    polygons: vec![ExPolygon {
                        contour: Polygon { points: unit_square },
                        holes: vec![Polygon { points: inner_square }],
                    }],
                    value: PaintValueInput::ToolIndex(7),
                };
                if let Err(e) = output.push_paint_region(&entry) {
                    return Err(ModuleError { code: 10, message: e, fatal: true });
                }
            }
            Some("custom_payload") => {
                let entry = PaintRegionEntry {
                    object_id: "obj-a".to_string(),
                    layer_index: 0,
                    semantic: "custom:my_profile".to_string(),
                    polygons: vec![ExPolygon {
                        contour: Polygon { points: unit_square },
                        holes: vec![],
                    }],
                    value: PaintValueInput::Custom("profile_high".to_string()),
                };
                if let Err(e) = output.push_paint_region(&entry) {
                    return Err(ModuleError { code: 10, message: e, fatal: true });
                }
            }
            Some("empty_polygons") => {
                let entry = PaintRegionEntry {
                    object_id: "obj-a".to_string(),
                    layer_index: 0,
                    semantic: "material".to_string(),
                    polygons: vec![],
                    value: PaintValueInput::ToolIndex(0),
                };
                if let Err(e) = output.push_paint_region(&entry) {
                    return Err(ModuleError { code: 10, message: e, fatal: true });
                }
            }
            _ => {
                // Default: no-op — keeps existing tests that load this guest happy.
            }
        }

        Ok(())
    }

    // ── run-mesh-segmentation ─────────────────────────────────────────────

    fn run_mesh_segmentation(
        _objects: Vec<MeshObjectView>,
        output: MeshSegmentationOutput,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        if let Some("marks_basic") = config.get_string("fixture_case").as_deref() {
            if let Err(e) = output.mark_triangle_paint(&"obj-a".to_string(), 12, "material", "3") {
                return Err(ModuleError { code: 8, message: e, fatal: true });
            }
        }
        // All other fixture_case values (and missing key): no-op.
        Ok(())
    }

    // ── run-layer-planning (no-op stub) ───────────────────────────────────

    fn run_layer_planning(
        _objects: Vec<ObjectId>,
        _output: LayerPlanOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    // ── run-seam-planning (no-op stub) ────────────────────────────────────

    fn run_seam_planning(
        _objects: Vec<MeshObjectView>,
        _output: SeamPlanningOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    // ── run-support-geometry (no-op stub) ─────────────────────────────────

    fn run_support_geometry(
        _objects: Vec<MeshObjectView>,
        _layer_plan: LayerPlanView,
        _region_segmentation: RegionSegmentationView,
        _support_geometry: SupportGeometryView,
    ) -> SupportGeometryOutput {
        SupportGeometryOutput {
            support_plan_entries: vec![],
        }
    }
}

export!(Component);
