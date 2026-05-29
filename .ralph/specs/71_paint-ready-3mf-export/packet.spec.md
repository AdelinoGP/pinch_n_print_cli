---
status: draft
packet: 71_paint-ready-3mf-export
task_ids:
  - TASK-060
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 71_paint-ready-3mf-export

## Goal

Light up geometry-only OBJ and 3MF mesh writers and a `pnp_cli mesh convert` verb so the CLI emits OrcaSlicer-shaped, paint-ready meshes, fanning a multi-solid input out into N separate 3MF objects via OrcaSlicer-parity connected-component splitting.

## Scope Boundaries

This packet replaces the `Unsupported` stub in `write_mesh` with real `write_3mf`/`write_obj` writers, adds a `mesh convert` subcommand for mesh-format I/O (STL/OBJ/3MF), and adds a `split_connected_components` utility in `slicer-helpers`. Output is **geometry only** plus the OrcaSlicer `model_settings.config` `normal_part` skeleton — paint data and region modifiers are authored downstream in the GUI and are out of scope. The slicing `load_model` pipeline path and the meanings of `repair`/`decimate` are unchanged. Full in/out lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: TASK-059 (STL writer + repair/decimate/import wiring — done), TASK-058 (STEP import — done).
- Unblocks: the downstream GUI "drop a file → paint-ready 3MF" flow; un-gates the external `benchy_import_under_2s` acceptance check (it skips today only because no binary can emit a 3MF).
- Activation blockers: none open. See `design.md` §Open Questions.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID.

- **AC-1. Given** a single-solid `MeshIR`, **when** it is serialized with `write_3mf` and reloaded with `load_3mf`, **then** the reloaded mesh has exactly 1 object whose `vertices` and `indices` are bit-identical to the source and whose AABB equals the source AABB. | `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- roundtrip_single_solid_exact --exact`
- **AC-2. Given** the two-solid fixture `crates/slicer-helpers/tests/resources/assembly.step`, **when** `pnp_cli mesh import --input assembly.step --output out.3mf --output-format 3mf` runs without `--merge-components`, **then** exactly one file `out.3mf` is written (no `out_0.3mf`/`out_1.3mf`) and `load_3mf(out.3mf)` returns exactly 2 objects. | `cargo test -p pnp-cli --test helpers_cli -- import_multi_solid_step_to_single_3mf_two_objects --exact`
- **AC-3. Given** a `MeshIR` containing two spatially-disjoint cubes built inline, **when** `run_convert` writes 3MF without `--merge-components`, **then** `load_3mf` returns 2 objects; **and when** the same input is converted with `--merge-components`, **then** `load_3mf` returns 1 object. | `cargo test -p pnp-cli --test helpers_cli -- convert_split_vs_merge_object_count --exact`
- **AC-4. Given** any `write_3mf` output, **when** the ZIP is inspected, **then** it contains the entries `[Content_Types].xml`, `_rels/.rels`, `3D/3dmodel.model`, and `Metadata/model_settings.config`; the `<model>` root carries `unit="millimeter"` and `xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"`; and every `<part>` in the sidecar carries `subtype="normal_part"`. | `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- threemf_opc_package_and_namespaces --exact`
- **AC-5. Given** a mesh of two tetrahedra that are fully disjoint, `split_connected_components` returns 2 components; **given** one watertight cube it returns 1; **given** two tetrahedra that touch at a single shared vertex only (no shared edge), it returns 2 (vertex-only contact does not merge, matching OrcaSlicer `its_split`). | `cargo test -p slicer-helpers --test split_tdd -- split_component_counts --exact`
- **AC-6. Given** a multi-object `MeshIR`, **when** serialized with `write_obj`, **then** the output parses via `tobj` with total vertex and triangle counts equal to the source and contains one `o ` group line per object. | `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- obj_geometry_and_object_groups --exact`

## Negative Test Cases

- **AC-N1. Given** a `.step` path passed to `pnp_cli mesh convert`, **when** the command runs, **then** it exits non-zero (`exit_codes::UNREADABLE`) and prints a message directing the user to `mesh import` (it does not attempt mesh-loader parsing of the STEP file). | `cargo test -p pnp-cli --test helpers_cli -- convert_rejects_step_input --exact`
- **AC-N2. Given** a `MeshIR` with one large solid plus one spatially-disjoint single-triangle fragment, **when** `split_connected_components` runs, **then** it returns 2 components and the single-triangle fragment is retained (no minimum-size threshold drops geometry). | `cargo test -p slicer-helpers --test split_tdd -- split_keeps_tiny_fragment --exact`

## Verification

Gate commands only — full matrix in `requirements.md` §Verification Commands.

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd`

## Authoritative Docs

- `docs/13_slicer_helpers_crate.md` — owner of the `repair`/`decimate`/`import` CLI surface, exit-code table, and TASK-060. Delegate a ranged read around the Import section + line 598; do not load in full.
- `docs/02_ir_schemas.md` — exact `MeshIR` / `ObjectMesh` / `IndexedTriangleSet` / `Point3` field names. Delegate a FACT for field spellings; do not load in full.
- `CLAUDE.md` — post-merge naming (`slicer-runtime`, `pnp_cli`) and snake_case config-key convention. Load the two relevant sections directly (short).

## Doc Impact Statement (Required)

A list of specific doc sections this packet adds or modifies (doc edits land in this packet, not a follow-up):

- `docs/13_slicer_helpers_crate.md` §"Mesh output writers (OBJ / 3MF)" — `rg -q 'write_3mf' docs/13_slicer_helpers_crate.md`
- `docs/13_slicer_helpers_crate.md` §"`mesh convert` (split-to-objects)" — `rg -q 'mesh convert' docs/13_slicer_helpers_crate.md`
- `docs/13_slicer_helpers_crate.md` TASK-060 marked done — `rg -q 'TASK-060.*(done|complete)' docs/13_slicer_helpers_crate.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MeshSplitImpl.hpp` — `its_split` / `create_face_neighbors_index`: connected-component grouping by **edge-shared, opposite-winding** adjacency (borrowed verbatim as the split semantics; AC-5).
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `3dmodel.model` model header, `<resources>`/`<object>`/`<mesh>`, `<build>`/`<item>` shape, and the `Metadata/model_settings.config` `<part subtype=…>` skeleton (borrowed output shape; AC-4).
- `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp` — literal `[Content_Types].xml` and `_rels/.rels` contents (borrowed OPC plumbing; AC-4).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
