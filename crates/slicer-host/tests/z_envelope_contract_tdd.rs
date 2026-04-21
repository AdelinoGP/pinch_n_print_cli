//! TDD tests for Non-Planar Z Envelope contract (DEV-005).
//!
//! These tests verify that all 8 Z-bearing push methods on
//! `HostExecutionContext` enforce the invariant:
//!
//!   For any module that writes path Z in Tier 2:
//!     - Lower bound: `layer.z` (or `catchup_z_bottom` for catch-up layers)
//!     - Upper bound: `lower_bound + effective_layer_height`
//!
//! Violations are treated as fatal contract errors with code `Z_ENVELOPE_VIOLATION`.
//!
//! Tests call push methods DIRECTLY on HostExecutionContext (no WASM needed).
//! This avoids the WIT world mismatch that would occur when using test-guest components.
//!
//! Acceptance criteria:
//!   AC-1: z_below_layer_z_floor  → fatal "Z {z} below layer.z floor {floor}"
//!   AC-2: z_above_layer_z_ceiling → fatal "Z {z} above layer.z ceiling {ceiling}"
//!   AC-3: catchup_layer_pass  → Z at catchup_z_bottom + effective_layer_height is valid
//!   AC-4: perim_only_pass    → per-layer module with valid Z completes without violation
//!   AC-N1: z_at_floor_boundary → Z exactly at layer.z is valid (inclusive lower bound)
//!   AC-N2: z_at_ceiling_boundary → Z exactly at layer.z + effective_layer_height is valid

#![allow(missing_docs)]

use slicer_host::wit_host::{
    HostExecutionContext, ExtrusionPath3d, ExtrusionRole, Point3, Point3WithWidth,
    WallLoopType, WallLoopView,
};
use slicer_host::wit_host::layer::slicer::world_layer::ir_handles::{
    HostInfillOutputBuilder, HostPerimeterOutputBuilder, HostSupportOutputBuilder,
};

// ── Helper: build an ExtrusionPath3d with a given Z ───────────────────────

fn make_path(z: f32) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: vec![
            Point3WithWidth { x: 0.0, y: 0.0, z, width: 0.4, flow_factor: 1.0 },
            Point3WithWidth { x: 10.0, y: 0.0, z, width: 0.4, flow_factor: 1.0 },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn make_wall_loop(z: f32) -> WallLoopView {
    WallLoopView {
        perimeter_index: 0,
        loop_type: WallLoopType::Outer,
        path: ExtrusionPath3d {
            points: vec![
                Point3WithWidth { x: 0.0, y: 0.0, z, width: 0.4, flow_factor: 1.0 },
                Point3WithWidth { x: 10.0, y: 0.0, z, width: 0.4, flow_factor: 1.0 },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        feature_flags: vec![],
    }
}

// ── AC-1: Z below layer.z floor → fatal Z_ENVELOPE_VIOLATION ─────────────

#[test]
fn z_below_layer_z_floor() {
    // envelope: layer_z=0.2, effective_layer_height=0.2 → [0.2, 0.4]
    // path.z = 0.1 → below floor → fatal
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(0.1));
    println!("z_below_layer_z_floor: {:?}", result);
    let inner = result.unwrap();
    assert!(inner.is_err(), "Z below floor should return Err");
    let msg = inner.unwrap_err();
    assert!(
        msg.contains("Z_ENVELOPE_VIOLATION"),
        "error should contain Z_ENVELOPE_VIOLATION, got: {msg}"
    );
    assert!(
        msg.contains("below"),
        "error message should mention 'below', got: {msg}"
    );
}

// ── AC-2: Z above layer.z ceiling → fatal Z_ENVELOPE_VIOLATION ───────────

#[test]
fn z_above_layer_z_ceiling() {
    // envelope: layer_z=0.2, effective_layer_height=0.2 → [0.2, 0.4]
    // path.z = 0.5 → above ceiling → fatal
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(0.5));
    println!("z_above_layer_z_ceiling: {:?}", result);
    let inner = result.unwrap();
    assert!(inner.is_err(), "Z above ceiling should return Err");
    let msg = inner.unwrap_err();
    assert!(
        msg.contains("Z_ENVELOPE_VIOLATION"),
        "error should contain Z_ENVELOPE_VIOLATION, got: {msg}"
    );
    assert!(
        msg.contains("above"),
        "error message should mention 'above', got: {msg}"
    );
}

// ── AC-3: catchup layer — Z at catchup_z_bottom + H is valid ─────────────

#[test]
fn catchup_layer_pass() {
    // catchup_z_bottom=0.0, effective_layer_height=0.2 → catchup envelope [0.0, 0.2]
    // path.z = 0.2 → at catchup ceiling → valid (not a violation)
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, Some(0.0), None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(0.2));
    println!("catchup_layer_pass: {:?}", result);
    let inner = result.unwrap();
    assert!(
        inner.is_ok(),
        "Z at catchup_z_bottom + effective_layer_height should be valid, got: {inner:?}"
    );
}

// ── AC-4: per-layer module with all Z within envelope → no violation ───────

#[test]
fn perim_only_pass() {
    // envelope: layer_z=0.2, effective_layer_height=0.2 → [0.2, 0.4]
    // wall loop.z = 0.3 → within envelope → valid
    let mut ctx = HostExecutionContext::new("test.perimeters".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_perimeter_output_builder().unwrap();

    let result = HostPerimeterOutputBuilder::push_wall_loop(&mut ctx, handle, make_wall_loop(0.3));
    println!("perim_only_pass: {:?}", result);
    let inner = result.unwrap();
    assert!(
        inner.is_ok(),
        "perimeter with valid Z should complete without Z envelope violation, got: {inner:?}"
    );
}

// ── AC-N1: Z exactly at layer.z floor → valid (inclusive lower bound) ───────

#[test]
fn z_at_floor_boundary() {
    // envelope: layer_z=0.2, effective_layer_height=0.2 → [0.2, 0.4]
    // path.z = 0.2 → at lower bound → valid (inclusive)
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(0.2));
    println!("z_at_floor_boundary: TEST PASSED");
    let inner = result.unwrap();
    assert!(
        inner.is_ok(),
        "Z exactly at layer.z floor should be valid (inclusive lower bound), got: {inner:?}"
    );
}

// ── AC-N2: Z exactly at layer.z + effective_layer_height → valid ───────────

#[test]
fn z_at_ceiling_boundary() {
    // envelope: layer_z=0.2, effective_layer_height=0.2 → [0.2, 0.4]
    // path.z = 0.4 → at upper bound → valid (inclusive)
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(0.4));
    println!("z_at_ceiling_boundary: TEST PASSED");
    let inner = result.unwrap();
    assert!(
        inner.is_ok(),
        "Z exactly at layer.z + effective_layer_height ceiling should be valid (inclusive upper bound), got: {inner:?}"
    );
}

// ── AC-support: push_solid_path with invalid Z is fatal ───────────────────

#[test]
fn push_solid_path_above_ceiling_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_solid_path(&mut ctx, handle, make_path(0.5));
    let inner = result.unwrap();
    assert!(inner.is_err(), "solid path Z above ceiling should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_ironing_path with invalid Z is fatal ──────────────────

#[test]
fn push_ironing_path_below_floor_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_ironing_path(&mut ctx, handle, make_path(0.1));
    let inner = result.unwrap();
    assert!(inner.is_err(), "ironing path Z below floor should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_seam_candidate with Z below floor is fatal ────────────

#[test]
fn push_seam_candidate_below_floor_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.perimeters".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_perimeter_output_builder().unwrap();

    let result = HostPerimeterOutputBuilder::push_seam_candidate(&mut ctx, handle, Point3 { x: 0.0, y: 0.0, z: 0.1 }, 1.0);
    let inner = result.unwrap();
    assert!(inner.is_err(), "seam candidate Z below floor should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_seam_candidate with Z at boundary is valid ────────────

#[test]
fn push_seam_candidate_at_floor_boundary_is_valid() {
    let mut ctx = HostExecutionContext::new("test.perimeters".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_perimeter_output_builder().unwrap();

    let result = HostPerimeterOutputBuilder::push_seam_candidate(&mut ctx, handle, Point3 { x: 0.0, y: 0.0, z: 0.2 }, 1.0);
    let inner = result.unwrap();
    assert!(inner.is_ok(), "seam candidate Z at floor boundary should be valid, got: {inner:?}");
}

// ── AC-support: push_wall_loop with invalid Z is fatal ─────────────────────

#[test]
fn push_wall_loop_above_ceiling_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.perimeters".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_perimeter_output_builder().unwrap();

    let result = HostPerimeterOutputBuilder::push_wall_loop(&mut ctx, handle, make_wall_loop(0.5));
    let inner = result.unwrap();
    assert!(inner.is_err(), "wall loop Z above ceiling should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_support_path with invalid Z is fatal ───────────────────

#[test]
fn push_support_path_above_ceiling_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.support".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_support_output_builder().unwrap();

    let result = HostSupportOutputBuilder::push_support_path(&mut ctx, handle, make_path(0.5));
    let inner = result.unwrap();
    assert!(inner.is_err(), "support path Z above ceiling should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_interface_path with invalid Z is fatal ─────────────────

#[test]
fn push_interface_path_below_floor_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.support".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_support_output_builder().unwrap();

    let result = HostSupportOutputBuilder::push_interface_path(&mut ctx, handle, make_path(0.1), true);
    let inner = result.unwrap();
    assert!(inner.is_err(), "interface path Z below floor should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── AC-support: push_raft_path with invalid Z is fatal ─────────────────────

#[test]
fn push_raft_path_above_ceiling_is_fatal() {
    let mut ctx = HostExecutionContext::new("test.support".into(), 0.2, 0.2, None, None);
    let handle = ctx.push_support_output_builder().unwrap();

    let result = HostSupportOutputBuilder::push_raft_path(&mut ctx, handle, make_path(0.5));
    let inner = result.unwrap();
    assert!(inner.is_err(), "raft path Z above ceiling should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── Catchup layer: Z below catchup floor is still fatal ─────────────────────

#[test]
fn catchup_layer_z_below_catchup_floor_is_fatal() {
    // catchup_z_bottom=0.0, effective_layer_height=0.2 → catchup envelope [0.0, 0.2]
    // path.z = -0.1 → below catchup floor → fatal (not caught by normal layer check)
    let mut ctx = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, Some(0.0), None);
    let handle = ctx.push_infill_output_builder().unwrap();

    let result = HostInfillOutputBuilder::push_sparse_path(&mut ctx, handle, make_path(-0.1));
    let inner = result.unwrap();
    assert!(inner.is_err(), "Z below catchup_z_bottom should be fatal");
    let msg = inner.unwrap_err();
    assert!(msg.contains("Z_ENVELOPE_VIOLATION"), "got: {msg}");
}

// ── Normal layer vs catchup layer: different floors ─────────────────────────

#[test]
fn normal_layer_uses_layer_z_catchup_uses_catchup_z_bottom() {
    // Normal layer: envelope [0.2, 0.4]
    // Catchup layer: envelope [0.0, 0.2] (catchup_z_bottom=0.0)
    // Z = 0.1 is INVALID for normal layer but VALID for catchup layer

    // Normal layer: Z 0.1 should fail
    let mut ctx_normal = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, None, None);
    let handle_normal = ctx_normal.push_infill_output_builder().unwrap();
    let result_normal = HostInfillOutputBuilder::push_sparse_path(&mut ctx_normal, handle_normal, make_path(0.1));
    assert!(result_normal.unwrap().is_err(), "Z 0.1 should be invalid for normal layer [0.2, 0.4]");

    // Catchup layer: Z 0.1 should pass
    let mut ctx_catchup = HostExecutionContext::new("test.infill".into(), 0.2, 0.2, Some(0.0), None);
    let handle_catchup = ctx_catchup.push_infill_output_builder().unwrap();
    let result_catchup = HostInfillOutputBuilder::push_sparse_path(&mut ctx_catchup, handle_catchup, make_path(0.1));
    assert!(result_catchup.unwrap().is_ok(), "Z 0.1 should be valid for catchup layer [0.0, 0.2]");
}
