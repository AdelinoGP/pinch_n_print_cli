wit_bindgen::generate!({
    inline: r#"
        package slicer:world-prepass@1.0.0;

        interface geometry {
            record point3 { x: f32, y: f32, z: f32 }
            record bounding-box3 { min: point3, max: point3 }
            record point2 { x: s64, y: s64 }
            record polygon { points: list<point2> }
            record ex-polygon { contour: polygon, holes: list<polygon> }
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

            export run-mesh-segmentation: func(
                objects: list<mesh-object-view>,
                output: mesh-segmentation-output,
                config: config-view,
            ) -> result<_, module-error>;

            use geometry.{ex-polygon};

            record paint-region-entry {
                object-id: object-id,
                layer-index: u32,
                semantic: string,
                polygons: list<ex-polygon>,
                value: string,
            }
            resource paint-segmentation-output {
                push-paint-region: func(entry: paint-region-entry) -> result<_, string>;
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

            use geometry.{point3};

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

            record paint-segmentation-object-view {
                object-id: object-id,
                vertices: list<point3>,
                triangles: list<tuple<u32, u32, u32>>,
                paint-layers: list<paint-layer-view>,
                transform-matrix: list<f64>,
                participating-layer-indices: list<u32>,
            }

            resource layer-plan-output {
                push-layer: func(proposal: layer-proposal) -> result<_, string>;
            }

            export run-layer-planning: func(
                objects: list<object-id>,
                output: layer-plan-output,
                config: config-view,
            ) -> result<_, module-error>;

            // SeamPlanning stage
            record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
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
                objects: list<object-id>,
                output: seam-planning-output,
                config: config-view,
            ) -> result<_, module-error>;
        }
    "#,
    world: "prepass-module",
});

struct Component;

impl Guest for Component {
    fn run_mesh_analysis(_objects: Vec<ObjectId>, _output: MeshAnalysisOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_layer_planning(_objects: Vec<ObjectId>, _output: LayerPlanOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_mesh_segmentation(_objects: Vec<MeshObjectView>, _output: MeshSegmentationOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_paint_segmentation(_objects: Vec<PaintSegmentationObjectView>, _output: PaintSegmentationOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_seam_planning(_objects: Vec<ObjectId>, _output: SeamPlanningOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
