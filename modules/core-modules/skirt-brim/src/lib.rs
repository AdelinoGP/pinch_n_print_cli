// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Brim.cpp / Print.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Skirt and brim module for priming and adhesion paths.
//!
//! Runs in the `PostPass::LayerFinalization` stage, operating on the full
//! set of `LayerCollectionIR` outputs. Generates skirt loops around the
//! print for priming and/or brim adhesion paths at the base.
//!
//! Geometry is the canonical family at the pinned OrcaSlicer `v2.5.0`
//! (`40ecc66f78`): the brim is a contour-anchored concentric ring
//! (`make_brim` / `tryExPolygonOffset`, `Brim.cpp`) and the skirt is anchored
//! to the convex hull of the occupied outline, brim included
//! (`Print::_make_skirt`, `Print.cpp`).

#![warn(missing_docs)]
#![warn(unused_imports)]

mod geometry;

use std::collections::HashMap;

use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::OffsetJoinType;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// One generated bed-adhesion entity, addressed by its position in the input
/// layer slice so both the legacy `process()` path and `run_finalization` can
/// place it without duplicating the generator.
struct GeneratedEntity {
    layer_pos: usize,
    path: ExtrusionPath3D,
    region_key: RegionKey,
}

/// Read-only view of one layer, identical for the owned IR and the WIT view.
#[derive(Clone, Copy)]
struct LayerSnapshot<'a> {
    layer_index: u32,
    z: f32,
    entities: &'a [PrintEntity],
}

/// Skirt and brim path generator.
///
/// Generates a convex-hull-anchored skirt around the print for extruder
/// priming, and optionally a contour-anchored concentric brim for bed
/// adhesion.
pub struct SkirtBrim {
    skirt_loops: u32,
    skirt_distance: f32,
    skirt_height: u32,
    brim_width: f32,
    brim_object_gap: f32,
    min_skirt_length: f32,
    line_width: f32,
    /// First-layer height in mm, used for the canonical flow spacing. `0.0`
    /// when neither `first_layer_height` nor `layer_height` is readable, which
    /// falls back to `line_width` as the spacing.
    first_layer_height: f32,
    enabled: bool,
}

impl SkirtBrim {
    /// Construct from a config view, reading skirt/brim settings.
    ///
    /// Required reads (packet 06 fail-closed semantics; mirrors
    /// arachne-perimeters): every key is declared in skirt-brim.toml with a
    /// registry default, and the registry seeds those defaults into a bound
    /// view — absence is a contract violation, not a configurable fallback.
    pub fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = config.require_bool("skirt_brim_enabled")?;

        let skirt_loops = config.require_int("skirt_loops")? as u32;

        let skirt_distance = config.require_float("skirt_distance")? as f32;

        let skirt_height = config.require_int("skirt_height")? as u32;

        let brim_width = config.require_float("brim_width")? as f32;

        let brim_object_gap = config.require_float("brim_object_gap")? as f32;

        let min_skirt_length = config.require_float("min_skirt_length")? as f32;

        let line_width = config.require_float("line_width")? as f32;

        // Canonical flow spacing needs the first-layer height. Both keys are
        // global (typed `ResolvedConfig` fields, not module-declared), so the
        // read is optional and falls back to `layer_height`, then to a plain
        // `line_width` spacing when neither is present.
        let first_layer_height = config
            .get_float("first_layer_height")
            .or_else(|| config.get_float("layer_height"))
            .unwrap_or(0.0) as f32;

        if min_skirt_length > 0.0 {
            // Pinned semantics: the port has no filament-flow model, so a
            // non-zero minimum cannot be honoured faithfully. Fail closed
            // rather than silently print fewer loops than the config asks for.
            return Err(ModuleError::fatal(
                2,
                "skirt-brim: min_skirt_length > 0 is unsupported by this port \
                 (the module has no filament cross-section model); author 0 or \
                 raise skirt_loops explicitly",
            ));
        }

        Ok(Self {
            skirt_loops,
            skirt_distance,
            skirt_height,
            brim_width,
            brim_object_gap,
            min_skirt_length,
            line_width,
            first_layer_height,
            enabled,
        })
    }

    /// Process all layers, inserting skirt and/or brim paths.
    ///
    /// If the module is disabled, returns immediately without modification.
    #[allow(clippy::ptr_arg)]
    pub fn process(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<(), ModuleError> {
        let snapshots: Vec<LayerSnapshot<'_>> = layers
            .iter()
            .map(|layer| LayerSnapshot {
                layer_index: layer.global_layer_index,
                z: layer.z,
                entities: layer.ordered_entities.as_slice(),
            })
            .collect();
        let generated = self.generate(&snapshots);
        if generated.is_empty() {
            return Ok(());
        }

        let mut additions: Vec<Vec<(ExtrusionPath3D, RegionKey)>> = vec![Vec::new(); layers.len()];
        for entity in generated {
            if let Some(slot) = additions.get_mut(entity.layer_pos) {
                slot.push((entity.path, entity.region_key));
            }
        }

        for (layer, mut add) in layers.iter_mut().zip(additions) {
            if add.is_empty() {
                continue;
            }
            // TODO(packet-41): retire this legacy `process()` path; live
            // path is `run_finalization` which routes through the builder API.
            let mut new_entities: Vec<PrintEntity> = add
                .drain(..)
                .map(|(path, region_key)| {
                    let role = path.role.clone();
                    PrintEntity {
                        entity_id: 0,
                        path,
                        role,
                        // Skirt/brim print with the base tool (T0). region_id is a
                        // pure region identity post region_id↔tool split — never the
                        // tool selector (D-125-TOOL-IDENTITY-SPLIT invariant).
                        tool_index: 0,
                        region_key,
                        topo_order: 0,
                    }
                })
                .collect();
            new_entities.append(&mut layer.ordered_entities);
            layer.ordered_entities = new_entities;
        }

        Ok(())
    }

    /// Generates every bed-adhesion entity for the given layer set.
    ///
    /// Mirrors the canonical split: the brim is built from the first layer's
    /// outer-wall footprint, one concentric ring per line width, and the skirt
    /// is built from the convex hull of the occupied outline up to
    /// `skirt_height`, with the brim footprint folded in (`Print::_make_skirt`
    /// groups brim areas into the skirt hull).
    fn generate(&self, layers: &[LayerSnapshot<'_>]) -> Vec<GeneratedEntity> {
        if !self.enabled || layers.is_empty() {
            return Vec::new();
        }

        let spacing = self.flow_spacing();
        let max_layer = (self.skirt_height as usize).min(layers.len());
        let first = layers[0];
        // Skirt entities are emitted before brim entities: canonical prints the
        // priming skirt first, then the adhesion brim, then the part.
        let mut brim_entities: Vec<GeneratedEntity> = Vec::new();
        let mut generated: Vec<GeneratedEntity> = Vec::new();

        // ---- Brim: contour-anchored concentric ring on layer 0 only --------
        let mut brim_points: Vec<(f32, f32)> = Vec::new();
        if self.brim_width > 0.0 {
            let mut object_order: Vec<String> = Vec::new();
            let mut footprints: HashMap<String, Vec<ExPolygon>> = HashMap::new();
            // Per-object outer-wall point widths. Canonical `make_brim` anchors
            // on the *slice contour*; the IR carries the outer-wall
            // **centerline**, half a wall width inside it. The contour is
            // reconstructed by offsetting the filled footprint outward by half
            // the wall width (the fourth recorded parity exception, t140/t144).
            let mut anchor_widths: HashMap<String, Vec<f32>> = HashMap::new();
            let mut brim_region_key: Option<RegionKey> = None;

            for entity in first.entities {
                if matches!(entity.role, ExtrusionRole::Skirt | ExtrusionRole::Brim) {
                    continue;
                }
                if entity.role != ExtrusionRole::OuterWall {
                    continue;
                }
                let points: Vec<(f32, f32)> =
                    entity.path.points.iter().map(|p| (p.x, p.y)).collect();
                let Some(poly) = geometry::closed_loop_to_ex_polygon(&points) else {
                    continue;
                };
                let key = entity.region_key.object_id.clone();
                if !footprints.contains_key(&key) {
                    object_order.push(key.clone());
                    footprints.insert(key.clone(), Vec::new());
                }
                if let Some(slot) = footprints.get_mut(&key) {
                    slot.push(poly);
                }
                anchor_widths
                    .entry(key.clone())
                    .or_default()
                    .extend(entity.path.points.iter().map(|p| p.width));
                if brim_region_key.is_none() {
                    brim_region_key = Some(RegionKey {
                        global_layer_index: first.layer_index,
                        // Brim is a distinct `ExtrusionRole::Brim` (packet-?, R1):
                        // the `__brim__` object_id marker is dropped, the role is the
                        // single source of truth for g-code labeling and host routing.
                        object_id: "brim".to_string(),
                        region_id: 0,
                        variant_chain: Vec::new(),
                    });
                }
            }

            // `make_brim`: brim_width is quantised to an even number of
            // extrusion spacings (`floor(brim_width / spacing / 2) * spacing * 2`)
            // so the ring closes on a whole number of lines.
            let quantised = ((self.brim_width / spacing) / 2.0).floor() * spacing * 2.0;
            let region_key = brim_region_key.unwrap_or(RegionKey {
                global_layer_index: first.layer_index,
                object_id: "brim".to_string(),
                region_id: 0,
                variant_chain: Vec::new(),
            });

            if quantised > 0.0 {
                // `no_brim_area_support`: canonical subtracts the first layer's
                // support footprint from every brim area, so the brim never
                // prints through a support pad that reaches the bed. Closed
                // support contours contribute their filled footprint; open
                // infill polylines contribute one bead-width strip per segment
                // (`polygons_covered_by_spacing()`).
                let mut support_polys: Vec<ExPolygon> = Vec::new();
                for entity in first.entities.iter().filter(|entity| {
                    matches!(
                        entity.role,
                        ExtrusionRole::SupportMaterial
                            | ExtrusionRole::SupportInterface
                            | ExtrusionRole::SupportBaseInterface
                    )
                }) {
                    let points: Vec<(f32, f32)> =
                        entity.path.points.iter().map(|p| (p.x, p.y)).collect();
                    if points.len() >= 4 {
                        let first_pt = points[0];
                        let last_pt = *points.last().expect("non-empty");
                        let closed = (first_pt.0 - last_pt.0).abs() < 1e-4
                            && (first_pt.1 - last_pt.1).abs() < 1e-4;
                        if closed {
                            if let Some(poly) = geometry::closed_loop_to_ex_polygon(&points) {
                                support_polys.push(poly);
                                continue;
                            }
                        }
                    }
                    support_polys.extend(geometry::polyline_strips(&points, self.line_width));
                }
                let support_area: Vec<ExPolygon> = {
                    let beads = geometry::union(support_polys);
                    // Canonical subtracts the support *slice contour*
                    // (`ex_poly.contour` for tree supports), which the IR does
                    // not carry: it carries beads. Merge each island's beads
                    // with a closing whose radius bridges the inter-bead gap,
                    // then fill the island so the brim is clipped at the
                    // support boundary rather than threading between lines.
                    let merged = geometry::closing(
                        &beads,
                        self.support_area_closing_mm(),
                        geometry::BRIM_ARC_TOLERANCE_MM,
                    );
                    merged
                        .iter()
                        .map(|ex| ExPolygon {
                            contour: ex.contour.clone(),
                            holes: Vec::new(),
                        })
                        .collect()
                };

                for object_id in object_order {
                    let loops = footprints.remove(&object_id).unwrap_or_default();
                    let footprint = geometry::union(loops);
                    if footprint.is_empty() {
                        continue;
                    }
                    // Canonical `outer_only` brims the *filled* outline: it
                    // offsets `ex_poly.contour` alone, so the object's own
                    // holes are filled and the ring never grows into them
                    // (`has_inner_brim` is false for this type).
                    let filled: Vec<ExPolygon> = footprint
                        .iter()
                        .map(|ex| ExPolygon {
                            contour: ex.contour.clone(),
                            holes: Vec::new(),
                        })
                        .collect();
                    // Reconstruct the slice contour `make_brim` anchors on:
                    // the filled wall-centerline footprint grown by half the
                    // resolved outer-wall width. The uniform form (per-object
                    // modal width) is the t140-settled approximation on
                    // Arachne, whose wall width varies per point.
                    let anchor_width = anchor_widths
                        .get(&object_id)
                        .and_then(|widths| geometry::modal_non_zero_width(widths))
                        .unwrap_or(self.line_width);
                    let contour = geometry::offset(
                        &filled,
                        0.5 * anchor_width,
                        OffsetJoinType::Round,
                        geometry::BRIM_ARC_TOLERANCE_MM,
                    );
                    // Canonical `make_brim` builds the brim *area* (an annulus
                    // between the gap-offset outline and the width-offset
                    // outline), subtracts the no-brim areas, and only then
                    // fills it with concentric lines (`tryExPolygonOffset`).
                    // Filling the area — rather than clipping individual
                    // ring lines — is what carries a support notch inward
                    // through every successive line.
                    let inner = geometry::offset(
                        &contour,
                        self.brim_object_gap,
                        OffsetJoinType::Round,
                        geometry::BRIM_ARC_TOLERANCE_MM,
                    );
                    let outer = geometry::offset(
                        &contour,
                        self.brim_object_gap + quantised,
                        OffsetJoinType::Round,
                        geometry::BRIM_ARC_TOLERANCE_MM,
                    );
                    let mut area = slicer_sdk::host::clip_polygons(
                        &outer,
                        &inner,
                        slicer_sdk::host::ClipOperation::Difference,
                    );
                    if !support_area.is_empty() {
                        area = slicer_sdk::host::clip_polygons(
                            &area,
                            &support_area,
                            slicer_sdk::host::ClipOperation::Difference,
                        );
                    }
                    if area.is_empty() {
                        continue;
                    }

                    // The skirt hull's brim input is the **brim area's outer
                    // contour**, not the emitted beads' centreline: canonical
                    // `Print::_make_skirt` appends
                    // `m_objectBrimAreasByInstance[...]` `area.contour.points`
                    // (holes excluded) to the group's `occupied_points`, and
                    // `make_brim` fills that map with the brim areas after the
                    // no-brim subtraction. Folding the centreline instead sat
                    // half a flow spacing (0.1786 mm at the matched job) inside
                    // canonical's hull input (t146 measured it, t151 fixes it).
                    // The contour is already in hand here; it is never
                    // re-derived from the beads.
                    for ex in &area {
                        brim_points.extend(ex.contour.points.iter().map(slicer_ir::Point2::to_mm));
                    }

                    // `tryExPolygonOffset`: the first line sits half a spacing
                    // inside the outer boundary; each successive pass steps one
                    // full spacing inward. Every pass emits both the outer and
                    // the hole boundary of the shrunken annulus, interleaving
                    // to a one-spacing line pitch.
                    let mut depth = spacing * 0.5;
                    loop {
                        let band = geometry::offset(
                            &area,
                            -depth,
                            OffsetJoinType::Round,
                            geometry::BRIM_ARC_TOLERANCE_MM,
                        );
                        if band.is_empty() {
                            break;
                        }
                        for ex in &band {
                            let mut polygons = Vec::with_capacity(1 + ex.holes.len());
                            polygons.push(&ex.contour);
                            polygons.extend(ex.holes.iter());
                            for polygon in polygons {
                                let Some(path) = self.contour_path(
                                    polygon,
                                    first.z,
                                    ExtrusionRole::Brim,
                                    self.line_width,
                                ) else {
                                    continue;
                                };
                                brim_entities.push(GeneratedEntity {
                                    layer_pos: 0,
                                    path,
                                    region_key: region_key.clone(),
                                });
                            }
                        }
                        depth += spacing;
                    }
                }
            }
        }

        // ---- Skirt: convex-hull anchored, `skirt_height` layers -----------
        if self.skirt_loops > 0 && max_layer > 0 {
            let mut hull_points: Vec<(f32, f32)> = Vec::new();
            for (layer_pos, layer) in layers.iter().take(max_layer).enumerate() {
                let mut layer_points: Vec<(f32, f32)> = Vec::new();
                let mut layer_widths: Vec<f32> = Vec::new();
                for entity in layer.entities {
                    if matches!(entity.role, ExtrusionRole::Skirt | ExtrusionRole::Brim) {
                        continue;
                    }
                    layer_points.extend(entity.path.points.iter().map(|p| (p.x, p.y)));
                    if entity.role == ExtrusionRole::OuterWall {
                        layer_widths.extend(entity.path.points.iter().map(|p| p.width));
                    }
                }
                // Layer 0 is dominated by the brim footprint (`brim_points`
                // joins the hull below) and needs no separate contour offset.
                // Higher layers carry only wall centerlines, so each layer's
                // hull is reconstructed by half the outer-wall width before it
                // joins the final hull: the convex hull commutes with a
                // Minkowski sum, so the hull of the offset contours is the
                // offset of the hull.
                if layer_pos == 0 {
                    hull_points.extend(layer_points);
                    continue;
                }
                let layer_hull = geometry::convex_hull(&layer_points);
                if layer_hull.len() < 3 {
                    continue;
                }
                let layer_poly = ExPolygon {
                    contour: slicer_ir::Polygon {
                        points: layer_hull
                            .iter()
                            .map(|&(x, y)| slicer_ir::Point2::from_mm(x, y))
                            .collect(),
                    },
                    holes: Vec::new(),
                };
                let anchor_width =
                    geometry::modal_non_zero_width(&layer_widths).unwrap_or(self.line_width);
                let contour = geometry::offset(
                    std::slice::from_ref(&layer_poly),
                    0.5 * anchor_width,
                    OffsetJoinType::Round,
                    geometry::BRIM_ARC_TOLERANCE_MM,
                );
                for ex in &contour {
                    hull_points.extend(ex.contour.points.iter().map(slicer_ir::Point2::to_mm));
                }
            }
            hull_points.extend(brim_points);

            let hull = geometry::convex_hull(&hull_points);
            if hull.len() >= 3 {
                let hull_poly = ExPolygon {
                    contour: slicer_ir::Polygon {
                        points: hull
                            .iter()
                            .map(|&(x, y)| slicer_ir::Point2::from_mm(x, y))
                            .collect(),
                    },
                    holes: Vec::new(),
                };
                // Outermost first: `Print::_make_skirt` generates inward to
                // outward and its callers reverse before G-code export.
                let mut loops: Vec<Vec<(f32, f32)>> =
                    Vec::with_capacity(usize::try_from(self.skirt_loops).unwrap_or(0));
                for i in (0..self.skirt_loops).rev() {
                    let distance = self.skirt_distance + spacing * 0.5 + (i as f32) * spacing;
                    let offset = geometry::offset(
                        std::slice::from_ref(&hull_poly),
                        distance,
                        OffsetJoinType::Round,
                        geometry::SKIRT_ARC_TOLERANCE_MM,
                    );
                    let Some(outer) = offset
                        .iter()
                        .max_by(|a, b| {
                            geometry::ring_area_mm2(&a.contour)
                                .partial_cmp(&geometry::ring_area_mm2(&b.contour))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|ex| ex.contour.clone())
                    else {
                        continue;
                    };
                    loops.push(outer.points.iter().map(slicer_ir::Point2::to_mm).collect());
                }

                for (layer_pos, layer) in layers.iter().take(max_layer).enumerate() {
                    let region_key = RegionKey {
                        global_layer_index: layer.layer_index,
                        object_id: "__skirt__".to_string(),
                        region_id: 0,
                        variant_chain: Vec::new(),
                    };
                    for loop_points in &loops {
                        let contour = slicer_ir::Polygon {
                            points: loop_points
                                .iter()
                                .map(|&(x, y)| slicer_ir::Point2::from_mm(x, y))
                                .collect(),
                        };
                        let Some(path) =
                            self.contour_path(&contour, layer.z, ExtrusionRole::Skirt, spacing)
                        else {
                            continue;
                        };
                        generated.push(GeneratedEntity {
                            layer_pos,
                            path,
                            region_key: region_key.clone(),
                        });
                    }
                }
            }
        }

        generated.extend(brim_entities);
        generated
    }

    /// Canonical flow spacing: `width - height x (1 - pi/4)`
    /// (`Flow::rounded_rectangle_extrusion_spacing`, `Flow.cpp`).
    ///
    /// The brim's ring width is quantised in these units and the concentric
    /// lines step by one spacing, exactly as `make_brim` / `tryExPolygonOffset`
    /// do. Falls back to `line_width` when no first-layer height is readable,
    /// and never returns less than half the line width (a spacing below that is
    /// a degenerate flow).
    fn flow_spacing(&self) -> f32 {
        let spacing =
            self.line_width - self.first_layer_height * (1.0 - std::f32::consts::FRAC_PI_4);
        if spacing.is_finite() && spacing > self.line_width * 0.5 {
            spacing.max(1e-4)
        } else {
            self.line_width.max(1e-4)
        }
    }

    /// Radius of the morphological closing used to turn support beads into
    /// their island footprint, in mm.
    ///
    /// Canonical clips the brim against the support *slice contour*; the IR
    /// carries bead centerlines only, so the beads are closed by a radius that
    /// bridges the inter-bead gap. `2.5 x line_width` (1.0 mm at the matched
    /// job) is the declared approximation; it is a parity tolerance, not a
    /// physical constant.
    fn support_area_closing_mm(&self) -> f32 {
        self.line_width * 2.5
    }

    /// Builds a closed extrusion loop from a polygon contour.
    fn contour_path(
        &self,
        contour: &slicer_ir::Polygon,
        z: f32,
        role: ExtrusionRole,
        width: f32,
    ) -> Option<ExtrusionPath3D> {
        if contour.points.len() < 3 {
            return None;
        }
        let mk = |x: f32, y: f32| Point3WithWidth {
            x,
            y,
            z,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        };
        let mut points: Vec<Point3WithWidth> = contour
            .points
            .iter()
            .map(|p| {
                let (x, y) = p.to_mm();
                mk(x, y)
            })
            .collect();
        points.push(points[0]);
        Some(ExtrusionPath3D {
            points,
            role,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        })
    }

    /// Whether the module is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Number of skirt loops.
    pub fn skirt_loops(&self) -> u32 {
        self.skirt_loops
    }

    /// Skirt distance from print in mm.
    pub fn skirt_distance(&self) -> f32 {
        self.skirt_distance
    }

    /// Number of layers to apply skirt to.
    pub fn skirt_height(&self) -> u32 {
        self.skirt_height
    }

    /// Brim width in mm (0 = disabled).
    pub fn brim_width(&self) -> f32 {
        self.brim_width
    }

    /// Gap between the object outline and the brim in mm.
    pub fn brim_object_gap(&self) -> f32 {
        self.brim_object_gap
    }

    /// Minimum skirt filament length in mm (pinned to 0 by this port).
    pub fn min_skirt_length(&self) -> f32 {
        self.min_skirt_length
    }

    /// Line width in mm.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

// `from_config` delegates to `from_config`; `run_finalization` is
// fully implemented via `LayerCollectionView` + `FinalizationOutputBuilder`
// (packet 16, TASK-142). The legacy `process()` helper remains for tests
// but is no longer called on the live host path.
#[slicer_module]
impl FinalizationModule for SkirtBrim {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        Self::from_config(config)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled || layers.is_empty() {
            return Ok(());
        }
        let mut needed_layers = 0usize;
        if self.brim_width > 0.0 {
            needed_layers = needed_layers.max(1);
        }
        if self.skirt_loops > 0 && self.skirt_height > 0 {
            needed_layers = needed_layers.max(self.skirt_height as usize);
        }
        if needed_layers == 0 {
            return Ok(());
        }
        let needed_layers = needed_layers.min(layers.len());

        let snapshots: Vec<LayerSnapshot<'_>> = layers
            .iter()
            .take(needed_layers)
            .map(|view| LayerSnapshot {
                layer_index: view.layer_index(),
                z: view.z(),
                entities: view.ordered_entities(),
            })
            .collect();

        for entity in self.generate(&snapshots) {
            let Some(view) = layers.get(entity.layer_pos) else {
                continue;
            };
            // Skirt/brim print with the base tool (T0); region_id stays a pure
            // identity and is never read as the tool (D-125 invariant).
            let tool_index = 0u32;
            output
                .push_entity_to_layer(
                    view.layer_index(),
                    entity.path,
                    tool_index,
                    entity.region_key,
                )
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        Ok(())
    }
}
