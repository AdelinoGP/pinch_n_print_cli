---
status: implemented
packet: non-planar-z-envelope
task_ids:
  - TASK-127
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: non-planar-z-envelope

## Goal

Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at per-layer output-commit boundaries (Tier 2), treating violations as fatal contract errors. Covers DEV-005.

## Scope Boundaries

- In scope:
  - Z envelope validation in all per-layer `push_*` methods on `HostExecutionContext` (`push_sparse_path`, `push_solid_path`, `push_ironing_path`, `push_wall_loop`, `push_seam_candidate`, `push_support_path`, `push_interface_path`, `push_raft_path`) that accept `ExtrusionPath3d` or `Point3`
  - Catch-up layer envelope adjustment: `is_catchup_layer = true` replaces the lower bound floor with `catchup_z_bottom`
  - Fatal contract error code `Z_ENVELOPE_VIOLATION` with descriptive message on violation
  - TDD test file `z_envelope_contract_tdd.rs` covering all acceptance criteria
  - Out-of-scope negative guard confirming postpass Z behavior is unaffected

- Out of scope:
  - Postpass Z behavior (governed by a separate contract; handled by a future packet)
  - Non-planar module Z behavior specifically (separate task)
  - Mesh-query host services (no Z emission from those)

## Prerequisites and Blockers

- Depends on: TASK-124 (undeclared runtime read/write enforcement at WIT boundary) — same mechanism, extended to Z validation
- Unblocks: TASK-120 (Benchy parity — without Z envelope enforcement, path optimization could emit invalid Z and mask Benchy output quality issues)

## Acceptance Criteria

- **Given** a per-layer module that commits an extrusion path with a `Point3.z` below `layer.z`, **when** the output builder calls `push_entity` or equivalent commit, **then** the host returns a fatal contract error with error code `Z_ENVELOPE_VIOLATION` and message `"Z {z} below layer.z floor {floor}"`. | `cargo test -p slicer-host --test z_envelope_contract_tdd -- z_below_layer_z_floor --nocapture 2>&1 | grep -E "Z_ENVELOPE|below layer.z floor"`

- **Given** a per-layer module that commits an extrusion path with a `Point3.z` above `layer.z + effective_layer_height`, **when** the output builder calls `push_entity`, **then** the host returns a fatal contract error with error code `Z_ENVELOPE_VIOLATION` and message `"Z {z} above layer.z ceiling {ceiling}"`. | `cargo test -p slicer-host --test z_envelope_contract_tdd -- z_above_layer_z_ceiling --nocapture 2>&1 | grep -E "Z_ENVELOPE|above layer.z ceiling"`

- **Given** a catch-up layer where `is_catchup_layer = true`, `catchup_z_bottom = B`, and `effective_layer_height = H`, **when** an entity with `z = B + H` is committed, **then** no envelope violation is raised. | `cargo test -p slicer-host --test z_envelope_contract_tdd -- catchup_layer_pass --nocapture 2>&1 | grep -E "catchup.*pass|envelope.*pass"`

- **Given** a per-layer module that writes only `PerimeterIR` with all `z` values within `[layer.z, layer.z + effective_layer_height]`, **when** all entities are committed, **then** the slice completes without `Z_ENVELOPE_VIOLATION`. | `cargo test -p slicer-host --test z_envelope_contract_tdd -- perim_only_pass --nocapture 2>&1 | grep -E "TEST.*PASSED|ok"`

## Negative Test Cases

- **Given** a postpass module (`LayerFinalization` or later), **when** it emits an entity with `z` outside the global print Z range, **then** this is outside this packet's scope — postpass Z is governed by a separate contract and is OUT OF SCOPE for this packet.

- **Given** a per-layer module emits an entity with `z` exactly at `layer.z` (floor boundary), **when** committed, **then** no envelope violation is raised (boundary is inclusive). | `cargo test -p slicer-host --test z_envelope_contract_tdd -- z_at_floor_boundary --nocapture 2>&1 | grep -E "TEST.*PASSED|z_at_floor"`

- **Given** a per-layer module emits an entity with `z` exactly at `layer.z + effective_layer_height` (ceiling boundary), **when** committed, **then** no envelope violation is raised (boundary is inclusive). | `cargo test -p slicer-host --test z_envelope_contract_tdd -- z_at_ceiling_boundary --nocapture 2>&1 | grep -E "TEST.*PASSED|z_at_ceiling"`

## Verification

- `cargo test -p slicer-host --test z_envelope_contract_tdd -- --nocapture`
- `cargo build --package slicer-host`
- `cargo clippy --package slicer-host -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules (lines 260-268)
- `docs/02_ir_schemas.md` — `GlobalLayer` struct: `z`, `effective_layer_height`, `is_catchup_layer`, `catchup_z_bottom` (lines 259-284)
- `docs/04_host_scheduler.md` — Phase 4 execution, Per-Layer Execution section

## OrcaSlicer Reference Obligations

None. This is an internal contract enforcement task, not geometry parity with OrcaSlicer.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
