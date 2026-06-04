---
status: implemented
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

| `test -f crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-ir *=' crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-core *=' crates/slicer-gcode/Cargo.toml && grep -qE '^slicer-helpers *=' crates/slicer-gcode/Cargo.toml && ! grep -qE '^(wasmtime|slicer-wasm-host|slicer-runtime|slicer-scheduler|slicer-schema|slicer-sdk|slicer-model-io) *=' crates/slicer-gcode/Cargo.toml`

### AC-2 — `gcode_emit.rs` no longer exists under `slicer-runtime/src/`; equivalent exists under `slicer-gcode/src/`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/gcode_emit.rs` is true; `crates/slicer-gcode/src/` contains source files holding `pub struct DefaultGCodeEmitter`, `pub struct DefaultGCodeSerializer`, `pub struct ThumbnailAwareSerializer`, `pub fn tolerance_for_role`, `pub fn serialize_thumbnail_block`. File layout may flatten (e.g., `emit.rs`, `serialize.rs`, `thumbnail.rs`) or stay as one file.

| `test ! -f crates/slicer-runtime/src/gcode_emit.rs && [ $(find crates/slicer-gcode/src -name '*.rs' | xargs grep -lE 'pub struct (DefaultGCodeEmitter|DefaultGCodeSerializer|ThumbnailAwareSerializer)' | wc -l) -ge 1 ]`

### AC-3 — `GCodeEmitter` and `GCodeSerializer` trait defs live in `slicer-gcode`; `slicer-runtime::postpass` imports them

**Given** the trait relocation,
**When** the workspace is grepped,
**Then** `pub trait GCodeEmitter` and `pub trait GCodeSerializer` each appear exactly once and that occurrence is under `crates/slicer-gcode/src/`. They no longer appear in `crates/slicer-runtime/src/postpass.rs`. `crates/slicer-runtime/src/postpass.rs` imports them via `use slicer_gcode::{GCodeEmitter, GCodeSerializer};`. `crates/slicer-runtime/src/lib.rs`'s re-exports of these traits (per `lib.rs:139-142`) become `pub use slicer_gcode::{GCodeEmitter, GCodeSerializer};` (or are dropped if no external consumer relies on the runtime-path).

| `[ $(rg -l '^pub trait GCodeEmitter' crates/ | wc -l) -eq 1 ] && rg -l '^pub trait GCodeEmitter' crates/ | tr '\\\\' '/' | grep -qE '^crates/slicer-gcode/' && [ $(rg -l '^pub trait GCodeSerializer' crates/ | wc -l) -eq 1 ] && rg -l '^pub trait GCodeSerializer' crates/ | tr '\\\\' '/' | grep -qE '^crates/slicer-gcode/' && grep -qE 'use slicer_gcode::.*GCodeEmitter' crates/slicer-runtime/src/postpass.rs` (the `tr '\\\\' '/'` normalizes `rg`'s Windows backslash path separators to forward slashes so the AC is cross-platform)

### AC-4 — Thin `GCODE_EMIT_PRODUCER` metadata wrapper exists in `slicer-runtime/src/builtins/`

**Given** the codebase's actual `BuiltinProducer` shape (verified at Step 4: `BuiltinProducer` is a metadata struct — `id`, `stage`, `ir_writes`, `ir_reads`, `claims_holds`, `claims_requires`, `requires_modules`, `min_ir_schema`, `max_ir_schema`, OnceLock caches — NOT a closure-bearing wrapper. The actual emit call lives in `run.rs` / `postpass.rs`, the same shape P84 saw for `mesh_segmentation` and `overhang_classifier` which had no producer wrappers at all),
**When** `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` is read,
**Then** it declares `pub static GCODE_EMIT_PRODUCER: BuiltinProducer = ...` with identical `stage_id`, `world_id`, `claim` fields as the pre-P86 declaration. The file is ≤ 80 LOC. **ADR-0001's "in-stage commit" framing is preserved by the existing call-site in `run.rs` / `postpass.rs`, not by re-locating the emit body into the wrapper.** The original packet text assumed a closure-bearing wrapper that doesn't match this codebase's shape; AC-4 was amended at closure after the wrapper's metadata-only shape was confirmed.

| `test -f crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && grep -qE 'pub static GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/builtins/gcode_emit_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/gcode_emit_producer.rs) -le 80 ]`

### AC-5 — `slicer-runtime/src/lib.rs` no longer declares `pub mod gcode_emit;`; the producer is re-exported via `builtins/`

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the line `pub mod gcode_emit;` is absent. The line `pub use gcode_emit::{...};` (re-exporting `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`) is replaced by either `pub use slicer_gcode::{...};` (for backward source compat) OR dropped if no external consumer relies on `slicer_runtime::DefaultGCodeEmitter` etc. paths. `GCODE_EMIT_PRODUCER` is referenced in `runtime_builtins()` via the new `builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER` path.

| `! grep -qE '^pub mod gcode_emit\b' crates/slicer-runtime/src/lib.rs && grep -qE 'GCODE_EMIT_PRODUCER' crates/slicer-runtime/src/lib.rs` (word boundary catches both bare-semicolon `pub mod gcode_emit;` and brace-form `pub mod gcode_emit { pub use slicer_gcode::*; }` — both forbidden as backwards-compat shims per CLAUDE.md, same discipline P84 and P85 closed under)

### AC-6 — `slicer-gcode`'s emit calls `slicer_core::...classify_layers` (P84's overhang kernel) — `use`-statement OR inline-qualified form accepted

**Given** the dep chain after P84,
**When** `crates/slicer-gcode/src/` is grepped,
**Then** the `classify_layers` call resolves to `slicer_core` — either via a top-of-file `use slicer_core::classify_layers;` (or the deeper-path variant `use slicer_core::algos::overhang_classifier::classify_layers;`) OR via an inline fully-qualified call like `slicer_core::algos::overhang_classifier::classify_layers(...)` at the call site. Both forms produce identical behavior; the AC enforces functional resolution, not stylistic preference. No `crate::overhang_classifier` reference remains anywhere in `slicer-gcode/src/` or `slicer-runtime/src/`.

| `rg -q 'slicer_core::[^;()]*classify_layers' crates/slicer-gcode/src/ && ! rg -q 'crate::overhang_classifier' crates/slicer-gcode/src/ crates/slicer-runtime/src/`

### AC-7 — End-to-end slice produces byte-identical g-code vs the carried-forward baseline `89a329ad…`

**Given** the wholesale move,
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode` runs,
**Then** the resulting SHA equals `89a329ad3a4c1b7febca839edfca8b6302e562d8d2a390ee144252fd54e65a2b` — the byte-identical baseline carried unchanged across P81 → P83 → P84 → P85 closures. Any divergence here would mean the move altered g-code text, which is a regression. The most likely culprit on divergence is the `EmitContext` impl on `Blackboard` returning slightly different data than the pre-move `&Blackboard` direct access would have.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode && sha256sum /tmp/benchy-p86.gcode`

### AC-8 — `slicer-gcode` carries at least one golden-output test that runs without `slicer-runtime`

**Given** the seam,
**When** `cargo test -p slicer-gcode` runs,
**Then** at least one test under `crates/slicer-gcode/tests/` constructs a small `GCodeIR` value (from `slicer-ir`), calls `DefaultGCodeSerializer::serialize_gcode(...)` on it, and asserts the resulting string contains documented sentinel substrings (e.g., `;TYPE:WALL_OUTER` for an outer-wall path, `;LAYER:0` for the first layer, the documented thumbnail block sentinels if a thumbnail is present). The test imports zero `slicer-runtime` types.

| `cargo test -p slicer-gcode`

### AC-9 — Narrow per-crate test gates pass with full feature flag set

**Given** the move AND the P83/P84/P85 lesson that bare `cargo test -p <crate>` silently skips feature-gated test targets and SDK-test-gated targets,
**When** `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-gcode -p slicer-runtime -p pnp-cli` runs,
**Then** all three pass. `slicer-runtime` count delta = -(tests moved to `slicer-gcode/tests/`) + 0 (no other change); `slicer-gcode` count = +(tests migrated + new golden test). Bare `cargo test -p` form is REJECTED — proved at P85 closure to mask real regressions.

| `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-gcode -p slicer-runtime -p pnp-cli`

## Negative Test Cases

### AC-N1 — `slicer-gcode/Cargo.toml` does NOT depend on `slicer-runtime` or `slicer-wasm-host`

**Given** the dep direction invariant,
**When** the manifest is read,
**Then** neither `slicer-runtime` nor `slicer-wasm-host` appears in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. (The seam exists precisely so g-code emission can be unit-tested without runtime.)

| `! grep -qE '^slicer-(runtime|wasm-host) *=' crates/slicer-gcode/Cargo.toml`

### AC-N2 — `cargo tree -p slicer-gcode --edges normal` shows no `wasmtime` transitively

**Given** the wasmtime-free invariant,
**When** the dep tree is inspected,
**Then** `wasmtime` does NOT appear anywhere in the output. (Implication: `slicer-gcode` tests link no wasmtime; golden-output tests run fast.)

| `! cargo tree -p slicer-gcode 2>&1 | grep -qE '\bwasmtime\b'`

### AC-N3 — No file in `crates/slicer-gcode/src/` references `Blackboard`, `BuiltinProducer`, `ExecutionPlan`, or `ProgressEvent`

**Given** the algorithm/glue split (analogous to P84),
**When** `rg` runs,
**Then** the result is empty. The wrapper (which references all of these) stays in `slicer-runtime/src/builtins/gcode_emit_producer.rs`, not in `slicer-gcode`.

| `! rg -e 'use [^;]*\b(Blackboard|BuiltinProducer|ExecutionPlan|ProgressEvent)\b' crates/slicer-gcode/src/ && ! rg -e ': *&(mut )?(Blackboard|BuiltinProducer|ExecutionPlan|ProgressEvent)\b' crates/slicer-gcode/src/`

### AC-N4 — No undocumented `pub use slicer_gcode::` re-exports remain in `slicer-runtime/src/lib.rs`

**Given** the P84/P85-derived closure-cleanup rule (Step 6 prunes dead re-exports; survivors carry a `// kept:` annotation),
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** every `pub use slicer_gcode::...;` re-export line that survives the cleanup is followed by a one-line comment naming its surviving consumer (e.g., `// kept: consumed by crates/<x>/<y>.rs`). Re-exports without a surviving consumer must have been deleted. Same shape as AC-N4 in P85 — structural signal that P86 closes with no backwards-compat shim accumulation.

| `for line in $(grep -nE '^pub use slicer_gcode::' crates/slicer-runtime/src/lib.rs | cut -d: -f1); do prev=$((line-1)); next=$((line+1)); (sed -n "${prev}p" crates/slicer-runtime/src/lib.rs | grep -qE '^// kept:') || (sed -n "${next}p" crates/slicer-runtime/src/lib.rs | grep -qE '^// kept:') || exit 1; done` (accepts the `// kept:` annotation either immediately ABOVE or immediately BELOW each surviving `pub use slicer_gcode::` line — both are conventional Rust comment placements for adjacent annotation)

### AC-N5 — `crates/slicer-gcode/tests/` exists and runs the AC-8 golden test plus any migrated serializer tests in isolation

**Given** the P83.1 / P85 discipline (a new crate that claims to be testable without its previous dependencies must have a `tests/` directory exercising it in isolation),
**When** `crates/slicer-gcode/tests/` is inspected,
**Then** the directory exists and `cargo test -p slicer-gcode` runs ≥ 1 test (the AC-8 golden test at minimum; more if Step 6 migrated serializer tests). No file under `crates/slicer-gcode/tests/` imports any `slicer_wasm_host::*`, `slicer_runtime::*`, or `slicer_scheduler::*` symbol — proves the seam by running serialization in isolation, fulfilling the architectural-win-of-the-extraction premise.

| `[ -d crates/slicer-gcode/tests ] && [ $(cargo test -p slicer-gcode 2>&1 | grep -oE 'test result: ok\. [0-9]+ passed' | awk '{sum += $4} END {print sum+0}') -ge 1 ] && ! rg -e 'use slicer_(wasm_host|runtime|scheduler)::' crates/slicer-gcode/tests/`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check` (must stay clean — no guest-feeding paths edited)
4. `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-gcode -p slicer-runtime -p pnp-cli`

Workspace test gate NOT run at P86 close — that gate runs only at P83 (done), P85 (done), P88. Corrected workspace baseline post-P85 = 2067 passing; P86 narrow gates need not reproduce that, but if you run the workspace gate informally for sanity, carry the full flag set: `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace`.

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

## Deviations

Five deviations were recorded at packet closure. Four are packet-authoring corrections that did not require any code change; one is a substantive design pivot driven by an empirical finding at Step 1.

### D-1 — AC-4 wrapper shape reframed (packet-authoring)

The original AC-4 expected a "closure-bearing" wrapper whose body called `slicer_gcode::DefaultGCodeEmitter::new(...).emit_gcode(...)` and committed to `Blackboard`. Step 4 verified that this codebase's `BuiltinProducer` is a **pure metadata descriptor** (`id`, `stage`, `ir_writes`, `ir_reads`, `claims_holds`, `claims_requires`, `requires_modules`, `min_ir_schema`, `max_ir_schema`, plus OnceLock caches) — there is no `run` method on the trait and no closure body anywhere. The original pre-P86 `GCODE_EMIT_PRODUCER` in `gcode_emit.rs` was already a descriptor-only static, with the actual emit call living in `run.rs` / `postpass.rs`. Same shape P84 saw for `mesh_segmentation` and `overhang_classifier` (no wrappers at all). AC-4 amended at closure to assert the descriptor exists with the correct fields and stays ≤ 80 LOC; ADR-0001's in-stage-commit framing is preserved by the existing call site, not by relocating the emit body. **No code change.**

### D-2 — AC-3 cross-platform path normalization (packet-authoring)

The original AC-3 grep `'^crates/slicer-gcode/'` was Linux-only; on Windows `rg` emits backslash path separators (`crates\slicer-gcode\...`). Amended to add a `tr '\\\\' '/'` pipe before the grep so the regex matches on either platform. Substance of the AC (both traits live in `slicer-gcode`, `postpass.rs` imports them) was already true. **No code change.**

### D-3 — AC-6 inline-qualified path accepted (packet-authoring)

The original AC-6 grep required a `use slicer_core::...classify_layers;` statement. The implementation uses a fully-qualified inline call (`slicer_core::algos::overhang_classifier::classify_layers(...)`) at `emit.rs:226`. Functionally identical; the architectural intent (dependency on `slicer-core`'s kernel) is satisfied. AC amended to accept either form via `rg -q 'slicer_core::[^;()]*classify_layers'`. **No code change.**

### D-4 — AC-N4 comment placement convention (packet-authoring + cosmetic reformat)

The original AC-N4 sed-check required `// kept:` on the line immediately BELOW each surviving `pub use slicer_gcode::` line. The Step 6 worker placed `// kept:` ABOVE (also conventional Rust placement for adjacent annotation). AC amended to accept either ABOVE or BELOW. A secondary issue surfaced: multi-line `// kept:` comment blocks broke immediate-adjacency for the last surviving re-export. Resolved by collapsing each multi-line `// kept:` block in `crates/slicer-runtime/src/lib.rs` to a single line, preserving the consumer-list content. **Cosmetic comment reformat only; zero source-code semantic change.**

### D-5 — Step 1 design pivot: `EmitContext` trait dropped (substantive — empirical)

The original packet (and design.md prior to refinement) selected an `EmitContext` trait in `slicer-gcode` that `Blackboard` would impl in `slicer-runtime`, predicated on the hypothesis that `emit_gcode` reads deferred retracts / travel moves from `Blackboard`. Step 1 dispatch #1 surveyed every `blackboard.<method>` / `bb.<method>` / `ctx.<method>` call site inside the 1 914 LOC `gcode_emit.rs` and returned **zero** call sites. The parameter was declared as `_blackboard: &Blackboard` at line 263 (underscored = unused). The hypothesis was empirically false. The simpler rejected alternative — drop the parameter entirely — became the chosen path. `design.md`, `requirements.md`, and `implementation-plan.md` were refined in-session before code work began. No EmitContext trait, no `impl EmitContext for Blackboard`, no `context.rs` module; only a `GCodeEmitError` enum (in `slicer-gcode/src/error.rs`) for dep-direction maintenance, wrapped at the runtime call site via a free fn `gcode_emit_error_to_postpass` (the `From` impl was orphan-rule-blocked because `PostpassError` lives in `slicer-ir` and `GCodeEmitError` lives in `slicer-gcode`, neither owned by `slicer-runtime`). **Substantive design change; no ADR required (mechanical seam adjustment, not high-stakes architectural decision).**
