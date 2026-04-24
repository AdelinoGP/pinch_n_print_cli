//! Canonical `slicer:world-prepass@1.0.0` component binary for the
//! `PrePass::PaintSegmentation` stage.
//!
//! Projects 3D painted facets into 2D per-layer polygon regions and
//! emits them through the WIT `paint-segmentation-output::push-paint-
//! region` resource method. The host reshapes every entry into
//! `PaintRegionIR.per_layer[layer].semantic_regions[semantic]` via
//! `harvest_paint_segmentation_ir` (docs/03 world-prepass.wit;
//! docs/02 §Paint Region IR).
//!
//! # MVP region source
//!
//! Per-region entries are driven by host-supplied config keys of the
//! shape:
//!
//!   `paint_region:<object_id>:<layer_index>:<semantic>:<value>` =
//!     `"x0,y0;x1,y1;x2,y2;..."`
//!
//! One key per region, value is a semicolon-separated list of `x,y`
//! integer scaled coordinates (`slicer_ir` units — see
//! `slicer_ir::mm_to_units`). The guest parses each key, builds one
//! `paint-region-entry` with a single contour polygon and no holes,
//! and pushes through the WIT resource. Invalid coordinate lists fail
//! with a precise structured error that the host surfaces as
//! `PrepassExecutionError::FatalModule`.
//!
//! Unpainted meshes (Benchy) supply no `paint_region:*` keys and the
//! guest is a deterministic zero-entry no-op — a correct semantic
//! outcome.
//!
//! # Ordering
//!
//! Entries are sorted by `(object_index_in_objects, layer_index asc,
//! semantic asc, value asc)` before they are pushed, so two identical
//! runs produce byte-identical `PaintRegionIR.per_layer` maps.

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
                objects: list<mesh-object-view>,
                output: seam-planning-output,
                config: config-view,
            ) -> result<_, module-error>;
        }
    "#,
    world: "prepass-module",
});

use slicer::world_prepass::config_types::ConfigValue;
use slicer::world_prepass::geometry::{Point2, Polygon};

struct Component;

#[derive(Debug, Clone)]
struct ParsedEntry {
    object_id: String,
    layer_index: u32,
    semantic: String,
    value: String,
    polygon: Polygon,
}

fn parse_point(tok: &str) -> Option<Point2> {
    let mut parts = tok.split(',');
    let x: i64 = parts.next()?.trim().parse().ok()?;
    let y: i64 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Point2 { x, y })
}

fn parse_polygon(s: &str) -> Result<Polygon, String> {
    let points: Result<Vec<Point2>, String> = s
        .split(';')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| parse_point(t).ok_or_else(|| format!("invalid point '{t}'")))
        .collect();
    let points = points?;
    if points.len() < 3 {
        return Err(format!(
            "polygon must have >= 3 points, got {}",
            points.len()
        ));
    }
    Ok(Polygon { points })
}

fn parse_entry(key: &str, value: &ConfigValue) -> Option<Result<ParsedEntry, String>> {
    const PREFIX: &str = "paint_region:";
    let rest = key.strip_prefix(PREFIX)?;
    let mut parts = rest.splitn(4, ':');
    let object_id = match parts.next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Some(Err(format!("missing object_id in '{key}'"))),
    };
    let layer_str = match parts.next() {
        Some(s) => s,
        None => return Some(Err(format!("missing layer_index in '{key}'"))),
    };
    let layer_index: u32 = match layer_str.parse() {
        Ok(n) => n,
        Err(_) => return Some(Err(format!("invalid layer_index '{layer_str}' in '{key}'"))),
    };
    let semantic = match parts.next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Some(Err(format!("missing semantic in '{key}'"))),
    };
    let val_tag = match parts.next() {
        Some(s) => s.to_string(),
        None => return Some(Err(format!("missing value tag in '{key}'"))),
    };
    let polygon_str = match value {
        ConfigValue::StringVal(s) => s.clone(),
        _ => return Some(Err(format!("'{key}' must be a string value"))),
    };
    let polygon = match parse_polygon(&polygon_str) {
        Ok(p) => p,
        Err(e) => return Some(Err(format!("'{key}': {e}"))),
    };
    Some(Ok(ParsedEntry {
        object_id,
        layer_index,
        semantic,
        value: val_tag,
        polygon,
    }))
}

impl Guest for Component {
    fn run_mesh_analysis(
        _objects: Vec<ObjectId>,
        _output: MeshAnalysisOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_mesh_segmentation(
        _objects: Vec<MeshObjectView>,
        _output: MeshSegmentationOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_layer_planning(
        _objects: Vec<ObjectId>,
        _output: LayerPlanOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_paint_segmentation(
        objects: Vec<PaintSegmentationObjectView>,
        output: PaintSegmentationOutput,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        let mut parsed: Vec<ParsedEntry> = Vec::new();
        for key in config.keys() {
            let value = match config.get(&key) {
                Some(v) => v,
                None => continue,
            };
            match parse_entry(&key, &value) {
                None => continue,
                Some(Ok(e)) => parsed.push(e),
                Some(Err(msg)) => {
                    return Err(ModuleError {
                        code: 1,
                        message: format!("paint-segmentation: malformed config entry: {msg}"),
                        fatal: true,
                    });
                }
            }
        }

        let object_index = |id: &str| -> usize {
            objects
                .iter()
                .position(|o| o.object_id == id)
                .unwrap_or(objects.len())
        };
        parsed.sort_by(|a, b| {
            object_index(&a.object_id)
                .cmp(&object_index(&b.object_id))
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.layer_index.cmp(&b.layer_index))
                .then_with(|| a.semantic.cmp(&b.semantic))
                .then_with(|| a.value.cmp(&b.value))
        });

        for entry in parsed {
            let ex = ExPolygon {
                contour: entry.polygon,
                holes: Vec::new(),
            };
            let paint_entry = PaintRegionEntry {
                object_id: entry.object_id.clone(),
                layer_index: entry.layer_index,
                semantic: entry.semantic.clone(),
                polygons: vec![ex],
                value: entry.value.clone(),
            };
            output
                .push_paint_region(&paint_entry)
                .map_err(|e| ModuleError {
                    code: 2,
                    message: format!(
                        "push_paint_region rejected {}/{}/{}/{}: {}",
                        entry.object_id, entry.layer_index, entry.semantic, entry.value, e
                    ),
                    fatal: true,
                })?;
        }
        Ok(())
    }

    fn run_seam_planning(
        _objects: Vec<MeshObjectView>,
        _output: SeamPlanningOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
