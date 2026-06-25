//! TDD: G-code toolchange wrapping â€” retract â†’ travel â†’ prime â†’ wipe bracketing.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 2 scaffolding.
//! All three tests are expected to FAIL today (no production wiring yet).
//!
//! AC1: tool change is bracketed by retract â†’ travel â†’ `;TYPE:Prime tower` â†’ prime+wipe.
//! AC3: purge volume is within Â±20% of OrcaSlicer reference flush volume.
//! NC1: bare tool change (no surrounding purge entities) returns Err.

#![allow(missing_docs)]

use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, ObjectId, Point3WithWidth, PrintEntity,
    RegionKey, ResolvedConfig, RetractMode, SemVer, ToolChange, TravelRetract,
};

// â”€â”€ Fixtures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

/// Build a minimal print entity with the given tool (encoded in region_id).
fn make_entity(id: u64, x: f32, y: f32, role: ExtrusionRole, tool: u32) -> PrintEntity {
    PrintEntity {
        entity_id: id,
        path: ExtrusionPath3D {
            points: vec![pt(x, y, 0.2), pt(x + 1.0, y, 0.2)],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: tool,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("cube"),
            region_id: tool as u64,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

/// Build a wipe-tower entity for tool `tool` located at the tower corner.
/// The cumulative positive-E contribution of this entity is computed as:
///   E = distance_along_path * width * flow_factor
/// Two-point path of length 40 mm Ã— width 0.4 mm Ã— flow 1.0 = 16 mm
/// (tower X=180, width=60 â†’ scan-line from 180 to 240; height contributes too)
fn make_wipe_entity(id: u64, purge_len_mm: f32, tool: u32) -> PrintEntity {
    // Two-point entity whose path length equals purge_len_mm (to get
    // cumulative E â‰ˆ purge_len_mm * width * flow when serialized).
    // width=0.4, flow=1.0 â†’ E â‰ˆ purge_len_mm * 0.4
    // Caller controls purge_len_mm; for 70 mmÂ³ volume with 0.4 mmÂ² cross-section
    // length â‰ˆ 437 mm â€” keep it large enough to exercise the assertion.
    let x_end = purge_len_mm; // purely for path length
    PrintEntity {
        entity_id: id,
        path: ExtrusionPath3D {
            points: vec![pt(0.0, 0.0, 0.2), pt(x_end, 0.0, 0.2)],
            role: ExtrusionRole::WipeTower,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::WipeTower,
        tool_index: tool,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("cube"),
            region_id: tool as u64,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

/// Emit + serialize a single-layer GCode and return the output string.
fn emit_and_serialize(layer: LayerCollectionIR) -> Result<String, slicer_gcode::GCodeEmitError> {
    let emitter = DefaultGCodeEmitter::new("test".to_string());
    let ir = emitter.emit_gcode(&[layer])?;
    let serializer = DefaultGCodeSerializer::new();
    serializer.serialize_gcode(&ir)
}

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Return the indices of lines matching `needle` in the output text.
fn find_lines<'a>(text: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .filter(|(_i, l)| l.contains(needle))
        .collect()
}

/// True if `text` has a `G1 E-...` line (retract) before the first `T<n>` line.
fn has_retract_before_toolchange(text: &str) -> bool {
    let mut saw_retract = false;
    for line in text.lines() {
        // Relative-E retract: "G1 E-..."
        if line.starts_with("G1 E-") {
            saw_retract = true;
        }
        // Tool change token
        if line.starts_with('T') && line[1..].trim().parse::<u32>().is_ok() {
            return saw_retract;
        }
    }
    false
}

// â”€â”€ AC1 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC1: tool change must be bracketed in order:
///   (1) retract (negative-E G1) before T1
///   (2) G1 travel to tower XY before T1
///   (3) literal line `T1`
///   (4) `;TYPE:Prime tower` after T1 and before the next print-role extrusion
///   (5) cumulative positive-E â‰¥ wipe_tower_purge_volume mm after T1
///
/// FAILS TODAY: production code emits no retract, no travel-to-tower, no prime,
/// and emits `;TYPE:Wipe tower` (wrong spelling) instead of `;TYPE:Prime tower`.
#[test]
fn toolchange_emits_retract_prime_wipe() {
    // Layer: [entity0 (T0)] â†’ ToolChange(0â†’1) â†’ [wipe-tower entities (T1)]
    // wipe_tower_purge_volume = 70.0 mmÂ³; line_width=0.4, layer_height=0.2
    // required path length â‰ˆ 70 / (0.4 * 0.2) = 875 mm
    // Use 1000 mm to be safely above threshold.
    let purge_len_mm: f32 = 1000.0;

    let entity0 = make_entity(1, 5.0, 5.0, ExtrusionRole::OuterWall, 0);
    let wipe_entity = make_wipe_entity(2, purge_len_mm, 1);

    let layer = LayerCollectionIR {
        schema_version: SemVer::default(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity0, wipe_entity],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![TravelRetract {
            after_entity_index: 0,
            length: 0.8,
            speed: 45.0,
            is_unretract: false,
            mode: RetractMode::Gcode,
        }],
        travel_moves: vec![],
    };

    let gcode =
        emit_and_serialize(layer).expect("emit_and_serialize must not error for a valid layer");

    // (1) Retract (negative-E G1) must appear before T1.
    assert!(
        has_retract_before_toolchange(&gcode),
        "AC1 FAIL: expected a negative-E G1 retract before T1, but none found.\n\
         Relevant gcode excerpt:\n{}",
        gcode
            .lines()
            .filter(|l| l.starts_with("G1 E") || l.starts_with('T'))
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    );

    // (2) Travel to tower XY before T1.
    // Tower X default = 0.0 in fixture (no config override) â€” we check structural
    // presence of a travel G0/G1 without E.
    let t1_lines = find_lines(&gcode, "T1");
    assert!(
        !t1_lines.is_empty(),
        "AC1 FAIL: expected line 'T1' in gcode output, not found.\nGcode:\n{}",
        &gcode[..gcode.len().min(500)]
    );
    let (t1_line_idx, _) = t1_lines[0];
    let has_travel = gcode.lines().enumerate().take(t1_line_idx).any(|(_i, l)| {
        (l.starts_with("G0") || l.starts_with("G1"))
            && !l.contains('E')
            && l.contains('X')
            && l.contains('Y')
    });
    let pre_t1_lines: Vec<&str> = gcode.lines().take(t1_line_idx).collect();
    let show_start = pre_t1_lines.len().saturating_sub(15);
    assert!(
        has_travel,
        "AC1 FAIL: expected a G0/G1 XY travel move to tower before T1 (line {}), none found.\n\
         Lines before T1:\n{}",
        t1_line_idx,
        pre_t1_lines[show_start..].join("\n")
    );

    // (3) `;TYPE:Prime tower` must appear after T1.
    let prime_tower_lines = find_lines(&gcode, ";TYPE:Prime tower");
    assert!(
        !prime_tower_lines.is_empty(),
        "AC1 FAIL: expected ';TYPE:Prime tower' marker in gcode output, not found.\n\
         Note: current code emits ';TYPE:Wipe tower' (wrong spelling) â€” fix gcode_emit.rs."
    );
    let (prime_line_idx, _) = prime_tower_lines[0];
    assert!(
        prime_line_idx > t1_line_idx,
        "AC1 FAIL: ';TYPE:Prime tower' (line {}) must come AFTER T1 (line {})",
        prime_line_idx,
        t1_line_idx
    );

    // (4 + 5) Cumulative positive-E after T1 must be â‰¥ purge_volume (70.0 mm).
    // In relative mode each G1 E<delta> accumulates; sum all positive-E lines
    // after T1.
    let lines: Vec<&str> = gcode.lines().collect();
    let post_t1: &[&str] = &lines[t1_line_idx..];
    let cum_e: f32 = post_t1
        .iter()
        .filter(|l| l.starts_with("G1"))
        .filter_map(|l| {
            l.split_whitespace()
                .find(|t| t.starts_with('E'))
                .and_then(|t| t[1..].parse::<f32>().ok())
        })
        .filter(|v| *v > 0.0)
        .sum();

    let purge_volume_mm: f32 = 70.0; // wipe_tower_purge_volume default
    assert!(
        cum_e >= purge_volume_mm,
        "AC1 FAIL: cumulative positive-E after T1 ({:.3} mm) is less than wipe_tower_purge_volume \
         ({:.1} mm). Prime+wipe block is missing or undersized.",
        cum_e,
        purge_volume_mm
    );
}

// â”€â”€ AC3 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC3: purge volume parity Â±20% vs OrcaSlicer reference (0â†’1 = 70.0 mmÂ³).
///
/// Reference: OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:2258-2270
/// TODO(spec-58): reconcile hardcoded 70.0 against the WipeTower2.cpp flush_volumes_matrix
///               once the file has been reviewed at the acceptance ceremony.
///
/// FAILS TODAY: no production purge emission.
#[test]
fn purge_volume_within_tolerance() {
    // OrcaSlicer reference flush volume for (0â†’1): 70.0 mmÂ³
    let reference_volume_mm3: f32 = 70.0;
    let tolerance_lo = 0.80_f32;
    let tolerance_hi = 1.20_f32;

    // Fixture: single tool change 0â†’1 with a wipe entity sized to exactly the
    // reference volume. layer_height=0.2, line_width=0.4 â†’ path length = 70/(0.4*0.2) = 875 mm.
    let path_len_mm: f32 = reference_volume_mm3 / (0.4 * 0.2);
    let entity0 = make_entity(1, 5.0, 5.0, ExtrusionRole::OuterWall, 0);
    let wipe_entity = make_wipe_entity(2, path_len_mm, 1);

    let layer = LayerCollectionIR {
        schema_version: SemVer::default(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity0, wipe_entity],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![TravelRetract {
            after_entity_index: 0,
            length: 0.8,
            speed: 45.0,
            is_unretract: false,
            mode: RetractMode::Gcode,
        }],
        travel_moves: vec![],
    };

    let gcode = emit_and_serialize(layer).expect("emit_and_serialize must not error");

    // Locate T1 to bound the wipe block.
    let lines: Vec<&str> = gcode.lines().collect();
    let t1_idx = lines
        .iter()
        .position(|l| l.starts_with("T1"))
        .expect("AC3 FAIL: T1 not found in gcode");

    // Sum cumulative positive-E mm after T1 (relative mode: each G1 E<delta>).
    let cum_e_mm: f32 = lines[t1_idx..]
        .iter()
        .filter(|l| l.starts_with("G1"))
        .filter_map(|l| {
            l.split_whitespace()
                .find(|t| t.starts_with('E'))
                .and_then(|t| t[1..].parse::<f32>().ok())
        })
        .filter(|v| *v > 0.0)
        .sum();

    // Convert mm (E axis) to mmÂ³ volume: V = cum_e Ã— layer_height
    // cum_e already includes line_width via emitter's E = distance Ã— line_width Ã— flow_factor;
    // multiplying by line_width again would double-count it.
    let measured_volume_mm3 = cum_e_mm * 0.2;

    let lo = reference_volume_mm3 * tolerance_lo;
    let hi = reference_volume_mm3 * tolerance_hi;

    assert!(
        measured_volume_mm3 >= lo && measured_volume_mm3 <= hi,
        "AC3 FAIL: measured purge volume {:.2} mmÂ³ is outside [{:.1}, {:.1}] mmÂ³ \
         (Â±20% of OrcaSlicer reference {:.1} mmÂ³ for flush (0â†’1)).\n\
         Cumulative positive-E after T1: {:.3} mm.",
        measured_volume_mm3,
        lo,
        hi,
        reference_volume_mm3,
        cum_e_mm
    );
}

// â”€â”€ NC1 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// NC1: a bare tool change (no surrounding retract/wipe entities) under
/// `wipe_tower_enabled=true` must be rejected with an error that names
/// "MissingToolchangePurge".
///
/// STRATEGY: use format!("{:?}") inspection because PostpassError::MissingToolchangePurge
/// does not exist yet (Step 4 adds it). Once Step 4 lands, this test will pass.
///
/// FAILS TODAY: emit_gcode returns Ok for bare tool changes (no guard implemented yet).
#[test]
fn bare_toolchange_rejected() {
    // A layer with a ToolChange but NO surrounding retract/wipe entities.
    let entity0 = make_entity(1, 5.0, 5.0, ExtrusionRole::OuterWall, 0);
    let entity1 = make_entity(2, 6.0, 5.0, ExtrusionRole::OuterWall, 1);

    let layer = LayerCollectionIR {
        schema_version: SemVer::default(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity0, entity1],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![], // no retract
        travel_moves: vec![],
    };

    // Emitter must be configured with wipe_tower_enabled=true so the guard runs.
    // The guard is intentionally disabled for single-material (wipe_tower_enabled=false)
    // to avoid breaking pre-existing single-material emit tests.
    let cfg = ResolvedConfig {
        wipe_tower_enabled: true,
        ..ResolvedConfig::default()
    };
    let emitter = DefaultGCodeEmitter::new("test".to_string()).with_resolved_config(cfg);
    let result = emitter.emit_gcode(&[layer]);

    match result {
        Err(err) => {
            let debug_str = format!("{:?}", err);
            assert!(
                debug_str.contains("MissingToolchangePurge")
                    || debug_str.contains("missing_toolchange_purge")
                    || debug_str.contains("toolchange")
                    || debug_str.contains("purge"),
                "NC1 FAIL: expected error describing missing toolchange purge, got: {:?}",
                err
            );
            // Test passes once Step 4 lands the guard + variant.
        }
        Ok(_) => {
            panic!(
                "NC1 FAIL: expected emit_gcode to return Err for a bare ToolChange \
                 (no retract/wipe entities) with wipe_tower_enabled=true, but got Ok.\n\
                 Step 4 must add PostpassError::MissingToolchangePurge and the defensive guard."
            );
        }
    }
}
