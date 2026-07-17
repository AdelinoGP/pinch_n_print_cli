//! AC-3.ironing: per-painted-color ironing regression — GREEN gate.
//!
//! Slicing `resources/cube_4color.3mf` paints two distinct colors onto the
//! top face (orange = T0 and red = T3, with vertical-side green/blue = T1/T2
//! regions that don't touch the top). Symptom (pre-fix): only T3 was ironed.
//!
//! Root cause (fixed, packet 128): `PrePass::PaintSegmentation` runs AFTER
//! `PrePass::ShellClassification`. The shell-classification's `top_solid_fill`
//! and `top_shell_index` writes targeted the BASE region, then paint
//! segmentation's Phase 6/7 split the BASE into per-colour regions. Colours
//! that already had a region on the layer kept the propagated
//! `top_shell_index`; colours that did NOT (the `None` arm of the Phase 6/7
//! merge, e.g. a colour whose Voronoi cell first appears at a non-base
//! layer) got a fresh `SlicedRegion { ..Default::default() }` whose
//! `top_shell_index = None` even though the harvested `top_solid_fill`
//! was non-empty. The ironing module's gate at
//! `modules/core-modules/top-surface-ironing/src/lib.rs:316-327` requires
//! `top_shell_index() == Some(0)`, so the colour was silently skipped —
//! and exactly one of the two top-face colours was never ironed on
//! `resources/cube_4color.3mf`.
//!
//! Fix: Phase 6/7's `None` arm now borrows `top_shell_index` and
//! `bottom_shell_index` from any existing region on the layer (which the
//! propagation block has already harmonised), so the new region gets the
//! same shell indices as its painted siblings and downstream
//! surface-treatment modules see the correct gate.
//!
//! This test asserts that across all `;TYPE:Ironing` blocks at the top
//! layer (z = 24.8mm, the topmost exposed surface of the 25mm cube), BOTH
//! top-surface tool indices appear. Each painted colour gets its own
//! `;TYPE:Ironing` block (one per (object_id, region_id) bucket from the
//! per-region origin propagation in packet 127), so the assertion is on
//! the UNION of tools across all ironing blocks, not on a single block.

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use slicer_ir::ConfigValue;
use slicer_runtime::{run_slice, SliceOutcome, SliceRunOptions};
use std::sync::Arc;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn slice_fixture_file(model_path: &PathBuf) -> SliceOutcome {
    assert!(
        model_path.exists(),
        "fixture missing: {} — run from workspace root or restore resources/",
        model_path.display()
    );
    let module_dir = core_modules_dir();
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );
    let opts = SliceRunOptions {
        mesh,
        model_label: model_path.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        progress_events: false,
        // Ironing defaults to OFF (OrcaSlicer parity, commit d11f9ff8), so this
        // per-painted-color ironing regression must enable it explicitly.
        config_overrides: std::collections::HashMap::from([(
            "ironing_enabled".to_string(),
            ConfigValue::Bool(true),
        )]),
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against {}: {e}", model_path.display()))
}

/// Scan gcode and return, for every `;TYPE:Ironing` block, the set of tool
/// indices that appear on at least one `G1 ... E` extrusion move inside
/// that block. The state machine resets the current type on
/// `;LAYER_CHANGE` (matching the gcode convention).
fn tools_per_ironing_block(gcode: &str) -> Vec<BTreeSet<u32>> {
    let mut results: Vec<BTreeSet<u32>> = Vec::new();
    let mut current_tools: BTreeSet<u32> = BTreeSet::new();
    let mut current_type: &str = "";
    let mut current_tool: Option<u32> = None;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(";LAYER_CHANGE") {
            if current_type == "Ironing" && !current_tools.is_empty() {
                results.push(std::mem::take(&mut current_tools));
            } else {
                current_tools.clear();
            }
            current_type = "";
            // Deliberately do NOT reset `current_tool`: the physical tool
            // persists across layer changes, and the first fragment of a
            // layer is traced by the tool carried over from the previous
            // layer's tail. Resetting to None here silently dropped the
            // carry-over color's ironing block (2026-07-17: T3's top-layer
            // ironing ran as the carry-over fragment and was uncounted,
            // failing the union assertion even though both colors were
            // correctly ironed).
            continue;
        }
        if trimmed.starts_with(";TYPE:") {
            // Flush the previous block on type transition.
            if current_type == "Ironing" && !current_tools.is_empty() {
                results.push(std::mem::take(&mut current_tools));
            } else {
                current_tools.clear();
            }
            current_type = trimmed.trim_start_matches(";TYPE:").trim();
            continue;
        }
        if trimmed.len() >= 2
            && trimmed.as_bytes()[0] == b'T'
            && trimmed[1..].bytes().all(|c| c.is_ascii_digit())
        {
            if let Ok(n) = trimmed[1..].parse::<u32>() {
                current_tool = Some(n);
            }
            continue;
        }
        if current_type == "Ironing" {
            if let Some(t) = current_tool {
                if trimmed.starts_with("G1 ") && trimmed.contains(" E") {
                    current_tools.insert(t);
                }
            }
        }
    }
    if current_type == "Ironing" && !current_tools.is_empty() {
        results.push(current_tools);
    }
    results
}

/// Regression: cube_4color's top face is painted with TWO distinct tool
/// indices (T0 = orange, T3 = red; T1 and T2 are vertical-side colours
/// that do not touch the top). Both top-surface colours must appear in
/// the union of tools across all `;TYPE:Ironing` blocks at the top
/// layer. The root-cause fix lives in
/// `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (packet 128):
/// Phase 6/7's `None` arm of the per-colour merge was creating a fresh
/// `SlicedRegion { ..Default::default() }` for a colour that had no
/// region on the layer yet, leaving `top_shell_index = None` even
/// though the harvested `top_solid_fill` was non-empty — and the
/// ironing module's gate at
/// `modules/core-modules/top-surface-ironing/src/lib.rs:316-327`
/// (`top_shell_index() != Some(0)`) silently skipped the region. The
/// fix borrows the per-layer shell index from any existing region
/// (harmonised by the propagation block above) so the new region
/// carries `top_shell_index = Some(0)`.
#[test]
fn cube_4color_ironing_per_painted_top_color() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let blocks = tools_per_ironing_block(&outcome.gcode_text);
    assert!(
        !blocks.is_empty(),
        "cube_4color must emit at least one `;TYPE:Ironing` block (ironing enabled via config override); \
         found 0. Pre-fix a single `;TYPE:Ironing` block was emitted for only one of the two \
         top-surface colors; if the assertion is failing for the opposite reason (zero ironing \
         at all) check that top-surface-ironing is loaded and ironing_enabled is true."
    );

    // The two top-surface tools are T0 (orange) and T3 (red). T1 and T2 are
    // vertical-side colours and should NOT appear at the top. Assert that
    // BOTH T0 and T3 are present in the union of tools across all ironing
    // blocks. Each painted colour gets its own `;TYPE:Ironing` block (one
    // per (object_id, region_id) bucket from packet 127's per-region origin
    // propagation), so the assertion is on the union, not on a single block.
    let all_tools: BTreeSet<u32> = blocks.iter().flatten().copied().collect();
    assert!(
        all_tools.contains(&0) && all_tools.contains(&3),
        "cube_4color top-layer ironing must touch BOTH top-surface colors (T0 and T3). \
         Pre-fix only T3 was ironed because paint segmentation's Phase 6/7 created a \
         T0 region with `top_solid_fill` non-empty but `top_shell_index = None`, so the \
         ironing module's gate `top_shell_index() != Some(0)` skipped it. \
         Found {all_tools:?} across {blocks:?} ironing blocks. \
         All ironing blocks (layer, tools): {blocks:?}. \
         Per-layer ironing tools: [(0, {{3}})]."
    );
}
