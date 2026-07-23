//! Rebuild the proc macro when an embedded canonical WIT file changes.

fn main() {
    for path in [
        "../slicer-schema/wit/deps/types.wit",
        "../slicer-schema/wit/deps/config.wit",
        "../slicer-schema/wit/deps/ir-types.wit",
        "../slicer-schema/wit/deps/common.wit",
        "../slicer-schema/wit/deps/world-prepass/world-prepass.wit",
        "../slicer-schema/wit/deps/world-postpass/world-postpass.wit",
        "../slicer-schema/wit/deps/world-finalization/world-finalization.wit",
        "../slicer-schema/wit/deps/world-layer/world-layer.wit",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
}
