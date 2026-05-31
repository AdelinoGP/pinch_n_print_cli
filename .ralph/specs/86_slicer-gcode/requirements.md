# Packet 86 — Requirements

## Problem Statement

`gcode_emit.rs` (1 914 LOC) is the final large concern in `slicer-runtime` that has nothing to do with orchestration: it is pure IR → text serialization. The structural problem is the same as the prior moves — three concerns braided:

1. **Pure serialization** (`DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`) — the depth this packet extracts.
2. **A `BuiltinProducer` impl + `Blackboard` commit** — the runtime wrapper preserved per ADR-0001.
3. **Trait definitions** (`GCodeEmitter`, `GCodeSerializer`) that today live in `crates/slicer-runtime/src/postpass.rs` and are re-exported through `slicer-runtime`'s `pub use postpass::{GCodeEmitter, GCodeSerializer};` block. They belong with the serialization, not with the orchestrator that calls them.

The fix mirrors P84's algorithm split: kernel + trait defs move to `slicer-gcode`; a ~30-LOC `GCodeEmitProducer` wrapper stays in `slicer-runtime/src/builtins/`. After P86, the runtime can be unit-tested without g-code knowledge, and g-code serialization can be golden-tested without runtime — both interfaces deepen.

`FeedrateConfig`'s move to `slicer-ir` (P84 prework) means `slicer-gcode` imports `FeedrateConfig` from `slicer-ir`, not from runtime. `overhang_classifier::classify_layers` lives in `slicer-core` (P84) — `slicer-gcode`'s emit path calls it from there.

## Grouped Task IDs

- **TASK-236** (new) — Extract G-code emission into `slicer-gcode`. Final TASK in "Architecture Deepening Phase II"; closes the four-seam goal alongside TASK-231/232/233/234/235.

## In Scope

- Create `crates/slicer-gcode/` with:
  - `Cargo.toml` declaring `slicer-ir`, `slicer-core`, `slicer-helpers` as path deps. External deps preserved from `gcode_emit.rs` usage (e.g., `base64` for thumbnail; whatever `gcode_emit` imports today). NO `wasmtime`, NO `slicer-wasm-host`, NO `slicer-runtime`, NO `slicer-scheduler`, NO `slicer-schema`, NO `slicer-sdk`.
  - `src/lib.rs` with `pub mod emit; pub mod serialize; pub mod thumbnail;` (or a single `pub mod gcode_emit;` if a flat layout is preferred). Plus public re-exports for the documented surface: `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`, `GCodeEmitter` (trait), `GCodeSerializer` (trait).
- Move `crates/slicer-runtime/src/gcode_emit.rs` (1 914 LOC) verbatim minus the `GCODE_EMIT_PRODUCER` static + `BuiltinProducer` impl. The pure types and free functions go to `slicer-gcode/src/`.
- Move `GCodeEmitter` and `GCodeSerializer` trait definitions from `crates/slicer-runtime/src/postpass.rs` (lines ~144–163) into `crates/slicer-gcode/src/lib.rs` (or its `serialize.rs` submodule). Their signatures preserved exactly:
  ```rust
  pub trait GCodeEmitter {
      fn emit_gcode(&self, layer_irs: &[LayerCollectionIR], blackboard: &Blackboard)
          -> Result<GCodeIR, PostpassError>;
      fn travel_feedrate_mm_per_min(&self) -> Option<f32> { None }
  }
  pub trait GCodeSerializer {
      fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError>;
  }
  ```
  **Trait sig caveat**: `GCodeEmitter::emit_gcode` takes `&Blackboard` and returns `PostpassError`. After the move, the trait sig stays the same. `Blackboard` and `PostpassError` are `slicer-runtime` types — so `slicer-gcode` would need them in scope. Resolution: change the trait's parameter from `&Blackboard` to a `&dyn EmitContext` (or similar minimal trait that `Blackboard` impls), AND change the error type from `PostpassError` to a new `GCodeError` (or accept `PostpassError` via a tracked imports rewrite — design.md picks the lightest option). The shape is preserved; only the type names cross crates.
- Create `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` (~30–60 LOC) holding `pub static GCODE_EMIT_PRODUCER: BuiltinProducer = ...` and its body: construct a `DefaultGCodeEmitter`, call `emit_gcode(layers, blackboard)`, commit the result to `Blackboard` (per ADR-0001).
- Update `crates/slicer-runtime/src/builtins/mod.rs` to declare `pub mod gcode_emit_producer;` and re-export `GCODE_EMIT_PRODUCER`.
- Update `crates/slicer-runtime/src/postpass.rs`:
  - Delete the `pub trait GCodeEmitter` and `pub trait GCodeSerializer` definitions.
  - Add `use slicer_gcode::{GCodeEmitter, GCodeSerializer};` at the top of the file.
  - The rest of `postpass.rs` (orchestration) is unchanged.
- Update `crates/slicer-runtime/src/lib.rs`:
  - Drop `pub mod gcode_emit;`.
  - Drop or rewrite `pub use postpass::{GCodeEmitter, GCodeSerializer};` (these traits are in `slicer-gcode` now; either drop the re-export or rewrite to `pub use slicer_gcode::{GCodeEmitter, GCodeSerializer};` for backward source compat).
  - Drop or rewrite `pub use gcode_emit::{serialize_thumbnail_block, tolerance_for_role, DefaultGCodeEmitter, DefaultGCodeSerializer, ThumbnailAwareSerializer};` — either drop, or rewrite to `pub use slicer_gcode::{...};`.
  - Update `runtime_builtins()` to reference `&builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER as &dyn Producer`.
- Update `crates/slicer-runtime/Cargo.toml`:
  - Add `slicer-gcode = { path = "../slicer-gcode" }`.
- Migrate tests under `crates/slicer-runtime/tests/` whose SUT is a serialization symbol (`DefaultGCodeSerializer`, `format_coord`, `format_xyz`, etc.) into `crates/slicer-gcode/tests/`. Tests whose SUT is the producer wrapper (`GCODE_EMIT_PRODUCER` end-to-end) stay in `slicer-runtime/tests/` and rewire imports.
- Add at least one golden-output test to `crates/slicer-gcode/tests/`: construct a small `GCodeIR`, serialize it, assert the documented sentinel substrings.

## Out of Scope

- `crates/slicer-test/`, `crates/slicer-sdk/` — concurrent work.
- The `machine-gcode-emit` core-module under `modules/core-modules/` — orthogonal to gcode_emit (it's a `PostPass::GCodePostProcess` module that mutates an already-emitted `GCodeIR`, not a serializer). Not touched.
- New abstractions or trait redesigns. The `GCodeEmitter` / `GCodeSerializer` interfaces are preserved (modulo the `Blackboard` / `PostpassError` cross-crate rewrite in the sig — see design.md for the chosen resolution).
- Refactoring `gcode_emit.rs`'s internals. The file moves verbatim; its `pub fn`s, helpers, OrcaSlicer-parity constants, and tolerance tables are preserved exactly.
- Modularising `gcode_emit` itself into a WASM core-module (i.e., making `PostPass::GCodeEmit` a swappable stage). Out of scope; the host builtin remains the only implementation. (A future packet could explore that, but it requires a new WIT export and is bigger than P86.)
- Touching `pnp-cli` other than for any test rewires it needs (none expected — pnp-cli doesn't import `DefaultGCodeEmitter` or the traits directly).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `GCodeIR`, `GCodeCommand`, `LayerCollectionIR`, `ExtrusionPath3D`, `ExtrusionRole`, `RetractMode`. The exact field set the serializer consumes; no change.
- `docs/04_host_scheduler.md` — `PostPass::LayerFinalization` / `PostPass::GCodePostProcess` stage placement, confirms `GCODE_EMIT_PRODUCER`'s `stage_id` placement is unchanged.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — the wrapper-keeps-commit pattern, mirrored from P84.
- `CLAUDE.md` §"Coordinate System Hazard" — the 1 unit = 100 nm convention. G-code text outputs millimetres; the serializer converts at the boundary.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-9, AC-N1..AC-N3). Measurable refinements:

- **AC-3 — Trait sig preservation**: the `GCodeEmitter` and `GCodeSerializer` trait method signatures are byte-for-byte identical to their P85-closure forms (modulo crate-path imports). The implementation log records both before/after sigs.
- **AC-7 — Byte-identical g-code**: SHA carries from P85 closure (which equals P84/P83/P81). Any divergence is a regression.
- **AC-8 — Golden test**: at minimum one test constructs a small `GCodeIR` (one wall path, one infill path, one travel move, optionally a thumbnail) and asserts the serialized string contains `;TYPE:WALL_OUTER`, `;LAYER:0`, the documented thumbnail sentinels when applicable. The test imports zero `slicer_runtime::*` types — proves the seam.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test -f crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-ir' crates/slicer-gcode/Cargo.toml && ! grep -qE '^(wasmtime\|slicer-wasm-host\|slicer-runtime\|slicer-scheduler) *=' crates/slicer-gcode/Cargo.toml` | FACT pass/fail |
| AC-2 | `test ! -f crates/slicer-runtime/src/gcode_emit.rs && find crates/slicer-gcode/src -name '*.rs' \| xargs grep -lE 'pub struct DefaultGCodeEmitter' \| head -1 \| grep -q .` | FACT pass/fail |
| AC-3 | `rg -l 'pub trait GCodeEmitter' crates/ \| grep -qE '^crates/slicer-gcode/' && grep -qE 'use slicer_gcode::.*GCodeEmitter' crates/slicer-runtime/src/postpass.rs` | FACT pass/fail |
| AC-4 | `grep -qE 'pub static GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/gcode_emit_producer.rs) -le 80 ]` | FACT pass/fail |
| AC-5 | `! grep -qE '^pub mod gcode_emit;' crates/slicer-runtime/src/lib.rs && grep -qE 'GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-6 | `grep -rqE 'use slicer_core::.*classify_layers' crates/slicer-gcode/src/ && ! rg -q 'crate::overhang_classifier' crates/slicer-gcode/src/ crates/slicer-runtime/src/` | FACT pass/fail |
| AC-7 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode && sha256sum /tmp/benchy-p86.gcode` | SNIPPET (SHA) |
| AC-8 | `cargo test -p slicer-gcode` | FACT pass/fail + count |
| AC-9 | `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli` | FACT pass/fail + counts |
| AC-N1 | `! grep -qE '^slicer-(runtime\|wasm-host) *=' crates/slicer-gcode/Cargo.toml` | FACT pass/fail |
| AC-N2 | `! cargo tree -p slicer-gcode 2>&1 \| grep -qE '\bwasmtime\b'` | FACT pass/fail |
| AC-N3 | `! rg -e '\b(Blackboard\|BuiltinProducer\|ExecutionPlan\|ProgressEvent)\b' crates/slicer-gcode/src/` | FACT empty/non-empty |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- The `GCodeEmitter` trait sig rewrite (`&Blackboard` → `&dyn EmitContext` OR `&Blackboard` via re-export — design.md's chosen option) MUST land together with the `gcode_emit.rs` move; otherwise `postpass.rs` cannot compile against either signature.
- `slicer-runtime/src/builtins/gcode_emit_producer.rs` MUST be created BEFORE `slicer-runtime/src/lib.rs` drops `pub mod gcode_emit;` and updates `runtime_builtins()`; the producer reference must always resolve.
- Guest rebuild is NOT required (no guest-feeding path is edited); `cargo xtask build-guests --check` should stay clean. STALE means investigate.

## Packet-Specific Context Discipline

- `gcode_emit.rs` is 1 914 LOC. NEVER load in full. Approach: identify section boundaries via grep (`pub struct`, `pub fn`, `static GCODE_EMIT_PRODUCER`, `impl BuiltinProducer`). Move section-by-section.
- `postpass.rs` contains the two trait defs at ~L144–163; ±30-line reads only.
- `OrcaSlicerDocumented/` is consulted only via delegated sub-agents per the `orca-delegation` snippet.
