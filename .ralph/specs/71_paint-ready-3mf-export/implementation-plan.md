# Implementation Plan: 71_paint-ready-3mf-export

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-060`.
- TDD first (write the failing test), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are the budget contract.
- Dependency order is fixed: Step 1 (split) → Step 2 (writers) → Step 3 (wire) → Step 4 (convert) → Step 5 (docs).

## Steps

### Step 1: `split_connected_components` (OrcaSlicer `its_split` parity)

- Task IDs:
  - `TASK-060`
- Objective: add `split_connected_components(&IndexedTriangleSet) -> Vec<IndexedTriangleSet>` using edge-shared, opposite-winding adjacency; DFS components; deterministic seed order; per-component vertex remap; no size threshold.
- Precondition: `crates/slicer-helpers/src/repair.rs` component-counting and `IndexedTriangleSet` field names confirmed.
- Postcondition: `split_tdd` green; `split.rs` exported from `lib.rs`.
- Files allowed to read (with line-range hints):
  - `crates/slicer-helpers/src/repair.rs` — lines `140-250` (edge-adjacency reference, deliberately a different criterion)
  - `crates/slicer-helpers/tests/repair_tdd.rs` — `valid_cube`/`single_object_mesh` helper shapes
  - `crates/slicer-ir/src/slice_ir.rs` — `IndexedTriangleSet`/`Point3` defs only
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/split.rs` (new)
  - `crates/slicer-helpers/src/lib.rs`
  - `crates/slicer-helpers/tests/split_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/...` (delegate); `crates/slicer-runtime/**` (later steps)
- Expected sub-agent dispatches:
  - "Summarize `create_face_neighbors_index` adjacency test in `OrcaSlicerDocumented/src/libslic3r/MeshSplitImpl.hpp`; SUMMARY ≤ 150 words + ≤ 1 snippet ≤ 30 lines"
  - "Run `cargo test -p slicer-helpers --test split_tdd`; FACT pass/fail, SNIPPETS ≤ 20 lines on fail"
- Context cost: `M`
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — delegate; helpers crate conventions
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MeshSplitImpl.hpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-helpers --test split_tdd -- split_component_counts --exact` (AC-5)
  - `cargo test -p slicer-helpers --test split_tdd -- split_keeps_tiny_fragment --exact` (AC-N2)
- Exit condition: AC-5 and AC-N2 tests pass; two disjoint tetrahedra → 2, watertight cube → 1, vertex-only contact → 2, tiny fragment retained.

### Step 2: `write_3mf` + `write_obj` in `model_writer.rs`

- Task IDs:
  - `TASK-060`
- Objective: implement geometry-only OrcaSlicer-shaped 3MF writer (OPC package, namespaces, `<object>`/`<build>` per `MeshIR` object, `normal_part` sidecar skeleton, identity transforms, mm vertices, shortest-round-trip floats) and an OBJ writer (`o` groups, per-object 1-based offsets).
- Precondition: Step 1 merged; 3MF reader element/attribute names confirmed from `load_3mf`.
- Postcondition: `model_writer_roundtrip_tdd` round-trip, package, and OBJ tests green; `mod model_writer;` registered.
- Files allowed to read (with line-range hints):
  - `crates/slicer-runtime/src/model_loader.rs` — `load_3mf`/`resolve_object` + `b"vertex"`/`b"triangle"` parse window only (symbol-search; >1500 lines — never load whole)
  - `crates/slicer-runtime/src/helpers_cmd.rs` — lines `459-496` (`write_stl_binary` template)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/model_writer.rs` (new)
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/model_writer_roundtrip_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `model_loader_sidecar.rs` paint/modifier branches (FACT only for the `normal_part` string); `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "Return literal `[Content_Types].xml` + `_rels/.rels` from `OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp`; SNIPPETS ≤ 30 lines"
  - "Summarize the `model_settings.config` `<part>` skeleton OrcaSlicer writes for a normal solid in `bbs_3mf.cpp`; SUMMARY ≤ 150 words"
  - "Run `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd`; FACT pass/fail"
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate FACT for IR field names
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/{bbs_3mf.cpp,3mf.cpp}` — delegate; never load
- Verification:
  - `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- roundtrip_single_solid_exact --exact` (AC-1)
  - `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- threemf_opc_package_and_namespaces --exact` (AC-4)
  - `cargo test -p slicer-runtime --test model_writer_roundtrip_tdd -- obj_geometry_and_object_groups --exact` (AC-6)
- Exit condition: AC-1, AC-4, AC-6 pass; multi-object `MeshIR` round-trips with matching object count, verts/indices, AABB.

### Step 3: Wire `write_mesh` arms + `run_import` 3MF combine

- Task IDs:
  - `TASK-060`
- Objective: replace the `Unsupported` arm with `Obj`/`ThreeMf` dispatch to `model_writer`; update `OutputFormat` doc comment; make `run_import` combine all STEP solids into one `MeshIR` and write a single `.3mf` when format is 3MF and not merging (STL/OBJ keep the `_i` split).
- Precondition: Step 2 merged.
- Postcondition: AC-2 passes; existing import/repair/decimate STL tests still green.
- Files allowed to read:
  - `crates/slicer-runtime/src/helpers_cmd.rs` — lines `234-337`, `440-457`
  - `crates/slicer-runtime/src/cli.rs` — lines `6-20`
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/helpers_cmd.rs`
  - `crates/slicer-runtime/src/cli.rs`
  - `crates/pnp-cli/tests/helpers_cli.rs` (add AC-2 test)
- Files explicitly out-of-bounds for this step:
  - `model_writer.rs` internals (Step 2 done); `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - "Find all call sites of `write_mesh` in `crates/slicer-runtime`; LOCATIONS" — confirm only `run_repair`/`run_decimate`/`run_import`
  - "Run `cargo test -p pnp-cli --test helpers_cli -- import_multi_solid_step_to_single_3mf_two_objects --exact`; FACT pass/fail"
- Context cost: `S`
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — delegate; Import multi-solid rule + exit codes
- OrcaSlicer refs:
  - none (wiring only)
- Verification:
  - `cargo test -p pnp-cli --test helpers_cli -- import_multi_solid_step_to_single_3mf_two_objects --exact` (AC-2)
  - `cargo test -p pnp-cli --test helpers_cli` (regression: existing import/repair/decimate plumbing tests still pass)
- Exit condition: AC-2 passes; `assembly.step → out.3mf` is one file with 2 objects and no `out_0.3mf` is produced.

### Step 4: `pnp_cli mesh convert` verb

- Task IDs:
  - `TASK-060`
- Objective: add `Convert { input, output, format: Option<OutputFormat>, merge_components, repair }` to `MeshCmd` + dispatch; implement `helpers_cmd::run_convert` (load via `load_model`, reject `.step/.stp` → `UNREADABLE` + redirect message, optional `repair`, split each object via `split_connected_components` unless `--merge-components`, write target format once); reuse `resolve_output_format`.
- Precondition: Steps 1–3 merged.
- Postcondition: AC-3 and AC-N1 pass.
- Files allowed to read:
  - `crates/pnp-cli/src/main.rs` — lines `120-189`, `410-440` (enum + dispatch pattern)
  - `crates/slicer-runtime/src/helpers_cmd.rs` — `run_repair` (`30-130`) for exit-code/flow shape
- Files allowed to edit (≤ 3):
  - `crates/pnp-cli/src/main.rs`
  - `crates/slicer-runtime/src/helpers_cmd.rs`
  - `crates/pnp-cli/tests/helpers_cli.rs`
- Files explicitly out-of-bounds for this step:
  - `model_writer.rs`/`split.rs` internals (done); `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - "Run `cargo test -p pnp-cli --test helpers_cli -- convert_split_vs_merge_object_count --exact`; FACT pass/fail"
  - "Run `cargo test -p pnp-cli --test helpers_cli -- convert_rejects_step_input --exact`; FACT pass/fail"
- Context cost: `M`
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — delegate; CLI exit-code conventions
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p pnp-cli --test helpers_cli -- convert_split_vs_merge_object_count --exact` (AC-3)
  - `cargo test -p pnp-cli --test helpers_cli -- convert_rejects_step_input --exact` (AC-N1)
- Exit condition: AC-3 (split→2 objects, merge→1) and AC-N1 (STEP rejected with redirect) pass.

### Step 5: Documentation + close TASK-060

- Task IDs:
  - `TASK-060`
- Objective: document the OBJ/3MF writers, `mesh convert`, and split-to-objects in `docs/13_slicer_helpers_crate.md`; mark TASK-060 done.
- Precondition: Steps 1–4 merged; all ACs green.
- Postcondition: Doc Impact greps in `packet.spec.md` return hits.
- Files allowed to read:
  - `docs/13_slicer_helpers_crate.md` — Import section + line `598` (ranged)
- Files allowed to edit (≤ 3):
  - `docs/13_slicer_helpers_crate.md`
- Files explicitly out-of-bounds for this step:
  - source crates (no code edits in the doc step)
- Expected sub-agent dispatches:
  - "Run `rg -q 'mesh convert' docs/13_slicer_helpers_crate.md && rg -q 'write_3mf' docs/13_slicer_helpers_crate.md`; FACT hit/miss"
- Context cost: `S`
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — edit target
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'mesh convert' docs/13_slicer_helpers_crate.md` (Doc Impact)
  - `rg -q 'TASK-060.*(done|complete)' docs/13_slicer_helpers_crate.md` (closure)
- Exit condition: all three Doc Impact greps in `packet.spec.md` return hits.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | New split utility + TDD; one delegated OrcaSlicer summary |
| Step 2 | M | Largest: zip + XML + sidecar writer + round-trip TDD; two delegated OrcaSlicer reads |
| Step 3 | S | Wiring + one new CLI test; one LOCATIONS dispatch |
| Step 4 | M | New verb across two crates + two CLI tests |
| Step 5 | S | Single doc file |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All steps complete; every step exit condition met.
- Every pipe-suffixed AC command (AC-1..6, AC-N1, AC-N2) dispatched and returned PASS.
- `cargo check --workspace` and `cargo clippy --workspace -- -D warnings` green.
- Doc Impact greps return hits; TASK-060 marked done.
- `docs/07_implementation_status.md` deviation/status note added for the new `mesh convert` + split-to-objects surface (via worker dispatch — never load the full backlog).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (FACT pass/fail each).
- Confirm the gate commands are green.
- Independent parity smoke: open one `write_3mf` output in OrcaSlicer to confirm it loads (manual; record result).
- Record any remaining packet-local risk before moving to `status: implemented`.
- Confirm the implementer's peak context stayed under 70%; if not, log it as a spec-packet-generator lesson.
