//! Host-built-in `Layer::Slice` stage (TASK-107).
//!
//! Produces a `SliceIR` for a single global layer by calling
//! `slicer_core::slice_mesh_ex` on each object mesh at the layer's Z. The
//! `SliceIR` is staged in the per-layer arena before any user
//! `Layer::Slice` / `Layer::SlicePostProcess` module runs.

use std::collections::HashMap;
use std::fmt;

use slicer_core::slice_mesh_ex;
use slicer_ir::{GlobalLayer, MeshIR, ObjectId, SemVer, SliceIR, SlicedRegion};

/// Structured failures for the host-built-in `Layer::Slice` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSliceError {
    /// A layer referenced an `ObjectId` that is not present in `MeshIR`.
    UnknownObject {
        /// Layer that referenced the unknown object.
        layer_index: u32,
        /// The missing object identifier.
        object_id: ObjectId,
    },
}

impl fmt::Display for LayerSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObject {
                layer_index,
                object_id,
            } => write!(
                f,
                "Layer::Slice at layer {layer_index} references unknown object '{object_id}'"
            ),
        }
    }
}

impl std::error::Error for LayerSliceError {}

/// Produce the `SliceIR` for `layer` by slicing every referenced object mesh
/// at `layer.z`.
///
/// Deterministic: regions are emitted in `layer.active_regions` order.
/// If `layer.active_regions` is empty the returned `SliceIR` has an empty
/// `regions` vector (e.g. a layer with no participating objects).
pub fn execute_layer_slice(mesh: &MeshIR, layer: &GlobalLayer) -> Result<SliceIR, LayerSliceError> {
    let mut regions = Vec::with_capacity(layer.active_regions.len());
    for active in &layer.active_regions {
        let object = mesh
            .objects
            .iter()
            .find(|o| o.id == active.object_id)
            .ok_or_else(|| LayerSliceError::UnknownObject {
                layer_index: layer.index,
                object_id: active.object_id.clone(),
            })?;

        let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
        let polygons = sliced.pop().unwrap_or_default();

        regions.push(SlicedRegion {
            object_id: active.object_id.clone(),
            region_id: active.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: active.effective_layer_height,
            boundary_paint: HashMap::new(),
        });
    }

    Ok(SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer.index,
        z: layer.z,
        regions,
    })
}
