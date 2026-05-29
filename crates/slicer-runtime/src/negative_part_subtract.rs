//! Host-built-in negative-part subtract step (Packet 56c).
//!
//! `apply_negative_part_subtract` is called from `layer_executor::run_paint_annotation`
//! immediately after the per-layer `SliceIR` is pulled from the arena — before any paint
//! annotation logic sees the polygons. This ensures that downstream annotators (fuzzy-skin,
//! support routing) operate on geometry that already has void volumes removed.

use slicer_core::{difference, slice_mesh_ex};
use slicer_ir::{ConfigValue, ModifierVolume, SliceIR};

/// Subtract every `negative_part` modifier volume from all regions in `slice_ir`.
///
/// For each modifier volume whose `config_delta` carries `subtype = "negative_part"`:
/// 1. Determine the modifier mesh's Z extent from its vertices.
/// 2. Skip layers whose Z falls outside the modifier's Z extent.
/// 3. Project the modifier mesh at `slice_ir.z` via `slice_mesh_ex`.
/// 4. Subtract the projected polygons from every `SlicedRegion.polygons`.
///
/// Empty meshes and out-of-range layers are silently skipped.
pub fn apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume]) {
    for mv in modifier_volumes {
        // Check the subtype config key.
        let is_negative = mv.config_delta.fields.get("subtype").map_or(false, |v| {
            v == &ConfigValue::String("negative_part".to_string())
        });
        if !is_negative || mv.mesh.vertices.is_empty() {
            continue;
        }

        // Determine the Z extent of the modifier mesh.
        let z_min = mv
            .mesh
            .vertices
            .iter()
            .map(|v| v.z)
            .fold(f32::INFINITY, f32::min);
        let z_max = mv
            .mesh
            .vertices
            .iter()
            .map(|v| v.z)
            .fold(f32::NEG_INFINITY, f32::max);

        // Skip if this layer is outside the modifier's Z range.
        if slice_ir.z < z_min || slice_ir.z > z_max {
            continue;
        }

        // Project the modifier mesh at this layer's Z.
        let projected = slice_mesh_ex(&mv.mesh, &[slice_ir.z]);
        let negative_polys = match projected.into_iter().next() {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        // Subtract from each region's polygons.
        for region in &mut slice_ir.regions {
            if !region.polygons.is_empty() {
                region.polygons = difference(&region.polygons, &negative_polys);
            }
        }
    }
}
