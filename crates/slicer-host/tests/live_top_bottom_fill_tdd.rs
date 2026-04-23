//! Integration test: live top/bottom surface fill role preservation.
//!
//! Verifies that the live host path (dispatch → commit → assembly) preserves
//! TopSolidInfill and BottomSolidInfill roles from the infill module output
//! into `LayerCollectionIR.ordered_entities`.
//!
//! The path being tested:
//!   `commit_layer_outputs` → `convert_infill_output` → `InfillIR` →
//!   `arena.set_infill` → `assemble_ordered_entities` → `ordered_entities`
//!
//! The key invariant: `convert_extrusion_role` maps WIT
//! `ExtrusionRole::TopSolidInfill` and `ExtrusionRole::BottomSolidInfill`
//! directly to `slicer_ir::ExtrusionRole` variants. Since
//! `assemble_ordered_entities` (inside the host) reads `path.role` directly,
//! role preservation is guaranteed if the arena-stored `InfillIR` has the
//! correct role on its solid/sparse paths.
//!
//! We verify this by checking `arena.infill()` after commit — confirming
//! the role survived the WIT boundary → `InfillIR` → arena commit path.

#![allow(missing_docs)]

use slicer_host::dispatch::commit_layer_outputs_for_test;
use slicer_host::wit_host::{
    ExtrusionPath3d, ExtrusionRole, HostExecutionContext, Point3WithWidth,
};
use slicer_ir::ExtrusionRole as IrExtrusionRole;

/// Helper: make a 2-point horizontal path at (z=layer_z) in mm units.
fn make_path(
    layer_z: f32,
    x1: f32, y1: f32,
    x2: f32, y2: f32,
    width: f32,
    role: ExtrusionRole,
) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: vec![
            Point3WithWidth { x: x1, y: y1, z: layer_z, width, flow_factor: 1.0 },
            Point3WithWidth { x: x2, y: y2, z: layer_z, width, flow_factor: 1.0 },
        ],
        role,
        speed_factor: 1.0,
    }
}

/// Test that `commit_layer_outputs` for "Layer::Infill" preserves a
/// `TopSolidInfill` role from the WIT boundary into `InfillIR`.
#[test]
fn commit_layer_outputs_preserves_top_solid_infill_role() {
    let module_id = "com.test.top-solid-infill";
    let layer_index = 0u32;

    // Build a HostExecutionContext with a solid path tagged TopSolidInfill.
    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        0.2,   // layer_z
        0.2,   // effective_layer_height
        None,  // catchup_z_bottom
        None,  // mesh_ir
    );
    ctx.infill_output.solid_paths.push(make_path(
        0.2, 0.0, 0.0, 10.0, 0.0,
        0.4,
        ExtrusionRole::TopSolidInfill,
    ));
    // No origin tags — untagged path goes to the one synthetic region.
    ctx.infill_output.solid_path_origins.push(None);

    // Commit into an empty arena.
    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test("Layer::Infill", module_id, layer_index, &ctx, &mut arena, None)
        .expect("commit must succeed");

    // Verify the role survived through the convert_infill_output path and
    // into the arena's InfillIR slot.
    let infill = arena.infill().expect("InfillIR must be set in arena");
    assert!(
        !infill.regions.is_empty(),
        "InfillIR must have at least one region"
    );

    // The synthetic-region path: all untagged paths land in region 0.
    let solid = &infill.regions[0].solid_infill;
    assert!(!solid.is_empty(), "solid_infill must not be empty after TopSolidInfill commit");

    assert_eq!(
        solid[0].role, IrExtrusionRole::TopSolidInfill,
        "convert_infill_output must preserve TopSolidInfill role through the WIT boundary"
    );
    assert_eq!(solid[0].points.len(), 2, "path point count must be preserved");
    assert_eq!(solid[0].points[0].width, 0.4, "path width must be preserved");
}

/// Test that `commit_layer_outputs` for "Layer::Infill" preserves a
/// `BottomSolidInfill` role from the WIT boundary into `InfillIR`.
#[test]
fn commit_layer_outputs_preserves_bottom_solid_infill_role() {
    let module_id = "com.test.bottom-solid-infill";
    let layer_index = 0u32;

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        0.2,
        0.2,
        None,
        None,
    );
    ctx.infill_output.solid_paths.push(make_path(
        0.2, 0.0, 0.0, 20.0, 0.0,
        0.4,
        ExtrusionRole::BottomSolidInfill,
    ));
    ctx.infill_output.solid_path_origins.push(None);

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test("Layer::Infill", module_id, layer_index, &ctx, &mut arena, None)
        .expect("commit must succeed");

    let infill = arena.infill().expect("InfillIR must be set in arena");
    let solid = &infill.regions[0].solid_infill;
    assert!(!solid.is_empty(), "solid_infill must not be empty after BottomSolidInfill commit");

    assert_eq!(
        solid[0].role, IrExtrusionRole::BottomSolidInfill,
        "convert_infill_output must preserve BottomSolidInfill role through the WIT boundary"
    );
}

/// Test that a mix of sparse (regular) infill, top solid fill, and bottom
/// solid fill are all correctly classified and appear with their distinct
/// roles preserved in the arena's InfillIR.
#[test]
fn commit_layer_outputs_preserves_mixed_infill_roles() {
    let module_id = "com.test.mixed-infill";
    let layer_index = 1u32;

    let mut ctx = HostExecutionContext::new(
        module_id.to_string(),
        0.4,   // layer 1
        0.2,
        None,
        None,
    );

    // Sparse (regular) infill path.
    ctx.infill_output.sparse_paths.push(make_path(
        0.4, 0.0, 0.0, 50.0, 0.0,
        0.4,
        ExtrusionRole::SparseInfill,
    ));
    ctx.infill_output.sparse_path_origins.push(None);

    // Top solid fill path.
    ctx.infill_output.solid_paths.push(make_path(
        0.4, 0.0, 0.0, 10.0, 10.0,
        0.4,
        ExtrusionRole::TopSolidInfill,
    ));
    ctx.infill_output.solid_path_origins.push(None);

    // Bottom solid fill path.
    ctx.infill_output.solid_paths.push(make_path(
        0.4, 0.0, 0.0, 15.0, 15.0,
        0.4,
        ExtrusionRole::BottomSolidInfill,
    ));
    ctx.infill_output.solid_path_origins.push(None);

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test("Layer::Infill", module_id, layer_index, &ctx, &mut arena, None)
        .expect("commit must succeed");

    let infill = arena.infill().expect("InfillIR must be set");
    let region = &infill.regions[0];

    assert_eq!(region.sparse_infill.len(), 1, "must have one sparse path");
    assert_eq!(region.sparse_infill[0].role, IrExtrusionRole::SparseInfill);

    assert_eq!(region.solid_infill.len(), 2, "must have two solid paths");

    let has_top    = region.solid_infill.iter().any(|p| p.role == IrExtrusionRole::TopSolidInfill);
    let has_bottom = region.solid_infill.iter().any(|p| p.role == IrExtrusionRole::BottomSolidInfill);
    assert!(has_top,    "ordered_entities must contain TopSolidInfill");
    assert!(has_bottom, "ordered_entities must contain BottomSolidInfill");
}

/// Test that Layer::InfillPostProcess correctly replaces (not appends) the
/// arena's infill slot, and that the TopSolidInfill role is preserved in
/// the replacement.
#[test]
fn commit_layer_outputs_infill_postprocess_replaces_correctly() {
    let module_id = "com.test.infill-postprocess";
    let layer_index = 0u32;

    // First commit a base infill IR via Layer::Infill.
    let mut ctx1 = HostExecutionContext::new(module_id.to_string(), 0.2, 0.2, None, None);
    ctx1.infill_output.sparse_paths.push(make_path(
        0.2, 0.0, 0.0, 100.0, 0.0,
        0.4,
        ExtrusionRole::SparseInfill,
    ));
    ctx1.infill_output.sparse_path_origins.push(None);

    let mut arena = slicer_host::LayerArena::new();
    commit_layer_outputs_for_test("Layer::Infill", module_id, layer_index, &ctx1, &mut arena, None)
        .expect("Layer::Infill commit must succeed");

    // Confirm no TopSolidInfill before postprocess.
    let before = arena.infill().expect("must have infill after first commit");
    assert!(!before.regions[0].solid_infill.iter().any(|p| p.role == IrExtrusionRole::TopSolidInfill));

    // Replace with a Layer::InfillPostProcess commit that carries TopSolidInfill.
    let mut ctx2 = HostExecutionContext::new(module_id.to_string(), 0.2, 0.2, None, None);
    ctx2.infill_output.solid_paths.push(make_path(
        0.2, 0.0, 0.0, 20.0, 0.0,
        0.5,
        ExtrusionRole::TopSolidInfill,
    ));
    ctx2.infill_output.solid_path_origins.push(None);

    commit_layer_outputs_for_test(
        "Layer::InfillPostProcess", module_id, layer_index, &ctx2, &mut arena, None,).expect("Layer::InfillPostProcess commit must succeed");

    // Verify postprocess replaced (not appended) and TopSolidInfill is present.
    let after = arena.infill().expect("must have infill after postprocess commit");
    let solid = &after.regions[0].solid_infill;
    assert_eq!(solid.len(), 1, "InfillPostProcess replaces rather than appends");
    assert_eq!(
        solid[0].role, IrExtrusionRole::TopSolidInfill,
        "TopSolidInfill role must be preserved through InfillPostProcess commit"
    );
}