//! Marlin-style simplified trapezoid print-time estimator + per-tool extruded
//! volume accounting (packet 169, Step 2).
//!
//! Operates in mm (G-code space), NOT IR 100 nm units.
//!
//! E-semantics contract (FACT, mirrored from `serialize.rs`): `GCodeIR`
//! `Move.e` is **always an absolute E position** regardless of the stream's
//! `ExtrusionMode` command — `ExtrusionMode` only selects how the serializer
//! renders E (M83 deltas vs. M82 absolutes). `Retract`/`Unretract` carry
//! physical **delta** lengths and adjust the absolute-E accumulator
//! (retract subtracts, unretract adds), and a `Raw` "G92 E<val>" resets the
//! accumulator. The estimator reproduces exactly that accumulator logic.

use std::collections::BTreeMap;

use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR, ResolvedConfig};

/// Default filament diameter (mm) when a tool is absent from the caller's
/// diameter map (schema default, matches `DefaultGCodeSerializer`).
const DEFAULT_FILAMENT_DIAMETER_MM: f64 = 1.75;

/// Kinematic machine limits used by the estimator.
///
/// All speeds are mm/s, accelerations mm/s². Defaults are conservative
/// Marlin-ish fallbacks used when the machine config does not provide values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstimatorLimits {
    /// Max acceleration for extruding moves (mm/s²).
    pub max_acceleration: f32,
    /// Max acceleration for travel (non-extruding) moves (mm/s²).
    pub max_acceleration_travel: f32,
    /// Max X/Y axis speed (mm/s) — min of the per-axis X/Y limits.
    pub max_speed_xy: f32,
    /// Max Z axis speed (mm/s).
    pub max_speed_z: f32,
    /// Max E axis speed (mm/s).
    pub max_speed_e: f32,
    /// X/Y jerk (mm/s) — min of the per-axis X/Y jerks.
    pub jerk_xy: f32,
    /// Z jerk (mm/s).
    pub jerk_z: f32,
    /// E jerk (mm/s).
    pub jerk_e: f32,
}

impl Default for EstimatorLimits {
    fn default() -> Self {
        Self {
            max_acceleration: 1500.0,
            max_acceleration_travel: 1500.0,
            max_speed_xy: 200.0,
            max_speed_z: 12.0,
            max_speed_e: 25.0,
            jerk_xy: 9.0,
            jerk_z: 0.2,
            jerk_e: 2.5,
        }
    }
}

impl EstimatorLimits {
    /// Builds limits from a `ResolvedConfig`, falling back per-field to
    /// [`EstimatorLimits::default`] for any key the config leaves `None`.
    pub fn from_config(cfg: &ResolvedConfig) -> Self {
        let d = Self::default();
        Self {
            max_acceleration: cfg
                .machine_max_acceleration_extruding
                .unwrap_or(d.max_acceleration),
            max_acceleration_travel: cfg
                .machine_max_acceleration_travel
                .unwrap_or(d.max_acceleration_travel),
            max_speed_xy: cfg
                .machine_max_speed_x
                .unwrap_or(d.max_speed_xy)
                .min(cfg.machine_max_speed_y.unwrap_or(d.max_speed_xy)),
            max_speed_z: cfg.machine_max_speed_z.unwrap_or(d.max_speed_z),
            max_speed_e: cfg.machine_max_speed_e.unwrap_or(d.max_speed_e),
            jerk_xy: cfg
                .machine_max_jerk_x
                .unwrap_or(d.jerk_xy)
                .min(cfg.machine_max_jerk_y.unwrap_or(d.jerk_xy)),
            jerk_z: cfg.machine_max_jerk_z.unwrap_or(d.jerk_z),
            jerk_e: cfg.machine_max_jerk_e.unwrap_or(d.jerk_e),
        }
    }
}

/// Result of a print estimation pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrintEstimate {
    /// Total estimated print time in seconds.
    pub total_time_s: f64,
    /// Extruded volume per tool in mm³ (filament length × filament
    /// cross-section area).
    pub extruded_volume_mm3: BTreeMap<u32, f64>,
    /// Filament length consumed per tool in mm (positive E deltas of
    /// extruding moves only; retract/unretract excluded).
    pub filament_length_mm: BTreeMap<u32, f64>,
    /// Number of `ToolChange` commands seen.
    pub toolchange_count: u32,
}

/// Which axis class dominates a planned segment (selects speed cap + jerk).
#[derive(Debug, Clone, Copy, PartialEq)]
enum AxisClass {
    Xy,
    Z,
    E,
}

/// A planned kinematic segment (mm / mm/s space).
struct Segment {
    /// Segment length in mm.
    dist: f64,
    /// Target (programmed, axis-capped) cruise speed in mm/s.
    v_target: f64,
    /// Acceleration limit for this segment in mm/s².
    accel: f64,
    /// Jerk limit for this segment's axis class in mm/s.
    jerk: f64,
    /// XY unit direction (0,0 for non-XY segments).
    dir: (f64, f64),
    /// Axis class.
    axis: AxisClass,
}

/// Time to traverse `dist` with entry speed `ve`, exit speed `vx`, cruise cap
/// `vc`, and acceleration `a` — trapezoid profile, degrading to a triangle
/// when the segment is too short to reach `vc`.
fn segment_time(dist: f64, ve: f64, vx: f64, vc: f64, a: f64) -> f64 {
    if dist <= 0.0 {
        return 0.0;
    }
    let a = a.max(1e-6);
    let vc = vc.max(ve).max(vx).max(1e-6);
    let d_acc = (vc * vc - ve * ve) / (2.0 * a);
    let d_dec = (vc * vc - vx * vx) / (2.0 * a);
    if d_acc + d_dec <= dist {
        (vc - ve) / a + (vc - vx) / a + (dist - d_acc - d_dec) / vc
    } else {
        // Triangle profile: peak speed reachable within `dist`.
        let vp2 = (2.0 * a * dist + ve * ve + vx * vx) / 2.0;
        let vp = vp2.max(0.0).sqrt().max(ve).max(vx).max(1e-6);
        (vp - ve) / a + (vp - vx) / a
    }
}

/// Junction speed between two consecutive segments (Marlin-style simplified):
/// collinear XY segments keep `min(v1, v2)`; any direction / axis-class
/// change is jerk-limited to `min(jerk1, jerk2, v1, v2)`.
fn junction_speed(a: &Segment, b: &Segment) -> f64 {
    let v = a.v_target.min(b.v_target);
    if a.axis == AxisClass::Xy && b.axis == AxisClass::Xy {
        let cos = a.dir.0 * b.dir.0 + a.dir.1 * b.dir.1;
        if cos > 0.98 {
            return v;
        }
    }
    a.jerk.min(b.jerk).min(v)
}

struct CommandEstimate {
    elapsed_deltas_s: Vec<f64>,
    filament_length_mm: BTreeMap<u32, f64>,
    toolchange_count: u32,
}

fn estimate_command_deltas(gcode_ir: &GCodeIR, limits: &EstimatorLimits) -> CommandEstimate {
    let mut segments: Vec<(usize, Segment)> = Vec::new();
    let mut filament_length_mm: BTreeMap<u32, f64> = BTreeMap::new();
    let mut toolchange_count: u32 = 0;

    // Machine state.
    let mut pos: Option<(f64, f64, f64)> = None;
    let mut e_accumulator: f64 = 0.0; // absolute E, mirroring serialize.rs
    let mut feed_mm_s: f64 = 0.0; // sticky programmed feedrate (F is mm/min)
    let mut current_tool: u32 = 0;

    let max_xy = limits.max_speed_xy as f64;
    let max_z = limits.max_speed_z as f64;
    let max_e = limits.max_speed_e as f64;
    let accel_extrude = limits.max_acceleration as f64;
    let accel_travel = limits.max_acceleration_travel as f64;
    let jerk_xy = limits.jerk_xy as f64;
    let jerk_z = limits.jerk_z as f64;
    let jerk_e = limits.jerk_e as f64;

    for (command_index, command) in gcode_ir.commands.iter().enumerate() {
        match command {
            GCodeCommand::Move {
                x,
                y,
                z,
                e,
                f,
                role,
            } => {
                if let Some(f_val) = f {
                    feed_mm_s = (*f_val as f64 / 60.0).max(0.0);
                }
                let (px, py, pz) = pos.unwrap_or((
                    x.unwrap_or(0.0) as f64,
                    y.unwrap_or(0.0) as f64,
                    z.unwrap_or(0.0) as f64,
                ));
                let nx = x.map(|v| v as f64).unwrap_or(px);
                let ny = y.map(|v| v as f64).unwrap_or(py);
                let nz = z.map(|v| v as f64).unwrap_or(pz);
                let had_pos = pos.is_some();
                pos = Some((nx, ny, nz));

                // Extrusion accounting: Move.e is ALWAYS absolute (serialize.rs
                // contract). Only positive deltas count as extruded filament.
                let e_delta = if let Some(e_abs) = e {
                    let delta = *e_abs as f64 - e_accumulator;
                    e_accumulator = *e_abs as f64;
                    delta
                } else {
                    0.0
                };
                if e_delta > 0.0 {
                    *filament_length_mm.entry(current_tool).or_insert(0.0) += e_delta;
                }

                if !had_pos {
                    // First positioned move: establishes position, no travel.
                    continue;
                }

                let dx = nx - px;
                let dy = ny - py;
                let dz = nz - pz;
                let dist_xy = (dx * dx + dy * dy).sqrt();
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                let is_travel =
                    matches!(role, ExtrusionRole::Custom(s) if s == "Travel") || e_delta <= 0.0;
                let accel = if is_travel {
                    accel_travel
                } else {
                    accel_extrude
                };

                if dist > 1e-9 {
                    let (axis, cap, jerk, dir) = if dist_xy > 1e-9 {
                        (AxisClass::Xy, max_xy, jerk_xy, (dx / dist_xy, dy / dist_xy))
                    } else {
                        (AxisClass::Z, max_z, jerk_z, (0.0, 0.0))
                    };
                    let v = if feed_mm_s > 0.0 { feed_mm_s } else { cap };
                    segments.push((
                        command_index,
                        Segment {
                            dist,
                            v_target: v.min(cap).max(1e-6),
                            accel,
                            jerk,
                            dir,
                            axis,
                        },
                    ));
                } else if e_delta.abs() > 1e-9 {
                    // E-only move.
                    let v = if feed_mm_s > 0.0 { feed_mm_s } else { max_e };
                    segments.push((
                        command_index,
                        Segment {
                            dist: e_delta.abs(),
                            v_target: v.min(max_e).max(1e-6),
                            accel: accel_extrude,
                            jerk: jerk_e,
                            dir: (0.0, 0.0),
                            axis: AxisClass::E,
                        },
                    ));
                }
            }
            GCodeCommand::Retract { length, speed, .. } => {
                // Delta on the E axis; adjusts the absolute-E accumulator
                // (serialize.rs contract). Excluded from volume. `speed` is an
                // F value (mm/min).
                e_accumulator -= *length as f64;
                let v = (*speed as f64 / 60.0).max(1e-6).min(max_e);
                segments.push((
                    command_index,
                    Segment {
                        dist: (*length as f64).abs(),
                        v_target: v,
                        accel: accel_extrude,
                        jerk: jerk_e,
                        dir: (0.0, 0.0),
                        axis: AxisClass::E,
                    },
                ));
            }
            GCodeCommand::Unretract { length, speed, .. } => {
                e_accumulator += *length as f64;
                let v = (*speed as f64 / 60.0).max(1e-6).min(max_e);
                segments.push((
                    command_index,
                    Segment {
                        dist: (*length as f64).abs(),
                        v_target: v,
                        accel: accel_extrude,
                        jerk: jerk_e,
                        dir: (0.0, 0.0),
                        axis: AxisClass::E,
                    },
                ));
            }
            GCodeCommand::ToolChange { to, .. } => {
                toolchange_count += 1;
                current_tool = *to;
            }
            GCodeCommand::Raw { text } => {
                // Mirror serialize.rs: "G92 E<val>" resets the accumulator.
                let trimmed = text.trim();
                if trimmed.starts_with("G92") {
                    if let Some(e_str) = trimmed
                        .split_whitespace()
                        .find(|tok| tok.starts_with('E') || tok.starts_with('e'))
                    {
                        if let Ok(val) = e_str[1..].parse::<f64>() {
                            e_accumulator = val;
                        }
                    }
                }
            }
            // ExtrusionMode changes serialization only — Move.e stays absolute.
            GCodeCommand::ExtrusionMode { .. }
            | GCodeCommand::Comment { .. }
            | GCodeCommand::FanSpeed { .. }
            | GCodeCommand::Temperature { .. } => {}
        }
    }

    // Plan junction speeds and integrate time.
    let mut elapsed_deltas_s = vec![0.0; gcode_ir.commands.len()];
    for (i, (command_index, seg)) in segments.iter().enumerate() {
        let entry = if i == 0 {
            seg.jerk.min(seg.v_target)
        } else {
            junction_speed(&segments[i - 1].1, seg)
        };
        let exit = if i + 1 < segments.len() {
            junction_speed(seg, &segments[i + 1].1)
        } else {
            seg.jerk.min(seg.v_target)
        };
        // Clamp entry/exit to what is physically reachable within `dist`.
        let v_reach = |from: f64| (from * from + 2.0 * seg.accel * seg.dist).sqrt();
        let exit = exit.min(v_reach(entry));
        let entry = entry.min(v_reach(exit));
        elapsed_deltas_s[*command_index] +=
            segment_time(seg.dist, entry, exit, seg.v_target, seg.accel);
    }

    CommandEstimate {
        elapsed_deltas_s,
        filament_length_mm,
        toolchange_count,
    }
}

fn print_estimate(
    command_estimate: CommandEstimate,
    tool_diameters: &BTreeMap<u32, f32>,
) -> (PrintEstimate, Vec<f64>) {
    let total_time_s = command_estimate.elapsed_deltas_s.iter().sum();
    let extruded_volume_mm3 = command_estimate
        .filament_length_mm
        .iter()
        .map(|(&tool, &len)| {
            let d = tool_diameters
                .get(&tool)
                .map(|d| *d as f64)
                .unwrap_or(DEFAULT_FILAMENT_DIAMETER_MM);
            let area = std::f64::consts::PI * (d / 2.0) * (d / 2.0);
            (tool, len * area)
        })
        .collect();
    let mut elapsed_s = Vec::with_capacity(command_estimate.elapsed_deltas_s.len());
    let mut cumulative_s = 0.0;
    for delta_s in command_estimate.elapsed_deltas_s {
        cumulative_s += delta_s;
        elapsed_s.push(cumulative_s);
    }

    (
        PrintEstimate {
            total_time_s,
            extruded_volume_mm3,
            filament_length_mm: command_estimate.filament_length_mm,
            toolchange_count: command_estimate.toolchange_count,
        },
        elapsed_s,
    )
}

/// Estimates print time and per-tool extruded volume for a `GCodeIR` stream.
///
/// One forward walk over `gcode_ir.commands` with a Marlin-style simplified
/// trapezoid per segment. `tool_diameters` maps tool index → filament
/// diameter in mm (missing tools default to 1.75 mm).
pub fn estimate_print(
    gcode_ir: &GCodeIR,
    limits: &EstimatorLimits,
    tool_diameters: &BTreeMap<u32, f32>,
) -> PrintEstimate {
    estimate_print_with_elapsed(gcode_ir, limits, tool_diameters).0
}

/// Estimates print time and cumulative elapsed time after every G-code command.
pub fn estimate_print_with_elapsed(
    gcode_ir: &GCodeIR,
    limits: &EstimatorLimits,
    tool_diameters: &BTreeMap<u32, f32>,
) -> (PrintEstimate, Vec<f64>) {
    print_estimate(estimate_command_deltas(gcode_ir, limits), tool_diameters)
}
