//! G-code emission: `LayerCollectionIR` → `GCodeIR`.
//!
//! This module hosts the [`GCodeEmitter`] trait and the canonical
//! [`DefaultGCodeEmitter`] implementation extracted from
//! `crates/slicer-runtime/src/gcode_emit.rs` (packet 86).
//!
//! Emit behavior (from docs/02_ir_schemas.md, docs/04_host_scheduler.md):
//! - Walk `LayerCollectionIR` in Z-sorted order (already sorted by LayerFinalization)
//! - Convert `PrintEntity.path` (ExtrusionPath3D) → `GCodeCommand::Move`
//! - Insert `GCodeCommand::ToolChange` where `ToolChange` appears
//! - Insert Z-hop travel moves where `ZHop` appears
//! - Build `PrintMetadata` (estimated time, filament used, layer count, slicer version)
//!
//! OrcaSlicer references:
//! - OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp — G-code emission patterns
//! - OrcaSlicerDocumented/tests/fff_print/test_gcodewriter.cpp — test patterns

use std::collections::HashMap;

use slicer_helpers::{drop_short_segments_mm, simplify_polyline_mm};
use slicer_ir::FeedrateConfig;
use slicer_ir::{
    ExtrusionRole, GCodeCommand, GCodeIR, LayerAnnotationKind, LayerCollectionIR, PrintMetadata,
    ResolvedConfig, TravelMove,
};

use crate::error::GCodeEmitError;
use crate::serialize::{format_xyz, tolerance_for_role};

/// Trait for GCode emission (host-built-in).
///
/// Implementations consume a `&[LayerCollectionIR]` and produce a `GCodeIR`.
/// The blackboard is intentionally *not* part of this trait: the empirical
/// audit performed in packet 86 / Step 1 confirmed the default implementation
/// never reads from it.
pub trait GCodeEmitter {
    /// Emit GCode IR from layer collections.
    fn emit_gcode(&self, layer_irs: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError>;

    /// Resolved travel feedrate in mm/min for finalization-aware travel inserts.
    /// `None` means the caller should fall back to whatever resolution path it owns.
    fn travel_feedrate_mm_per_min(&self) -> Option<f32> {
        None
    }
}

/// Default GCode emitter (host-built-in).
///
/// Converts `LayerCollectionIR` to `GCodeIR` by walking layers in Z-sorted order,
/// converting print entities to move commands, and inserting tool changes and Z-hops.
pub struct DefaultGCodeEmitter {
    /// Slicer version string to include in metadata.
    slicer_version: String,
    /// Feedrate configuration.
    feedrate_config: FeedrateConfig,
    /// Decimal places for XYZ coordinate values in emitted GCode comments (default 3).
    gcode_xy_decimals: u32,
    /// Resolved slicer config for precision / simplification parameters.
    resolved_config: ResolvedConfig,
    /// `true` = relative-E mode (M83); `false` = absolute-E mode (M82).
    /// Must match the `DefaultGCodeSerializer`'s extrusion-mode setting.
    relative: bool,
}

impl DefaultGCodeEmitter {
    /// Creates a new `DefaultGCodeEmitter` with the given slicer version.
    ///
    /// Defaults to relative-E mode (M83) to match `DefaultGCodeSerializer::new()`.
    pub fn new(slicer_version: String) -> Self {
        Self {
            slicer_version,
            feedrate_config: FeedrateConfig::default(),
            gcode_xy_decimals: 3,
            resolved_config: ResolvedConfig::default(),
            relative: true,
        }
    }

    /// Creates a new `DefaultGCodeEmitter` with explicit configuration.
    pub fn new_with_config(slicer_version: String, feedrate_config: FeedrateConfig) -> Self {
        Self {
            slicer_version,
            feedrate_config,
            gcode_xy_decimals: 3,
            resolved_config: ResolvedConfig::default(),
            relative: true,
        }
    }

    /// Sets the extrusion mode.
    ///
    /// - `relative = true`  → emits `M83` (relative-E); must match the serializer.
    /// - `relative = false` → emits `M82` (absolute-E); must match the serializer.
    pub fn with_extrusion_mode(mut self, relative: bool) -> Self {
        self.relative = relative;
        self
    }

    /// Sets the resolved slicer config (precision / simplification parameters).
    pub fn with_resolved_config(mut self, cfg: ResolvedConfig) -> Self {
        self.resolved_config = cfg;
        self
    }

    /// Resolves the feedrate (in mm/min) for a given extrusion role, speed factor multiplier,
    /// and optional overhang quartile.
    pub fn resolve_feedrate(
        &self,
        role: &ExtrusionRole,
        speed_factor: f32,
        overhang_quartile: Option<u8>,
    ) -> Option<f32> {
        // Overhang speed dispatch for wall-family roles
        if let Some(q) = overhang_quartile {
            if matches!(
                role,
                ExtrusionRole::OuterWall | ExtrusionRole::InnerWall | ExtrusionRole::ThinWall
            ) {
                let speed = match q {
                    1 => self.feedrate_config.overhang_1_4_speed,
                    2 => self.feedrate_config.overhang_2_4_speed,
                    3 => self.feedrate_config.overhang_3_4_speed,
                    4 => self.feedrate_config.overhang_4_4_speed,
                    _ => 0.0,
                };
                if speed > 0.0 {
                    let clamped = speed_factor.clamp(0.05, 5.0);
                    return Some(speed * 60.0 * clamped);
                }
            }
        }

        let base_speed = match role {
            ExtrusionRole::OuterWall => self.feedrate_config.outer_wall_speed,
            ExtrusionRole::InnerWall => self.feedrate_config.inner_wall_speed,
            ExtrusionRole::ThinWall => self.feedrate_config.thin_wall_speed,
            ExtrusionRole::TopSolidInfill => self.feedrate_config.top_surface_speed,
            ExtrusionRole::BottomSolidInfill => self.feedrate_config.bottom_surface_speed,
            ExtrusionRole::SparseInfill => self.feedrate_config.sparse_infill_speed,
            ExtrusionRole::BridgeInfill => self.feedrate_config.bridge_speed,
            ExtrusionRole::SupportMaterial => self.feedrate_config.support_speed,
            ExtrusionRole::SupportInterface => self.feedrate_config.support_interface_speed,
            ExtrusionRole::Skirt => self.feedrate_config.skirt_speed,
            ExtrusionRole::WipeTower => self.feedrate_config.wipe_tower_speed,
            ExtrusionRole::PrimeTower => self.feedrate_config.prime_tower_speed,
            ExtrusionRole::Ironing => {
                if self.feedrate_config.filament_ironing_speed > 0.0 {
                    self.feedrate_config.filament_ironing_speed
                } else {
                    self.feedrate_config.ironing_speed
                }
            }
            ExtrusionRole::Custom(s) => match s.as_str() {
                "Travel" => self.feedrate_config.travel_speed,
                "Wipe" => self.feedrate_config.wipe_speed,
                "GapInfill" => self.feedrate_config.gap_infill_speed,
                "InternalBridge" => self.feedrate_config.internal_bridge_speed,
                _ => self.feedrate_config.outer_wall_speed,
            },
        };

        let clamped_factor = speed_factor.clamp(0.05, 5.0);
        let f_value = base_speed * 60.0 * clamped_factor;
        let rounded = (f_value * 1000.0).round() / 1000.0;
        Some(rounded)
    }

    /// Returns the slicer version string.
    pub fn slicer_version(&self) -> &str {
        &self.slicer_version
    }
}

/// Returns true if two extrusion roles are functionally equal for ;TYPE: labeling.
fn role_equals(a: &ExtrusionRole, b: &ExtrusionRole) -> bool {
    match (a, b) {
        (ExtrusionRole::OuterWall, ExtrusionRole::OuterWall) => true,
        (ExtrusionRole::InnerWall, ExtrusionRole::InnerWall) => true,
        (ExtrusionRole::ThinWall, ExtrusionRole::ThinWall) => true,
        (ExtrusionRole::TopSolidInfill, ExtrusionRole::TopSolidInfill) => true,
        (ExtrusionRole::BottomSolidInfill, ExtrusionRole::BottomSolidInfill) => true,
        (ExtrusionRole::SparseInfill, ExtrusionRole::SparseInfill) => true,
        (ExtrusionRole::BridgeInfill, ExtrusionRole::BridgeInfill) => true,
        (ExtrusionRole::SupportMaterial, ExtrusionRole::SupportMaterial) => true,
        (ExtrusionRole::SupportInterface, ExtrusionRole::SupportInterface) => true,
        (ExtrusionRole::Skirt, ExtrusionRole::Skirt) => true,
        (ExtrusionRole::WipeTower, ExtrusionRole::WipeTower) => true,
        (ExtrusionRole::PrimeTower, ExtrusionRole::PrimeTower) => true,
        (ExtrusionRole::Ironing, ExtrusionRole::Ironing) => true,
        (ExtrusionRole::Custom(a_str), ExtrusionRole::Custom(b_str)) => a_str == b_str,
        _ => false,
    }
}

/// Returns the canonical OrcaSlicer ";TYPE:{label}" comment text for an extrusion role.
fn orca_type_label(role: &ExtrusionRole) -> &'static str {
    match role {
        ExtrusionRole::OuterWall => ";TYPE:Outer wall",
        ExtrusionRole::InnerWall => ";TYPE:Inner wall",
        ExtrusionRole::ThinWall => ";TYPE:Inner wall",
        ExtrusionRole::TopSolidInfill => ";TYPE:Top surface",
        ExtrusionRole::BottomSolidInfill => ";TYPE:Bottom surface",
        ExtrusionRole::SparseInfill => ";TYPE:Sparse infill",
        ExtrusionRole::BridgeInfill => ";TYPE:Bridge infill",
        ExtrusionRole::SupportMaterial => ";TYPE:Support",
        ExtrusionRole::SupportInterface => ";TYPE:Support interface",
        ExtrusionRole::Skirt => ";TYPE:Skirt/Brim",
        ExtrusionRole::WipeTower => ";TYPE:Prime tower",
        ExtrusionRole::PrimeTower => ";TYPE:Prime tower",
        ExtrusionRole::Ironing => ";TYPE:Ironing",
        ExtrusionRole::Custom(_) => ";TYPE:Custom",
    }
}

impl GCodeEmitter for DefaultGCodeEmitter {
    fn emit_gcode(&self, layer_irs: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError> {
        // Clone for mutation (overhang classification + tool rotation).
        let mut owned_layers: Vec<LayerCollectionIR> = layer_irs.to_vec();
        // Apply cross-layer tool rotation before classification and emission.
        // This rotates each layer's entity clusters so the first cluster's tool
        // matches the previous layer's ending tool, avoiding redundant T<n>
        // commands at every layer boundary. The WASM module always orders
        // clusters in ascending tool order (H1: module state doesn't survive
        // across calls), so the host performs the rotation post-dispatch.
        apply_cross_layer_tool_rotation(&mut owned_layers);
        slicer_core::algos::overhang_classifier::classify_layers(
            &mut owned_layers,
            &self.feedrate_config,
        );
        let layer_irs: &[LayerCollectionIR] = &owned_layers;

        let layer_count = layer_irs.len() as u32;

        // Push ExtrusionMode as index-0 so postpass modules (Step 4) can prepend
        // machine_start_gcode BEFORE it via commands.insert(0, Raw(...)).
        let mut commands = vec![GCodeCommand::ExtrusionMode {
            absolute: !self.relative,
        }];
        // Track filament used per tool (tool index -> filament mm)
        let mut filament_per_tool: HashMap<u32, f32> = HashMap::new();
        // Current tool (default 0)
        let mut current_tool: u32 = 0;
        // Cumulative E position
        let mut e_position: f32 = 0.0;

        // Previous layer Z for computing ;HEIGHT: delta
        let mut prev_layer_z: Option<f32> = None;
        // Track the last non-zero height delta (for first-layer fallback)
        let mut last_height_delta: f32 = 0.2;
        // Previous role for ;TYPE: emission
        let mut prev_role: Option<ExtrusionRole> = None;

        // Walk layers in order (already Z-sorted by LayerFinalization)
        for layer in layer_irs {
            let layer_z = layer.z;

            // Emit Orca layer-change headers BEFORE the first Move of this layer
            // Insert ;LAYER_CHANGE, ;Z:{z}, ;HEIGHT:{h} before the first command
            // Note: push bare text; serializer adds "; " prefix for regular comments.
            // Orca header lines are output via Raw so they go through verbatim.
            let height_delta = if let Some(prev_z) = prev_layer_z {
                layer_z - prev_z
            } else {
                last_height_delta
            };
            if prev_layer_z.is_some() {
                last_height_delta = height_delta;
            }
            prev_layer_z = Some(layer_z);

            commands.push(GCodeCommand::Raw {
                text: ";LAYER_CHANGE".to_string(),
            });
            commands.push(GCodeCommand::Raw {
                text: format!(";Z:{}", format_xyz(layer_z, self.gcode_xy_decimals)),
            });
            commands.push(GCodeCommand::Raw {
                text: format!(
                    ";HEIGHT:{}",
                    format_xyz(height_delta, self.gcode_xy_decimals)
                ),
            });

            // Cross-layer tool reset: path-optimization-default only records
            // intra-layer tool transitions, so layer N+1's first cluster
            // inherits whatever tool layer N ended on. Without this reset,
            // unpainted (T0) body extrusions are silently emitted under the
            // last painted tool of the previous layer. By host convention,
            // each ordered entity's `region_key.region_id` is its required
            // tool index (see layer_executor::assemble_ordered_entities and
            // path-optimization-default::tool_index_of). Emit a tool change
            // before the first entity whenever it differs from `current_tool`.
            if let Some(first_entity) = layer.ordered_entities.first() {
                let required_tool = first_entity.region_key.region_id as u32;
                if required_tool != current_tool {
                    // Synthesize the pre-T<n> retract for layer-boundary tool
                    // changes too (the entity-loop synth at the bottom of this
                    // function only handles intra-layer tool changes recorded
                    // in `layer.tool_changes`; the layer-boundary one here is
                    // emitted via a separate path). See packet 58 / DEV-054
                    // follow-up (i).
                    if self.resolved_config.wipe_tower_enabled {
                        commands.push(GCodeCommand::Retract {
                            length: self.resolved_config.retract_length,
                            speed: 2400.0,
                            mode: slicer_ir::RetractMode::Gcode,
                        });
                    }
                    commands.push(GCodeCommand::ToolChange {
                        after_entity_index: u32::MAX,
                        from: current_tool,
                        to: required_tool,
                    });
                    current_tool = required_tool;
                }
            }

            // Build lookup maps for tool_changes and z_hops by after_entity_index.
            // Value is (tc_index_in_layer, &ToolChange) so the guard can report
            // the 0-based tool_change_index for GCodeEmitError::MissingToolchangePurge.
            let tool_changes: HashMap<u32, (u32, &_)> = layer
                .tool_changes
                .iter()
                .enumerate()
                .map(|(i, tc)| (tc.after_entity_index, (i as u32, tc)))
                .collect();
            let z_hops: HashMap<u32, &_> = layer
                .z_hops
                .iter()
                .map(|zh| (zh.after_entity_index, zh))
                .collect();
            // retracts: per entity index, collect all in order (Retract entries first, Unretract entries last)
            let mut retracts_by_entity: std::collections::HashMap<
                u32,
                Vec<&slicer_ir::TravelRetract>,
            > = std::collections::HashMap::new();
            for r in &layer.retracts {
                retracts_by_entity
                    .entry(r.after_entity_index)
                    .or_default()
                    .push(r);
            }
            // travel_moves: per entity_id, collect all in order
            let mut travel_moves_by_entity: std::collections::HashMap<
                u64,
                Vec<&slicer_ir::TravelMove>,
            > = std::collections::HashMap::new();
            for tm in &layer.travel_moves {
                travel_moves_by_entity
                    .entry(tm.entity_id)
                    .or_default()
                    .push(tm);
            }

            // Process each entity
            for (entity_idx, entity) in layer.ordered_entities.iter().enumerate() {
                let entity_idx = entity_idx as u32;
                let points = &entity.path.points;
                let role = &entity.path.role;

                // Emit ;TYPE: comment when role changes from previous entity
                let role_changed = prev_role
                    .as_ref()
                    .is_none_or(|prev| !role_equals(prev, role));
                if role_changed {
                    commands.push(GCodeCommand::Raw {
                        text: orca_type_label(role).to_string(),
                    });
                }
                prev_role = Some(role.clone());

                // Apply per-role polyline simplification (D-P + min-segment).
                // Extract XY mm-pairs for the helpers, then remap kept indices
                // back onto the original Point3WithWidth slice so metadata is preserved.
                let tol = tolerance_for_role(role, &self.resolved_config);
                let is_travel = matches!(role, ExtrusionRole::Custom(s) if s == "Travel");
                let simplified_points: Vec<&slicer_ir::Point3WithWidth> = if points.len() >= 2 {
                    let xy: Vec<(f32, f32)> = points.iter().map(|p| (p.x, p.y)).collect();
                    let simplified_xy = if tol > 0.0 {
                        simplify_polyline_mm(&xy, tol)
                    } else {
                        xy.clone()
                    };
                    let pruned_xy = if self.resolved_config.min_segment_length > 0.0 && !is_travel {
                        drop_short_segments_mm(
                            &simplified_xy,
                            self.resolved_config.min_segment_length,
                        )
                    } else {
                        simplified_xy
                    };
                    // Map kept (x,y) pairs back to original point indices.
                    // Both slices are in emission order; match on coordinate identity.
                    let mut kept = Vec::with_capacity(pruned_xy.len());
                    let mut search_from = 0usize;
                    for (kx, ky) in &pruned_xy {
                        for i in search_from..points.len() {
                            if (points[i].x - kx).abs() < f32::EPSILON
                                && (points[i].y - ky).abs() < f32::EPSILON
                            {
                                kept.push(&points[i]);
                                search_from = i + 1;
                                break;
                            }
                        }
                    }
                    kept
                } else {
                    points.iter().collect()
                };

                // Emit Move commands for each point in the path
                let mut prev_point: Option<&slicer_ir::Point3WithWidth> = None;
                for point in simplified_points {
                    // Calculate extrusion (E) based on travel distance and width
                    let e_delta = if let Some(prev) = prev_point {
                        // Calculate 3D distance
                        let dx = point.x - prev.x;
                        let dy = point.y - prev.y;
                        let dz = point.z - prev.z;
                        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                        // E = distance * width * flow_factor (simplified)
                        distance * point.width * point.flow_factor
                    } else {
                        0.0 // First point, no extrusion
                    };

                    e_position += e_delta;
                    *filament_per_tool.entry(current_tool).or_insert(0.0) += e_delta;

                    commands.push(GCodeCommand::Move {
                        x: Some(point.x),
                        y: Some(point.y),
                        z: Some(point.z),
                        // Emit E for any non-zero delta. Negative deltas (retracts) were
                        // previously dropped, which made wipe-tower's `generate_purge_paths`
                        // retract entity invisible in the live gcode stream.
                        e: if e_delta != 0.0 {
                            Some(e_position)
                        } else {
                            None
                        },
                        f: self.resolve_feedrate(
                            role,
                            entity.path.speed_factor,
                            point.overhang_quartile,
                        ),
                        role: role.clone(),
                    });

                    prev_point = Some(point);
                }

                // Emit canonical retract/z-hop/travel/unretract sequence after this entity,
                // BEFORE any ToolChange. OrcaSlicer ordering: retract → z-hop → travel →
                // T<n> → (unretract handled by next-entity start or wipe-tower prime).
                let entity_retracts = retracts_by_entity.get(&entity_idx);
                let entity_travels = travel_moves_by_entity.get(&entity.entity_id);
                let entity_z_hop = z_hops.get(&entity_idx);

                // 1. Retracts (before tool change and travel)
                if let Some(retracts) = entity_retracts {
                    for r in retracts.iter().filter(|r| !r.is_unretract) {
                        commands.push(GCodeCommand::Retract {
                            length: r.length,
                            speed: r.speed,
                            mode: r.mode,
                        });
                    }
                }
                // 2. Z-hop up (before travel)
                if let Some(zh) = entity_z_hop {
                    let hop_z = layer_z + zh.hop_height;
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(hop_z),
                        e: None,
                        f: self.resolve_feedrate(
                            &ExtrusionRole::Custom("Travel".to_string()),
                            1.0,
                            None,
                        ),
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                }
                // 3. Travel moves (before tool change)
                if let Some(travels) = entity_travels {
                    for tm in travels.iter() {
                        debug_assert!(
                            tm.entity_id == entity.entity_id,
                            "dangling travel anchor: entity_id={}",
                            tm.entity_id
                        );
                        commands.push(GCodeCommand::Move {
                            x: tm.x,
                            y: tm.y,
                            z: None,
                            e: None,
                            f: tm.f.or_else(|| {
                                self.resolve_feedrate(
                                    &ExtrusionRole::Custom("Travel".to_string()),
                                    1.0,
                                    None,
                                )
                            }),
                            role: ExtrusionRole::Custom("Travel".to_string()),
                        });
                    }
                }

                // 4. Check for tool change after this entity (AFTER retract/travel)
                if let Some((tc_index, tc)) = tool_changes.get(&entity_idx) {
                    // Defensive guard: verify that at least one ExtrusionRole::WipeTower
                    // entity follows each ToolChange in the layer. If not, the wipe-tower
                    // module has not emitted purge geometry, and the bare ToolChange would
                    // produce bad output (NC1).
                    //
                    // Guard is ONLY active when wipe_tower_enabled=true. Single-material
                    // prints (wipe_tower_enabled=false, the default) skip the check entirely.
                    if self.resolved_config.wipe_tower_enabled {
                        // Heuristic: verify the wipe-tower module inserted at least one
                        // ExtrusionRole::WipeTower entity in this layer. After cross-layer
                        // tool rotation (apply_cross_layer_tool_rotation), tool_change
                        // positions may shift relative to WipeTower insertions, so this
                        // guard uses a layer-scoped check rather than the stricter
                        // per-tool_change position check.
                        let has_wipe_in_layer = layer
                            .ordered_entities
                            .iter()
                            .any(|e| matches!(e.role, ExtrusionRole::WipeTower));

                        if !has_wipe_in_layer {
                            return Err(GCodeEmitError::MissingToolchangePurge {
                                layer_index: layer.global_layer_index,
                                tool_change_index: *tc_index,
                            });
                        }

                        // Synthesize the pre-T<n> retract. The wipe-tower module's
                        // retract entity is inserted at `after_entity_index + 1` and
                        // therefore serializes AFTER T<n> — but physical correctness
                        // requires the retract BEFORE T<n> so the unloading filament
                        // doesn't smear on travel. Until the SDK exposes a way to
                        // push a TravelRetract from a finalization-stage module, the
                        // host emitter owns this retract synthesis. Gated on
                        // wipe_tower_enabled so single-material prints are untouched.
                        // (See packet 58 / DEV-054 follow-up (i).)
                        commands.push(GCodeCommand::Retract {
                            length: self.resolved_config.retract_length,
                            speed: 2400.0,
                            mode: slicer_ir::RetractMode::Gcode,
                        });
                    }

                    commands.push(GCodeCommand::ToolChange {
                        after_entity_index: tc.after_entity_index,
                        from: tc.from_tool,
                        to: tc.to_tool,
                    });
                    current_tool = tc.to_tool;
                }

                // 5. Emit Comment/Raw annotations attached to this entity index,
                // in the deterministic order they appear in `annotations`.
                for ann in layer
                    .annotations
                    .iter()
                    .filter(|a| a.after_entity_index == entity_idx)
                {
                    match &ann.kind {
                        LayerAnnotationKind::Comment(text) => {
                            commands.push(GCodeCommand::Comment { text: text.clone() });
                        }
                        LayerAnnotationKind::Raw(text) => {
                            commands.push(GCodeCommand::Raw { text: text.clone() });
                        }
                    }
                }

                // 6. Z-hop down (after tool change)
                if let Some(zh) = entity_z_hop {
                    commands.push(GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(layer_z),
                        e: None,
                        f: self.resolve_feedrate(
                            &ExtrusionRole::Custom("Travel".to_string()),
                            1.0,
                            None,
                        ),
                        role: ExtrusionRole::Custom("Travel".to_string()),
                    });
                    let _ = zh;
                }
                // 7. Unretracts (after tool change, before next entity)
                if let Some(retracts) = entity_retracts {
                    for r in retracts.iter().filter(|r| r.is_unretract) {
                        commands.push(GCodeCommand::Unretract {
                            length: r.length,
                            speed: r.speed,
                            mode: r.mode,
                        });
                    }
                }
            }

            // Trailing annotations whose anchor lies past the last entity
            // (e.g. layer with no ordered_entities) are still emitted in
            // declaration order so guest-emitted comments/raw lines are not
            // silently dropped.
            let entity_count = layer.ordered_entities.len() as u32;
            for ann in layer
                .annotations
                .iter()
                .filter(|a| a.after_entity_index >= entity_count)
            {
                match &ann.kind {
                    LayerAnnotationKind::Comment(text) => {
                        commands.push(GCodeCommand::Comment { text: text.clone() });
                    }
                    LayerAnnotationKind::Raw(text) => {
                        commands.push(GCodeCommand::Raw { text: text.clone() });
                    }
                }
            }
        }

        // Build filament_used_mm vector (indexed by tool)
        let max_tool = filament_per_tool.keys().max().copied().unwrap_or(0);
        let mut filament_used_mm = vec![0.0; (max_tool + 1) as usize];
        for (tool, amount) in filament_per_tool {
            filament_used_mm[tool as usize] = amount;
        }
        // Ensure at least one entry
        if filament_used_mm.is_empty() {
            filament_used_mm.push(0.0);
        }

        Ok(GCodeIR {
            commands,
            metadata: PrintMetadata {
                estimated_print_time_s: 0, // Not calculated in this implementation
                filament_used_mm,
                layer_count,
                slicer_version: self.slicer_version.clone(),
            },
            ..Default::default()
        })
    }

    fn travel_feedrate_mm_per_min(&self) -> Option<f32> {
        Some(self.feedrate_config.travel_speed * 60.0)
    }
}

/// Reconcile travel moves to route through finalization geometry (Skirt/Brim,
/// WipeTower) without modifying `ordered_entities`.
///
/// This pass runs on each `LayerCollectionIR` *before* `emit_gcode` so that
/// travel transitions correctly incorporate finalization geometry.
///
/// Invariants:
/// - `ordered_entities` is never modified.
/// - Only `travel_moves` is mutated (new entries appended).
/// - If no Skirt or WipeTower entities exist, the layer is unchanged (no-op).
pub fn reconcile_finalization_travel(
    layer: &mut LayerCollectionIR,
    travel_f_mm_per_min: Option<f32>,
) {
    let entities = &layer.ordered_entities;

    // Collect indices of finalization entities
    let skirt_indices: Vec<usize> = entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.role == ExtrusionRole::Skirt)
        .map(|(i, _)| i)
        .collect();
    let wipe_indices: Vec<usize> = entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.role == ExtrusionRole::WipeTower)
        .map(|(i, _)| i)
        .collect();

    if skirt_indices.is_empty() && wipe_indices.is_empty() {
        return; // no-op
    }

    // Find the first model (non-finalization) entity index
    let first_model = entities.iter().enumerate().find_map(|(i, e)| {
        if e.role != ExtrusionRole::Skirt && e.role != ExtrusionRole::WipeTower {
            Some(i)
        } else {
            None
        }
    });

    // AC1: If skirt entities exist before model entities, add a travel move
    // from the last skirt entity's endpoint to the first model entity's start.
    if let (Some(&last_skirt_idx), Some(model_idx)) = (skirt_indices.last(), first_model) {
        if last_skirt_idx < model_idx {
            let skirt_entity = &entities[last_skirt_idx];
            let model_entity = &entities[model_idx];
            if let (Some(_skirt_end), Some(model_start)) = (
                skirt_entity.path.points.last(),
                model_entity.path.points.first(),
            ) {
                layer.travel_moves.push(TravelMove {
                    entity_id: entities[last_skirt_idx].entity_id,
                    x: Some(model_start.x),
                    y: Some(model_start.y),
                    z: None,
                    f: travel_f_mm_per_min,
                });
            }
        }
    }

    // AC2: If wipe tower entities exist, add travel moves that route to the
    // wipe tower start from the preceding entity.
    for &wipe_idx in &wipe_indices {
        if wipe_idx > 0 {
            let wipe_entity = &entities[wipe_idx];
            if let Some(wipe_start) = wipe_entity.path.points.first() {
                layer.travel_moves.push(TravelMove {
                    entity_id: entities[wipe_idx - 1].entity_id,
                    x: Some(wipe_start.x),
                    y: Some(wipe_start.y),
                    z: None,
                    f: travel_f_mm_per_min,
                });
            }
        }
    }

    // Keep travel_moves sorted by anchored entity position for deterministic emission.
    let id_to_idx: std::collections::HashMap<u64, usize> = entities
        .iter()
        .enumerate()
        .map(|(i, e)| (e.entity_id, i))
        .collect();
    layer
        .travel_moves
        .sort_by_key(|tm| id_to_idx.get(&tm.entity_id).copied().unwrap_or(usize::MAX));
}

/// Apply cross-layer tool rotation to entity order.
///
/// The WASM path-optimization module always orders entity clusters in ascending
/// tool order (it cannot carry cross-layer state — see DEV-054 follow-up (iii),
/// H1). This function post-processes `LayerCollectionIR` entries in layer-index
/// order to rotate each layer's clusters so the first cluster's tool matches the
/// previous layer's ending tool. This avoids a redundant `T0` tool-change at the
/// start of nearly every layer when the previous layer ended on a non-zero tool.
///
/// Recomputes `tool_changes` from the new entity order and remaps all
/// index-anchored data (`z_hops`, `retracts`, `annotations`).
fn apply_cross_layer_tool_rotation(layers: &mut [LayerCollectionIR]) {
    if layers.is_empty() {
        return;
    }

    let mut prev_ending_tool: Option<u32> = None;

    for layer in layers.iter_mut() {
        let entities = &mut layer.ordered_entities;
        if entities.is_empty() {
            continue;
        }

        let first_tool = entities[0].region_key.region_id as u32;

        if let Some(prev_tool) = prev_ending_tool {
            if prev_tool != first_tool {
                if let Some(start) = entities
                    .iter()
                    .position(|e| e.region_key.region_id as u32 == prev_tool)
                {
                    let mut end = start;
                    while end < entities.len()
                        && entities[end].region_key.region_id as u32 == prev_tool
                    {
                        end += 1;
                    }

                    let n = entities.len();
                    let cluster_size = end - start;
                    let mut old_to_new: Vec<usize> = vec![0; n];

                    for i in start..end {
                        old_to_new[i] = i - start;
                    }
                    for i in 0..start {
                        old_to_new[i] = i + cluster_size;
                    }
                    for i in end..n {
                        old_to_new[i] = i;
                    }

                    let mut new_entities = Vec::with_capacity(n);
                    new_entities.extend(entities.drain(start..end));
                    new_entities.append(entities);
                    *entities = new_entities;

                    let mut new_tcs: Vec<slicer_ir::ToolChange> = Vec::new();
                    for i in 0..entities.len().saturating_sub(1) {
                        let tool_i = entities[i].region_key.region_id as u32;
                        let tool_next = entities[i + 1].region_key.region_id as u32;
                        if tool_i != tool_next {
                            new_tcs.push(slicer_ir::ToolChange {
                                after_entity_index: i as u32,
                                from_tool: tool_i,
                                to_tool: tool_next,
                            });
                        }
                    }
                    layer.tool_changes = new_tcs;

                    for zh in &mut layer.z_hops {
                        let old_idx = zh.after_entity_index as usize;
                        if let Some(&new_idx) = old_to_new.get(old_idx) {
                            zh.after_entity_index = new_idx as u32;
                        }
                    }
                    for r in &mut layer.retracts {
                        let old_idx = r.after_entity_index as usize;
                        if let Some(&new_idx) = old_to_new.get(old_idx) {
                            r.after_entity_index = new_idx as u32;
                        }
                    }
                    for ann in &mut layer.annotations {
                        let old_idx = ann.after_entity_index as usize;
                        if let Some(&new_idx) = old_to_new.get(old_idx) {
                            ann.after_entity_index = new_idx as u32;
                        }
                    }
                }
            }
        }

        prev_ending_tool = entities.last().map(|e| e.region_key.region_id as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gcode_emitter_stores_slicer_version() {
        let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
        assert_eq!(emitter.slicer_version(), "1.0.0-test");
    }

    // ── apply_cross_layer_tool_rotation regression tests (packet 58 / DEV-054 (iii)) ──
    //
    // The function rotates each layer's first cluster to match the previous
    // layer's ending tool, suppressing redundant T<n> emissions at layer
    // boundaries. These tests cover the contract directly (the function is
    // pure Rust; no WASM boundary).

    fn tool_entity(entity_id: u64, layer: u32, tool: u32) -> slicer_ir::PrintEntity {
        slicer_ir::PrintEntity {
            entity_id,
            path: slicer_ir::ExtrusionPath3D {
                points: vec![slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2 * layer as f32,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                }],
                role: slicer_ir::ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            role: slicer_ir::ExtrusionRole::OuterWall,
            region_key: slicer_ir::RegionKey {
                global_layer_index: layer,
                object_id: "obj".to_string(),
                region_id: tool as u64,
            },
            topo_order: 0,
        }
    }

    fn layer_with_tools(layer_index: u32, tools: &[u32]) -> slicer_ir::LayerCollectionIR {
        let entities: Vec<_> = tools
            .iter()
            .enumerate()
            .map(|(i, &t)| tool_entity((i + 1) as u64, layer_index, t))
            .collect();
        let mut tcs = Vec::new();
        for i in 0..entities.len().saturating_sub(1) {
            let from = entities[i].region_key.region_id as u32;
            let to = entities[i + 1].region_key.region_id as u32;
            if from != to {
                tcs.push(slicer_ir::ToolChange {
                    after_entity_index: i as u32,
                    from_tool: from,
                    to_tool: to,
                });
            }
        }
        slicer_ir::LayerCollectionIR {
            global_layer_index: layer_index,
            z: 0.2 * (layer_index + 1) as f32,
            ordered_entities: entities,
            tool_changes: tcs,
            ..Default::default()
        }
    }

    /// Empty layer list and single-layer input must not panic or rotate.
    #[test]
    fn apply_cross_layer_tool_rotation_handles_empty_and_singleton() {
        // Empty: no-op.
        let mut empty: Vec<slicer_ir::LayerCollectionIR> = Vec::new();
        apply_cross_layer_tool_rotation(&mut empty);
        assert!(empty.is_empty());

        // Singleton: nothing to rotate against; layer left intact.
        let original = layer_with_tools(0, &[0, 0, 1, 1]);
        let mut singleton = vec![original.clone()];
        apply_cross_layer_tool_rotation(&mut singleton);
        assert_eq!(
            singleton[0]
                .ordered_entities
                .iter()
                .map(|e| e.region_key.region_id)
                .collect::<Vec<_>>(),
            original
                .ordered_entities
                .iter()
                .map(|e| e.region_key.region_id)
                .collect::<Vec<_>>(),
            "single layer must not be rotated"
        );
        assert_eq!(
            singleton[0].tool_changes.len(),
            original.tool_changes.len(),
            "tool_changes must be preserved for singleton input"
        );
    }

    /// Golden case: layer 0 ends on tool 2; layer 1 starts in ascending order
    /// [0, 1, 2, 3]. The tool-2 cluster in layer 1 must rotate to the front,
    /// producing [2, 0, 1, 3], and tool_changes for layer 1 must be recomputed
    /// to reflect the new order (so the first transition is 2→0 at index 0,
    /// not the redundant 0-leads layer-boundary emission).
    #[test]
    fn apply_cross_layer_tool_rotation_rotates_matching_cluster_to_front() {
        // Layer 0 ends on tool 2.
        let layer0 = layer_with_tools(0, &[0, 0, 1, 2, 2]);
        // Layer 1 in ascending tool order — one entity per tool.
        let layer1 = layer_with_tools(1, &[0, 1, 2, 3]);
        let mut layers = vec![layer0, layer1];

        apply_cross_layer_tool_rotation(&mut layers);

        let new_tools: Vec<u64> = layers[1]
            .ordered_entities
            .iter()
            .map(|e| e.region_key.region_id)
            .collect();
        assert_eq!(
            new_tools,
            vec![2u64, 0, 1, 3],
            "layer 1's tool-2 cluster (single entity) must rotate to position 0"
        );

        // Recomputed tool_changes must reflect the new order: 2→0 at idx 0,
        // 0→1 at idx 1, 1→3 at idx 2.
        let new_tcs: Vec<(u32, u32, u32)> = layers[1]
            .tool_changes
            .iter()
            .map(|tc| (tc.after_entity_index, tc.from_tool, tc.to_tool))
            .collect();
        assert_eq!(new_tcs, vec![(0, 2, 0), (1, 0, 1), (2, 1, 3)]);
    }

    /// When layer N's ending tool already matches layer N+1's leading tool,
    /// no rotation occurs and the layer's data is identical to its input.
    #[test]
    fn apply_cross_layer_tool_rotation_noop_when_tool_already_matches() {
        let layer0 = layer_with_tools(0, &[0, 1, 1]); // ends on 1
        let layer1 = layer_with_tools(1, &[1, 2, 3]); // starts with 1
        let original_layer1 = layer1.clone();
        let mut layers = vec![layer0, layer1];

        apply_cross_layer_tool_rotation(&mut layers);

        let after: Vec<u64> = layers[1]
            .ordered_entities
            .iter()
            .map(|e| e.region_key.region_id)
            .collect();
        let before: Vec<u64> = original_layer1
            .ordered_entities
            .iter()
            .map(|e| e.region_key.region_id)
            .collect();
        assert_eq!(
            after, before,
            "matching boundary tool must not trigger rotation"
        );
        assert_eq!(layers[1].tool_changes, original_layer1.tool_changes);
    }

    /// Rotation must remap every positional anchor: z_hops, retracts, and
    /// annotations. Layer 1: [0, 1, 2] with a ZHop after index 0 and a
    /// TravelRetract after index 1; layer 0 ends on tool 2. After rotation
    /// layer 1 becomes [2, 0, 1]; old_to_new[0]=1, old_to_new[1]=2,
    /// old_to_new[2]=0; so the ZHop's anchor moves 0 → 1 and the retract's
    /// anchor moves 1 → 2.
    #[test]
    fn apply_cross_layer_tool_rotation_remaps_zhops_retracts_annotations() {
        let layer0 = layer_with_tools(0, &[2, 2]); // ends on tool 2
        let mut layer1 = layer_with_tools(1, &[0, 1, 2]);
        layer1.z_hops = vec![slicer_ir::ZHop {
            after_entity_index: 0,
            hop_height: 0.3,
        }];
        layer1.retracts = vec![slicer_ir::TravelRetract {
            after_entity_index: 1,
            length: 0.8,
            speed: 1800.0,
            ..Default::default()
        }];
        layer1.annotations = vec![slicer_ir::LayerAnnotation {
            after_entity_index: 2,
            kind: slicer_ir::LayerAnnotationKind::Comment("hello".to_string()),
        }];

        let mut layers = vec![layer0, layer1];
        apply_cross_layer_tool_rotation(&mut layers);

        let new_tools: Vec<u64> = layers[1]
            .ordered_entities
            .iter()
            .map(|e| e.region_key.region_id)
            .collect();
        assert_eq!(new_tools, vec![2u64, 0, 1]);

        assert_eq!(
            layers[1].z_hops[0].after_entity_index, 1,
            "ZHop anchor must remap from old 0 to new 1"
        );
        assert_eq!(
            layers[1].retracts[0].after_entity_index, 2,
            "TravelRetract anchor must remap from old 1 to new 2"
        );
        assert_eq!(
            layers[1].annotations[0].after_entity_index, 0,
            "Annotation anchor must remap from old 2 to new 0"
        );
    }
}
