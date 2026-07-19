#![allow(missing_docs)]

use std::collections::BTreeMap;

use slicer_gcode::{filament_stats_comment_block, inject_m73, PrintEstimate};
use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR};

fn move_command(x: f32, speed: f32) -> GCodeCommand {
    GCodeCommand::Move {
        x: Some(x),
        y: Some(0.0),
        z: None,
        e: None,
        f: Some(speed),
        role: ExtrusionRole::OuterWall,
    }
}

fn layer_change() -> GCodeCommand {
    GCodeCommand::Raw {
        text: ";LAYER_CHANGE".to_string(),
    }
}

fn test_ir() -> GCodeIR {
    GCodeIR {
        commands: vec![
            layer_change(),
            move_command(10.0, 1200.0),
            move_command(20.0, 1800.0),
            layer_change(),
            move_command(30.0, 2400.0),
            layer_change(),
            move_command(40.0, 900.0),
        ],
        ..Default::default()
    }
}

fn raw_lines(ir: &GCodeIR) -> Vec<&str> {
    ir.commands
        .iter()
        .filter_map(|command| match command {
            GCodeCommand::Raw { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn m73_lines(ir: &GCodeIR) -> Vec<&str> {
    raw_lines(ir)
        .into_iter()
        .filter(|line| line.starts_with("M73 "))
        .collect()
}

#[test]
fn layer_boundary_p_r_monotonic_first_last() {
    let mut ir = test_ir();
    inject_m73(&mut ir, &[0.0, 20.0, 45.0, 120.0, 210.0, 300.0, 600.0]);

    let lines = m73_lines(&ir);
    assert!(lines[0].starts_with("M73 P0 R"));
    assert_eq!(
        lines.iter().rev().find(|line| line.starts_with("M73 P")),
        Some(&"M73 P100 R0")
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("M73 P"))
            .count(),
        4
    );

    let percentages = lines
        .iter()
        .filter_map(|line| line.strip_prefix("M73 P"))
        .map(|line| line.split(' ').next().unwrap().parse::<u8>().unwrap())
        .collect::<Vec<_>>();
    assert!(percentages.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn stealth_q_s_mirrors_p_r() {
    let mut ir = test_ir();
    inject_m73(&mut ir, &[0.0, 20.0, 45.0, 120.0, 210.0, 300.0, 600.0]);

    let lines = m73_lines(&ir);
    for pair in lines.chunks_exact(2) {
        let p = pair[0].strip_prefix("M73 P").unwrap();
        let q = pair[1].strip_prefix("M73 Q").unwrap();
        let (pct, remaining) = p.split_once(" R").unwrap();
        assert_eq!(q, format!("{pct} S{remaining}"));
    }
}

fn two_tool_estimate() -> PrintEstimate {
    PrintEstimate {
        total_time_s: 3725.0,
        filament_length_mm: BTreeMap::from([(0, 1000.0), (1, 500.0)]),
        extruded_volume_mm3: BTreeMap::from([(0, 2405.28), (1, 1202.64)]),
        toolchange_count: 0,
    }
}

fn raw_texts(commands: &[GCodeCommand]) -> Vec<&str> {
    commands
        .iter()
        .map(|command| match command {
            GCodeCommand::Raw { text } => text.as_str(),
            _ => panic!("comment block must contain only Raw commands"),
        })
        .collect()
}

#[test]
fn filament_stats_block_two_tools_with_density() {
    let estimate = two_tool_estimate();
    let commands = filament_stats_comment_block(&estimate, Some(1.24));
    let lines = raw_texts(&commands);

    assert_eq!(
        lines,
        vec![
            "; filament used [mm] = 1000.00, 500.00",
            "; filament used [cm3] = 2.41, 1.20",
            "; filament used [g] = 2.98, 1.49",
            "; estimated printing time (normal mode) = 1h 2m 5s",
        ]
    );
}

#[test]
fn filament_g_line_omitted_without_density() {
    let estimate = two_tool_estimate();
    let commands = filament_stats_comment_block(&estimate, None);
    let lines = raw_texts(&commands);

    assert_eq!(
        lines,
        vec![
            "; filament used [mm] = 1000.00, 500.00",
            "; filament used [cm3] = 2.41, 1.20",
            "; estimated printing time (normal mode) = 1h 2m 5s",
        ]
    );
    assert!(!lines.iter().any(|line| line.contains("filament used [g]")));
}
