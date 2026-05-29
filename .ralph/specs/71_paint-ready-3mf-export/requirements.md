# Requirements: 71_paint-ready-3mf-export

## Packet Metadata

- Grouped task IDs:
  - `TASK-060`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`pnp_cli` parses `--output-format 3mf` and `--format obj` but cannot write either: `write_mesh` returns `io::ErrorKind::Unsupported` for both (`crates/slicer-runtime/src/helpers_cmd.rs:452`). Only binary STL is wired through (TASK-059). TASK-060 was filed to close this gap and points at `docs/handoff_obj_3mf_writers.md`, which does not exist — so the implementer has no spec. This packet is that spec.

The gap matters because a GUI is being built **against this repository**: its "drop a mesh → paint-ready 3MF" flow needs the backend to emit a clean, container-format 3MF. 3MF (unlike STL) holds multiple objects in one file, and a multi-solid input is expected to fan out into N separately-addressable objects — the behavior OrcaSlicer calls "Split to objects" — so the GUI user can transform, configure, and paint each solid independently. Today STL→3MF would have to masquerade through `repair`/`decimate` (verbs that imply mutation) and there is no connected-component split at all.

This packet does not reopen or supersede a prior packet; it extends the slicer-helpers CLI workstream (TASK-056/058/059) and introduces `mesh convert` + split-to-objects as new surface not yet present in `docs/07`.

## In Scope

- Replace the `write_mesh` `Unsupported` arm with real `OutputFormat::Obj` and `OutputFormat::ThreeMf` dispatch.
- New `write_3mf(&MeshIR, impl Write + Seek)` and `write_obj(&MeshIR, &mut impl Write)` in a new `crates/slicer-runtime/src/model_writer.rs`. Geometry only.
- 3MF output is OrcaSlicer-shaped: full OPC package (`[Content_Types].xml`, `_rels/.rels`, `3D/3dmodel.model`), core + `slic3rpe` namespaces, `unit="millimeter"`, one `<object type="model">` + `<build><item>` (identity transform) per object, and a `Metadata/model_settings.config` skeleton with `subtype="normal_part"` per solid (no `<plate>` block).
- New `split_connected_components(&IndexedTriangleSet) -> Vec<IndexedTriangleSet>` in `crates/slicer-helpers/src/split.rs`, using OrcaSlicer `its_split` adjacency (edge-shared, opposite winding), DFS components, per-component vertex remap, **no** minimum-size threshold.
- New `pnp_cli mesh convert --input --output [--output-format] [--merge-components] [--repair]` verb for STL/OBJ/3MF: loads via `load_model`, optionally repairs, splits to N objects unless `--merge-components`, writes the target format.
- `run_import`: when `--output-format 3mf` and not merging, combine all STEP solids into one `MeshIR` (N objects) and write a single `.3mf` (no `_i` file split); STL/OBJ keep the per-file split.
- Update `OutputFormat` doc comment in `crates/slicer-runtime/src/cli.rs` (drop "not yet implemented").
- Document the writers, `mesh convert`, and split-to-objects in `docs/13_slicer_helpers_crate.md`; mark TASK-060 done.
- Tests: `split_tdd.rs`, `model_writer_roundtrip_tdd.rs`, and `helpers_cli.rs` additions.

## Out of Scope

- Paint data serialization (`FacetPaintData`, `paint_*` triangle attributes) — authored in the GUI.
- Region-modifier / `ModifierVolume` serialization, component composition, `subtype` ≠ `normal_part` — GUI-authored.
- The `<plate>` block, thumbnails, bed/printer metadata — a convert has no plate context.
- Any change to the slicing `load_model` consumer path or to `repair`/`decimate` semantics.
- New crate dependencies (`zip`, `quick-xml`, `stl_io`, `tobj` already present).
- The GUI itself and its loader.
- Editing any other packet's directory.

## Authoritative Docs

- `docs/13_slicer_helpers_crate.md` — ~600 lines; **delegate** ranged reads (Import/exit-code section + line 598). Owner of the CLI surface and TASK-060.
- `docs/02_ir_schemas.md` — large; **delegate** a FACT for exact `MeshIR`/`ObjectMesh`/`IndexedTriangleSet`/`Point3`/`BoundingBox3` field names.
- `docs/01_system_architecture.md` — optional; module search / data ownership context only if needed. Delegate.
- `CLAUDE.md` — load directly (short sections): post-merge naming + snake_case config-key rule.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MeshSplitImpl.hpp` — `its_split` + `create_face_neighbors_index`: edge-shared opposite-winding adjacency, DFS, per-component vertex remap, no size threshold (split semantics, AC-5/AC-N2).
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — model XML header, `<resources>`/`<object>`/`<mesh>`, `<build>`/`<item>`, and the `model_settings.config` `<part subtype=…>` skeleton (output shape, AC-4).
- `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp` — literal `[Content_Types].xml` + `_rels/.rels` (OPC plumbing, AC-4).

## Acceptance Summary

Acceptance Criteria are owned by `packet.spec.md`; referenced here by ID.

- Positive cases: `AC-1` (3MF geometry round-trip, exact verts/indices + AABB), `AC-2` (multi-solid STEP → one 3MF, 2 objects), `AC-3` (convert split vs merge object count), `AC-4` (OrcaSlicer OPC package + namespaces + `normal_part`), `AC-5` (split counts incl. vertex-only-contact = 2), `AC-6` (OBJ geometry + `o` groups).
- Negative cases: `AC-N1` (convert rejects `.step` with redirect to `import`), `AC-N2` (split keeps tiny fragment — no size threshold).
- Refinements not in Given/When/Then form: vertices serialized in **millimetres** with Rust shortest-round-trip `f32` Display (round-trip-exact, AC-1); build `<item transform>` is the identity 12-float string `1 0 0 0 1 0 0 0 1 0 0 0` (AC-4); 3MF `<object id>` resource ids are sequential `u32` starting at 1; object names `"<stem>"` (single) / `"<stem>_<i>"` (split).
- Cross-packet impact: none blocked; downstream GUI flow unblocked.

## Verification Commands

Full matrix. `packet.spec.md` §Verification carries only the gate subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-helpers --test split_tdd -- split_component_counts --exact` | AC-5 split parity | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-helpers --test split_tdd -- split_keeps_tiny_fragment --exact` | AC-N2 no size threshold | FACT pass/fail |
| `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- roundtrip_single_solid_exact --exact` | AC-1 round-trip + AABB | FACT pass/fail |
| `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- threemf_opc_package_and_namespaces --exact` | AC-4 package shape | FACT pass/fail |
| `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- obj_geometry_and_object_groups --exact` | AC-6 OBJ writer | FACT pass/fail |
| `cargo test -p pnp-cli --test helpers_cli -- import_multi_solid_step_to_single_3mf_two_objects --exact` | AC-2 import combine | FACT pass/fail |
| `cargo test -p pnp-cli --test helpers_cli -- convert_split_vs_merge_object_count --exact` | AC-3 convert verb | FACT pass/fail |
| `cargo test -p pnp-cli --test helpers_cli -- convert_rejects_step_input --exact` | AC-N1 reject STEP | FACT pass/fail |
| `cargo check --workspace` | compiles | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | lint gate (CLAUDE.md required-before-commit) | FACT pass/fail |
| `rg -q 'mesh convert' docs/13_slicer_helpers_crate.md` | Doc Impact closure | FACT hit/miss |

## Step Completion Expectations

Cross-step invariants only (per-step fields live in `implementation-plan.md`):

- The split utility (Step 1) must land before the writers/convert (Steps 2–4) consume it; `write_3mf` (Step 2) must land before the `run_import`/`run_convert` wiring (Steps 3–4) calls it.
- No step may regress the existing `import`/`repair`/`decimate` CLI tests in `helpers_cli.rs` even when not editing them: STL/OBJ output must keep the current per-file `_i` split; only the 3MF path combines into one file.
- `write_3mf` is a pure 1:1 serializer (one `<object>` per `MeshIR.objects[i]`); the connected-component split happens upstream in the CLI handlers, never inside the writer (preserves round-trip for deliberately multi-component objects).

## Context Discipline Notes

Packet-specific hazards (workspace-wide discipline lives in the `context-discipline` snippet in `packet.spec.md`):

- Large files in the read-only path that MUST be ranged or delegated: `crates/slicer-runtime/src/model_loader.rs` (>1500 lines — read only `load_3mf`/`resolve_object` and the vertex/triangle parse window via symbol search; never load whole); `docs/13_slicer_helpers_crate.md` (~600 lines — ranged); `docs/02_ir_schemas.md` (delegate FACT).
- Likely temptation reads to skip: `resources/benchy.stl` (11 MB binary — never open); `crates/slicer-runtime/src/model_loader_sidecar.rs` paint/modifier branches (out of scope — only the `<part subtype>` / `PartSubtype::NormalPart` string is needed, get it as a FACT).
- Heaviest-dispatch return-format hints: OrcaSlicer reads → `SUMMARY` (≤ 200 words) or `LOCATIONS`; never request code dumps. `cargo test`/`clippy` → FACT pass/fail, SNIPPETS ≤ 20 lines only on failure.
