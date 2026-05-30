//! AC-4: `runtime_builtins()` enumerates exactly 8 host built-in producers
//! with the canonical `(id, stage, ir_writes)` triples.
//!
//! Reference: `.ralph/specs/69_pnp-cli-unification/packet.spec.md` AC-4.

#![allow(missing_docs)]

use slicer_runtime::runtime_builtins;

#[test]
fn enumerates_exactly_eight_host_builtin_producers() {
    let producers = runtime_builtins();
    let triples: Vec<(String, String, Vec<String>)> = producers
        .iter()
        .map(|p| {
            (
                p.id().to_string(),
                p.stage().to_string(),
                p.ir_writes().to_vec(),
            )
        })
        .collect();

    let expected: Vec<(&str, &str, Vec<&str>)> = vec![
        ("host:mesh", "PrePass::MeshAnalysis", vec!["MeshIR"]),
        (
            "host:mesh_analysis",
            "PrePass::MeshAnalysis",
            vec!["SurfaceClassificationIR"],
        ),
        (
            "host:region_mapping",
            "PrePass::RegionMapping",
            vec!["RegionMapIR"],
        ),
        ("host:slice", "PrePass::Slice", vec!["SliceIR"]),
        (
            "host:shell_classification",
            "PrePass::ShellClassification",
            vec!["SliceIR"],
        ),
        (
            "host:support_geometry",
            "PrePass::SupportGeometry",
            vec!["SupportGeometryIR"],
        ),
        (
            "host:paint_segmentation",
            "PrePass::PaintSegmentation",
            vec!["PaintRegionIR"],
        ),
        ("host:gcode_emit", "PostPass::GCodeEmit", vec!["GCodeIR"]),
    ];

    assert_eq!(triples.len(), 8, "expected 8 host built-in producers");
    for (i, (exp_id, exp_stage, exp_writes)) in expected.iter().enumerate() {
        let (act_id, act_stage, act_writes) = &triples[i];
        assert_eq!(act_id, exp_id, "id mismatch at index {i}");
        assert_eq!(act_stage, exp_stage, "stage mismatch at index {i}");
        let act_writes_strs: Vec<&str> = act_writes.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            &act_writes_strs, exp_writes,
            "ir_writes mismatch at index {i}"
        );
    }
}
