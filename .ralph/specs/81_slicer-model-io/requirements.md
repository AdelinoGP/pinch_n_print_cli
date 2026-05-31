# Packet 81 — Requirements

## Problem Statement

`slicer-runtime` today carries the entire host-side file-format ingestion: ~2 900 LOC of STL/OBJ/3MF parsing, 3MF sidecar interpretation, and geometry-only 3MF writing, plus the five external dependencies (`stl_io`, `tobj`, `zip`, `quick-xml`, `uuid`) that exist only to support them. Three structural consequences hurt:

1. **Boundary blur** — `slicer-runtime` is a host orchestrator; bytes-to-`MeshIR` translation is a boundary concern with no relationship to the slicing pipeline. Co-locating them confuses what `slicer-runtime` is for.
2. **Test fan-out** — anyone writing a parser bug test must spin up the full runtime; format-parsing fixtures cannot be exercised against bytes alone.
3. **Build cost everywhere** — every consumer of `slicer-runtime` transitively pays for `zip`, `quick-xml`, and friends, including future embeddings that may not load files (e.g., a GUI front-end that already has a `MeshIR` in memory).

A separate `slicer-model-io` crate gives format I/O the boundary module it deserves, deletes five deps from `slicer-runtime`'s Cargo.toml, and lets `pnp-cli` and any future caller load meshes without dragging the orchestrator along.

## Grouped Task IDs

- **TASK-231** (new) — Extract model I/O into `slicer-model-io`. Recorded under "Architecture Deepening Phase I" in the next `docs/07_implementation_status.md` update; packet 81 establishes the topic.

## In Scope

- New crate `crates/slicer-model-io/` with `Cargo.toml` declaring `slicer-ir` (path) plus the five direct file-format deps.
- Move `crates/slicer-runtime/src/model_loader.rs` (2 439 LOC), `crates/slicer-runtime/src/model_loader_sidecar.rs` (253 LOC), `crates/slicer-runtime/src/model_writer.rs` (194 LOC) into `crates/slicer-model-io/src/`. File names may be flattened (e.g., `loader.rs`, `sidecar.rs`, `writer.rs`); content is moved, not rewritten.
- Promote `model_loader::assemble_object` from `pub(crate)` to `pub` (required by packet 82's `helpers_cmd` consumer, but the promotion lands here so packet 82 doesn't need to touch the crate again).
- Promote any other `pub(crate)` items that have external callers today (specifically: items used by `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/helpers_cmd.rs`, and `crates/slicer-runtime/tests/`).
- Update `slicer-runtime/src/lib.rs` to drop `pub mod model_loader; pub mod model_loader_sidecar; pub mod model_writer;` and the `pub use model_writer::{write_3mf, write_obj};` re-export.
- Delete `stl_io`, `tobj`, `zip`, `quick-xml`, and `uuid` from `crates/slicer-runtime/Cargo.toml`.
- Reshape `crates/slicer-runtime/src/run.rs::run_slice` so its first parameter is a constructed `MeshIR` (or `Arc<MeshIR>`), not `&Path`. The file-loading step moves to `crates/pnp-cli/src/main.rs` (or the file from which `slice` is dispatched).
- Add `slicer-model-io = { path = "../slicer-model-io" }` to `crates/pnp-cli/Cargo.toml`.
- Update `crates/pnp-cli/src/main.rs` (or its slice subcommand) to call `slicer_model_io::load_mesh(&args.model)` before `slicer_runtime::run::run_slice`.
- Update `crates/slicer-runtime/src/helpers_cmd.rs` to import from `slicer_model_io::` (it consumes `assemble_object` and `load_model`). Packet 82 then moves `helpers_cmd.rs` into `pnp-cli`; this packet just rewires the imports in place.
- Migrate test files that exercise loader/writer behavior (those whose SUT is `load_model`, `assemble_object`, `parse_3mf_sidecar`, `write_3mf`, or `write_obj`) into `crates/slicer-model-io/tests/`. Tests whose SUT is a runtime symbol but which use `load_model` only as a fixture builder stay in `slicer-runtime/tests/` and update their imports to `slicer_model_io::load_model`.

## Out of Scope

- Touching `crates/slicer-test/` or `crates/slicer-sdk/` — concurrent work is folding the former into the latter (see packet 78).
- WIT contract changes (`crates/slicer-schema/wit/**`). None are needed.
- Any change to `crates/slicer-runtime/src/` beyond removing the three modules, updating `lib.rs`, rewiring `run.rs` and `helpers_cmd.rs` imports, and updating `Cargo.toml`.
- Adding a new public surface to `slicer-model-io` beyond what the move requires. No invention; no abstraction; preserve the existing function shapes.
- Moving the `pnp-cli` CLI parser or `helpers_cmd.rs` itself — those are packet 82.
- Re-exporting `slicer-model-io` types from `slicer-runtime` or `slicer-sdk`. Consumers depend on the new crate directly.
- Documenting the new crate in `docs/01_system_architecture.md` — that lands in the deepening batch's doc-sweep packet.

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers, host-side data ownership. Read only the §"Module search path / file ingestion" and §"Data ownership" sections; if either exceeds 200 lines, delegate a SUMMARY return.
- `docs/02_ir_schemas.md` — `MeshIR` schema (the value that crosses the new seam). Read only the `MeshIR` section; ≤ 80 lines typically.
- `CONTEXT.md` — Paint-ready 3MF definition (the writer's output contract). Full file; ~100 lines.
- `CLAUDE.md` — §"Build & Test Commands" for the canonical `pnp_cli slice ...` invocation.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-8, AC-N1, AC-N2). Measurable refinements that did not fit Given/When/Then:

- **AC-1 — Dep declaration shape**: `slicer-model-io/Cargo.toml`'s `[dependencies]` block must list exactly these crates (workspace inheritance allowed): `slicer-ir`, `stl_io`, `tobj`, `zip` (with `default-features = false, features = ["deflate"]`), `quick-xml`, `uuid` (with `features = ["v4", "v5"]`), plus whatever else the moved files import (likely `nalgebra`, `serde`). No `wasmtime`, no `pyo3`.
- **AC-6 — Byte-identical g-code baseline**: capture the SHA of `pnp_cli slice ... resources/benchy.stl` BEFORE moving anything (the implementer's first step in `implementation-plan.md` step 0). After the move, the same invocation produces a file with the same SHA. Any divergence is a regression.
- **AC-7 — Per-format test fixtures**: `crates/slicer-model-io/tests/` must include at least one STL fixture (binary or ASCII), one OBJ fixture, one 3MF roundtrip. Fixtures can be borrowed from `resources/` or constructed inline; total fixture size ≤ 50 KB to keep cargo test fast.

## Verification Commands

Full matrix. Each command is delegation-friendly (exit-coded, parseable, or returns ≤ 1 line on success).

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test -f crates/slicer-model-io/Cargo.toml && grep -qE '^slicer-ir = \{ *path = "\.\./slicer-ir"' crates/slicer-model-io/Cargo.toml && ! grep -qE '^slicer-(runtime\|core\|helpers\|sdk\|schema\|wasm-host) *=' crates/slicer-model-io/Cargo.toml` | FACT pass/fail |
| AC-2 | `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs` | FACT pass/fail |
| AC-3 | `! grep -qE '^(stl_io\|tobj\|zip\|quick-xml\|uuid) *=' crates/slicer-runtime/Cargo.toml` | FACT pass/fail |
| AC-4 | `! grep -qE '^pub mod (model_loader\|model_loader_sidecar\|model_writer);' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-5 | `grep -E 'pub fn run_slice' crates/slicer-runtime/src/run.rs \| head -1 \| grep -qE '(MeshIR\|Arc<MeshIR>)' && grep -qE '^slicer-model-io' crates/pnp-cli/Cargo.toml` | FACT pass/fail |
| AC-6 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p81.gcode && sha256sum /tmp/benchy-p81.gcode` | SNIPPET (last line of stdout — the SHA — for log) |
| AC-7 | `cargo test -p slicer-model-io` | FACT pass/fail + count |
| AC-8 | `cargo test -p slicer-runtime && cargo test -p pnp-cli` | FACT pass/fail + count |
| AC-N1 | `cargo tree -p slicer-runtime --depth 5 --edges normal 2>&1 \| grep -E '\b(stl_io\|tobj\|zip\|quick-xml\|uuid)\b'` (success = empty output) | FACT empty/non-empty |
| AC-N2 | Manual ceremony — see `implementation-plan.md`. | (not CI) |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests --check` | FACT clean/STALE |

## Step Completion Expectations

Cross-step invariants that the per-step blocks in `implementation-plan.md` cannot express:

- The pre-move `sha256sum` of `pnp_cli slice resources/benchy.stl` MUST be captured in the implementation log before any source file is touched. Without this baseline, AC-6 cannot be falsified.
- No commit may leave `slicer-runtime/src/lib.rs` referencing a `model_loader*` module that has already been moved — i.e., the lib.rs edits and the file moves land together (one commit or sequential commits with the intermediate state never built).

## Packet-Specific Context Discipline

- `crates/slicer-runtime/src/model_loader.rs` is 2 439 LOC. **Do not load it in full.** When inspecting, delegate a SUMMARY of the public surface, or read with line-range hints (≤ 200 lines at a time).
- `OrcaSlicerDocumented/` is irrelevant to this packet — none of the moved files port OrcaSlicer behavior. Do not consult it.
