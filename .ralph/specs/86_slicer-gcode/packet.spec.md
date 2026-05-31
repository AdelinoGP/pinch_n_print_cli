---
status: draft
packet: 86
task_ids: [TASK-236]
requires: [84]
backlog_source: docs/07_implementation_status.md
---

# Packet 86 — Extract G-code Emission into `slicer-gcode`

## Goal

Move `gcode_emit.rs` (1 914 LOC: `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`, and the in-process `GCODE_EMIT_PRODUCER` static) out of `slicer-runtime/src/` into a new `slicer-gcode` crate, together with the `GCodeEmitter` and `GCodeSerializer` trait definitions currently in `crates/slicer-runtime/src/postpass.rs`; keep a thin `GCodeEmitProducer` wrapper (~30 LOC) in `crates/slicer-runtime/src/builtins/` that owns the `BuiltinProducer` impl and the `Blackboard` commit (per ADR-0001); `FeedrateConfig` already lives in `slicer-ir` after P84, so `slicer-gcode` imports it from there; the `classify_layers` call inside `emit_gcode` imports from `slicer-core` (where P84 moved the kernel).

## Scope Boundaries

This packet completes the four-seam goal of the deepening batch: WIT marshalling (P83 / `slicer-wasm-host`), static planning (P85 / `slicer-scheduler`), file I/O (P81 / `slicer-model-io`), and now G-code emission (P86 / `slicer-gcode`). The slicer-gcode crate's `[dependencies]` block contains `slicer-ir`, `slicer-core` (for `classify_layers`), `slicer-helpers` (for `drop_short_segments_mm`, `simplify_polyline_mm`); no `wasmtime`, no `slicer-wasm-host`, no `slicer-runtime`. The runtime keeps a ~30-LOC wrapper that calls `slicer_gcode::DefaultGCodeEmitter`'s `emit_gcode` and commits to `Blackboard` — ADR-0001's in-stage-commit pattern preserved. `pnp-cli` does NOT add a `slicer-gcode` dep; the runtime is the only consumer at the host level. The `machine-gcode-emit` core-module under `modules/core-modules/` is orthogonal (it's a `PostPass::GCodePostProcess` module that prepends/appends raw commands to an already-emitted `GCodeIR`) and is untouched. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 84 closed**: `FeedrateConfig` is in `slicer-ir`, and `overhang_classifier::classify_layers` is in `slicer-core`. Both are non-negotiable inputs to `gcode_emit`'s emit path.
- P85 is NOT a strict prerequisite — `gcode_emit` does not consume `ExecutionPlan` or planning types. P86 can follow P84 directly. Standard ordering keeps P85 first only for narrative clarity.
- Closure requires `cargo xtask build-guests --check` clean. This packet edits no `slicer-ir` / `slicer-sdk` / `slicer-schema` / `slicer-macros` content. Guests should stay clean; STALE means investigate, not paper over.
- Not a workspace-test checkpoint packet — closes on narrow per-crate gates per the deepening-batch policy.

## Acceptance Criteria

### AC-1 — `slicer-gcode` crate exists with the documented dep set

**Given** the extraction,
**When** `crates/slicer-gcode/Cargo.toml` is read,
**Then** it declares path deps `slicer-ir`, `slicer-core`, `slicer-helpers`. It does NOT declare `wasmtime`, `slicer-wasm-host`, `slicer-runtime`, `slicer-scheduler`, `slicer-schema`, `slicer-sdk`, `slicer-gcode`, or `slicer-model-io` (the latter two as path deps — `slicer-gcode` is itself; the others are out of layer). External deps: whatever `gcode_emit.rs` uses (`base64` for thumbnail, `serde` if applicable; preserved exactly).

| `test -f crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-ir *=' crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-core *=' crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-helpers *=' crates/slicer-gcode/Cargo.toml && ! grep -qE '^(wasmtime\|slicer-wasm-host\|slicer-runtime\|slicer-scheduler\|slicer-schema\|slicer-sdk\|slicer-model-io) *=' crates/slicer-gcode/Cargo.toml`

### AC-2 — `gcode_emit.rs` no longer exists under `slicer-runtime/src/`; equivalent exists under `slicer-gcode/src/`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/gcode_emit.rs` is true; `crates/slicer-gcode/src/` contains source files holding `pub struct DefaultGCodeEmitter`, `pub struct DefaultGCodeSerializer`, `pub struct ThumbnailAwareSerializer`, `pub fn tolerance_for_role`, `pub fn serialize_thumbnail_block`. File layout may flatten (e.g., `emit.rs`, `serialize.rs`, `thumbnail.rs`) or stay as one file.

| `test ! -f crates/slicer-runtime/src/gcode_emit.rs && [ $(find crates/slicer-gcode/src -name '*.rs' | xargs grep -lE 'pub struct (DefaultGCodeEmitter\|DefaultGCodeSerializer\|ThumbnailAwareSerializer)' | wc -l) -ge 1 ]`

### AC-3 — `GCodeEmitter` and `GCodeSerializer` trait defs live in `slicer-gcode`; `slicer-runtime::postpass` imports them

**Given** the trait relocation,
**When** the workspace is grepped,
**Then** `pub trait GCodeEmitter` and `pub trait GCodeSerializer` each appear exactly once and that occurrence is under `crates/slicer-gcode/src/`. They no longer appear in `crates/slicer-runtime/src/postpass.rs`. `crates/slicer-runtime/src/postpass.rs` imports them via `use slicer_gcode::{GCodeEmitter, GCodeSerializer};`. `crates/slicer-runtime/src/lib.rs`'s re-exports of these traits (per `lib.rs:139-142`) become `pub use slicer_gcode::{GCodeEmitter, GCodeSerializer};` (or are dropped if no external consumer relies on the runtime-path).

| `[ $(rg -l '^pub trait GCodeEmitter' crates/ | wc -l) -eq 1 ] && rg -l '^pub trait GCodeEmitter' crates/ | grep -qE '^crates/slicer-gcode/' && [ $(rg -l '^pub trait GCodeSerializer' crates/ | wc -l) -eq 1 ] && rg -l '^pub trait GCodeSerializer' crates/ | grep -qE '^crates/slicer-gcode/' && grep -qE 'use slicer_gcode::.*GCodeEmitter' crates/slicer-runtime/src/postpass.rs`

### AC-4 — Thin `GCodeEmitProducer` wrapper in `slicer-runtime/src/builtins/` retains the `BuiltinProducer` impl and the `Blackboard` commit (ADR-0001 preserved)

**Given** the wrapper-keeps-commit pattern,
**When** `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` (or its equivalent) is read,
**Then** it declares `pub static GCODE_EMIT_PRODUCER: BuiltinProducer = ...` with identical `stage_id`, `world_id`, `claim` (or whatever the existing `BuiltinProducer` fields are) as before P86. Its body calls `slicer_gcode::DefaultGCodeEmitter::new(...).emit_gcode(...)` then commits to `Blackboard`. Total LOC of this wrapper file ≤ 60.

| `test -f crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && grep -qE 'pub static GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && grep -qE 'slicer_gcode::DefaultGCodeEmitter\|use slicer_gcode' crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/gcode_emit_producer.rs) -le 80 ]`

### AC-5 — `slicer-runtime/src/lib.rs` no longer declares `pub mod gcode_emit;`; the producer is re-exported via `builtins/`

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the line `pub mod gcode_emit;` is absent. The line `pub use gcode_emit::{...};` (re-exporting `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`) is replaced by either `pub use slicer_gcode::{...};` (for backward source compat) OR dropped if no external consumer relies on `slicer_runtime::DefaultGCodeEmitter` etc. paths. `GCODE_EMIT_PRODUCER` is referenced in `runtime_builtins()` via the new `builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER` path.

| `! grep -qE '^pub mod gcode_emit;' crates/slicer-runtime/src/lib.rs && grep -qE 'GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/lib.rs`

### AC-6 — `slicer-gcode`'s emit calls `slicer_core::classify_layers` (P84's overhang kernel)

**Given** the dep chain after P84,
**When** `crates/slicer-gcode/src/` is grepped,
**Then** at least one source file contains `use slicer_core::classify_layers;` (or `use slicer_core::algos::overhang_classifier::classify_layers;` — depending on P84's chosen path) AND `classify_layers(&mut layers, &feedrate_config)` is called from `DefaultGCodeEmitter::emit_gcode` (or its helper). No `crate::overhang_classifier` reference remains anywhere.

| `grep -rqE 'use slicer_core::.*classify_layers' crates/slicer-gcode/src/ && ! rg -q 'crate::overhang_classifier' crates/slicer-gcode/src/ crates/slicer-runtime/src/`

### AC-7 — End-to-end slice produces byte-identical g-code vs the P85 baseline SHA (the current carried-forward baseline)

**Given** the wholesale move,
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode` runs,
**Then** the resulting SHA matches the SHA captured at P85 closure (which equals P84's, which equals P83's, which equals P81's — the byte-identical baseline carried through the entire deepening batch). Any divergence here would mean the move altered g-code text, which is a regression.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode && sha256sum /tmp/benchy-p86.gcode`

### AC-8 — `slicer-gcode` carries at least one golden-output test that runs without `slicer-runtime`

**Given** the seam,
**When** `cargo test -p slicer-gcode` runs,
**Then** at least one test under `crates/slicer-gcode/tests/` constructs a small `GCodeIR` value (from `slicer-ir`), calls `DefaultGCodeSerializer::serialize_gcode(...)` on it, and asserts the resulting string contains documented sentinel substrings (e.g., `;TYPE:WALL_OUTER` for an outer-wall path, `;LAYER:0` for the first layer, the documented thumbnail block sentinels if a thumbnail is present). The test imports zero `slicer-runtime` types.

| `cargo test -p slicer-gcode`

### AC-9 — Narrow per-crate test gates pass

**Given** the move,
**When** `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli` runs,
**Then** all three pass. `slicer-runtime` count delta = -(tests moved to `slicer-gcode/tests/`) + 0 (no other change); `slicer-gcode` count = +(tests migrated + new golden test).

| `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli`

## Negative Test Cases

### AC-N1 — `slicer-gcode/Cargo.toml` does NOT depend on `slicer-runtime` or `slicer-wasm-host`

**Given** the dep direction invariant,
**When** the manifest is read,
**Then** neither `slicer-runtime` nor `slicer-wasm-host` appears in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. (The seam exists precisely so g-code emission can be unit-tested without runtime.)

| `! grep -qE '^slicer-(runtime\|wasm-host) *=' crates/slicer-gcode/Cargo.toml`

### AC-N2 — `cargo tree -p slicer-gcode --edges normal` shows no `wasmtime` transitively

**Given** the wasmtime-free invariant,
**When** the dep tree is inspected,
**Then** `wasmtime` does NOT appear anywhere in the output. (Implication: `slicer-gcode` tests link no wasmtime; golden-output tests run fast.)

| `! cargo tree -p slicer-gcode 2>&1 | grep -qE '\bwasmtime\b'`

### AC-N3 — No file in `crates/slicer-gcode/src/` references `Blackboard`, `BuiltinProducer`, `ExecutionPlan`, or `ProgressEvent`

**Given** the algorithm/glue split (analogous to P84),
**When** `rg` runs,
**Then** the result is empty. The wrapper (which references all of these) stays in `slicer-runtime/src/builtins/gcode_emit_producer.rs`, not in `slicer-gcode`.

| `! rg -e '\b(Blackboard\|BuiltinProducer\|ExecutionPlan\|ProgressEvent)\b' crates/slicer-gcode/src/ 2>/dev/null`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check` (must stay clean — no guest-feeding paths edited)
4. `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli`

Workspace test gate NOT run at P86 close — that gate runs only at P83 (done), P85, P88.

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `GCodeIR`, `GCodeCommand`, `LayerCollectionIR`, `ExtrusionPath3D`, `ExtrusionRole`, `RetractMode`. The serializer's input contracts. No content change.
- `docs/04_host_scheduler.md` — `PostPass::LayerFinalization` / `PostPass::GCodePostProcess` stage placement. Confirms the `GCODE_EMIT_PRODUCER` wrapper's stage wiring is unchanged.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — the wrapper-keeps-commit pattern P86 preserves (analogous to P84's wrappers for the 6 algorithm builtins).
- `CLAUDE.md` §"Coordinate System Hazard" — the 1 unit = 100 nm convention. G-code serialization is the boundary where integer units convert back to millimetres.

## Doc Impact Statement

No doc files are edited by this packet. `docs/02_ir_schemas.md` defines `GCodeIR` and `GCodeCommand` exactly as the moved serializer consumes them. `docs/04_host_scheduler.md` describes the stage placement; the producer's `stage_id` is preserved.

No ADR follow-up. The trait relocation (from `postpass.rs` to `slicer-gcode`) is mechanical; the rationale is already documented under "deepening batch — extract g-code emission" in the plan file at `C:\Users\agpen\.claude\plans\lets-plan-the-packet-smooth-token.md`, and no future architecture reviewer would benefit from a dedicated ADR (the decision is self-evident from the dep graph after the move).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — confirm the `;TYPE:<role>` comment placement and the layer-boundary `;LAYER_CHANGE` / `;LAYER:<n>` markers that `DefaultGCodeSerializer` mirrors.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — confirm retract / unretract semantics (`G10`/`G11` vs inline-E `G1`) that `RetractMode` switching reproduces.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` (~:524) — already referenced by `overhang_classifier` (moved to `slicer-core` in P84). The `AC-2` parity baseline (strict `>` at band boundaries) is preserved by the kernel move; `slicer-gcode`'s `emit_gcode` just consumes the per-point `overhang_quartile` annotations.
- `OrcaSlicerDocumented/src/libslic3r/Format/SL1.cpp` (or the thumbnail-related file) — confirm the thumbnail block sentinels `; thumbnail begin` / `; thumbnail end` and the base64 chunk shape that `serialize_thumbnail_block` mirrors.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
