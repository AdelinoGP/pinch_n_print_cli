# Task Map: non-planar-z-envelope

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This file is required when the packet spans more than one task ID, reopens prior packet work, or supersedes an earlier packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-127` | `Step 1` | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `crates/slicer-host/tests/z_envelope_contract_tdd.rs` (new) | None | Write TDD test file with 6+ failing tests covering all acceptance criteria from packet.spec.md. Model: `core_module_ir_access_contract_tdd.rs`. |
| `TASK-127` | `Step 2` | `docs/02_ir_schemas.md` (GlobalLayer fields) | `crates/slicer-host/src/wit_host.rs` (HostExecutionContext struct + new signature), `crates/slicer-host/src/dispatch.rs` (all call sites) | None | Add `layer_z`, `effective_layer_height`, `catchup_z_bottom: Option<f32>` fields to HostExecutionContext and plumb from dispatch. Confirms catch-up layer metadata is populated by PrePass. |
| `TASK-127` | `Step 3` | `docs/01_system_architecture.md` (Non-Planar Z Envelope Rules) | `crates/slicer-host/src/wit_host.rs` (check_z_envelope + 8 push_* methods) | None | Add `check_z_envelope` helper and wire into all Z-bearing push methods. Returns Z_ENVELOPE_VIOLATION on violation. Validates 8 methods: push_sparse_path, push_solid_path, push_ironing_path, push_wall_loop, push_seam_candidate, push_support_path, push_interface_path, push_raft_path. |
| `TASK-127` | `Step 4` | `CLAUDE.md` (workspace gate) | None | None | Run `cargo build --workspace && cargo clippy --workspace -- -D warnings`. Confirms no regressions. |
