# ADR-0017 — File-Format I/O Lives in `slicer-model-io`, Not `slicer-runtime`

## Status

Accepted (Packet 81 / TASK-231).

## Context

Before Packet 81, all model-format I/O (STL parsing via `stl_io`, OBJ parsing via `tobj`, 3MF unpacking via `zip` + `quick-xml`, UUID generation via `uuid`) lived inside `slicer-runtime`. The runtime crate accepted a `PathBuf` for the input model, opened the file, parsed it, and produced `MeshIR` inline as part of the `run_slice` entry point.

This had two visible costs:

1. **Heavy dep graph.** `slicer-runtime` carried `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` directly. Every downstream consumer of `slicer-runtime` (CLI, future GUI, future server embedding) inherited the file-format crates whether they used them or not.
2. **Tightly coupled entry point.** `SliceRunOptions.model_path: PathBuf` meant the orchestrator did file I/O. A future GUI front-end that wanted to feed pre-parsed `MeshIR` to the slice pipeline had no clean seam to do so — it would have to write a temporary file, point the runtime at it, and accept the redundant parse cost.

Packet 81 broke the boundary cleanly. The decision is larger than a one-time refactor: it establishes which layer owns file I/O for the indefinite future.

## Decision

**File-format I/O lives in `slicer-model-io`, a leaf crate whose only first-party dep is `slicer-ir`. `slicer-runtime` no longer carries any file-format dep, and `run_slice` takes a pre-loaded `Arc<MeshIR>`, not a path.**

Concretely:

- **`crates/slicer-model-io/`** owns STL / OBJ / 3MF read, the 3MF sidecar parser, `assemble_object` (`ObjectMesh` construction + Z-extent), and the 3MF/OBJ writer paths. Direct deps: `stl_io`, `tobj`, `zip`, `quick-xml`, `uuid`, `slicer-ir`, `slicer-helpers`.
- **`crates/slicer-runtime/`** declares zero of those file-format crates. Verify with `cargo tree -p slicer-runtime --edges normal --depth 5 | grep -E '(stl_io|tobj|^├── zip|quick-xml|^├── uuid)'` (must be empty).
- **`SliceRunOptions.mesh: Arc<MeshIR>`** replaces the previous `model_path: PathBuf`. A companion `model_label: String` carries the display label for HTML reports (a presentation concern that does not belong on `MeshIR` itself).
- **`pnp-cli`** is now responsible for: (a) determining the format, (b) calling the corresponding `slicer-model-io` loader, (c) constructing `SliceRunOptions { mesh, model_label, ... }`, (d) handing the options to `run_slice`.
- **`slicer-runtime` does not depend on `slicer-model-io`.** The forward edge is `pnp-cli → slicer-model-io → slicer-ir`; `slicer-runtime` sits on a different branch consuming only `slicer-ir`. The runtime cannot accidentally re-add file I/O without an explicit Cargo.toml edit that fails the AC-N2 check.

## Consequences

- **Future embeddings stay clean.** A GUI front-end that already has a parsed mesh hands it to `run_slice` directly without involving the file system. A future REST/RPC server can accept a binary `MeshIR` payload (or a path it loaded itself) without paying the full I/O dep cost.
- **`slicer-runtime` builds faster.** Dropping `zip`, `quick-xml`, and `stl_io` removes several seconds from cold rebuilds of the runtime.
- **CLI complexity grows slightly.** `pnp-cli` gains a small format-detection + load step in `main.rs`. Acceptable trade-off — the CLI was the natural place for this logic anyway.
- **`write_with_parents` and `OutputFormat` moved with the loader.** Helpers that were previously private to `slicer-runtime::cli` are now public in `pnp_cli::io`. They handle parent-directory creation for `--output` and `--report` paths (Packet 65 surface).
- **Public API surface widened, deliberately.** `assemble_object` was promoted from `pub(crate)` to `pub` in Packet 81 so `pnp_cli::helpers_cmd` can call it. Other internal helpers that future tooling needs may follow.
- **`helpers_cmd.rs` moved early.** Originally scoped to Packet 82, the helpers-command module had to move into `pnp-cli` during 81 to compile against the new public surface. DEV-001 in the packet's deviation block (and `DEVIATION_LOG.md`) records the cross-packet bleed.

## Rejected alternatives

- **Keep file I/O in `slicer-runtime` and add a parallel `mesh: Option<Arc<MeshIR>>` entry-point.** Two ways to do the same thing; deps still leak. Rejected.
- **Put file I/O in a new module inside `slicer-runtime` but feature-gate the deps.** Conditional compilation across the runtime API surface is hard to reason about; downstream embedders would have to know which feature flags to disable. Rejected.
- **Inline format-specific helpers into `slicer-ir`.** The IR crate must stay dependency-light; pulling `zip` and `quick-xml` into it is a worse outcome than leaving them in `slicer-runtime`. Rejected.

## Future reviewers

- Do not add file-format crates to `slicer-runtime`. Verify before commit with `cargo tree -p slicer-runtime --edges normal | grep <crate>`.
- Do not make `slicer-runtime` depend on `slicer-model-io`. The forward-edge direction is `pnp-cli → slicer-model-io`; backwards is forbidden.
- Future loaders (e.g. STEP, 3MX, PLY) go in `slicer-model-io`. New writer formats follow the same rule.
- If a future embedding cannot use `pnp-cli`'s loading path, that's a sign that `slicer-model-io`'s API needs to surface as a library — not that file I/O should re-enter `slicer-runtime`.
