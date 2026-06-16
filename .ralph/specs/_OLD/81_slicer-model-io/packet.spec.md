---
status: implemented
packet: 81
task_ids: [TASK-233]
requires: []
backlog_source: docs/07_implementation_status.md
---

# Packet 81 — Extract Model I/O into `slicer-model-io`

## Goal

Move `model_loader.rs`, `model_loader_sidecar.rs`, and `model_writer.rs` from `slicer-runtime/src/` into a new `slicer-model-io` crate that exposes `load_model(path) -> Result<MeshIR, ModelLoadError>` plus the geometry-only writers, then delete `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` from `slicer-runtime/Cargo.toml`; `pnp-cli` and `slicer-runtime::run::run_slice` consume the new crate so the runtime no longer touches bytes.

## Scope Boundaries

This packet moves three host-side files (~2 900 LOC) into a new leaf crate that depends only on `slicer-ir`. Promotes `model_loader::assemble_object` from `pub(crate)` to `pub` to support the P82 `helpers_cmd` move; promotes `decode_paint_hex_strokes`, `detect_format`, `validate_non_uniform_scale`, `validate_world_z_floor`, and `object_world_z_extent` to `pub` if any caller outside the new crate needs them. The slice entry point in `slicer-runtime::run::run_slice` changes signature to take a constructed `MeshIR` instead of a `&Path`, with the file load happening in `pnp-cli` before invocation. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- None. This packet is the first in the architecture-deepening batch and has no prior-packet dependencies.
- Closure requires `cargo xtask build-guests --check` clean. No WIT is touched, so guests should stay clean without rebuild.

## Acceptance Criteria

### AC-1 — `slicer-model-io` crate exists with the documented public surface and `slicer-ir` as its only first-party dep

**Given** the extraction,
**When** the workspace is inspected,
**Then** `crates/slicer-model-io/Cargo.toml` exists and declares `slicer-ir = { path = "../slicer-ir" }`. It does NOT declare any path dep on `slicer-runtime`, `slicer-core`, `slicer-helpers`, `slicer-sdk`, `slicer-schema`, or `slicer-wasm-host`. It DOES declare `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` as direct deps. `crates/slicer-model-io/src/lib.rs` re-exports `load_model`, `assemble_object`, `detect_format`, `ModelFormat`, `ModelLoadError`, `write_3mf`, `write_obj`, `parse_3mf_sidecar`, `ObjectSidecarInfo`, `PartSubtype`.

| `test -f crates/slicer-model-io/Cargo.toml && grep -qE '^slicer-ir = \{ *path = "\.\./slicer-ir"' crates/slicer-model-io/Cargo.toml && ! grep -qE '^slicer-(runtime|core|helpers|sdk|schema|wasm-host) *=' crates/slicer-model-io/Cargo.toml && grep -qE '(stl_io|tobj|zip|quick-xml|uuid)' crates/slicer-model-io/Cargo.toml && grep -qE 'pub use.*\b(load_model|assemble_object|detect_format|write_3mf|write_obj|parse_3mf_sidecar)\b' crates/slicer-model-io/src/lib.rs`

### AC-2 — Three loader/writer files no longer exist under `slicer-runtime/src/`; equivalents exist under `slicer-model-io/src/`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs` is true; the equivalent files exist under `crates/slicer-model-io/src/` (file name may be split into `loader.rs`, `sidecar.rs`, `writer.rs` or kept as the originals — either layout is acceptable as long as the public surface from AC-1 is exposed).

| `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs && find crates/slicer-model-io/src -name '*.rs' | xargs grep -lE 'pub fn (load_model|assemble_object|write_3mf|write_obj|parse_3mf_sidecar)' | head -1 | grep -q .`

### AC-3 — `slicer-runtime/Cargo.toml` no longer declares `stl_io`, `tobj`, `zip`, `quick-xml`, or `uuid`

**Given** the dep migration,
**When** `crates/slicer-runtime/Cargo.toml` is grepped,
**Then** none of the five direct file-format deps appear in the `[dependencies]` block. `slicer-model-io = { path = "../slicer-model-io" }` does NOT appear either — `slicer-runtime` does not consume the loader; the loader's output (`MeshIR`) is passed in by the caller.

| `! grep -qE '^(stl_io|tobj|zip|quick-xml|uuid) *=' crates/slicer-runtime/Cargo.toml && ! grep -qE '^slicer-model-io *=' crates/slicer-runtime/Cargo.toml`

### AC-4 — `slicer-runtime::lib.rs` removes the three module declarations and matching re-exports

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the lines `pub mod model_loader;`, `pub mod model_loader_sidecar;`, `pub mod model_writer;` are absent. The `pub use model_writer::{write_3mf, write_obj};` re-export is absent. No `pub use model_loader::*;` re-export remains.

| `! grep -qE '^pub mod (model_loader|model_loader_sidecar|model_writer);' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use model_writer::' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use model_loader::' crates/slicer-runtime/src/lib.rs`

### AC-5 — `run_slice` consumes a pre-loaded `MeshIR`, not a path; `pnp-cli` loads the mesh and passes it in

**Given** the entry-point reshape,
**When** `crates/slicer-runtime/src/run.rs::run_slice` and `crates/slicer-runtime/src/cli.rs::SliceRunOptions` are read,
**Then** `SliceRunOptions` (declared at `crates/slicer-runtime/src/cli.rs:251`) NO LONGER carries a `model_path: PathBuf` field — that field is replaced by `mesh: Arc<MeshIR>` (other fields unchanged). `crates/pnp-cli/Cargo.toml` declares `slicer-model-io = { path = "../slicer-model-io" }`. `crates/pnp-cli/src/main.rs` (or its `slice` subcommand entry) calls `slicer_model_io::load_model(&args.model)` to construct the `MeshIR` before populating `SliceRunOptions` and calling `slicer_runtime::run::run_slice(...)`.

| `! grep -qE '^[[:space:]]*pub model_path: PathBuf' crates/slicer-runtime/src/cli.rs && grep -qE '^[[:space:]]*pub mesh:[[:space:]]+Arc<MeshIR>' crates/slicer-runtime/src/cli.rs && grep -qE '^slicer-model-io *=' crates/pnp-cli/Cargo.toml && grep -rqE 'slicer_model_io::load_model' crates/pnp-cli/src/`

### AC-6 — End-to-end slice still works against the canonical fixture, with byte-identical g-code

**Given** the new wiring AND a baseline SHA file `/tmp/p81-baseline.sha` captured in Step 0 (single 64-hex line, no filename suffix),
**When** the same slice runs against `resources/benchy.stl` from a clean working tree,
**Then** the command succeeds (exit 0) AND `sha256sum /tmp/benchy-p81.gcode` produces the same hex digest as the recorded baseline. The runnable command computes the post-packet hex digest and `diff`s it against the baseline — non-empty diff = AC fail. The implementation log records both digests.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p81.gcode && sha256sum /tmp/benchy-p81.gcode | cut -d' ' -f1 > /tmp/p81-post.sha && diff /tmp/p81-baseline.sha /tmp/p81-post.sha`

### AC-7 — `slicer-model-io` carries a round-trip integration test per format (STL, OBJ, 3MF)

**Given** the new crate,
**When** `cargo test -p slicer-model-io` runs,
**Then** three tests under `crates/slicer-model-io/tests/` pass: one loading an STL fixture and asserting non-zero triangle count, one loading an OBJ fixture and asserting non-zero triangle count, one loading the `resources/benchy.stl` mesh and asserting the post-load `mesh_ir.objects[0].mesh.indices.len() > 0` (field chain: `MeshIR.objects: Vec<ObjectMesh>` → `ObjectMesh.mesh: IndexedTriangleSet` → `IndexedTriangleSet.indices: Vec<u32>`, per `crates/slicer-ir/src/slice_ir.rs:432,402,150`). The 3MF writer test writes a single-object MeshIR to a temp 3MF and reloads it, asserting triangle-count parity.

| `cargo test -p slicer-model-io`

### AC-8 — `cargo test -p slicer-runtime` and `cargo test -p pnp-cli` still pass

**Given** the move and signature change,
**When** the narrow per-crate tests run,
**Then** all `slicer-runtime` tests pass with zero regressions vs the pre-packet count (the moved tests, if any, are deleted or migrated to `slicer-model-io/tests/`). `pnp-cli`'s tests pass including any helper-CLI integration tests that exercise the slice entry.

| `cargo test -p slicer-runtime`

## Negative Test Cases

### AC-N1 — `slicer-runtime` declares no direct (normal-edge) dep on `stl_io`, `tobj`, `zip`, `quick-xml`, or `uuid`

**Given** the dep deletion,
**When** `cargo tree -p slicer-runtime --depth 1 --edges normal` is inspected (direct-deps only),
**Then** none of the five crates appear in the output. The grep is narrowed from full-transitive (depth 5) to direct-deps (depth 1) per the P81 closure deviation log: `quick-xml v0.22.0` is a pre-existing transitive via `slicer-helpers → truck-meshalgo → vtkio`, and `uuid v1.23.0` is a pre-existing transitive via `wasmtime → fxprof-processed-profile → debugid`. Neither was introduced by P81 nor relates to file ingestion. The architectural seam this AC enforces (no model-ingestion crates declared by `slicer-runtime`) is satisfied at the direct-dep boundary.

| `! cargo tree -p slicer-runtime --depth 1 --edges normal 2>&1 | grep -qE '\b(stl_io|tobj|zip|quick-xml|uuid)\b'`

### AC-N2 — The seam is enforced by the dep graph: `slicer-runtime` has no dep edge to `slicer-model-io`

**Given** the dep graph is the structural enforcement of the seam (a hypothetical patch reverting `run_slice` to take `&Path` and calling `load_model` from within `slicer-runtime` would only compile if `slicer-runtime` gained a `slicer-model-io` dep — which the dep graph forbids),
**When** `cargo tree -p slicer-runtime --depth 5 --edges normal` is grepped for `slicer-model-io`,
**Then** the match is empty. This is a sibling check to AC-N1: AC-N1 proves the five file-format crates are gone; AC-N2 proves the loader crate itself never re-enters via a backdoor `slicer-runtime` dep.

| `! cargo tree -p slicer-runtime --depth 5 --edges normal 2>&1 | grep -qE '\bslicer-model-io\b'`

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

`none` — pure crate-move refactor. Rationale: this packet preserves every existing public function signature (no IR field, WIT type, scheduler rule, claim ID, manifest schema, host service shape, or module SDK contract is renamed, retyped, or restructured); it only relocates the `model_loader`/`model_loader_sidecar`/`model_writer` files into a new leaf crate and replaces a `PathBuf` field with `Arc<MeshIR>` inside `SliceRunOptions` (caller-visible only). Verified: `docs/01_system_architecture.md` does NOT currently maintain a workspace crate-map table (grep confirmed `slicer-core`, `slicer-helpers`, `slicer-ir` are absent), so no existing crate-inventory section requires the new `slicer-model-io` entry. The `run_slice` signature is not enumerated in any authoritative doc (grepped `pub fn run_slice` in `docs/` → zero hits). If a workspace crate-map is later added to `docs/01_system_architecture.md`, that work belongs to the deepening-batch doc-sweep, not this packet.

## Deviations (recorded at closure)

All five deviations below were surfaced by the swarm implementation, reviewed by the user, and accepted before the status flip from `draft` → `implemented`.

### D-1 — `helpers_cmd.rs` moved to `pnp-cli` (P82 scope pulled forward)

**Original packet position.** §"In Scope" item: *"Update `crates/slicer-runtime/src/helpers_cmd.rs` to import from `slicer_model_io::`. Packet 82 then moves this file into pnp-cli; this packet just rewires the imports in place."*

**Conflict surfaced during implementation.** The in-place import rewrite (`use crate::model_loader::...` → `use slicer_model_io::...`) would have required `slicer-runtime` to declare `slicer-model-io` as a normal (non-dev) dep — which AC-N2 forbids. The packet design.md did not anticipate this collision.

**Resolution.** `git mv crates/slicer-runtime/src/helpers_cmd.rs → crates/pnp-cli/src/helpers_cmd.rs`. This is the work P82 (`requires: [81]`, TASK-232) was scheduled to do; pulling it forward here is the only path that satisfies AC-N2 cleanly.

**Consequence for P82.** Scope shrinks to (a) `cli.rs` deletion and (b) report feature-gating. The packet's `requirements.md` has been annotated to record that `helpers_cmd.rs` is already moved by P81.

### D-2 — AC-N1 verification command narrowed from `--depth 5` to `--depth 1`

**Original packet position.** AC-N1's grep was `cargo tree -p slicer-runtime --depth 5 --edges normal | grep -qE '\b(stl_io|tobj|zip|quick-xml|uuid)\b'` — i.e., absence of the five file-format crates anywhere in the full transitive tree.

**Conflict surfaced during implementation.** After the direct deps were deleted, two pre-existing transitives became visible:

- `quick-xml v0.22.0` ← `vtkio v0.6.3` ← `truck-meshalgo v0.4.0` ← `slicer-helpers v0.1.0` ← `slicer-runtime` (mesh-algorithm subsystem, predates P81)
- `uuid v1.23.0` ← `debugid v0.8.0` ← `fxprof-processed-profile v0.8.1` ← `wasmtime v43.0.1` ← `slicer-runtime` (profiler debug-id, predates P81)

Both predate P81; neither relates to file ingestion. Eliminating them would require dropping `wasmtime` or `truck-meshalgo` — far out of scope.

**Resolution.** The AC's grep is narrowed to direct (depth-1) deps only. AC-N1 now PASSes: `slicer-runtime`'s direct deps no longer list any of the five file-format crates. The architectural seam the AC enforces (no model-ingestion crates declared by the orchestrator) is preserved; the wording is matched to the stated intent rather than broadened to an unsatisfiable literal.

**`stl_io`, `tobj`, `zip` are absent from the tree entirely**; only `quick-xml` and `uuid` retain pre-existing transitive paths through wholly unrelated subsystems.

### D-3 — `SliceRunOptions` gained a `model_label: String` field

**Original packet position.** AC-5: *"`SliceRunOptions` … carries `pub mesh: Arc<MeshIR>` in place of `pub model_path: PathBuf` (other fields unchanged)."*

**Conflict surfaced during implementation.** `run.rs:134` previously fed `opts.model_path.to_string_lossy().to_string()` into the HTML report's `Collector` constructor as the model display label. After `model_path` was removed, that information had nowhere to live.

**Resolution.** A new `pub model_label: String` field was added to `SliceRunOptions` alongside `mesh: Arc<MeshIR>`. `pnp-cli` populates it from the user-supplied path before invoking `run_slice`. AC-5's grep still PASSes (the verification only checks for absence of `model_path: PathBuf` and presence of `mesh: Arc<MeshIR>`, not absence of new fields). The HTML report's behavior is preserved verbatim.

**Rationale.** Presentation strings don't belong in `MeshIR`. `SliceRunOptions` is the orchestrator's input bag; carrying a display label there is the correct seam.

### D-4 — `slicer-runtime/Cargo.toml` gained `[dev-dependencies]` entries

`slicer-model-io`, `zip`, and `stl_io` were added to `slicer-runtime`'s `[dev-dependencies]` (NOT `[dependencies]`):

- `slicer-model-io` — used by ~10 test files under `slicer-runtime/tests/` that call `load_model` as a fixture-builder
- `zip`, `stl_io` — used directly by two existing tests (`threemf_transform_tdd.rs`, `slicing_promotion_e2e_dispatch_regression_tdd.rs`) as fixture builders; previously transitively available via the now-deleted `model_loader` module

AC-N2's `cargo tree -p slicer-runtime --edges normal` excludes dev-dependencies, so the architectural seam remains enforced.

**Future cleanup (non-blocking).** If those two tests' SUT is genuinely loader behavior rather than runtime behavior, they could be migrated to `slicer-model-io/tests/` in a follow-up — at which point the `zip`/`stl_io` dev-deps could be dropped from `slicer-runtime`.

### D-5 — `docs/07_implementation_status.md` update bundled with status flip

Per the closure sequence, the TASK-233 backlog row is added (status `[x]`, closed by P81) at the same moment the packet's status moves `draft` → `implemented`. This is procedural rather than a scope change.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
