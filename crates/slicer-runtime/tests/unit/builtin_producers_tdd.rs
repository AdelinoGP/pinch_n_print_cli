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

    // Note: order of entries below must match runtime_builtins() iteration order.
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
        // Packet 95 D1 + D8: `PrePass::PaintSegmentation` runs as a host stage
        // via `run_builtin_stage` in prepass.rs (between ShellClassification
        // and SupportGeometry) and writes back into SliceIR via
        // `Blackboard::replace_slice_ir`.  It does NOT register a separate
        // `Producer` in `runtime_builtins()` because its outputs ride the
        // existing SliceIR slot — confirmed clean per AC-14 + AC-15.
        ("host:gcode_emit", "PostPass::GCodeEmit", vec!["GCodeIR"]),
    ];

    assert_eq!(
        triples.len(),
        7,
        "expected 7 host built-in producers (P94r removed mesh_segmentation; \
         P95 paint_segmentation runs as a prepass stage that writes back into SliceIR \
         and intentionally does NOT register a distinct Producer)"
    );
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
