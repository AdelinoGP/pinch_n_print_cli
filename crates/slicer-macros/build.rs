//! Rebuild the proc macro when an embedded canonical WIT file changes.

fn main() {
    for path in [
        "../slicer-schema/wit/deps/common.wit",
        "../slicer-schema/wit/deps/config.wit",
        "../slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit",
        "../slicer-schema/wit/deps/ir-types.wit",
        "../slicer-schema/wit/deps/layer-infill-postprocess/layer-infill-postprocess.wit",
        "../slicer-schema/wit/deps/layer-infill/layer-infill.wit",
        "../slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit",
        "../slicer-schema/wit/deps/layer-perimeters-postprocess/layer-perimeters-postprocess.wit",
        "../slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit",
        "../slicer-schema/wit/deps/layer-slice-postprocess/layer-slice-postprocess.wit",
        "../slicer-schema/wit/deps/layer-support-postprocess/layer-support-postprocess.wit",
        "../slicer-schema/wit/deps/layer-support/layer-support.wit",
        "../slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit",
        "../slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit",
        "../slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit",
        "../slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit",
        "../slicer-schema/wit/deps/prepass-seam-planning/prepass-seam-planning.wit",
        "../slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit",
        "../slicer-schema/wit/deps/prepass-types.wit",
        "../slicer-schema/wit/deps/types.wit",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
}
