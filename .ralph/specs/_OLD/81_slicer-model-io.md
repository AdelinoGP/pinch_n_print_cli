---
status: implemented
packet: 81
task_ids: [TASK-233]
---

# 81_slicer-model-io

## Goal

Move `model_loader.rs`, `model_loader_sidecar.rs`, and `model_writer.rs` from `slicer-runtime/src/` into a new `slicer-model-io` crate that exposes `load_model(path) -> Result<MeshIR, ModelLoadError>` plus the geometry-only writers, then delete `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` from `slicer-runtime/Cargo.toml`; `pnp-cli` and `slicer-runtime::run::run_slice` consume the new crate so the runtime no longer touches bytes.

## Problem Statement

`slicer-runtime` today carries the entire host-side file-format ingestion: ~2 900 LOC of STL/OBJ/3MF parsing, 3MF sidecar interpretation, and geometry-only 3MF writing, plus the five external dependencies (`stl_io`, `tobj`, `zip`, `quick-xml`, `uuid`) that exist only to support them. Three structural consequences hurt:

1. **Boundary blur** — `slicer-runtime` is a host orchestrator; bytes-to-`MeshIR` translation is a boundary concern with no relationship to the slicing pipeline. Co-locating them confuses what `slicer-runtime` is for.
2. **Test fan-out** — anyone writing a parser bug test must spin up the full runtime; format-parsing fixtures cannot be exercised against bytes alone.
3. **Build cost everywhere** — every consumer of `slicer-runtime` transitively pays for `zip`, `quick-xml`, and friends, including future embeddings that may not load files (e.g., a GUI front-end that already has a `MeshIR` in memory).

A separate `slicer-model-io` crate gives format I/O the boundary module it deserves, deletes five deps from `slicer-runtime`'s Cargo.toml, and lets `pnp-cli` and any future caller load meshes without dragging the orchestrator along.

## Architecture Constraints

- ADR-0001 (built-in commits stay in-stage), ADR-0002 (host `bindgen!` `with:` remap), and ADR-0003 (no guest-side WIT conversion crate) are all preserved trivially — none of the moved files touch WIT, bindgen, or the built-in producer machinery.
- `slicer-model-io` MUST NOT depend on `slicer-runtime`, `slicer-core`, `slicer-helpers`, `slicer-sdk`, `slicer-schema`, or `slicer-wasm-host` (the latter does not exist yet; this packet does not introduce it). Its only first-party dep is `slicer-ir`.
- `slicer-runtime` MUST NOT depend on `slicer-model-io`. The seam exists precisely so `slicer-runtime` can be embedded in callers that never read a file (e.g., a future GUI that hands over an in-memory mesh).
- `pnp-cli` gains a `slicer-model-io` dep. This is the only direction the dep graph moves.
- Config keys are unchanged; none of the moved files declare or read config.

## Data and Contract Notes

- `MeshIR` is consumed as a value (`Arc<MeshIR>` works; bare `MeshIR` works). The slicer pipeline already holds `Arc<MeshIR>` internally (`crates/slicer-runtime/src/blackboard.rs:59` — `mesh_ir: Arc<MeshIR>`). The slice entry can take either; choose `Arc<MeshIR>` for symmetry with the blackboard.
- `ModelLoadError` is preserved verbatim — no error-shape change. Callers (`pnp-cli`'s slice subcommand) propagate it via `?` as today.
- `assemble_object` becomes `pub` — promoted, not redesigned. Its signature is unchanged.
- The `slicer-model-io` crate name is unprefixed (no `pnp-` or `modular-`); this matches the existing workspace naming (`slicer-core`, `slicer-helpers`, etc.).

## Locked Assumptions and Invariants

- No change in g-code output: AC-6 enforces byte-identical output via SHA comparison on `resources/benchy.stl`. If the SHA diverges, the packet fails closure.
- No change in error reporting: `ModelLoadError` variants are preserved; the `pnp-cli` error path uses the same `Display` impls.
- No change in WIT contracts: nothing in this packet touches `crates/slicer-schema/wit/**` (snippet `wasm-staleness` is intentionally NOT included; the change surface does not feed guest builds).
- The runtime's blackboard still receives `Arc<MeshIR>` exactly as today; only the construction site moves out.

## Risks and Tradeoffs

- **Risk: hidden `pub(crate)` calls.** The 2 439-LOC `model_loader.rs` may have callers (test or module) we miss. Mitigation: dispatch #1 in the table above enumerates them; the implementer reads the LOCATIONS and promotes accordingly. If a missed caller surfaces at `cargo build`, the fix is a single `pub` flip.
- **Risk: `run_slice` callers outside `pnp-cli`.** Tests and benches may construct `SliceRunOptions` with the old `model_path` field. Mitigation: after the field rename (`model_path: PathBuf` → `mesh: Arc<MeshIR>`), `cargo build --workspace` surfaces every caller. Each caller adds a `slicer_model_io::load_model(&path).map(Arc::new)` line before populating `SliceRunOptions`. Mechanical.
- **Tradeoff: `pnp-cli` grows.** Its `Cargo.toml` gains one dep and `main.rs` gains one line. This is the intended direction — the binary is the place to read files.
- **Tradeoff: `slicer-model-io` is dep-heavy.** It carries the five format deps. That's by design — keeping them off `slicer-runtime` is the win.
