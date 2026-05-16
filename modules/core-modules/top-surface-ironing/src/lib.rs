//! Top-surface ironing module.
//!
//! Runs in the `PostPass::LayerFinalization` stage with full read-only
//! visibility into every `LayerCollectionIR` for the print. For each
//! `(object_id, region_id)` pair, locates the highest layer index whose
//! ordered entities contain `ExtrusionRole::TopSolidInfill` paths, and emits
//! a low-flow rectilinear ironing pass over the union of those paths via the
//! `FinalizationOutputBuilder`. Mirrors the `skirt-brim` module's skeleton
//! and Orca's `PrintObject::ironing()` algorithm.
//!
//! Coordinate system: extrusion-path point coordinates are stored as mm
//! (`f32`) on `Point3WithWidth`; helper conversions from `1 unit = 100 nm`
//! integer space are not required here since the input geometry already
//! arrives in mm via `LayerCollectionView::ordered_entities()`.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::BTreeMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, PrintEntity,
    RegionKey,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Base speed (mm/s) used to normalise the ironing speed into a
/// `speed_factor` multiplier on the emitted `ExtrusionPath3D`.
const BASE_SPEED: f64 = 50.0;

/// Default ironing line width (mm).
const IRONING_LINE_WIDTH: f32 = 0.4;

/// Top-surface ironing module.
///
/// Consumes the full `&[LayerCollectionView]` and emits low-flow `Ironing`
/// extrusion paths only on the topmost layer per `(object_id, region_id)`.
#[derive(Debug)]
pub struct TopSurfaceIroning {
    ironing: bool,
    ironing_speed: f64,
    ironing_flow: f64,
    ironing_spacing: f64,
    ironing_pattern: String,
}

impl TopSurfaceIroning {
    /// Whether ironing is enabled.
    pub fn ironing(&self) -> bool {
        self.ironing
    }

    /// Ironing speed in mm/s.
    pub fn ironing_speed(&self) -> f64 {
        self.ironing_speed
    }

    /// Ironing flow multiplier.
    pub fn ironing_flow(&self) -> f64 {
        self.ironing_flow
    }

    /// Ironing line spacing in mm.
    pub fn ironing_spacing(&self) -> f64 {
        self.ironing_spacing
    }

    /// Ironing pattern name.
    pub fn ironing_pattern(&self) -> &str {
        &self.ironing_pattern
    }
}

/// Axis-aligned bounding box in mm.
#[derive(Debug, Clone, Copy)]
struct BBox2D {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

impl BBox2D {
    fn from_points<'a, I>(pts: I) -> Option<Self>
    where
        I: IntoIterator<Item = &'a Point3WithWidth>,
    {
        let mut bb: Option<BBox2D> = None;
        for pt in pts {
            match &mut bb {
                Some(b) => {
                    b.x_min = b.x_min.min(pt.x);
                    b.y_min = b.y_min.min(pt.y);
                    b.x_max = b.x_max.max(pt.x);
                    b.y_max = b.y_max.max(pt.y);
                }
                None => {
                    bb = Some(BBox2D {
                        x_min: pt.x,
                        y_min: pt.y,
                        x_max: pt.x,
                        y_max: pt.y,
                    });
                }
            }
        }
        bb
    }
}

/// Identifier of a top-solid region: `(object_id, region_id)`.
type RegionIdent = (String, u64);

/// Generate a rectilinear (horizontal-zigzag, snake) ironing path over a
/// 2D bounding box at z. Returns a single connected `ExtrusionPath3D`
/// whose points trace the snake — so callers see one path per region
/// covering all strokes. Two points per stroke (start + end), with
/// alternating direction.
fn generate_zigzag_strokes(
    bb: &BBox2D,
    z: f32,
    ironing_spacing_mm: f64,
    flow_factor: f32,
    speed_factor: f32,
) -> Vec<ExtrusionPath3D> {
    let spacing = ironing_spacing_mm as f32;
    if spacing <= 0.0 || bb.x_max <= bb.x_min || bb.y_max <= bb.y_min {
        return Vec::new();
    }

    let span = bb.y_max - bb.y_min;
    // Inclusive count: a 10mm span at 0.1mm spacing yields 101 lines
    // (y = y_min, y_min+0.1, ..., y_max). Guarantees >= width/spacing strokes.
    let n = ((span / spacing).floor() as usize).saturating_add(1);
    if n == 0 {
        return Vec::new();
    }

    let mk = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: IRONING_LINE_WIDTH,
        flow_factor,
        overhang_quartile: None,
    };

    // Build a single connected snake polyline: 2 points per stroke.
    let mut points: Vec<Point3WithWidth> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let y = (bb.y_min + (i as f32) * spacing).min(bb.y_max);
        let (x_start, x_end) = if i % 2 == 0 {
            (bb.x_min, bb.x_max)
        } else {
            (bb.x_max, bb.x_min)
        };
        points.push(mk(x_start, y));
        points.push(mk(x_end, y));
    }

    vec![ExtrusionPath3D {
        points,
        role: ExtrusionRole::Ironing,
        speed_factor,
    }]
}

/// Returns `true` if the entity carries `TopSolidInfill` (either on the
/// `PrintEntity::role` or on the inner path role).
fn is_top_solid_infill(entity: &PrintEntity) -> bool {
    matches!(entity.role, ExtrusionRole::TopSolidInfill)
        || matches!(entity.path.role, ExtrusionRole::TopSolidInfill)
}

#[slicer_module]
impl FinalizationModule for TopSurfaceIroning {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let ironing = match config.get("ironing") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let ironing_speed = match config.get("ironing_speed") {
            Some(ConfigValue::Float(s)) => *s,
            Some(ConfigValue::Int(s)) => *s as f64,
            _ => 20.0,
        };

        let ironing_flow = match config.get("ironing_flow") {
            Some(ConfigValue::Float(f)) => *f,
            Some(ConfigValue::Int(f)) => *f as f64,
            _ => 0.10,
        };

        if ironing_flow <= 0.0 {
            return Err(ModuleError::fatal(
                1,
                "ironing_flow must be greater than 0.0 (key: ironing_flow)",
            ));
        }

        let ironing_spacing = match config.get("ironing_spacing") {
            Some(ConfigValue::Float(s)) => *s,
            Some(ConfigValue::Int(s)) => *s as f64,
            _ => 0.1,
        };

        let ironing_pattern = match config.get("ironing_pattern") {
            Some(ConfigValue::String(p)) => {
                if p != "rectilinear" {
                    return Err(ModuleError::fatal(
                        2,
                        format!(
                            "unsupported ironing_pattern '{}'; only 'rectilinear' is supported \
                             (key: ironing_pattern)",
                            p
                        ),
                    ));
                }
                p.clone()
            }
            _ => "rectilinear".to_string(),
        };

        Ok(Self {
            ironing,
            ironing_speed,
            ironing_flow,
            ironing_spacing,
            ironing_pattern,
        })
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.ironing || layers.is_empty() {
            return Ok(());
        }

        // Build a map (object_id, region_id) -> highest layer index that
        // carries TopSolidInfill paths. Use a BTreeMap for deterministic
        // iteration order.
        let mut topmost: BTreeMap<RegionIdent, usize> = BTreeMap::new();
        for (idx, view) in layers.iter().enumerate() {
            for entity in view.ordered_entities() {
                if !is_top_solid_infill(entity) {
                    continue;
                }
                let key = (
                    entity.region_key.object_id.clone(),
                    entity.region_key.region_id,
                );
                topmost
                    .entry(key)
                    .and_modify(|cur| {
                        if idx > *cur {
                            *cur = idx;
                        }
                    })
                    .or_insert(idx);
            }
        }

        let speed_factor = (self.ironing_speed / BASE_SPEED) as f32;
        let flow_factor = self.ironing_flow as f32;

        for ((object_id, region_id), idx) in topmost {
            let view = &layers[idx];
            // Collect all TopSolidInfill points on this layer for this region.
            let mut bb: Option<BBox2D> = None;
            for entity in view.ordered_entities() {
                if !is_top_solid_infill(entity) {
                    continue;
                }
                if entity.region_key.object_id != object_id
                    || entity.region_key.region_id != region_id
                {
                    continue;
                }
                let region_bb = BBox2D::from_points(entity.path.points.iter());
                bb = match (bb, region_bb) {
                    (None, b) => b,
                    (Some(a), None) => Some(a),
                    (Some(a), Some(b)) => Some(BBox2D {
                        x_min: a.x_min.min(b.x_min),
                        y_min: a.y_min.min(b.y_min),
                        x_max: a.x_max.max(b.x_max),
                        y_max: a.y_max.max(b.y_max),
                    }),
                };
            }
            let bb = match bb {
                Some(b) => b,
                None => continue,
            };

            let z = view.z();
            let layer_index = view.layer_index();
            let strokes =
                generate_zigzag_strokes(&bb, z, self.ironing_spacing, flow_factor, speed_factor);
            for path in strokes {
                let region_key = RegionKey {
                    global_layer_index: layer_index,
                    object_id: object_id.clone(),
                    region_id,
                };
                output
                    .push_entity_with_priority(
                        layer_index,
                        path,
                        region_key,
                        ExtrusionRole::Ironing.default_priority(),
                    )
                    .map_err(|e| ModuleError::fatal(3, e))?;
            }
        }

        Ok(())
    }
}
