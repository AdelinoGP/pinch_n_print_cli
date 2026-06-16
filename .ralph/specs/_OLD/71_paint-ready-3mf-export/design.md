# Design: 71_paint-ready-3mf-export

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/helpers_cmd.rs` — `write_mesh` (`:440-457`, the `Unsupported` arm to replace), `write_stl_binary` (`:459-496`, per-object iteration template), `run_import` (`:299-313`, the multi-solid write loop), `resolve_output_format`/`format_from_extension` (`:391-424`, reuse for `convert`), and exit-code constants (`exit_codes::{SUCCESS, UNREADABLE, EMPTY_OR_TRIVIAL, WARNINGS_OR_PARTIAL, PARSE_ERROR}`).
- New surfaces: `crates/slicer-runtime/src/model_writer.rs` (`write_3mf`, `write_obj`); `crates/slicer-helpers/src/split.rs` (`split_connected_components`); `Convert` variant + dispatch in `crates/pnp-cli/src/main.rs` (`MeshCmd` enum `:123-189`).
- Round-trip oracle: `crates/slicer-runtime/src/model_loader.rs::load_3mf` and `resolve_object` (the reader the writer must satisfy) and `model_loader.rs::load_model` (`:145`, extension dispatch used by `convert`).
- Neighboring tests/fixtures: `crates/pnp-cli/tests/helpers_cli.rs` (helpers `cube_step_fixture`, `write_tiny_stl`, `write_sphere_stl`); `crates/slicer-helpers/tests/repair_tdd.rs` (`valid_cube`, `single_object_mesh` inline-mesh helpers to mirror); fixture `crates/slicer-helpers/tests/resources/assembly.step` (two solids, AC-2).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Guard against the above:** `MeshIR`/`IndexedTriangleSet` vertices are already stored in **millimetres** (`Point3 { x,y,z: f32 // mm }`), and `load_3mf` reads / `write_stl_binary` writes them as mm verbatim. The 100 nm unit system applies to slicing `Point2`/layer geometry, NOT input meshes. The writers MUST emit `vertex` coords as-is (no `/100`, no `mm_to_units`) and declare `unit="millimeter"`; applying the conversion here is a bug.
- No guest-WASM input is touched (the change surface is `slicer-helpers`/`slicer-runtime`/`pnp-cli`, none of which are guest sources per `CLAUDE.md`'s guest-staleness list), so no `build-*-guests.sh --check` is required.
- Config-key strings (if any are emitted into the sidecar) use snake_case per `CLAUDE.md`.

## Code Change Surface

- Selected approach: split lives in the **CLI handler** path (not in `load_model`, not in `write_3mf`). `write_3mf` stays a pure 1:1 serializer over `MeshIR.objects`; `run_convert`/`run_import` perform the component split before calling it. This contains blast radius (slice pipeline untouched) and keeps round-trip clean for deliberately multi-component 3MF objects.
- Exact functions/files expected to change:
  - `crates/slicer-helpers/src/split.rs` (new) + `lib.rs` re-export — `split_connected_components`.
  - `crates/slicer-runtime/src/model_writer.rs` (new) + `lib.rs` `mod model_writer;` — `write_3mf`, `write_obj`, private XML/zip helpers.
  - `crates/slicer-runtime/src/helpers_cmd.rs` — wire `write_mesh` arms; `run_import` 3MF combine; new `run_convert`.
  - `crates/slicer-runtime/src/cli.rs` — `OutputFormat` doc comment.
  - `crates/pnp-cli/src/main.rs` — `Convert` variant + dispatch.
  - Tests: `crates/slicer-helpers/tests/split_tdd.rs` (new), `crates/slicer-runtime/tests/model_writer_roundtrip_tdd.rs` (new), `crates/pnp-cli/tests/helpers_cli.rs` (extend).
  - `docs/13_slicer_helpers_crate.md` — document + close TASK-060.
- Rejected alternatives: (a) reuse `repair.rs` undirected edge-component logic — rejected: diverges from OrcaSlicer on non-manifold input, which can occur because `convert --repair` is opt-in. (b) split inside `load_model` — rejected: perturbs the slice pipeline's object count. (c) split inside `write_3mf` — rejected: fragments deliberately-grouped objects on round-trip and couples geometry analysis into the serializer.

## Files in Scope (read + edit)

Primary (≤ 3 per step; see `implementation-plan.md` for per-step assignment):

- `crates/slicer-helpers/src/split.rs` — role: connected-component split; expected change: new file + `lib.rs` export.
- `crates/slicer-runtime/src/model_writer.rs` — role: OBJ/3MF serializers; expected change: new file + `lib.rs` mod.
- `crates/slicer-runtime/src/helpers_cmd.rs` — role: CLI handlers; expected change: wire `write_mesh`, add `run_convert`, `run_import` combine.

## Read-Only Context

- `crates/slicer-runtime/src/model_loader.rs` — read only `load_3mf`, `resolve_object`, and the `b"vertex"`/`b"triangle"` parse window (symbol-search to the lines; >1500-line file — never load whole) — purpose: match the reader's element/attribute names so output round-trips.
- `crates/slicer-runtime/src/helpers_cmd.rs` lines `30-160` and `234-496` — purpose: exit-code conventions, `run_repair`/`run_import` shape, `write_stl_binary` template.
- `crates/pnp-cli/src/main.rs` lines `120-189`, `410-440` — purpose: `MeshCmd` enum + dispatch pattern.
- `crates/slicer-ir/src/slice_ir.rs` — read only the `MeshIR`/`ObjectMesh`/`IndexedTriangleSet`/`Point3`/`BoundingBox3` struct defs — purpose: field names. Prefer a delegated FACT.
- `docs/13_slicer_helpers_crate.md` lines around the Import section + 598 — purpose: exit-code table + TASK-060 text.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load.
- `resources/benchy.stl` and any binary mesh fixture — never open (11 MB).
- `crates/slicer-runtime/src/model_loader_sidecar.rs` paint/modifier decode branches — out of scope; only the `PartSubtype::NormalPart == "normal_part"` string is needed (FACT).
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Crates outside the change surface (`slicer-core`, `slicer-schema`, modules/) — delegate any lookup.

## Expected Sub-Agent Dispatches

- "Run `<verification-command>`; return FACT (pass) or SNIPPETS (fail: assertion + ≤ 20 lines)" — purpose: validate each AC step.
- "From `OrcaSlicerDocumented/src/libslic3r/MeshSplitImpl.hpp`, summarize the `create_face_neighbors_index` adjacency test (how two faces are decided to share an edge); return SUMMARY ≤ 150 words, ≤ 1 snippet ≤ 30 lines" — purpose: get the opposite-winding rule for Step 1 without loading the file.
- "From `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp`, return the literal `[Content_Types].xml` and `_rels/.rels` strings OrcaSlicer writes; return SNIPPETS ≤ 30 lines" — purpose: Step 2 OPC plumbing.
- "In `docs/02_ir_schemas.md`, what are the exact field names of `IndexedTriangleSet` and `Point3`? return FACT ≤ 5 lines" — purpose: avoid loading the schema doc.
- "Find all call sites of `write_mesh` in `crates/slicer-runtime`; return LOCATIONS" — purpose: confirm `run_repair`/`run_decimate`/`run_import` are the only callers before changing behavior.

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

## Context Cost Estimate

- Aggregate (sum across all steps): `M`.
- Largest single step: `M` (Step 2, `write_3mf` — zip + XML + sidecar + tests).
- Highest-risk dispatch: the `MeshSplitImpl.hpp` adjacency summary — must be returned as SUMMARY ≤ 150 words + ≤ 1 snippet ≤ 30 lines, or it risks importing a large C++ header into context.

## Open Questions

- None. `[FWD]` The exact `model_settings.config` skeleton fields beyond `name`/`matrix`(identity)/`mesh_stat` (e.g. whether to include `source_file`) are non-blocking: emit `name` + identity `matrix` + empty `mesh_stat` minimum; the implementer may add Orca's `source_*` keys if a delegated read shows the GUI relies on them. Does not affect any AC.
