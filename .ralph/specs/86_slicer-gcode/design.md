# Packet 86 — Design

## Controlling Code Paths

```
slicer-ir, slicer-core, slicer-helpers
                │
                ▼
         slicer-gcode  (zero wasmtime, zero runtime, zero scheduler)
                │
                ▼
         slicer-runtime/src/builtins/gcode_emit_producer.rs  (~30–60 LOC wrapper)
                │
                ▼
         slicer-runtime/src/lib.rs::runtime_builtins()
```

OrcaSlicer comparison surface: g-code text format, retract semantics, thumbnail block. See `requirements.md` §OrcaSlicer Reference Obligations; do not restate the delegation rules here.

## Architecture Constraints

- ADR-0001 preserved: the `GCODE_EMIT_PRODUCER` `BuiltinProducer` impl and the `Blackboard` commit live in `slicer-runtime/src/builtins/gcode_emit_producer.rs` (in-stage).
- ADR-0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0007 (P85 — CompiledModule Static/Live split with HashMap-keyed pairing) all preserved. ADR-0004 (Test support in slicer-sdk, P77) is unrelated to this packet's surface.
- `slicer-gcode` MUST NOT depend on `slicer-runtime`, `slicer-wasm-host`, `slicer-scheduler`, `slicer-schema`, `slicer-sdk`, `slicer-model-io`. Path deps in `slicer-gcode/Cargo.toml` are limited to `slicer-ir`, `slicer-core`, `slicer-helpers` plus crates.io external deps the moved code uses.
- No path in this packet's change surface feeds the guest WASM build. `wasm-staleness` snippet intentionally NOT included.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`gcode_emit` converts INTEGER UNITS BACK TO MILLIMETRES at the text-output boundary. The serializer's `format_xyz` / `format_coord` helpers are the conversion site; preserve their exact rounding behavior or AC-7 SHA parity breaks.)

## Selected Approach

Verbatim file move + trait relocation + wrapper retention. The trait-sig question: the `GCodeEmitter::emit_gcode` method today takes `&Blackboard` and returns `Result<_, PostpassError>`. After the move, both would create a back-edge if `slicer-gcode` imported them from runtime.

**Chosen resolution: drop the `&Blackboard` parameter from `emit_gcode` entirely** and return `Result<GCodeIR, GCodeEmitError>` where `GCodeEmitError` is a new error enum in `slicer-gcode` mirroring the `PostpassError` variants that `gcode_emit` originated. The runtime wraps `GCodeEmitError` into `PostpassError` at the wrapper site via a `From` impl.

**Empirical basis (P86 Step 1 dispatch #1).** A grep of every `blackboard.<method>` / `bb.<method>` / `ctx.<method>` call site inside `crates/slicer-runtime/src/gcode_emit.rs` (1 914 LOC) returned **zero** call sites. The parameter is declared as `_blackboard: &Blackboard` at line 263 — the leading underscore is the Rust convention for an intentionally unused parameter. The only `Blackboard` reference in the file is the `use crate::{Blackboard, ...}` import at line 37. The original packet-author hypothesis ("`emit_gcode` reads deferred retracts/travel moves from `Blackboard`") is therefore empirically false. The simpler alternative — drop the parameter — is the demonstrably correct seam.

Rejected alternatives:

- **Introduce an `EmitContext` trait in `slicer-gcode`** that `Blackboard` impls in `slicer-runtime`, keeping `emit_gcode`'s parameter as `&dyn EmitContext`. Rejected: with zero `Blackboard` methods actually called by `emit_gcode`, the trait would have an empty method set — pure ceremony with no payload. Would also require an `impl EmitContext for Blackboard {}` in runtime and a new `context.rs` module. Net cost is several files for zero behavioral benefit. (Was the original Selected Approach until Step 1 dispatch #1 falsified its premise.)
- **Keep `&Blackboard` in the trait sig** by adding `slicer-runtime` to `slicer-gcode/Cargo.toml`. Rejected: defeats the seam. The whole point of the move is to make `slicer-gcode` testable without runtime.
- **Move `Blackboard` itself into a deeper crate** so `slicer-gcode` can import it without a runtime dep. Rejected: scope explosion. `Blackboard` is the runtime's shared mutable state; relocating it is a separate architectural decision worth its own packet.
- **Add a free fn `serialize_to_string(ir: &GCodeIR) -> String` in `slicer-gcode` and keep the trait def in `slicer-runtime::postpass`** as a wrapper. Rejected: the trait is the seam — moving the impl without the trait leaves the seam shallow.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-gcode/Cargo.toml` | **CREATE** | Deps: `slicer-ir` (path), `slicer-core` (path), `slicer-helpers` (path), `base64` (workspace inheritance, or whatever `gcode_emit.rs` uses today), `thiserror` (for the new `GCodeEmitError`). |
| `crates/slicer-gcode/src/lib.rs` | **CREATE** | `pub mod emit;`, `pub mod serialize;`, `pub mod thumbnail;`, `pub mod error;`. Re-exports for the public surface. |
| `crates/slicer-gcode/src/emit.rs` | **CREATE (from move)** | Holds `DefaultGCodeEmitter` and the `GCodeEmitter` trait def. |
| `crates/slicer-gcode/src/serialize.rs` | **CREATE (from move)** | Holds `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `GCodeSerializer` trait def. |
| `crates/slicer-gcode/src/thumbnail.rs` | **CREATE (from move)** | Holds `serialize_thumbnail_block`. |
| `crates/slicer-gcode/src/error.rs` | **CREATE** | `pub enum GCodeEmitError { ... }` — new error enum (variants mirror the failure modes that `gcode_emit.rs` returned as `PostpassError` today). No `EmitContext` trait: empirically (Step 1 dispatch #1) `emit_gcode` calls zero `Blackboard` methods, so the seam needs only the error type. |
| `crates/slicer-gcode/tests/golden_emit_tdd.rs` | **CREATE** | One golden test per the AC-8 contract. |
| `crates/slicer-runtime/src/gcode_emit.rs` | **DELETE** | |
| `crates/slicer-runtime/src/postpass.rs` | **EDIT** | Delete the two trait defs at ~L144–163. Add `use slicer_gcode::{GCodeEmitter, GCodeSerializer};` at the top. Update any internal `PostpassError` conversions if needed. |
| `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` | **CREATE** | The wrapper: `pub static GCODE_EMIT_PRODUCER: BuiltinProducer = ...` + body. Constructs `DefaultGCodeEmitter` from runtime config; calls `emit_gcode(&layers)`; converts the returned `GCodeEmitError` → `PostpassError` (or whatever the wrapper signature expects); commits `GCodeIR` to `Blackboard`. ≤ 80 LOC. |
| `crates/slicer-runtime/src/builtins/mod.rs` | **EDIT** | Add `pub mod gcode_emit_producer;` and re-export `GCODE_EMIT_PRODUCER`. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop `pub mod gcode_emit;`. Drop or rewrite `pub use postpass::{GCodeEmitter, GCodeSerializer};` (rewrite to `pub use slicer_gcode::{GCodeEmitter, GCodeSerializer};` for transitional source compat). Drop or rewrite `pub use gcode_emit::{...};`. Update `runtime_builtins()` to reference `builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER`. |
| `crates/slicer-runtime/Cargo.toml` | **EDIT** | Add `slicer-gcode = { path = "../slicer-gcode" }`. No deps removed (`base64` if it was a runtime dep is still needed for transitive use — verify via dispatch #4). |
| `crates/slicer-runtime/tests/**` | **EDIT or MOVE** | Tests whose SUT is serialization (`DefaultGCodeSerializer`, `format_coord`, `format_xyz`, `tolerance_for_role`) → move to `crates/slicer-gcode/tests/`. Tests whose SUT is `GCODE_EMIT_PRODUCER` end-to-end → stay; rewire `use slicer_runtime::DefaultGCodeEmitter;` to `use slicer_gcode::DefaultGCodeEmitter;` (or via the transitional re-export). |

Primary edit target ≤ 3 files: the new `slicer-gcode` crate (counted as one), `crates/slicer-runtime/src/postpass.rs`, `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` (new wrapper). All other edits are mechanical follow-on.

## Files in Scope (read+edit)

The 14 files in the table above plus conditional test files from dispatch #3.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-runtime/src/gcode_emit.rs` | Identify section boundaries: `pub struct`, `pub fn`, `pub static GCODE_EMIT_PRODUCER`, `impl BuiltinProducer`. NEVER load full 1 914 LOC. | Targeted grep + ±50-line reads per match. |
| `crates/slicer-runtime/src/postpass.rs` | Find the two trait defs at ~L144–163. | Line range L130–180. |
| `crates/slicer-runtime/src/lib.rs` | Identify current `pub use postpass::{GCodeEmitter, GCodeSerializer};` and `pub use gcode_emit::{...};` blocks. | Lines 106–142 (re-export region around postpass/gcode). |
| `docs/02_ir_schemas.md` | Confirm `GCodeIR`, `GCodeCommand` shape (unchanged). | Section search. |
| `docs/04_host_scheduler.md` | Confirm `GCODE_EMIT_PRODUCER`'s stage placement (no change). | Section search. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — consulted only via delegated sub-agents per the `orca-delegation` snippet in `packet.spec.md`.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work.
- `crates/slicer-runtime/src/wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` — moved in P83; do not read.
- `crates/slicer-runtime/src/{model_loader,helpers_cmd,cli}.rs` — already gone (P81/P82).
- `crates/slicer-runtime/src/{mesh_analysis,paint_segmentation,prepass_slice,support_geometry,mesh_segmentation,overhang_classifier}.rs` — gone (P84).
- `crates/slicer-runtime/src/{manifest,dag,validation,execution_plan,dag_cli,topology,stage_order,module_search_path,config_resolution}.rs` — gone (P85).
- `crates/slicer-runtime/src/region_mapping.rs` — P87 territory; stays.
- `crates/slicer-runtime/src/{layer_executor,prepass,layer_finalization,run,pipeline}.rs` — read only their `use` lines to confirm rewrites.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | Which `Blackboard` methods does `gcode_emit.rs` call (i.e., what's the minimal `EmitContext` trait surface)? Search for `bb.<method>` and `blackboard.<method>` in `crates/slicer-runtime/src/gcode_emit.rs`. | `crates/slicer-runtime/src/gcode_emit.rs` | LOCATIONS (file:line + method name, ≤ 20 entries) |
| 2 | What is the verbatim signature of `pub trait GCodeEmitter` and `pub trait GCodeSerializer` in `crates/slicer-runtime/src/postpass.rs`? | The two trait def line ranges | SNIPPETS (≤ 30 lines total) |
| 3 | Which test files under `crates/slicer-runtime/tests/` reference `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `format_coord`, `format_xyz`, `tolerance_for_role`, or `serialize_thumbnail_block`? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 20 entries) |
| 4 | Is `base64` used anywhere in `crates/slicer-runtime/src/` besides `gcode_emit.rs`? | `crates/slicer-runtime/src/` | FACT (yes/no) |
| 5 | OrcaSlicer reference: which file(s) in `OrcaSlicerDocumented/src/libslic3r/GCode*` define the `;TYPE:<role>` comment placement and the layer-boundary `;LAYER:<n>` markers? Return LOCATIONS (≤ 10 entries), no code. | `OrcaSlicerDocumented/` | LOCATIONS |
| 6 | OrcaSlicer reference: the thumbnail block sentinels (`; thumbnail begin`/`; thumbnail end`) and the base64 chunk length OrcaSlicer uses. | `OrcaSlicerDocumented/` | FACT (1–2 lines: sentinel strings + chunk width) |
| 7 | Baseline g-code SHA from P85 closure. | repo root | FACT `<hex>` |
| 8 | Post-packet g-code SHA. | repo root | FACT `<hex>` |
| 9 | After move, `cargo build --workspace`. | repo root | FACT pass/fail |
| 10 | After move, `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli`. | repo root | FACT pass/fail + counts |

## Data and Contract Notes

- `GCodeEmitter::emit_gcode` signature after move:
  ```rust
  pub trait GCodeEmitter {
      fn emit_gcode(&self, layers: &[LayerCollectionIR])
          -> Result<GCodeIR, GCodeEmitError>;
      fn travel_feedrate_mm_per_min(&self) -> Option<f32> { None }
  }
  ```
  The wrapper in `slicer-runtime` calls `emit_gcode(&layers)` directly; no context indirection. The `&Blackboard` parameter is dropped (Step 1 dispatch #1: it was `_blackboard`, unused, with zero method calls).
- `GCodeSerializer::serialize_gcode` signature:
  ```rust
  pub trait GCodeSerializer {
      fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError>;
  }
  ```
- `GCodeEmitError` enum mirrors the existing `PostpassError` variants that `gcode_emit` was the source of. The wrapper converts `GCodeEmitError` → `PostpassError` via a `From` impl.

## Locked Assumptions and Invariants

- ADR-0001 preserved: the `GCODE_EMIT_PRODUCER`'s commit-to-`Blackboard` happens inside the wrapper in `slicer-runtime/src/builtins/gcode_emit_producer.rs`.
- Byte-identical g-code: AC-7 SHA = P85 closure SHA = ... = P81 closure SHA. The cross-packet baseline is preserved.
- OrcaSlicer-parity constants (`;TYPE:` labels, retract opcodes, thumbnail format) are preserved verbatim — `gcode_emit.rs`'s content moves unchanged into `slicer-gcode`.
- Guest WASMs stay clean (`--check`); no guest-feeding path is edited.

## Risks and Tradeoffs

- **Risk: `GCodeEmitError`-to-`PostpassError` conversion misses a variant.** Mitigation: walk `gcode_emit.rs` `Err(_)` paths and map each to a `GCodeEmitError` variant; the `From<GCodeEmitError> for PostpassError` impl in `slicer-runtime` covers all of them.
- **Risk: SHA divergence on AC-7** because the moved code's behavior differs from before. Mitigation: the move is verbatim — dropping `_blackboard` changes nothing observable because it was already unused (line 263 of pre-move gcode_emit.rs). If SHA does diverge, the likely culprit is the `classify_layers` re-import from `slicer-core` (P84) or the `FeedrateConfig` re-import from `slicer-ir` (P84).
- **Tradeoff: introduces `GCodeEmitError`** — one new type to maintain the dep direction. Acceptable cost; the alternative is `slicer-gcode → slicer-runtime` back-edge.

## Context Cost Estimate

- Aggregate: **M.** No L step. Total step count: 9.
- Largest single step: step 5 (the bulk move + wrapper creation + trait sig rewrite). Rated M.
- Highest-risk dispatch: dispatch #1 (`EmitContext` trait surface) — if the trait is incomplete, the wrapper compile breaks.

## Open Questions

`None — change is reversible. The transitional re-export block in slicer-runtime/src/lib.rs is the rollback hatch; GCodeEmitError is the minimal dep-direction maintenance and could be replaced by re-exporting PostpassError from a deeper crate in a future packet if preferred.`

No ADR follow-up — the trait sig change + new error type are mechanical seam adjustments, not high-stakes architectural decisions worth a future-reader explanation.
