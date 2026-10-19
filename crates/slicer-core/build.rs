//! Build-time declarations for reserved slicer-core cfg names.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(pnp_perimeter_spatial_accelerated)");
}
