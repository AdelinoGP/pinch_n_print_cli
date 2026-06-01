# Task Map — Packet 81

This packet spans **1 task ID** in `docs/07_implementation_status.md`: **TASK-233**.

> Numbering note: this packet's `packet.spec.md` originally claimed `TASK-231`, but the doc reassigned TASK-231 to "Audit `docs/05_module_sdk.md` §Geometry Helpers" before this packet activated; `TASK-232` is held by packet 82. `TASK-233` is the next free ID and is reserved at packet 81's docs/07 sync (which lands in Step 3 of the implementation, alongside the bulk move).

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-233 | Steps 0, 1, 2, 3, 4, 5, 6 | Extract model I/O (`model_loader.rs`, `model_loader_sidecar.rs`, `model_writer.rs` — ~2 900 LOC) from `slicer-runtime` into a new `slicer-model-io` leaf crate; delete `stl_io`, `tobj`, `zip`, `quick-xml`, `uuid` direct deps from `slicer-runtime/Cargo.toml`; replace `SliceRunOptions.model_path: PathBuf` with `mesh: Arc<MeshIR>` and move the file load into `pnp-cli`'s slice subcommand; promote `assemble_object` (and any externally-needed `pub(crate)` items) to `pub`; migrate loader/writer tests into the new crate's `tests/` directory. |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-233 | `crates/slicer-ir/src/slice_ir.rs:432,402,150` — `MeshIR` / `ObjectMesh` / `IndexedTriangleSet` struct definitions (read for AC-7 field-path assertion and to confirm `MeshIR` is the value crossing the new seam). `crates/slicer-runtime/src/blackboard.rs:59` — `mesh_ir: Arc<MeshIR>` confirms the existing internal storage shape; `Arc<MeshIR>` is the natural type to cross the seam. `crates/slicer-runtime/src/cli.rs:251` — `SliceRunOptions` definition (the struct whose `model_path` field is replaced by `mesh`). `docs/01_system_architecture.md` §"Module search path / file ingestion" and §"Data ownership" — read only, no edit. `docs/02_ir_schemas.md` §`MeshIR` — read only, no edit. `CONTEXT.md` §"Paint-ready 3MF" — the geometry-only writer's output contract, unchanged. `CLAUDE.md` §"Build & Test Commands" — the canonical `pnp_cli slice ...` invocation, unchanged. |

## OrcaSlicer references

None. The relocated files implement format parsers (STL via `stl_io`, OBJ via `tobj`, 3MF via `zip` + `quick-xml`), not slicing algorithms. `OrcaSlicerDocumented/` is out of scope for this packet — `design.md` §"Controlling Code Paths" makes this explicit and the implementation plan instructs the implementer not to consult it.

## Predecessor / successor relationships

- **Predecessors**: None. Packet 81 is the first in the architecture-deepening batch (P81–P88) and has no prior-packet dependencies. Closure requires only `cargo xtask build-guests --check` clean (no WIT touched, so guests should stay clean without rebuild).
- **Successors**:
  - **Packet 82** (`.ralph/specs/82_cli-bodies-out-of-runtime/`, `requires: [81]`, TASK-232). Hard dependency — P82 moves `helpers_cmd.rs` from `slicer-runtime/src/` into `pnp-cli/src/`, importing `slicer_model_io::{assemble_object, load_model}`. P81 prepares the ground by (a) creating `slicer-model-io`, (b) promoting `assemble_object` from `pub(crate)` to `pub`, and (c) rewriting `helpers_cmd.rs`'s imports to use `slicer_model_io::` in place — so P82 is a pure file move.
  - **Packets 83–88** (the rest of the deepening batch) reference `slicer-model-io` indirectly through `pnp-cli` and the `slicer-runtime` orchestrator's reshaped surface; none have a `requires:` edge to P81, but all assume P81's seam is in place.

## Backlog sync status

TASK-233 is added to `docs/07_implementation_status.md` under "Architecture Deepening Phase I" with status `[ ]` during Step 3 of this packet (alongside the bulk move that closes AC-1..AC-5). It transitions to `[x]` with `Closed <date> — packet 81` suffix at the end of this packet's Acceptance Ceremony, after the full `cargo test --workspace` gate passes.

## End-state of packet 81

At packet 81's closure:

- `crates/slicer-model-io/` exists as a leaf crate with `slicer-ir` as its only first-party dep and the five file-format crates (`stl_io`, `tobj`, `zip`, `quick-xml`, `uuid`) as its only direct external file-I/O deps.
- `crates/slicer-runtime/Cargo.toml` no longer declares any of those five file-format deps; `cargo tree -p slicer-runtime` shows them absent.
- `crates/slicer-runtime/` no longer declares a dep on `slicer-model-io` either — the dep graph structurally enforces the seam (`slicer-runtime` cannot reach `load_model` without re-adding the edge).
- `SliceRunOptions` (in `crates/slicer-runtime/src/cli.rs`) carries `pub mesh: Arc<MeshIR>` in place of `pub model_path: PathBuf`. `run_slice`'s arity is unchanged; the file load happens in `pnp-cli`'s slice subcommand before the runtime is entered.
- `slicer_model_io::load_model`, `assemble_object`, `detect_format`, `write_3mf`, `write_obj`, `parse_3mf_sidecar`, `ObjectSidecarInfo`, `PartSubtype` are the public surface; `assemble_object` is `pub` (promoted from `pub(crate)`) so packet 82 can consume it from `pnp-cli` without further surface changes.
- `crates/slicer-model-io/tests/` holds round-trip integration tests for STL, OBJ, and 3MF formats; the `resources/benchy.stl` mesh round-trip asserts the IR field-chain `mesh_ir.objects[0].mesh.indices.len() > 0`.
- `pnp_cli slice --model resources/benchy.stl ...` produces byte-identical g-code to the pre-packet baseline (SHA captured in Step 0, verified in Step 5).
- No documentation file is edited; `docs/07_implementation_status.md` gains a single line claiming TASK-233 (added during Step 3, closed at the ceremony).
