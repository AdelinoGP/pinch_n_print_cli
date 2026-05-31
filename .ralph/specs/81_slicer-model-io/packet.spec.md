---
status: draft
packet: 81
task_ids: [TASK-231]
requires: []
backlog_source: docs/07_implementation_status.md
---

# Packet 81 — Extract Model I/O into `slicer-model-io`

## Goal

Move `model_loader.rs`, `model_loader_sidecar.rs`, and `model_writer.rs` from `slicer-runtime/src/` into a new `slicer-model-io` crate that exposes `load_mesh(path) -> Result<MeshIR, ModelLoadError>` plus the geometry-only writers, then delete `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` from `slicer-runtime/Cargo.toml`; `pnp-cli` and `slicer-runtime::run::run_slice` consume the new crate so the runtime no longer touches bytes.

## Scope Boundaries

This packet moves three host-side files (~2 900 LOC) into a new leaf crate that depends only on `slicer-ir`. Promotes `model_loader::assemble_object` from `pub(crate)` to `pub` to support the P82 `helpers_cmd` move; promotes `decode_paint_hex_strokes`, `detect_format`, `validate_non_uniform_scale`, `validate_world_z_floor`, and `object_world_z_extent` to `pub` if any caller outside the new crate needs them. The slice entry point in `slicer-runtime::run::run_slice` changes signature to take a constructed `MeshIR` instead of a `&Path`, with the file load happening in `pnp-cli` before invocation. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- None. This packet is the first in the architecture-deepening batch and has no prior-packet dependencies.
- Closure requires `cargo xtask build-guests --check` clean. No WIT is touched, so guests should stay clean without rebuild.

## Acceptance Criteria

### AC-1 — `slicer-model-io` crate exists with the documented public surface and `slicer-ir` as its only first-party dep

**Given** the extraction,
**When** the workspace is inspected,
**Then** `crates/slicer-model-io/Cargo.toml` exists and declares `slicer-ir = { path = "../slicer-ir" }`. It does NOT declare any path dep on `slicer-runtime`, `slicer-core`, `slicer-helpers`, `slicer-sdk`, `slicer-schema`, or `slicer-wasm-host`. It DOES declare `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` as direct deps. `crates/slicer-model-io/src/lib.rs` re-exports `load_mesh`, `assemble_object`, `detect_format`, `ModelFormat`, `ModelLoadError`, `write_3mf`, `write_obj`, `parse_3mf_sidecar`, `ObjectSidecarInfo`, `PartSubtype`.

| `test -f crates/slicer-model-io/Cargo.toml && grep -qE '^slicer-ir = \{ *path = "\.\./slicer-ir"' crates/slicer-model-io/Cargo.toml && ! grep -qE '^slicer-(runtime\|core\|helpers\|sdk\|schema\|wasm-host) *=' crates/slicer-model-io/Cargo.toml && grep -qE '(stl_io|tobj|zip|quick-xml|uuid)' crates/slicer-model-io/Cargo.toml && grep -qE 'pub use.*\b(load_mesh|assemble_object|detect_format|write_3mf|write_obj|parse_3mf_sidecar)\b' crates/slicer-model-io/src/lib.rs`

### AC-2 — Three loader/writer files no longer exist under `slicer-runtime/src/`; equivalents exist under `slicer-model-io/src/`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs` is true; the equivalent files exist under `crates/slicer-model-io/src/` (file name may be split into `loader.rs`, `sidecar.rs`, `writer.rs` or kept as the originals — either layout is acceptable as long as the public surface from AC-1 is exposed).

| `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs && find crates/slicer-model-io/src -name '*.rs' | xargs grep -lE 'pub fn (load_model|assemble_object|write_3mf|write_obj|parse_3mf_sidecar)' | head -1 | grep -q .`

### AC-3 — `slicer-runtime/Cargo.toml` no longer declares `stl_io`, `tobj`, `zip`, `quick-xml`, or `uuid`

**Given** the dep migration,
**When** `crates/slicer-runtime/Cargo.toml` is grepped,
**Then** none of the five direct file-format deps appear in the `[dependencies]` block. `slicer-model-io = { path = "../slicer-model-io" }` does NOT appear either — `slicer-runtime` does not consume the loader; the loader's output (`MeshIR`) is passed in by the caller.

| `! grep -qE '^(stl_io\|tobj\|zip\|quick-xml\|uuid) *=' crates/slicer-runtime/Cargo.toml && ! grep -qE '^slicer-model-io *=' crates/slicer-runtime/Cargo.toml`

### AC-4 — `slicer-runtime::lib.rs` removes the three module declarations and matching re-exports

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the lines `pub mod model_loader;`, `pub mod model_loader_sidecar;`, `pub mod model_writer;` are absent. The `pub use model_writer::{write_3mf, write_obj};` re-export is absent. No `pub use model_loader::*;` re-export remains.

| `! grep -qE '^pub mod (model_loader\|model_loader_sidecar\|model_writer);' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use model_writer::' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use model_loader::' crates/slicer-runtime/src/lib.rs`

### AC-5 — `run_slice` signature consumes `MeshIR`, not a path; `pnp-cli` loads the mesh and passes it in

**Given** the entry-point reshape,
**When** `crates/slicer-runtime/src/run.rs::run_slice` is read,
**Then** its first parameter is a `MeshIR` (or `Arc<MeshIR>`) — NOT `&Path` or `PathBuf`. `crates/pnp-cli/Cargo.toml` declares `slicer-model-io = { path = "../slicer-model-io" }`. `crates/pnp-cli/src/main.rs` (or its `slice` subcommand entry) calls `slicer_model_io::load_mesh(&args.model)` before calling `slicer_runtime::run::run_slice(...)`.

| `grep -E 'pub fn run_slice' crates/slicer-runtime/src/run.rs | head -1 | grep -qE '(MeshIR|Arc<MeshIR>)' && grep -qE '^slicer-model-io *=' crates/pnp-cli/Cargo.toml && grep -rqE 'slicer_model_io::load_mesh' crates/pnp-cli/src/`

### AC-6 — End-to-end slice still works against the canonical fixture, with byte-identical g-code

**Given** the new wiring,
**When** `pnp_cli slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p81.gcode` runs from a clean working tree,
**Then** the command succeeds (exit 0) AND `/tmp/benchy-p81.gcode` is byte-identical to a pre-packet reference output captured before the move (`/tmp/benchy-prep81.gcode` if recorded in the implementation log, OR `cargo run --bin pnp_cli --release -- slice ...` produces a file whose `sha256sum` matches the recorded baseline). The implementation log records the SHA before and after.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p81.gcode`

### AC-7 — `slicer-model-io` carries a round-trip integration test per format (STL, OBJ, 3MF)

**Given** the new crate,
**When** `cargo test -p slicer-model-io` runs,
**Then** three tests under `crates/slicer-model-io/tests/` pass: one loading an STL fixture and asserting non-zero triangle count, one loading an OBJ fixture and asserting non-zero triangle count, one loading the `resources/benchy.stl` mesh and asserting the post-load `MeshIR.objects[0].object_mesh.indexed_triangle_set.indices.len() > 0`. The 3MF writer test writes a single-object MeshIR to a temp 3MF and reloads it, asserting triangle-count parity.

| `cargo test -p slicer-model-io`

### AC-8 — `cargo test -p slicer-runtime` and `cargo test -p pnp-cli` still pass

**Given** the move and signature change,
**When** the narrow per-crate tests run,
**Then** all `slicer-runtime` tests pass with zero regressions vs the pre-packet count (the moved tests, if any, are deleted or migrated to `slicer-model-io/tests/`). `pnp-cli`'s tests pass including any helper-CLI integration tests that exercise the slice entry.

| `cargo test -p slicer-runtime`

## Negative Test Cases

### AC-N1 — `slicer-runtime`'s dep tree contains no transitive reference to `stl_io`, `tobj`, `zip`, `quick-xml`, or `uuid`

**Given** the dep deletion,
**When** `cargo tree -p slicer-runtime --depth 5 --edges normal` is inspected,
**Then** none of the five crates appear in the output. (They may appear elsewhere in the workspace tree as deps of `slicer-model-io`, but `slicer-runtime` itself does not pull them.)

| `! cargo tree -p slicer-runtime --depth 5 --edges normal 2>&1 | grep -qE '\b(stl_io\|tobj\|zip\|quick-xml\|uuid)\b'`

### AC-N2 — `slicer-runtime/src/lib.rs::run_slice` no longer accepts a `&Path` for the mesh input

**Given** the signature change,
**When** an experimental patch reverts `run_slice` to take `&Path` and tries to call `load_mesh` from within the runtime,
**Then** the build fails (because `slicer-model-io` is not a `slicer-runtime` dep). This proves the boundary is enforced by the dep graph, not just by convention. Documented in `implementation-plan.md` step "Verify the seam is enforced by the dep graph".

| (Manual implementer ceremony documented in `implementation-plan.md`. Not CI-gated.)

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-model-io -p slicer-runtime -p pnp-cli`
4. `cargo xtask build-guests --check` (rebuild if STALE — this packet edits no guest-feeding paths, so STALE is unexpected and indicates an unrelated drift)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/01_system_architecture.md` — §pipeline tiers, §data ownership. Read for the seam between "file ingestion" and "host-orchestrated slicing". No change.
- `docs/02_ir_schemas.md` — §`MeshIR` (the value type that crosses the new seam). Read only to confirm the exported handle shape. No change.
- `CONTEXT.md` — §Paint-ready 3MF (the geometry-only writer's contract). No change.
- `CLAUDE.md` §"Build & Test Commands" — confirm `cargo run --bin pnp_cli --release -- slice ...` still matches the new entry shape. No change.

## Doc Impact Statement

No doc files are edited by this packet. `docs/01_system_architecture.md` could grow a one-line crate-map entry for `slicer-model-io`, but that is part of the doc-sweep packet that will close out the deepening batch (P88 or similar; not in scope here).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
