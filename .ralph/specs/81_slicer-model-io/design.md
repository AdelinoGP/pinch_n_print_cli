# Packet 81 — Design

## Controlling Code Paths

The change surface is bounded by three host-side files plus their consumers. After the move, the dep graph layer for file ingestion becomes:

```
slicer-ir  ◄── slicer-model-io  ◄── pnp-cli  ──► slicer-runtime
                                                  ▲
                                                  └── (also reachable from
                                                       helpers_cmd.rs imports,
                                                       updated in this packet)
```

`slicer-runtime` no longer depends on `slicer-model-io`. Its slice entry point receives a pre-loaded `MeshIR`; the path argument disappears from the runtime API.

OrcaSlicer comparison surface: none. The moved files implement format parsers (STL/OBJ/3MF), not slicing algorithms — `OrcaSlicerDocumented/` does not need to be consulted.

## Architecture Constraints

- ADR-0001 (built-in commits stay in-stage), ADR-0002 (host `bindgen!` `with:` remap), and ADR-0003 (no guest-side WIT conversion crate) are all preserved trivially — none of the moved files touch WIT, bindgen, or the built-in producer machinery.
- `slicer-model-io` MUST NOT depend on `slicer-runtime`, `slicer-core`, `slicer-helpers`, `slicer-sdk`, `slicer-schema`, or `slicer-wasm-host` (the latter does not exist yet; this packet does not introduce it). Its only first-party dep is `slicer-ir`.
- `slicer-runtime` MUST NOT depend on `slicer-model-io`. The seam exists precisely so `slicer-runtime` can be embedded in callers that never read a file (e.g., a future GUI that hands over an in-memory mesh).
- `pnp-cli` gains a `slicer-model-io` dep. This is the only direction the dep graph moves.
- Config keys are unchanged; none of the moved files declare or read config.

## Selected Approach

Pure move + dep migration + entry-point reshape. No rewrite, no abstraction, no new public types.

Rejected alternatives:

- **Wrap loaders behind a trait** (`MeshLoader::load(path) -> Result<MeshIR, _>`). Rejected: one production implementation, one test fixture path, no second adapter justifies a trait. Two adapters would justify a seam; one does not. Add the trait later if a streaming or in-memory loader appears.
- **Keep `slicer-runtime` as a thin re-exporter of `slicer-model-io`**. Rejected: defeats the whole purpose of deleting the five file-format deps from `slicer-runtime`. The dep tree gain (AC-N1) is the win.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-model-io/Cargo.toml` | **CREATE** | `[package] name = "slicer-model-io"`; deps as listed in `requirements.md` §Acceptance Summary AC-1. |
| `crates/slicer-model-io/src/lib.rs` | **CREATE** | `pub mod loader; pub mod sidecar; pub mod writer;` + matching `pub use` re-exports for the documented surface. |
| `crates/slicer-model-io/src/loader.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/model_loader.rs` verbatim except `use crate::model_loader_sidecar::*` → `use crate::sidecar::*`; promote `assemble_object` and any externally-needed `pub(crate)` items to `pub`. |
| `crates/slicer-model-io/src/sidecar.rs` | **CREATE (from move)** | Content of `model_loader_sidecar.rs` verbatim. |
| `crates/slicer-model-io/src/writer.rs` | **CREATE (from move)** | Content of `model_writer.rs` verbatim. |
| `crates/slicer-runtime/src/model_loader.rs` | **DELETE** | After move. |
| `crates/slicer-runtime/src/model_loader_sidecar.rs` | **DELETE** | After move. |
| `crates/slicer-runtime/src/model_writer.rs` | **DELETE** | After move. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop three `pub mod ...;` lines and the `pub use model_writer::{write_3mf, write_obj};` re-export. Drop any `pub use model_loader::...` lines. |
| `crates/slicer-runtime/src/run.rs` | **EDIT** | Reshape `run_slice` to take `MeshIR`/`Arc<MeshIR>` instead of `&Path`. Delete the body's `let mesh = load_model(path)?;` step. |
| `crates/slicer-runtime/src/helpers_cmd.rs` | **EDIT (imports only)** | Change `use crate::model_loader::{assemble_object, load_model};` → `use slicer_model_io::{assemble_object, load_model};`. Body unchanged. (Packet 82 then moves this file into pnp-cli; the import rewrite happens here so packet 82 is a pure move.) |
| `crates/slicer-runtime/Cargo.toml` | **EDIT** | Delete the five direct deps (`stl_io`, `tobj`, `zip`, `quick-xml`, `uuid`). Add NO new deps. |
| `crates/pnp-cli/Cargo.toml` | **EDIT** | Add `slicer-model-io = { path = "../slicer-model-io" }`. |
| `crates/pnp-cli/src/main.rs` (or its slice subcommand module) | **EDIT** | At the top of the slice subcommand body, call `let mesh = slicer_model_io::load_mesh(&args.model)?;` and pass `mesh` to `slicer_runtime::run::run_slice(...)`. |
| `crates/slicer-runtime/tests/**` | **EDIT (imports) or MOVE** | Tests whose SUT is a moved symbol → move to `crates/slicer-model-io/tests/`. Tests whose SUT is a runtime symbol but which call `load_model` as a fixture step → update the import (`use slicer_model_io::load_model;`). The `slicer-runtime/tests/integration/main.rs` and `tests/executor/main.rs` aggregators lose `mod` declarations for any moved tests. |

Primary edit target ≤ 3 files: the new `slicer-model-io` crate (counted as one), `slicer-runtime/src/lib.rs`, `slicer-runtime/src/run.rs`. All other edits are mechanical follow-on.

## Files in Scope (read+edit)

- The 14 files in the table above. Plus, conditionally: any test file under `crates/slicer-runtime/tests/` that grep matches for `model_loader::`, `model_writer::`, `load_model`, `assemble_object`, `write_3mf`, `write_obj`, `parse_3mf_sidecar`. The implementer should enumerate these via a single `rg` dispatch in step 1, not read each test in full.

## Read-Only Context

| File | Why | Line-range hint |
|---|---|---|
| `crates/slicer-runtime/src/model_loader.rs` | The largest of the moved files (2 439 LOC). Read only to identify which `pub(crate)` items need promotion to `pub`. | Inspect `pub fn`, `pub(crate) fn`, `pub struct`, `pub(crate) struct` lines — delegate a `rg "^pub(\\(crate\\))? (fn\|struct\|enum)" crates/slicer-runtime/src/model_loader.rs` and consume the output. Do not load the whole file. |
| `crates/slicer-runtime/src/lib.rs` | Confirm which loader symbols are re-exported today. | Lines 22–32 (the `pub mod` block) and lines 127–138 (the `pub use` block around model_writer). |
| `crates/slicer-runtime/src/run.rs` | Confirm the current `run_slice` signature and its load-mesh body step. | First 50 lines. |
| `crates/slicer-runtime/src/helpers_cmd.rs` | Confirm which loader symbols it imports. | `use crate::model_loader::` greps. |
| `crates/pnp-cli/src/main.rs` (and any submodules under `crates/pnp-cli/src/`) | Identify the slice subcommand body to know where to insert `load_mesh`. | Search for `fn slice` or `Commands::Slice` match arm. |
| `docs/02_ir_schemas.md` §`MeshIR` | Confirm the `MeshIR` shape (mutability, `Arc` policy). | The `MeshIR` section. |
| `CONTEXT.md` | "Paint-ready 3MF" definition — confirms `model_writer.rs`'s output contract is unchanged. | Full file (~100 lines). |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work, do not touch.
- `crates/slicer-runtime/test-guests/**` — guest WASM fixtures, not affected by this packet.
- `modules/core-modules/**` — guest module sources, not affected.
- `crates/slicer-runtime/src/wit_host.rs` (5 259 LOC), `crates/slicer-runtime/src/dispatch.rs` (3 148 LOC), `crates/slicer-runtime/src/wasm_instance.rs`, `crates/slicer-runtime/src/instance_pool.rs` — packet 83 territory; do not read.
- `crates/slicer-runtime/src/gcode_emit.rs` (1 914 LOC) — packet 86 (B) territory; do not read.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | Which `pub(crate)` items in `model_loader.rs` are referenced from outside the file (callers in `slicer-runtime/src/run.rs`, `helpers_cmd.rs`, or `slicer-runtime/tests/`)? | `crates/slicer-runtime/src/{run.rs,helpers_cmd.rs}`, `crates/slicer-runtime/tests/` | LOCATIONS (file:line + symbol, ≤ 20 entries) |
| 2 | Which tests under `crates/slicer-runtime/tests/` import `model_loader::`, `model_writer::`, or `model_loader_sidecar::`? | `crates/slicer-runtime/tests/` | LOCATIONS (file:line, ≤ 20 entries) |
| 3 | Baseline SHA: capture `sha256sum` of `pnp_cli slice resources/benchy.stl` output before any source edit. | repo root | FACT `<hex>` |
| 4 | Post-move SHA: capture same `sha256sum` after AC-2 and AC-3 are green. | repo root | FACT `<hex>` |
| 5 | After move, confirm `cargo build --workspace` is green. | repo root | FACT pass/fail (first failing crate name if fail) |
| 6 | After move, confirm `cargo test -p slicer-runtime -p slicer-model-io -p pnp-cli` is green; report pass count delta vs pre-packet. | repo root | FACT pass/fail + pre/post counts |
| 7 | After move, confirm `cargo tree -p slicer-runtime --depth 5 --edges normal` is clean of the five file-format crates. | repo root | FACT clean/dirty (with offending crate names if dirty) |

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
- **Risk: `run_slice` callers outside `pnp-cli`.** Tests and benches may call `run_slice` with a `&Path`. Mitigation: dispatch #2 enumerates them; each caller adds a `load_mesh` line before the call. Mechanical.
- **Tradeoff: `pnp-cli` grows.** Its `Cargo.toml` gains one dep and `main.rs` gains one line. This is the intended direction — the binary is the place to read files.
- **Tradeoff: `slicer-model-io` is dep-heavy.** It carries the five format deps. That's by design — keeping them off `slicer-runtime` is the win.

## Context Cost Estimate

- Aggregate: **M** (4 steps × S, 1 step × M). No L step.
- Largest single step: step 3 (the actual file move + lib.rs edit + dep deletion + run.rs reshape, rated M). Reason: spans multiple files, requires careful import-path rewrite, and the implementer must dispatch the baseline-SHA capture before touching source.
- Highest-risk dispatch: dispatch #6 (workspace-wide test run delta). Mitigated by being a per-crate test (not `--workspace`), and the return format is a single line.

## Open Questions

None. `None — change is reversible via config-free file moves; no behavior locks introduced beyond the byte-identical g-code SHA assertion.`
