---
status: implemented
packet: 71_paint-ready-3mf-export
task_ids:
  - TASK-060
---

# 71_paint-ready-3mf-export

## Goal

Light up geometry-only OBJ and 3MF mesh writers and a `pnp_cli mesh convert` verb so the CLI emits OrcaSlicer-shaped, paint-ready meshes, fanning a multi-solid input out into N separate 3MF objects via OrcaSlicer-parity connected-component splitting.

## Problem Statement

`pnp_cli` parses `--output-format 3mf` and `--format obj` but cannot write either: `write_mesh` returns `io::ErrorKind::Unsupported` for both (`crates/slicer-runtime/src/helpers_cmd.rs:452`). Only binary STL is wired through (TASK-059). TASK-060 was filed to close this gap and points at `docs/handoff_obj_3mf_writers.md`, which does not exist — so the implementer has no spec. This packet is that spec.

The gap matters because a GUI is being built **against this repository**: its "drop a mesh → paint-ready 3MF" flow needs the backend to emit a clean, container-format 3MF. 3MF (unlike STL) holds multiple objects in one file, and a multi-solid input is expected to fan out into N separately-addressable objects — the behavior OrcaSlicer calls "Split to objects" — so the GUI user can transform, configure, and paint each solid independently. Today STL→3MF would have to masquerade through `repair`/`decimate` (verbs that imply mutation) and there is no connected-component split at all.

This packet does not reopen or supersede a prior packet; it extends the slicer-helpers CLI workstream (TASK-056/058/059) and introduces `mesh convert` + split-to-objects as new surface not yet present in `docs/07`.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Guard against the above:** `MeshIR`/`IndexedTriangleSet` vertices are already stored in **millimetres** (`Point3 { x,y,z: f32 // mm }`), and `load_3mf` reads / `write_stl_binary` writes them as mm verbatim. The 100 nm unit system applies to slicing `Point2`/layer geometry, NOT input meshes. The writers MUST emit `vertex` coords as-is (no `/100`, no `mm_to_units`) and declare `unit="millimeter"`; applying the conversion here is a bug.
- No guest-WASM input is touched (the change surface is `slicer-helpers`/`slicer-runtime`/`pnp-cli`, none of which are guest sources per `CLAUDE.md`'s guest-staleness list), so no `build-*-guests.sh --check` is required.
- Config-key strings (if any are emitted into the sidecar) use snake_case per `CLAUDE.md`.

## Data and Contract Notes

- IR/contracts touched: read-only consumers of `MeshIR`/`ObjectMesh`/`IndexedTriangleSet`; no IR struct is modified. No schema-version bump.
- WIT boundary: none — host-side CLI + helpers only; no guest interface change.
- Determinism: `split_connected_components` must emit components in a deterministic order (seed faces in ascending index order) so object ids / `_i` names and tests are stable. Vertex remap preserves first-seen order.
- 3MF reader contract the writer must satisfy: `<model unit="millimeter" xmlns="…/core/2015/02">`, `<object id type="model"><mesh><vertices><vertex x= y= z=/></vertices><triangles><triangle v1= v2= v3=/></triangles></mesh></object>`, `<build><item objectid= transform=/></build>`; sidecar `Metadata/model_settings.config` `<config><object id><part id subtype="normal_part">…</part></object></config>`.

## Locked Assumptions and Invariants

- `write_3mf` emits exactly one `<object>` per `MeshIR.objects[i]`; splitting is never performed inside the writer.
- 3MF `<object id>` resource ids are sequential `u32` from 1; build `<item>` transforms are identity (`MeshIR` mesh coords are already world/object-local mm).
- No minimum-component-size threshold is ever applied — geometry is never silently dropped (AC-N2).
- STL/OBJ output keeps the existing per-solid `_i` file split; only 3MF combines into one container file.
- `repair`/`decimate`/`load_model`/slice-pipeline behavior is unchanged.

## Risks and Tradeoffs

- Split adjacency correctness on non-manifold meshes is the highest-risk area: the opposite-winding edge test must match OrcaSlicer or component counts drift. Mitigated by AC-5 (incl. the vertex-only-contact = 2 case) and AC-N2.
- 3MF byte-shape need not be byte-identical to OrcaSlicer (this repo is the contract); the bar is "round-trips through our `load_3mf`" + "opens in OrcaSlicer" as an independent parity smoke check. Float formatting uses Rust shortest round-trip `f32` (round-trip-exact like Orca's `%.9g`), not Orca's exact digits.
- `zip` writer needs `Write + Seek`; pass `std::fs::File` directly (not `BufWriter`, which is not `Seek`).
